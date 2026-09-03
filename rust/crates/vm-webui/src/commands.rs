//! tauri command 薄壳: 前端 invoke → mpsc → 主线程 dispatch → oneshot 回执。
//! 本文件只做转发 (Send 面), 业务语义全在 ipc::dispatch (主线程纯函数)。
//! 例外 (不经 dispatcher 的直算命令, 与窗口数据域同款模式):
//! [`get_app_version`] (静态值) / [`fm_list`] (目录扫描, 见其注释)。

use std::sync::mpsc;

use serde_json::Value;

use crate::dto::FormMessageDto;
use crate::ipc::{IpcReply, IpcRequest, RequestKind};

/// Serialize → invoke 返回值 (直算命令薄壳共用, 三域拆分时自 commands_windows 收编)
pub(crate) fn to_json<T: serde::Serialize>(v: &T) -> Result<serde_json::Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

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

/// 试听语音 (Java VoiceRowRenderer 播放按钮; key=voice_<alert> 配置键)
#[tauri::command]
pub async fn preview_voice(
    state: tauri::State<'_, IpcState>,
    key: String,
    pack: String,
) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::PreviewVoice { key, pack }).await
}

/// FM 列表 (阶段③ FMLIST 行)
#[tauri::command]
pub async fn get_fm_list(state: tauri::State<'_, IpcState>) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::GetFmList).await
}

/// 导入外部配置 (阶段③ importConfig; path 来自 tauri-plugin-dialog 的 open())
#[tauri::command]
pub async fn import_config(
    state: tauri::State<'_, IpcState>,
    path: String,
) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::ImportConfig { path }).await
}

/// 资产根目录 (desc-img 图片气泡; 阶段③)
#[tauri::command]
pub async fn get_asset_root(state: tauri::State<'_, IpcState>) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::GetAssetRoot).await
}

/// 打开对比 web 窗口 (批3 FMLIST 行 对比按钮 — Java FMListRowRenderer View:
/// 选中机型单机视图 fm1=None)。经 mpsc → 主线程 dispatcher 执行 (窗口创建必须
/// 主线程, W1 直算先例不适用)
#[tauri::command]
pub async fn open_comparison_window(
    state: tauri::State<'_, IpcState>,
    fm0: String,
    fm1: Option<String>,
) -> Result<Value, String> {
    roundtrip(&state.tx, RequestKind::OpenComparisonWindow { fm0, fm1 }).await
}

/// 应用版本号 (Java Application.readVersion):
/// MANIFEST Implementation-Version ↔ 构建期 VOIDMEI_VERSION 环境变量注入
/// (build.py jar / CI 从 git tag 提取同源); 未注入 = 本地开发形态 → "dev"
/// (checkUpdate 的 dev 守卫以此判定, 标题栏版本同源)。
pub fn app_version() -> &'static str {
    option_env!("VOIDMEI_VERSION").unwrap_or("dev")
}

/// 版本号查询 (静态值, 无需 IPC roundtrip — 不经主线程 dispatcher)
#[tauri::command]
pub fn get_app_version() -> String {
    app_version().to_string()
}

/// FM 机型列表 (GridSelectorDialog.loadPlanes 搜索下拉; fm/ 物理文件目录)。
///
/// 与本文件 [`get_fm_list`] 双命令并存, 接线按窗口对号不可混用:
/// 设置页 FMLIST 行走 get_fm_list (mpsc → 主线程 dispatcher, 对位 Java
/// FMListRowRenderer); 对比/功率曲线窗口的机型下拉走本命令 (直连 vm-core,
/// 对位 GridSelectorDialog.loadPlanes)。当前两者数据面同源 (fm/ 目录 FileUtils
/// 剥后缀全枚举 + 排序), 差在通道 — dispatcher 版受主线程泵节流, 直连版即时;
/// 未来演化路径也不同, 前端用错源会造成两窗口列表不一致。
/// (直算模式与 W3 备案见 commands_comparison 模块头)
#[tauri::command]
pub async fn fm_list() -> Result<serde_json::Value, String> {
    to_json(&vm_core::fm::data_paths::list_fm_names("fm"))
}

/// 真机数据根注入 (数据域测试共用: data/ 缺失 → false, 调用方打印真因后 SKIP)
#[cfg(test)]
pub(crate) fn ensure_real_data() -> bool {
    // vm-webui 位于 rust/crates/vm-webui → 仓库根 = ../../.. (realtests 同款)
    let root = format!("{}/../../../data", env!("CARGO_MANIFEST_DIR"));
    if !std::path::Path::new(&root)
        .join("aces/gamedata/flightmodels")
        .exists()
    {
        return false;
    }
    vm_core::fm::data_paths::set_data_root(&root);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 真机_fm列表_物理文件目录() {
        if !ensure_real_data() {
            println!("SKIP: 真机 data/ 不存在 (fm_list 无数据源)");
            return;
        }
        let planes = vm_core::fm::data_paths::list_fm_names("fm");
        assert!(planes.len() > 100, "fm/ 目录应有千级机型: {}", planes.len());
        assert!(
            planes.contains(&"spitfire_f24".to_string()),
            "应含 spitfire_f24"
        );
        assert!(
            planes.contains(&"a-10c".to_string()),
            "应含连字符机型 a-10c"
        );
        // 已排序
        let mut sorted = planes.clone();
        sorted.sort();
        assert_eq!(planes, sorted);
    }
}
