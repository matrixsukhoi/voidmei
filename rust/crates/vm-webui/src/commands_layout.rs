//! W4 HUD 布局编辑器命令面 (全部经主线程 dispatcher — Rc 配置树/渲染边界)。
//! 业务语义在 vm-app form_dispatch 的布局分支 + vm-overlay widgets 域。

use serde_json::Value;

use crate::commands::{roundtrip, IpcState};
use crate::ipc::RequestKind;

/// 组件目录 (palette: 类型名/显示名/分类/复合标记/属性 schema)
#[tauri::command]
pub async fn get_component_catalog(state: tauri::State<'_, IpcState>) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::GetComponentCatalog).await
}

/// 页面列表 (id/name/switchKey/出厂标记 + 升级提示)
#[tauri::command]
pub async fn get_pages(state: tauri::State<'_, IpcState>) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::GetPages).await
}

/// 页面保存 (delta: 出厂 → owned 提升 / 用户 → upsert)
#[tauri::command]
pub async fn save_page(state: tauri::State<'_, IpcState>, page: Value) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::SavePage { page }).await
}

/// 页面删除
#[tauri::command]
pub async fn delete_page(state: tauri::State<'_, IpcState>, id: String) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::DeletePage { id }).await
}

/// 页面恢复出厂
#[tauri::command]
pub async fn reset_page_to_factory(
    state: tauri::State<'_, IpcState>,
    id: String,
) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::ResetPageToFactory { id }).await
}
