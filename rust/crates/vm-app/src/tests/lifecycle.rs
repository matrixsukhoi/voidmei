//! shell 生命周期/状态机/监督循环 (波11 自 tests.rs 分片)

use super::*;

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
    let data = live_service_data("test-plane");
    *shell.shared.live.write().unwrap() = Some(frame_store_of(&data));
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
    let mut data = live_service_data("p1");
    let live_store = frame_store_of(&data);
    *shell.shared.live.write().unwrap() = Some(Arc::clone(&live_store));
    shell.controller.as_mut().unwrap().drive_from_live(); // 进 Preview + identify(p1)
    // flags 丢失 (Java Service.java:1780 路径): 仅 flag 翻假, 其余保留
    {
        data.s_state.as_mut().unwrap().flag = false;
        data.s_indic.as_mut().unwrap().flag = false;
        update_live_frame(&live_store, &data);
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
    let data = live_service_data("p1");
    *shell.shared.live.write().unwrap() = Some(frame_store_of(&data));
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
    let mut data = live_service_data("p1");
    *shell.shared.live.write().unwrap() = Some(frame_store_of(&data));
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
    data.player_live = false;
    if let Some(store) = shell.shared.live.read().unwrap().as_ref() {
        update_live_frame(store, &data);
    }
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
    let mut data = live_service_data("a1");
    *shell.shared.live.write().unwrap() = Some(frame_store_of(&data));
    let c = shell.controller.as_mut().unwrap();
    c.drive_from_live();
    assert_eq!(shell.fm.current_target_name().as_deref(), Some("a1"));
    // 同机: 幂等 (目标不变)
    c.on_aircraft_changed(Some("a1"));
    assert_eq!(shell.fm.current_target_name().as_deref(), Some("a1"));
    // 换机: FM 目标切换 (identify), 不重启 Controller (state 保持 Preview)
    data = live_service_data("b2");
    if let Some(store) = shell.shared.live.read().unwrap().as_ref() {
        update_live_frame(store, &data);
    }
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
        // 波4: live 槽 = 真 Service 的帧仓; ServiceAnalyzerSource/open_flight_log
        // 仍读 handle.data (锁面), 手动发一帧驱动 drive 链
        let handle = shell.controller.as_ref().unwrap().service.as_ref().unwrap();
        *shell.shared.live.write().unwrap() = Some(Arc::clone(&handle.frames));
        *handle.data.write().unwrap() = live_service_data("s1");
        let frame = {
            let d = handle.data.read().unwrap();
            vm_data::frame::Frame::from_service_data(&d)
        };
        handle.frames.publish(frame);
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
        let d2 = live_service_data("b2");
        let handle = shell.controller.as_ref().unwrap().service.as_ref().unwrap();
        *handle.data.write().unwrap() = d2;
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
    let data = live_service_data("s1");
    *shell.shared.live.write().unwrap() = Some(frame_store_of(&data));
    shell.pump(); // drive: Service live 数据 → InGame → Preview
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Preview
    );

    let gen_before = shell.shared.preview_generation.load(Ordering::SeqCst);
    // 路由总线按 event_type 计数 (原桩总线广播计数已随波1 退役)
    let cfg_subs_before = shell.ui_bus.subscriber_count(ui_state_events::CONFIG_CHANGED);
    let ready_subs_before = shell.ui_bus.subscriber_count(ui_state_events::UI_READY);
    let fm_subs_before = shell.fm.fm_changed_bus().subscriber_count();
    assert!(cfg_subs_before >= 1, "Controller 应持 CONFIG_CHANGED 订阅");
    assert!(ready_subs_before >= 1, "Controller 应持 UI_READY 订阅");
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
    // ②: 订阅全部退订 (路由后按类型各 -1)
    assert_eq!(
        shell.ui_bus.subscriber_count(ui_state_events::CONFIG_CHANGED),
        cfg_subs_before - 1
    );
    assert_eq!(
        shell.ui_bus.subscriber_count(ui_state_events::UI_READY),
        ready_subs_before - 1
    );
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

/// 托盘 Activate: 旧核 stop (退订) → 新核 (State=INIT, 订阅重建, 无泄漏累积)
#[test]
fn tray_activate_rebuilds_controller() {
    let mut shell = fixture();
    let before = shell.ui_bus.subscriber_count(ui_state_events::CONFIG_CHANGED)
        + shell.ui_bus.subscriber_count(ui_state_events::UI_READY);
    shell.handle_main_event(MainEvent::Tray(TrayCommand::Activate));
    assert_eq!(
        shell.controller.as_ref().unwrap().state(),
        ControllerState::Init,
        "新核从 INIT 开始"
    );
    assert_eq!(
        shell.ui_bus.subscriber_count(ui_state_events::CONFIG_CHANGED)
            + shell.ui_bus.subscriber_count(ui_state_events::UI_READY),
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
    // 用户再点 MainForm "开　始" (叠加态): confirm 链被 service.is_some() 守卫拦下
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
    let data = live_service_data("g1");
    *shell.shared.live.write().unwrap() = Some(frame_store_of(&data));
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

/// autoStartGameMode=true → Controller 自启动分支 (跳过 MainForm, Service 直起;
/// --live/--mock-smoke 的配置注入对位, Controller.java:589-606)
#[test]
fn auto_start_live_skips_main_form() {
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

/// 托盘 About 置关于请求位 (一次性取走; Java:236-245 纯展示动作不重建核 —
/// 组装层主循环据此 emit about-requested 转发前端 Modal)
#[test]
fn tray_about_requests_modal_without_rebuild() {
    let mut shell = fixture();
    shell.handle_main_event(MainEvent::Tray(TrayCommand::About));
    assert!(shell.take_about_request(), "托盘 About 应置请求位");
    assert!(!shell.take_about_request(), "取走后复位 (一次性)");
    assert!(
        !shell.take_form_request(),
        "About 不触发设置窗请求 (与 Activate 分流)"
    );
}

/// 分相监督循环: 退出请求 → Exit (EndGame/托盘 Exit 的相位出口;
/// 阻断自动 spawn 渲染线程 — 纯状态机断言, 不开真窗)
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
