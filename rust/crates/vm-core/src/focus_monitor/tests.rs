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
