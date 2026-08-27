//! 对应 Java: `src/prog/FocusMonitor.java` (一比一翻译, B 类: Service 轮询驱动的
//! 节流探测器)
//!
//! 游戏窗口焦点监控辅助类。
//! 不创建线程，由Service轮询调用tick()方法。
//! 内部实现节流和状态追踪。
//!
//! 使用方式:
//! 1. Service.java中创建实例
//! 2. openpad时调用setEnabled(true)
//! 3. 每次轮询调用tick()
//! 4. closepad时调用setEnabled(false)
//!
//! PORT (依赖注入): Java 的两处进程级静态依赖本波次均不可达, 解散为构造参数:
//! - `FocusDetector.isWarThunderFocused()` (静态工具; FocusDetector.java 本体
//!   为 A 类, 仅 Windows 腿 JNA→Win32 为 C 类) → 本文件定义 [`FocusDetector`]
//!   trait (接口签名照 Java), Windows 实现留 P4 (// PORT: 见 trait 注);
//! - `AlwaysOnTopCoordinator.getInstance()` (Swing 窗口单例 → C 类) →
//!   [`AlwaysOnTopCoordinatorApi`] 最小接口 = FocusMonitor 的全部访问面 (3 方法),
//!   真实协调器落地时 impl 接线 (overlay_context.rs `ControllerRef` /
//!   flight_log.rs `ControllerLogSink` 同款消费方桩先例)。
//!
//! PORT (线程模型): Java 侧本类字段被多线程读写 —— setEnabled(true) 跑在
//! changeS3 的匿名延迟线程 (Controller.java:241-246 sleep 100ms → openpad:356),
//! setEnabled(false) 主要跑在 Service 线程 (S4toS1 → closepad:390;
//! onAircraftChanged:319 复用 closepad), 仅 stop() 路径 (Controller.java:773-774
//! ← Application.java:265 托盘) 在 EDT; tick 由 Service 轮询线程调
//! (Service.java:1821)。无任何同步原语 (现存可见性隐患, 保真不修);
//! Rust 以 `&mut self` 表达独占写, 集成方 (vm-data Service) 以
//! `Arc<Mutex<FocusMonitor>>` 承载跨线程共享。§2.8 重入面: 本类方法互不调用,
//! 无二锁自身 Mutex 的风险; 但 set_enabled/tick 在集成方锁内回调注入的
//! trait 对象 (LIFETIMES §3.3 "锁内回调外部代码" 模式), 各 trait 的非阻塞
//! 合同见其 doc —— 实现若同步等待 UI 线程, 会与 EDT 上 set_enabled
//! (stop() 托盘路径) 成 AB-BA 死锁 (Java 无此险: 本类无锁 + invokeLater
//! 即发即忘)。

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// 对应 Java: `src/prog/util/FocusDetector.java` (接口签名照 Java)。
/// 分类: 本 trait 为 FocusMonitor 的消费面, 定义于本文件; FocusDetector.java
/// 本体 (os.name OS 分派) 为 **A 类** (CLASSIFY.md:110) 另立波次, 仅
/// WindowsFocusDetector.java (JNA→Win32) 为 C 类/P4 (CLASSIFY.md:122)。
///
/// 跨平台前台窗口焦点检测器。
/// 纯工具类，不维护状态，不创建线程。
///
/// 仅支持 Windows 平台（使用 JNA 直接调用 Win32 API）。
/// 非 Windows 平台返回 true（安全降级，不隐藏 overlay）。
///
/// 性能提升：从 PowerShell 方案的 300-400ms 降至 JNA 方案的 3-5ms。
///
/// PORT: Java 静态方法 `static boolean isWarThunderFocused()` → trait 实例方法
/// (§1 interface→trait dyn: dyn 注入需要 receiver); Windows JNA 实现
/// (WindowsFocusDetector.java: GetForegroundWindow → GetWindowThreadProcessId →
/// OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION=0x1000) +
/// QueryFullProcessImageName 取进程名, 与 "aces.exe" 忽略大小写比较; 无前台
/// 窗口 / PID 0、4 / 取名失败 / 任何异常均安全降级 true) **留 P4** 以 windows
/// crate 绑定实现; 非 Windows 平台恒 true 的 OS 分派 (Java `os.name` 判定,
/// Rust 可用 `cfg!(windows)` 表达) 是 FocusDetector.java 本体的 A 类机械翻译,
/// 随其自身波次落地, 不随 Windows 腿顺延 P4。
/// 已收口: WindowsFocusDetector 已实装 (vm-overlay platform_extras.rs,
/// GetForegroundWindow 链), impl FocusDetector 生产可用; trait 归属维持本文件。
///
/// 实现合同 (锁内回调): 本方法在集成方 `Mutex<FocusMonitor>` 锁内被调
/// (tick); 真实 Win32 实现阻塞 3-5ms, 会延长该轮锁持有至上限 ~5ms, 可接受,
/// 实现不得再同步等待其他锁/线程。
pub trait FocusDetector: Send + Sync {
    /// 检测 War Thunder 是否为当前前台窗口。
    ///
    /// 安全降级原则：检测失败或非 Windows 平台时返回 true，不误隐藏 overlay。
    ///
    /// @return true 如果 War Thunder 为前台窗口，或非 Windows 平台，或检测失败
    fn is_war_thunder_focused(&self) -> bool;
}

/// FocusMonitor 对 `prog.AlwaysOnTopCoordinator` 的依赖面: Java 静态单例
/// `AlwaysOnTopCoordinator.getInstance()` 在 FocusMonitor.java 的全部访问
/// (setEnabled 禁用分支 isOverlaysHidden/showAllOverlays + tick 变化分支
/// showAllOverlays/hideAllOverlays, 去重共 3 方法)。
///
/// PORT: Coordinator 是 Swing 窗口 z 序协调器 (C 类, P4 语义复刻), 本波次以
/// 消费方最小接口代餐; hide/showAllOverlays 各自的 overlaysHidden 幂等标志与
/// EDT 派发属协调器自身职责 (AlwaysOnTopCoordinator.java:197-233), 不在本文件
/// 复刻。
/// TODO(port): 真实协调器 (P4) 落地时为 AlwaysOnTopCoordinator impl 本 trait。
///
/// 实现合同 (非阻塞, 硬性): 本 trait 全部方法在集成方 `Mutex<FocusMonitor>`
/// 锁内被调 (set_enabled 禁用分支 / tick 变化分支, 见模块头线程模型注)。
/// 真实实现必须非阻塞 —— 对齐 Java 语义 (窗口操作经 setVisibleOnEDT →
/// invokeLater 即发即忘到 EDT, AlwaysOnTopCoordinator.java:197-233), 禁止
/// 同步等待 UI 线程、禁止回锁 FocusMonitor 或其集成方 Mutex, 否则与 EDT 上
/// set_enabled (stop() 托盘路径) 成 AB-BA 死锁 (见模块头)。
pub trait AlwaysOnTopCoordinatorApi: Send + Sync {
    /// Java: `public boolean isOverlaysHidden()` — 检查overlay是否因游戏失焦而被隐藏。
    /// @return true如果overlay当前被隐藏
    fn is_overlays_hidden(&self) -> bool;

    /// Java: `public void hideAllOverlays()` — 隐藏所有已注册的overlay窗口（不销毁实例）。
    /// 用于游戏失焦时自动隐藏HUD。
    fn hide_all_overlays(&self);

    /// Java: `public void showAllOverlays()` — 显示所有被隐藏的overlay窗口。
    /// 用于游戏重新获得焦点时恢复显示。
    fn show_all_overlays(&self);
}

/// 对应 Java: `public class FocusMonitor`
pub struct FocusMonitor {
    /// 上次检测时间戳
    last_check_time: i64,

    /// 上次检测到的焦点状态
    last_focus_state: bool,

    /// 是否启用焦点监控
    enabled: bool,

    /// PORT: Java `FocusDetector.isWarThunderFocused()` 静态直调 → 构造注入 (见模块头)
    detector: Arc<dyn FocusDetector>,

    /// PORT: Java `AlwaysOnTopCoordinator.getInstance()` 单例 → 构造注入 (见模块头)
    coordinator: Arc<dyn AlwaysOnTopCoordinatorApi>,
}

/// 焦点检测间隔（毫秒），200ms足够响应用户切换窗口
// PORT: Java `private static final long` → const (§1)
const CHECK_INTERVAL_MS: i64 = 200;

impl FocusMonitor {
    /// 对应 Java: `new FocusMonitor()` (构造点 Service.java:117)。
    /// PORT: 追加依赖注入两参数 (见模块头); 字段默认值 §2.10 原样显式化 ——
    /// long 0 / `lastFocusState = true` 显式初始化 / `enabled = false`。
    pub fn new(
        detector: Arc<dyn FocusDetector>,
        coordinator: Arc<dyn AlwaysOnTopCoordinatorApi>,
    ) -> Self {
        FocusMonitor {
            last_check_time: 0,
            last_focus_state: true,
            enabled: false,
            detector,
            coordinator,
        }
    }

    /// 启用/禁用焦点监控。
    /// 禁用时会恢复所有被隐藏的overlay。
    ///
    /// @param enabled true启用焦点监控，false禁用
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            // 启用时假设有焦点
            self.last_focus_state = true;
            // 立即检测一次（重置计时器）
            self.last_check_time = 0;
        } else {
            // 禁用时确保overlay可见
            // PORT: Java 两处 `getInstance()` 取同一单例 → 同一字段两次访问
            if self.coordinator.is_overlays_hidden() {
                self.coordinator.show_all_overlays();
            }
        }
    }

    /// 检查焦点监控是否启用。
    ///
    /// @return true如果启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 由Service线程每次轮询时调用。
    /// 内部实现200ms节流，避免过于频繁的进程检测。
    pub fn tick(&mut self) {
        if !self.enabled {
            return;
        }

        // PORT: System.currentTimeMillis → SystemTime (§3 库映射,
        // event/flight_data_event.rs 同款): as_millis 的 u128 经 `as i64` 截断;
        // 时钟早于 epoch 时 duration_since 报错 → 取 0。now 与 last_check_time
        // 均为非负 epoch 毫秒, 差值不可能 i64 溢出, 普通 `-` 即可 (§2.2)。
        let now = current_time_millis();
        if now - self.last_check_time < CHECK_INTERVAL_MS {
            return;
        }
        self.last_check_time = now;

        // 检测焦点并响应变化
        let has_focus = self.detector.is_war_thunder_focused();

        if has_focus != self.last_focus_state {
            self.last_focus_state = has_focus;
            if has_focus {
                self.coordinator.show_all_overlays();
            } else {
                self.coordinator.hide_all_overlays();
            }
        }
    }
}

/// System.currentTimeMillis 的复刻 (语义说明见 tick 内 PORT 注)。
fn current_time_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// =====================================================================
// Tests — Java 侧无独立测试文件; 按"公共项写边界测试"规则, 以 mock
// FocusDetector / AlwaysOnTopCoordinatorApi 覆盖节流与显示/隐藏逻辑。
// 时间边界用白盒拨 last_check_time (排除调度抖动), 另留一条真实时钟端到端用例。
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::time::Duration;

    /// FocusDetector mock: 可配置焦点值 + 检测调用计数
    struct MockDetector {
        focus: AtomicBool,
        calls: AtomicUsize,
    }

    impl MockDetector {
        fn new(focus: bool) -> Self {
            MockDetector {
                focus: AtomicBool::new(focus),
                calls: AtomicUsize::new(0),
            }
        }

        fn set_focus(&self, focus: bool) {
            self.focus.store(focus, Ordering::SeqCst);
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl FocusDetector for MockDetector {
        fn is_war_thunder_focused(&self) -> bool {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.focus.load(Ordering::SeqCst)
        }
    }

    /// AlwaysOnTopCoordinatorApi mock: 调用记录 + overlaysHidden 标志
    /// (真实协调器由 hide/show 翻转该标志, mock 同构)
    struct MockCoordinator {
        calls: Mutex<Vec<String>>,
        hidden: AtomicBool,
    }

    impl MockCoordinator {
        fn new() -> Self {
            MockCoordinator {
                calls: Mutex::new(Vec::new()),
                hidden: AtomicBool::new(false),
            }
        }

        fn set_hidden(&self, hidden: bool) {
            self.hidden.store(hidden, Ordering::SeqCst);
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl AlwaysOnTopCoordinatorApi for MockCoordinator {
        fn is_overlays_hidden(&self) -> bool {
            self.hidden.load(Ordering::SeqCst)
        }

        fn hide_all_overlays(&self) {
            self.calls.lock().unwrap().push("hide".to_string());
            self.hidden.store(true, Ordering::SeqCst);
        }

        fn show_all_overlays(&self) {
            self.calls.lock().unwrap().push("show".to_string());
            self.hidden.store(false, Ordering::SeqCst);
        }
    }

    fn monitor(focus: bool) -> (FocusMonitor, Arc<MockDetector>, Arc<MockCoordinator>) {
        let detector = Arc::new(MockDetector::new(focus));
        let coordinator = Arc::new(MockCoordinator::new());
        let m = FocusMonitor::new(detector.clone(), coordinator.clone());
        (m, detector, coordinator)
    }

    /// 白盒: 把 last_check_time 拨到"已过去 interval 毫秒" (耗时只增不减 ⇒ 必放行)
    fn advance_past_interval(m: &mut FocusMonitor) {
        m.last_check_time = current_time_millis() - CHECK_INTERVAL_MS;
    }

    // -- Java 默认态: enabled=false, tick 直接短路 (不触检测不触协调器) --
    #[test]
    fn test_defaults_disabled_and_tick_short_circuits() {
        let (mut m, det, coord) = monitor(true);
        assert!(!m.is_enabled(), "Java `private boolean enabled = false` 默认值");
        m.tick();
        assert_eq!(det.calls(), 0, "未启用时 tick 立即返回, 不做进程检测");
        assert!(coord.calls().is_empty());
    }

    // -- set_enabled(true): 启用即假设有焦点 + 计时器清零 → 首个 tick 立即检测 --
    #[test]
    fn test_enable_resets_state_and_detects_immediately() {
        let (mut m, det, coord) = monitor(true);
        m.set_enabled(true);
        assert!(m.is_enabled());
        // lastCheckTime=0 ⇒ now-0 ≥ 200 恒成立, 无需真实等待
        m.tick();
        assert_eq!(det.calls(), 1, "计时器重置后首个 tick 立即检测");
        assert!(
            coord.calls().is_empty(),
            "lastFocusState 重置 true 与检测值 true 相等 ⇒ 无显示/隐藏动作"
        );
    }

    // -- set_enabled(false): 被隐藏则恢复, 未隐藏则不动 --
    #[test]
    fn test_disable_restores_hidden_overlays() {
        let (mut m, _det, coord) = monitor(true);
        coord.set_hidden(true);
        m.set_enabled(false);
        assert_eq!(coord.calls(), vec!["show".to_string()], "禁用时确保overlay可见");
        assert!(!coord.is_overlays_hidden());

        // 未隐藏时禁用: 不产生任何调用
        let (mut m2, _d2, coord2) = monitor(true);
        m2.set_enabled(false);
        assert!(coord2.calls().is_empty(), "overlay 本来可见 ⇒ 不调 showAllOverlays");
    }

    // -- 节流: 间隔内 (< 200ms) 的 tick 被吞 --
    // 白盒: lastCheckTime 拨到 1 小时后的未来, diff 为负恒 < 200, 排除调度抖动
    #[test]
    fn test_tick_throttled_within_interval() {
        let (mut m, det, coord) = monitor(false);
        m.set_enabled(true);
        m.tick();
        assert_eq!(det.calls(), 1);
        m.last_check_time = current_time_millis() + 3_600_000;
        m.tick();
        assert_eq!(det.calls(), 1, "间隔内的 tick 直接返回, 不做检测");
        assert_eq!(coord.calls(), vec!["hide".to_string()], "只有首次检测的失焦动作");
    }

    // -- 节流边界: 恰好 200ms (== CHECK_INTERVAL_MS) 不算间隔内, 检测放行 --
    // (Java `now - lastCheckTime < CHECK_INTERVAL_MS` 为严格小于)
    #[test]
    fn test_tick_boundary_exactly_interval_fires() {
        let (mut m, det, _coord) = monitor(true);
        m.set_enabled(true);
        advance_past_interval(&mut m);
        m.tick();
        assert_eq!(det.calls(), 1, "now - last == 200 不满足 < 200 ⇒ 放行");
    }

    // -- 真实时钟端到端: 间隔过后再次检测, 紧随其后的 tick 再被吞 --
    #[test]
    fn test_tick_realtime_throttle() {
        let (mut m, det, _coord) = monitor(true);
        m.set_enabled(true);
        m.tick();
        assert_eq!(det.calls(), 1);
        std::thread::sleep(Duration::from_millis(250));
        m.tick();
        assert_eq!(det.calls(), 2, "250ms > 200ms 间隔 ⇒ 再次检测");
        m.tick();
        assert_eq!(det.calls(), 2, "紧邻的下一 tick 落回间隔内 ⇒ 吞");
    }

    // -- 焦点变化: 失焦隐藏 / 回焦恢复 --
    #[test]
    fn test_focus_loss_hides_and_regain_shows() {
        let (mut m, det, coord) = monitor(false);
        m.set_enabled(true);
        m.tick(); // lastFocusState(true) → false: hide
        assert_eq!(coord.calls(), vec!["hide".to_string()]);
        assert!(coord.is_overlays_hidden());
        advance_past_interval(&mut m);
        det.set_focus(true);
        m.tick(); // false → true: show
        assert_eq!(
            coord.calls(),
            vec!["hide".to_string(), "show".to_string()],
            "变化分支按方向分派 show/hide"
        );
        assert!(!coord.is_overlays_hidden());
    }

    // -- 焦点不变: 不重复触发显示/隐藏 --
    #[test]
    fn test_no_action_when_focus_unchanged() {
        let (mut m, _det, coord) = monitor(false);
        m.set_enabled(true);
        m.tick(); // hide ×1
        advance_past_interval(&mut m);
        m.tick(); // 仍失焦: 无新动作
        assert_eq!(
            coord.calls(),
            vec!["hide".to_string()],
            "hasFocus == lastFocusState ⇒ 不动作 (但检测照常发生)"
        );
    }

    // -- 重启用语义: 禁用恢复后, 再启用假设有焦点 + 计时器清零 → 首个 tick 立即检测 --
    #[test]
    fn test_reenable_assumes_focus_then_redetects() {
        let (mut m, det, coord) = monitor(false);
        m.set_enabled(true);
        m.tick(); // 失焦: hide
        m.set_enabled(false); // 恢复: show
        assert_eq!(coord.calls(), vec!["hide".to_string(), "show".to_string()]);
        m.set_enabled(true); // lastFocusState 重置 true, lastCheckTime 清零
        det.set_focus(true);
        m.tick(); // 立即检测 (无需等 200ms); true == true ⇒ 不动作
        assert_eq!(det.calls(), 2, "计时器随重启用清零 ⇒ 立即检测");
        assert_eq!(coord.calls().len(), 2, "焦点与重置假设一致 ⇒ 无新动作");
        // 再失焦: 又一次 hide
        advance_past_interval(&mut m);
        det.set_focus(false);
        m.tick();
        assert_eq!(coord.calls()[2], "hide");
        assert_eq!(det.calls(), 3);
    }
}
