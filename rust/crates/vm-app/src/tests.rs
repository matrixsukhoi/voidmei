use super::*;
use std::sync::atomic::AtomicUsize;
use vm_core::fm::FMHandle;

static CFG_N: AtomicUsize = AtomicUsize::new(0);

/// tmp 配置文件 (vm-core configuration_service 测试同款惯例)
fn tmp_cfg(content: &str) -> String {
    let n = CFG_N.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir()
        .join(format!("vm_app_shell_{}_{n}.cfg", std::process::id()))
        .to_str()
        .unwrap()
        .to_string();
    std::fs::write(&p, content).unwrap();
    p
}

/// 测试配置: crosshairSwitch 开 / enableEngineControl 关 / 无自启动
fn test_cfg() -> String {
    fixture_cfg(
        "(panel \"T\" :visible true\n\
             \x20 (item \"hud\" :type switch :target \"crosshairSwitch\" :value true)\n\
             \x20 (item \"engine\" :type switch :target \"enableEngineControl\" :value false)\n\
             \x20 (item \"auto\" :type switch :target \"autoStartGameMode\" :value false))\n\
            ",
    )
}

/// fixture 内容直装 (autoStartGameMode 变体等)
fn fixture_cfg(content: &str) -> String {
    tmp_cfg(content)
}

/// 自启动变体配置 (对位 --game-mode / Java autoStartGameMode=true)
fn auto_start_cfg() -> String {
    fixture_cfg(
        "(panel \"T\" :visible true\n\
             \x20 (item \"auto\" :type switch :target \"autoStartGameMode\" :value true))\n\
            ",
    )
}

/// AppShell 测试装配: tmp cfg (无 init_config 写盘副作用) + 30ms 短防抖 +
/// **网络隔离** (Service 指向 9 号死端口 — 连接立即拒绝; FM-Detect 探测关闭
/// — 8111 可能被 mock/游戏占用, 项目惯例端口占用即隔离, 不做假通过);
/// 不起 win32 线程 — ui_cmd 接收端留在 shell 内供测试观察。
fn fixture() -> AppShell {
    fixture_with_debounce(30)
}

fn fixture_with_debounce(ms: u64) -> AppShell {
    fixture_full(ms, test_cfg())
}

/// 全参 fixture (自定义 cfg 内容; 见 fixture_with_debounce 注)
fn fixture_full(ms: u64, cfg: String) -> AppShell {
    let ui_bus = Arc::new(EventBus::new());
    let config = ConfigurationService::new(Some(Arc::clone(&ui_bus)));
    config.load_layout(&cfg);
    let (hotkey, hotkey_rx) = HotkeyManager::with_channel();
    let mut env = Env::probe(&Lang::init_lang(), false);
    env.app_port = 9; // discard 端口: 无服务监听, connect 立即 RST
    env.app_port_bkp = 9;
    // 字体目录钉在仓库根 (cargo 测试 CWD=crate 根, CWD 相对探测不稳;
    // win32 线程注册面的 spec 工厂需要真实字体文件)
    env.fonts_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts");
    let mut shell = AppShell::with_parts(ShellParts {
        env,
        config,
        ui_bus,
        flight_bus: Arc::new(FlightDataBus::new()),
        fm: Arc::new(FMManager::new(Arc::new(EventBus::new()))),
        hotkey,
        hotkey_rx,
        debounce_delay: Duration::from_millis(ms),
    });
    // 网络探测关闭 (见 fixture 注): FM-Detect/Preview 只走 selectedFM0 兜底
    shell.probe_network_for_test(false);
    shell.rebuild_controller(true);
    shell
}

/// 泵监督事件 (订阅转发 → 主线程处理链), 返回是否处理过事件
fn pump_events(shell: &mut AppShell) -> bool {
    let mut handled = false;
    while let Ok(ev) = shell.main_event_rx.try_recv() {
        shell.handle_main_event(ev);
        handled = true;
    }
    handled
}

fn publish_ui_event(bus: &EventBus<UiStateEvent>, event_type: &str, data: &str) {
    bus.publish(&UiStateEvent {
        event_type: event_type.to_string(),
        source: "MainForm".to_string(),
        data: data.to_string(),
    });
}

// ------------------------------------------------------------------
// 状态机: preview → game → (drive) → stop
// ------------------------------------------------------------------

/// UI_READY → Preview() → state=Preview (Java uiReadyHandler → Preview)
#[test]
fn ui_ready_enters_preview_state() {
    let mut shell = fixture();
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Init
    );
    publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
    assert!(pump_events(&mut shell), "UI_READY 应经转发到达监督循环");
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Preview
    );
}

/// confirm 链: Preview → (endPreview) Init → start (INIT 守卫过) → Service 起
#[test]
fn confirm_start_game_runs_service() {
    let mut shell = fixture();
    publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
    pump_events(&mut shell);
    shell.dispatch(UiCommand::StartGame);
    let c = shell.controller.as_ref().unwrap();
    // end_preview 置 INIT; start 不改 State (Java 同 — 等 Service 轮询驱动)
    assert_eq!(c.state(), ControllerState::Init);
    assert!(c.service.is_some(), "start() 应建 Service 线程");
    assert!(
        shell.shared.live.read().unwrap().is_some(),
        "live 句柄应登记"
    );
}

/// Service 轮询驱动: flags 真 + playerLive → changeS2/changeS3 (InGame→Preview)
/// + FM identify 提交 (换机轻量 swap 的会话记忆同时落位)
#[test]
fn drive_from_live_opens_overlays() {
    let mut shell = fixture();
    {
        let c = shell.controller.as_mut().unwrap();
        c.init_status_bar();
        c.change_s2();
        assert_eq!(c.state(), ControllerState::InGame);
    }
    // 手工装填 live 数据 (真机由 Service 线程写; ServiceData 公开字段面)
    let data = Arc::new(std::sync::RwLock::new(live_service_data("test-plane")));
    *shell.shared.live.write().unwrap() = Some(data);
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Preview,
        "changeS3 应进 PREVIEW"
    );
    // FM 识别目标已提交 (identify 异步加载, 目标名同步可见)
    assert_eq!(
        shell.fm.current_target_name().as_deref(),
        Some("test-plane")
    );
    // 会话首机只记名 (sessionAircraftType)
    assert_eq!(
        shell.shared.flags.lock().unwrap().session_aircraft_type.as_deref(),
        Some("test-plane")
    );
}

/// flags 丢失 → S4toS1: Preview → Init + FM 目标清除 (会话结束语义)。
/// 真实形态 (审查 B 掩蔽指正): 串非空 + valid=false → State/Indicators 的
/// update 照常执行后 flag=false — 对象保留, **不是** ServiceData 回 default
/// (真实断连时快照从不整体复位)
#[test]
fn drive_from_live_exit_resets_session() {
    let mut shell = fixture();
    {
        let c = shell.controller.as_mut().unwrap();
        c.init_status_bar();
        c.change_s2();
    }
    let data = Arc::new(std::sync::RwLock::new(live_service_data("p1")));
    *shell.shared.live.write().unwrap() = Some(Arc::clone(&data));
    shell.controller.as_mut().unwrap().drive_from_live(); // 进 Preview + identify(p1)
    // flags 丢失 (Java Service.java:1780 路径): 仅 flag 翻假, 其余保留
    {
        let mut d = data.write().unwrap();
        d.s_state.as_mut().unwrap().flag = false;
        d.s_indic.as_mut().unwrap().flag = false;
    }
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Init
    );
    assert_eq!(
        shell.fm.current_target_name(),
        None,
        "会话结束清识别目标"
    );
    assert!(
        shell.shared.flags.lock().unwrap().session_aircraft_type.is_none(),
        "会话机型记忆清除"
    );
}

/// B1 补偿核心: 串空路径 (游戏进程退出/8111 消失 → HTTP 失败串复位空 →
/// update 不执行 → flags/playerLive 保留**陈旧真值**) — 事件流静默超时
/// 触发 S4toS1; 且退出后状态稳定停 Init (串空轮不跑 initStatusBar/
/// changeS2, 对位 Java 串空分支)
#[test]
fn drive_from_live_silent_stream_exits_on_game_exit() {
    let mut shell = fixture();
    {
        let c = shell.controller.as_mut().unwrap();
        c.init_status_bar();
        c.change_s2();
    }
    let data = Arc::new(std::sync::RwLock::new(live_service_data("p1")));
    *shell.shared.live.write().unwrap() = Some(Arc::clone(&data));
    // 首轮事件新鲜 (100ms 前): 正常进 Preview + identify
    shell
        .shared
        .last_flight_event_ms
        .store(current_time_millis() - 100, Ordering::SeqCst);
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Preview
    );
    // 游戏退出: ServiceData 一字不动 (flags/playerLive 陈旧真), 仅事件停发
    shell.shared.last_flight_event_ms.store(
        current_time_millis() - FLIGHT_SILENT_EXIT_MS - 100,
        Ordering::SeqCst,
    );
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Init,
        "事件静默超时应触发 S4toS1 (串空路径补偿, B1)"
    );
    assert_eq!(shell.fm.current_target_name(), None, "会话结束清识别目标");
    assert!(shell
        .shared
        .flags
        .lock()
        .unwrap()
        .session_aircraft_type
        .is_none());
    // 再一轮 (仍静默): 稳定停 Init, 不回弹 Connected/InGame
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Init
    );
}

/// 静默不误杀: playerLive=false (着陆停机/坠机等待, Java Service.java:746-754
/// 的 sleep 等待路径) + 事件停发 — 补偿判定含 playerLive 真前置, 不触发
#[test]
fn drive_from_live_silent_player_not_live_waits() {
    let mut shell = fixture();
    {
        let c = shell.controller.as_mut().unwrap();
        c.init_status_bar();
        c.change_s2();
    }
    let data = Arc::new(std::sync::RwLock::new(live_service_data("p1")));
    *shell.shared.live.write().unwrap() = Some(Arc::clone(&data));
    shell
        .shared
        .last_flight_event_ms
        .store(current_time_millis() - 100, Ordering::SeqCst);
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Preview
    );
    // 坠机/停机: playerLive 翻假 + 事件停发超阈值
    data.write().unwrap().player_live = false;
    shell.shared.last_flight_event_ms.store(
        current_time_millis() - FLIGHT_SILENT_EXIT_MS - 100,
        Ordering::SeqCst,
    );
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Preview,
        "playerLive=false 的静默是等待态, 不应退出会话"
    );
}

/// 换机轻量 swap: 同机幂等; 换机 FM 目标切换且不重启 Controller
#[test]
fn aircraft_change_lightweight_swap() {
    let mut shell = fixture();
    {
        let c = shell.controller.as_mut().unwrap();
        c.init_status_bar();
        c.change_s2();
    }
    let data = Arc::new(std::sync::RwLock::new(live_service_data("a1")));
    *shell.shared.live.write().unwrap() = Some(Arc::clone(&data));
    let c = shell.controller.as_mut().unwrap();
    c.drive_from_live();
    assert_eq!(shell.fm.current_target_name().as_deref(), Some("a1"));
    // 同机: 幂等 (目标不变)
    c.on_aircraft_changed(Some("a1"));
    assert_eq!(shell.fm.current_target_name().as_deref(), Some("a1"));
    // 换机: FM 目标切换 (identify), 不重启 Controller (state 保持 Preview)
    *data.write().unwrap() = live_service_data("b2");
    c.drive_from_live();
    assert_eq!(shell.fm.current_target_name().as_deref(), Some("b2"));
    assert_eq!(c.state(), ControllerState::Preview, "换机不重启生命周期");
}

/// FlightLog 开/存生命周期 (Controller.java:366-376 openpad / 402-411 closepad):
/// enableLogging=true → 进 Preview (openpad_rest) 槽置 Some + records/ 建
/// 机型命名 CSV; s4to_s1 → closepad 保存 (槽清 None, 表头落盘)。
/// tick 行数语义由 vm-data 的 flight_log_tick 测试锁定 (此处 Service 真线程
/// HTTP 失败, 行数时序不定, 只断言 ≥ 表头)。
#[test]
fn flight_log_open_close_lifecycle() {
    // CWD 沙箱: FlightLog 的 records/ 是相对 CWD 的硬编码 (与 Java 一致);
    // 串行化 + 用完恢复 (vm-core/vm-data 同款; 跨 crate 测试进程天然互斥)
    static FL_CWD_LOCK: Mutex<()> = Mutex::new(());
    let _guard = FL_CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let root = std::env::temp_dir().join(format!("vm_app_fl_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("records")).unwrap();
    let old = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // enableLogging=true 配置 (fixture 其余键同 test_cfg)
        let cfg = fixture_cfg(
            "(panel \"T\" :visible true\n\
                 \x20 (item \"log\" :type switch :target \"enableLogging\" :value true)\n\
                 \x20 (item \"auto\" :type switch :target \"autoStartGameMode\" :value false))\n\
                ",
        );
        let mut shell = fixture_full(30, cfg);
        // 进游戏: UI_READY → StartGame (spawn Service) → live → Preview
        publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
        pump_events(&mut shell);
        shell.dispatch(UiCommand::StartGame);
        // 生产语义: shared.live 即 handle.data 的同一 Arc (start() 内 clone);
        // fixture 手塞场景下需同步塞真 Service 侧 (open_flight_log 的机型名
        // 与 ServiceAnalyzerSource 都读 handle.data)
        let svc_data =
            Arc::clone(&shell.controller.as_ref().unwrap().service.as_ref().unwrap().data);
        *shell.shared.live.write().unwrap() = Some(Arc::clone(&svc_data));
        *svc_data.write().unwrap() = live_service_data("s1");
        shell.pump();
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Preview
        );

        // openpad_rest → open_flight_log: 槽 Some, 主 CSV 已建 (表头 flush)
        let file_name = {
            let c = shell.controller.as_ref().unwrap();
            let slot = c.flight_log.lock().expect("flight_log 槽锁中毒");
            let log = slot.as_ref().expect("openpad 应建 FlightLog").lock().unwrap();
            log.file_name.clone()
        };
        assert!(
            file_name.starts_with("records/S1_"),
            "机型大写命名: {file_name}"
        );
        let content = std::fs::read_to_string(&file_name).unwrap();
        assert!(
            content.lines().count() >= 1 && content.starts_with("时间/s,"),
            "init 写表头: {content:?}"
        );

        // 换机: 关旧开新 (onAircraftChanged:320-333) — 新文件名随新机型
        *svc_data.write().unwrap() = live_service_data("b2");
        let c = shell.controller.as_mut().unwrap();
        c.on_aircraft_changed(Some("b2"));
        {
            let slot = c.flight_log.lock().expect("flight_log 槽锁中毒");
            let log = slot.as_ref().expect("换机应开新 FlightLog").lock().unwrap();
            assert!(
                log.file_name.starts_with("records/B2_"),
                "换机开新: {}",
                log.file_name
            );
        }

        // 退出 (S4toS1 → closepad): 保存 + 槽清 None (Log = null)
        c.s4to_s1();
        assert_eq!(c.state(), ControllerState::Init);
        assert!(
            c.flight_log.lock().unwrap().is_none(),
            "closepad 后槽应清空 (Log = null)"
        );
        // 旧/新两份主 CSV 均留存 (fA==null 的 close NPE 路径由 catch_unwind 吞,
        // 不中断 closepad — Java 由 Service 顶层 catch 兜住的净效果)。
        // 主 CSV 与分析 CSV 以 _climb/_roll/_ny 后缀区分
        let is_main = |n: &str| {
            n.ends_with(".csv")
                && !n.ends_with("_climb.csv")
                && !n.ends_with("_roll.csv")
                && !n.ends_with("_ny.csv")
        };
        let mut s1_seen = false;
        let mut b2_seen = false;
        for e in std::fs::read_dir("records").unwrap() {
            let name = e.unwrap().file_name().to_string_lossy().to_string();
            s1_seen |= name.starts_with("S1_") && is_main(&name);
            b2_seen |= name.starts_with("B2_") && is_main(&name);
        }
        assert!(s1_seen, "s1 主 CSV 留存");
        assert!(b2_seen, "b2 主 CSV 留存");

        // 停 Service 线程 (恢复 CWD / 删沙箱前必须 — 免线程写已删目录)
        shell.controller.as_mut().unwrap().stop(&mut || {});
    }));
    std::env::set_current_dir(old).unwrap();
    let _ = std::fs::remove_dir_all(&root);
    drop(_guard); // 先放锁再重抛
    if let Err(e) = r {
        std::panic::resume_unwind(e);
    }
}

// ------------------------------------------------------------------
// stop 五步序 (LIFETIMES §4.2)
// ------------------------------------------------------------------

/// 五步销毁: ①gen++/CloseAll → ②退订 → ③释放设置窗 → ④停 Service → ⑤存配置;
/// 顺序断言: 步③执行时步①的 CloseAllOverlays 命令已入队 (①先于③);
/// 步④后 service 句柄已收 + live 清空
#[test]
fn stop_five_step_order() {
    let mut shell = fixture();
    // 进 Preview + 起 Service (五步全路径)
    publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
    pump_events(&mut shell);
    shell.dispatch(UiCommand::StartGame);
    let data = Arc::new(std::sync::RwLock::new(live_service_data("s1")));
    *shell.shared.live.write().unwrap() = Some(data);
    shell.pump(); // drive: Service live 数据 → InGame → Preview
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Preview
    );

    let gen_before = shell.shared.preview_generation.load(Ordering::SeqCst);
    let ui_subs_before = shell.ui_bus.subscriber_count();
    let fm_subs_before = shell.fm.fm_changed_bus().subscriber_count();
    assert!(ui_subs_before >= 2, "Controller 应持 config/uiReady 两订阅");
    assert!(fm_subs_before >= 1, "Controller 应持 FM_CHANGED 订阅");

    // 步③注入: 游戏模式下 M 已被 start() 释放 (Java start:619-623 dispose M),
    // stop 的步③ (M!=null 判定) 不触发 — 断言不调用 (步③的触发面见
    // stop_preview_releases_main_form_after_overlays_closed)
    let step3_seen = Arc::new(Mutex::new(false));
    let seen = Arc::clone(&step3_seen);
    let mut release = move || {
        *seen.lock().unwrap() = true;
    };
    shell.controller.as_mut().unwrap().stop(&mut release);

    // ①: 世代号 ++ (作废在途回调) — Preview 分支
    assert_eq!(
        shell.shared.preview_generation.load(Ordering::SeqCst),
        gen_before + 1
    );
    // ①: CloseAllOverlays 命令已入队 (容忍 openpad 延迟线程 OpenAll 抢先)
    let ui_rx = shell.ui_cmd_rx.take().unwrap();
    let mut got_close = false;
    for _ in 0..3 {
        match ui_rx.try_recv() {
            Ok(UiCommand::CloseAllOverlays) => {
                got_close = true;
                break;
            }
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    assert!(got_close, "步①应发 CloseAllOverlays (closepad 路径)");
    // ②: 订阅全部退订
    assert_eq!(shell.ui_bus.subscriber_count(), ui_subs_before - 2);
    assert_eq!(
        shell.fm.fm_changed_bus().subscriber_count(),
        fm_subs_before - 1
    );
    // ③: 游戏模式 M 已在 start() 释放 — 步③跳过 (Java M=null 判定)
    assert!(
        !*step3_seen.lock().unwrap(),
        "游戏模式 stop 不应再释放 MainForm"
    );
    // ④: Service 句柄已收 + live 清空
    assert!(shell.controller.as_ref().unwrap().service.is_none());
    assert!(shell.shared.live.read().unwrap().is_none());
    // ⑤: save_config 空实现 (全量在 ui_layout.cfg), 无可断言面 — 顺序由代码序保证
}

/// 步①→③ 顺序 (预览模式, M 存活): 步③执行时步①的 CloseAllOverlays
/// 已入队 (Java: "必须在 dispose MainForm 之前执行" 注释的顺序语义)
#[test]
fn stop_preview_releases_main_form_after_overlays_closed() {
    let mut shell = fixture();
    // 进 Preview (M 存活, 无 Service — 纯预览路径)
    publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
    pump_events(&mut shell);
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Preview
    );

    let ui_rx = shell.ui_cmd_rx.take().unwrap();
    let order_ok = Arc::new(Mutex::new(false));
    let ok = Arc::clone(&order_ok);
    let mut release = move || {
        // 步③执行点: 步①的 CloseAll 应已先入队 (容忍 Preview-Refresh 线程的
        // RefreshPreviews 抢先)
        let mut got_close = false;
        for _ in 0..3 {
            match ui_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(UiCommand::CloseAllOverlays) => {
                    got_close = true;
                    break;
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        *ok.lock().unwrap() = got_close;
    };
    shell.controller.as_mut().unwrap().stop(&mut release);
    assert!(
        *order_ok.lock().unwrap(),
        "步③执行时步①的 CloseAllOverlays 应已入队 (overlay 先于 MainForm)"
    );
}

// ------------------------------------------------------------------
// 防过期 generation (previewGeneration)
// ------------------------------------------------------------------

/// is_stale_refresh 守卫三态: 状态离开 Preview / 世代号漂移 / 均正常
#[test]
fn stale_generation_guard() {
    let shared = ControllerShared::new();
    *shared.state.write().unwrap() = ControllerState::Preview;
    shared.preview_generation.store(5, Ordering::SeqCst);
    assert!(!is_stale_refresh(&shared, 5), "Preview + 同代 → 放行");
    assert!(is_stale_refresh(&shared, 4), "世代号不匹配 → 丢弃");
    *shared.state.write().unwrap() = ControllerState::Init;
    assert!(is_stale_refresh(&shared, 5), "已离开 Preview → 丢弃");
}

/// 端到端: Preview() 捕获的世代号被 end_preview() 作废后, 消费侧丢弃
#[test]
fn end_preview_invalidates_inflight_generation() {
    let shared = ControllerShared::new();
    *shared.state.write().unwrap() = ControllerState::Preview;
    let in_flight = shared.preview_generation.load(Ordering::SeqCst);
    shared.preview_generation.fetch_add(1, Ordering::SeqCst); // stop()/endPreview()
    *shared.state.write().unwrap() = ControllerState::Init;
    assert!(is_stale_refresh(&shared, in_flight));
}

// ------------------------------------------------------------------
// debounce 时序 (Java ConfigDebounce 语义, 短间隔)
// ------------------------------------------------------------------

/// 连发变更合并为一次刷新, 载荷 = 最后一条键; 安静后无重复
#[test]
fn debounce_coalesces_rapid_changes() {
    let shared = Arc::new(ControllerShared::new());
    let (out_tx, out_rx) = std::sync::mpsc::channel::<UiCommand>();
    let mut deb =
        ConfigDebouncer::spawn(Duration::from_millis(40), out_tx, Arc::clone(&shared));
    let tx = deb.sender();
    for k in ["k1", "k2", "k3", "k4", "k5"] {
        tx.send(DebounceMsg::ConfigKey(k.to_string())).unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(tx); // shutdown 前 drop 全部发送端克隆, 否则 join 等 Disconnected 永阻塞
    match out_rx.recv_timeout(Duration::from_millis(500)) {
        Ok(UiCommand::RefreshPreviews {
            changed_key,
            generation,
        }) => {
            assert_eq!(changed_key, Some("k5".to_string()), "最后一条变更生效");
            assert_eq!(
                generation,
                shared.preview_generation.load(Ordering::SeqCst),
                "世代号为发送时快照"
            );
        }
        other => panic!("防抖后应送达一次刷新: {:?}", other),
    }
    // 安静期无第二条 (合并为一次)
    assert!(
        out_rx.recv_timeout(Duration::from_millis(120)).is_err(),
        "防抖窗口内连发只触发一次"
    );
    deb.shutdown();
}

/// FmChanged → 全量刷新 (changed_key=None); RESET_COMPLETED → 全量
#[test]
fn debounce_fm_and_reset_full_refresh() {
    let shared = Arc::new(ControllerShared::new());
    let (out_tx, out_rx) = std::sync::mpsc::channel::<UiCommand>();
    let mut deb = ConfigDebouncer::spawn(Duration::from_millis(30), out_tx, shared);
    {
        let tx = deb.sender();
        tx.send(DebounceMsg::FmChanged).unwrap();
    } // 块尾 drop 发送端克隆
    match out_rx.recv_timeout(Duration::from_millis(500)) {
        Ok(UiCommand::RefreshPreviews { changed_key: None, .. }) => {}
        other => panic!("FmChanged 应产全量刷新: {:?}", other),
    }
    {
        let tx = deb.sender();
        tx.send(DebounceMsg::ConfigKey(
            ui_state_events::ACTION_RESET_COMPLETED.to_string(),
        ))
        .unwrap();
    }
    match out_rx.recv_timeout(Duration::from_millis(500)) {
        Ok(UiCommand::RefreshPreviews { changed_key: None, .. }) => {}
        other => panic!("RESET_COMPLETED 应产全量刷新: {:?}", other),
    }
    deb.shutdown();
}

// ------------------------------------------------------------------
// WYSIWYG 链 (CONFIG_CHANGED → 防抖 → RefreshPreviews)
// ------------------------------------------------------------------

/// Preview 态: CONFIG_CHANGED → 转发 → 防抖 (30ms) → RefreshPreviews(键) 命令
#[test]
fn wysiwyg_config_change_refreshes_via_debounce() {
    let mut shell = fixture_with_debounce(30);
    publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
    pump_events(&mut shell);
    // MainForm 侧配置写入 (服务内联发布 CONFIG_CHANGED — vm-ui 链同源)
    publish_ui_event(&shell.ui_bus, ui_state_events::CONFIG_CHANGED, "showSpeedBar");
    assert!(pump_events(&mut shell), "CONFIG_CHANGED 应经转发到达监督循环");
    // 防抖产出直达 win32 命令通道 — 接收端留在 shell (未 spawn win32)。
    // 容忍 UI_READY→preview() 的 Preview-Refresh 线程全量刷新 (None) 抢先入队
    let mut keyed = None;
    let mut generation_seen = 0u64;
    for _ in 0..4 {
        match shell
            .ui_cmd_rx
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_millis(500))
        {
            Ok(UiCommand::RefreshPreviews {
                changed_key: Some(k),
                generation,
            }) => {
                keyed = Some(k);
                generation_seen = generation;
                break;
            }
            Ok(_) => continue, // 全量刷新 (Preview-Refresh/FmChanged) — 竞态容忍
            Err(e) => panic!("防抖后应有键控刷新: {:?}", e),
        }
    }
    assert_eq!(keyed.as_deref(), Some("showSpeedBar"), "最后一条变更生效");
    assert_eq!(
        generation_seen,
        shell.shared.preview_generation.load(Ordering::SeqCst)
    );
}

/// 非 Preview 态: CONFIG_CHANGED → 立即 ReinitActiveOverlays (无防抖)
#[test]
fn config_change_non_preview_reinits_active() {
    let mut shell = fixture_with_debounce(30);
    publish_ui_event(&shell.ui_bus, ui_state_events::CONFIG_CHANGED, "fontNum");
    assert!(pump_events(&mut shell));
    let cmd = shell
        .ui_cmd_rx
        .as_ref()
        .unwrap()
        .recv_timeout(Duration::from_millis(200))
        .expect("fontNum 触发五色直送 + ReinitActiveOverlays");
    // 五色键: 先 SetGlobalColors (fixture cfg 五色全白 #FFFFFFFF), 后 Reinit
    let all_white = GlobalColors {
        num: [255, 255, 255, 255],
        label: [255, 255, 255, 255],
        unit: [255, 255, 255, 255],
        warning: [255, 255, 255, 255],
        shade_shape: [255, 255, 255, 255],
    };
    assert_eq!(cmd, UiCommand::SetGlobalColors(all_white));
    // WYSIWYG reinit 参数直送 (ReinitActiveOverlays 前置): 参数包随命令进
    // win32 线程 (配置 !Send — 五色直送同款模式)
    let cmd_rp = shell
        .ui_cmd_rx
        .as_ref()
        .unwrap()
        .recv_timeout(Duration::from_millis(200))
        .expect("ReinitActiveOverlays 前应先到 ReinitOverlays");
    match cmd_rp {
        UiCommand::ReinitOverlays { params } => {
            // fixture cfg 无地平仪组 → Java reinitConfig 缺省 150×300
            assert_eq!(params.attitude_width, 150);
        }
        other => panic!("应是 ReinitOverlays: {:?}", other),
    }
    let cmd2 = shell
        .ui_cmd_rx
        .as_ref()
        .unwrap()
        .recv_timeout(Duration::from_millis(200))
        .expect("非 Preview 态应立即 ReinitActiveOverlays");
    assert_eq!(cmd2, UiCommand::ReinitActiveOverlays);
    // 防抖不排队 (无后续刷新命令)
    assert!(shell
        .ui_cmd_rx
        .as_ref()
        .unwrap()
        .recv_timeout(Duration::from_millis(100))
        .is_err());
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Init
    );
}

/// CONFIG_CHANGED 的 reinit 参数直送: 配置写入值即时进 ReinitOverlays 参数包
/// (写 "地平仪" 组的 attitudeIndicatorWidth 行 → attitude_width)
#[test]
fn config_change_reinit_params_carry_written_config() {
    // cfg 需含目标行 (Java setConfig 只改既有行, 无行即 no-op)
    let cfg = fixture_cfg(
        "(panel \"地平仪\" :visible true\n\
             \x20 (item \"宽\" :type slider :target \"attitudeIndicatorWidth\" :value 150))\n",
    );
    let mut shell = fixture_full(30, cfg);
    // 发布方 (渲染器) 已写配置树 — 本测试直接经 set_config 模拟写点
    shell
        .controller
        .as_ref()
        .unwrap()
        .config
        .set_config("attitudeIndicatorWidth", "222");
    publish_ui_event(
        &shell.ui_bus,
        ui_state_events::CONFIG_CHANGED,
        "attitudeIndicatorWidth",
    );
    assert!(pump_events(&mut shell));
    let mut saw_reinit = None;
    for _ in 0..4 {
        match shell
            .ui_cmd_rx
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_millis(200))
        {
            Ok(UiCommand::ReinitOverlays { params }) => {
                saw_reinit = Some(params);
                break;
            }
            Ok(_) => continue,
            Err(e) => panic!("应有 ReinitOverlays: {:?}", e),
        }
    }
    let params = saw_reinit.expect("ReinitOverlays 应到达");
    // 写值即时进参数包 (初值 150, 写 222 — 证明非 spawn 期冻结快照)
    assert_eq!(params.attitude_width, 222, "写值应即时进参数包");
}

/// Preview 态: ReinitOverlays 先于防抖的 RefreshPreviews 入队
/// (win32 消费序 = 参数先刷新, 再跑各 overlay reinit — Java refreshPreviews →
/// reinitConfig 读即时配置的时序等价)
#[test]
fn preview_reinit_params_precede_debounced_refresh() {
    let mut shell = fixture_with_debounce(30);
    publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
    pump_events(&mut shell);
    publish_ui_event(&shell.ui_bus, ui_state_events::CONFIG_CHANGED, "showSpeedBar");
    assert!(pump_events(&mut shell));
    let mut reinit_at: Option<usize> = None;
    let mut refresh_at: Option<usize> = None;
    for i in 0..6 {
        match shell
            .ui_cmd_rx
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_millis(400))
        {
            Ok(UiCommand::ReinitOverlays { .. }) => reinit_at = Some(i),
            Ok(UiCommand::RefreshPreviews { changed_key: Some(k), .. }) if k == "showSpeedBar" => {
                refresh_at = Some(i)
            }
            Ok(_) => {}
            Err(_) => break,
        }
        if reinit_at.is_some() && refresh_at.is_some() {
            break;
        }
    }
    let (r, f) = (reinit_at.expect("ReinitOverlays 应到达"), refresh_at.expect("键控 RefreshPreviews 应到达"));
    assert!(r < f, "参数直送 ({}) 应先于防抖刷新 ({})", r, f);
}

/// FM_CHANGED (missing-like) → 摘要转发 (toast 面) + Preview 态防抖全量刷新
#[test]
fn fm_changed_missing_schedules_full_refresh() {
    let mut shell = fixture_with_debounce(30);
    publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
    pump_events(&mut shell);
    // 直接向 FM 通道发布 missing-like 句柄 (Java FM-Loader 线程发布形态)
    let handle = FMHandle {
        name: Some("nosuch-plane".to_string()),
        status: FMStatus::Missing,
        ..FMHandle::UNRESOLVED
    };
    shell.fm.fm_changed_bus().publish(&handle);
    // 两条转发 (missing 摘要 + 刷新调度) 到达监督通道
    let mut saw_missing = false;
    while let Ok(ev) = shell.main_event_rx.recv_timeout(Duration::from_millis(200)) {
        if let MainEvent::FmChanged { name: Some(_), .. } = &ev {
            saw_missing = true;
        }
        shell.handle_main_event(ev);
    }
    assert!(saw_missing, "missing-like 摘要应转发");
    // Preview 态 → 防抖全量刷新命令
    let cmd = shell
        .ui_cmd_rx
        .as_ref()
        .unwrap()
        .recv_timeout(Duration::from_millis(500))
        .expect("FM_CHANGED 应触发防抖全量刷新");
    match cmd {
        UiCommand::RefreshPreviews { changed_key: None, .. } => {}
        other => panic!("应为全量刷新: {:?}", other),
    }
}

// ------------------------------------------------------------------
// 托盘重建 (Application.ctr 替换)
// ------------------------------------------------------------------

/// 托盘 Activate: 旧核 stop (退订) → 新核 (State=INIT, 订阅重建, 无泄漏累积)
#[test]
fn tray_activate_rebuilds_controller() {
    let mut shell = fixture();
    let before = shell.ui_bus.subscriber_count();
    shell.handle_main_event(MainEvent::Tray(TrayCommand::Activate));
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Init,
        "新核从 INIT 开始"
    );
    assert_eq!(
        shell.ui_bus.subscriber_count(),
        before,
        "旧核退订 + 新核订阅, 净计数不变 (无泄漏累积)"
    );
}

/// A-W1: 托盘重建复位 — sessionAircraftType / overlay_ctx_preview /
/// last_flight_event_ms 随新核归零 (Java 均为 Controller 实例字段,
/// 每次重建随新构造器归默认; Rust 跨核共享故需显式复位)
#[test]
fn reset_for_rebuild_clears_session_state() {
    let shared = ControllerShared::new();
    shared.flags.lock().unwrap().session_aircraft_type = Some("old-plane".into());
    shared.overlay_ctx_preview.store(false, Ordering::SeqCst);
    shared.last_flight_event_ms.store(12345, Ordering::SeqCst);
    *shared.state.write().unwrap() = ControllerState::Preview;
    shared.reset_for_rebuild();
    assert_eq!(*shared.state.read().unwrap(), ControllerState::Init);
    assert!(shared.flags.lock().unwrap().session_aircraft_type.is_none());
    assert!(shared.overlay_ctx_preview.load(Ordering::SeqCst));
    assert_eq!(shared.last_flight_event_ms.load(Ordering::SeqCst), 0);
}

/// A-W2: EndGame = 保存 + 退出请求 (Java mCancel 的 System.exit(0) 归属)
#[test]
fn end_game_requests_exit() {
    let mut shell = fixture();
    assert!(!shell.is_exit_requested());
    shell.dispatch(UiCommand::EndGame);
    assert!(shell.is_exit_requested(), "EndGame 应置退出请求");
}

/// A-W6: 托盘 Start 与 MainForm 确认叠加 — start() 对"Service 已在跑"幂等:
/// 不二次 spawn (live 句柄不换新), 首次会话不中断
#[test]
fn start_guard_prevents_double_spawn_on_tray_start_overlay() {
    let mut shell = fixture();
    shell.handle_main_event(MainEvent::Tray(TrayCommand::Start));
    assert!(
        shell.controller.as_ref().unwrap().service.is_some(),
        "托盘 Start 应起 Service"
    );
    let live1 = shell.shared.live.read().unwrap().clone();
    // 用户再点 MainForm "开始游戏" (叠加态): confirm 链被 service.is_some() 守卫拦下
    shell.dispatch(UiCommand::StartGame);
    let live2 = shell.shared.live.read().unwrap().clone();
    assert!(live2.is_some(), "首次 Service 应存活");
    assert!(
        Arc::ptr_eq(&live1.unwrap(), &live2.unwrap()),
        "叠加 start 不应重建 Service (live 句柄不变)"
    );
}

/// B-W5: change_s3 延迟开面板的停止守卫 — 100ms 窗口内游戏退出 (S4toS1 →
/// Init) 则 OpenAllOverlays 被丢弃 (Java 无守卫; Rust 侧重开残留形态更重,
/// 显式改进偏离, 见 change_s3 的 PORT 注)
#[test]
fn change_s3_openpad_delay_guarded_on_exit() {
    let mut shell = fixture();
    {
        let c = shell.controller.as_mut().unwrap();
        c.init_status_bar();
        c.change_s2();
    }
    let data = Arc::new(std::sync::RwLock::new(live_service_data("g1")));
    *shell.shared.live.write().unwrap() = Some(data);
    shell
        .shared
        .last_flight_event_ms
        .store(current_time_millis(), Ordering::SeqCst);
    shell.controller.as_mut().unwrap().drive_from_live(); // changeS3 → 延迟线程
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Preview
    );
    // 100ms 窗口内游戏退出 (静默) → S4toS1 → state=Init
    shell.shared.last_flight_event_ms.store(
        current_time_millis() - FLIGHT_SILENT_EXIT_MS - 1,
        Ordering::SeqCst,
    );
    shell.controller.as_mut().unwrap().drive_from_live();
    assert_eq!(shell.controller.as_ref().unwrap().state(), ControllerState::Init);
    std::thread::sleep(Duration::from_millis(250)); // 越过延迟窗口
    // 通道内可有 CloseAllOverlays (s4to_s1 的 closepad), 但不应有 OpenAllOverlays
    let ui_rx = shell.ui_cmd_rx.take().unwrap();
    loop {
        match ui_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(UiCommand::OpenAllOverlays) => {
                panic!("退出后的 OpenAllOverlays 应被守卫丢弃")
            }
            Ok(_) => continue,
            Err(_) => break,
        }
    }
}

/// 激活缓存随配置装载 (win32 激活面的 WYSIWYG 输入)
#[test]
fn activation_cache_tracks_config() {
    let shell = fixture();
    let v = shell
        .activation
        .lock()
        .unwrap()
        .get("crosshairSwitch")
        .cloned();
    assert_eq!(
        v.as_deref(),
        Some("true"),
        "tmp cfg 的 crosshairSwitch=true 应入缓存"
    );
    let v2 = shell
        .activation
        .lock()
        .unwrap()
        .get("enableEngineControl")
        .cloned();
    assert_eq!(v2.as_deref(), Some("false"));
}

// ------------------------------------------------------------------
// win32 线程生命周期 (真实窗口冒烟)
// ------------------------------------------------------------------

/// win32 线程: 注册不 panic → 刷新命令消费 → Shutdown → join 干净退出
/// (JoinHandle 无泄漏; 真实 Win32 窗口/托盘创建, 无桌面环境时托盘缺失仍可跑)
#[test]
fn win32_thread_shutdown_joins_cleanly() {
    let mut shell = fixture();
    shell.spawn_win32_thread().expect("win32 线程启动");
    // Preview 态 + 有效世代号 → 全量刷新命令 (守卫放行路径, 不 stale)
    *shell.shared.state.write().unwrap() = ControllerState::Preview;
    let gen = shell.shared.preview_generation.load(Ordering::SeqCst);
    shell
        .ui_cmd_tx
        .send(UiCommand::RefreshPreviews {
            changed_key: None,
            generation: gen,
        })
        .unwrap();
    std::thread::sleep(Duration::from_millis(150)); // 泵消费 + 渲染一拍
    // 过期命令: 世代号 +1 → 消费侧丢弃 (守卫路径)
    shell
        .ui_cmd_tx
        .send(UiCommand::RefreshPreviews {
            changed_key: None,
            generation: gen + 1,
        })
        .unwrap();
    std::thread::sleep(Duration::from_millis(60));
    shell.ui_cmd_tx.send(UiCommand::Shutdown).unwrap();
    let join = shell.win32.take().unwrap();
    assert!(join.join().is_ok(), "win32 线程应干净退出");
    assert!(shell.win32.is_none());
}

// ------------------------------------------------------------------
// 组装接线 (本批新增面)
// ------------------------------------------------------------------

/// autoStartGameMode=true → Controller 自启动分支 (跳过 MainForm, Service 直起;
/// --game-mode/--mock-smoke 的配置注入对位, Controller.java:589-606)
#[test]
fn auto_start_game_mode_skips_main_form() {
    let shell = fixture_full(30, auto_start_cfg());
    let c = shell.controller.as_ref().unwrap();
    assert!(!c.main_form_alive, "自启动路径 M 恒 null (Java:604-606)");
    assert!(c.service.is_some(), "自启动应直起 Service 线程");
}

/// 托盘 Activate 置设置窗请求位 (一次性取走; 组装层主循环的相位切换信号)
#[test]
fn tray_activate_requests_main_form() {
    let mut shell = fixture();
    shell.handle_main_event(MainEvent::Tray(TrayCommand::Activate));
    assert!(shell.take_form_request(), "托盘 Activate 应置请求位");
    assert!(!shell.take_form_request(), "取走后复位 (一次性)");
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Init,
        "Activate 同时重建核 (INIT)"
    );
}

/// 分相监督循环: 退出请求 → Exit (EndGame/托盘 Exit 的相位出口;
/// 阻断自动 spawn win32 — 纯状态机断言, 不开真窗)
#[test]
fn supervisor_phase_returns_exit_on_request() {
    let mut shell = fixture();
    shell.ui_cmd_rx.take();
    shell.handle_main_event(MainEvent::Tray(TrayCommand::Exit));
    assert_eq!(shell.run_supervisor_phase(), SupervisorOutcome::Exit);
}

/// 分相监督循环: 托盘 Activate → MainFormRequested (重建核 + 请求开窗)
#[test]
fn supervisor_phase_returns_form_request_on_tray_activate() {
    let mut shell = fixture();
    shell.ui_cmd_rx.take();
    shell.handle_main_event(MainEvent::Tray(TrayCommand::Activate));
    assert_eq!(
        shell.run_supervisor_phase(),
        SupervisorOutcome::MainFormRequested
    );
}

/// win32 渲染帧计数: 预览刷新打开 overlay 后 present 帧数递增
/// (--mock-smoke 核心断言的库内等价; 字体目录钉仓库根, 见 fixture 注)
#[test]
fn win32_render_frames_advance_with_active_overlays() {
    let mut shell = fixture();
    shell.spawn_win32_thread().expect("win32 线程启动");
    // Preview 态 + 有效世代号 → 全量刷新 (MiniHUD crosshairSwitch=true 激活)
    *shell.shared.state.write().unwrap() = ControllerState::Preview;
    let gen = shell.shared.preview_generation.load(Ordering::SeqCst);
    shell
        .ui_cmd_tx
        .send(UiCommand::RefreshPreviews {
            changed_key: None,
            generation: gen,
        })
        .unwrap();
    // 泵消费 (10ms 节拍) + 窗口物化 + 至少数个 50ms 渲染节拍
    std::thread::sleep(Duration::from_millis(800));
    let frames = shell.shared.render_frames.load(Ordering::SeqCst);
    shell.ui_cmd_tx.send(UiCommand::Shutdown).unwrap();
    let join = shell.win32.take().unwrap();
    assert!(join.join().is_ok());
    assert!(
        frames > 0,
        "活跃 overlay 的 present 帧数应递增 (实测 {frames})"
    );
}

/// 逐 overlay present 计数: 游戏模式全开 (open_all) 后 6 注册键全部 present>0
/// (--mock-smoke 断言 3 的库内等价; 字体目录钉仓库根, 见 fixture 注)
#[test]
fn win32_overlay_present_counts_per_registered_overlay() {
    let all_on_cfg = fixture_cfg(
        "(panel \"T\" :visible true\n\
             \x20 (item \"a\" :type switch :target \"crosshairSwitch\" :value true)\n\
             \x20 (item \"b\" :type switch :target \"engineInfoSwitch\" :value true)\n\
             \x20 (item \"c\" :type switch :target \"enableEngineControl\" :value true)\n\
             \x20 (item \"d\" :type switch :target \"enablegearAndFlaps\" :value true)\n\
             \x20 (item \"e\" :type switch :target \"enableAxis\" :value true)\n\
             \x20 (item \"f\" :type switch :target \"enableAttitudeIndicator\" :value true))\n\
            ",
    );
    let mut shell = fixture_full(30, all_on_cfg);
    shell.spawn_win32_thread().expect("win32 线程启动");
    shell.ui_cmd_tx.send(UiCommand::OpenAllOverlays).unwrap();
    // 泵消费 (10ms 节拍) + 6 窗物化 + 至少数个 50ms 渲染节拍
    std::thread::sleep(Duration::from_millis(1200));
    let counts = shell
        .shared
        .overlay_present
        .lock()
        .expect("overlay_present 锁中毒")
        .clone();
    shell.ui_cmd_tx.send(UiCommand::Shutdown).unwrap();
    let join = shell.win32.take().unwrap();
    assert!(join.join().is_ok());
    for id in [
        "enableEngineControl",
        "engineInfoSwitch",
        "crosshairSwitch",
        "enablegearAndFlaps",
        "enableAxis",
        "enableAttitudeIndicator",
    ] {
        let c = counts.get(id).copied().unwrap_or(0);
        assert!(c > 0, "overlay {id} present 应 >0 (实测 {c}, 全量 {counts:?})");
    }
}

/// RefreshPreviews 不翻转会话窗口形态 (审查 blocker 回归锚): 游戏稳态
/// (openpad 后 overlay_ctx_preview=false, State=Preview) 下 FM_CHANGED/
/// ConfigChanged 防抖必发 RefreshPreviews — 原实现置 true 后生产路径无复位点,
/// feed_overlays_live 整帧 early-return, 6 窗口数据冻结; 修复 = 激活探测期
/// 临时置位 (Java refreshPreviews 的 forPreviewMode ctx), 完毕恢复会话形态。
/// CloseAllOverlays 复位 true (会话结束回预览态)。
///
/// 断言用轮询终态: refresh_preview 的激活探测期内物化真窗口 (Win32
/// CreateWindow 冷启动可达百 ms 级), 标志短暂为 true 是修复的合法瞬态 —
/// 轮询穿过后核对终值; 原 bug 形态下永久 true, 轮询超时必失败。
#[test]
fn refresh_previews_keep_session_window_mode() {
    fn wait_flag(flag: &AtomicBool, want: bool) -> bool {
        let start = Instant::now();
        while flag.load(Ordering::SeqCst) != want {
            if start.elapsed() > Duration::from_millis(2000) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        true
    }
    let mut shell = fixture();
    shell.spawn_win32_thread().expect("win32 线程启动");
    *shell.shared.state.write().unwrap() = ControllerState::Preview;
    let gen = shell.shared.preview_generation.load(Ordering::SeqCst);
    // 模拟游戏形态 (openpad → OpenAllOverlays 处理点置 false)
    shell.shared.overlay_ctx_preview.store(false, Ordering::SeqCst);
    shell
        .ui_cmd_tx
        .send(UiCommand::RefreshPreviews {
            changed_key: None,
            generation: gen,
        })
        .unwrap();
    assert!(
        wait_flag(&shell.shared.overlay_ctx_preview, false),
        "游戏稳态刷新预览后 live 门控应恢复开启 (原 bug: 置 true 后 live 冻结)"
    );
    // CloseAll (stop/end_preview/S4toS1 的公共出口) → 回预览态
    shell.ui_cmd_tx.send(UiCommand::CloseAllOverlays).unwrap();
    assert!(
        wait_flag(&shell.shared.overlay_ctx_preview, true),
        "CloseAll 后窗口形态应回预览态"
    );
    shell.ui_cmd_tx.send(UiCommand::Shutdown).unwrap();
    let join = shell.win32.take().unwrap();
    assert!(join.join().is_ok());
}

// ------------------------------------------------------------------
// 辅助
// ------------------------------------------------------------------

/// 构造 drive_from_live 判定所需的 live ServiceData 快照
/// (真机由 Service.update 写; 测试直填公开字段 — flags/type/playerLive)
fn live_service_data(plane: &str) -> ServiceData {
    let mut st = vm_core::parser::State::new();
    st.flag = true;
    let mut ind = vm_core::parser::Indicators::new();
    ind.flag = true;
    ind.r#type = Some(plane.to_string());
    let mut d = ServiceData::default();
    // s_state/s_indic 构造期由 Service::new 建立 (service_fields Default 注);
    // 测试直填 Some
    d.s_state = Some(st);
    d.s_indic = Some(ind);
    d.player_live = true;
    d
}

// ------------------------------------------------------------------
// P6 收口: 注册面 6 窗口条目 / live 喂数全链 / 注册键 ↔ ui_layout.cfg 核对
// ------------------------------------------------------------------

/// 测试用空窗口 (field2 测试 MiniWin 同款; 免真窗依赖)
struct NullWin;

impl vm_overlay::platform::OverlayWindow for NullWin {
    fn present(&mut self, _buf: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn set_position(&mut self, _x: i32, _y: i32) {}
    fn position(&self) -> (i32, i32) {
        (0, 0)
    }
    fn set_click_through(&mut self, _on: bool) {}
    fn poll_event(&mut self) -> Option<vm_overlay::platform::OverlayEvent> {
        None
    }
    fn screen_size(&self) -> (i32, i32) {
        (1920, 1080)
    }
}

fn test_overlay_inputs() -> OverlayInputs {
    OverlayInputs {
        dpi_scale: 1.0,
        hud: HudSettingsSnapshot::build(
            &fixture().controller.as_ref().unwrap().config.get_hud_settings(),
        ),
        font_add_engine: 0,
        font_add_power: 0,
        power_columns: 1,
        font_add_flight: 0,
        flight_columns: 1,
        font_add_gear: 0,
        gear_show_edge: false,
        font_add_axis: 0,
        axis_show_edge: false,
        attitude_width: 150,
        attitude_height: 300,
        attitude_freq_ms: 40,
        attitude_show_direction: false,
        attitude_show_aoa_limits: true,
        service_loop_interval_ms: 50,
        colors: GlobalColors::JAVA_DEFAULT,
        aa: true,
        engine_disables: [false; 7],
    }
}

/// 注册面: Java registerGameModeOverlays 的 7 个窗口条目全部落位
/// (open_all 默认激活全真; 剩 3 键非窗口/降级备案见 register_game_mode_overlays 头注)
#[test]
fn register_game_mode_overlays_seven_window_entries() {
    let mut host = OverlayHost::with_factory(Box::new(|_cfg| {
        Ok(Box::new(NullWin) as Box<dyn vm_overlay::platform::OverlayWindow>)
    }));
    let mut handles = OverlayHandles {
        minihud: None,
        power_info: None,
        engine_control: None,
        gear_flaps: None,
        attitude: None,
        control_surfaces: None,
    flight_info: None,
    };
    let shell = fixture();
    let lang = Rc::new(Lang::init_lang());
    let params = Rc::new(RefCell::new(vm_overlay::ReinitParams::from(
        &test_overlay_inputs(),
    )));
    register_game_mode_overlays(
        &mut host,
        &mut handles,
        &shell.env,
        &test_overlay_inputs(),
        &params,
        &lang,
        &shell.shared,
    );
    // 注册面逐窗计数落键: 6 键全部以 0 落位 (present 计数起点)
    let reg_keys: Vec<String> = shell
        .shared
        .overlay_present
        .lock()
        .expect("overlay_present 锁中毒")
        .keys()
        .cloned()
        .collect();
    assert_eq!(reg_keys.len(), 7, "注册落键应恰为 7 键 (实测 {reg_keys:?})");
    // 7 个共享句柄全部登记 (spec 工厂成功)
    assert!(handles.minihud.is_some(), "MiniHUD 句柄");
    assert!(handles.power_info.is_some(), "动力信息句柄");
    assert!(handles.engine_control.is_some(), "引擎控制句柄");
    assert!(handles.flight_info.is_some(), "飞行信息句柄");
    assert!(handles.gear_flaps.is_some(), "起落襟翼句柄");
    assert!(handles.attitude.is_some(), "地平仪句柄");
    assert!(handles.control_surfaces.is_some(), "操纵面句柄");
    host.open_all().expect("全激活 open_all");
    let mut ids: Vec<String> = host.active_ids();
    ids.sort();
    assert_eq!(
        ids,
        vec![
            "crosshairSwitch",
            "enableAttitudeIndicator",
            "enableAxis",
            "enableEngineControl",
            "enablegearAndFlaps",
            "engineInfoSwitch",
            "flightInfoSwitch",
        ],
        "注册键 10 键中的 7 窗口条目 (Java 键一一对应)"
    );
}

/// live 喂数全链: 一帧 payload 喂 6 个 overlay, 各 state 推进到遥测值
/// (ServiceData 的引擎数组必须非空 — get_pitch/get_thrust 的保真 panic 点,
/// 真实链路由 State.update 填满; catch_unwind 吞帧路径由 malformed 变体覆盖)
#[test]
fn feed_overlays_live_updates_all_handles() {
    let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts");
    let lang = Rc::new(Lang::init_lang());
    let inputs = test_overlay_inputs();
    let (h_mini, _) = vm_overlay::minihud_overlay_spec(
        false,
        50,
        &inputs.hud,
        1.0,
        &fonts.join("sarasa-mono-sc-bold.ttf"),
        &Rc::new(RefCell::new(vm_overlay::ReinitParams::default())),
    )
    .unwrap();
    let (h_power, _) = vm_overlay::power_info_overlay_spec(
        &fonts,
        &Rc::new(RefCell::new(vm_overlay::ReinitParams::default())),
    )
    .unwrap();
    let (h_engine, _) = vm_overlay::engine_control_overlay_spec(
        &fonts,
        Rc::clone(&lang),
        &Rc::new(RefCell::new(vm_overlay::ReinitParams {
            service_loop_interval_ms: 50,
            ..Default::default()
        })),
    )
    .unwrap();
    let (h_gear, _) = vm_overlay::gear_flaps_overlay_spec(
        &fonts,
        &Rc::new(RefCell::new(vm_overlay::ReinitParams::default())),
    )
    .unwrap();
    let (h_att, _) = vm_overlay::attitude_overlay_spec(&Rc::new(RefCell::new(
        vm_overlay::ReinitParams::default(),
    )))
    .unwrap();
    let (h_cs, _) = vm_overlay::control_surfaces_overlay_spec(
        &fonts,
        &Rc::new(RefCell::new(vm_overlay::ReinitParams::default())),
    )
    .unwrap();
    let (h_fi, _) = vm_overlay::flight_info_overlay_spec(
        &fonts,
        &Rc::new(RefCell::new(vm_overlay::ReinitParams::default())),
    )
    .unwrap();
    let handles = OverlayHandles {
        minihud: Some(h_mini),
        power_info: Some(h_power),
        engine_control: Some(h_engine),
        gear_flaps: Some(h_gear),
        attitude: Some(h_att),
        control_surfaces: Some(h_cs),
        flight_info: Some(h_fi),
    };

    // live 快照: throttle 55 / flaps 25 / gear 100 / aileron 100 / aoa 10 /
    // aviahorizon pitch 5 / 功率 1200 / airbrake 100 (引擎数组填满防 panic 点)
    let mut d = live_service_data("feed-plane");
    {
        let st = d.s_state.as_mut().unwrap();
        st.throttle = 55;
        st.flaps = 25;
        st.gear = 100;
        st.airbrake = 100;
        st.aileron = 100;
        st.aoa = 10.0;
        st.pitch = vec![0.0; 8];
        st.thrust = vec![0; 8];
        st.power = vec![0.0; 8];
        st.efficiency = vec![0.0; 8];
        st.throttles = vec![0; 8];
        st.rpm_throttle = 60;
    }
    d.s_indic.as_mut().unwrap().aviahorizon_pitch = 5.0;
    d.total_hp = 1200;
    let shared = ControllerShared::new();
    shared.overlay_ctx_preview.store(false, Ordering::SeqCst); // 游戏窗口形态
    *shared.live.write().unwrap() = Some(Arc::new(std::sync::RwLock::new(d)));
    let fm = FMManager::new(Arc::new(EventBus::new()));
    let settings = inputs.hud.clone();
    let payload = EventPayload::builder().build();
    let mut attitude_feed = AttitudeFeedState { freq_ms: 40, last_ms: 0 };

    feed_overlays_live(&handles, &payload, &shared, &fm, &settings, &lang, &mut attitude_feed);

    // 动力信息: 功率 1200 → 首字段 buffer (50ms 节流: now-0 恒放行)
    let p = handles.power_info.as_ref().unwrap().borrow();
    assert_eq!(p.fields()[0].buffer, "1200", "PowerInfo 功率字段");
    drop(p);
    // 引擎控制: throttle 55 (refreshInterval=100, 首帧放行)
    let e = handles.engine_control.as_ref().unwrap().borrow();
    assert_eq!(e.gauge_by_key("throttle").unwrap().gauge.gauge.cur_value, 55);
    drop(e);
    // 起落襟翼: gear=100 + airbrake=100 → "起落架 减速板" 告警; flaps=25 → flap_pix
    let g = handles.gear_flaps.as_ref().unwrap().borrow();
    assert_eq!(g.warn_text, "起落架 减速板");
    assert_eq!(g.flap_pix, 24);
    drop(g);
    // 操纵面: aileron=100 → px = (100+100)*144/200 = 144 (has_service 喂入点置位)
    let cs = handles.control_surfaces.as_ref().unwrap().borrow();
    assert_eq!(cs.px, 144);
    drop(cs);
    // 地平仪: aoa=10 → AoA = round((10+30)·300/60) = 200
    let a = handles.attitude.as_ref().unwrap().borrow();
    assert_eq!(a.aoa_y, 200);
    drop(a);
    // 地平仪节流: 40ms 窗口内第二帧不重算 (last_ms 已推进)
    assert!(attitude_feed.last_ms > 0);
    // MiniHUD: 重构事件携带 state 快照 → hud_calculator 读到 sState.airbrake=100
    // → warnVne 置真 (state 丢失时该块整体跳过, warn_vne 恒 false = "bar 恒 0"
    // 根因的回归哨)
    assert!(
        handles.minihud.as_ref().unwrap().borrow().warn_vne,
        "MiniHUD 应收到 sState 快照 (airbrake=100 → warnVne)"
    );

    // preview 门控: overlay_ctx_preview=true → 整帧跳过 (值不推进)
    shared.overlay_ctx_preview.store(true, Ordering::SeqCst);
    {
        let live = shared.live.read().unwrap().clone();
        let mut st = live.as_ref().unwrap().write().unwrap();
        st.s_state.as_mut().unwrap().throttle = 99;
    }
    feed_overlays_live(&handles, &payload, &shared, &fm, &settings, &lang, &mut attitude_feed);
    let e = handles.engine_control.as_ref().unwrap().borrow();
    assert_eq!(
        e.gauge_by_key("throttle").unwrap().gauge.gauge.cur_value,
        55,
        "preview 期不喂入 (Java initPreview 不订阅)"
    );
}

/// 畸形 s_state (引擎数组空, update 未跑) 的保真 panic 点: catch_unwind 吞帧不杀线程
#[test]
fn feed_overlays_live_swallows_malformed_frame() {
    let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts");
    let lang = Lang::init_lang();
    // 只接 PowerInfo (get_pitch 空 Vec panic 点; 其余 handle 缺省 None)
    let (h_power, _) = vm_overlay::power_info_overlay_spec(
        &fonts,
        &Rc::new(RefCell::new(vm_overlay::ReinitParams::default())),
    )
    .unwrap();
    let handles = OverlayHandles {
        minihud: None,
        power_info: Some(h_power),
        engine_control: None,
        gear_flaps: None,
        attitude: None,
        control_surfaces: None,
    flight_info: None,
    };
    let shared = ControllerShared::new();
    shared.overlay_ctx_preview.store(false, Ordering::SeqCst);
    // State::new() 的 pitch/thrust 空 Vec — get_pitch 的 s.pitch[0] panic (保真)
    *shared.live.write().unwrap() =
        Some(Arc::new(std::sync::RwLock::new(live_service_data("bad"))));
    let fm = FMManager::new(Arc::new(EventBus::new()));
    let settings = test_overlay_inputs().hud;
    let payload = EventPayload::builder().build();
    let mut attitude_feed = AttitudeFeedState { freq_ms: 40, last_ms: 0 };
    // 不 panic 即通过 (吞帧 + ERROR 留痕; Java NPE 由 EDT 吞的同位形态)
    feed_overlays_live(&handles, &payload, &shared, &fm, &settings, &lang, &mut attitude_feed);
}

/// 注册键 ↔ ui_layout.cfg 核对: 9 个激活键 (ACTIVATION_KEYS) 全部以 panel switch
/// 形式存在 (Java 端第 10 键 thrustdFS 无 cfg 项 — 策略读 enableFMPrint,
/// DrawFrameSimpl 无独立开关, Java 同形态)
#[test]
fn activation_keys_match_ui_layout_cfg() {
    let cfg_path =
        locate_template_cfg().expect("仓库模板 ui_layout.cfg 应可达 (上溯三级)");
    let text = std::fs::read_to_string(&cfg_path).unwrap();
    for key in ACTIVATION_KEYS {
        let target = format!(":target \"{}\"", key);
        assert!(
            text.contains(&target),
            "激活键 {key} 应以 :target 开关项存在于 ui_layout.cfg"
        );
    }
    // 6 个窗口条目键 (注册面) 与 cfg 面板一一对应 (10 panel 中 9 有 switch 键,
    // thrustdFS 例外; 欢迎/飞行记录/全局设置 无 overlay 开关)
    for key in [
        "crosshairSwitch",
        "flightInfoSwitch",
        "engineInfoSwitch",
        "enableEngineControl",
        "enableAxis",
        "enablegearAndFlaps",
        "enableAttitudeIndicator",
        "enableFMPrint",
        "enableVoiceWarn",
    ] {
        assert!(text.contains(&format!(":target \"{}\"", key)), "键 {key} 缺失");
    }
}

/// 提取 ui_layout.cfg 全部 :target 键值 (兴趣键命中核对的键空间源)
fn cfg_target_keys(text: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find(":target \"") {
        rest = &rest[i + 9..];
        match rest.find('"') {
            Some(j) => {
                keys.push(rest[..j].to_string());
                rest = &rest[j..];
            }
            None => break,
        }
    }
    keys
}

/// MiniHUD 兴趣键 ↔ ui_layout.cfg 键空间核对 (审查 W1 回归锚): with_interest
/// 为前缀匹配 (host is_interested_in), 死键不命中任何 cfg 键 → WYSIWYG 开关
/// 切换时 MiniHUD 不刷新 (Java 会刷新)。曾笔误 "showAttitudeIndicator"
/// (正确键 showAttitudeGauge, ui_layout.cfg:63 / Java Controller.java:676)。
/// 注: "S." (PowerInfo) 为 Java 原样搬移的死前缀, cfg 无此键族 — 不在本测试面。
#[test]
fn minihud_interest_keys_hit_ui_layout_cfg() {
    let cfg_path =
        locate_template_cfg().expect("仓库模板 ui_layout.cfg 应可达 (上溯三级)");
    let text = std::fs::read_to_string(&cfg_path).unwrap();
    let keys = cfg_target_keys(&text);
    assert!(!keys.is_empty(), "cfg 键空间非空 (解析自检)");
    for p in MINIHUD_INTEREST_KEYS {
        assert!(
            keys.iter().any(|k| k.starts_with(p)),
            "MiniHUD 兴趣键 {p} 应命中 ui_layout.cfg 的 :target 键 (前缀匹配)"
        );
    }
}

/// FocusMonitor 通道桥: coordinator 回调 → UiCommand 命令 + shared 镜像
#[test]
fn focus_bridge_sends_commands_and_mirrors_hidden() {
    use vm_core::focus_monitor::AlwaysOnTopCoordinatorApi as _;
    let (tx, rx) = std::sync::mpsc::channel::<UiCommand>();
    let shared = ControllerShared::default();
    let bridge = ChannelFocusBridge { tx, shared: Arc::new(shared) };
    assert!(!bridge.is_overlays_hidden(), "初始未隐藏");
    bridge.hide_all_overlays();
    assert_eq!(rx.recv_timeout(Duration::from_millis(200)).unwrap(), UiCommand::HideAllOverlays);
    bridge.show_all_overlays();
    assert_eq!(rx.recv_timeout(Duration::from_millis(200)).unwrap(), UiCommand::ShowAllOverlays);
}

/// 位置映射 ↔ ui_layout.cfg panel 标题核对: OVERLAY_SECTIONS 的 section 查不到
/// GroupConfig → group_position 返回 None → 该 overlay 恒居中, 位置持久化静默失效
#[test]
fn overlay_sections_hit_ui_layout_cfg() {
    let cfg_path =
        locate_template_cfg().expect("仓库模板 ui_layout.cfg 应可达 (上溯三级)");
    let text = std::fs::read_to_string(&cfg_path).unwrap();
    // 顶层 panel 标题集 (行首 `(panel "标题"`; GroupConfig.x/y 挂在顶层标题上)
    let mut titles: Vec<&str> = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("(panel \"") {
            if let Some(end) = rest.find('"') {
                titles.push(&rest[..end]);
            }
        }
    }
    assert!(titles.len() >= 6, "cfg 顶层 panel 数量自检 (实得 {})", titles.len());
    for (id, section) in OVERLAY_SECTIONS {
        assert!(
            titles.contains(&section),
            "overlay {id} 的 section {section} 不在 ui_layout.cfg 顶层 panel 标题中 — \
                 位置读写将永远落空"
        );
    }
}
