import prog.fm.FMHandle;
import prog.fm.FMStatus;
import prog.util.PistonPowerModel;

/**
 * FMHandle 白盒测试 —— 六态语义 / hasFM / isMissingLike / UNRESOLVED 哨兵字段值
 *
 * 运行方式: python script/build.py test fmhandle
 */
public class TestFMHandle {

	private static int passed = 0;
	private static int failed = 0;

	public static void main(String[] args) {
		System.out.println("=== FMHandle 测试 ===\n");

		testUnresolvedSentinel();
		testReadyHandle();
		testMissingHandle();
		testCorruptHandle();
		testNotAircraftHandle();
		testMissingLikeSemantics();

		System.out.println("\n=== 测试结果 ===");
		System.out.println("通过: " + passed);
		System.out.println("失败: " + failed);

		if (failed > 0) {
			System.exit(1);
		}
	}

	private static void testUnresolvedSentinel() {
		System.out.println("-- UNRESOLVED 哨兵字段值测试 --");
		FMHandle h = FMHandle.UNRESOLVED;
		check(h.name == null, "哨兵 name 应为 null");
		check(h.status == FMStatus.UNRESOLVED, "哨兵 status 应为 UNRESOLVED");
		check(h.blkx == null, "哨兵 blkx 应为 null");
		check(h.peakWepPower == 0.0, "哨兵 peakWepPower 应为 0");
		check(h.peakThrust == 0.0, "哨兵 peakThrust 应为 0");
		check(h.compressorStages == null, "哨兵 compressorStages 应为 null");
		check(!h.hasFM(), "哨兵 hasFM 应为 false");
		check(!h.isMissingLike(), "哨兵 isMissingLike 应为 false");
	}

	private static void testReadyHandle() {
		System.out.println("-- READY 句柄语义测试 --");
		// dummy Blkx: 路径不存在的文件 → 对象非 null 即可 (hasFM 只看 status 与 blkx 非空)
		parser.Blkx dummy = new parser.Blkx("__no_such_file__.blkx", "dummy");
		PistonPowerModel.CompressorStageParams[] stages = new PistonPowerModel.CompressorStageParams[1];
		stages[0] = new PistonPowerModel.CompressorStageParams();
		FMHandle h = FMHandle.ready("plane1", dummy, 1850.5, 0, stages);

		check(h.status == FMStatus.READY, "ready() 工厂 status 应为 READY");
		check("plane1".equals(h.name), "name 应保留规范化机型名");
		check(h.blkx == dummy, "blkx 应携带解析对象");
		check(h.peakWepPower == 1850.5, "peakWepPower 应保留传入值");
		check(h.peakThrust == 0.0, "活塞机 peakThrust 应为 0");
		check(h.compressorStages == stages, "compressorStages 应保留传入引用");
		check(h.hasFM(), "READY 且 blkx 非空 → hasFM 为 true");
		check(!h.isMissingLike(), "READY 不是 missing-like");

		// 喷气机形态: stages=null, peakThrust>0
		FMHandle jet = FMHandle.ready("me262", dummy, 0, 1800, null);
		check(jet.hasFM() && jet.compressorStages == null && jet.peakThrust == 1800,
				"喷气机句柄: stages null / thrust 1800");
	}

	private static void testMissingHandle() {
		System.out.println("-- MISSING 句柄语义测试 --");
		FMHandle h = FMHandle.missing("ghost");
		check(h.status == FMStatus.MISSING, "missing() 工厂 status 应为 MISSING");
		check("ghost".equals(h.name), "name 应为机型名");
		check(h.blkx == null, "MISSING 不携带 blkx");
		check(h.peakWepPower == 0 && h.peakThrust == 0, "MISSING 功率/推力应为 0");
		check(!h.hasFM(), "MISSING hasFM 应为 false");
		check(h.isMissingLike(), "MISSING isMissingLike 应为 true");
	}

	private static void testCorruptHandle() {
		System.out.println("-- CORRUPT 句柄语义测试 --");
		FMHandle h = FMHandle.corrupt("badplane");
		check(h.status == FMStatus.CORRUPT, "corrupt() 工厂 status 应为 CORRUPT");
		check("badplane".equals(h.name), "name 应为机型名");
		check(h.blkx == null, "CORRUPT 不携带 blkx");
		check(!h.hasFM(), "CORRUPT hasFM 应为 false");
		check(h.isMissingLike(), "CORRUPT isMissingLike 应为 true");
	}

	/** NOT_AIRCRAFT: 非飞机载具（坦克/军舰）——无 FM 但也不是数据缺失，不该弹缺失 toast */
	private static void testNotAircraftHandle() {
		System.out.println("-- NOT_AIRCRAFT 句柄语义测试 (陆战坦克) --");
		FMHandle h = FMHandle.notAircraft("tankmodels/us_n4a3e8_76_sherman");
		check(h.status == FMStatus.NOT_AIRCRAFT, "notAircraft() 工厂 status 应为 NOT_AIRCRAFT");
		check("tankmodels/us_n4a3e8_76_sherman".equals(h.name), "name 应保留原始载具名");
		check(h.blkx == null, "NOT_AIRCRAFT 不携带 blkx");
		check(!h.hasFM(), "NOT_AIRCRAFT hasFM 应为 false (HUD 走降级)");
		check(!h.isMissingLike(), "NOT_AIRCRAFT 不是 missing-like (不进负缓存/不弹缺失 toast)");
	}

	private static void testMissingLikeSemantics() {
		System.out.println("-- isMissingLike 全枚举覆盖测试 --");
		check(!FMHandle.UNRESOLVED.isMissingLike(), "UNRESOLVED 不是 missing-like");
		check(FMHandle.missing("x").blkx == null, "MISSING 永不携带 blkx");
		check(FMHandle.corrupt("x").blkx == null, "CORRUPT 永不携带 blkx");
		check(FMHandle.corrupt("x").isMissingLike(), "CORRUPT 属于 missing-like");
		check(FMHandle.missing("x").isMissingLike(), "MISSING 属于 missing-like");
		check(!FMHandle.notAircraft("x/y").isMissingLike(), "NOT_AIRCRAFT 不属于 missing-like");
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
}
