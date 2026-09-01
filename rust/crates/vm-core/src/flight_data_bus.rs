//! 对应 Java: `src/prog/event/FlightDataBus.java`
//!
//! Event Bus for Flight Data communication.
//! Used by Service (producer) to push flight physics updates to Overlays
//! (consumers).
//! Thread-safe using CopyOnWriteArrayList.
//! (以上为原 Java 类注释逐字保留; 实际承载见下方 PORT 说明)
//!
//! PORT (LIFETIMES §1.1/§2.3/§7 裁决, B 类适配层):
//! - Java 饿汉单例 (`static final INSTANCE` + 私有构造器 + `getInstance()`)
//!   → Rust 不做全局单例 (§2.9 禁 OnceLock 全局静态, 防状态分裂), 实例由
//!   调用方 (AppState/组装层) 持有; 跨 Controller 重建时传递同一 Arc 以对齐
//!   Java 单例"订阅跨重建存活"的语义。
//! - `register(listener)`/`unregister(listener)` → `register(...) -> Subscription`,
//!   RAII Drop 即注销 (订阅 guard 模式, 根治 LIFETIMES §2.1 记录的
//!   VoiceWarning 式忘记 unregister 泄漏)。显式退订用 `Subscription::unsubscribe()`。
//! - 底座 [`EventBus`] 的 publish 为同步快照迭代 (订阅顺序调用), 与 Java
//!   CopyOnWriteArrayList 迭代语义一致: 本轮发布开始后新增/退订的订阅者
//!   不影响已开始的遍历。

use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use crate::bus::{EventBus, Subscription};
use crate::event::flight_data_event::FlightDataEvent;
use crate::event::flight_data_listener::FlightDataListener;

/// Event Bus for Flight Data communication. (类注释见模块头, 逐字保留)
///
/// PORT: Java `private static final FlightDataBus INSTANCE = new FlightDataBus();`
/// 单例字段不移植 —— 实例归调用方持有 (见模块头 PORT 说明)。
/// PORT: Java `private final List<FlightDataListener> listeners =
/// new CopyOnWriteArrayList<>();` 由组合的底座 `EventBus<FlightDataEvent>`
/// 承担 (`RwLock<Vec<Weak<...>>>`, 读多写少 + 迭代中修改安全语义等价)。
pub struct FlightDataBus {
    inner: EventBus<FlightDataEvent>,
}

impl Default for FlightDataBus {
    fn default() -> Self {
        Self::new()
    }
}

impl FlightDataBus {
    /// 对应 Java `private FlightDataBus() {}`。
    /// PORT: 单例已废 (见模块头), 构造器转公有, 供调用方持有实例。
    pub fn new() -> Self {
        FlightDataBus {
            inner: EventBus::new(),
        }
    }

    /// 对应 Java `public void register(FlightDataListener listener)`。
    /// 返回 RAII 订阅句柄, Drop/`unsubscribe()` 即 Java `unregister`。
    ///
    /// PORT: Java 按对象引用 `contains` 去重防重复注册 —— Rust 无引用等值,
    /// 每次调用产生独立订阅; 防重复注册由调用方只持有一个 Subscription 表达
    /// (对齐 FieldOverlay `subscribed` 标志的既有防御模式)。
    /// PORT: Java `publish` 内 per-listener try-catch (§2.7 异常控制流) 移植为
    /// 订阅闭包内的 catch_unwind 护栏: 单个订阅者 panic 被吞掉并打印 stderr,
    /// 不打断其余订阅者、不逃逸到发布线程 (~10Hz 的 Service 轮询不被杀死),
    /// 订阅者不因异常被移除 (对齐 Java catch 后 listener 留存列表)。
    /// `e.printStackTrace()` ≈ 默认 panic hook 已在 stderr 打印位置与消息。
    pub fn register<F: FnMut(&FlightDataEvent) + Send + 'static>(
        &self,
        mut listener: F,
    ) -> Subscription<FlightDataEvent> {
        self.inner.subscribe(move |event| {
            // AssertUnwindSafe: 异常后订阅者留在可能不一致的状态继续用, 与 Java 同
            if let Err(panic_val) = catch_unwind(AssertUnwindSafe(|| listener(event))) {
                eprintln!(
                    "[FlightDataBus] Error in listener: {}",
                    panic_message(panic_val)
                );
            }
        })
    }

    /// [`register`] 的 Java 签名直接形态: 订阅一个已译的 [`FlightDataListener`]
    /// trait 对象 (Arc 共享, 调用方保活本体, 对齐 Java `register(this)`)。
    ///
    /// PORT (强引用环警示, LIFETIMES §6.3.1): 本方法与"订阅方自持 Subscription"
    /// 组合成环 —— 若监听者结构体 (如 overlay) 把返回的 Subscription 存回自身
    /// 字段, 则 overlay→Subscription→闭包→Arc<overlay> 成环永不释放, 每次进出
    /// 游戏模式泄漏一个实例 (VoiceWarning 式累积泄漏以新形态复活)。规避:
    /// 闭包捕获 `Weak<监听者>` 走 [`register`] 并在回调内 upgrade 判空, 或把
    /// Subscription 的存放点与监听者本体分离 (由拥有者/注册表持有, 而非监听者
    /// 自己存)。
    pub fn register_listener(
        &self,
        listener: Arc<dyn FlightDataListener + Send + Sync>,
    ) -> Subscription<FlightDataEvent> {
        // on_flight_data 取 &self → 闭包为 Fn, 经 Arc 跨发布线程共享
        self.register(move |event| listener.on_flight_data(event))
    }

    /// 对应 Java `public void publish(FlightDataEvent event)`。
    /// 同步依次调用存活订阅者; 回调线程 = 调用本方法的线程 (Service 线程),
    /// 订阅方碰 UI 须自行切线程 (对齐 Java 模式)。
    /// PORT: 返回送达数为 Rust 底座新增的诊断信息, Java 返回 void。
    ///
    /// PORT (重入死锁警戒, §2.8 Mutex 不可重入): 底座 publish 执行回调期间持有
    /// 该订阅者自己的回调 Mutex —— 订阅闭包内**禁止在同线程重入同一 bus 的
    /// publish**, 否则永久死锁。Java CopyOnWriteArrayList 迭代器无锁可重入
    /// (回调内递归 publish 通常以 StackOverflowError 收场), Rust 直接挂死。
    /// 当前 Java 全库唯一发布者是 Service, 无监听器重入 publish; 移植
    /// Service/overlay 时订阅闭包须把事件经 channel 转发到目标线程处理,
    /// 不得在回调内 (同线程) 直接再调 `publish`。
    pub fn publish(&self, event: &FlightDataEvent) -> usize {
        self.inner.publish(event)
    }

    /// 当前存活订阅者数 (诊断/测试用)。
    /// PORT: Rust 底座新增, Java 无对应成员。
    pub fn subscriber_count(&self) -> usize {
        self.inner.subscriber_count()
    }
}

/// panic 载荷 → 文本, 对齐 Java `"Error in listener: " + e.getMessage()` 的拼接。
/// PORT: `panic!("boom")` 载荷为 &str, `panic!("{}x", n)` 为 String, 其余无 Java
/// 对应 (Java 异常恒有类型名) → "unknown" 兜底。
fn panic_message(p: Box<dyn Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests;
