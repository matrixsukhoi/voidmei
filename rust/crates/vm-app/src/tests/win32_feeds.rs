//! win32 线程/overlay 注册喂入/配置键表校验 (波11 自 tests.rs 分片)

use super::*;

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

/// 注册面: Java registerGameModeOverlays 的 9 个窗口条目全部落位
/// (open_all 默认激活全真 — thrustdFS 的 jetOnly 策略在 ctx 真值下生效;
/// 剩 1 键非窗口备案见 register_live_overlays 头注)
#[test]
fn register_live_overlays_nine_window_entries() {
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
        fm_unpacked: None,
        draw_frame_simpl: None,
    };
    let shell = fixture();
    let lang = Rc::new(Lang::init_lang());
    let params = Rc::new(RefCell::new(vm_overlay::ReinitParams::from(
        &test_overlay_inputs(),
    )));
    register_live_overlays(
        &mut host,
        &mut handles,
        &shell.env,
        &test_overlay_inputs(),
        &params,
        &lang,
        &shell.shared,
        &shell.fm,
        &shell.fm_field_config,
    );
    // 注册面逐窗计数落键: 9 键全部以 0 落位 (present 计数起点)
    let reg_keys: Vec<String> = shell
        .shared
        .overlay_present
        .lock()
        .expect("overlay_present 锁中毒")
        .keys()
        .cloned()
        .collect();
    assert_eq!(reg_keys.len(), 9, "注册落键应恰为 9 键 (实测 {reg_keys:?})");
    // 9 个共享句柄全部登记 (spec 工厂成功)
    assert!(handles.minihud.is_some(), "MiniHUD 句柄");
    assert!(handles.power_info.is_some(), "动力信息句柄");
    assert!(handles.engine_control.is_some(), "引擎控制句柄");
    assert!(handles.flight_info.is_some(), "飞行信息句柄");
    assert!(handles.gear_flaps.is_some(), "起落襟翼句柄");
    assert!(handles.attitude.is_some(), "地平仪句柄");
    assert!(handles.control_surfaces.is_some(), "操纵面句柄");
    assert!(handles.fm_unpacked.is_some(), "FM拆包数据句柄");
    assert!(handles.draw_frame_simpl.is_some(), "推力曲线句柄");
    // 初始形态 = preview (恒可见 + 空面板, Java initPreview; spec 尺寸 = init 几何)
    {
        let fm = handles.fm_unpacked.as_ref().unwrap().borrow();
        assert!(fm.visible && fm.base.is_preview, "preview 形态起步");
        assert_eq!((fm.base.width, fm.base.height), (spec_fm_size(&shell)), "init 几何");
    }
    // 推力曲线: initPreview 形态恒可见 (setBounds 900×500 几何在 vm-overlay
    // draw_frame_simpl/tests.rs 锁定, 此处锁注册面)
    {
        let d = handles.draw_frame_simpl.as_ref().unwrap().borrow();
        assert!(d.is_preview && d.visible && d.should_show(), "preview 形态恒可见");
    }
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
            "enableFMPrint",
            "enablegearAndFlaps",
            "engineInfoSwitch",
            "flightInfoSwitch",
            "thrustdFS",
        ],
        "注册键 10 键中的 9 窗口条目 (Java 键一一对应)"
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
        &Rc::new(RefCell::new(vm_overlay::ReinitParams::from(&inputs))),
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
        &Rc::new(RefCell::new(vm_overlay::ReinitParams::from(&inputs))),
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
    fm_unpacked: None,
    draw_frame_simpl: None,
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
    // W-E 后 warn_vne 只走公式槽 — 槽注入 1.0 作喂通哨 (state 经 guard 直传已由
    // throttle/airbrake 各 handle 断言覆盖)
    {
        let mut slots = std::collections::HashMap::new();
        slots.insert("warn_vne".to_string(), 0u16);
        d.formula_slots = std::sync::Arc::new(slots);
        d.formula_values = vm_core::formula::FormulaResults { values: vec![1.0] };
    }
    let shared = ControllerShared::new();
    shared.overlay_ctx_preview.store(false, Ordering::SeqCst); // 游戏窗口形态
    *shared.live.write().unwrap() = Some(frame_store_of(&d));
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
    // MiniHUD: 公式槽 warn_vne=1.0 → 置真 (喂通回归哨: 槽值经 feed 链到达 HUD)
    assert!(
        handles.minihud.as_ref().unwrap().borrow().warn_vne,
        "MiniHUD 应收到公式槽 warn_vne"
    );

    // preview 门控: overlay_ctx_preview=true → 整帧跳过 (值不推进)
    shared.overlay_ctx_preview.store(true, Ordering::SeqCst);
    {
        // 改源数据后重发布帧 (原 RwLock 直写观测的帧仓等价物)
        d.s_state.as_mut().unwrap().throttle = 99;
        if let Some(store) = shared.live.read().unwrap().as_ref() {
            update_live_frame(store, &d);
        }
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
        &Rc::new(RefCell::new(vm_overlay::ReinitParams::from(&test_overlay_inputs()))),
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
    fm_unpacked: None,
    draw_frame_simpl: None,
    };
    let shared = ControllerShared::new();
    shared.overlay_ctx_preview.store(false, Ordering::SeqCst);
    // State::new() 的 pitch/thrust 空 Vec — get_pitch 的 s.pitch[0] panic (保真)
    *shared.live.write().unwrap() = Some(frame_store_of(&live_service_data("bad")));
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
    use vm_core::platform::focus_monitor::AlwaysOnTopCoordinatorApi as _;
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

/// win32 CloseAllOverlays 数据面重置 (reset_handles_preview_values 接线面):
/// 四个 reinit 闭包不重建数据态的 overlay, live 残留 → preview 静态初值。
/// 语义断言在 vm-overlay 各单测, 此处锁 win32 处理点的调用面 (托盘 live→preview
/// 后重开的预览窗不得显示上次 live 数据 — TODO 项根治的回归面)
#[test]
fn reset_handles_preview_values_clears_live_residue() {
    let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts");
    let cell = Rc::new(RefCell::new(vm_overlay::ReinitParams::default()));
    let (power, _) = vm_overlay::power_info_overlay_spec(&fonts, &cell).unwrap();
    let (flight, _) = vm_overlay::flight_info_overlay_spec(&fonts, &cell).unwrap();
    let (axis, _) = vm_overlay::control_surfaces_overlay_spec(&fonts, &cell).unwrap();
    let (att, _) = vm_overlay::attitude_overlay_spec(&cell).unwrap();
    let (fm_unpacked, _) = vm_overlay::fm_unpacked_data_overlay_spec(
        &fonts,
        1080,
        &cell,
        None,
        &Arc::new(FMManager::new(Arc::new(EventBus::new()))),
    )
    .unwrap();
    let handles = OverlayHandles {
        minihud: None,
        power_info: Some(power),
        engine_control: None,
        gear_flaps: None,
        attitude: Some(att),
        control_surfaces: Some(axis),
        flight_info: Some(flight),
        fm_unpacked: Some(fm_unpacked),
        draw_frame_simpl: None,
    };
    // live 残留注入 (各 handle 公开喂入面)
    {
        let mut cs = handles.control_surfaces.as_ref().unwrap().borrow_mut();
        cs.has_service = true;
        assert!(cs.on_flight_data(200, 100.0, -80.0, 60.0, 40.0, true));
    }
    // FM拆包数据 live 残留: 游戏形态 + 隐藏中 (OpenAll 处理点同款翻转)
    {
        let mut fm = handles.fm_unpacked.as_ref().unwrap().borrow_mut();
        fm.base.is_preview = false;
        fm.visible = false;
    }
    handles
        .attitude
        .as_ref()
        .unwrap()
        .borrow_mut()
        .update_telemetry(10.0, 5.0, -20.0, 30.0, 90.0, Some((20.0, -8.0)));
    let mut v = vm_data::service_fields::ServiceData::default();
    // W-C: 派生量唯一真相 = 公式槽 (mach 经槽 0 注入)
    {
        let mut slots = std::collections::HashMap::new();
        slots.insert("mach".to_string(), 0u16);
        v.formula_slots = std::sync::Arc::new(slots);
        v.formula_values = vm_core::formula::FormulaResults { values: vec![0.72] };
    }
    let v = &v as &dyn vm_core::formula::registry::FormulaView;
    handles
        .flight_info
        .as_ref()
        .unwrap()
        .borrow_mut()
        .update(v);
    handles.power_info.as_ref().unwrap().borrow_mut().last_refresh_time = 999;
    // 重置 (win32 CloseAllOverlays 处理点同款)
    reset_handles_preview_values(&handles);
    // 四路断言: 全部回 preview 态
    {
        let p = handles.power_info.as_ref().unwrap().borrow();
        assert_eq!(p.last_refresh_time, 0, "动力信息节流基准复位");
        assert!(p.fields().iter().all(|f| f.length == 0), "动力信息 buffer 清空");
    }
    {
        let cs = handles.control_surfaces.as_ref().unwrap().borrow();
        assert_eq!(
            (cs.px, cs.py),
            (cs.width / 2, cs.height / 2),
            "舵面值游标回几何中心 (live 位置清除)"
        );
    }
    assert_eq!(
        handles.attitude.as_ref().unwrap().borrow().pitch_y, 0,
        "地平仪姿态点集复位"
    );
    {
        let rows = handles.flight_info.as_ref().unwrap().borrow().rows().to_vec();
        let defs = handles.flight_info.as_ref().unwrap().borrow().defs.clone();
        assert_eq!(rows.len(), defs.len(), "飞行信息回全量行");
        for (row, f) in rows.iter().zip(defs.iter()) {
            assert_eq!(row.2, f.preview_value, "飞行信息值列回 preview 静态: {}", f.label);
        }
    }
    {
        let fm = handles.fm_unpacked.as_ref().unwrap().borrow();
        assert!(fm.visible && fm.base.is_preview, "FM拆包数据回 preview 形态 (恒可见)");
    }
}

// ------------------------------------------------------------------
// FM拆包数据装配面 (P5 组装契约销号: enableFMPrint 注册 + withInterest + 配置快照)
// ------------------------------------------------------------------

/// withInterest 键 ↔ Java Controller.java:739-743 逐字核对 (20 键; 审查 W1
/// 同族回归锚 — 死键 fmInfoColumn 为 Java 原样, 见 const 注)
#[test]
fn fm_unpacked_interest_keys_verbatim_java_controller() {
    assert_eq!(
        FM_UNPACKED_INTEREST_KEYS,
        [
            "displayFmKey",
            "selectedFM",
            "fmInfoColumn",
            "fontName",
            "showWeight",
            "showCritSpeed",
            "showGLoadLimits",
            "showFlapLimits",
            "showControlEffectiveness",
            "showNitro",
            "showHeatRecovery",
            "showMaxLiftLoad",
            "showInertia",
            "showLift",
            "showDrag",
            "showNoFlapsWing",
            "showFullFlapsWing",
            "showFuselage",
            "showFin",
            "showStab",
        ]
    );
}

/// FM show* 配置键快照: 构造期全量落 + CONFIG_CHANGED 逐键同步 (win32 线程
/// generate_lines 的跨线程读面; voice_config 同族)
#[test]
fn fm_field_config_snapshot_syncs_config_changed() {
    let cfg = fixture_cfg(
        "(panel \"T\" :visible true\n\
             \x20 (item \"w\" :type switch :target \"showWeight\" :value true)\n\
             \x20 (item \"fm\" :type switch :target \"enableFMPrint\" :value true)\n\
             \x20 (item \"auto\" :type switch :target \"autoStartGameMode\" :value false))\n\
            ",
    );
    let mut shell = fixture_full(30, cfg);
    // 构造期: 16 键全量落 (无 cfg 项的键 = 空串, isFieldEnabled 空串→默认启用,
    // Java getConfig 返回 null 的对位)
    assert_eq!(shell.fm_field_config.lock().unwrap().len(), FM_FIELD_KEYS.len());
    assert_eq!(
        shell.fm_field_config.lock().unwrap().get("showWeight").map(|s| s.as_str()),
        Some("true"),
        "初始快照应含配置树现值"
    );
    // 发布方写配置树 (set_config 放锁后补发 CONFIG_CHANGED 到桩总线)
    shell
        .controller
        .as_ref()
        .unwrap()
        .config
        .set_config("showWeight", "false");
    pump_events(&mut shell); // Controller 转发 → handle_main_event → 快照同步
    assert_eq!(
        shell.fm_field_config.lock().unwrap().get("showWeight").map(|s| s.as_str()),
        Some("false"),
        "show* 变更应同步进跨线程快照 (generate_lines 读到新值的前提)"
    );
    // 非 show* 键不入快照 (键集封闭; enableFMPrint 走激活缓存)
    shell
        .controller
        .as_ref()
        .unwrap()
        .config
        .set_config("enableFMPrint", "false");
    pump_events(&mut shell);
    assert!(
        !shell.fm_field_config.lock().unwrap().contains_key("enableFMPrint"),
        "enableFMPrint 走激活缓存, 不入 show* 快照"
    );
}

// ------------------------------------------------------------------
// 语音子系统装配: 共享 VoiceResourceManager (Java getInstance() 单例落位)
// ------------------------------------------------------------------
