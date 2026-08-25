package prog.fm;

import prog.util.PistonPowerModel;

/**
 * 不可变的 FM 句柄 —— "当前飞机的 FM 加载结果"的单一真相（P2 重构，取代 Controller 上
 * 分散的 Blkx/loadedFMName/identifiedFMName/failedFMName 四个手动同步的变量）。
 *
 * <p>一个句柄完整描述一次加载的结果：机型名、状态、解析好的 {@link parser.Blkx}、
 * 以及由 FM 派生的功率/推力缓存。换机 = 换一个新句柄实例，旧句柄保持不可变，
 * 不存在"半新半旧"的中间态。
 *
 * <p><b>共享会话状态说明</b>：{@code blkx.engLoad} 是 Service 线程在飞行过程中就地改写的
 * 共享会话状态（水温/油温计时等）。本类刻意<b>不拷贝</b>这层状态——因为换机必然产生
 * 新的 FMHandle → 新的 Blkx 实例，"换机 = 新实例"的语义天然保证会话状态不会串机，
 * 无需额外防御。
 *
 * <p>构造只经静态工厂，字段全 final，线程安全（volatile 发布由 {@link FMManager} 负责）。
 */
public final class FMHandle {

	/**
	 * 哨兵句柄：未识别到机型时的初始值。字段值恒为
	 * name=null / status=UNRESOLVED / blkx=null / 功率推力全 0。
	 */
	public static final FMHandle UNRESOLVED = new FMHandle(null, FMStatus.UNRESOLVED, null, 0, 0, null);

	/** 规范化小写机型名（toLowerCase+trim）；UNRESOLVED 时为 null */
	public final String name;
	/** 加载结果状态 */
	public final FMStatus status;
	/** 解析完成的 FM 对象；仅 {@link FMStatus#READY} 时非 null */
	public final parser.Blkx blkx;
	/** 活塞机 WEP 峰值功率（hp，已乘引擎数）；非活塞/未就绪为 0 */
	public final double peakWepPower;
	/** 喷气机加力峰值推力（kgf）；活塞机/未就绪为 0 */
	public final double peakThrust;
	/** 活塞机多级增压器参数；喷气机/未就绪为 null */
	public final PistonPowerModel.CompressorStageParams[] compressorStages;

	private FMHandle(String name, FMStatus status, parser.Blkx blkx, double peakWepPower, double peakThrust,
			PistonPowerModel.CompressorStageParams[] compressorStages) {
		this.name = name;
		this.status = status;
		this.blkx = blkx;
		this.peakWepPower = peakWepPower;
		this.peakThrust = peakThrust;
		this.compressorStages = compressorStages;
	}

	/** 加载成功句柄（仅 READY 允许携带 blkx） */
	public static FMHandle ready(String name, parser.Blkx blkx, double peakWepPower, double peakThrust,
			PistonPowerModel.CompressorStageParams[] compressorStages) {
		return new FMHandle(name, FMStatus.READY, blkx, peakWepPower, peakThrust, compressorStages);
	}

	/** 中央文件确认不存在 */
	public static FMHandle missing(String name) {
		return new FMHandle(name, FMStatus.MISSING, null, 0, 0, null);
	}

	/**
	 * 非飞机载具（陆战坦克/军舰等，type 带路径前缀如 "tankmodels/..."）。
	 * FM 不适用而非数据缺失：不进负缓存、不触发缺失 toast（见 {@link #isMissingLike()}）。
	 */
	public static FMHandle notAircraft(String name) {
		return new FMHandle(name, FMStatus.NOT_AIRCRAFT, null, 0, 0, null);
	}

	/** 存在但解析失败（物理文件缺失 / 解析异常） */
	public static FMHandle corrupt(String name) {
		return new FMHandle(name, FMStatus.CORRUPT, null, 0, 0, null);
	}

	/**
	 * 是否持有可用的 FM 数据。
	 * 注意不要直接判 {@code status == READY} 以外的字段——blkx 为 null 的句柄
	 * （UNRESOLVED/LOADING/MISSING/CORRUPT）对调用方一律视为"无 FM"。
	 */
	public boolean hasFM() {
		return status == FMStatus.READY && blkx != null;
	}

	/**
	 * 是否属于"缺失类"状态（MISSING 或 CORRUPT）。
	 * 这类结果会进 {@link FMManager} 的负缓存，是 issue #55 死循环的根治点；
	 * Controller 也以本方法为闸门弹缺失 toast。
	 * 注意 NOT_AIRCRAFT 刻意不在其中——坦克/军舰不是数据缺失，不该被当飞机提示。
	 */
	public boolean isMissingLike() {
		return status == FMStatus.MISSING || status == FMStatus.CORRUPT;
	}

	@Override
	public String toString() {
		return "FMHandle[" + status + " " + name + "]";
	}
}
