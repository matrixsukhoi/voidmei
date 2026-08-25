import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.function.BooleanSupplier;

import prog.fm.FMDataPaths;
import prog.fm.FMHandle;
import prog.fm.FMLoader;
import prog.fm.FMManager;
import prog.fm.FMStatus;
import prog.i18n.Lang;

/**
 * FMManager/FMLoader 白盒测试 —— issue #55 死循环回归（P2 单一真相源架构）
 *
 * 在临时目录合成最小中央/物理 blkx 文件（不依赖真机 data/，CI 可跑）：
 *   plane1/plane2 —— central + physical 齐全，可加载到 READY
 *   badplane     —— 只有 central（物理文件缺失）→ CORRUPT
 *   ghost        —— 什么都不放 → MISSING
 *
 * 核心回归点：缺失机型反复 identify 不再触发磁盘加载（负缓存），
 * 取代旧架构 failedFMName 手动同步失效导致的每秒 ~20 次解析风暴。
 *
 * 运行方式: python script/build.py test fmstore
 */
public class TestFMStore {

	private static int passed = 0;
	private static int failed = 0;

	/** 轮询等待的超时上限（合成文件很小，正常毫秒级完成；10s 是宽松上界） */
	private static final long WAIT_TIMEOUT_MS = 10_000;

	public static void main(String[] args) throws Exception {
		System.out.println("=== FMManager/FMLoader 死循环回归测试 ===\n");

		// Blkx 构造器依赖 Lang.noblkx 等字符串，先初始化语言
		Lang.initLang();

		Path tmpRoot = Files.createTempDirectory("voidmei_fmtest");
		try {
			setupSyntheticData(tmpRoot);
			FMDataPaths.setDataRoot(tmpRoot.toString());

			FMManager m = FMManager.getInstance();

			testIdentifyDedup(m);
			testNegativeCacheNoStorm(m);
			testCorruptAlsoCached(m);
			testClearTargetKeepsHandle(m);
			testRateGuardAndLiveness(m);
			testNotAircraftShortCircuit(m);
			testReset(m);
			testConcurrentIdentify(m);
		} finally {
			// 还原全局状态，保证不影响同 JVM 后续逻辑（测试进程独立，此为双保险）
			FMDataPaths.setDataRoot("./data");
			FMManager.getInstance().reset();
			rmtree(tmpRoot.toFile());
		}

		System.out.println("\n=== 测试结果 ===");
		System.out.println("通过: " + passed);
		System.out.println("失败: " + failed);

		if (failed > 0) {
			System.exit(1);
		}
	}

	// ==================== 合成数据 ====================

	private static void setupSyntheticData(Path tmpRoot) throws Exception {
		Path fmDir = tmpRoot.resolve("aces").resolve("gamedata").resolve("flightmodels");
		Path fmSub = fmDir.resolve("fm");
		Files.createDirectories(fmSub);

		// 可加载机型: central 指向 fm/<name>.blk, 物理文件存在
		writeCentral(fmDir, "plane1");
		writeCentral(fmDir, "plane2");
		writePhysical(fmSub, "plane1");
		writePhysical(fmSub, "plane2");

		// CORRUPT 机型: central 在库但物理文件缺失
		writeCentral(fmDir, "badplane");

		// ghost: 什么都不写 → MISSING
	}

	/** 最小中央文件 —— 只需 getlastone("fmfile") 能命中（参考真机文件头 fmFile:t = "fm/xxx.blk"） */
	private static void writeCentral(Path fmDir, String name) throws Exception {
		String content = "model:t = \"" + name + "\"\nfmFile:t = \"fm/" + name + ".blk\"\n";
		Files.write(fmDir.resolve(name + ".blkx"), content.getBytes("UTF-8"));
	}

	/**
	 * 最小物理 FM —— 非空且不以 '{' 开头即可全量解析：
	 * getload 对缺失字段全部按 0 处理（无 Jet/Compressor 块 → 按喷气形态、compNumSteps=0，
	 * extractStages 返回 null、peakThrust=0），最终 valid=true → READY。
	 */
	private static void writePhysical(Path fmSub, String name) throws Exception {
		String content = "synthetic-fm:t = \"" + name + "\"\nEmptyMass:r = 1000\nWingspan:r = 11\n";
		Files.write(fmSub.resolve(name + ".blkx"), content.getBytes("UTF-8"));
	}

	// ==================== 测试用例 ====================

	/**
	 * 用例① identify 去重：同 target 调 1000 次，加载任务只执行一次。
	 * （旧架构等价场景：getBlkx 每 50ms 被调 → 每次都进 loadFMData）
	 */
	private static void testIdentifyDedup(FMManager m) throws Exception {
		System.out.println("-- 用例① identify 去重 --");
		m.reset();
		FMLoader.resetLoadCount();

		for (int i = 0; i < 1000; i++) {
			m.identify("plane1");
		}

		boolean ok = waitFor(() -> m.current().status == FMStatus.READY
				&& "plane1".equals(m.current().name));
		check(ok, "1000 次 identify 后应到达 READY(plane1)");
		check(FMLoader.getLoadCount() == 1,
				"FMLoader.load 只应执行 1 次 (实际 " + FMLoader.getLoadCount() + ")");
		check(m.current().hasFM() && m.current().blkx != null, "READY 句柄应携带 blkx");
		check("plane1".equals(m.currentTargetName()), "目标名应规范化为小写 plane1");
	}

	/**
	 * 用例② 死循环核心回归：identify 不存在的机型 1000 次 ——
	 * 第一次加载落 MISSING 进负缓存后，其余 999 次零磁盘加载。
	 */
	private static void testNegativeCacheNoStorm(FMManager m) throws Exception {
		System.out.println("-- 用例② 负缓存防风暴 (核心死循环回归) --");
		m.reset();
		FMLoader.resetLoadCount();

		m.identify("ghost"); // 第一次: 发任务 → FMLoader.load → MISSING → 负缓存
		boolean ok = waitFor(() -> m.current().status == FMStatus.MISSING && !m.isLoading());
		check(ok, "首次 identify(ghost) 后应落定 MISSING");
		check(FMLoader.getLoadCount() == 1, "ghost 只应真正加载 1 次");

		long afterFirst = FMLoader.getLoadCount();
		for (int i = 0; i < 999; i++) {
			m.identify("ghost"); // 全部应被负缓存拦截
		}
		Thread.sleep(300); // 给潜在误发任务留出现形时间
		check(FMLoader.getLoadCount() == afterFirst,
				"后续 999 次 identify 不应产生新加载 (期望 " + afterFirst + ", 实际 "
						+ FMLoader.getLoadCount() + ")");
		check(m.current().status == FMStatus.MISSING, "状态应稳定停留在 MISSING");
		check(!m.isLoading(), "不应有在途任务");
	}

	/** 用例②b CORRUPT 同样进负缓存（central 在库但物理文件缺失） */
	private static void testCorruptAlsoCached(FMManager m) throws Exception {
		System.out.println("-- 用例②b CORRUPT 也进负缓存 --");
		m.reset();
		FMLoader.resetLoadCount();

		m.identify("badplane");
		boolean ok = waitFor(() -> m.current().isMissingLike() && !m.isLoading());
		check(ok, "identify(badplane) 应落定 missing-like (MISSING/CORRUPT)");
		check(m.current().status == FMStatus.CORRUPT, "物理文件缺失应为 CORRUPT");

		long afterFirst = FMLoader.getLoadCount();
		for (int i = 0; i < 500; i++) {
			m.identify("badplane");
		}
		Thread.sleep(200);
		check(FMLoader.getLoadCount() == afterFirst, "CORRUPT 后续 identify 不应产生新加载");
	}

	/** 用例③ clearTarget 保留句柄：下次同名 identify 秒开（零加载） */
	private static void testClearTargetKeepsHandle(FMManager m) throws Exception {
		System.out.println("-- 用例③ clearTarget 保留句柄 --");
		m.reset();
		FMLoader.resetLoadCount();

		m.identify("plane1");
		boolean ok = waitFor(() -> m.current().status == FMStatus.READY);
		check(ok, "前置: plane1 到达 READY");

		m.clearTarget();
		check(m.currentTargetName() == null, "clearTarget 后目标应为 null");
		check(m.current().hasFM(), "clearTarget 后句柄应保留 (下次秒开)");

		long before = FMLoader.getLoadCount();
		m.identify("plane1"); // 句柄已在 → 恢复目标即可，零成本
		check("plane1".equals(m.currentTargetName()), "identify 后目标恢复为 plane1");
		check(m.current().hasFM(), "句柄持续可用");
		Thread.sleep(200);
		check(FMLoader.getLoadCount() == before, "秒开路径不应触发重新加载");
	}

	/**
	 * 用例④ 速率护栏与活性：A→B→A 快速切换（60s 护栏窗口内）不得卡死在 B 的句柄上。
	 * 护栏只在 current 正是该机型时拦截；目标切走又切回时放行重载（正确性优先于限速）。
	 */
	private static void testRateGuardAndLiveness(FMManager m) throws Exception {
		System.out.println("-- 用例④ 速率护栏与回切活性 --");
		m.reset();
		FMLoader.resetLoadCount();

		m.identify("plane1");
		check(waitFor(() -> "plane1".equals(m.current().name) && m.current().hasFM()), "plane1 READY");

		m.identify("plane2");
		check(waitFor(() -> "plane2".equals(m.current().name) && m.current().hasFM()), "plane2 READY");

		m.identify("plane1"); // 60s 内已尝试过 plane1 —— 必须放行, 否则卡死在 plane2
		boolean ok = waitFor(() -> "plane1".equals(m.current().name) && m.current().hasFM());
		check(ok, "A→B→A 回切后应回到 READY(plane1), 不得卡在 plane2");
		check(FMLoader.getLoadCount() == 3,
				"两次首载 + 一次回切重载 = 3 次执行 (实际 " + FMLoader.getLoadCount() + ")");
	}

	/**
	 * 用例④b 非飞机载具短路：坦克 type（"tankmodels/..." 路径前缀名）直接落
	 * NOT_AIRCRAFT——零磁盘加载、不进负缓存、飞机↔坦克往返行为正确。
	 * 回归: 陆战时误把坦克当"FM 缺失的新飞机"弹 toast + 白做磁盘查找。
	 */
	private static void testNotAircraftShortCircuit(FMManager m) throws Exception {
		System.out.println("-- 用例④b 非飞机载具短路 (陆战坦克) --");
		m.reset();
		FMLoader.resetLoadCount();

		// 同步落定: 不经过 loader 线程
		m.identify("tankmodels/us_n4a3e8_76_sherman");
		check(m.current().status == FMStatus.NOT_AIRCRAFT, "坦克应立即落定 NOT_AIRCRAFT");
		check(!m.current().isMissingLike(), "不属于 missing-like (不弹缺失 toast)");
		check(!m.current().hasFM(), "无 FM, HUD 走降级");
		check(FMLoader.getLoadCount() == 0, "不应触发任何磁盘加载");
		check(!m.isLoading(), "无在途任务");

		// 重复 identify 同一坦克: 目标去重拦截, 仍零加载
		for (int i = 0; i < 100; i++)
			m.identify("tankmodels/us_n4a3e8_76_sherman");
		check(FMLoader.getLoadCount() == 0, "重复 identify 同一坦克仍零加载");

		// 飞机 → 坦克 → 换坦克 → 回飞机: 往返行为正确
		m.identify("plane1");
		check(waitFor(() -> m.current().hasFM()), "前置: plane1 到达 READY");
		long loadsBefore = FMLoader.getLoadCount();

		m.identify("tankmodels/us_n4a3e8_76_sherman");
		check(m.current().status == FMStatus.NOT_AIRCRAFT && !m.isLoading(),
				"飞机→坦克: 句柄应让位为 NOT_AIRCRAFT");
		m.identify("tankmodels/germ_panther_ii");
		check(m.current().status == FMStatus.NOT_AIRCRAFT
				&& "tankmodels/germ_panther_ii".equals(m.current().name),
				"坦克→坦克: 直接换 NOT_AIRCRAFT 句柄");
		check(FMLoader.getLoadCount() == loadsBefore, "坦克切换全程零加载");

		m.identify("plane1");
		check(waitFor(() -> m.current().hasFM()), "坦克→飞机: 应重新加载回 READY(plane1)");
	}

	/** 用例⑤ reset：清一切（含负缓存），停掉 pending 任务 */
	private static void testReset(FMManager m) throws Exception {
		System.out.println("-- 用例⑤ reset --");
		m.identify("ghost");
		check(waitFor(() -> m.current().status == FMStatus.MISSING), "前置: ghost 已进负缓存");

		m.reset();
		check(m.current().status == FMStatus.UNRESOLVED, "reset 后 current 应为 UNRESOLVED");
		check(m.currentTargetName() == null, "reset 后目标应为 null");
		check(!m.isLoading(), "reset 后无在途任务");

		FMLoader.resetLoadCount();
		m.identify("ghost"); // 负缓存已清 → 应重新发任务
		boolean ok = waitFor(() -> m.current().status == FMStatus.MISSING);
		check(ok, "reset 清负缓存后 ghost 可重新尝试 (并再次落 MISSING)");
		check(FMLoader.getLoadCount() == 1, "reset 后应执行 1 次新加载");
	}

	/**
	 * 用例⑥ 并发 identify：两个线程各 50 次交替识别不同机型，
	 * 最终 current 必须与 currentTarget 一致（单线程 loader 串行 + 任务过期校验保证）。
	 */
	private static void testConcurrentIdentify(FMManager m) throws Exception {
		System.out.println("-- 用例⑥ 并发 identify 最终一致 --");
		m.reset();
		FMLoader.resetLoadCount();

		final FMManager mgr = m;
		Thread t1 = new Thread(() -> {
			for (int i = 0; i < 50; i++)
				mgr.identify("plane1");
		}, "identify-plane1");
		Thread t2 = new Thread(() -> {
			for (int i = 0; i < 50; i++)
				mgr.identify("plane2");
		}, "identify-plane2");
		t1.start();
		t2.start();
		t1.join();
		t2.join();

		boolean ok = waitFor(() -> !mgr.isLoading() && mgr.currentTargetName() != null
				&& mgr.currentTargetName().equals(mgr.current().name));
		check(ok, "任务清空后 current 应与最后 target 一致 (target="
				+ mgr.currentTargetName() + ", current=" + mgr.current().name + ")");
		long loads = FMLoader.getLoadCount();
		check(loads >= 1 && loads <= 100,
				"加载次数应远小于 identify 次数且非零 (实际 " + loads + ")");

		// 无论最终是谁，句柄必须完整可用
		check(mgr.current().hasFM(), "最终句柄应为 READY 且携带 blkx");
	}

	// ==================== 工具 ====================

	/** 轮询等待条件成立（20ms 间隔），超时返回最后一次求值结果 */
	private static boolean waitFor(BooleanSupplier cond) throws InterruptedException {
		long deadline = System.currentTimeMillis() + WAIT_TIMEOUT_MS;
		boolean v = cond.getAsBoolean();
		while (!v && System.currentTimeMillis() < deadline) {
			Thread.sleep(20);
			v = cond.getAsBoolean();
		}
		return v;
	}

	private static void check(boolean cond, String desc) {
		if (cond) {
			System.out.println("  [通过] " + desc);
			passed++;
		} else {
			System.out.println("  [失败] " + desc);
			failed++;
		}
	}

	private static void rmtree(File f) {
		if (f == null || !f.exists())
			return;
		File[] children = f.listFiles();
		if (children != null) {
			for (File c : children)
				rmtree(c);
		}
		f.delete();
	}
}
