//! 事件总线基建: FlightDataBus/UIStateBus 的 Rust 化 (B 类适配层, 主 agent 设计)
//!
//! 对齐 Java 语义: publish 为同步调用 (订阅者依次执行, 调用线程 = publish 线程,
//! 对齐 Java FlightDataBus.publish 在 Service 线程、订阅者自行转 EDT 的模式);
//! RAII Subscription Drop 即注销 —— 根治 Java 版 VoiceWarning 式忘记 unregister 泄漏
//! (LIFETIMES.md §2 记录的现存 bug)。
//! 锁纪律 (LIFETIMES: Java OverlayEntry 锁内回调是死锁风险点): listeners 列表锁
//! 只在登记/清扫/快照时持有, 回调执行持的是各监听器自己的可变性锁, 二者不嵌套。

use std::sync::{Arc, Mutex, RwLock, Weak};

type Listener<M> = Mutex<Box<dyn FnMut(&M) + Send>>;

pub struct EventBus<M> {
    listeners: RwLock<Vec<Weak<Listener<M>>>>,
}

/// 订阅句柄: Drop 自动注销 (RAII); unsubscribe() 为显式退订 (对齐 Java 语义)
pub struct Subscription<M> {
    /// RAII 保活字段: Drop 即注销的机制载体, 天然不被读取
    #[allow(dead_code)]
    listener: Arc<Listener<M>>,
}

impl<M> Subscription<M> {
    pub fn unsubscribe(self) {
        drop(self)
    }
}

impl<M> Default for EventBus<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> EventBus<M> {
    pub fn new() -> Self {
        EventBus {
            listeners: RwLock::new(Vec::new()),
        }
    }

    /// 订阅: 返回 RAII 句柄。回调要求 Send (可能在任意 publish 线程执行)。
    pub fn subscribe<F: FnMut(&M) + Send + 'static>(&self, mut f: F) -> Subscription<M> {
        let listener: Arc<Listener<M>> = Arc::new(Mutex::new(Box::new(move |m: &M| f(m))));
        self.listeners
            .write()
            .expect("bus 列表锁中毒")
            .push(Arc::downgrade(&listener));
        Subscription { listener }
    }

    /// 发布: 同步依次调用存活订阅者 (调用顺序 = 订阅顺序), 顺带清扫死引用。
    /// 返回送达数 (诊断用)。
    pub fn publish(&self, msg: &M) -> usize {
        // 阶段 1: 持列表锁做升级快照 + 清扫死引用, 随即释放锁
        let alive: Vec<Arc<Listener<M>>> = {
            let mut list = self.listeners.write().expect("bus 列表锁中毒");
            let mut alive = Vec::with_capacity(list.len());
            list.retain(|w| match w.upgrade() {
                Some(arc) => {
                    alive.push(arc);
                    true
                }
                None => false, // 订阅者已 Drop
            });
            alive
        };
        // 阶段 2: 列表锁已释放, 逐监听器持各自可变性锁执行回调 (锁不嵌套)
        let mut delivered = 0;
        for listener in alive {
            let mut cb = listener.lock().expect("bus 回调锁中毒");
            cb(msg);
            delivered += 1;
        }
        delivered
    }

    /// 当前存活订阅者数 (诊断/测试用)
    pub fn subscriber_count(&self) -> usize {
        self.listeners
            .read()
            .expect("bus 列表锁中毒")
            .iter()
            .filter(|w| w.upgrade().is_some())
            .count()
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(*order.lock().unwrap(), vec!["a:x".to_string(), "b:x".to_string()]);
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
}
