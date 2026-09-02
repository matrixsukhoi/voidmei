use super::*;
use crate::base::event::event_payload::EventPayload;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

fn mk_event(map_grid: &str) -> FlightDataEvent {
    FlightDataEvent::new(EventPayload::builder().map_grid(map_grid.to_string()).build())
}

struct RecordingListener {
    seen: Mutex<Vec<String>>,
}

impl FlightDataListener for RecordingListener {
    fn on_flight_data(&self, event: &FlightDataEvent) {
        self.seen
            .lock()
            .unwrap()
            .push(event.get_payload().map_grid.clone());
    }
}

struct CountingListener {
    hits: Arc<AtomicU32>, // 计数经 Arc 外置, 测试侧可读 (trait 对象无法 downcast)
}

impl FlightDataListener for CountingListener {
    fn on_flight_data(&self, _event: &FlightDataEvent) {
        self.hits.fetch_add(1, Ordering::SeqCst);
    }
}

// register + publish: 订阅者按发布收到事件载荷, 返回送达数
#[test]
fn register_and_publish_delivers() {
    let bus = FlightDataBus::new();
    let hits = Arc::new(AtomicU32::new(0));
    let h2 = Arc::clone(&hits);
    let _sub = bus.register(move |_| {
        h2.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(bus.subscriber_count(), 1);
    assert_eq!(bus.publish(&mk_event("A1")), 1);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert_eq!(bus.publish(&mk_event("A2")), 1);
    assert_eq!(hits.load(Ordering::SeqCst), 2);
}

// Drop 订阅句柄 = Java unregister: 之后发布不再送达
#[test]
fn drop_subscription_unsubscribes() {
    let bus = FlightDataBus::new();
    let hits = Arc::new(AtomicU32::new(0));
    let h2 = Arc::clone(&hits);
    let sub = bus.register(move |_| {
        h2.fetch_add(1, Ordering::SeqCst);
    });
    bus.publish(&mk_event("B1"));
    drop(sub); // RAII 注销
    assert_eq!(bus.publish(&mk_event("B2")), 0);
    assert_eq!(bus.subscriber_count(), 0);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

// 显式 unsubscribe() 消费句柄, 与 Drop 等价 (Java unregister 的直接对应)
#[test]
fn explicit_unsubscribe() {
    let bus = FlightDataBus::new();
    let sub = bus.register(|_| {});
    assert_eq!(bus.subscriber_count(), 1);
    sub.unsubscribe();
    assert_eq!(bus.subscriber_count(), 0);
}

// 订阅顺序 = 调用顺序 (Java CopyOnWriteArrayList 插入序迭代)
#[test]
fn listeners_called_in_registration_order() {
    let bus = FlightDataBus::new();
    let order = Arc::new(Mutex::new(Vec::new()));
    let o1 = Arc::clone(&order);
    let _a = bus.register(move |e| {
        o1.lock().unwrap().push(format!("a:{}", e.get_payload().map_grid));
    });
    let o2 = Arc::clone(&order);
    let _b = bus.register(move |e| {
        o2.lock().unwrap().push(format!("b:{}", e.get_payload().map_grid));
    });
    bus.publish(&mk_event("O1"));
    assert_eq!(
        *order.lock().unwrap(),
        vec!["a:O1".to_string(), "b:O1".to_string()]
    );
}

// 同一事件引用 (同 timestamp/载荷) 送达全部订阅者, Java 单对象逐个传递语义
#[test]
fn same_event_to_all_listeners() {
    let bus = FlightDataBus::new();
    let g1 = Arc::new(Mutex::new((String::new(), 0i64)));
    let g2 = Arc::new(Mutex::new((String::new(), 0i64)));
    let s1 = Arc::clone(&g1);
    let _a = bus.register(move |e| {
        *s1.lock().unwrap() = (
            e.get_payload().map_grid.clone(),
            e.get_timestamp(),
        );
    });
    let s2 = Arc::clone(&g2);
    let _b = bus.register(move |e| {
        *s2.lock().unwrap() = (
            e.get_payload().map_grid.clone(),
            e.get_timestamp(),
        );
    });
    bus.publish(&mk_event("S1"));
    let v1 = g1.lock().unwrap().clone();
    let v2 = g2.lock().unwrap().clone();
    assert_eq!(v1.0, "S1");
    assert_eq!(v2.0, "S1");
    assert_eq!(v1.1, v2.1); // 同一事件对象的 timestamp
}

// Java publish per-listener catch: 订阅者 panic 不打断其余订阅者、
// 不逃逸到发布线程、且订阅保留 (下一轮仍被调用)
#[test]
fn panicking_listener_does_not_break_publish() {
    let bus = FlightDataBus::new();
    let calls = Arc::new(AtomicU32::new(0));
    let c2 = Arc::clone(&calls);
    let _bad = bus.register(move |_| {
        let n = c2.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            panic!("listener boom"); // 对齐 Java listener 抛 Exception
        }
    });
    let got = Arc::new(AtomicU32::new(0));
    let g2 = Arc::clone(&got);
    let _good = bus.register(move |_| {
        g2.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(bus.publish(&mk_event("P1")), 2);
    assert_eq!(got.load(Ordering::SeqCst), 1);
    // catch 后订阅者留在列表: 第二轮照常调用 (首轮已 panic 过)
    assert_eq!(bus.publish(&mk_event("P2")), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(got.load(Ordering::SeqCst), 2);
}

// 无订阅者发布: 返回 0, 无异常 (Java 空 CopyOnWriteArrayList 循环)
#[test]
fn publish_without_subscribers() {
    let bus = FlightDataBus::new();
    assert_eq!(bus.publish(&mk_event("N0")), 0);
}

// 回调线程 = 发布线程 (Java: Service 线程 publish, 监听器在 Service 线程执行)
#[test]
fn callback_runs_on_publishing_thread() {
    let bus = Arc::new(FlightDataBus::new());
    let seen_tid = Arc::new(Mutex::new(None));
    let s2 = Arc::clone(&seen_tid);
    let _sub = bus.register(move |_| {
        *s2.lock().unwrap() = Some(std::thread::current().id());
    });
    let main_tid = std::thread::current().id();
    let bus2 = Arc::clone(&bus);
    let handle = std::thread::spawn(move || {
        bus2.publish(&mk_event("T1"));
    });
    let worker_tid = handle.thread().id();
    handle.join().unwrap();
    assert_eq!(*seen_tid.lock().unwrap(), Some(worker_tid));
    assert_ne!(worker_tid, main_tid);
}

// register_listener: 已译 FlightDataListener trait 对象经适配注册, 逐轮收载荷
#[test]
fn register_listener_trait_object() {
    let bus = FlightDataBus::new();
    let l = Arc::new(RecordingListener {
        seen: Mutex::new(vec![]),
    });
    let _sub = bus.register_listener(l.clone());
    bus.publish(&mk_event("L1"));
    bus.publish(&mk_event("L2"));
    assert_eq!(*l.seen.lock().unwrap(), vec!["L1", "L2"]);
}

// Java register 的 contains 去重不移植: 同一对象注册两次 = 两个独立订阅,
// 各送达一次 (防重复注册由调用方持单 Subscription 表达, 见 register 注释)
#[test]
fn duplicate_registration_yields_two_subscriptions() {
    let bus = FlightDataBus::new();
    let hits = Arc::new(AtomicU32::new(0));
    let l: Arc<dyn FlightDataListener + Send + Sync> = Arc::new(CountingListener {
        hits: Arc::clone(&hits),
    });
    let _a = bus.register_listener(Arc::clone(&l));
    let _b = bus.register_listener(Arc::clone(&l));
    assert_eq!(bus.subscriber_count(), 2);
    assert_eq!(bus.publish(&mk_event("D1")), 2);
    assert_eq!(hits.load(Ordering::SeqCst), 2);
}

// 非全局单例裁决: 各实例独立, 互不串台 (实例归调用方持有)
#[test]
fn instances_are_independent_no_global() {
    let bus1 = FlightDataBus::new();
    let bus2 = FlightDataBus::new();
    let hits = Arc::new(AtomicU32::new(0));
    let h2 = Arc::clone(&hits);
    let _s1 = bus1.register(move |_| {
        h2.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(bus2.publish(&mk_event("I1")), 0); // bus2 无订阅者
    assert_eq!(bus1.publish(&mk_event("I2")), 1); // 只送达 bus1 的订阅者
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

// panic 载荷文本提取 (&str / String / 其它)
#[test]
fn panic_message_extraction() {
    assert_eq!(panic_message(Box::new("boom")), "boom");
    assert_eq!(panic_message(Box::new("fmt 42".to_string())), "fmt 42");
    assert_eq!(panic_message(Box::new(7i32)), "unknown");
}
