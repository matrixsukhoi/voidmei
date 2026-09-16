import parser.Blkx;
import prog.i18n.Lang;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.util.TreeSet;

/**
 * hasFlapsControl 解析验证 —— 遍历 data/ 全部物理 FM 文件:
 *
 * 1. 代表机型硬断言: f_16xl/j_10a/直升机 = false (无襟翼), p-51d-5/f_16aj/a6m2 = true
 * 2. 全量对拍: 解析结果 false 的集合 == 文本里显式 "hasFlapsControl:b = false" 的集合
 *    (防点路径解析写错; 数量随游戏版本浮动, 只比集合不绑死数值)
 * 3. 字段缺失机 (如 i_180_event03) 必须降级为 true (缺省语义)
 *
 * 背景: 无襟翼机不能用 FlapsDestructionIndSpeed/FlapsPolar 有无判断——
 * 模板数据齐全 (f_16xl 两套极线都有), 唯一权威信号是 AvailableControls.hasFlapsControl。
 *
 * Run with: python script/build.py test flaps-ctrl  (data/ 缺失时由 build.py 跳过)
 */
public class TestHasFlapsControl {

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

		TreeSet<String> parsedFalse = new TreeSet<String>();  // 解析结果 = false 的机型
		TreeSet<String> textFalse = new TreeSet<String>();    // 文本显式 false 的机型
		TreeSet<String> textMissing = new TreeSet<String>();  // 字段缺失的机型 (应降级 true)

		for (File f : files) {
			String name = f.getName();
			if (!name.endsWith(".blkx"))
				continue;

			// 文本侧: 读原文判断字段落点 (对拍基准, 独立于解析器实现)
			boolean textHasFalse = false;
			boolean textHasField = false;
			try {
				BufferedReader br = new BufferedReader(new FileReader(f));
				try {
					String line;
					while ((line = br.readLine()) != null) {
						String t = line.trim();
						if (t.startsWith("hasFlapsControl:")) {
							textHasField = true;
							if (t.equals("hasFlapsControl:b = false")) {
								textHasFalse = true;
								break; // 同块只出现一次, 找到即止
							}
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
			if (!b.valid)
				continue; // invalid 文件不在本测试范围 (TestFMAllBoundaries 已覆盖)

			if (textHasFalse)
				textFalse.add(name);
			if (!textHasField)
				textMissing.add(name);
			if (!b.hasFlapsControl)
				parsedFalse.add(name);
		}

		System.out.println("=== hasFlapsControl 解析验证 ===");
		System.out.println("总机型 " + files.length + ", 解析 false " + parsedFalse.size()
				+ ", 文本 false " + textFalse.size() + ", 字段缺失 " + textMissing.size());

		// 1. 代表机型硬断言
		assertTrue(parsedFalse.contains("f_16xl.blkx"), "f_16xl (无襟翼) 解析为 false");
		assertTrue(parsedFalse.contains("j_10a.blkx"), "j_10a (无襟翼) 解析为 false");
		assertTrue(parsedFalse.contains("ka_50.blkx"), "ka_50 (直升机) 解析为 false");
		assertTrue(!parsedFalse.contains("p-51d-5.blkx"), "p-51d-5 (有襟翼) 解析为 true");
		assertTrue(!parsedFalse.contains("f_16aj.blkx"), "f_16aj (有襟翼) 解析为 true");
		assertTrue(!parsedFalse.contains("a6m2_zero.blkx"), "a6m2_zero (有襟翼) 解析为 true");

		// 2. 全量对拍: 两个集合必须完全一致
		if (parsedFalse.equals(textFalse)) {
			assertTrue(true, "解析 false 集合与文本 false 集合一致 (" + parsedFalse.size() + " 架)");
		} else {
			TreeSet<String> onlyParsed = new TreeSet<String>(parsedFalse);
			onlyParsed.removeAll(textFalse);
			TreeSet<String> onlyText = new TreeSet<String>(textFalse);
			onlyText.removeAll(parsedFalse);
			assertTrue(false, "对拍不一致: 仅解析 false=" + onlyParsed + ", 仅文本 false=" + onlyText);
		}

		// 3. 字段缺失机必须降级 true (缺省语义; 名单随版本浮动只验证不硬编码)
		if (textMissing.isEmpty()) {
			assertTrue(true, "全量机型均含 hasFlapsControl 字段");
		} else {
			boolean allDefaultTrue = true;
			for (String n : textMissing) {
				if (parsedFalse.contains(n)) {
					allDefaultTrue = false;
					System.out.println("  字段缺失却解析为 false: " + n);
				}
			}
			assertTrue(allDefaultTrue, "字段缺失的 " + textMissing.size() + " 架机型均降级为 true: " + textMissing);
		}

		System.out.println();
		if (failed > 0) {
			System.out.println("RESULT: FAILED (" + failed + " failures)");
			System.exit(1);
		}
		System.out.println("RESULT: PASSED (" + passed + " assertions)");
	}
}
