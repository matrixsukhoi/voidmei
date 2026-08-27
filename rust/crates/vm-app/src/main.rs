//! voidmei 主程序组装 (P5 批十四最终组装): 对齐 Java `Launcher → Application.main`
//! 启动序, GPU 兼容段按 D5 消亡 (Java2D sun.java2d.* 属性专属, Rust 无对应物 —
//! iced 走 tiny-skia 纯 CPU 软渲染, D1 决策本身即 GPU 兼容哲学)。
//!
//! Java Application.main (Application.java:534-604) 启动序对位:
//! 1. Logger 级别 (debug 标志)        → `--debug` 参数 (Java: Application.debug 静态)
//! 2. Lang.initLang + 端口/屏幕探测    → `AppShell::new` → `Env::probe`
//! 3. initFont (字体)                 → Env.fonts_dir → win32 线程 (D8: 字体→win32)
//! 4. initSystemTray                  → win32 线程内 (D8 单泵共享)
//! 5. SwingUtilities.invokeLater(EDT): initWebLaf + `new Controller(true)` + checkUpdate
//!    → 主线程组装: rebuild_controller(true) (AppShell::new 内) + iced MainForm。
//!    (checkUpdate 的更新检查未移植 — 网络面 P6 收口, TODO(port))
//!
//! 相位主循环 (D9 后为 web 壳单循环; 原 iced 相 A/B 已合并, 见 desktop_main 注):
//! - 主线程: `shell.pump()` + `ShellForm::pump_once()` (tao 事件 + IPC) +
//!   sleep(可见 10ms / 隐藏 50ms); 设置窗常驻隐藏预热, 每次 show 发布 UI_READY。
//! - 无窗降级 (`run_supervisor_phase`) 与 `--game-mode`/`--mock-smoke` 形态保留。
//!
//! CLI:
//! - `--game-mode`: 对齐 `autoStartGameMode=true` (e2e 用) — 跳过 MainForm,
//!   Controller 自启动 Service, 主线程直接进监督循环。
//! - `--port <p>`: 白盒 e2e 端口覆盖 (rust_e2e.sh 默认 9222)。白盒测试端口约定:
//!   一律 9222 (Java 备用端口 appPortBkp 域, 游戏本地 API 恒占 8111 而 9222
//!   游戏永不监听) — 真机在跑测试也不再被挤掉/误读游戏数据。
//! - `--mock-smoke`: 起 `script/mock_8111.py` s2 场景 (端口 9222) → 游戏模式跑
//!   8 秒 → 断言 Service 收数 + 全部注册 overlay 逐窗 present>0 → 清理退出
//!   (无 MainForm 冒烟; 9222 被占自动跳过 — 项目惯例, 不做假通过)。
//! - `--debug`: Application.debug = true (Logger DEBUG 级)。

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use vm_app::{AppShell, SupervisorOutcome};

use tauri::Emitter;
use vm_core::bus::EventBus;
use vm_core::configuration_service::UiStateEvent;
use vm_core::event::ui_state_events;
use vm_core::logger;

mod form_dispatch;

/// 冒烟默认时长 (任务验收单: 游戏模式跑 8 秒)
const MOCK_SMOKE_RUN_MS: u64 = 8_000;
/// mock server 就绪等待上限
const MOCK_READY_TIMEOUT_MS: u64 = 8_000;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let debug = args.iter().any(|a| a == "--debug");

    // Java Application.main:541-546 Logger 级别 (debugLog||debug → DEBUG, 否则 INFO)
    // TODO(port) (P6 日志族, 审查 A-W5): debugLog=true 时 stdout/stderr 重定向
    // (Application.setDebugLog/setErrLog → ./output.log ./error.log, :550-553)
    // 未移植 — Rust Logger 出文件面后接。
    logger::set_min_level(if debug {
        logger::Level::Debug
    } else {
        logger::Level::Info
    });

    let code = if args.iter().any(|a| a == "--mock-smoke") {
        mock_smoke_main(debug)
    } else if args.iter().any(|a| a == "--game-mode") {
        // 白盒 e2e 端口覆盖: rust_e2e.sh 默认 9222 (见 CLI 头注), 不与真机 8111 冲突
        let port = parse_port_arg(&args);
        game_mode_main(debug, port)
    } else {
        desktop_main(debug)
    };
    std::process::exit(code);
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
/// - 窗口 X 由 vm-webui on_window_event 拦截转 hide (对位 Java DISPOSE→托盘态);
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
    // D8: win32 线程先行 (托盘 + overlay host + 热键泵)。预览模式全开语义 =
    // UI_READY → Preview() → RefreshPreviews 命令由主循环泵触发,
    // 对齐 Java autoStartGameMode=false 默认 (MainForm 先行, 预览窗随后)
    if let Err(e) = shell.spawn_win32_thread() {
        logger::error("App", &format!("win32 线程启动失败: {e}"));
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

    // AppShell 主线程持有 (D8: 含 !Send 配置树恒留主线程 — 原 iced 相 A/B
    // 形态下的 hooks 闭包共享点已随单循环消失, Arc 仅留 dispatcher 注入用)
    // PORT(allow arc_with_non_send_sync): 同 configuration_service.rs 先例 —
    // Arc 复刻 Java this 引用共享, 不为 lint 改 Rc
    #[allow(clippy::arc_with_non_send_sync)]
    let shell = Arc::new(Mutex::new(shell));

    // Tauri 壳: 常驻隐藏, build 即后台预热 (不阻塞; 首显等 is_web_ready);
    // dispatcher = 表单写链真实现 (数据面请求经 MainFormState/UiCommand)
    let mut form = match vm_webui::ShellForm::new(form_dispatch::make_dispatcher(
        &shell,
        Rc::clone(&form_cell),
    )) {
        Ok(f) => Some(f),
        Err(e) => {
            logger::error("App", &format!("Web 设置壳不可用, 降级无窗监督: {e}"));
            None
        }
    };
    // 事件桥: CONFIG_CHANGED → 前端 config-changed (reset/import 后整树刷新);
    // FM_CHANGED → fm-changed (MISSING/CORRUPT toast, 对位 NotificationService);
    // Subscription RAII — 与主循环同生命周期
    let fm_changed_bus = shell.lock().expect("AppShell 锁中毒").fm.fm_changed_bus();
    let _bridge_sub = form.as_ref().map(|f| {
        (
            vm_webui::bridge::bridge_config_changed(f.app_handle(), &ui_bus),
            vm_webui::bridge::bridge_fm_changed(f.app_handle(), &fm_changed_bus),
        )
    });

    // Java Controller(true) 的自启动分支 (autoStartGameMode=true): 不显设置窗
    // (UI_READY 不发布, 游戏模式不被 Preview 翻转)。仅 desktop 形态首迭代判定
    let mut first_iteration = true;
    let mut initial_shown = false;
    // StatusBar 面: 核状态变化 → 前端 controller-state (Init/Preview/Connected/InGame)
    let mut last_state = String::new();
    loop {
        let auto_started = first_iteration
            && shell
                .lock()
                .expect("AppShell 锁中毒")
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
            match shell.lock().expect("AppShell 锁中毒").run_supervisor_phase() {
                SupervisorOutcome::Exit => break,
                SupervisorOutcome::MainFormRequested => logger::info(
                    "App",
                    "托盘请求设置窗 — web 壳不可用, 已重建核继续监督",
                ),
            }
            continue;
        };

        let (exit, form_req, in_game, state_str) = {
            let mut s = shell.lock().expect("AppShell 锁中毒");
            s.pump();
            // 游戏模式运行判定 (收窗面): Connected/InGame = start() 后的形态
            let state = s.shared.state();
            let in_game = matches!(
                state,
                vm_core::controller_state::ControllerState::Connected
                    | vm_core::controller_state::ControllerState::InGame
            );
            (
                s.is_exit_requested(),
                s.take_form_request(),
                in_game,
                format!("{state:?}"),
            )
        };
        // StatusBar: 核状态变化 → 前端 controller-state
        if state_str != last_state {
            last_state = state_str.clone();
            // form 此处为 &mut ShellForm (as_mut 解构); 方法调用自动可变降级
            let _ = form.app_handle().emit("controller-state", state_str);
        }

        if exit {
            break; // EndGame (mCancel IPC, 阶段②接线) / 托盘 Exit
        }

        let visible = form.is_main_visible();
        if form_req && !in_game {
            // 托盘 Activate: 核已由 handle_main_event 重建 (rebuild_controller) —
            // 表单态随之重建 (与核共享新 config 服务, 对位原相 A 重开窗的重新构造),
            // show 幂等 (可能已可见) + UI_READY → 新核进 Preview
            {
                let s = shell.lock().expect("AppShell 锁中毒");
                *form_cell.borrow_mut() = Some(form_dispatch::build_form_state(&s));
            }
            form.show();
            initial_shown = true;
            publish_ui_ready(&ui_bus);
        } else if !visible && !in_game && !initial_shown && form.is_web_ready() {
            // 首显: 预热就绪即开窗 (对位原"启动即开设置窗")
            form.show();
            initial_shown = true;
            publish_ui_ready(&ui_bus);
        } else if visible && in_game {
            // 开始游戏 (托盘 Start / StartGame): 收窗, 对位 confirm 的 setVisible(false)
            form.hide();
        }

        form.pump_once();
        // 泵率: 可见期 10ms (IPC 交互手感 — 滑条/选色实时回执), 隐藏期 50ms
        // (监督节拍, 对位原 iced 50ms Tick)
        let visible = form.is_main_visible();
        std::thread::sleep(Duration::from_millis(if visible { 10 } else { 50 }));
    }
    shell.lock().expect("AppShell 锁中毒").shutdown();
    0
}

/// UI_READY 发布 (Java MainForm 首显 → uiReadyHandler → Preview 的触发面)
fn publish_ui_ready(bus: &Arc<EventBus<UiStateEvent>>) {
    bus.publish(&UiStateEvent {
        event_type: ui_state_events::UI_READY.to_string(),
        source: "MainForm".to_string(),
        data: String::new(),
    });
}

// =====================================================================
// --game-mode: 跳过 MainForm 直接游戏模式 (Java autoStartGameMode=true, e2e)
// =====================================================================

fn game_mode_main(debug: bool, port_override: Option<u16>) -> i32 {
    let shell = match AppShell::new_with_port(debug, true, port_override) {
        Ok(s) => s,
        Err(e) => {
            logger::error("App", &format!("AppShell 构造失败: {e}"));
            return 1;
        }
    };
    // 阻塞监督循环 (内含 win32 自动补启防呆; Exit 托盘命令/通道关闭退出)
    shell.run_supervisor();
    0
}

// =====================================================================
// --mock-smoke: mock s2 场景 → 游戏模式 8 秒 → 断言 → 清理退出
// =====================================================================

fn mock_smoke_main(debug: bool) -> i32 {
    // 白盒测试端口约定 (用户指令): 一律 9222 —— Java 备用端口 (appPortBkp) 域,
    // 游戏本地 API 恒占 8111 而 9222 游戏永不监听, 真机在跑也互不干扰。
    const SMOKE_PORT: u16 = 9222;
    // 端口占用跳过 (项目惯例: 被其他 mock/白盒测试占用时退出码 0 + SKIP)。
    // PORT(探测形态, 真机踩坑): bind 探测对通配监听者假阴性 (127.0.0.1 特定地址
    // 仍可 bind 成功) → 后续 mock 抢绑失败 + 喂数连到别人, 误报 FAIL。connect
    // 探测对任何在场监听者恒真 (service_loop.rs mock e2e 同修)。
    if std::net::TcpStream::connect(("127.0.0.1", SMOKE_PORT)).is_ok() {
        println!("[mock-smoke] SKIP: {SMOKE_PORT} 已有监听者 (其他 mock/白盒测试在跑?)");
        return 0;
    }
    let repo_root = repo_root();
    // 起 mock (s2_preview_live: 正常 p-51d 快照持续供应)
    let mut mock = match std::process::Command::new("python")
        .arg("script/mock_8111.py")
        .args(["serve", "--port", &SMOKE_PORT.to_string(), "--scenario", "s2_preview_live"])
        .current_dir(&repo_root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            println!("[mock-smoke] SKIP: python 不可用 ({e})");
            return 0;
        }
    };
    if !wait_mock_ready(MOCK_READY_TIMEOUT_MS, SMOKE_PORT) {
        println!("[mock-smoke] FAIL: mock server 未在限时就绪");
        let _ = mock.kill();
        let _ = mock.wait();
        return 1;
    }
    println!(
        "[mock-smoke] mock s2_preview_live 就绪 (端口 {SMOKE_PORT}), 游戏模式运行 {}ms",
        MOCK_SMOKE_RUN_MS
    );

    // 游戏模式组装 (autoStartGameMode=true 注入; 探测面保持真实 — mock 就在本机;
    // 端口走 9222 覆盖, 见 SMOKE_PORT 注)
    let mut shell = match AppShell::new_with_port(debug, true, Some(SMOKE_PORT)) {
        Ok(s) => s,
        Err(e) => {
            println!("[mock-smoke] FAIL: AppShell 构造失败: {e}");
            stop_mock(&mut mock, SMOKE_PORT);
            return 1;
        }
    };
    // win32 线程 (overlay host + 托盘); 失败如实报错 (帧断言必依赖它)
    if let Err(e) = shell.spawn_win32_thread() {
        println!("[mock-smoke] FAIL: win32 线程启动失败: {e}");
        drop(shell);
        stop_mock(&mut mock, SMOKE_PORT);
        return 1;
    }

    // 8 秒 pump 循环 (相 B 监督循环的无窗限时等价): 事件消费 + drive_from_live
    let deadline = Instant::now() + Duration::from_millis(MOCK_SMOKE_RUN_MS);
    while Instant::now() < deadline {
        shell.pump();
        std::thread::sleep(Duration::from_millis(50));
    }

    // 断言 1: Service 收数 (s2 的 /state + /indicators 双 flag 真 + playerLive)
    let mut service_ok = false;
    if let Some(data) = shell.shared.live.read().expect("live 锁中毒").clone() {
        let d = data.read().unwrap_or_else(|e| e.into_inner());
        service_ok = d.s_state.as_ref().is_some_and(|s| s.flag)
            && d.s_indic.as_ref().is_some_and(|i| i.flag)
            && d.player_live;
    }
    // 断言 2: overlay present 帧数 > 0 (win32 渲染节拍计数, 见 ControllerShared 注)
    let frames = shell.shared.render_frames.load(std::sync::atomic::Ordering::SeqCst);
    // 断言 3: 全部注册 overlay 逐窗 present>0 (QA 冒烟判据; drop 前取走快照)
    let overlay_counts = shell
        .shared
        .overlay_present
        .lock()
        .expect("overlay_present 锁中毒")
        .clone();

    // 清理: 先收应用 (Drop = 五步销毁 + win32 join + 防抖 + 热键), 再停 mock
    drop(shell);
    stop_mock(&mut mock, SMOKE_PORT);

    if !service_ok {
        return fail(format!("Service 未收到有效数据 (frames={frames})"));
    }
    if frames == 0 {
        return fail("overlay present 帧数为 0 (窗口未开/渲染未跑)".to_string());
    }
    // 游戏模式注册全集 (register_game_mode_overlays 的 7 键; 缺键 = 注册失败)
    const GAME_MODE_OVERLAYS: [&str; 7] = [
        "enableEngineControl",
        "engineInfoSwitch",
        "crosshairSwitch",
        "enablegearAndFlaps",
        "enableAxis",
        "enableAttitudeIndicator",
        "flightInfoSwitch",
    ];
    let mut missing = Vec::new();
    let mut zero = Vec::new();
    for id in GAME_MODE_OVERLAYS {
        match overlay_counts.get(id) {
            None => missing.push(id),
            Some(0) => zero.push(id),
            Some(_) => {}
        }
    }
    if !missing.is_empty() || !zero.is_empty() {
        return fail(format!(
            "逐 overlay present 断言不过 (注册缺失: {missing:?}; present=0: {zero:?}; 全量计数 {overlay_counts:?})"
        ));
    }
    println!("[mock-smoke] PASS: Service 收数 + present 帧数 = {frames} (逐 overlay: {overlay_counts:?})");
    0
}

fn fail(msg: String) -> i32 {
    println!("[mock-smoke] FAIL: {msg}");
    1
}

/// `--port <p>` 解析 (白盒 e2e 端口覆盖; 缺失/非法 → None = Lang 默认 8111)
fn parse_port_arg(args: &[String]) -> Option<u16> {
    let idx = args.iter().position(|a| a == "--port")?;
    args.get(idx + 1)?.parse::<u16>().ok()
}

/// 停 mock: 优雅 /_mock/shutdown → 限期等待 → 兜底 kill (对位 e2e_fm.sh 收尾)。
/// 限期 wait (审查 B-W3): mock 进程不响应 shutdown 时裸 wait() 会永久挂起冒烟
/// 而非快速失败 — 3s 内 try_wait 轮询, 超时强杀后再收尸。
fn stop_mock(mock: &mut std::process::Child, port: u16) {
    let _ = http_get_raw(port, "/_mock/shutdown");
    let deadline = Instant::now() + Duration::from_millis(3_000);
    while Instant::now() < deadline {
        match mock.try_wait() {
            Ok(Some(_)) => return, // 已优雅退出
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => break,       // wait 系统调用异常, 直接走强杀
        }
    }
    let _ = mock.kill();
    let _ = mock.wait();
}

/// 阻塞等待 mock 的 /_mock/state 可连 (TCP 层就绪即可)
fn wait_mock_ready(timeout_ms: u64, port: u16) -> bool {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    while Instant::now() < deadline {
        if http_get_raw(port, "/_mock/state").is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// 极简 HTTP GET (只判可达/有响应; 不进 vm-core http 模块 — 冒烟控制通道专用)
fn http_get_raw(port: u16, path: &str) -> Option<String> {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_millis(2_000)))
        .ok()?;
    let req = format!("GET {path} HTTP/1.0\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).ok()?;
    let mut buf = String::new();
    stream.read_to_string(&mut buf).ok()?;
    Some(buf)
}

/// 仓库根 (rust/crates/vm-app → 上溯三级; 与 app_shell::locate_template_cfg 同源)
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
