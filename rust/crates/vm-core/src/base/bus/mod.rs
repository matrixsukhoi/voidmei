//! 总线家族 (波8 收拢: 泛型基建 EventBus + flight_data_bus/ui_state_bus 同居;
//! 事件载荷类型在 base/event — 原三地分裂是"哪波改的就放哪"的历史痕迹)。
//!
//! 对齐 Java 语义: publish 为同步调用 (订阅者依次执行, 调用线程 = publish 线程,
//! 对齐 Java FlightDataBus.publish 在 Service 线程、订阅者自行转 UI 线程的模式);
//! RAII Subscription Drop 即注销 —— 根治 Java 版 VoiceWarning 式忘记 unregister 泄漏
//! (LIFETIMES.md §2 记录的现存 bug)。
//! 锁纪律 (LIFETIMES: Java OverlayEntry 锁内回调是死锁风险点): listeners 列表锁
//! 只在登记/清扫/快照时持有, 回调执行持的是各监听器自己的可变性锁, 二者不嵌套。
//! PORT(重构波1 裁决注): 同事件类型嵌套同步 publish 会死锁 (阶段 2 持监听器
//! Mutex 执行回调, 内层快照含执行中监听器 → 同线程二次 lock)。UIStateBus 层
//! 已以 thread_local 重入检测 + pending 延迟补投根治 (ui_state_bus.rs);
//! 本泛型层不动 — 强类型 FmChangedBus 的发布纪律 (锁外发布、禁再 publish)
//! 由 fm_manager 满足。

pub mod flight_data_bus;
pub mod ui_state_bus;

use std::sync::{Arc, Mutex, RwLock, Weak};

type Listener<M> = Mutex<Box<dyn FnMut(&M) + Send>>;

pub struct EventBus<M> {
    listeners: RwLock<Vec<Weak<Listener<M>>>>,
}

/// 订阅句柄: Drop 自动注销 (RAII); unsubscribe() 为显式退订 (对齐 Java 语义)
pub struct Subscription<M> {
    /// RAII 保活字段: Drop 即注销的机制载体, 天然不被读取
    // DEAD(kept): 字段存在即功能 (保活防 Weak 清扫), 读点即无意义
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
mod tests;
