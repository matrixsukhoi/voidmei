package prog.fm;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicInteger;

import prog.event.UIStateBus;
import prog.event.UIStateEvents;
import prog.util.Logger;

/**
 * FM 管理器（单例）—— "当前飞机 / FM 加载状态"的单一真相源（P2 重构，issue #55）。
 *
 * <p>旧 Controller 用 5 个分散变量（cur_fmtype / identifiedFMName / loadedFMName /
 * failedFMName / Blkx）手动同步描述同一件事，失步即死循环：FM 缺失 → 加载失败 →
 * 回退重解析默认机 → 清失败记录 → 又重试坏机型 → 每秒 ~20 次"解析+gc+事件"风暴。
 * 本类以「一个 volatile current 句柄 + 一个 currentTarget 目标名」取代：
 * <ul>
 *   <li>identify(name) 是唯一入口：目标去重 → 负缓存拦截 → 提交后台单线程加载；</li>
 *   <li>加载在 "FM-Loader" daemon 线程执行，完成后原子 swap current 并广播
 *       {@link UIStateEvents#FM_CHANGED}；</li>
 *   <li>MISSING/CORRUPT 结果进负缓存，同名 identify 永不再触磁盘加载——死循环根治点。</li>
 * </ul>
 *
 * <p><b>线程模型</b>：current/currentTarget 为 volatile，读方法无锁；
 * 本类不使用 synchronized（reset 除外），事件发布天然在锁外进行。
 * {@link UIStateEvents#FM_CHANGED} 在 loader 线程同步派发，订阅方碰 Swing 必须自行
 * invokeLater。
 */
public final class FMManager {

	private static final FMManager INSTANCE = new FMManager();

	public static FMManager getInstance() {
		return INSTANCE;
	}

	private FMManager() {
	}

	/**
	 * 速率护栏窗口（毫秒）：同一机型在窗口内已真正执行过加载且结果仍挂在 current 上时，
	 * 跳过重复加载。纵深防御——正常防抖由"目标去重 + 负缓存"完成。
	 */
	private static final long RETRY_INTERVAL_MS = 60_000;

	/** 当前句柄；加载期间保留旧句柄（HUD 用旧 FM 平滑过渡），完成后原子 swap */
	private volatile FMHandle current = FMHandle.UNRESOLVED;
	/** 当前识别目标（规范化小写机型名）；null = 尚未识别 */
	private volatile String currentTarget = null;

	/** 负缓存：isMissingLike 的机型 → 失败时间戳。命中则 identify 不再发加载任务 */
	private final ConcurrentHashMap<String, Long> negativeCache = new ConcurrentHashMap<>();
	/** 速率护栏：机型 → 最近一次真正执行 FMLoader.load 的时刻 */
	private final ConcurrentHashMap<String, Long> lastAttemptMs = new ConcurrentHashMap<>();

	/** 在途任务计数（提交 ++ / 任务 finally --），支撑 isLoading() 纯读观测 */
	private final AtomicInteger inFlight = new AtomicInteger(0);

	/** 单线程串行加载器：天然免除并发加载同一/不同机型的竞态 */
	private volatile ExecutorService loader = newLoader();

	private static ExecutorService newLoader() {
		return Executors.newSingleThreadExecutor(r -> {
			Thread t = new Thread(r, "FM-Loader");
			t.setDaemon(true);
			return t;
		});
	}

	/** 当前 FM 句柄（纯 volatile 读，无锁）。未识别时返回 UNRESOLVED 哨兵。 */
	public FMHandle current() {
		return current;
	}

	/** 是否有加载任务在途（纯读观测；LOADING 期间 current() 仍返回旧句柄） */
	public boolean isLoading() {
		return inFlight.get() > 0;
	}

	/** 当前识别目标名（规范化小写）；未识别返回 null */
	public String currentTargetName() {
		return currentTarget;
	}

	/**
	 * 识别（并按需异步加载）机型 —— 唯一入口。高频调用安全：目标未变时零成本返回。
	 *
	 * @param planeName 机型名（任意大小写/空白，内部规范化）；null/空直接忽略
	 */
	public void identify(String planeName) {
		if (planeName == null || planeName.isEmpty())
			return;
		final String name = planeName.toLowerCase().trim();

		// 去重：目标没变就什么都不做（Service 轮询/配置刷新等高频调用方零成本）
		if (name.equals(currentTarget))
			return;

		// 句柄已在：clearTarget 后切回刚加载过的机型 —— 恢复目标即可，零成本秒开
		//（current 未变，无需广播事件）
		if (current.hasFM() && name.equals(current.name)) {
			currentTarget = name;
			return;
		}

		// 负缓存：确认 MISSING/CORRUPT 的机型不再发加载任务（issue #55 死循环根治点）。
		// 直接落 MISSING 句柄并广播，让 HUD 立即知道当前机型无 FM 可用
		if (negativeCache.containsKey(name)) {
			current = FMHandle.missing(name);
			currentTarget = name;
			publishFmChanged(current);
			return;
		}

		// 速率护栏：60s 内刚加载过且该结果仍挂在 current 上时跳过重复加载；
		// 目标已切走又切回则放行重载（正确性优先于限速）
		Long last = lastAttemptMs.get(name);
		if (last != null && System.currentTimeMillis() - last < RETRY_INTERVAL_MS
				&& name.equals(current.name)) {
			Logger.debug("FMManager", "速率护栏命中，跳过重复加载: " + name);
			return;
		}

		// 只记目标，current 保持不动：加载期间 HUD 继续用旧 FM 平滑过渡，
		// 加载完成后一次性原子 swap（不会出现半新半旧的中间态）
		currentTarget = name;
		submitLoad(name);
	}

	/**
	 * 清除识别目标（退出游戏/预览时调用）。刻意保留 current 句柄 ——
	 * 用户马上切回同一机型时秒开（identify 的"句柄已在"分支）。
	 */
	public void clearTarget() {
		currentTarget = null;
	}

	/**
	 * 手动作废某机型的负缓存（例如 data/ 更新后确认文件已补齐）。
	 * 下次 identify 将重新尝试磁盘加载。
	 */
	public void invalidate(String name) {
		if (name == null)
			return;
		String norm = name.toLowerCase().trim();
		negativeCache.remove(norm);
		lastAttemptMs.remove(norm);
	}

	/**
	 * 测试用：清一切状态（current/target/负缓存/护栏计数）并停掉排队中的任务，
	 * 重建 loader 线程供后续用例使用。
	 */
	public synchronized void reset() {
		current = FMHandle.UNRESOLVED;
		currentTarget = null;
		negativeCache.clear();
		lastAttemptMs.clear();
		inFlight.set(0);
		loader.shutdownNow();
		loader = newLoader();
	}

	/** 提交后台加载任务（单线程串行执行，天然免除并发加载竞态） */
	private void submitLoad(final String targetName) {
		inFlight.incrementAndGet();
		loader.execute(() -> {
			try {
				// 排队期间目标可能又变（identify 了别的机型），过期任务直接放弃
				if (!targetName.equals(currentTarget))
					return;
				lastAttemptMs.put(targetName, System.currentTimeMillis());
				FMHandle result = FMLoader.load(targetName);
				// 加载耗时期间目标也可能又变，过期结果不落 current
				if (!targetName.equals(currentTarget))
					return;
				current = result;
				if (result.isMissingLike()) {
					// 失败结果进负缓存：此后同名 identify 不再触发磁盘加载
					negativeCache.put(result.name, System.currentTimeMillis());
				}
				publishFmChanged(result);
			} finally {
				inFlight.decrementAndGet();
			}
		});
	}

	/**
	 * 广播句柄变化。本类无锁（reset 外），此处发布天然在锁外、在 loader 线程执行；
	 * UIStateBus 内部线程安全，订阅方是同步回调 —— 碰 Swing 必须自行 invokeLater。
	 */
	private void publishFmChanged(FMHandle handle) {
		UIStateBus.getInstance().publish(UIStateEvents.FM_CHANGED, this, handle);
	}
}
