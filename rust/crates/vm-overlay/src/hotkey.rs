//! 对应 Java: `src/prog/hotkey/HotkeyManager.java`
//! Unified global hotkey manager.
//! Centralizes all hotkey listeners to avoid duplicate registrations.
//!
//! 平台层复刻说明 (PORTING.md §3 库映射表: jnativehook → 自实现 WH_KEYBOARD_LL):
//! - Java 侧 `GlobalScreen.registerNativeHook()` 的 Windows 原生实现 (libuiohook)
//!   = 在**独立钩子线程**上 `SetWindowsHookExW(WH_KEYBOARD_LL)` + 该线程内部
//!   `GetMessageW` 消息循环; 键事件在钩子回调里做 VK→VC 转换后同步 dispatch 给
//!   Java 监听器 (即 `nativeKeyPressed` 跑在 jnativehook 线程上, VoidMei 的
//!   FM_OVERLAY_TOGGLE 因此发布在非 EDT 线程 —— LIFETIMES §2.2)。
//!   本文件逐层对应: `init()` 起独立钩子线程装钩子跑消息泵; 回调内 vk_to_vc
//!   转换 + NumLock 过滤 + 绑定表查找, 之后经 `HotkeyEventSink` (**Send 边界**)
//!   送出; Drop / shutdown 投 WM_QUIT 退泵 → 钩子线程卸钩退出。
//! - **键码语义 = jnativehook VC 码 (set-1 扫描码域)**: VoidMei 的 displayFmKey
//!   配置值 (默认 VC_P=25)、HotkeyRowRenderer 捕获值、HotkeyManager 绑定键
//!   全部是该域, 与 Windows VK 域不同 (VK_A=0x41 vs VC_A=0x1E)。
//!   `VK_TO_VC` 表 + 扩展键 OR 逻辑从随包分发的
//!   `dep/JNativeHook-2.2.2.x86_64.dll` **二进制逐字节提取** (表 @ 文件偏移
//!   0xe4b0, 每项交错 [u16 VC, u16 VK] 取首列; 扩展键开关取自 0x1800072b0
//!   处 keycode_to_scancode 的跳转表), 并以 jar 内 NativeKeyEvent.class 的
//!   常量 (javap -constants) 逐一核对 —— 两源一致才算数。
//! - PORT: Java `Application.silenceNativeHookLogger()` (关 jnativehook 的 JUL
//!   日志) 在 Rust 无对应物, 不移植。
//! - PORT(§2.9): Java `getInstance()` 懒加载单例 (跨 Controller 重建存活) 解散,
//!   实例由 App 层拥有并注入; "绑定表跨重建存活"由所有者持同一实例保证。
//!   事件出口 `UIStateBus.publish(eventType, HotkeyManager.this, Integer code)`
//!   → sink trait (source 字段不随行, 由接线层补; payload=VC 键码保留)。
//! - PORT(线程模型): Java shutdown() 只摘监听器不清全局钩子 (jnativehook 进程级
//!   常驻); Rust 实例自持钩子, 摘监听 ≡ 停泵卸钩, 事件不再产生的语义一致;
//!   Drop 按任务契约卸钩收线程。
//! - PORT(D8 拓扑偏差, 批十四组装时处理): 本实现固化自管独立 "HotkeyHook"
//!   线程 + 私有消息泵 (对齐 jnativehook 独立钩子线程语义); D8 规定热键
//!   WH_KEYBOARD_LL 最终并入与 overlay/托盘共享的 win32 单泵线程。组装时
//!   需提供在外部线程装钩的入口或豁免并记录。

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use vm_core::logger;

#[cfg(target_os = "windows")]
use std::cell::RefCell;
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::GetCurrentThreadId;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    KBDLLHOOKSTRUCT, LLKHF_EXTENDED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
};

// ---------------------------------------------------------------------------
// jnativehook VC 键码常量
// 对应 Java `NativeKeyEvent.VC_*` (值经 javap -constants 从
// dep/jnativehook-2.2.2.jar 的 NativeKeyEvent.class 提取核对)。
// 仅列出 VoidMei Java 源实际引用的键 + 域内哨兵; 完整域由 VK_TO_VC 承载。
// ---------------------------------------------------------------------------

/// The native key code for the undefined key. (NativeKeyEvent.VC_UNDEFINED = 0x0000)
pub const VC_UNDEFINED: i32 = 0x0000;

/// FM overlay 热键默认键 'P' (Application.java:94 `displayFmKey = VC_P`)
pub const VC_P: i32 = 0x0019;

/// HotkeyManager.java:64 / HotkeyRowRenderer.java:59 过滤用的三个 Lock 键
pub const VC_CAPS_LOCK: i32 = 0x003A;
/// NumLock 伪事件过滤 (HotkeyManager.java:64)
pub const VC_NUM_LOCK: i32 = 0x0045;
/// HotkeyRowRenderer.java:60 采键时忽略
pub const VC_SCROLL_LOCK: i32 = 0x0046;

/// VK → VC 查找表: VK 索引 → jnativehook VC 码 (set-1 扫描码域)。
/// 从 dep/JNativeHook-2.2.2.x86_64.dll 的 `keycode_scancode_table[i][0]` 列
/// 逐字节提取 (libuiohook src/windows/input_helper.c 的编译产物), 与 jar 内
/// NativeKeyEvent 常量核对一致。0x0000 = 无映射。
/// 行号 = VK 高 4 位 (每行 16 项)。
#[rustfmt::skip]
const VK_TO_VC: [u16; 256] = [
    /* 0x00 */ 0x0000, 0x0001, 0x0002, 0x0000, 0x0003, 0x0004, 0x0005, 0x0000, 0x000E, 0x000F, 0x0000, 0x0000, 0xE04C, 0x001C, 0x0000, 0x0000,
    /* 0x10 */ 0x002A, 0x001D, 0x0038, 0x0E45, 0x003A, 0x0070, 0x0000, 0x0000, 0x0000, 0x0079, 0x0000, 0x0001, 0x0000, 0x0000, 0x0000, 0x0000,
    /* 0x20 */ 0x0039, 0x0E49, 0x0E51, 0x0E4F, 0x0E47, 0xE04B, 0xE048, 0xE04D, 0xE050, 0x0000, 0x0000, 0x0000, 0x0E37, 0x0E52, 0x0E53, 0x0000,
    /* 0x30 */ 0x000B, 0x0002, 0x0003, 0x0004, 0x0005, 0x0006, 0x0007, 0x0008, 0x0009, 0x000A, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    /* 0x40 */ 0x0000, 0x001E, 0x0030, 0x002E, 0x0020, 0x0012, 0x0021, 0x0022, 0x0023, 0x0017, 0x0024, 0x0025, 0x0026, 0x0032, 0x0031, 0x0018,
    /* 0x50 */ 0x0019, 0x0010, 0x0013, 0x001F, 0x0014, 0x0016, 0x002F, 0x0011, 0x002D, 0x0015, 0x002C, 0x0E5B, 0x0E5C, 0x0E5D, 0x0000, 0xE05F,
    /* 0x60 */ 0x0052, 0x004F, 0x0050, 0x0051, 0x004B, 0x004C, 0x004D, 0x0047, 0x0048, 0x0049, 0x0037, 0x004E, 0x0000, 0x004A, 0x0053, 0x0E35,
    /* 0x70 */ 0x003B, 0x003C, 0x003D, 0x003E, 0x003F, 0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0057, 0x0058, 0x005B, 0x005C, 0x005D, 0x0063,
    /* 0x80 */ 0x0064, 0x0065, 0x0066, 0x0067, 0x0068, 0x0069, 0x006A, 0x006B, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    /* 0x90 */ 0x0045, 0x0046, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    /* 0xA0 */ 0x002A, 0x0036, 0x001D, 0x0E1D, 0x0038, 0x0E38, 0xE06A, 0xE069, 0xE067, 0xE068, 0xE065, 0xE066, 0xE032, 0xE020, 0xE02E, 0xE030,
    /* 0xB0 */ 0xE019, 0xE010, 0xE024, 0xE022, 0x0000, 0xE06D, 0xE06C, 0xE021, 0x0000, 0x0000, 0x0027, 0x000D, 0x0033, 0x000C, 0x0034, 0x0035,
    /* 0xC0 */ 0x0029, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    /* 0xD0 */ 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x001A, 0x002B, 0x001B, 0x0028, 0x007D,
    /* 0xE0 */ 0x0000, 0x0000, 0x0E46, 0x0000, 0x0000, 0xE064, 0xE03C, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    /* 0xF0 */ 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0xE04C, 0x0000,
];

/// Windows VK 码 → jnativehook VC 码 (Java `NativeKeyEvent.getKeyCode()` 的值域)。
///
/// 对应 libuiohook `keycode_to_scancode(vk_code, flags)` (jnativehook Windows
/// 原生层, 钩子回调内调用): 表查找 + LLKHF_EXTENDED 时对导航/编辑键组 OR
/// 前缀。逻辑从 dep/JNativeHook-2.2.2.x86_64.dll 反汇编核对 (跳转表见
/// 0x18000738c), **包含其历史怪癖** (表值已带 0xE0 前缀的导航键再 OR 0xEE00
/// 会得 0xFE48 之类 —— Java 实机行为如此, 用户配置里存的就是这些值, 保真
/// 不修正)。越界 VK (>0xFF) 返回 VC_UNDEFINED。
pub fn vk_to_vc(vk_code: u32, extended: bool) -> u16 {
    // Check the vk_code is in range.
    // NOTE vk_code >= 0 is assumed because DWORD is unsigned.
    let mut scancode = VC_UNDEFINED as u16;

    if (vk_code as usize) < VK_TO_VC.len() {
        scancode = VK_TO_VC[vk_code as usize];

        if extended {
            // DLL 跳转表: 仅下列 VK 参与 OR (i = vk - 0x0D 的字节索引表逐项核对)
            match vk_code {
                // VK_PRIOR(0x21) VK_NEXT(0x22) VK_END(0x23) VK_HOME(0x24)
                // VK_LEFT(0x25) VK_UP(0x26) VK_RIGHT(0x27) VK_DOWN(0x28)
                // VK_INSERT(0x2D) VK_DELETE(0x2E)
                0x21..=0x28 | 0x2D | 0x2E => scancode |= 0xEE00,
                // VK_RETURN (0x0D): 小键盘回车 (E0 1C) → 0x1C | 0x0E00 = 0xE1C
                0x0D => scancode |= 0x0E00,
                _ => {}
            }
        }
    }

    scancode
}

// ---------------------------------------------------------------------------
// 事件出口 (Send 边界)
// ---------------------------------------------------------------------------

/// 热键事件: Java `UIStateBus.publish(eventType, HotkeyManager.this, code)`
/// 三元组的结构化替身。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyEvent {
    /// 事件类型 (Java eventType, 如 `UIStateEvents.FM_OVERLAY_TOGGLE`)
    pub event_type: String,
    /// 触发键的 VC 码 (Java payload: `Integer` 键码)
    pub key_code: i32,
}

/// 热键事件出口 trait: 对应 Java 监听器内 `UIStateBus.publish(...)`。
/// 回调只在钩子线程上执行 (对齐 jnativehook 派发线程), 实现方自行负责
/// 跨线程投递 (Java 订阅方同样"自行 invokeLater")。
/// 回调 panic 不得依赖传播中断派发: 调用方 catch_unwind 捕获记日志后继续
/// (对齐 UIStateBus 对订阅方异常逐个 catch 不中断, LIFETIMES §2.2)。
/// Sync 约束是 Rust 侧要求: sink 以 Arc 克隆进钩子线程 (Arc<T>: Send 需
/// T: Send + Sync), 非行为语义 — Java 的 bus 单例天然"跨线程共享"。
pub trait HotkeyEventSink: Send + Sync {
    fn on_hotkey(&self, event: &HotkeyEvent);
}

/// mpsc 出口形态: 接线方拿 Receiver 在自己的线程消费。
/// Mutex 包 `mpsc::Sender` (Sender 是 Send 但 !Sync, Arc 化需要 Sync;
/// 钩子线程单线程使用, Mutex 是零竞争的合法 Sync 化)。
pub struct ChannelHotkeySink(Mutex<Sender<HotkeyEvent>>);

impl ChannelHotkeySink {
    /// `(共享 sink, 接收端)` 对
    pub fn new() -> (Arc<Self>, Receiver<HotkeyEvent>) {
        let (tx, rx) = mpsc::channel();
        (Arc::new(ChannelHotkeySink(Mutex::new(tx))), rx)
    }
}

impl HotkeyEventSink for ChannelHotkeySink {
    fn on_hotkey(&self, event: &HotkeyEvent) {
        // PORT: Java publish 无失败路径; send 仅在接收端已 drop 时失败,
        // 对应 Java "订阅方全撤后 publish 静默" — 忽略错误保持静默
        if let Ok(tx) = self.0.lock() {
            let _ = tx.send(event.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// HotkeyManager (HotkeyManager.java)
// ---------------------------------------------------------------------------

/// 钩子线程句柄; `Some` = Java `initialized == true`。
struct HookHandle {
    /// 钩子线程 id (PostThreadMessageW 目标; 仅 Windows 有意义)
    #[cfg(target_os = "windows")]
    thread_id: u32,
    /// 钩子线程存活标志 (防线程已死时向被复用的线程 id 误投 WM_QUIT)
    #[cfg(target_os = "windows")]
    alive: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl HookHandle {
    /// 投 WM_QUIT 退消息泵 → join 收线程 (卸钩在钩子线程退出路径完成,
    /// `UnhookWindowsHookEx` 必须由装钩线程调用)。
    fn stop(self) {
        #[cfg(target_os = "windows")]
        {
            if self.alive.load(Ordering::Acquire) {
                // jnativehook 停泵方式: 向钩子线程投 WM_QUIT 唤醒阻塞中的 GetMessageW
                let _ = unsafe {
                    PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0))
                };
            }
        }
        if let Some(j) = self.join {
            // PORT: sink 回调内触发 drop/stop 时不能 join 自身 — WM_QUIT 已投,
            // 回调返回后线程自然退出, 此处放弃等待 (Java 无此形态, Rust 所有权
            // 防御; 正常路径 (任意其他线程 drop) 均走 join)
            if j.thread().id() != std::thread::current().id() {
                let _ = j.join();
            }
        }
    }
}

pub struct HotkeyManager {
    /// Java: `private final Map<Integer, String> keyBindings = new ConcurrentHashMap<>();`
    /// (钩子线程读 / 调用方写, 读多写少; CHM → Mutex<HashMap>, LIFETIMES §3.2)
    key_bindings: Arc<Mutex<HashMap<i32, String>>>,
    /// Java: `private boolean initialized = false;`
    hook: Option<HookHandle>,
    /// init() 时随钩子线程注入的事件出口
    /// (Java: 监听器经 GlobalScreen 注册, 命中后 publish 到 UIStateBus 单例)
    sink: Arc<dyn HotkeyEventSink>,
}

impl HotkeyManager {
    /// 对应 Java `private HotkeyManager()` + `getInstance()`。
    /// PORT: 单例解散 (§2.9) — 由 App 层持有实例; 事件出口构造时注入。
    pub fn new(sink: Arc<dyn HotkeyEventSink>) -> Self {
        HotkeyManager {
            key_bindings: Arc::new(Mutex::new(HashMap::new())),
            hook: None,
            sink,
        }
    }

    /// channel 出口便捷构造: `(manager, receiver)`。
    /// PORT: Java 无此形态 (走 UIStateBus 单例); Rust 用 mpsc 承担 Send 边界,
    /// 接线方 (vm-app 主循环) 持 Receiver 消费, 对位 "订阅方自行跨线程"。
    pub fn with_channel() -> (Self, Receiver<HotkeyEvent>) {
        let (sink, rx) = ChannelHotkeySink::new();
        (Self::new(sink), rx)
    }

    /// Initialize the native hook and register the key listener.
    /// Safe to call multiple times - only initializes once.
    /// (HotkeyManager.java:41-79)
    ///
    /// PORT: Java void + 失败仅记日志 (initialized 保持 false 可重入);
    /// Rust 显式 Result, 语义同形 (重复 init → Ok 早退)。
    pub fn init(&mut self) -> Result<(), String> {
        if self.hook.is_some() {
            logger::debug("HotkeyManager", "Already initialized, skipping");
            return Ok(());
        }

        match spawn_hook_thread(self.key_bindings.clone(), self.sink.clone()) {
            Ok(h) => {
                // Java: Logger.info("Native hook registered") — register 成功即记
                logger::info("HotkeyManager", "Native hook registered");
                self.hook = Some(h);
                logger::info("HotkeyManager", "Initialized with native key listener");
                Ok(())
            }
            Err(ex) => {
                logger::error("HotkeyManager", &format!("Failed to register native hook: {}", ex));
                Err(ex)
            }
        }
    }

    /// Bind a key code to an event type.
    /// When the key is pressed, the event will be published to UIStateBus.
    /// (HotkeyManager.java:88-95)
    pub fn bind(&self, key_code: i32, event_type: &str) {
        if key_code == 0 {
            logger::debug("HotkeyManager", "Ignoring bind for keyCode 0");
            return;
        }
        // keyBindings.put(keyCode, eventType)
        if let Ok(mut m) = self.key_bindings.lock() {
            m.insert(key_code, event_type.to_string());
        }
        logger::info("HotkeyManager", &format!("Bound key {} -> {}", key_code, event_type));
    }

    /// Unbind a key code. (HotkeyManager.java:102-107)
    pub fn unbind(&self, key_code: i32) {
        let removed = self
            .key_bindings
            .lock()
            .ok()
            .and_then(|mut m| m.remove(&key_code));
        if let Some(was) = removed {
            logger::info("HotkeyManager", &format!("Unbound key {} (was {})", key_code, was));
        }
    }

    /// Rebind a key from old code to new code. (HotkeyManager.java:116-120)
    pub fn rebind(&self, old_key_code: i32, new_key_code: i32, event_type: &str) {
        self.unbind(old_key_code);
        self.bind(new_key_code, event_type);
        // PORT: Java 此行无条件输出 (即便 newKeyCode==0 被 bind 跳过也打日志) — 保真
        logger::info(
            "HotkeyManager",
            &format!("Rebound {} -> {} for {}", old_key_code, new_key_code, event_type),
        );
    }

    /// Check if a key code is currently bound. (HotkeyManager.java:125-127)
    pub fn is_bound(&self, key_code: i32) -> bool {
        self.key_bindings.lock().map(|m| m.contains_key(&key_code)).unwrap_or(false)
    }

    /// Get the event type bound to a key code. (HotkeyManager.java:132-134)
    pub fn get_binding(&self, key_code: i32) -> Option<String> {
        self.key_bindings.lock().ok().and_then(|m| m.get(&key_code).cloned())
    }

    /// Shutdown the hotkey manager. (HotkeyManager.java:139-152)
    ///
    /// PORT: Java 只摘监听器 (GlobalScreen 全局钩子进程级常驻); Rust 实例自持
    /// 钩子, 等价动作为停泵卸钩 (见模块头注释)。
    pub fn shutdown(&mut self) {
        let Some(h) = self.hook.take() else {
            return; // Java: if (!initialized) return;
        };

        // Java: GlobalScreen.removeNativeKeyListener(keyListener)
        h.stop();

        // keyBindings.clear()
        if let Ok(mut m) = self.key_bindings.lock() {
            m.clear();
        }
        logger::info("HotkeyManager", "Shutdown complete");
    }
}

impl Drop for HotkeyManager {
    /// 任务契约: Drop 卸钩子 (Java 主流程从不 shutdown, 靠 JVM 退出; Rust
    /// 由所有权在实例消亡时收尾 — 对齐 LIFETIMES "Drop = 销毁链")。
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ---------------------------------------------------------------------------
// 钩子线程 (Windows)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
struct HookCtx {
    bindings: Arc<Mutex<HashMap<i32, String>>>,
    sink: Arc<dyn HotkeyEventSink>,
}

// 钩子回调上下文: WH_KEYBOARD_LL 回调只会在安装它的线程上执行 (即本管理器
// 起的钩子线程), thread_local 即按实例隔离, 无裸全局 (§2.9)。
// 初始化已是 const block (该 lint 建议的最终形态), clippy 1.97.0 的
// missing_const_for_thread_local 不跳过 const block 形态仍触发 (最简
// `const { RefCell::new(0) }` 实验同报) — 确系误报, allow 收口 (审查 A-B2;
// 属性须落在宏内 static 上, 落在宏调用上会被编译器忽略)
#[cfg(target_os = "windows")]
thread_local! {
    #[allow(clippy::missing_const_for_thread_local)]
    static HOOK_CTX: RefCell<Option<HookCtx>> = const { RefCell::new(None) };
}

/// WH_KEYBOARD_LL 回调 = jnativehook 原生键盘钩子 → `nativeKeyPressed`
/// (HotkeyManager.java:60-73) 的合体。
#[cfg(target_os = "windows")]
unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // 低级钩子契约: code < 0 (非 HC_ACTION) 必须直通系统。
    // PORT: DLL 的派发点位于 nCode<0 检查之前 (nCode<0 时仍会派发键事件);
    // 但 WH_KEYBOARD_LL 的 nCode 恒为 HC_ACTION(0), 该分支运行时不可达,
    // Rust 收进门内更符合 Win32 文档要求, 零行为差异。
    if code >= 0 {
        let msg = wparam.0 as u32;
        // WM_SYSKEYDOWN: Alt 组合键走此消息 (jnativehook 同样两者都派发);
        // 按住不放的自动重复会反复进此分支 — Java nativeKeyPressed 同样逐次触发
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            // int code = e.getKeyCode() — VK → VC 转换 (jnativehook 原生层职责)
            let vc = vk_to_vc(kb.vkCode, kb.flags.0 & LLKHF_EXTENDED.0 != 0) as i32;

            // Filter out spurious NumLock events (HotkeyManager.java:63-66)
            // 注: 硬件 NumLock 带 E0 前缀, 但 VK_NUMLOCK(0x90) 不参与扩展 OR,
            // 转换后仍为 VC_NUM_LOCK(69) — 过滤依赖此不变量
            if vc != VC_NUM_LOCK {
                // String eventType = keyBindings.get(code)
                // (锁内只取拷贝, sink 在锁外调 — §2.8 锁内不执行回调)
                let hit = HOOK_CTX.with(|c| {
                    c.borrow().as_ref().and_then(|ctx| {
                        // PORT: Java CHM 无中毒概念; Mutex 中毒 (持锁 panic,
                        // 理论不可达) 按未绑定跳过
                        let et = ctx.bindings.lock().ok()?.get(&vc).cloned()?;
                        Some((ctx.sink.clone(), et))
                    })
                });
                if let Some((sink, event_type)) = hit {
                    logger::debug(
                        "HotkeyManager",
                        &format!("Hotkey pressed: {} -> {}", vc, event_type),
                    );
                    // UIStateBus.getInstance().publish(eventType, this, code)
                    // PORT: Java UIStateBus 对订阅方异常逐个 catch 不中断
                    // (LIFETIMES §2.2); 回调 panic 跨 extern "system" 边界
                    // unwind 会 abort 进程, 此处捕获记日志后继续走钩子链
                    let event = HotkeyEvent { event_type, key_code: vc };
                    if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        sink.on_hotkey(&event);
                    })) {
                        let msg = p
                            .downcast_ref::<&str>()
                            .map(|s| (*s).to_string())
                            .or_else(|| p.downcast_ref::<String>().cloned())
                            .unwrap_or_else(|| "unknown panic".to_string());
                        logger::error("HotkeyManager", &format!("Hotkey sink 回调 panic: {}", msg));
                    }
                }
            }
        }
    }
    // 永不吞键: jnativehook 键事件不 consume, 链下去让游戏/前台照常收键
    CallNextHookEx(None, code, wparam, lparam)
}

/// 起独立钩子线程: SetWindowsHookExW + GetMessageW 泵 (jnativehook
/// `GlobalScreen.registerNativeHook()` 的线程模型), 就绪/失败经 channel 回报
/// (对位 Java registerNativeHook 阻塞等待钩子线程启动, 失败抛
/// NativeHookException)。
#[cfg(target_os = "windows")]
fn spawn_hook_thread(
    bindings: Arc<Mutex<HashMap<i32, String>>>,
    sink: Arc<dyn HotkeyEventSink>,
) -> Result<HookHandle, String> {
    let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();
    let alive = Arc::new(AtomicBool::new(true));
    let alive_thr = alive.clone();

    let join = std::thread::Builder::new()
        .name("HotkeyHook".into())
        .spawn(move || {
            let thread_id = unsafe { GetCurrentThreadId() };
            // 回调上下文装填 (回调只会跑在本线程)
            HOOK_CTX.with(|c| {
                *c.borrow_mut() = Some(HookCtx { bindings, sink });
            });

            let hook = unsafe {
                match GetModuleHandleW(None) {
                    Ok(h) => SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), Some(h.into()), 0),
                    Err(e) => Err(e),
                }
            };
            let hook = match hook {
                Ok(h) => {
                    let _ = ready_tx.send(Ok(thread_id));
                    h
                }
                Err(e) => {
                    // 失败: 回报并清上下文退出 (initialized 保持 false 可重入)
                    let _ = ready_tx.send(Err(format!("{}", e)));
                    HOOK_CTX.with(|c| *c.borrow_mut() = None);
                    alive_thr.store(false, Ordering::Release);
                    return;
                }
            };

            // 消息泵: LL 钩子的回调由本线程 GetMessage 驱动。
            // PORT: libuiohook hook_run 语义 — >0 继续, 0 = WM_QUIT, -1 = 错误, 均退出
            let mut msg = MSG::default();
            loop {
                let r = unsafe { GetMessageW(&mut msg, None, 0, 0) }.0;
                if r <= 0 {
                    break;
                }
            }

            // 卸钩必须由装钩线程执行 (Windows 契约)
            unsafe { let _ = UnhookWindowsHookEx(hook); };
            HOOK_CTX.with(|c| *c.borrow_mut() = None);
            alive_thr.store(false, Ordering::Release);
        })
        .map_err(|e| format!("spawn hook thread: {}", e))?;

    match ready_rx.recv() {
        Ok(Ok(thread_id)) => Ok(HookHandle { thread_id, alive, join: Some(join) }),
        Ok(Err(e)) => {
            // 线程已自退 (钩子安装失败), join 只收尸
            let _ = join.join();
            Err(e)
        }
        Err(_) => {
            // 线程在回报前 panic — join 吞掉 panic, 上报为注册失败
            let panicked = join.join().err().map(|_| "hook 线程启动阶段 panic".to_string());
            Err(panicked.unwrap_or_else(|| "hook 线程异常退出".into()))
        }
    }
}

/// 非 Windows: X11 XGrabKey 路径未移植 (PORTING.md §3 只落地 Windows 实现,
/// 项目分发形态即 Windows EXE)。init 显式失败, 不装假钩子; 绑定面照常可用。
#[cfg(not(target_os = "windows"))]
fn spawn_hook_thread(
    _bindings: Arc<Mutex<HashMap<i32, String>>>,
    _sink: Arc<dyn HotkeyEventSink>,
) -> Result<HookHandle, String> {
    Err("全局热键: 非 Windows 平台 (jnativehook X11 XGrabKey 对应路径) 未移植".into())
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
