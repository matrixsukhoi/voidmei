use super::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

#[test]
fn basic_pubsub() {
    let bus: EventBus<u32> = EventBus::new();
    let hits = Arc::new(AtomicU32::new(0));
    let h2 = Arc::clone(&hits);
    let _sub = bus.subscribe(move |m| {
        h2.fetch_add(*m, Ordering::SeqCst);
    });
    assert_eq!(bus.publish(&5), 1);
    assert_eq!(hits.load(Ordering::SeqCst), 5);
    assert_eq!(bus.publish(&3), 1);
    assert_eq!(hits.load(Ordering::SeqCst), 8);
}

#[test]
fn drop_unsubscribes() {
    let bus: EventBus<u32> = EventBus::new();
    let hits = Arc::new(AtomicU32::new(0));
    let h2 = Arc::clone(&hits);
    let sub = bus.subscribe(move |_| {
        h2.fetch_add(1, Ordering::SeqCst);
    });
    bus.publish(&1);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    drop(sub); // RAII 注销
    assert_eq!(bus.publish(&1), 0);
    assert_eq!(bus.subscriber_count(), 0);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn multiple_listeners_in_order() {
    let bus: EventBus<&'static str> = EventBus::new();
    let order = Arc::new(Mutex::new(Vec::new()));
    let o1 = Arc::clone(&order);
    let _a = bus.subscribe(move |m| o1.lock().unwrap().push(format!("a:{}", m)));
    let o2 = Arc::clone(&order);
    let _b = bus.subscribe(move |m| o2.lock().unwrap().push(format!("b:{}", m)));
    bus.publish(&"x");
    assert_eq!(
        *order.lock().unwrap(),
        vec!["a:x".to_string(), "b:x".to_string()]
    );
}

#[test]
fn cross_thread_publish() {
    // 对齐 FlightDataBus 的跨线程用法: 主线程订阅, 工作线程发布
    let bus: Arc<EventBus<u32>> = Arc::new(EventBus::new());
    let bus2 = Arc::clone(&bus);
    let hits = Arc::new(Mutex::new(0u32));
    let h2 = Arc::clone(&hits);
    let _sub = bus.subscribe(move |m| {
        *h2.lock().unwrap() += *m;
    });
    let t = std::thread::spawn(move || {
        bus2.publish(&7);
    });
    t.join().unwrap();
    assert_eq!(*hits.lock().unwrap(), 7);
}
