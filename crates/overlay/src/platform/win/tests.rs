use super::*; // EVENT_QUEUES/push_event/drain_event/remove_queue/create + 平台项
use crate::platform::OverlayWindow; // poll_event/set_topmost trait 方法

/// 串行化本模块测试: 共享静态队列表, cargo 默认并行会互相清场
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// 假 hwnd (仅作队列表 key, 不触碰真实窗口)
fn fake_hwnd(n: isize) -> HWND {
    HWND(n as *mut std::ffi::c_void)
}

fn clear_queues() {
    EVENT_QUEUES.lock().unwrap().clear();
}

fn queue_len(hwnd: HWND) -> usize {
    EVENT_QUEUES
        .lock()
        .unwrap()
        .get(&(hwnd.0 as isize))
        .map(|q| q.len())
        .unwrap_or(0)
}

/// 事件按 hwnd 分流, 窗口间不串台
#[test]
fn event_routing_isolated_per_hwnd() {
    let _guard = TEST_LOCK.lock().unwrap();
    clear_queues();
    let h1 = fake_hwnd(1);
    let h2 = fake_hwnd(2);
    push_event(
        h1,
        OverlayEvent::MousePress {
            root_x: 1,
            root_y: 1,
        },
    );
    push_event(
        h2,
        OverlayEvent::MousePress {
            root_x: 2,
            root_y: 2,
        },
    );
    push_event(h2, OverlayEvent::MouseRelease);
    assert_eq!(queue_len(h1), 1);
    assert_eq!(queue_len(h2), 2);
    // h1 的队列只有 h1 的事件
    assert_eq!(
        drain_event(h1),
        Some(OverlayEvent::MousePress {
            root_x: 1,
            root_y: 1
        })
    );
    assert_eq!(drain_event(h1), None);
    // h2 的队列完整且未受 h1 消费影响
    assert_eq!(
        drain_event(h2),
        Some(OverlayEvent::MousePress {
            root_x: 2,
            root_y: 2
        })
    );
    assert_eq!(drain_event(h2), Some(OverlayEvent::MouseRelease));
    assert_eq!(drain_event(h2), None);
}

/// MouseMove 合并: 连续 move 只保留最新 (per 队列独立)
#[test]
fn mousemove_merge_per_queue() {
    let _guard = TEST_LOCK.lock().unwrap();
    clear_queues();
    let h1 = fake_hwnd(1);
    push_event(
        h1,
        OverlayEvent::MouseMove {
            root_x: 1,
            root_y: 1,
            left_down: true,
        },
    );
    push_event(
        h1,
        OverlayEvent::MouseMove {
            root_x: 5,
            root_y: 6,
            left_down: true,
        },
    );
    assert_eq!(queue_len(h1), 1);
    assert_eq!(
        drain_event(h1),
        Some(OverlayEvent::MouseMove {
            root_x: 5,
            root_y: 6,
            left_down: true
        })
    );
    // 非相邻事件间不合并
    push_event(h1, OverlayEvent::MouseRelease);
    push_event(
        h1,
        OverlayEvent::MouseMove {
            root_x: 9,
            root_y: 9,
            left_down: false,
        },
    );
    assert_eq!(queue_len(h1), 2);
}

/// 队列条目移除后滞留事件丢弃
#[test]
fn remove_queue_drops_pending() {
    let _guard = TEST_LOCK.lock().unwrap();
    clear_queues();
    let h1 = fake_hwnd(1);
    push_event(h1, OverlayEvent::Close);
    remove_queue(h1);
    assert_eq!(drain_event(h1), None);
}

/// 真实窗口冒烟 = POC 单窗口路径回归 (window.rs run/run_live 的同一路径:
/// create → present → poll_event → drop), 兼 hwnd 条目登记/移除核对
#[test]
fn real_window_smoke_single_poc_path() {
    let _guard = TEST_LOCK.lock().unwrap();
    clear_queues();
    let mut win = create(WindowConfig {
        width: 8,
        height: 8,
        x: 0,
        y: 0,
        click_through: true,
    })
    .expect("创建真实窗口失败");
    // 条目已登记
    assert_eq!(queue_len(win.hwnd), 0);
    assert!(EVENT_QUEUES
        .lock()
        .unwrap()
        .contains_key(&(win.hwnd.0 as isize)));
    // present 全透明帧 (8x8 预乘 BGRA)
    win.present(&vec![0u8; 8 * 8 * 4]).expect("present 失败");
    // 无消息时 poll 为空
    assert_eq!(win.poll_event(), None);
    // 置顶切换 (AlwaysOnTopCoordinator 底层动作)
    win.set_topmost(false);
    win.set_topmost(true);
    let hwnd = win.hwnd;
    drop(win);
    // Drop 后条目移除
    assert!(!EVENT_QUEUES
        .lock()
        .unwrap()
        .contains_key(&(hwnd.0 as isize)));
}

/// 多真实窗口: 同进程二次 create (同类重注册) + 各自队列条目独立
#[test]
fn multi_real_windows_isolated_queues() {
    let _guard = TEST_LOCK.lock().unwrap();
    clear_queues();
    let w1 = create(WindowConfig {
        width: 4,
        height: 4,
        x: 0,
        y: 0,
        click_through: true,
    })
    .expect("第一个窗口创建失败");
    let w2 = create(WindowConfig {
        width: 4,
        height: 4,
        x: 20,
        y: 0,
        click_through: true,
    })
    .expect("第二个窗口创建失败 (RegisterClassW 重注册应被容忍)");
    {
        let map = EVENT_QUEUES.lock().unwrap();
        assert!(map.contains_key(&(w1.hwnd.0 as isize)));
        assert!(map.contains_key(&(w2.hwnd.0 as isize)));
        assert_ne!(w1.hwnd.0, w2.hwnd.0);
    }
    let h1 = w1.hwnd;
    drop(w1);
    assert!(!EVENT_QUEUES.lock().unwrap().contains_key(&(h1.0 as isize)));
    assert!(EVENT_QUEUES
        .lock()
        .unwrap()
        .contains_key(&(w2.hwnd.0 as isize)));
    drop(w2);
}
