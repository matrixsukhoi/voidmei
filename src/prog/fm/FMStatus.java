package prog.fm;

/**
 * FM（飞行数据包）加载状态机的五种状态（P2 单一真相源架构，issue #55 死循环重构）。
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
 * </ul>
 *
 * <p>{@link #MISSING} 与 {@link #CORRUPT} 统称 "missing-like"（见
 * {@link FMHandle#isMissingLike()}），二者都会进入 {@link FMManager} 的负缓存，
 * 杜绝旧架构"每次轮询都重试坏机型"的风暴。
 */
public enum FMStatus {
	UNRESOLVED,
	LOADING,
	READY,
	MISSING,
	CORRUPT
}
