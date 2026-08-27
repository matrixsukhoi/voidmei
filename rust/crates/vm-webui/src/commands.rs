//! tauri command 薄壳: 前端 invoke → mpsc → 主线程 dispatch → oneshot 回执。
//! 本文件只做转发 (Send 面), 业务语义全在 ipc::dispatch (主线程纯函数)。

use std::sync::mpsc;

use serde_json::Value;

use crate::dto::FormMessageDto;
use crate::ipc::{IpcReply, IpcRequest, RequestKind};

/// managed 状态: 主线程通道的发送端 (command 线程持有, Send+Clone)
pub struct IpcState {
    pub tx: mpsc::Sender<IpcRequest>,
}

/// 发送并等待回执的公共路径 (所有 command 复用)
async fn roundtrip(tx: &mpsc::Sender<IpcRequest>, kind: RequestKind) -> Result<Value, String> {
    let (rtx, rrx) = tokio::sync::oneshot::channel::<IpcReply>();
    tx.send(IpcRequest {
        kind,
        reply: Some(rtx),
    })
    .map_err(|e| format!("IPC 通道关闭: {e}"))?;
    match rrx.await.map_err(|e| format!("IPC 回执丢失: {e}"))? {
        IpcReply::Ok(v) => Ok(v),
        IpcReply::Err(e) => Err(e),
    }
}

/// 活性探测 (阶段① POC)
#[tauri::command]
pub async fn ping(state: tauri::State<'_, IpcState>, nonce: u64) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::Ping { nonce }).await
}

/// 前端就绪上报 (D9 预热链路)
#[tauri::command]
pub async fn ui_ready(state: tauri::State<'_, IpcState>) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::UiReady).await
}

/// show 后活性回执 (--bench-reopen 测量终点)
#[tauri::command]
pub async fn window_echo(state: tauri::State<'_, IpcState>) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::WindowEcho).await
}

/// 全量 cfg 树 (PanelDto[]; import/reset 后前端重新拉取)
#[tauri::command]
pub async fn get_layout_tree(state: tauri::State<'_, IpcState>) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::GetLayoutTree).await
}

/// 下拉选项源解析 (Rust 侧同款缓存; _FONTS_ 依赖当前值)
#[tauri::command]
pub async fn get_combo_options(
    state: tauri::State<'_, IpcState>,
    source: String,
    current: String,
) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::GetComboOptions { source, current }).await
}

/// 表单消息 (→ main_form::update 全链; WYSIWYG 写回在 Rust 侧完成)
#[tauri::command]
pub async fn form_message(
    state: tauri::State<'_, IpcState>,
    message: FormMessageDto,
) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::FormMessage(message)).await
}

/// 语音包列表 (阶段③ VOICE/VOICE_GLOBAL 行)
#[tauri::command]
pub async fn get_voice_packs(state: tauri::State<'_, IpcState>) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::GetVoicePacks).await
}

/// FM 列表 (阶段③ FMLIST 行)
#[tauri::command]
pub async fn get_fm_list(state: tauri::State<'_, IpcState>) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::GetFmList).await
}

/// 导入外部配置 (阶段③ importConfig; path 来自 tauri-plugin-dialog 的 open())
#[tauri::command]
pub async fn import_config(state: tauri::State<'_, IpcState>, path: String) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::ImportConfig { path }).await
}

/// 资产根目录 (desc-img 图片气泡; 阶段③)
#[tauri::command]
pub async fn get_asset_root(state: tauri::State<'_, IpcState>) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::GetAssetRoot).await
}
