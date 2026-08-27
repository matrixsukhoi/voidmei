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
//! 相位主循环 (D8 线程拓扑下的 MainForm 生命周期, 对位 Java EDT 常驻):
//! - 相 A (窗口期): iced MainForm 消息循环阻塞主线程, 50ms Tick 泵驱动
//!   `AppShell::pump` (vm-ui subscription — 见 vm-ui lib.rs 头注的执行器备案)。
//!   出口: 开始游戏 (confirm → iced::exit) / 结束游戏 (mCancel → exit) /
//!   窗口 X (Java setDefaultCloseOperation(3)=DISPOSE, 应用继续) / 托盘重建。
//! - 相 B (监督期): `run_supervisor_phase` (Service 驱动状态机 + 托盘),
//!   出口: 退出请求 / 托盘 Activate 请求弹设置窗 (回相 A)。
//!
//! CLI:
//! - `--game-mode`: 对齐 `autoStartGameMode=true` (e2e 用) — 跳过 MainForm,
//!   Controller 自启动 Service, 主线程直接进监督循环。
//! - `--mock-smoke`: 起 `script/mock_8111.py` s2 场景 → 游戏模式跑 8 秒 →
//!   断言 Service 收数 + 全部注册 overlay 逐窗 present>0 → 清理退出 (无 MainForm
//!   冒烟; 8111 被占自动跳过 — 项目惯例, 不做假通过)。
//! - `--debug`: Application.debug = true (Logger DEBUG 级)。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use vm_app::{AppShell, SupervisorOutcome, UiCommand};

use vm_core::bus::EventBus;
use vm_core::config_manager;
use vm_core::configuration_service::{ConfigurationService, UiStateEvent};
use vm_core::event::ui_state_events;
use vm_core::logger;

use vm_ui::main_form::MainFormState;
use vm_ui::MainFormHooks;

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
        game_mode_main(debug)
    } else {
        desktop_main(debug)
    };
    std::process::exit(code);
}

// =====================================================================
// 默认路径: MainForm 相位主循环 (Java autoStartGameMode=false 默认)
// =====================================================================

fn desktop_main(debug: bool) -> i32 {
    let mut shell = match AppShell::new(debug, false) {
        Ok(s) => s,
        Err(e) => {
            logger::error("App", &format!("AppShell 构造失败: {e}"));
            return 1;
        }
    };
    // D8: win32 线程先行 (托盘 + overlay host + 热键泵)。预览模式全开语义 =
    // UI_READY → Preview() → RefreshPreviews 命令由相 A 的 Tick 泵触发,
    // 对齐 Java autoStartGameMode=false 默认 (MainForm 先行, 预览窗随后)
    if let Err(e) = shell.spawn_win32_thread() {
        logger::error("App", &format!("win32 线程启动失败: {e}"));
        return 1;
    }
    // start/stop 的设置窗释放闭包: iced 窗口的生灭由相位循环统一管理,
    // Controller 侧释放 = 记日志 (Java release 链的真窗面已由相位切换承担)
    shell.release_main_form = Box::new(|| {
        logger::info("AppShell", "释放设置窗 (相位循环已收口 iced 窗口)");
    });

    // AppShell 主线程持有; hooks 闭包经 Arc<Mutex> 共享 (AppShell 含 !Send 的
    // 配置树, iced State 仅要求 'static — 恒留主线程, 跨线程不发生)
    // PORT(allow arc_with_non_send_sync): 同 configuration_service.rs 先例 —
    // Arc 复刻 Java this 引用共享, 不为 lint 改 Rc (相位循环生命周期等价)
    #[allow(clippy::arc_with_non_send_sync)]
    let shell = Arc::new(Mutex::new(shell));

    // Java Controller(true) 的自启动分支 (autoStartGameMode=true): 不构造 MainForm,
    // 直接游戏模式 — 相 A 整体跳过 (UI_READY 只在 MainForm 首显发布, 游戏模式不该
    // 被 Preview 翻转)。仅首迭代判定; 后续相 A 只能经托盘 Activate 进入 (重建核)
    let mut first_iteration = true;
    loop {
        let auto_started = first_iteration
            && shell
                .lock()
                .expect("AppShell 锁中毒")
                .controller
                .as_ref()
                .is_some_and(|c| c.service.is_some());
        first_iteration = false;

        if !auto_started {
            // ---- 相 A: iced MainForm (阻塞至关窗/退出) ----
            let form = build_form_state(&shell);
            let hooks = make_hooks(&shell);
            match vm_ui::run_shell_form(form, hooks) {
                Ok(()) => {}
                Err(e) => {
                    // 窗口不可开 (无显示/复跑失败): 降级监督模式 (Java 无窗 + 托盘继续)
                    logger::error("App", &format!("MainForm 窗口异常退出, 转监督模式: {e}"));
                }
            }
        }
        let mut s = shell.lock().expect("AppShell 锁中毒");
        // 不变量 (审查 B-W5): 本 MutexGuard 跨整个相 B 的阻塞期持有。当前无竞争者
        // — iced 已退 (hooks 随 iced State drop, 不再持 shell 锁), win32/Service
        // 线程只经 channel 通信、从不 lock shell — 故安全。未来任何来自 win32/
        // Service 线程的 shell 锁获取都会与整个监督相位串行化, 接线时须先破此形态。
        if s.is_exit_requested() {
            break; // EndGame (mCancel) / 托盘 Exit
        }
        if s.take_form_request() {
            continue; // 相 A 期托盘 Activate → 核已重建, 直接重开窗
        }
        // ---- 相 B: 监督循环 (开始游戏后的常驻态 / 窗口 X 后的托盘态) ----
        match s.run_supervisor_phase() {
            SupervisorOutcome::Exit => break,
            SupervisorOutcome::MainFormRequested => continue, // 托盘弹设置 → 回相 A
        }
    }
    shell.lock().expect("AppShell 锁中毒").shutdown();
    0
}

/// 相 A 的表单状态: 与当前核共享同一 ConfigurationService (Arc<ServiceInner> 克隆
/// = Java tc.configService 单对象语义, clone-split 备案见 main_form.rs 头注)
fn build_form_state(shell: &Arc<Mutex<AppShell>>) -> MainFormState {
    let s = shell.lock().expect("AppShell 锁中毒");
    let config = s
        .controller
        .as_ref()
        .map(|c| c.config.clone())
        .unwrap_or_else(|| ConfigurationService::new(Some(Arc::clone(&s.ui_bus))));
    MainFormState::new(
        config,
        Arc::clone(&s.ui_bus),
        Some(config_manager::get_user_config_path().to_string()),
    )
}

/// shell 回调 (全部在 iced 主循环内调用):
/// - on_ready: UI_READY 发布 (Java MainForm 首显 → uiReadyHandler → Preview)
/// - on_start_game/on_end_game: MainForm.confirm/mCancel 的 tc 侧序列
/// - on_tick: 50ms 泵; true = 托盘重建/退出请求 → iced::exit 切相位
fn make_hooks(shell: &Arc<Mutex<AppShell>>) -> MainFormHooks {
    let ready_bus = {
        let s = shell.lock().expect("AppShell 锁中毒");
        Arc::clone(&s.ui_bus)
    };
    let on_ready = {
        let bus: Arc<EventBus<UiStateEvent>> = Arc::clone(&ready_bus);
        Box::new(move || {
            bus.publish(&UiStateEvent {
                event_type: ui_state_events::UI_READY.to_string(),
                source: "MainForm".to_string(),
                data: String::new(),
            });
        })
    };
    let on_start_game = {
        let s = Arc::clone(shell);
        Box::new(move || {
            if let Ok(mut s) = s.lock() {
                s.dispatch(UiCommand::StartGame);
            }
        })
    };
    let on_end_game = {
        let s = Arc::clone(shell);
        Box::new(move || {
            if let Ok(mut s) = s.lock() {
                s.dispatch(UiCommand::EndGame);
            }
        })
    };
    let on_tick = {
        let s = Arc::clone(shell);
        Box::new(move || {
            let mut close = false;
            if let Ok(mut s) = s.lock() {
                s.pump();
                if s.is_exit_requested() || s.take_form_request() {
                    close = true;
                }
            }
            close
        })
    };
    MainFormHooks {
        on_ready,
        on_start_game,
        on_end_game,
        on_tick,
    }
}

// =====================================================================
// --game-mode: 跳过 MainForm 直接游戏模式 (Java autoStartGameMode=true, e2e)
// =====================================================================

fn game_mode_main(debug: bool) -> i32 {
    let shell = match AppShell::new(debug, true) {
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
    // 端口占用跳过 (项目惯例: 8111 被游戏/真机/mock 占用时退出码 0 + SKIP)
    if std::net::TcpListener::bind(("127.0.0.1", 8111)).is_err() {
        println!("[mock-smoke] SKIP: 8111 已被占用 (游戏/mock 在跑?)");
        return 0;
    }
    let repo_root = repo_root();
    // 起 mock (s2_preview_live: 正常 p-51d 快照持续供应)
    let mut mock = match std::process::Command::new("python")
        .arg("script/mock_8111.py")
        .args(["serve", "--port", "8111", "--scenario", "s2_preview_live"])
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
    if !wait_mock_ready(MOCK_READY_TIMEOUT_MS) {
        println!("[mock-smoke] FAIL: mock server 未在限时就绪");
        let _ = mock.kill();
        let _ = mock.wait();
        return 1;
    }
    println!(
        "[mock-smoke] mock s2_preview_live 就绪, 游戏模式运行 {}ms",
        MOCK_SMOKE_RUN_MS
    );

    // 游戏模式组装 (autoStartGameMode=true 注入; 探测面保持真实 — mock 就在本机)
    let mut shell = match AppShell::new(debug, true) {
        Ok(s) => s,
        Err(e) => {
            println!("[mock-smoke] FAIL: AppShell 构造失败: {e}");
            stop_mock(&mut mock);
            return 1;
        }
    };
    // win32 线程 (overlay host + 托盘); 失败如实报错 (帧断言必依赖它)
    if let Err(e) = shell.spawn_win32_thread() {
        println!("[mock-smoke] FAIL: win32 线程启动失败: {e}");
        drop(shell);
        stop_mock(&mut mock);
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
    stop_mock(&mut mock);

    if !service_ok {
        return fail(format!("Service 未收到有效数据 (frames={frames})"));
    }
    if frames == 0 {
        return fail("overlay present 帧数为 0 (窗口未开/渲染未跑)".to_string());
    }
    // 游戏模式注册全集 (register_game_mode_overlays 的 6 键; 缺键 = 注册失败)
    const GAME_MODE_OVERLAYS: [&str; 6] = [
        "enableEngineControl",
        "engineInfoSwitch",
        "crosshairSwitch",
        "enablegearAndFlaps",
        "enableAxis",
        "enableAttitudeIndicator",
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

/// 停 mock: 优雅 /_mock/shutdown → 限期等待 → 兜底 kill (对位 e2e_fm.sh 收尾)。
/// 限期 wait (审查 B-W3): mock 进程不响应 shutdown 时裸 wait() 会永久挂起冒烟
/// 而非快速失败 — 3s 内 try_wait 轮询, 超时强杀后再收尸。
fn stop_mock(mock: &mut std::process::Child) {
    let _ = http_get_raw(8111, "/_mock/shutdown");
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
fn wait_mock_ready(timeout_ms: u64) -> bool {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    while Instant::now() < deadline {
        if http_get_raw(8111, "/_mock/state").is_some() {
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
