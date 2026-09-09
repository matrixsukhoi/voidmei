//! R7 真窗编辑会话命令 (进/出/编辑载荷) — 经主线程 dispatcher 转 UiCommand
//! 送渲染线程 (编辑命令须到 Rc 渲染资源侧; 业务在 vm-app edit_session.rs)。

use serde_json::Value;

use crate::commands::{roundtrip, IpcState};
use crate::ipc::RequestKind;

/// 进入编辑会话 (真窗即画布: 桌面 overlay 直接可编辑)
#[tauri::command]
pub async fn begin_edit_session(state: tauri::State<'_, IpcState>) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::BeginEditSession).await
}

/// 退出编辑会话 (commit=true 提交落盘 / false 丢弃)
#[tauri::command]
pub async fn end_edit_session(
    state: tauri::State<'_, IpcState>,
    commit: bool,
) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::EndEditSession { commit }).await
}

/// 编辑命令载荷 (EditCommand 的 serde Value: 选中/微调/增删改组件/页面管理/选项)
#[tauri::command]
pub async fn edit_command(
    state: tauri::State<'_, IpcState>,
    payload: Value,
) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::EditCommand { payload }).await
}
