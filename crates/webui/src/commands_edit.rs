//! R7 真窗编辑会话入口 (阶段 C 后唯一保留的编辑 IPC — MainForm footer
//! 「编辑HUD」按钮)。编辑动作全在渲染线程内部 (侧栏/悬浮条原生 chrome),
//! 出会话走悬浮条完成/放弃; 托盘自动收尾经 UiCommand 直达不经 IPC。

use serde_json::Value;

use crate::commands::{roundtrip, IpcState};
use crate::ipc::RequestKind;

/// 进入编辑会话 (真窗即画布: 桌面 overlay 直接可编辑)
#[tauri::command]
pub async fn begin_edit_session(state: tauri::State<'_, IpcState>) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::BeginEditSession).await
}
