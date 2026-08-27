//! 对应 Java: `src/prog/event/UIStateBus.java`
//!
//! Event Bus for UI State synchronization.
//! Used for decoupled communication between UI panels (e.g., switch State sync).
//! Thread-safe using ConcurrentHashMap + CopyOnWriteArrayList.
//!
//! B 类适配总览 (主 agent 裁决, 底座 = crate::bus 的 EventBus/Subscription):
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
//!   PORT(§2.8 已知破损, 待主 agent 裁决, 本文件不越文件修): **同事件类型的
//!   嵌套同步 publish 会死锁** — bus.rs publish 持各监听器自己的 Mutex 执行
//!   回调 (bus.rs "阶段 2"), 内层同类型 publish 的升级快照必含正在执行的
//!   监听器 → 同线程对同一 std::sync::Mutex 二次 lock 永久阻塞 (Java 无
//!   monitor 天然可重入, Rust Mutex 不可重入)。Java 真实调用链即踩此形态:
//!   ButtonRowRenderer.java:55 publish(CONFIG_CHANGED, ACTION_RESET_REQUEST)
//!   → ConfigurationService.java:34 构造器订阅的 handler 同步调
//!   resetAllLayoutDefaults() → ConfigurationService.java:361 嵌套同步
//!   publish(CONFIG_CHANGED, ACTION_RESET_COMPLETED) — Java 递归会重入同一
//!   handler 但靠 payload 不同 (handler 只认 RESET_REQUEST) 终止; Rust 等价
//!   链路直接挂死发布线程。修复本体在 bus.rs (try_lock+延迟补投/递归深度
//!   跟踪/监听器串行化粒度) 或本层改分发形状, 均属架构级, 须主 agent 裁决;
//!   在此之前**禁止**把 configuration_service.rs 的桩总线切到本总线并接线
//!   CONFIG_CHANGED 订阅 (reset/导入链路即触发)。
//! - §2.9 全局态: Java `getInstance()` 静态单例解散, 实例由 App 层拥有并经
//!   构造器注入 (configuration_service.rs 的总线注入先例)。

use std::any::Any;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, RwLock};

use crate::bus::{EventBus, Subscription};
use crate::logger;

/// 路由表锁中毒消息 (Java 无锁; 对应持锁线程崩溃后的一致性未知面)
const MAP_LOCK_MSG: &str = "UIStateBus 路由表锁中毒";

/// UIStateBus 消息: Java `publish(eventType, source, data)` 弱类型三元组的
/// 强类型化 (消息 struct 化)。
///
/// PORT: 与 configuration_service.rs 的依赖桩 `UiStateEvent` 同名同字段语义,
/// 本处为 UIStateBus 波次落地后的 crate 统一定义点; 该桩切换属跨文件改动,
/// 本波次不越文件修复 (§6) 仅在此标注。
/// PORT(接线形状, 审查上报): 桩侧字段为裸 String (source/data 非 Option),
/// 注入点 `ConfigurationService::new(Option<Arc<EventBus<UiStateEvent>>>)`
/// (configuration_service.rs ServiceInner.ui_state_bus) 是**未路由的裸
/// EventBus**, 与本路由总线无类型兼容接线点; 且桩文档 "广播给全部订阅者,
/// 订阅方自行按 event_type 过滤" 的假设切换后即过时 (本总线按 event_type
/// 精确路由)。切换形状须主 agent 裁决: 构造参数改 `Arc<UIStateBus>` 字段
/// 包 Some(), 或本文件补 `bus_for(event_type)` 取出接口 —— 防类型分裂扩散。
/// 切换前提另见模块文档 PORT: 同类型嵌套 publish 死锁未解前不得接线
/// CONFIG_CHANGED 订阅。
/// PORT: 消息三字段均 String 域 (publish(event, sourceId, configKey) 裁决);
/// Java payload 的异构类型 (FM_CHANGED=FMHandle, FM_PRINT_SWITCH_CHANGED=
/// Boolean, FM_OVERLAY_TOGGLE=Integer) 暂以字符串/None 顶位, 对应发布方
/// (FMManager/HotkeyManager/DynamicDataPage) 未翻译, 落地时如需携带原生
/// 类型须扩展本消息 —— 已上报主 agent。
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
    /// (§2.5 无涉)。
    /// PORT: Java `private static final UIStateBus INSTANCE` 单例字段随
    /// getInstance() 一并解散 (§2.9) — 实例由 App 层拥有并注入, 不造全局。
    map: RwLock<HashMap<String, Arc<EventBus<UiStateEvent>>>>,
}

impl Default for UIStateBus {
    fn default() -> Self {
        Self::new()
    }
}

impl UIStateBus {
    /// 对应 Java `private UIStateBus()` (私有构造) + `getInstance()`。
    /// PORT: 单例读法 → 显式构造 (§2.9; 消费方持 `Arc<UIStateBus>` 注入)。
    pub fn new() -> Self {
        UIStateBus {
            map: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribe to an event type.
    ///
    /// @param eventType The event type identifier (e.g., "fmPrintSwitchChanged")
    /// @param handler   The handler to invoke when the event is published
    ///
    /// PORT: Java `Consumer<Object>` 只收 data → Rust 回调收整个
    /// `&UiStateEvent` (消息 struct 化), 订阅方按需取 `msg.data`;
    /// 返回 RAII 订阅句柄, Drop 即注销 (对位 Java 靠 unsubscribe 防泄漏)。
    pub fn subscribe<F>(&self, event_type: &str, handler: F) -> Subscription<UiStateEvent>
    where
        F: FnMut(&UiStateEvent) + Send + 'static,
    {
        // Java: subscribers.computeIfAbsent(eventType, k -> new CopyOnWriteArrayList<>()).add(handler);
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
                // Java: prog.util.Logger.error("UIStateBus", "Error in handler for "
                // + eventType + ": " + e.getMessage());
                logger::error(
                    "UIStateBus",
                    &format!(
                        "Error in handler for {}: {}",
                        et,
                        panic_message(payload)
                    ),
                );
            }
        });

        // Java: prog.util.Logger.event("SUBSCRIBE", eventType, handler, null);
        // PORT: source 位 Java 反射取 handler 类简单名 (匿名类如 Controller$$Lambda),
        // Rust 闭包无名且禁引反射 (§1) → None (日志渲染 "Unknown")
        logger::event("SUBSCRIBE", event_type, None, None);
        sub
    }

    /// Unsubscribe a handler from an event type.
    ///
    /// PORT: Java 以 Consumer 引用等值 remove; Rust 闭包无身份 → 以 RAII 句柄
    /// Drop 注销 (bus.rs 机制)。event_type 仅复刻 Java 的 "键存在才记日志"
    /// 分支 (handlers != null); 若调用方传了与订阅不符的类型, Java 只是无操作,
    /// Rust 侧句柄所有权仍会被消费 (等价于退订成功) — 全库无此错用形态。
    /// 翻译 Controller.stop() 五步退订 (Controller.java:783-793) 时注意:
    /// 其依赖的 '键不存在则跳过日志' 分支在此仍成立, 但 '错配键 = 无操作'
    /// 不成立 (退订总会发生)。
    pub fn unsubscribe(&self, event_type: &str, handler: Subscription<UiStateEvent>) {
        // Java: List<Consumer<Object>> handlers = subscribers.get(eventType);
        //       if (handlers != null) { ... }
        let known = self
            .map
            .read()
            .expect(MAP_LOCK_MSG)
            .contains_key(event_type);
        // Java: handlers.remove(handler); — Drop 即注销, 键保留 (Java 清空后的
        // 空 List 也留在 map 里, 永不移除键)
        drop(handler);
        if known {
            // Java: prog.util.Logger.event("UNSUBSCRIBE", eventType, handler, null);
            logger::event("UNSUBSCRIBE", event_type, None, None);
        }
    }

    /// Publish an event to all subscribers with explicit source.
    ///
    /// @param eventType The event type identifier
    /// @param source    The object initiating the event (for logging)
    /// @param data      The event payload
    ///
    /// 返回送达数 (bus.rs 诊断扩展; Java void)。同步派发: 调用线程 =
    /// 发布线程逐个执行订阅方代码 (对齐 Java)。
    pub fn publish(&self, event_type: &str, source: Option<&str>, data: Option<&str>) -> usize {
        // Java: prog.util.Logger.event("PUBLISH", eventType, source, data);
        // PORT(日志渲染分歧备案): Java Logger.event 用 source.getClass()
        // .getSimpleName(), 调用方传 String source 的点 (ConfigurationService
        // .java:65/82/295/322/361, DynamicDataPage.java:149/252,
        // ButtonRowRenderer.java:55) 实际打出 "String" 类简单名而非字符串
        // 内容; Rust 调用方传 Some("ConfigurationService") 打出内容。本转发
        // 行为与 Java 一致 (分歧源于 logger.rs 已接受的字符串 source 设计),
        // 翻译上述调用方时须知此偏差。
        logger::event("PUBLISH", event_type, source, data);

        // Java: List<Consumer<Object>> handlers = subscribers.get(eventType);
        // 持读锁只做查表克隆随即放锁 — 回调在锁外执行 (§2.8: handler 重入
        // subscribe/clear/跨类型 publish 不死锁; 同类型嵌套 publish 死锁,
        // 见模块文档 PORT 标注); 迭代快照语义由 bus.rs publish 的升级快照
        // 承担 (对位 Java COW for 循环起始取快照)。
        let bus = self
            .map
            .read()
            .expect(MAP_LOCK_MSG)
            .get(event_type)
            .cloned();
        // Java: if (handlers != null) { for (Consumer<Object> handler : handlers) {...} }
        match bus {
            Some(bus) => {
                // Java 循环体内被注释掉的调试行 (逐字保留):
                // prog.util.Logger.debug("UIStateBus", " -> Calling handler: " +
                // handler.getClass().getName());
                bus.publish(&UiStateEvent {
                    event_type: event_type.to_string(),
                    source: source.map(|s| s.to_string()),
                    data: data.map(|d| d.to_string()),
                })
            }
            // Java: handlers == null → 无订阅者, 静默无调用
            None => 0,
        }
    }

    /// Publish an event to all subscribers (legacy).
    ///
    /// PORT: Java 两参重载 publish(eventType, data) → publish(eventType, null, data);
    /// Rust 无重载, 加 `_legacy` 后缀 (logger.rs `_default` 更名先例)。
    pub fn publish_legacy(&self, event_type: &str, data: Option<&str>) -> usize {
        self.publish(event_type, None, data)
    }

    /// Clear all subscribers (useful for cleanup/testing).
    ///
    /// 旧总线整体丢弃: 在途回调持快照不受影响 (对位 Java COW 迭代中 clear
    /// 不影响本轮), 之后所有发布送达 0。Java 全库无调用方 (LIFETIMES 审查 #10)。
    pub fn clear(&self) {
        // Java: subscribers.clear();
        self.map.write().expect(MAP_LOCK_MSG).clear();
    }
}

/// Java `e.getMessage()` 的对位物: catch_unwind 载荷 → 字符串。
/// 无字符串载荷 (如算术溢出类 panic) → 字面 "null", 对齐 Java
/// getMessage()==null 时 `"..." + null` 的拼接结果。
fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else {
        "null".to_string()
    }
}

// =====================================================================
// Tests — 公共项边界测试 (路由/RAII/异常隔离/快照/重入/跨线程)
// =====================================================================
#[cfg(test)]
mod tests;
