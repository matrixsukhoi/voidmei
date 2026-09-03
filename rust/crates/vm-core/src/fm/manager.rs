//! FM 管理器实现。

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::thread;

use crate::base::bus::EventBus;
use crate::base::event::ui_state_events;
use crate::base::exception_helper::panic_message_box;
use crate::base::java_compat::current_time_millis;
use crate::base::logger;
use crate::fm::handle::FMHandle;
use crate::fm::loader;

/// FM 管理器（单例）—— "当前飞机 / FM 加载状态"的单一真相源（P2 重构，issue #55）。
///
/// <p>旧 Controller 用 5 个分散变量（cur_fmtype / identifiedFMName / loadedFMName /
/// failedFMName / Blkx）手动同步描述同一件事，失步即死循环：FM 缺失 → 加载失败 →
/// 回退重解析默认机 → 清失败记录 → 又重试坏机型 → 每秒 ~20 次"解析+gc+事件"风暴。
/// 本类以「一个 volatile current 句柄 + 一个 currentTarget 目标名」取代：
/// <ul>
///   <li>identify(name) 是唯一入口：目标去重 → 负缓存拦截 → 提交后台单线程加载；</li>
///   <li>加载在 "FM-Loader" daemon 线程执行，完成后原子 swap current 并广播
///       {@link UIStateEvents#FM_CHANGED}；</li>
///   <li>MISSING/CORRUPT 结果进负缓存，同名 identify 永不再触磁盘加载——死循环根治点。</li>
/// </ul>
///
/// <p><b>线程模型</b>：current/currentTarget 为 volatile，读方法无锁；
/// 本类不使用 synchronized（reset 除外），事件发布天然在锁外进行。
/// {@link UIStateEvents#FM_CHANGED} 在 loader 线程同步派发，订阅方碰 Swing 必须自行
/// invokeLater。
///
/// PORT(单例解散, §2.9 / ): Java `static final INSTANCE` +
/// `getInstance()` → 调用方持有的 struct (AppState 收编), 跨 Controller 重建共享
/// 同一 FM 状态的语义由调用方持同一实例/Arc 表达。TestFMStore 用例移植见
/// fm/store_tests.rs (主 agent 预置, 对接本 API; 本模块 tests 不重复移植)。
/// PORT(FM_CHANGED 总线注入): Java 发布走 `UIStateBus.getInstance().publish
/// (FM_CHANGED, this, handle)` 的 "全局单例 + 字符串路由 + 弱类型 payload" →
/// 构造注入 [`FmChangedBus`] (专用强类型通道, 非全局; 见该类型注)。
/// PORT(volatile 映射, LIFETIMES 审查修正 ★1 + §7 草案): `current` 并非单写者 ——
/// identify 的 NOT_AIRCRAFT/负缓存分支在 Service/FM-Detect 调用线程直接写, loader
/// 线程完成后也写, 语义 = "多写者 + 原子替换" → `RwLock<Arc<FMHandle>>`
/// (草案 `ArcSwap<Arc<FmHandle>>` 的 std 等价形态, 禁新增依赖不引
/// arc-swap): 读侧 O(1) clone Arc = Java volatile 读返回共享引用, 且消费者共享
/// 同一句柄实例 —— blkx.engLoad 是 Service 线程 ~10Hz 就地改写的共享会话态
/// (handle.rs javadoc / blkx 波次陷阱注 5), 每读深拷会让消费者拿到 fork、对快照的
/// 改写静默丢失, Arc 共享在此销号。handle.rs 预留的 "Arc 共享 vs 每轮深拷"
/// 决策点已裁决为 Arc 共享。
/// PORT(锁外发布, §2.8): 本文件所有锁的临界区只做赋值/查表/清空 (无 panic 路径,
/// 锁不可中毒, fm_data_paths.rs 同款论证), 且**任何锁都不跨 publish 持有** ——
/// 对齐 Java "volatile 无 monitor + 事件在锁外发布" 的可重入语义 (订阅方可回调
/// current()/identify() 而不死锁)。
pub struct FMManager {
    /// 状态本体: Java 实例字段集合。loader 任务闭包在 Java 里捕获单例 `this`,
    /// Rust 无单例 → Arc<Inner> 克隆进任务闭包 (外壳由调用方持有, 无环)。
    inner: Arc<Inner>,
}

/// FM_CHANGED 广播通道类型。
/// Java `UIStateBus.getInstance().publish(FM_CHANGED, this, handle)` 的
/// 字符串路由 + 弱类型 payload → 专用强类型通道: 消息 = payload 本体 (Java 订阅方
/// `Consumer<Object>` 收到的 data 即 FMHandle, 不含 eventType), 订阅方持同一 Arc
/// 经 [`EventBus::subscribe`] 订阅 (RAII Subscription, Drop 即退订)。ui_state_bus.rs
/// 的 `UiStateEvent.data` 只能装 String 载荷 (其 PORT 注释已把 "扩展消息携带原生
/// 句柄" 的决策上报主 agent), 本波次禁越文件改 ui_state_bus.rs —— 专用通道是保住
/// 句柄载荷语义的唯一路径; 未来若主 agent 扩展 UiStateEvent, 切换点集中在
/// Inner.fm_changed 字段类型与 [`FMManager::new`] 参数。
/// 订阅方回调内**禁止同步重入** [`FMManager::identify`]
/// 或对本总线再次 publish —— bus.rs 的 publish 持各监听器自身 Mutex 执行回调,
/// 同线程同总线二次 lock 同一 Mutex = 永久挂死 (Java monitor 可重入无此形态;
/// ui_state_bus.rs 对同形态有明文已知破损注)。Java 对等调用链不可达
/// (Controller.fmChangedHandler 仅 toast(invokeLater)+防抖), 未来 C 类订阅方
/// (Controller/AttitudeOverlay/DrawFrameSimpl/FMUnpackedDataOverlay) 若需在回调内
/// 触发 identify, 必须先经 channel 转出发布线程 (或随 bus.rs 波次的监听器垫片
/// 一并裁决, 见 publish_fm_changed PORT 注)。
pub type FmChangedBus = EventBus<FMHandle>;

/// loader 任务载荷: 闭包本体 (Java executor 的 Runnable)
type LoaderJob = Box<dyn FnOnce() + Send + 'static>;

/// 状态锁中毒消息 (临界区全为无 panic 的赋值/查表/清空, 理论不可达;
/// ui_state_bus.rs MAP_LOCK_MSG 同款先例)
const LOCK_MSG: &str = "FMManager 状态锁中毒";

struct Inner {
    /// 当前句柄；加载期间保留旧句柄（HUD 用旧 FM 平滑过渡），完成后原子 swap
    // Java `volatile FMHandle current = FMHandle.UNRESOLVED` →
    // RwLock<Arc<FMHandle>>, 读侧 O(1) clone Arc 快照且共享同一实例
    // (见模块 doc 的 ★1/§7 裁决)
    current: RwLock<Arc<FMHandle>>,
    /// 当前识别目标（规范化小写机型名）；null = 尚未识别
    // Java `volatile String currentTarget = null` → Mutex<Option<String>>
    //
    current_target: Mutex<Option<String>>,
    /// 负缓存：isMissingLike 的机型 → 失败时间戳。命中则 identify 不再发加载任务
    // Java `ConcurrentHashMap<String, Long>` → Mutex<HashMap> (LIFETIMES
    // §3.2 "访问频率不高" 裁决); 只做 contains/insert/remove/clear, 迭代顺序无涉
    negative_cache: Mutex<HashMap<String, i64>>,
    /// 速率护栏：机型 → 最近一次真正执行 FMLoader.load 的时刻
    last_attempt_ms: Mutex<HashMap<String, i64>>,
    /// 在途任务计数（提交 ++ / 任务 finally --），支撑 isLoading() 纯读观测
    // Java `AtomicInteger` → AtomicI32; volatile 复合语义 → SeqCst
    in_flight: AtomicI32,
    /// 单线程串行加载器：天然免除并发加载同一/不同机型的竞态
    // Java `volatile ExecutorService loader = newLoader()` (单线程 daemon
    // executor "FM-Loader", reset 时整体换新) → 专用线程 + mpsc 无界通道的发送端;
    // Mutex 守护 reset 换新 vs submitLoad 取用的竞态 (Java volatile 读写的对位)
    loader: Mutex<mpsc::Sender<LoaderJob>>,
    /// loader 世代号 (无 Java 对应字段): 复刻 `shutdownNow()` 的 "丢弃排队任务"
    /// —— 任务提交时记世代, 执行时世代不符即放弃; reset 换 loader 时 +1。
    /// 取号必须与取发送端同在 loader 锁临界区内 (见 submit_load/reset), 保证
    /// "旧世代任务必落旧通道" 的原子性。
    loader_epoch: AtomicU64,
    /// reset 串行化锁 (无 Java 对应字段): Java `synchronized reset()` 的 monitor
    /// 仅互斥并发 reset (identify 等本就无锁) → 专用锁复刻; 恒最外层获取,
    /// 锁序 reset_lock → 状态锁, 无环
    reset_lock: Mutex<()>,
    /// FM_CHANGED 广播通道 (构造注入, 非全局) —— 见 [`FmChangedBus`] 的 PORT 注
    fm_changed: Arc<FmChangedBus>,
}

/// 速率护栏窗口（毫秒）：同一机型在窗口内已真正执行过加载且结果仍挂在 current 上时，
/// 跳过重复加载。纵深防御——正常防抖由"目标去重 + 负缓存"完成。
const RETRY_INTERVAL_MS: i64 = 60_000;

impl FMManager {
    /// 单例解散后的显式构造: 调用方持 Arc<FMManager> 注入;
    /// FM_CHANGED 通道为参数注入 (store_tests.rs new_manager 先例)。
    pub fn new(fm_changed: Arc<FmChangedBus>) -> Self {
        FMManager {
            inner: Arc::new(Inner {
                current: RwLock::new(Arc::new(FMHandle::UNRESOLVED)),
                current_target: Mutex::new(None),
                negative_cache: Mutex::new(HashMap::new()),
                last_attempt_ms: Mutex::new(HashMap::new()),
                in_flight: AtomicI32::new(0),
                loader: Mutex::new(new_loader()),
                loader_epoch: AtomicU64::new(0),
                reset_lock: Mutex::new(()),
                fm_changed,
            }),
        }
    }

    /// FM_CHANGED 通道取回 —— 供订阅方 (Controller/AttitudeOverlay/DrawFrameSimpl/
    /// FMUnpackedDataOverlay 波次) 订阅句柄变化。构造方分发同一 Arc,
    /// 本取回器是便捷入口 (RAII Subscription, Drop 即退订)。
    pub fn fm_changed_bus(&self) -> Arc<FmChangedBus> {
        Arc::clone(&self.inner.fm_changed)
    }

    /// 当前 FM 句柄（纯 volatile 读，无锁）。未识别时返回 UNRESOLVED 哨兵。
    // Java volatile 读返回共享引用 (O(1)) ↔ 读锁 + Arc 引用计数克隆
    // (模块 doc ★1/§7 裁决: 快照与发布方共享同一句柄实例, blkx 会话态改写不丢);
    // Deref 使 `.status/.name/has_fm()` 与 Display 调用点与按值返回零差异
    pub fn current(&self) -> Arc<FMHandle> {
        Arc::clone(&self.inner.current.read().expect(LOCK_MSG))
    }

    /// 是否有加载任务在途（纯读观测；LOADING 期间 current() 仍返回旧句柄）
    pub fn is_loading(&self) -> bool {
        self.inner.in_flight.load(Ordering::SeqCst) > 0
    }

    /// 当前识别目标名（规范化小写）；未识别返回 None
    pub fn current_target_name(&self) -> Option<String> {
        self.inner.current_target.lock().expect(LOCK_MSG).clone()
    }

    /// 识别（并按需异步加载）机型 —— 唯一入口。高频调用安全：目标未变时零成本返回。
    ///
    /// - `planeName`: 机型名（任意大小写/空白，内部规范化）；None/空直接忽略
    pub fn identify(&self, plane_name: Option<&str>) {
        let plane_name = match plane_name {
            None | Some("") => return,
            Some(p) => p,
        };
        // Java `toLowerCase().trim()` — 默认 Locale 的 toLowerCase ↔
        // to_lowercase (≡Locale.ROOT), 机型名域为 ASCII 逐字符一致; trim 的
        // C0 vs Unicode White_Space 差异在该域不可达 (fm_loader.rs 同款先例注)
        let name = plane_name.to_lowercase().trim().to_string();

        // 去重：目标没变就什么都不做（Service 轮询/配置刷新等高频调用方零成本）
        let target_now = self.inner.current_target.lock().expect(LOCK_MSG).clone();
        if target_now.as_deref() == Some(name.as_str()) {
            return;
        }

        // 句柄已在：clearTarget 后切回刚加载过的机型 —— 恢复目标即可，零成本秒开
        //（current 未变，无需广播事件）
        // PORT(有利微竞态备案, 审查 W3a): Java `current.hasFM() &&
        // name.equals(current.name)` 对 volatile current 是两次独立读 (间窗内并发
        // swap 可致 hasFM 与 name 取自不同句柄), Rust 单次快照读两字段更自洽 ——
        // 无行为契约, 有意不复刻双读 (速率护栏分支的复刻见下)
        let (has_fm, cur_name) = {
            let c = self.inner.current.read().expect(LOCK_MSG);
            (c.has_fm(), c.name.clone())
        };
        if has_fm && cur_name.as_deref() == Some(name.as_str()) {
            *self.inner.current_target.lock().expect(LOCK_MSG) = Some(name);
            return;
        }

        // 非飞机载具短路：坦克/军舰等 type 带路径前缀（如 "tankmodels/..."），
        // 而飞机 type 恒为裸名（与 flightmodels/ 下文件名一致）。FM 数据库只有
        // flightmodels，直接落 NOT_AIRCRAFT 句柄——不发加载任务（省一次注定失败的
        // 磁盘查找）、不进负缓存（不是数据缺失而是不适用）、不弹缺失 toast。
        // 修复: 陆战时误把坦克当"FM 缺失的新飞机"弹提示。
        // Java `name.indexOf('/') >= 0` ↔ contains('/') ('/' 单字符, 语义全等)
        if name.contains('/') {
            let handle = Arc::new(FMHandle::not_aircraft(Some(name.clone())));
            *self.inner.current.write().expect(LOCK_MSG) = Arc::clone(&handle);
            *self.inner.current_target.lock().expect(LOCK_MSG) = Some(name);
            // 写锁临时守卫均已释放后才发布 (§2.8: 订阅方可重入 current()/identify())
            // PORT(有利微竞态备案, 审查 W3b): Java `publishFmChanged(current)` 传
            // 赋值后重读的字段, 间窗内并发 loader swap 下载荷可与本线程刚写的不同;
            // Rust 发布本地 handle 快照, 载荷恒为本线程写入的句柄, 更自洽
            Self::publish_fm_changed(&self.inner.fm_changed, &handle);
            return;
        }

        // 负缓存：确认 MISSING/CORRUPT 的机型不再发加载任务（issue #55 死循环根治点）。
        // 直接落 MISSING 句柄并广播，让 HUD 立即知道当前机型无 FM 可用
        // 先取 bool 再分支 —— 若把 lock().contains_key() 直接挂 if
        // 条件, 临时守卫会存活到分支体结束, publish 期间持锁 + 订阅方重入
        // identify() 取同一 Mutex 即死锁 (Java CHM 无 monitor 无此形态)
        let cached_missing = self
            .inner
            .negative_cache
            .lock()
            .expect(LOCK_MSG)
            .contains_key(&name);
        if cached_missing {
            let handle = Arc::new(FMHandle::missing(Some(name.clone())));
            *self.inner.current.write().expect(LOCK_MSG) = Arc::clone(&handle);
            *self.inner.current_target.lock().expect(LOCK_MSG) = Some(name);
            // 同 NOT_AIRCRAFT 分支 —— 发布本地快照 (W3b 备案)
            Self::publish_fm_changed(&self.inner.fm_changed, &handle);
            return;
        }

        // 速率护栏：60s 内刚加载过且该结果仍挂在 current 上时跳过重复加载；
        // 目标已切走又切回则放行重载（正确性优先于限速）
        let last = self
            .inner
            .last_attempt_ms
            .lock()
            .expect(LOCK_MSG)
            .get(&name)
            .copied();
        if let Some(last) = last {
            // Java 的 && 短路序: 时间窗命中才重读 current.name (第二次 volatile 读,
            // 不复用前面快照 —— 与 Java 两处独立读 current 一致)
            if current_time_millis() - last < RETRY_INTERVAL_MS && {
                let n = self.inner.current.read().expect(LOCK_MSG).name.clone();
                n.as_deref() == Some(name.as_str())
            } {
                logger::debug("FMManager", &format!("速率护栏命中，跳过重复加载: {name}"));
                return;
            }
        }

        // 只记目标，current 保持不动：加载期间 HUD 继续用旧 FM 平滑过渡，
        // 加载完成后一次性原子 swap（不会出现半新半旧的中间态）
        *self.inner.current_target.lock().expect(LOCK_MSG) = Some(name.clone());
        self.submit_load(&name);
    }

    /// 清除识别目标（退出游戏/预览时调用）。刻意保留 current 句柄 ——
    /// 用户马上切回同一机型时秒开（identify 的"句柄已在"分支）。
    pub fn clear_target(&self) {
        *self.inner.current_target.lock().expect(LOCK_MSG) = None;
    }

    /// 手动作废某机型的负缓存（例如 data/ 更新后确认文件已补齐）。
    /// 下次 identify 将重新尝试磁盘加载。
    pub fn invalidate(&self, name: Option<&str>) {
        let name = match name {
            None => return,
            Some(n) => n,
        };
        let norm = name.to_lowercase().trim().to_string();
        self.inner
            .negative_cache
            .lock()
            .expect(LOCK_MSG)
            .remove(&norm);
        self.inner
            .last_attempt_ms
            .lock()
            .expect(LOCK_MSG)
            .remove(&norm);
    }

    /// 测试用：清一切状态（current/target/负缓存/护栏计数）并停掉排队中的任务，
    /// 重建 loader 线程供后续用例使用。
    // Java `synchronized reset()` — monitor 仅串行化并发 reset → 专用
    // reset_lock (锁序恒最外层, 见 Inner.reset_lock 注; Mutex 不可重入, §2.8)
    pub fn reset(&self) {
        let _reset_guard = self.inner.reset_lock.lock().expect(LOCK_MSG);
        *self.inner.current.write().expect(LOCK_MSG) = Arc::new(FMHandle::UNRESOLVED);
        *self.inner.current_target.lock().expect(LOCK_MSG) = None;
        self.inner.negative_cache.lock().expect(LOCK_MSG).clear();
        self.inner.last_attempt_ms.lock().expect(LOCK_MSG).clear();
        self.inner.in_flight.store(0, Ordering::SeqCst);
        // 运行中任务不可中断、跑完 (Java interrupt 对磁盘解析同样无效); 世代推进与
        // 通道换新同在 loader 锁内, 与 submit_load 的 "取号+取发送端" 互斥 —— 保证
        // 旧世代任务必落旧通道被丢弃, 换代后提交的任务带新世代正常执行
        *self.inner.loader.lock().expect(LOCK_MSG) = {
            self.inner.loader_epoch.fetch_add(1, Ordering::SeqCst);
            new_loader()
        };
    }

    /// 提交后台加载任务（单线程串行执行，天然免除并发加载竞态）
    fn submit_load(&self, target_name: &str) {
        self.inner.in_flight.fetch_add(1, Ordering::SeqCst);
        // 持锁窗口内原子地取 "世代号 + 发送端" (与 reset 的换代互斥, 见 reset 注);
        // 锁内无回调执行
        let (epoch, tx) = {
            let loader = self.inner.loader.lock().expect(LOCK_MSG);
            (
                self.inner.loader_epoch.load(Ordering::SeqCst),
                loader.clone(),
            )
        };
        let inner = Arc::clone(&self.inner);
        let target_name = target_name.to_string();
        let job: LoaderJob = Box::new(move || run_load_job(inner, target_name, epoch));
        if let Err(reject) = tx.send(job) {
            // Java 对已 shutdown 的 executor `execute()` 抛
            // RejectedExecutionException 向调用方传播 (Service 顶层 catch 兜底),
            // 计数不回退 (increment 已做, 任务 finally 不再执行)。本实现中接收端
            // 存活期覆盖发送端 (Inner 持有 tx), 该路径不可达 —— 防御性记 ERROR 而
            // 非 panic, 保持与 Java 同样不炸调用线程
            logger::error("FMManager", &format!("FM-Loader 任务提交失败: {reject:?}"));
        }
    }

    /// 广播句柄变化。本类无锁（reset 外），此处发布天然在锁外、在 loader 线程执行；
    /// UIStateBus 内部线程安全，订阅方是同步回调 —— 碰 Swing 必须自行 invokeLater。
    // Java 方法体 `UIStateBus.getInstance().publish(UIStateEvents.FM_CHANGED,
    // this, handle)` —— `this` 仅作日志 source, 方法不触其他实例态 → 收敛为关联
    // 函数, 接收者换成注入的 FM_CHANGED 通道 (identify 同步分支在调用线程派发,
    // loader 分支在 FM-Loader 线程派发, 与 Java 两处发布线程一致)。
    // Java UIStateBus.publish 首行 `Logger.event("PUBLISH", eventType,
    // source, data)` 随路由丢失 (专用通道不经 UIStateBus), 在此复刻 —— source =
    // Java this 的类简单名 "FMManager", data = handle.toString() (logger.rs e2e
    // 钉子 "PUBLISH: FMManager -> FMHandle[MISSING he_162]: fmChanged" 的口径)。
    // Java UIStateBus.publish 逐 handler catch(Exception) —— 发布方永不因
    // 订阅方失败上抛; bus.rs 裸 EventBus 无订阅侧 catch_unwind 垫片
    // (ui_state_bus.rs 先例), 此处在发布侧兜底对齐该语义。遗留上报: 订阅方 panic
    // 仍会打滥其监听器 Mutex, 后续对同一订阅者的 publish 在 bus 内部 panic 并被
    // 此处吞掉 (送达性损失) —— bus.rs 基建议题, 不越文件修。
    fn publish_fm_changed(bus: &FmChangedBus, handle: &FMHandle) {
        logger::event(
            "PUBLISH",
            ui_state_events::FM_CHANGED,
            Some("FMManager"),
            Some(&handle.to_string()),
        );
        if let Err(payload) = catch_unwind(AssertUnwindSafe(|| {
            bus.publish(handle);
        })) {
            logger::error(
                "FMManager",
                &format!("FM_CHANGED 订阅方执行失败: {}", panic_message_box(payload)),
            );
        }
    }
}

/// Java `newLoader()`: `Executors.newSingleThreadExecutor(r -> new Thread(r,
/// "FM-Loader").setDaemon(true))`。
// 单线程 daemon executor → 专用线程 + mpsc 无界通道 (草案
// loader_tx 形态)。Rust 线程不阻塞进程退出 (main 返回即终止) ≈ daemon; 任务异常
// 由 catch_unwind 吞掉记日志 —— 对齐 Java executor "线程死亡自动补替、队列不丢"
// 的效果 (Java 未捕获异常堆栈打印由 ERROR 日志顶位)
fn new_loader() -> mpsc::Sender<LoaderJob> {
    let (tx, rx) = mpsc::channel::<LoaderJob>();
    thread::Builder::new()
        .name("FM-Loader".to_string())
        .spawn(move || {
            for job in rx {
                if let Err(payload) = catch_unwind(AssertUnwindSafe(job)) {
                    logger::error(
                        "FMManager",
                        &format!("FM-Loader 任务执行失败: {}", panic_message_box(payload)),
                    );
                }
            }
        })
        .expect("FM-Loader 线程创建失败");
    tx
}

/// loader 任务体 —— Java submitLoad 内 lambda 的对位物。
/// Java 任务闭包捕获单例 `this` → 捕获 Arc<Inner>; epoch 为 shutdownNow
/// 队列清空的复刻载体 (无 Java 对应参数, 见 Inner.loader_epoch 注)。
fn run_load_job(inner: Arc<Inner>, target_name: String, epoch: u64) {
    // 排队后、执行前遇 reset (换代) → 直接放弃 = Java shutdownNow 的队列
    // 清空; 运行中任务不中断 (Java interrupt 同样止不住磁盘解析)。
    // 世代检查刻意置于 finally 守卫建立**之前** (审查 A/B W1): Java 被清队任务
    // 根本不运行 (lambda 不执行, finally 不减计数), 仅运行中任务事后补减 ——
    // 若先建守卫再查世代, 被弃任务的 Drop 会在 reset 清零后把 in_flight 减成
    // 负数, is_loading() 相对 Java 偏低; 前置检查 = "任务从未进入 try 块"
    if inner.loader_epoch.load(Ordering::SeqCst) != epoch {
        return;
    }
    // finally 的 "含早退 return 与 panic 双路径必执行" 语义由 Drop guard
    // 承接
    struct InFlightDecrement<'a>(&'a AtomicI32);
    impl Drop for InFlightDecrement<'_> {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }
    let _finally = InFlightDecrement(&inner.in_flight);

    // 排队期间目标可能又变（identify 了别的机型），过期任务直接放弃
    let target_now = inner.current_target.lock().expect(LOCK_MSG).clone();
    if target_now.as_deref() != Some(target_name.as_str()) {
        return;
    }
    inner
        .last_attempt_ms
        .lock()
        .expect(LOCK_MSG)
        .insert(target_name.clone(), current_time_millis());
    let result = Arc::new(loader::load(Some(&target_name)));
    // 加载耗时期间目标也可能又变，过期结果不落 current
    let target_now = inner.current_target.lock().expect(LOCK_MSG).clone();
    if target_now.as_deref() != Some(target_name.as_str()) {
        return;
    }
    *inner.current.write().expect(LOCK_MSG) = Arc::clone(&result);
    if result.is_missing_like() {
        // 失败结果进负缓存：此后同名 identify 不再触发磁盘加载
        // Java `negativeCache.put(result.name, ...)` — name 为 null 时 CHM
        // 抛 NPE, 但 load(非空名) 恒返回非 null name, 不可达; Option 防御性跳过
        if let Some(result_name) = result.name.clone() {
            inner
                .negative_cache
                .lock()
                .expect(LOCK_MSG)
                .insert(result_name, current_time_millis());
        }
    }
    // 全部锁释放后才发布 (§2.8 — 订阅方可重入 current()/identify())
    FMManager::publish_fm_changed(&inner.fm_changed, &result);
}

// =====================================================================
// Tests — TestFMStore.java 全量用例 (①~⑥ + ②b/④b) 已由 fm/store_tests.rs
// 移植 (主 agent 预置并对接本 API), 本模块不重复移植 (§5 不为凑覆盖写空转测试;
// 重叠消解见 store_tests.rs 模块头注的上报)。此处只落 store_tests 未覆盖的面:
//   1. identify 的 null/空名边界守卫 (store_tests.rs 注释引用本处的
//      identify_null_and_empty_are_ignored);
//   2. 负缓存**命中分支** (Java 用例② 的 999 次重调全被目标去重拦截, 未触达
//      containsKey 分支 —— 该分支要求 "目标切走后再切回缺失机型");
//   3. FM_CHANGED 同步派发时序 + 句柄载荷 (专用通道消息 = payload 本体);
//   4. invalidate (负缓存手动作废, Java 测试未覆盖);
//   5. 真机 data/ 的 identify→current 快照→负缓存命中 断言链 (本波次任务规则 2;
//      store_tests 刻意不依赖真机 data/)。
//
// 并发隔离 (PORT): cargo test 同二进制并行跑 #[test] —— 本模块全部用例挂
// crate::fm::test_support::data_root() 串行锁 (DATA_ROOT / LOAD_COUNT 进程级全局,
// fm_loader.rs W-B2 备案的兑现); **不翻转 DATA_ROOT** (store_tests.rs 头注的
// 竞态备案: 未挂锁的 data_paths::java_main_sequence 首断言
// `get_data_root()=="./data"` 会与翻转竞态) —— 对齐 fm_loader.rs/store_tests.rs
// 的 "多根铺数据" 免疫策略: 所需文件铺满 DATA_ROOT 全部可能取值 (ROOTS), load
// 在任何时刻读任何根, 命中/缺失判定恒定 (无 flaky fail, 无假通过窗口)。
// 跨进程边界 (审查 B W4 实证备案): test_guard 锁仅**进程内**互斥 —— 多 agent
// 在同一 workspace 并发跑 vm-core 测试时, 外部 cargo/测试进程共享 CWD 下的
// ./data/testroot/otherroot 夹具, 其 setup/cleanup 可互相删除文件 (实测中央文件
// 被清 → MISSING≠CORRUPT 假失败)。上述 "无 flaky fail" 担保仅单进程内成立;
// 流水线纪律: 同一 workspace 禁止多 cargo 进程并发跑本 crate 测试 (或夹具改
// 每进程唯一的绝对临时根, store_tests create_temp_root 先例)。
// =====================================================================
#[cfg(test)]
mod tests;
