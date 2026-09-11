//! 总线家族黑盒场景 (kernel::base::bus)。
//! 泛型 EventBus 的订阅/退订/送达计数 + UIStateBus 路由与同类型嵌套补投
//! (Java reset 链语义) + FlightDataBus 派发与 RAII 注销。
#![allow(non_snake_case)] // 测试名 = 模块前缀英文 + 中文场景 (风格约定, 豁免蛇形)

use std::sync::{Arc, Mutex};

use kernel::base::bus::flight_data_bus::FlightDataBus;
use kernel::base::bus::ui_state_bus::UIStateBus;
use kernel::base::bus::EventBus;
use kernel::base::event::event_payload::EventPayload;
use kernel::base::event::flight_data_event::FlightDataEvent;
use kernel::base::event::ui_state_events;

/// EventBus: 订阅 → publish 返回送达数; Drop 退订 / 显式 unsubscribe 双路
#[test]
fn eventbus_订阅退订与送达计数() {
    let bus: EventBus<u32> = EventBus::new();
    assert_eq!(bus.subscriber_count(), 0);

    let hits = Arc::new(Mutex::new(Vec::<u32>::new()));
    let mk = |hits: &Arc<Mutex<Vec<u32>>>| {
        let h = Arc::clone(hits);
        move |m: &u32| {
            h.lock().unwrap().push(*m);
        }
    };

    let s1 = bus.subscribe(mk(&hits));
    let s2 = bus.subscribe(mk(&hits));
    assert_eq!(bus.subscriber_count(), 2);

    assert_eq!(bus.publish(&7), 2, "两个存活订阅者都送达");
    // Drop (RAII) 退订 — 对齐 Java 忘记 unregister 的泄漏根治
    drop(s1);
    assert_eq!(bus.publish(&8), 1);
    assert_eq!(bus.subscriber_count(), 1);

    // 显式 unsubscribe (对位 Java 语义)
    s2.unsubscribe();
    assert_eq!(bus.subscriber_count(), 0);
    assert_eq!(bus.publish(&9), 0, "无订阅者送达 0");
    assert_eq!(*hits.lock().unwrap(), vec![7, 7, 8]);
}

/// EventBus: 回调按订阅顺序同步执行 (Java COW 快照迭代语义)
#[test]
fn eventbus_订阅顺序同步执行() {
    let bus: EventBus<&'static str> = EventBus::new();
    let order = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let s_a = {
        let o = Arc::clone(&order);
        bus.subscribe(move |m: &&'static str| o.lock().unwrap().push(m))
    };
    let s_b = {
        let o = Arc::clone(&order);
        bus.subscribe(move |m: &&'static str| o.lock().unwrap().push(m))
    };
    bus.publish(&"x"); // 同线程同步: publish 返回时回调已全部执行完
    let got = order.lock().unwrap().clone();
    assert_eq!(got, vec!["x", "x"]);
    drop((s_a, s_b));
}

/// UIStateBus: 事件类型路由 (A 订阅者不收 B) + 订阅回调内再 publish 同类型
/// 的嵌套安全 — 同类型嵌套入 pending 队列, 最外层派发完成后补投 (不死锁;
/// Java reset 链: publish(CONFIG_CHANGED, RESET_REQUEST) → handler 内再
/// publish(CONFIG_CHANGED, RESET_COMPLETED))
#[test]
fn ui_state_bus_路由与同类型嵌套补投() {
    let bus = Arc::new(UIStateBus::new());

    // 订阅者 A: 收到 typeA 后再发一条同类型 (嵌套) + 一条跨类型
    let inner_hits = Arc::new(Mutex::new(0u32));
    let nested_hits = Arc::new(Mutex::new(0u32));
    let bus2 = Arc::clone(&bus);
    let ih = Arc::clone(&inner_hits);
    let _sub_a = bus.subscribe("typeA", move |msg: &kernel::base::bus::ui_state_bus::UiStateEvent| {
        *ih.lock().unwrap() += 1;
        if msg.data.as_deref() == Some("outer") {
            // 同类型嵌套: 入 pending, 最外层完成后补投 (不再递归进本回调)
            bus2.publish("typeA", Some("inner"), Some("nested"));
            // 跨类型嵌套: 锁集合不相交, 立即递归派发
            bus2.publish("typeB", Some("inner"), Some("cross"));
        }
    });
    let _sub_b = bus.subscribe("typeB", {
        let nh = Arc::clone(&nested_hits);
        move |_msg| {
            *nh.lock().unwrap() += 1;
        }
    });

    assert_eq!(bus.subscriber_count("typeA"), 1);
    let delivered = bus.publish("typeA", Some("test"), Some("outer"));

    assert_eq!(*inner_hits.lock().unwrap(), 2, "外层 1 次 + 同类型嵌套补投 1 次");
    assert_eq!(*nested_hits.lock().unwrap(), 1, "跨类型嵌套立即派发");
    assert_eq!(delivered, 2, "补投送达数计入最外层 publish 返回值");

    // 路由隔离: typeB 的发布不会落进 typeA 订阅者
    let before = *inner_hits.lock().unwrap();
    bus.publish("typeB", Some("test"), Some("only-b"));
    assert_eq!(*inner_hits.lock().unwrap(), before, "typeA 订阅者不收 typeB");
}

/// FlightDataBus: 派发载荷 + subscriber_count + RAII Drop 注销
#[test]
fn flight_data_bus_派发与RAII注销() {
    let bus = FlightDataBus::new();
    let got_grids = Arc::new(Mutex::new(Vec::<String>::new()));

    let g = Arc::clone(&got_grids);
    let sub = bus.register(move |e: &FlightDataEvent| {
        g.lock().unwrap().push(e.get_payload().map_grid.clone());
    });
    assert_eq!(bus.subscriber_count(), 1);

    let payload = EventPayload::builder().map_grid("C4".into()).build();
    let event = FlightDataEvent::new(payload);
    assert_eq!(bus.publish(&event), 1);

    // 事件只承载标量 payload + 时间戳 (W-B 瘦身后的契约面)
    assert_eq!(event.get_payload().map_grid, "C4");
    assert!(event.get_timestamp() > 0);

    drop(sub); // RAII 注销 = Java unregister
    assert_eq!(bus.subscriber_count(), 0);
    assert_eq!(bus.publish(&event), 0);
    assert_eq!(*got_grids.lock().unwrap(), vec!["C4".to_string()]);
}

/// UIStateBus 常量面: 路由键与 ui_state_events 常量一致 (CONFIG_CHANGED 等)
#[test]
fn ui_state_events_常量路由() {
    let bus = UIStateBus::new();
    let hits = Arc::new(Mutex::new(Vec::<String>::new()));
    let h = Arc::clone(&hits);
    let _sub = bus.subscribe(
        ui_state_events::CONFIG_CHANGED,
        move |msg: &kernel::base::bus::ui_state_bus::UiStateEvent| {
            h.lock().unwrap()
                .push(format!("{:?}|{:?}", msg.event_type, msg.data));
        },
    );
    bus.publish(
        ui_state_events::CONFIG_CHANGED,
        Some("ConfigurationService"),
        Some("crosshairSwitch"),
    );
    assert_eq!(
        *hits.lock().unwrap(),
        vec![format!("{:?}|{:?}", "configChanged", Some("crosshairSwitch".to_string()))]
    );
}
