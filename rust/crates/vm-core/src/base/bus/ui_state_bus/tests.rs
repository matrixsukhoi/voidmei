use super::*;
use crate::event::ui_state_events;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

#[test]
fn subscribe_and_publish_delivers_payload() {
    let bus = UIStateBus::new();
    let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let s2 = Arc::clone(&seen);
    let _sub = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |msg| {
        s2.lock().unwrap().push(msg.clone());
    });
    let delivered = bus.publish(
        ui_state_events::CONFIG_CHANGED,
        Some("ConfigurationService"),
        Some("showSpeedBar"),
    );
    assert_eq!(delivered, 1);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].event_type, ui_state_events::CONFIG_CHANGED);
    assert_eq!(seen[0].source.as_deref(), Some("ConfigurationService"));
    assert_eq!(seen[0].data.as_deref(), Some("showSpeedBar"));
}

// Java 按 eventType 路由: 订阅 CONFIG_CHANGED 者绝不能收到 FM_CHANGED
#[test]
fn routing_isolation_between_event_types() {
    let bus = UIStateBus::new();
    let hits = Arc::new(AtomicU32::new(0));
    let h2 = Arc::clone(&hits);
    let _sub = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |_| {
        h2.fetch_add(1, Ordering::SeqCst);
    });
    // FM_CHANGED 无订阅者 (真实场景发布线程 = FM-Loader, 见跨线程测试)
    assert_eq!(
        bus.publish(ui_state_events::FM_CHANGED, Some("FMManager"), None),
        0
    );
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    assert_eq!(
        bus.publish(
            ui_state_events::CONFIG_CHANGED,
            Some("ConfigurationService"),
            Some("k")
        ),
        1
    );
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn multiple_subscribers_in_subscription_order() {
    let bus = UIStateBus::new();
    let order: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let o1 = Arc::clone(&order);
    let _a = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |m| {
        o1.lock()
            .unwrap()
            .push(format!("a:{}", m.data.as_deref().unwrap_or("null")));
    });
    let o2 = Arc::clone(&order);
    let _b = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |m| {
        o2.lock()
            .unwrap()
            .push(format!("b:{}", m.data.as_deref().unwrap_or("null")));
    });
    // DynamicDataPage:252 的真实形态 (source=类简单名, data="ui_layout.cfg")
    bus.publish(
        ui_state_events::CONFIG_CHANGED,
        Some("DynamicDataPage"),
        Some("ui_layout.cfg"),
    );
    assert_eq!(
        *order.lock().unwrap(),
        vec![
            "a:ui_layout.cfg".to_string(),
            "b:ui_layout.cfg".to_string()
        ]
    );
}

// RAII Drop 注销 + 显式 unsubscribe + clear 后键不存在分支静默
#[test]
fn raii_drop_and_explicit_unsubscribe() {
    let bus = UIStateBus::new();
    let hits = Arc::new(AtomicU32::new(0));
    let h2 = Arc::clone(&hits);
    let sub = bus.subscribe(ui_state_events::UI_READY, move |_| {
        h2.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(bus.publish_legacy(ui_state_events::UI_READY, None), 1);
    drop(sub); // RAII Drop = 注销
    assert_eq!(bus.publish_legacy(ui_state_events::UI_READY, None), 0);

    let h3 = Arc::clone(&hits);
    let sub2 = bus.subscribe(ui_state_events::UI_READY, move |_| {
        h3.fetch_add(1, Ordering::SeqCst);
    });
    bus.unsubscribe(ui_state_events::UI_READY, sub2);
    assert_eq!(bus.publish_legacy(ui_state_events::UI_READY, None), 0);

    let h4 = Arc::clone(&hits);
    let sub3 = bus.subscribe(ui_state_events::UI_READY, move |_| {
        h4.fetch_add(1, Ordering::SeqCst);
    });
    bus.clear();
    // clear 后键不存在: Java handlers==null 分支 → 静默无日志 (此处验证不 panic)
    bus.unsubscribe(ui_state_events::UI_READY, sub3);
    assert_eq!(bus.publish_legacy(ui_state_events::UI_READY, None), 0);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

// Java publish(eventType, data) → publish(eventType, null, data);
// MainForm:390 publish(UI_READY, null) — source/data 双 null
#[test]
fn legacy_publish_null_source_and_data() {
    let bus = UIStateBus::new();
    let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let s2 = Arc::clone(&seen);
    let _sub = bus.subscribe(ui_state_events::UI_READY, move |m| {
        s2.lock().unwrap().push(m.clone());
    });
    assert_eq!(bus.publish_legacy(ui_state_events::UI_READY, None), 1);
    let got = seen.lock().unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].event_type, ui_state_events::UI_READY);
    assert_eq!(got[0].source, None);
    assert_eq!(got[0].data, None);
}

#[test]
fn panicking_handler_isolated_and_loop_continues() {
    let bus = UIStateBus::new();
    let after = Arc::new(AtomicU32::new(0));
    let a2 = Arc::clone(&after);
    let _bad = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |_| {
        panic!("handler 崩溃");
    });
    let _good = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |_| {
        a2.fetch_add(1, Ordering::SeqCst);
    });
    // 两个都送达 (第一个的 panic 被垫片吞掉, 不中断循环);
    // 垫片在监听器锁内收束 unwind → 不产生锁中毒, 后续发布不受影响
    assert_eq!(
        bus.publish(ui_state_events::CONFIG_CHANGED, Some("X"), Some("k")),
        2
    );
    assert_eq!(after.load(Ordering::SeqCst), 1);
    assert_eq!(
        bus.publish(ui_state_events::CONFIG_CHANGED, Some("X"), Some("k")),
        2
    );
    assert_eq!(after.load(Ordering::SeqCst), 2);
}

#[test]
fn clear_removes_all_subscribers_across_types() {
    let bus = UIStateBus::new();
    let hits = Arc::new(AtomicU32::new(0));
    let h2 = Arc::clone(&hits);
    let _a = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |_| {
        h2.fetch_add(1, Ordering::SeqCst);
    });
    let h3 = Arc::clone(&hits);
    let _b = bus.subscribe(ui_state_events::FM_CHANGED, move |_| {
        h3.fetch_add(1, Ordering::SeqCst);
    });
    bus.clear();
    assert_eq!(bus.publish(ui_state_events::CONFIG_CHANGED, None, None), 0);
    assert_eq!(bus.publish(ui_state_events::FM_CHANGED, None, None), 0);
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    // clear 后重订阅 (Java: 清空 map 后 computeIfAbsent 重建)
    let h4 = Arc::clone(&hits);
    let _c = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |_| {
        h4.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(bus.publish(ui_state_events::CONFIG_CHANGED, None, None), 1);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn in_flight_delivery_survives_clear_inside_handler() {
    let bus = Arc::new(UIStateBus::new());
    let got = Arc::new(AtomicU32::new(0));
    let g1 = Arc::clone(&got);
    let b1 = Arc::clone(&bus);
    let _first = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |_| {
        g1.fetch_add(1, Ordering::SeqCst);
        b1.clear(); // 迭代中清空
    });
    let g2 = Arc::clone(&got);
    let _second = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |_| {
        g2.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(
        bus.publish(ui_state_events::CONFIG_CHANGED, None, None),
        2
    );
    assert_eq!(got.load(Ordering::SeqCst), 2); // 第二个 handler 本轮仍被调用
    assert_eq!(
        bus.publish(ui_state_events::CONFIG_CHANGED, None, None),
        0
    ); // 下轮为 0
}

// §2.8: 回调在路由表锁外执行 — handler 内重入订阅/清空/发布**另一**
// 事件类型不死锁 (Java CHM/COW 无 monitor, 同步派发天然可重入)。
// PORT(重构波1 已修复): 同事件类型嵌套 publish 的死锁已由 UIStateBus 层
// thread_local 重入检测 + pending 延迟补投根治 (见模块文档) — 本测试的
// 跨类型嵌套走"立即递归"路径 (锁集合不相交, 对齐 Java 栈内同步); 同类型
// 嵌套走"批次末补投"路径, 生产链 (reset 链) handler 只认特定 payload,
// 无行为差。
#[test]
fn reentrant_publish_and_subscribe_inside_handler() {
    let bus = Arc::new(UIStateBus::new());
    let inner_hits = Arc::new(AtomicU32::new(0));
    let i2 = Arc::clone(&inner_hits);
    let _inner = bus.subscribe(ui_state_events::FM_CHANGED, move |_| {
        i2.fetch_add(1, Ordering::SeqCst);
    });
    let outer_hits = Arc::new(AtomicU32::new(0));
    let late_hits = Arc::new(AtomicU32::new(0));
    let o2 = Arc::clone(&outer_hits);
    let l2 = Arc::clone(&late_hits);
    let b_out = Arc::clone(&bus);
    let late_slot: Arc<Mutex<Option<Subscription<UiStateEvent>>>> =
        Arc::new(Mutex::new(None));
    let slot2 = Arc::clone(&late_slot);
    let _outer = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |_| {
        o2.fetch_add(1, Ordering::SeqCst);
        // 重入发布: handler 内发布另一事件 (Java 同步派发的嵌套执行)。
        // 跨类型 → 立即递归派发 (锁不相交; 同类型则入 pending 批次末补投)
        b_out.publish(ui_state_events::FM_CHANGED, Some("nested"), None);
        // 重入订阅: 回调内登记新订阅, 须存外部槽位保活 (否则即订即弃);
        // l2 须在闭包体内克隆 — 外层 FnMut 可能多次执行, 不能整值移出
        let b2 = Arc::clone(&b_out);
        let l3 = Arc::clone(&l2);
        *slot2.lock().unwrap() =
            Some(b2.subscribe(ui_state_events::CONFIG_CHANGED, move |_| {
                l3.fetch_add(1, Ordering::SeqCst);
            }));
    });
    bus.publish(ui_state_events::CONFIG_CHANGED, Some("t"), Some("k"));
    assert_eq!(outer_hits.load(Ordering::SeqCst), 1);
    assert_eq!(inner_hits.load(Ordering::SeqCst), 1); // 嵌套发布已送达
    assert_eq!(late_hits.load(Ordering::SeqCst), 0); // 本轮快照已过, 未调用
    bus.publish(ui_state_events::CONFIG_CHANGED, Some("t"), Some("k"));
    assert_eq!(outer_hits.load(Ordering::SeqCst), 2);
    assert_eq!(late_hits.load(Ordering::SeqCst), 1); // 重入订阅下轮生效
}

// LIFETIMES §2.2: FM_CHANGED 发布线程 = FM-Loader 后台线程, 订阅方照常收到
#[test]
fn cross_thread_publish_from_loader_like_thread() {
    let bus = Arc::new(UIStateBus::new());
    let got: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let g2 = Arc::clone(&got);
    let _sub = bus.subscribe(ui_state_events::FM_CHANGED, move |m| {
        g2.lock().unwrap().push(m.clone());
    });
    let b2 = Arc::clone(&bus);
    let t = std::thread::spawn(move || {
        b2.publish(ui_state_events::FM_CHANGED, Some("FMManager"), None);
    });
    t.join().unwrap();
    let got = got.lock().unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].event_type, ui_state_events::FM_CHANGED);
    assert_eq!(got[0].source.as_deref(), Some("FMManager"));
}

#[test]
fn self_unsubscribe_during_delivery_snapshot_semantics() {
    let bus = UIStateBus::new();
    let slot: Arc<Mutex<Option<Subscription<UiStateEvent>>>> = Arc::new(Mutex::new(None));
    let hits = Arc::new(AtomicU32::new(0));
    let s2 = Arc::clone(&slot);
    let h2 = Arc::clone(&hits);
    let sub = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |_| {
        h2.fetch_add(1, Ordering::SeqCst);
        // 自退订: 取出 RAII 句柄就地 Drop
        let taken = s2.lock().unwrap().take();
        drop(taken);
    });
    *slot.lock().unwrap() = Some(sub);
    let other = Arc::new(AtomicU32::new(0));
    let o2 = Arc::clone(&other);
    let _keep = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |_| {
        o2.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(
        bus.publish(ui_state_events::CONFIG_CHANGED, None, Some("k")),
        2
    );
    assert_eq!(hits.load(Ordering::SeqCst), 1); // 自退订者本轮仍送达
    assert_eq!(other.load(Ordering::SeqCst), 1);
    assert_eq!(
        bus.publish(ui_state_events::CONFIG_CHANGED, None, Some("k")),
        1
    ); // 下轮只剩另一个
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert_eq!(other.load(Ordering::SeqCst), 2);
}

// Java e.getMessage() 对位物: String 载荷 / &str 载荷 / 无文本载荷 → "null"
#[test]
fn panic_message_downcast_shapes() {
    assert_eq!(panic_message(Box::new("boom".to_string())), "boom");
    assert_eq!(panic_message(Box::new("static str")), "static str");
    assert_eq!(panic_message(Box::new(42u32)), "null");
}
