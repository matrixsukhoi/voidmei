//! vm-webui lib 入口 (D9): Tauri 2 MainForm 壳, 窗口常驻隐藏预热 + 主线程手动泵。
//!
//! 与 D8 拓扑的接法 (vm-app 主循环, 对位原 iced 相A):
//! ```text
//! loop { shell.pump(); form.pump_once(); sleep(可见 ? 10ms : 50ms) }
//! ```
//! - `App::run_iteration` 手动泵事件 (tao, Windows 一等), 不阻塞不独占;
//! - IPC: command (tauri async 线程) → mpsc → [`ShellForm::pump_once`] 内 drain →
//!   dispatcher (主线程执行体) → oneshot 回执。**AppShell !Send 恒留主线程不变**;
//! - 窗口生命周期: 常驻隐藏, 托盘 Activate → `show()` (预热后 ≈100-200ms);
//!   开始 (mStart) → `hide()`; 窗口 X → 退出 (对位 Java setDefaultCloseOperation(3)
//!   = EXIT_ON_CLOSE; on_window_event prevent_close + emit, 前端走 EndGame 干净退出链);
//! - UI_READY 语义: 每次 show 由 vm-app 主循环发布 (rebuild 后 Init→preview 保真),
//!   本 crate 不持有 ui_bus (依赖方向: vm-app → vm-webui)。
//!
//! dispatcher 注入: 阶段①默认 [`ipc::dispatch`] (壳态); 阶段②起 vm-app 注入
//! 真实现 (AppShell + MainFormState 写链), FormRuntime 由本壳持有并传参。

pub mod bridge;
pub mod commands;
pub mod dto;
pub mod ipc;

use std::sync::mpsc;
use std::time::Instant;

use tauri::{App, Emitter, Manager, WebviewWindow, Wry};

use ipc::{FormRuntime, IpcReply, IpcRequest, RequestKind};

/// 主窗口 label (tauri.conf.json app.windows[0])
const MAIN_LABEL: &str = "main";

/// dispatch 执行体签名 (主线程调用; 阶段②由 vm-app 注入 shell 写链实现)
pub type Dispatcher = Box<dyn FnMut(RequestKind, &mut FormRuntime) -> IpcReply>;

/// Tauri MainForm 壳 (主线程独占; 内含 tauri App + IPC 接收端)
pub struct ShellForm {
    app: App<Wry>,
    rx: mpsc::Receiver<IpcRequest>,
    dispatcher: Dispatcher,
    /// 前端就绪/活性运行时 (web_ready_at 等; dispatcher 传参共享)
    rt: FormRuntime,
    /// 最近一次 WindowEcho 到达时刻 (--bench-reopen 测量终点)
    echo_at: Option<Instant>,
}

impl ShellForm {
    /// 构建常驻隐藏的 MainForm 壳 (build 不 run — 泵权归调用方)。
    /// Err = WebView2 不可用等致命面 (调用方降级监督模式, D9 风险表)。
    pub fn new(dispatcher: Dispatcher) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<IpcRequest>();
        let mut app = tauri::Builder::default()
            .plugin(tauri_plugin_dialog::init())
            .manage(commands::IpcState { tx })
            .invoke_handler(tauri::generate_handler![
                commands::ping,
                commands::ui_ready,
                commands::window_echo,
                commands::get_layout_tree,
                commands::get_combo_options,
                commands::form_message,
                commands::get_voice_packs,
                commands::get_fm_list,
                commands::import_config,
                commands::get_asset_root
            ])
            // 窗口 X = 退出 VoidMei (对位 Java MainForm.java:374
            // setDefaultCloseOperation(3)=EXIT_ON_CLOSE)。prevent_close 防
            // WebView2 默认销毁破坏常驻壳; hide 给即时视觉反馈, emit 交前端走
            // EndGame 干净退出链 (saveConfig + 主循环收尾, 覆盖 ✕/Alt+F4/任务栏关闭)
            .on_window_event(|window, event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                    let _ = window.emit("quit-requested", ());
                }
            })
            .build(tauri::generate_context!())
            .map_err(|e| format!("Tauri 壳构建失败 (WebView2 运行时缺失?): {e}"))?;
        // 隐藏窗口后台预热: WebView2 就绪/前端 dist 加载在 build 后首轮泵中推进,
        // 不阻塞调用方 (首启 1-3s 与 FM-Detect 并行, D9 决策)
        let _ = &mut app;
        Ok(ShellForm {
            app,
            rx,
            dispatcher,
            rt: FormRuntime::default(),
            echo_at: None,
        })
    }

    /// 默认 dispatcher (阶段①壳态: ipc::dispatch 纯函数)
    pub fn default_dispatcher() -> Dispatcher {
        Box::new(ipc::dispatch)
    }

    /// 单轮泵: tao 事件 + IPC drain-dispatch-回执 (主线程; 不阻塞)。
    /// deprecated 警告不适用: 我们有意分片泵 (调用方 sleep 10/50ms 节流),
    /// 非其文档所述"循环内 busy-loop 不让出"形态。
    #[allow(deprecated)]
    pub fn pump_once(&mut self) {
        self.app.run_iteration(|_handle, _event| {});
        while let Ok(req) = self.rx.try_recv() {
            if matches!(req.kind, RequestKind::WindowEcho) {
                self.echo_at = Some(Instant::now());
            }
            let reply = (self.dispatcher)(req.kind, &mut self.rt);
            if let Some(rtx) = req.reply {
                let _ = rtx.send(reply);
            }
        }
    }

    /// 主窗口句柄 (label = "main")
    pub fn main_window(&self) -> Option<WebviewWindow> {
        self.app.get_webview_window(MAIN_LABEL)
    }

    /// AppHandle 克隆 (事件桥/emit 用; Send+Sync 可跨线程)
    pub fn app_handle(&self) -> tauri::AppHandle<Wry> {
        self.app.handle().clone()
    }

    /// 显示设置窗 (预热重开路径; emit window-echo 供前端活性回执)
    pub fn show(&self) {
        if let Some(w) = self.main_window() {
            let _ = w.show();
            let _ = w.set_focus();
            let _ = self.app.emit("window-echo", ());
        }
    }

    /// 隐藏设置窗 (开始路径; X 退出路径的 hide 在 on_window_event 内联)
    pub fn hide(&self) {
        if let Some(w) = self.main_window() {
            let _ = w.hide();
        }
    }

    /// 主窗口当前可见性 (主循环同步 visible 态 — 窗口 X 被 on_window_event
    /// 拦截转 hide, Rust 侧经查询感知)
    pub fn is_main_visible(&self) -> bool {
        self.main_window().is_some_and(|w| w.is_visible().unwrap_or(false))
    }

    /// 前端是否已就绪 (UiReady 到达; show 前置参考, 非强制)
    pub fn is_web_ready(&self) -> bool {
        self.rt.web_ready_at.is_some()
    }

    /// 最近一次 WindowEcho 时刻 (bench: show() → echo 到达 = 预热重开延迟·webview 口径)
    pub fn echo_at(&self) -> Option<Instant> {
        self.echo_at
    }

    /// bench 辅助: 清零 echo 记录 (每轮 show 前调用)
    pub fn reset_echo(&mut self) {
        self.echo_at = None;
    }
}
