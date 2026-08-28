//! 事件桥 (D9 阶段③): EventBus (Rust 侧) → tauri emit (前端)。
//! 订阅闭包要求 Send (`vm-core/src/bus.rs:45`), AppHandle 满足 — publish 发生在
//! 任意线程 (set_config 的主线程 / watcher 线程) 都能转发。

use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Wry};
use vm_core::bus::{EventBus, Subscription};
use vm_core::configuration_service::UiStateEvent;
use vm_core::event::ui_state_events;
use vm_core::fm::FmChangedBus;
use vm_core::logger;

/// CONFIG_CHANGED → 前端 `config-changed` 事件 (data = 变更键, 如 "ui_layout.cfg")
/// 前端收到后重拉 get_layout_tree (reset/import 后的整树刷新对位 Java rebuild)。
pub fn bridge_config_changed(app: AppHandle<Wry>, bus: &EventBus<UiStateEvent>) -> Subscription<UiStateEvent> {
    bus.subscribe(move |ev: &UiStateEvent| {
        if ev.event_type == ui_state_events::CONFIG_CHANGED {
            // 审查 W4: 静默吞 = 前端 cfg 树失刷新且无自愈路径 — 留告警面
            if let Err(e) = app.emit("config-changed", ev.data.clone()) {
                logger::warn("WebBridge", &format!("config-changed 事件发送失败: {e}"));
            }
        }
    })
}

/// FM 状态载荷 (Java fmChangedHandler toast 面)
#[derive(Serialize, Clone)]
pub struct FmChangedPayload {
    pub name: Option<String>,
    pub status: String,
}

/// FM_CHANGED → 前端 `fm-changed` (MISSING/CORRUPT toast, 对位 NotificationService)
pub fn bridge_fm_changed(app: AppHandle<Wry>, bus: &FmChangedBus) -> Subscription<vm_core::fm::FMHandle> {
    bus.subscribe(move |h: &vm_core::fm::FMHandle| {
        let payload = FmChangedPayload {
            name: h.name.clone(),
            status: format!("{:?}", h.status),
        };
        // 审查 W4 对齐: emit 失败留告警 (MISSING/CORRUPT toast 丢失可追溯)
        if let Err(e) = app.emit("fm-changed", payload) {
            logger::warn("WebBridge", &format!("fm-changed 事件发送失败: {e}"));
        }
    })
}

/// 托盘"关于"载荷 (Application.java:236-245 三段 showAbout 文案 + 版本号,
/// 组装层 main 循环 emit `about-requested` → 前端 Modal)
#[derive(Serialize, Clone)]
pub struct AboutPayload {
    pub version: String,
    /// [aboutcontent, aboutcontentsub1, aboutcontentsub2] (Lang 单一来源, Rust 侧就绪)
    pub contents: [String; 3],
}

/// About Modal 展示期标记 (审查 B1): 主循环 emit `about-requested` 时开启一个
/// 60s 阅读窗口 (Java 三段通知展示时长 8/16/24s 的宽裕上界 — 防遗忘态永久豁免
/// InGame 收窗), 前端 Modal 关闭回调经 [`about_modal_closed`] 提前清零; 主循环
/// 的 InGame 收窗分支读取本标记 — Java 通知弹窗独立于 MainForm 可见性, 游戏中
/// 托盘"关于"恒可读, 不随 mStart 收窗闪没。
static ABOUT_MODAL_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

/// 阅读窗口上界 (Java showAbout 通知最长 24s, 放宽到 60s)
const ABOUT_READ_WINDOW: Duration = Duration::from_secs(60);

/// 标记/清除 About Modal 展示期 (true = 开 60s 窗口; false = 立即清除)
pub fn set_about_modal_open(open: bool) {
    let mut until = ABOUT_MODAL_UNTIL.lock().unwrap_or_else(|e| e.into_inner());
    *until = if open { Some(Instant::now() + ABOUT_READ_WINDOW) } else { None };
}

/// About Modal 是否处于展示期 (60s 上界内)
pub fn about_modal_open() -> bool {
    ABOUT_MODAL_UNTIL
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some_and(|deadline| Instant::now() < deadline)
}

/// 前端 About Modal 关闭回执 (dialogs.tsx afterClose → invoke): 清展示期标记,
/// 恢复 InGame 收窗。薄命令直改静态, 不经主线程 dispatcher (commands_windows 先例)
#[tauri::command]
pub fn about_modal_closed() {
    set_about_modal_open(false);
}

/// config_manager 弹窗载荷 (ConfigManager.java:425-477, 经 ConfigDialog sink
/// 转发 → 前端 `config-dialog` 事件 → Modal.error / Modal.info)
#[derive(Serialize, Clone)]
pub struct ConfigDialogPayload {
    /// "parse-error" (ERROR_MESSAGE) | "merge-report" (INFORMATION_MESSAGE)
    pub kind: &'static str,
    pub title: String,
    pub message: String,
}
