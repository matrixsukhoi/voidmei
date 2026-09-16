//! 事件桥 (D9 阶段③): EventBus (Rust 侧) → tauri emit (前端)。
//! 订阅闭包要求 Send (`kernel` bus 订阅签名), AppHandle 满足 — publish 发生在
//! 任意线程 (set_config 的主线程 / watcher 线程) 都能转发。
//!
//! E11 后本 crate 无模块级静态可变态: 原 About Modal 展示期静态
//! (ABOUT_MODAL_UNTIL) 已并入 FormRuntime 字段 (ipc.rs, 经 tauri State 分发)。

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Wry};
use kernel::base::bus::ui_state_bus::{UIStateBus, UiStateEvent};
use kernel::base::bus::Subscription;
use kernel::base::event::ui_state_events;
use kernel::base::logger;
use kernel::fm::FmChangedBus;

/// CONFIG_CHANGED → 前端 `config-changed` 事件 (data = 变更键, 如 "ui_layout.cfg")
/// 前端收到后重拉 get_layout_tree (reset/import 后的整树刷新对位 Java rebuild)。
pub fn bridge_config_changed(app: AppHandle<Wry>, bus: &UIStateBus) -> Subscription<UiStateEvent> {
    bus.subscribe(ui_state_events::CONFIG_CHANGED, move |ev: &UiStateEvent| {
        // 审查 W4: 静默吞 = 前端 cfg 树失刷新且无自愈路径 — 留告警面
        if let Err(e) = app.emit("config-changed", ev.data.clone()) {
            logger::warn("WebBridge", &format!("config-changed 事件发送失败: {e}"));
        }
    })
}

/// FM 状态载荷 (Java fmChangedHandler toast 面)
#[derive(Serialize, Clone)]
pub struct FmChangedPayload {
    pub name: Option<String>,
    pub status: String,
}

/// R7 真窗编辑会话事件桥 (阶段 C 后仅剩会话起止一键):
/// HUD_EDIT_SESSION → 前端 `hud-edit-session` emit + MainForm 隐退/恢复。
/// 原镜像推送三键 (doc/selection/error) 已随 web 编辑面板退役 — 编辑期
/// 前端零事件往来, 试驾场窗口群 (侧栏/悬浮条) 在渲染线程自建。
pub fn bridge_hud_edit(app: AppHandle<Wry>, bus: &UIStateBus) -> [Subscription<UiStateEvent>; 1] {
    // 键名 = voidmei edit_session::HUD_EDIT_SESSION (渲染线程发布; webui 不依赖
    // voidmei, 字面量对齐)
    [bus.subscribe(
        "HUD_EDIT_SESSION",
        move |ev: &UiStateEvent| {
            // 会话起止 → 机库隐退/恢复 (编辑期让屏; 事件泵与监听不受影响)
            let show = ev.data.as_deref() == Some("end");
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(main) = app2.get_webview_window(crate::MAIN_LABEL) {
                    let _ = if show { main.show() } else { main.hide() };
                }
            });
            if let Err(e) = app.emit("hud-edit-session", ev.data.clone()) {
                logger::warn("WebBridge", &format!("hud-edit-session 事件发送失败: {e}"));
            }
        },
    )]
}

/// FM_CHANGED → 前端 `fm-changed` (MISSING/CORRUPT toast, 对位 NotificationService)
pub fn bridge_fm_changed(
    app: AppHandle<Wry>,
    bus: &FmChangedBus,
) -> Subscription<kernel::fm::FMHandle> {
    bus.subscribe(move |h: &kernel::fm::FMHandle| {
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

/// 托盘"关于"载荷 (Java Application.showAbout 三段文案 + 版本号,
/// 组装层 main 循环 emit `about-requested` → 前端 Modal)
#[derive(Serialize, Clone)]
pub struct AboutPayload {
    pub version: String,
    /// [aboutcontent, aboutcontentsub1, aboutcontentsub2] (Lang 单一来源, Rust 侧就绪)
    pub contents: [String; 3],
}

/// 前端 About Modal 关闭回执 (dialogs.tsx afterClose → invoke): 清展示期标记
/// (FormRuntime 字段, 经 tauri State 同源读写), 恢复 InGame 收窗。薄命令直改,
/// 不经主线程 dispatcher (commands_comparison 先例); 展示期语义见 ipc.rs
/// FormRuntime::set_about_modal_open (B1 审查背景注)
#[tauri::command]
pub fn about_modal_closed(state: tauri::State<'_, crate::ipc::AboutModalShared>) {
    *state.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// config_manager 弹窗载荷 (Java ConfigManager, 经 ConfigDialog sink
/// 转发 → 前端 `config-dialog` 事件 → Modal.error / Modal.info)
#[derive(Serialize, Clone)]
pub struct ConfigDialogPayload {
    /// "parse-error" (ERROR_MESSAGE) | "merge-report" (INFORMATION_MESSAGE)
    pub kind: &'static str,
    pub title: String,
    pub message: String,
}
