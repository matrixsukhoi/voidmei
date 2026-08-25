package prog.fm;

/**
 * FM（飞行数据包）加载状态机的六种状态（P2 单一真相源架构，issue #55 死循环重构）。
 *
 * <p>状态语义：
 * <ul>
 *   <li>{@link #UNRESOLVED} —— 尚未识别到机型（没有 live 数据也没有配置默认机），
 *       一切从这里开始。</li>
 *   <li>{@link #LOADING} —— 后台线程正在加载中；期间 {@link FMManager#current()}
 *       仍返回旧句柄（平滑过渡），本状态只表达"有任务在途"。</li>
 *   <li>{@link #READY} —— FM 解析成功，{@link FMHandle#blkx} 可用。</li>
 *   <li>{@link #MISSING} —— 中央文件（&lt;dataRoot&gt;/aces/gamedata/flightmodels/&lt;name&gt;.blkx）
 *       不存在，确认该机型不在数据库中。</li>
 *   <li>{@link #CORRUPT} —— 中央文件存在但后续解析失败（物理 fm 文件缺失 / 构造异常 /
 *       getAllplotdata 抛错等）。</li>
 *   <li>{@link #NOT_AIRCRAFT} —— 识别到的是非飞机载具（陆战坦克/军舰等，type 带
 *       "tankmodels/" 之类路径前缀）。FM 数据库只有 flightmodels，这类目标不是
 *       "数据缺失"而是"根本不适用"：不发加载任务、不进负缓存、不弹缺失 toast，
 *       HUD 端按 hasFM()=false 正常降级。</li>
 * </ul>
 *
 * <p>{@link #MISSING} 与 {@link #CORRUPT} 统称 "missing-like"（见
 * {@link FMHandle#isMissingLike()}），二者都会进入 {@link FMManager} 的负缓存，
 * 杜绝旧架构"每次轮询都重试坏机型"的风暴。{@link #NOT_AIRCRAFT} 刻意不属于
 * missing-like：无数据问题、无需重试、不该打扰用户。
 */
public enum FMStatus {
	UNRESOLVED,
	LOADING,
	READY,
	MISSING,
	CORRUPT,
	NOT_AIRCRAFT
}
