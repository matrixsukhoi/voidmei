//! 事件桥 (D9 阶段③): EventBus (Rust 侧) → tauri emit (前端)。
//! 订阅闭包要求 Send (`vm-core/src/bus.rs:45`), AppHandle 满足 — publish 发生在
//! 任意线程 (set_config 的主线程 / watcher 线程) 都能转发。

use serde::Serialize;
use tauri::{AppHandle, Emitter, Wry};
use vm_core::bus::{EventBus, Subscription};
use vm_core::configuration_service::UiStateEvent;
use vm_core::event::ui_state_events;
use vm_core::fm::FmChangedBus;

/// CONFIG_CHANGED → 前端 `config-changed` 事件 (data = 变更键, 如 "ui_layout.cfg")
/// 前端收到后重拉 get_layout_tree (reset/import 后的整树刷新对位 Java rebuild)。
pub fn bridge_config_changed(app: AppHandle<Wry>, bus: &EventBus<UiStateEvent>) -> Subscription<UiStateEvent> {
    bus.subscribe(move |ev: &UiStateEvent| {
        if ev.event_type == ui_state_events::CONFIG_CHANGED {
            let _ = app.emit("config-changed", ev.data.clone());
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
        let _ = app.emit("fm-changed", payload);
    })
}
