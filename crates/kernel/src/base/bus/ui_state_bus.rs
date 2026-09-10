//! 对应 Java: `src/prog/event/UIStateBus.java`
//!
//! Event Bus for UI State synchronization.
//! Used for decoupled communication between UI panels (e.g., switch State sync).
//! Thread-safe using ConcurrentHashMap + CopyOnWriteArrayList.
//!
//! B 类适配总览 (主 agent 裁决, 底座 = crate::base::bus 的 EventBus/Subscription):
//! - Java `ConcurrentHashMap<String, List<Consumer<Object>>>` 的按 eventType
//!   路由 → 每事件类型一条 `Arc<EventBus<UiStateEvent>>` (路由保真: 订阅方
//!   只收自己订阅的类型, 等价 Java 先按类型查表再调用, 无广播串台);
//! - `publish(eventType, source, data)` 弱类型三元组 → 消息 struct 化
//!   `UiStateEvent { event_type, source, data }` (事件名 + sourceId + payload);
//! - Java `catch (Exception)` 的逐 handler 异常隔离 (单个 handler 崩溃记日志
//!   后继续下一个) → 订阅时包 catch_unwind 垫片;
//! - §2.8 锁重入: 路由表锁内只做查表/登记/清空, 回调一律在锁外执行 ——
//!   handler 内可重入 subscribe/clear/**跨事件类型** publish 不死锁 (对齐
//!   Java CHM/COW 无 monitor 的可重入语义);
//!   **同事件类型的嵌套同步 publish 曾死锁** —
//!   bus.rs publish 持各监听器自己的 Mutex 执行回调 (bus.rs "阶段 2"), 内层
//!   同类型 publish 的升级快照必含正在执行的监听器 → 同线程对同一
//!   std::sync::Mutex 二次 lock 永久阻塞 (Java 无 monitor 天然可重入)。
//!   Java 真实调用链: ButtonRowRenderer publish(CONFIG_CHANGED,
//!   ACTION_RESET_REQUEST) → handler 同步调 resetAllLayoutDefaults() →
//!   嵌套同步 publish(CONFIG_CHANGED, ACTION_RESET_COMPLETED)。修复在本层
//!   (bus.rs 泛型不动, FmChangedBus 纪律完好): thread_local 重入检测 —
//!   派发中的事件类型再 publish 即入 pending 队列, 最外层派发完成后排空
//!   (见 [`UIStateBus::publish`] 的补投语义); 跨类型嵌套保持立即递归
//!   (对齐 Java 的栈内同步执行)。
//! - §2.9 全局态: Java `getInstance()` 静态单例解散, 实例由 App 层拥有并经
//!   构造器注入 (configuration_service.rs 的总线注入先例)。

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, RwLock};

use crate::base::bus::{EventBus, Subscription};
use crate::base::exception_helper::panic_message_box;
use crate::base::logger;

/// 路由表锁中毒消息 (Java 无锁; 对应持锁线程崩溃后的一致性未知面)
const MAP_LOCK_MSG: &str = "UIStateBus 路由表锁中毒";

/// 同类型嵌套 publish 的 pending 队列深度上限 (防 handler 对拍乒乓打爆内存;
/// Java 等价链路 (reset 链) 深度恒为 1, 64 已极宽裕)
const MAX_PENDING: usize = 64;

// 线程局部重入状态:
// - `depth`: publish 调用栈深度 (1 = 最外层, 负责排空 pending)
// - `in_flight`: 正在派发中的事件类型集合 — 同类型再 publish 即入队
// - `pending`: 延迟补投队列 (最外层派发完成后按序排空)
// (clippy 1.97 的 missing_const_for_thread_local 对已含 const 块的本形态
// 恒误报 — workspace doc_lazy_continuation 同款豁免先例)
thread_local! {
    #[allow(clippy::missing_const_for_thread_local)]
    static REENTRY: RefCell<(usize, Vec<String>, VecDeque<UiStateEvent>)> = const { RefCell::new((0, Vec::new(), VecDeque::new())) };
}

/// UIStateBus 消息: Java `publish(eventType, source, data)` 弱类型三元组的
/// 强类型化 (消息 struct 化)。
///
/// PORT(重构波1): 本类型是全 crate 唯一定义点 — configuration_service.rs
/// 的同名依赖桩 (裸 String 三字段) 已删, 该服务的总线注入改为
/// `Option<Arc<UIStateBus>>` (路由总线), 发布统一走本类型三参形态。
/// 消息三字段均 String 域 (publish(event, sourceId, configKey) 裁决);
/// Java payload 的异构类型 (FM_CHANGED=FMHandle, FM_PRINT_SWITCH_CHANGED=
/// Boolean, FM_OVERLAY_TOGGLE=Integer) 暂以字符串/None 顶位 (FM_CHANGED
/// 实际走 FmChangedBus 强类型专用通道, 见 fm_manager.rs)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiStateEvent {
    /// 事件类型标识 (Java eventType, 总线路由键; Java 映射键不可能为 null)
    pub event_type: String,
    /// 发布者标识 (Java source: Object, 仅供日志, Java handler 不可见;
    /// Rust 由调用方给字符串 sourceId, 对位 Java 反射取的类简单名)
    pub source: Option<String>,
    /// 事件 payload (Java data: Object, 可 null → Option;
    /// CONFIG_CHANGED=String 配置键/ACTION_RESET_*, UI_READY/VOICE_PACKS_REFRESH=null)
    pub data: Option<String>,
}

/// Event Bus for UI State synchronization.
/// Used for decoupled communication between UI panels (e.g., switch State sync).
/// Thread-safe using ConcurrentHashMap + CopyOnWriteArrayList.
pub struct UIStateBus {
    /// Java: `private final Map<String, List<Consumer<Object>>> subscribers
    /// = new ConcurrentHashMap<>();` — 键 = 事件类型, 值 = 该类型的监听器表
    /// (Arc<EventBus> 提供 Weak+RAII 语义, 对位 CopyOnWriteArrayList 的
    /// 读多写少 + 迭代快照)。映射只做 get/entry/contains_key/clear, 从不迭代
    ///。
    /// Java `private static final UIStateBus INSTANCE` 单例字段随
    /// getInstance() 一并解散 — 实例由 App 层拥有并注入, 不造全局。
    map: RwLock<HashMap<String, Arc<EventBus<UiStateEvent>>>>,
}

impl Default for UIStateBus {
    fn default() -> Self {
        Self::new()
    }
}

impl UIStateBus {
    /// 对应 Java `private UIStateBus()` (私有构造) + `getInstance()`。
    /// 单例读法 → 显式构造。
    pub fn new() -> Self {
        UIStateBus {
            map: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribe to an event type.
    ///
    /// - `eventType`: The event type identifier (e.g., "fmPrintSwitchChanged")
    /// - `handler`: The handler to invoke when the event is published
    ///
    /// Java `Consumer<Object>` 只收 data → Rust 回调收整个
    /// `&UiStateEvent` (消息 struct 化), 订阅方按需取 `msg.data`;
    /// 返回 RAII 订阅句柄, Drop 即注销 (对位 Java 靠 unsubscribe 防泄漏)。
    pub fn subscribe<F>(&self, event_type: &str, handler: F) -> Subscription<UiStateEvent>
    where
        F: FnMut(&UiStateEvent) + Send + 'static,
    {
        // 写锁覆盖登记全程 (锁内无回调执行, EventBus::subscribe 只压 Weak, §2.8 安全);
        // 此写法顺带免除了 Java 版 "computeIfAbsent 与并发 clear 间让 handler
        // 落入孤儿表" 的窗口 (不可观察差异, 因 clear() 全库无调用方)。
        let bus = Arc::clone(
            self.map
                .write()
                .expect(MAP_LOCK_MSG)
                .entry(event_type.to_string())
                .or_insert_with(|| Arc::new(EventBus::new())),
        );

        // 垫片: Java publish 循环体的 try { handler.accept(data); }
        // catch (Exception e) { 记日志继续 } — Rust 闭包不抛异常, 以 panic
        // 为对位物逐 handler 隔离 (不中断循环、不杀发布线程)。
        // PORT(保真偏差备案, panic≡exception 等价口径): (1) 捕获面比 Java 宽 —
        // Java catch(Exception) 不捕 Error (StackOverflowError 会击穿循环杀
        // 线程), catch_unwind 捕一切 unwind panic; (2) 捕获前默认 panic hook
        // 已向 stderr 打印 panicked 消息 — Java 捕获后不打堆栈, Rust 侧每
        // handler 异常多一份 stderr 噪音 (e2e 日志断言若收紧会踩);
        // set_hook 是进程级全局副作用, 库 crate 不做, 交 App 层定夺。
        // 另: bus.rs 的 delivered 计数把 panic 的 handler 也计为送达 (Rust
        // 纯增量诊断, Java void 无此概念)。
        let et = event_type.to_string();
        let mut handler = handler;
        let sub = bus.subscribe(move |msg: &UiStateEvent| {
            if let Err(payload) = catch_unwind(AssertUnwindSafe(|| handler(msg))) {
                // + eventType + ": " + e.getMessage());
                logger::error(
                    "UIStateBus",
                    &format!(
                        "Error in handler for {}: {}",
                        et,
                        panic_message_box(payload)
                    ),
                );
            }
        });

        // source 位 Java 反射取 handler 类简单名 (匿名类如 Controller$$Lambda),
        // Rust 闭包无名且禁引反射 → None (日志渲染 "Unknown")
        logger::event("SUBSCRIBE", event_type, None, None);
        sub
    }

    /// Unsubscribe a handler from an event type.
    ///
    /// Java 以 Consumer 引用等值 remove; Rust 闭包无身份 → 以 RAII 句柄
    /// Drop 注销 (bus.rs 机制)。event_type 仅复刻 Java 的 "键存在才记日志"
    /// 分支 (handlers != null); 若调用方传了与订阅不符的类型, Java 只是无操作,
    /// Rust 侧句柄所有权仍会被消费 (等价于退订成功) — 全库无此错用形态。
    /// 翻译 Controller.stop() 五步退订时注意:
    /// 其依赖的 '键不存在则跳过日志' 分支在此仍成立, 但 '错配键 = 无操作'
    /// 不成立 (退订总会发生)。
    pub fn unsubscribe(&self, event_type: &str, handler: Subscription<UiStateEvent>) {
        //       if (handlers != null) { ... }
        let known = self
            .map
            .read()
            .expect(MAP_LOCK_MSG)
            .contains_key(event_type);
        // 空 List 也留在 map 里, 永不移除键)
        drop(handler);
        if known {
            logger::event("UNSUBSCRIBE", event_type, None, None);
        }
    }

    /// Publish an event to all subscribers with explicit source.
    ///
    /// - `eventType`: The event type identifier
    /// - `source`: The object initiating the event (for logging)
    /// - `data`: The event payload
    ///
    /// 返回送达数 (bus.rs 诊断扩展; Java void)。同步派发: 调用线程 =
    /// 发布线程逐个执行订阅方代码 (对齐 Java)。
    ///
    /// 同事件类型嵌套同步 publish 不再死锁 —
    /// 派发中的类型再 publish 时入 pending 队列, 由**最外层** publish 在
    /// 直接派发完成后按序排空补投 (送达时机由 Java 的"栈内立即"变为
    /// "当前批次末"; Java 真实链 (reset 链) 的 handler 只认特定 payload,
    /// 无行为差)。跨类型嵌套保持立即递归执行 (锁集合不相交, 对齐 Java)。
    pub fn publish(&self, event_type: &str, source: Option<&str>, data: Option<&str>) -> usize {
        // PORT(日志渲染分歧备案): Java Logger.event 用 source.getClass()
        // .getSimpleName(), 调用方传 String source 的点 (ConfigurationService /
        // DynamicDataPage / ButtonRowRenderer) 实际打出 "String" 类简单名而非
        // 字符串内容; Rust 调用方传 Some("ConfigurationService") 打出内容。本转发
        // 行为与 Java 一致 (分歧源于 logger.rs 已接受的字符串 source 设计),
        // 翻译上述调用方时须知此偏差。
        logger::event("PUBLISH", event_type, source, data);

        let ev = UiStateEvent {
            event_type: event_type.to_string(),
            source: source.map(|s| s.to_string()),
            data: data.map(|d| d.to_string()),
        };

        // 重入判定: 处于派发中 (depth > 1) 且本类型正被派发 → 入队延迟补投
        let reentry = REENTRY.with(|r| {
            let mut r = r.borrow_mut();
            let (depth, in_flight, pending) = &mut *r;
            if *depth > 0 && in_flight.iter().any(|t| t == event_type) {
                if pending.len() >= MAX_PENDING {
                    // 防乒乓上限: 丢弃并记错 (Java 无此形态, 对拍链深度恒 1)
                    logger::error(
                        "UIStateBus",
                        &format!(
                            "pending 队列超上限 {MAX_PENDING}, 丢弃嵌套事件: {}",
                            ev.event_type
                        ),
                    );
                } else {
                    pending.push_back(ev.clone());
                }
                true
            } else {
                false
            }
        });
        if reentry {
            return 0; // 补投送达数计入最外层调用
        }

        // 直接派发 (跨类型嵌套在此立即递归, 对齐 Java 栈内同步)
        let mut delivered = self.dispatch_now(&ev);

        // 最外层: 排空 pending (排空中新嵌套继续入队, 循环至清空)
        let is_outermost = REENTRY.with(|r| r.borrow().0 == 1);
        if is_outermost {
            loop {
                let next = REENTRY.with(|r| r.borrow_mut().2.pop_front());
                match next {
                    Some(ev) => {
                        logger::event(
                            "PUBLISH",
                            &ev.event_type,
                            ev.source.as_deref(),
                            ev.data.as_deref(),
                        );
                        delivered += self.dispatch_now(&ev);
                    }
                    None => break,
                }
            }
        }
        delivered
    }

    /// 单条消息的直接派发 (查表 + bus.rs 两阶段 publish); 前后维护 in_flight
    /// 集合供同类型重入判定。
    fn dispatch_now(&self, ev: &UiStateEvent) -> usize {
        // 持读锁只做查表克隆随即放锁 — 回调在锁外执行; 迭代快照语义
        // 由 bus.rs publish 的升级快照承担 (对位 Java COW for 循环起始取快照)。
        let bus = self
            .map
            .read()
            .expect(MAP_LOCK_MSG)
            .get(ev.event_type.as_str())
            .cloned();
        let Some(bus) = bus else { return 0 };

        REENTRY.with(|r| {
            let mut r = r.borrow_mut();
            let (depth, in_flight, _) = &mut *r;
            *depth += 1;
            in_flight.push(ev.event_type.clone());
        });
        let delivered = bus.publish(ev);
        REENTRY.with(|r| {
            let mut r = r.borrow_mut();
            let (depth, in_flight, _) = &mut *r;
            *depth -= 1;
            in_flight.retain(|t| t != &ev.event_type);
        });
        // Java 循环体内被注释掉的调试行 (逐字保留):
        // prog.util.Logger.debug("UIStateBus", " -> Calling handler: " +
        // handler.getClass().getName());
        delivered
    }

    /// Publish an event to all subscribers (legacy).
    ///
    /// Java 两参重载 publish(eventType, data) → publish(eventType, null, data);
    /// Rust 无重载, 加 `_legacy` 后缀 (logger.rs `_default` 更名先例)。
    pub fn publish_legacy(&self, event_type: &str, data: Option<&str>) -> usize {
        self.publish(event_type, None, data)
    }

    /// Clear all subscribers (useful for cleanup/testing).
    ///
    /// 旧总线整体丢弃: 在途回调持快照不受影响 (对位 Java COW 迭代中 clear
    /// 不影响本轮), 之后所有发布送达 0。Java 全库无调用方 (LIFETIMES 审查 #10)。
    pub fn clear(&self) {
        self.map.write().expect(MAP_LOCK_MSG).clear();
    }

    /// 指定事件类型的存活订阅者数 (诊断/测试用; 无订阅者返回 0 — bus.rs 同款面)
    pub fn subscriber_count(&self, event_type: &str) -> usize {
        self.map
            .read()
            .expect(MAP_LOCK_MSG)
            .get(event_type)
            .map(|bus| bus.subscriber_count())
            .unwrap_or(0)
    }
}

// =====================================================================
// Tests — 公共项边界测试 (路由/RAII/异常隔离/快照/重入/跨线程)
// =====================================================================
#[cfg(test)]
mod tests;
