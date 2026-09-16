//! voidmei 主程序组装 (P5 批十四最终组装): 对齐 Java `Launcher → Application.main`
//! 启动序, GPU 兼容段按 D5 消亡 (Java2D sun.java2d.* 属性专属, Rust 无对应物 —
//! overlay 渲染走 tiny-skia 纯 CPU 软渲染, D1 决策本身即 GPU 兼容哲学)。
//!
//! Java Application.main 启动序对位:
//! 1. Logger 级别 (debugLog||debug → DEBUG) → `--debug` 参数 +
//!    cfg 键 debugLog (`read_debug_log_flag`); debugLog 重定向
//!    (output.log/error.log) 同源
//! 2. Lang.initLang + 端口/屏幕探测    → `AppShell::new` → `Env::probe`
//! 3. initFont (字体)                 → Env.fonts_dir → 渲染线程 (D8: 字体→渲染线程)
//! 4. initSystemTray                  → 渲染线程内 (D8 单泵共享)
//! 5. Java UI 线程派发: initWebLaf + `new Controller(true)` + checkUpdate
//!    → 主线程组装: rebuild_controller(true) (AppShell::new 内) + web MainForm;
//!    checkUpdate → 前端 (web 就绪后异步一次, web/src/dialogs.tsx 的
//!    VersionChecker; 版本源 get_app_version 命令, dev 守卫同 Java)。
//!
//! 相位主循环 (D9 后为 web 壳单循环; 原 iced 相 A/B 已合并, 见 desktop_main 注):
//! - 主线程: `shell.pump()` + `ShellForm::pump_once()` (tao 事件 + IPC) +
//!   sleep(可见 10ms / 隐藏 50ms); 设置窗常驻隐藏预热, 每次 show 发布 UI_READY。
//! - 无窗降级 (`run_supervisor_phase`) 与 `--live` 形态保留。
//!
//! CLI:
//! - `--live`: 对齐 `autoStartGameMode=true` — 跳过 MainForm,
//!   Controller 自启动 Service, 主线程直接进监督循环。
//! - `--port <p>`: 端口覆盖 (调试/打桩用, 见 doc/打桩调试手册.md)。白盒测试
//!   端口约定: 一律 9222 (Java 备用端口 appPortBkp 域, 游戏本地 API 恒占 8111
//!   而 9222 游戏永不监听) — 真机在跑测试也不再被挤掉/误读游戏数据。
//! - `--debug`: Application.debug = true (Logger DEBUG 级)。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use voidmei::form_dispatch;
use voidmei::{AppShell, SupervisorOutcome};

use tauri::Emitter;
use kernel::base::bus::ui_state_bus::UIStateBus;
use kernel::base::event::ui_state_events;
use kernel::base::logger;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let debug = args.iter().any(|a| a == "--debug");
    install_panic_hook();

    // Java Application.main:539-543 Logger 级别: debugLog || debug → DEBUG, 否则
    // INFO。debugLog 是 cfg 键 (批3裁决配置化, 缺省 false), 须先读配置再定级 —
    // 审查 A1: 原实现漏并 ||, cfg 键 debugLog=true 时仅重定向日志文件而级别仍
    // INFO (output.log 缺 DEBUG 行)。先读后判的代价: 配置装载期的日志行走默认
    // INFO 级 (Java 读的是编译期静态, 无此面; 仅丢装载期的 debug 行, 可接受)
    let debug_log = read_debug_log_flag();
    logger::set_min_level(if debug || debug_log {
        logger::Level::Debug
    } else {
        logger::Level::Info
    });

    // Java Application.main:550-553: debugLog → setDebugLog("./output.log") +
    // setErrLog("./error.log") (System.setOut/setErr 重定向)。重定向须赶在任何
    // Logger 输出前 — read_debug_log_flag 独立轻装载配置树读键 (AppShell 随后
    // 完整装载; initialize 幂等: 首跑拷模板+存哈希, 二次装载哈希命中跳过合并,
    // 无双写副作用)。
    if debug_log {
        logger::set_debug_log("./output.log");
        logger::set_err_log("./error.log");
    }

    let code = if args.iter().any(|a| a == "--live") {
        // 端口覆盖 (打桩/调试, 见 CLI 头注), 不与真机 8111 冲突
        let port = parse_port_arg(&args);
        live_main(debug, port)
    } else {
        desktop_main(debug)
    };
    std::process::exit(code);
}

/// 波21 落地 (fm/loader.rs 与 ui_state_bus.rs 两处备案多年的
/// "App 组装层统一处置"): 默认 panic hook 在 catch_unwind 捕获前把
/// "thread ... panicked" 打到 stderr (不受日志级别控制), 与库内
/// error_with_throwable 构成双报告噪音; App 层统一为 logger 单通道。
/// 进程级副作用只在 bin 入口装 — cargo test 另起 harness 不受影响。
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "未知位置".to_string());
        let payload = info.payload();
        let msg = kernel::base::exception_helper::panic_message(payload);
        logger::error("Panic", &format!("panic at {loc}: {msg}"));
    }));
}

// =====================================================================
// 默认路径: Tauri web 壳单循环 (D9, 原 iced 相 A/B 合并; 常驻隐藏预热)
// =====================================================================

/// 单循环主形态 (D9 后):
/// - 主线程 = `shell.pump()` (监督事件 + drive_from_live) + `form.pump_once()`
///   (tao 事件 + IPC drain-dispatch) + sleep(可见 10ms / 隐藏 50ms);
/// - 设置窗常驻隐藏: 启动即后台预热 WebView2 (首启 1-3s 与 FM-Detect 并行),
///   前端就绪后首显 (对位原"启动即开窗"); 托盘 Activate → 核重建 → show;
/// - **每次 show 发布 UI_READY** (对位原相 A 每次构造 MainForm 的
///   on_ready → uiReadyHandler → Preview 链, rebuild 后新核必经此进 Preview);
/// - 窗口 X 由 webui on_window_event 拦截转 hide (对位 Java DISPOSE→托盘态);
/// - 核状态进游戏 (Connected/InGame) 时收窗 (Java confirm 的 setVisible(false));
/// - 壳不可用 (WebView2 缺失等) → 降级无窗阻塞监督 (托盘可退出)。
fn desktop_main(debug: bool) -> i32 {
    let mut shell = match AppShell::new(debug, false) {
        Ok(s) => s,
        Err(e) => {
            logger::error("App", &format!("AppShell 构造失败: {e}"));
            return 1;
        }
    };
    // D8: 渲染线程先行 (托盘 + overlay host + 热键泵)。预览模式全开语义 =
    // UI_READY → Preview() → RefreshPreviews 命令由主循环泵触发,
    // 对齐 Java autoStartGameMode=false 默认 (MainForm 先行, 预览窗随后)
    if let Err(e) = shell.spawn_render_thread() {
        logger::error("App", &format!("渲染线程启动失败: {e}"));
        return 1;
    }

    // UI_READY 发布句柄 (每次 show 发一次; 见函数头注)
    let ui_bus = Arc::clone(&shell.ui_bus);

    // 表单态 cell (D9 阶段②): dispatcher 与主循环共享 (Rc 单线程);
    // 初始 = 与首个核同源 (对位原相 A 首次 build_form_state)
    let form_cell: form_dispatch::FormCell = {
        let init = form_dispatch::build_form_state(&shell);
        Rc::new(RefCell::new(Some(init)))
    };

    // AppShell 主线程持有 (D8: 含 !Send 配置树恒留主线程 — 原共享点已随单循环
    // 消失, Rc 仅留 dispatcher 注入用; Dispatcher 闭包无 Send 界, Rc 即可)
    let shell = Rc::new(RefCell::new(shell));

    // 公式系统启动桥 (E11 注入形态统一 → AppShell 共享 cell): Service 未装配的
    // 空闲/preview 期, 公式编辑器 tab 也要可用 — 先注入独立 manager (装载出厂+
    // 用户文件); 进游戏模式 Controller::start 覆盖为会话实例 (编辑保存已落盘,
    // 会话装载不丢)
    {
        let mgr = std::sync::Arc::new(kernel::formula::FormulaManager::new());
        mgr.load_from_files();
        shell.borrow().formula_shared.set(mgr);
    }

    // Tauri 壳: 常驻隐藏, build 即后台预热 (不阻塞; 首显等 is_web_ready);
    // dispatcher = 表单写链真实现 (数据面请求经 MainFormState/UiCommand);
    // formula cell 传入接线 (E11): 命令线程经 tauri State 读到同源实例
    let formula_shared = shell.borrow().formula_shared.clone();
    let mut form = match webui::ShellForm::new(
        form_dispatch::make_dispatcher(&shell, Rc::clone(&form_cell)),
        formula_shared,
    ) {
        Ok(f) => Some(f),
        Err(e) => {
            logger::error("App", &format!("Web 设置壳不可用, 降级无窗监督: {e}"));
            None
        }
    };
    // 事件桥: CONFIG_CHANGED → 前端 config-changed (reset/import 后整树刷新);
    // FM_CHANGED → fm-changed (MISSING/CORRUPT toast, 对位 NotificationService);
    // Subscription RAII — 与主循环同生命周期
    let fm_changed_bus = shell.borrow().fm.fm_changed_bus();
    let _bridge_sub = form.as_ref().map(|f| {
        (
            webui::bridge::bridge_config_changed(f.app_handle(), &ui_bus),
            webui::bridge::bridge_fm_changed(f.app_handle(), &fm_changed_bus),
            // R7 真窗编辑会话事件 (阶段 C 后仅 begin/end — MainForm 隐退/恢复)
            webui::bridge::bridge_hud_edit(f.app_handle(), &ui_bus),
        )
    });

    // (JSON 化退役: 旧 config_manager 的 ParseError/MergeReport 弹窗桥 —
    // delta 损坏走 json_store 的 .corrupt 隔离 + 日志, 无弹窗面)

    // Java Controller(true) 的自启动分支 (autoStartGameMode=true): 不显设置窗
    // (UI_READY 不发布, live 模式不被 Preview 翻转)。仅 desktop 形态首迭代判定
    let mut first_iteration = true;
    let mut initial_shown = false;
    // W2: 启动期 (sink 安装前) config 弹窗缓存的回放是否已尝试 (web 就绪后一次)
    let mut startup_dialog_replayed = false;
    // StatusBar 面: 核状态变化 → 前端 controller-state (Init/Preview/Connected/InGame)
    let mut last_state = String::new();
    // rule_triggers 的已消费帧序号 (帧序号去重, 见循环内 W5 注)
    let mut rule_triggers_seen: u64 = 0;
    loop {
        let auto_started = first_iteration
            && shell
                .borrow()
                .controller
                .as_ref()
                .is_some_and(|c| c.service.is_some());
        if auto_started {
            initial_shown = true; // 自启动形态永不主动开窗
        }
        first_iteration = false;

        // 壳不可用降级: 阻塞监督 (事件驱动响应快); 托盘请求设置窗时无窗可开,
        // 核已重建, 记日志继续 (run_supervisor 形态同款)
        let Some(form) = form.as_mut() else {
            match shell.borrow_mut().run_supervisor_phase() {
                SupervisorOutcome::Exit => break,
                SupervisorOutcome::MainFormRequested => {
                    logger::info("App", "托盘请求设置窗 — web 壳不可用, 已重建核继续监督")
                }
            }
            continue;
        };

        let (exit, form_req, in_game, state_str) = {
            let mut s = shell.borrow_mut();
            s.pump();
            // live 模式运行判定 (收窗面): Connected/InGame = start() 后的形态
            let state = s.shared.state();
            let in_game = matches!(
                state,
                voidmei::ControllerState::Connected | voidmei::ControllerState::InGame
            );
            (
                s.is_exit_requested(),
                s.take_form_request(),
                in_game,
                format!("{state:?}"),
            )
        };
        // web 桥泵: 状态推送 + 规则触发转发 + 启动期弹窗回放 (见函数注)
        pump_web_bridges(
            form,
            &shell,
            &mut last_state,
            &mut rule_triggers_seen,
            &mut startup_dialog_replayed,
            &state_str,
        );

        if exit {
            break; // EndGame (mCancel IPC, 阶段②接线) / 托盘 Exit
        }

        // 托盘"关于" (Java about 菜单三段 showAbout) → 前端 About Modal。
        // Java 的通知弹窗独立于 MainForm 可见性; web 形态 Modal 寄居设置窗 —
        // 窗隐藏期 (托盘驻留常态) 连带 show 设置窗, 否则 Modal 落在不可见窗内。
        // B1 修复: emit 前标记 Modal 展示期 (仅 web 就绪时 — 冷启动期前端监听
        // 未注册, 事件会丢且标记无人清, 不标记防 InGame 恒不收窗), 下方 InGame
        // 收窗分支凭标记豁免; 前端 Modal 关闭回执 (about_modal_closed 命令)
        // 或 60s 上界清除标记
        if shell.borrow_mut().take_about_request() {
            if form.is_web_ready() {
                form.set_about_modal_open(true);
            }
            let lang = kernel::lang::Lang::init_lang();
            let payload = webui::bridge::AboutPayload {
                version: webui::commands::app_version().to_string(),
                contents: [
                    lang.aboutcontent.to_string(),
                    lang.aboutcontentsub1.to_string(),
                    lang.aboutcontentsub2.to_string(),
                ],
            };
            if let Err(e) = form.app_handle().emit("about-requested", payload) {
                logger::warn("App", &format!("about 事件发送失败: {e}"));
            }
            if !form.is_main_visible() {
                form.show();
            }
        }

        // 窗口形态一步: 开窗/收窗决策 (见函数注)
        window_visibility_step(
            form,
            &shell,
            &form_cell,
            &ui_bus,
            form_req,
            in_game,
            &mut initial_shown,
        );

        form.pump_once();
        // 泵率: 可见期 10ms (IPC 交互手感 — 滑条/选色实时回执), 隐藏期 50ms
        // (监督节拍, 对位原 iced 50ms Tick)
        let visible = form.is_main_visible();
        std::thread::sleep(Duration::from_millis(if visible { 10 } else { 50 }));
    }
    shell.borrow_mut().shutdown();
    0
}

/// web 桥泵 (主循环每迭代的 web 前端转发面, 自 desktop_main 主循环拆出):
/// - StatusBar 状态推送: 核状态变化 → `controller-state` 事件;
/// - 规则触发转发: `rule_triggers` → 前端 `rule-triggered` toast (W5 消费链
///   首段, 帧序号去重 — 波4, 冷却态机已保证不刷屏);
/// - 启动期 config 弹窗缓存回放 (web 就绪后一次)。
/// 去重/回放状态经可变参数随循环持有
fn pump_web_bridges(
    form: &mut webui::ShellForm,
    shell: &Rc<RefCell<AppShell>>,
    last_state: &mut String,
    rule_triggers_seen: &mut u64,
    startup_dialog_replayed: &mut bool,
    state_str: &str,
) {
    // StatusBar: 核状态变化 → 前端 controller-state
    if state_str != last_state {
        *last_state = state_str.to_string();
        // 审查 W4: 静默吞 emit 失败 → 徽标失更新无自愈, 至少留告警面
        if let Err(e) = form.app_handle().emit("controller-state", state_str) {
            logger::warn("App", &format!("controller-state 事件发送失败: {e}"));
        }
    }

    // W5: 规则触发事件转发 (rule_triggers → 前端 toast; 消费链首段)。
    // 波4: 帧序号去重 (原 ServiceData 读后清空 drain 语义的帧仓等价物);
    // 冷却态机已保证触发不刷屏
    {
        let triggers: Vec<_> = {
            let shell = shell.borrow();
            let live = shell.shared.live.read().expect("live 锁中毒").clone();
            match live.as_ref().and_then(|frames| frames.latest()) {
                Some(f) if f.frame_seq != *rule_triggers_seen => {
                    *rule_triggers_seen = f.frame_seq;
                    f.rule_triggers.clone()
                }
                _ => Vec::new(),
            }
        };
        for t in &triggers {
            let (kind, arg) = match &t.action {
                kernel::formula::rules::RuleAction::Toast(msg) => ("toast", msg.clone()),
                kernel::formula::rules::RuleAction::Voice(key) => ("voice", key.clone()),
                kernel::formula::rules::RuleAction::Flag(name) => ("flag", name.clone()),
            };
            let payload = serde_json::json!({
                "rule": t.rule, "kind": kind, "arg": arg, "at": t.at_ms,
            });
            if let Err(e) = form.app_handle().emit("rule-triggered", payload) {
                logger::warn("App", &format!("rule-triggered 发送失败: {e}"));
            }
        }
    }

    // (JSON 化退役: 旧启动期 config 弹窗缓存回放 — 无弹窗面)
    if !*startup_dialog_replayed && form.is_web_ready() {
        *startup_dialog_replayed = true;
    }
}

/// 窗口形态一步 (主循环的开窗/收窗决策, 自 desktop_main 主循环拆出):
/// - 托盘 Activate (`form_req`): 表单态随核重建 + show + UI_READY → 新核进
///   Preview (show 幂等);
/// - 首显: 预热就绪即开窗 (对位原"启动即开设置窗");
/// - 进游戏收窗: 对位 Java confirm 的 setVisible(false), About Modal 展示期
///   豁免 (B1 — 见调用点下方注释)。
/// `initial_shown` 经可变参数随循环持有
fn window_visibility_step(
    form: &mut webui::ShellForm,
    shell: &Rc<RefCell<AppShell>>,
    form_cell: &form_dispatch::FormCell,
    ui_bus: &Arc<UIStateBus>,
    form_req: bool,
    in_game: bool,
    initial_shown: &mut bool,
) {
    let visible = form.is_main_visible();
    if form_req && !in_game {
        // 托盘 Activate: 核已由 handle_main_event 重建 (rebuild_controller) —
        // 表单态随之重建 (与核共享新 config 服务, 对位原相 A 重开窗的重新构造),
        // show 幂等 (可能已可见) + UI_READY → 新核进 Preview
        {
            let s = shell.borrow();
            *form_cell.borrow_mut() = Some(form_dispatch::build_form_state(&s));
        }
        form.show();
        *initial_shown = true;
        publish_ui_ready(ui_bus);
    } else if !visible && !in_game && !*initial_shown && form.is_web_ready() {
        // 首显: 预热就绪即开窗 (对位原"启动即开设置窗")
        form.show();
        *initial_shown = true;
        publish_ui_ready(ui_bus);
    } else if visible && in_game && !form.about_modal_open() {
        // 开始 (托盘 Start / StartGame; mStart): 收窗, 对位 confirm 的
        // setVisible(false)。About Modal 展示期豁免 (B1): Java 通知弹窗独立
        // 于 MainForm 可见性, 游戏中托盘"关于"恒可读 — Modal 关闭回执/超时
        // 清标记后下一轮恢复收窗
        form.hide();
    }
}

/// UI_READY 发布 (Java MainForm 首显 → uiReadyHandler → Preview 的触发面)
fn publish_ui_ready(bus: &Arc<UIStateBus>) {
    bus.publish(ui_state_events::UI_READY, Some("MainForm"), None);
}

/// debugLog cfg 键读取 (Java Application.debugLog 静态开关的配置化, 缺省 false)。
/// 值域 = switch 行的 "true"/"false" 字符串 — Boolean.parseBoolean 语义
/// (equalsIgnoreCase("true"), 其余恒 false)
fn read_debug_log_flag() -> bool {
    use kernel::config::config_api::ConfigProvider as _;
    let cs = kernel::config::configuration_service::ConfigurationService::new(None);
    cs.init_config();
    cs.get_config("debugLog")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true")
}

// =====================================================================
// --live: 跳过 MainForm 直接 live 模式 (Java autoStartGameMode=true;
// 旧名 --game-mode, 术语 preview↔live 对仗, 见 D9 后命名统一)
// =====================================================================

fn live_main(debug: bool, port_override: Option<u16>) -> i32 {
    let shell = match AppShell::new_with_port(debug, true, port_override) {
        Ok(s) => s,
        Err(e) => {
            logger::error("App", &format!("AppShell 构造失败: {e}"));
            return 1;
        }
    };
    // 阻塞监督循环 (内含渲染线程自动补启防呆; Exit 托盘命令/通道关闭退出)
    shell.run_supervisor();
    0
}


/// `--port <p>` 解析 (打桩/调试端口覆盖; 缺失/非法 → None = Lang 默认 8111)
fn parse_port_arg(args: &[String]) -> Option<u16> {
    let idx = args.iter().position(|a| a == "--port")?;
    args.get(idx + 1)?.parse::<u16>().ok()
}

