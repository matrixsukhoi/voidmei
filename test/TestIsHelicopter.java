import parser.Blkx;
import prog.i18n.Lang;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.util.TreeSet;

/**
 * isHelicopter 解析验证 —— 遍历 data/ 全部物理 FM 文件:
 *
 * 1. 代表机型硬断言: mi_24a/ka_50/uh_1b = true (直升机), spitfire_f24/a6m2_zero/f_16aj = false
 * 2. 全量对拍: 解析结果直升机集合 == 文本里任一 VortexRingVFlowMult 首数非零的集合
 *    (防全文扫描写错; 数量随游戏版本浮动, 只比集合不绑死数值)
 * 3. 无该字段的机型 (老格式 FM, 全固定翼) 必须降级为 false
 *
 * 背景: 涡环系数 VortexRingVFlowMult 仅直升机 FM 有非零值 (全库实测 93 架全中零误报);
 * 固定翼恒 0,0, 老格式无此字段。段名不固定 (Propeller0~N/PropellerType0~N) 且同文件可
 * 多处 (mi_24a 主旋翼非零+尾桨为零), 故解析侧须全文遍历。
 *
 * Run with: python script/build.py test heli-detect  (data/ 缺失时由 build.py 跳过)
 */
public class TestIsHelicopter {

	private static int passed = 0;
	private static int failed = 0;

	private static void assertTrue(boolean cond, String msg) {
		if (cond) {
			passed++;
			System.out.println("  PASS: " + msg);
		} else {
			failed++;
			System.out.println("  FAIL: " + msg);
		}
	}

	public static void main(String[] args) {
		// Lang 先行: Blkx.getload() 输出用 Lang 格式串, 不初始化会 NPE (惯例同其它 FM 测试)
		prog.i18n.Lang.initLang();

		File fmDir = new File("data/aces/gamedata/flightmodels/fm");
		if (!fmDir.isDirectory()) {
			System.out.println("SKIP: 项目内 data/ 不存在 (先运行 python script/build.py fmdata)");
			return;
		}

		File[] files = fmDir.listFiles();
		if (files == null || files.length == 0) {
			System.out.println("SKIP: data/aces/gamedata/flightmodels/fm 为空");
			return;
		}

		TreeSet<String> parsedHeli = new TreeSet<String>(); // 解析结果 = true 的机型
		TreeSet<String> textHeli = new TreeSet<String>();   // 文本任一非零值的机型
		TreeSet<String> textNoField = new TreeSet<String>(); // 无该字段的机型 (应降级 false)
		int parseErr = 0;

		for (File f : files) {
			String name = f.getName();
			if (!name.endsWith(".blkx"))
				continue;

			// 文本侧: 读原文判断 (对拍基准, 独立于解析器实现)
			boolean textHasField = false;
			boolean textNonZero = false;
			try {
				BufferedReader br = new BufferedReader(new FileReader(f));
				try {
					String line;
					while ((line = br.readLine()) != null) {
						String t = line.trim();
						if (!t.startsWith("VortexRingVFlowMult:"))
							continue;
						textHasField = true;
						// 取 '=' 后逗号前首数, 任一处 > 0 即直升机
						String val = t.substring(t.indexOf('=') + 1).trim();
						String first = val.split(",")[0].trim();
						try {
							if (Double.parseDouble(first) > 0)
								textNonZero = true;
						} catch (NumberFormatException e) {
							// 脏值按该处无效
						}
					}
				} finally {
					br.close();
				}
			} catch (Exception e) {
				System.out.println("  EXCEPTION(读文件): " + name + " -> " + e);
				failed++;
				continue;
			}

			// 解析侧: 生产路径等价构造 (doLoad=true)
			Blkx b;
			try {
				b = new Blkx(f.getPath(), name);
			} catch (Throwable t) {
				System.out.println("  EXCEPTION(解析): " + name + " -> " + t);
				failed++;
				continue;
			}
			if (!b.valid) {
				parseErr++;
				continue; // invalid 文件不在本测试范围 (TestFMAllBoundaries 已覆盖)
			}

			if (textNonZero)
				textHeli.add(name);
			if (!textHasField)
				textNoField.add(name);
			if (b.isHelicopter)
				parsedHeli.add(name);
		}

		System.out.println("=== isHelicopter 解析验证 ===");
		System.out.println("总机型 " + files.length + ", 解析直升机 " + parsedHeli.size()
				+ ", 文本非零 " + textHeli.size() + ", 无字段 " + textNoField.size()
				+ ", invalid 跳过 " + parseErr);

		// 1. 代表机型硬断言
		assertTrue(parsedHeli.contains("mi_24a.blkx"), "mi_24a (可收轮直升机, issue #65) 解析为 true");
		assertTrue(parsedHeli.contains("ka_50.blkx"), "ka_50 (共轴双旋翼, 双段非零) 解析为 true");
		assertTrue(parsedHeli.contains("uh_1b.blkx"), "uh_1b (单旋翼+尾桨, 仅一段非零) 解析为 true");
		assertTrue(!parsedHeli.contains("spitfire_f24.blkx"), "spitfire_f24 (固定翼) 解析为 false");
		assertTrue(!parsedHeli.contains("a6m2_zero.blkx"), "a6m2_zero (固定翼, 值恒 0,0) 解析为 false");
		assertTrue(!parsedHeli.contains("f_16aj.blkx"), "f_16aj (固定翼) 解析为 false");
		assertTrue(!parsedHeli.isEmpty(), "直升机集合非空 (data 健全性)");

		// 2. 全量对拍: 两个集合必须完全一致
		if (parsedHeli.equals(textHeli)) {
			assertTrue(true, "解析直升机集合与文本非零集合一致 (" + parsedHeli.size() + " 架)");
		} else {
			TreeSet<String> onlyParsed = new TreeSet<String>(parsedHeli);
			onlyParsed.removeAll(textHeli);
			TreeSet<String> onlyText = new TreeSet<String>(textHeli);
			onlyText.removeAll(parsedHeli);
			assertTrue(false, "对拍不一致: 仅解析 true=" + onlyParsed + ", 仅文本非零=" + onlyText);
		}

		// 3. 无该字段的机型必须降级 false (老格式 FM 全为固定翼; 名单随版本浮动不硬编码)
		if (textNoField.isEmpty()) {
			assertTrue(true, "全量机型均含 VortexRingVFlowMult 字段");
		} else {
			boolean allDefaultFalse = true;
			for (String n : textNoField) {
				if (parsedHeli.contains(n)) {
					allDefaultFalse = false;
					System.out.println("  字段缺失却解析为 true: " + n);
				}
			}
			assertTrue(allDefaultFalse, "字段缺失的 " + textNoField.size() + " 架机型均降级为 false");
		}

		System.out.println();
		if (failed > 0) {
			System.out.println("RESULT: FAILED (" + failed + " failures)");
			System.exit(1);
		}
		System.out.println("RESULT: PASSED (" + passed + " assertions)");
	}
}
