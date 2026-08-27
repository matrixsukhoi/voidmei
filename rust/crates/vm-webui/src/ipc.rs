//! IPC 协议层 (D9): tauri command (async 线程) → mpsc → 主线程 dispatch → oneshot 回执。
//!
//! **!Send 不变量** (D8/AppShell 恒留主线程): command 侧只持 `std::sync::mpsc::Sender`
//! (Send+Clone), 真正的执行体 `dispatch` 在主线程被 `ShellForm::pump_once` 驱动 —
//! 绝不跨线程接触 AppShell。dispatch 为纯函数 + 可注入运行时, 不开 webview 即可单测。

use std::time::Instant;

use crate::dto::FormMessageDto;

/// 前端请求体 (阶段②: cfg 树/表单消息/下拉源; 壳态请求由 default dispatch 兜底)
#[derive(Debug, Clone)]
pub enum RequestKind {
    /// 活性探测 (回 nonce)
    Ping { nonce: u64 },
    /// 前端就绪上报 (D9 预热链路: web_ready_at 记录, bench-reopen 起点)
    UiReady,
    /// show 后活性回执 (bench: Rust show() emit → 前端听到 → 本回执)
    WindowEcho,
    /// 全量 cfg 树 (12 panels; import/reset 后前端重新拉取刷新)
    GetLayoutTree,
    /// 下拉选项源解析 (_FONTS_/_CROSSHAIRS_/目录扫描, 带 Rust 侧同款缓存)
    GetComboOptions { source: String, current: String },
    /// 表单消息 (→ vm-ui main_form::update 全链, WYSIWYG 写回; 含 Save/StartGame 等)
    FormMessage(FormMessageDto),
    /// 语音包列表 (voice/ 目录扫描, Java get_available_packs 语义; 阶段③)
    GetVoicePacks,
    /// FM 列表 (fm_dir 扫描 .blkx 文件名; 阶段③ FMLIST 行)
    GetFmList,
    /// 导入外部配置 (备份+模板哈希合并, Java ConfigImportDialog; 阶段③)
    ImportConfig { path: String },
    /// 资产根目录绝对路径 (desc-img 图片气泡经 asset protocol 加载, 阶段③)
    GetAssetRoot,
}

/// 一条 IPC 请求 (含回执通道; 单向通知类 reply=None)
pub struct IpcRequest {
    pub kind: RequestKind,
    pub reply: Option<tokio::sync::oneshot::Sender<IpcReply>>,
}

/// dispatch 结果 (serde_json 值面向前端)
#[derive(Debug, Clone)]
pub enum IpcReply {
    Ok(serde_json::Value),
    Err(String),
}

/// dispatch 运行时 (阶段①壳态; 阶段②由 vm-app 注入 shell+form 真实现)
#[derive(Default)]
pub struct FormRuntime {
    /// 前端就绪时刻 (UiReady 首次到达)
    pub web_ready_at: Option<Instant>,
}

/// 请求执行体 (纯函数, 主线程调用; 单测不开 webview)。
/// 阶段②数据面请求 (GetLayoutTree/GetComboOptions/FormMessage) 需要 MainFormState,
/// 由 vm-app 注入的 dispatcher 承担 — 本默认实现返回 Err (selftest 壳形态)。
pub fn dispatch(kind: RequestKind, rt: &mut FormRuntime) -> IpcReply {
    match kind {
        RequestKind::Ping { nonce } => IpcReply::Ok(serde_json::json!({
            "nonce": nonce,
            "pong": true,
        })),
        RequestKind::UiReady => {
            if rt.web_ready_at.is_none() {
                rt.web_ready_at = Some(Instant::now());
            }
            IpcReply::Ok(serde_json::json!({ "ok": true }))
        }
        RequestKind::WindowEcho => IpcReply::Ok(serde_json::json!({ "echo": true })),
        RequestKind::GetLayoutTree
        | RequestKind::GetComboOptions { .. }
        | RequestKind::FormMessage(_)
        | RequestKind::GetVoicePacks
        | RequestKind::GetFmList
        | RequestKind::ImportConfig { .. }
        | RequestKind::GetAssetRoot => {
            IpcReply::Err("壳形态 dispatcher 不支持数据面请求 (应由 vm-app 注入)".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_回投_nonce() {
        let mut rt = FormRuntime::default();
        let IpcReply::Ok(v) = dispatch(RequestKind::Ping { nonce: 42 }, &mut rt) else {
            panic!("期望 Ok");
        };
        assert_eq!(v["nonce"], 42);
        assert_eq!(v["pong"], true);
    }

    #[test]
    fn ui_ready_只记录首次() {
        let mut rt = FormRuntime::default();
        assert!(dispatch(RequestKind::UiReady, &mut rt).is_ok_variant());
        assert!(rt.web_ready_at.is_some());
        // 二次 ready 不覆盖 (常驻窗口只 ready 一次)
        let first = rt.web_ready_at;
        dispatch(RequestKind::UiReady, &mut rt);
        assert!(rt.web_ready_at == first);
    }

    #[test]
    fn window_echo_回显() {
        let mut rt = FormRuntime::default();
        let IpcReply::Ok(v) = dispatch(RequestKind::WindowEcho, &mut rt) else {
            panic!("期望 Ok");
        };
        assert_eq!(v["echo"], true);
    }

    #[test]
    fn 数据面请求_壳形态拒绝() {
        // 壳 dispatcher (selftest) 不持 MainFormState — 数据面请求必须显式 Err
        // 而非静默成功 (vm-app 注入的真 dispatcher 承担; 协议转移自原 vm-ui lib hooks 测试)
        let mut rt = FormRuntime::default();
        let cases = vec![
            RequestKind::GetLayoutTree,
            RequestKind::GetComboOptions { source: "_FONTS_".into(), current: "x".into() },
            RequestKind::FormMessage(crate::dto::FormMessageDto::Save),
        ];
        for c in cases {
            match dispatch(c, &mut rt) {
                IpcReply::Err(e) => assert!(e.contains("vm-app"), "拒绝信息应指向注入方: {e}"),
                IpcReply::Ok(_) => panic!("壳形态不应成功处理数据面请求"),
            }
        }
    }

    impl IpcReply {
        fn is_ok_variant(&self) -> bool {
            matches!(self, IpcReply::Ok(_))
        }
    }
}
