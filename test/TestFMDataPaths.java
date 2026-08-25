import java.io.File;

import prog.fm.FMDataPaths;

/**
 * FMDataPaths 白盒测试 —— 路径拼装 / 小写规范化 / setDataRoot 注入
 *
 * 纯字符串断言，无需 data/ 目录存在。
 * 运行方式: python script/build.py test fmpaths
 */
public class TestFMDataPaths {

	private static int passed = 0;
	private static int failed = 0;

	public static void main(String[] args) {
		System.out.println("=== FMDataPaths 测试 ===\n");

		try {
			testDefaultRoot();
			testCentralFileNormalization();
			testPhysicalFile();
			testFmDirAndVersionFile();
			testSetDataRootInjection();
		} finally {
			// 还原默认根目录，避免影响同 JVM 内后续逻辑
			FMDataPaths.setDataRoot("./data");
		}

		System.out.println("\n=== 测试结果 ===");
		System.out.println("通过: " + passed);
		System.out.println("失败: " + failed);

		if (failed > 0) {
			System.exit(1);
		}
	}

	/** 路径统一为 '/' 分隔，规避 Windows/Linux 分隔符差异 */
	private static String norm(File f) {
		return f.getPath().replace('\\', '/');
	}

	private static void testDefaultRoot() {
		System.out.println("-- 默认根目录测试 --");
		// 程序约定: repo 即工作区, data 在项目根
		assertEquals("./data", FMDataPaths.getDataRoot(), "默认数据根目录应为 ./data");
	}

	private static void testCentralFileNormalization() {
		System.out.println("-- 中央文件路径与小写规范化测试 --");
		assertEquals("./data/aces/gamedata/flightmodels/spitfire_f24.blkx",
				norm(FMDataPaths.centralFile("spitfire_f24")), "小写机型名直接拼接");

		// 大小写规范化: 任意大小写输入都归一到小写 (匹配游戏侧命名约定)
		assertEquals("./data/aces/gamedata/flightmodels/spitfire_f24.blkx",
				norm(FMDataPaths.centralFile("Spitfire_F24")), "大写输入应规范化为小写");
		assertEquals("./data/aces/gamedata/flightmodels/spitfire_f24.blkx",
				norm(FMDataPaths.centralFile("SPITFIRE_F24")), "全大写输入应规范化为小写");

		// 统一小写 .blkx 扩展名 (旧代码 ".Blkx" 仅 Windows 大小写不敏感下碰巧可用)
		String p = norm(FMDataPaths.centralFile("abc"));
		if (p.endsWith(".blkx") && !p.endsWith(".Blkx")) {
			pass("扩展名统一为小写 .blkx");
		} else {
			fail("扩展名应为小写 .blkx, 实际: " + p);
		}
	}

	private static void testPhysicalFile() {
		System.out.println("-- 物理 FM 文件路径测试 --");
		// physicalFile 接收带 x 的相对路径 (与 FMLoader 调用约定一致)
		assertEquals("./data/aces/gamedata/flightmodels/fm/spitfire_f24.blkx",
				norm(FMDataPaths.physicalFile("fm/spitfire_f24.blkx")), "物理文件 = fmDir + 相对路径");
	}

	private static void testFmDirAndVersionFile() {
		System.out.println("-- 目录与版本文件路径测试 --");
		assertEquals("./data/aces/gamedata/flightmodels",
				norm(FMDataPaths.fmDir()), "fmDir 应为 <root>/aces/gamedata/flightmodels");
		assertEquals("./data/aces/version",
				norm(FMDataPaths.versionFile()), "versionFile 应为 <root>/aces/version");
	}

	private static void testSetDataRootInjection() {
		System.out.println("-- setDataRoot 注入测试 --");
		FMDataPaths.setDataRoot("testroot");
		assertEquals("testroot", FMDataPaths.getDataRoot(), "getDataRoot 应返回注入值");
		assertEquals("testroot/aces/gamedata/flightmodels/plane1.blkx",
				norm(FMDataPaths.centralFile("Plane1")), "注入后所有路径以新根为准");
		assertEquals("testroot/aces/version",
				norm(FMDataPaths.versionFile()), "注入后 versionFile 跟随新根");

		// 再注入一次验证可重复切换 (测试套件间隔离的基础)
		FMDataPaths.setDataRoot("otherroot");
		assertEquals("otherroot/aces/gamedata/flightmodels/plane1.blkx",
				norm(FMDataPaths.centralFile("plane1")), "二次注入应生效");
	}

	private static void assertEquals(String expected, String actual, String desc) {
		if (expected.equals(actual)) {
			pass(desc);
		} else {
			fail(desc + " —— 期望: " + expected + ", 实际: " + actual);
		}
	}

	private static void pass(String desc) {
		System.out.println("  [通过] " + desc);
		passed++;
	}

	private static void fail(String desc) {
		System.out.println("  [失败] " + desc);
		failed++;
	}
}
