//! IPC 协议层 (D9): tauri command (async 线程) → mpsc → 主线程 dispatch → oneshot 回执。
//!
//! **!Send 不变量** (D8/AppShell 恒留主线程): command 侧只持 `std::sync::mpsc::Sender`
//! (Send+Clone), 真正的执行体 `dispatch` 在主线程被 `ShellForm::pump_once` 驱动 —
//! 绝不跨线程接触 AppShell。dispatch 为纯函数 + 可注入运行时, 不开 webview 即可单测。

use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use vm_core::formula::FormulaManager;

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
    /// 试听语音 (Java VoiceRowRenderer 播放按钮: loadClip + setFramePosition(0) +
    /// start, 忽略 enable 态; key = 配置键 voice_<alert>, pack = 当前选中包)
    PreviewVoice { key: String, pack: String },
    /// FM 列表 (fm_dir 扫描 .blkx 文件名; 阶段③ FMLIST 行)
    GetFmList,
    /// 导入外部配置 (备份+模板哈希合并, Java ConfigImportDialog; 阶段③)
    ImportConfig { path: String },
    /// 资产根目录绝对路径 (desc-img 图片气泡经 asset protocol 加载, 阶段③)
    GetAssetRoot,
    /// 打开对比 web 窗口 (批3: FMLIST 行 对比按钮 — Java FMListRowRenderer View,
    /// 选中机型单机视图 fm1=None; 窗口创建必须主线程, 故走 dispatcher 而非直算)
    OpenComparisonWindow { fm0: String, fm1: Option<String> },
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

/// 公式管理器共享 cell (E11 注入形态统一 — 原 commands_formula 全局桥的收敛位):
/// [`FormRuntime`] 字段持有, 同一实例经 tauri State 分发给公式编辑器**直算命令**
/// (命令线程直接读, 不经主线程 dispatcher — FormulaManager 全方法线程安全)。
/// 写点在 vm-app: 启动桥注入 load 过的实例 / 会话 start 覆盖 Service 实例。
#[derive(Clone, Default)]
pub struct FormulaShared {
    cell: Arc<RwLock<Arc<FormulaManager>>>,
}

impl FormulaShared {
    /// 换装 manager (vm-app 启动桥/会话覆盖写点)
    pub fn set(&self, mgr: Arc<FormulaManager>) {
        *self.cell.write().expect("公式 cell 锁中毒") = mgr;
    }

    /// 当前 manager (直算命令面读)
    pub fn get(&self) -> Arc<FormulaManager> {
        self.cell.read().expect("公式 cell 锁中毒").clone()
    }
}

/// About Modal 展示期共享 cell (B1 — 原 bridge.rs 静态 ABOUT_MODAL_UNTIL 的
/// FormRuntime 并入形态): 主循环经 ShellForm 标记/查询, 前端关闭回执命令经
/// tauri State 清零 (同 FormulaShared 分发模式)
pub type AboutModalShared = Arc<Mutex<Option<Instant>>>;

/// 阅读窗口上界 (Java showAbout 通知最长 24s, 放宽到 60s — 防遗忘态永久豁免
/// InGame 收窗)
const ABOUT_READ_WINDOW: Duration = Duration::from_secs(60);

/// dispatch 运行时 (阶段①壳态; 阶段②由 vm-app 注入 shell+form 真实现)。
/// E11 起 crate 内跨线程状态统一收敛为本 struct 的字段 (经 ShellForm 主线程
/// 访问 / 经 tauri State 命令线程访问), 不再有模块级静态可变态。
pub struct FormRuntime {
    /// 前端就绪时刻 (UiReady 首次到达)
    pub web_ready_at: Option<Instant>,
    /// Tauri AppHandle (批3: ShellForm 构造期注入 — dispatcher 开辅助 web 窗口
    /// 用; 窗口创建必须主线程, dispatcher 恰在主线程泵内执行, 无死锁面)
    pub app_handle: Option<tauri::AppHandle<tauri::Wry>>,
    /// 公式编辑器直算面的共享 manager (缺省 = 出厂空集, vm-app 装配方注入真实例)
    pub formula: FormulaShared,
    /// About Modal 展示期 (60s 阅读窗口上界; 见 AboutModalShared)
    pub about_modal_until: AboutModalShared,
}

impl Default for FormRuntime {
    fn default() -> Self {
        FormRuntime {
            web_ready_at: None,
            app_handle: None,
            formula: FormulaShared::default(),
            about_modal_until: Arc::new(Mutex::new(None)),
        }
    }
}

impl FormRuntime {
    /// 标记/清除 About Modal 展示期 (true = 开 60s 窗口; false = 立即清除)。
    /// 主循环 emit `about-requested` 时开启, 前端 Modal 关闭回执提前清零 —
    /// InGame 收窗分支读 [`Self::about_modal_open`] 豁免收窗 (B1: Java 通知
    /// 弹窗独立于 MainForm 可见性, 游戏中托盘"关于"恒可读, 不随 mStart 收窗闪没)
    pub fn set_about_modal_open(&self, open: bool) {
        let mut until = self
            .about_modal_until
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *until = if open { Some(Instant::now() + ABOUT_READ_WINDOW) } else { None };
    }

    /// About Modal 是否处于展示期 (60s 上界内)
    pub fn about_modal_open(&self) -> bool {
        self.about_modal_until
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some_and(|deadline| Instant::now() < deadline)
    }
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
        | RequestKind::PreviewVoice { .. }
        | RequestKind::GetFmList
        | RequestKind::ImportConfig { .. }
        | RequestKind::GetAssetRoot
        | RequestKind::OpenComparisonWindow { .. } => {
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
            // 批3: 开窗请求同样依赖注入侧 AppHandle (壳形态无 webview 可开)
            RequestKind::OpenComparisonWindow { fm0: "a_4h".into(), fm1: None },
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
