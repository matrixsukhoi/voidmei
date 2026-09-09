//! 渲染线程/overlay 注册喂入/配置键表校验 (波11 自 tests.rs 分片)

use super::*;

/// 渲染线程: 注册不 panic → 刷新命令消费 → Shutdown → join 干净退出
/// (JoinHandle 无泄漏; 真实 Win32 窗口/托盘创建, 无桌面环境时托盘缺失仍可跑)
#[test]
fn render_thread_shutdown_joins_cleanly() {
    let mut shell = fixture();
    shell.spawn_render_thread().expect("渲染线程启动");
    // Preview 态 + 有效世代号 → 全量刷新命令 (守卫放行路径, 不 stale)
    *shell.shared.state.write().unwrap() = ControllerState::Preview;
    let gen = shell.shared.preview_generation.load(Ordering::SeqCst);
    shell.send_ui(UiCommand::RefreshPreviews {
        changed_key: None,
        generation: gen,
    });
    std::thread::sleep(Duration::from_millis(150)); // 泵消费 + 渲染一拍
                                                    // 过期命令: 世代号 +1 → 消费侧丢弃 (守卫路径)
    shell.send_ui(UiCommand::RefreshPreviews {
        changed_key: None,
        generation: gen + 1,
    });
    std::thread::sleep(Duration::from_millis(60));
    shell.send_ui(UiCommand::Shutdown);
    let join = shell.render.take().unwrap();
    assert!(join.join().is_ok(), "渲染线程应干净退出");
    assert!(shell.render.is_none());
}

// ------------------------------------------------------------------
// 组装接线 (本批新增面)
// ------------------------------------------------------------------

/// 渲染线程帧计数: 预览刷新打开 overlay 后 present 帧数递增
/// (--mock-smoke 核心断言的库内等价; 字体目录钉仓库根, 见 fixture 注)
#[test]
fn render_frames_advance_with_active_overlays() {
    let mut shell = fixture();
    shell.spawn_render_thread().expect("渲染线程启动");
    // Preview 态 + 有效世代号 → 全量刷新 (MiniHUD crosshairSwitch=true 激活)
    *shell.shared.state.write().unwrap() = ControllerState::Preview;
    let gen = shell.shared.preview_generation.load(Ordering::SeqCst);
    shell.send_ui(UiCommand::RefreshPreviews {
        changed_key: None,
        generation: gen,
    });
    // 轮询等 present 帧 (固定 sleep 在并发负载下不够 → flaky; 上限 10s)
    let frames = poll_until(Duration::from_secs(10), || {
        shell.shared.render_frames.load(Ordering::SeqCst)
    });
    shell.send_ui(UiCommand::Shutdown);
    let join = shell.render.take().unwrap();
    assert!(join.join().is_ok());
    assert!(
        frames > 0,
        "活跃 overlay 的 present 帧数应递增 (实测 {frames})"
    );
}

/// 逐 overlay present 计数: 游戏模式全开 (open_all) 后 6 注册键全部 present>0
/// (--mock-smoke 断言 3 的库内等价; 字体目录钉仓库根, 见 fixture 注)
#[test]
fn render_overlay_present_counts_per_registered_overlay() {
    let all_on_cfg = vec![tpanel(
        "T",
        vec![
            trow("crosshairSwitch", true),
            trow("engineInfoSwitch", true),
            trow("enableEngineControl", true),
            trow("enablegearAndFlaps", true),
            trow("enableAxis", true),
            trow("enableAttitudeIndicator", true),
        ],
    )];
    let mut shell = fixture_full(30, all_on_cfg);
    shell.spawn_render_thread().expect("渲染线程启动");
    shell.send_ui(UiCommand::OpenAllOverlays);
    // 轮询等 6 窗全部 present (固定 sleep 在并发负载下不够 → flaky; 上限 10s)
    let wanted = [
        "enableEngineControl",
        "engineInfoSwitch",
        "crosshairSwitch",
        "enablegearAndFlaps",
        "enableAxis",
        "enableAttitudeIndicator",
    ];
    poll_until(Duration::from_secs(10), || {
        let counts = shell
            .shared
            .overlay_present
            .lock()
            .expect("overlay_present 锁中毒")
            .clone();
        let min = wanted
            .iter()
            .map(|id| counts.get(*id).copied().unwrap_or(0))
            .min()
            .unwrap_or(0);
        min
    });
    let counts = shell
        .shared
        .overlay_present
        .lock()
        .expect("overlay_present 锁中毒")
        .clone();
    shell.send_ui(UiCommand::Shutdown);
    let join = shell.render.take().unwrap();
    assert!(join.join().is_ok());
    for id in wanted {
        let c = counts.get(id).copied().unwrap_or(0);
        assert!(
            c > 0,
            "overlay {id} present 应 >0 (实测 {c}, 全量 {counts:?})"
        );
    }
}

/// 轮询直到样本 >0 或超时 (返回最后样本) — 渲染节拍类断言的负载无关等待
fn poll_until(timeout: Duration, mut sample: impl FnMut() -> u64) -> u64 {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let v = sample();
        if v > 0 || std::time::Instant::now() >= deadline {
            return v;
        }
        std::thread::sleep(Duration::from_millis(100));
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
        pages: Vec::new(),
    };
    let shell = fixture();
    let lang = Rc::new(Lang::init_lang());
    let inputs = test_overlay_inputs();
    let params = Rc::new(RefCell::new(
        vm_overlay::platform::reinit::ReinitParams::from(&inputs),
    ));
    register_live_overlays(
        &mut host,
        &mut handles,
        &OverlayRegSetup {
            env: &shell.env,
            inputs: &inputs,
            params: &params,
            lang: &lang,
            shared: &shell.shared,
        },
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
    // 9 个共享句柄全部登记 (spec 工厂成功): minihud + 6 通用页 + fm 两旧形态
    assert!(handles.minihud.is_some(), "MiniHUD 句柄");
    assert_eq!(
        handles.pages.len(),
        8,
        "W3 八页句柄 (实测 {:?})",
        handles.pages.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>()
    );

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

/// W3 测试页面句柄集: 出厂 PageDoc (minihud 除外) → PageOverlay 编排器
/// (参数组装 = render_thread::assemble_page_spec 的简化形态;
/// gauge 相关组件参数当前生产走 GaugeCfg::default — page_overlay build_page
/// 的 FactoryCtx.gauge_cfg=None 收口形态, 测试同源)
fn test_pages(
    fonts: &Path,
    inputs: &OverlayInputs,
) -> Vec<(String, vm_overlay::widgets::PageHandle)> {
    use vm_overlay::widgets::{page_overlay_spec, GaugeCfg, PageSpecParams};

    let mut pages = Vec::new();
    for doc in inputs.pages.iter() {
        if doc.id == "minihud-default" {
            continue; // minihud 走编排器 spec (测试单独构造)
        }
        let params = PageSpecParams {
            doc: doc.clone(),
            entry_key: doc.entry_key.clone(),
            font_path: fonts.join("sarasa-mono-sc-bold.ttf"),
            font_size: 24,
            lang: Lang::init_lang(),
            settings: inputs.hud.clone(),
            debug: false,
            gauge_cfg: GaugeCfg::default(),
            refresh: Box::new(|| unreachable!("测试不触发 reinit")),
        };
        let (handle, _) = page_overlay_spec(params).expect("页面 spec 构造");
        pages.push((doc.id.clone(), handle));
    }
    pages
}

/// live 喂数全链: 一帧 payload 喂全部页面, 各组件 state 推进到遥测值
/// (W3 改写: 旧六段 spec 工厂直调断言 → pages + cells downcast 组件 state;
/// ServiceData 的引擎数组必须非空 — get_pitch/get_thrust 的保真 panic 点,
/// 真实链路由 State.update 填满; catch_unwind 吞帧路径由 malformed 变体覆盖)
#[test]
fn feed_overlays_live_updates_all_handles() {
    let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts");
    let lang = Rc::new(Lang::init_lang());
    let inputs = test_overlay_inputs();
    let (h_mini, _) = vm_overlay::overlays::minihud::minihud_overlay_spec(
        false,
        50,
        &inputs.hud,
        1.0,
        &fonts.join("sarasa-mono-sc-bold.ttf"),
        &Rc::new(RefCell::new(vm_overlay::platform::reinit::ReinitParams {
            pages: vm_core::config::json_store::factory_pages_arc(),
            ..Default::default()
        })),
    )
    .unwrap();
    let pages = test_pages(&fonts, &inputs);
    assert_eq!(pages.len(), 8, "W3 八页 (6 通用页 + fm 两页)");
    let handles = OverlayHandles {
        minihud: Some(h_mini),
        pages,
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
        st.pitch = [0.0; 16];
        st.thrust = [0; 16];
        st.power = [0.0; 16];
        st.efficiency = [0.0; 16];
        st.throttles = [0; 16];
        st.rpm_throttle = 60;
    }
    d.s_indic.as_mut().unwrap().aviahorizon_pitch = 5.0;
    d.engine.total_hp = 1200;
    // W-E 后 warn_vne 只走公式槽 — 槽注入 1.0 作喂通哨 (state 经 guard 直传已由
    // throttle/airbrake 各组件断言覆盖)
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

    feed_overlays_live(
        &handles,
        &payload,
        &shared,
        &fm,
        &settings,
        &lang,
    );

    // 页句柄定位助手 (借用期内完成组件 state 断言)
    let page_of = |handles: &OverlayHandles, id: &str| {
        handles
            .pages
            .iter()
            .find(|(pid, _)| pid == id)
            .map(|(_, h)| h.clone())
            .unwrap_or_else(|| panic!("页面 {id} 应在 pages"))
    };
    use vm_overlay::widgets::axes_atom::RudderBarWidget;
    use vm_overlay::widgets::data_field::DataFieldWidget;
    use vm_overlay::widgets::engine_gauge::EngineGaugeWidget;
    use vm_overlay::widgets::gauges_composite::{AttitudeWidget, AxesWidget};
    use vm_overlay::widgets::gear_flaps_atom::{FlapBarWidget, GearWarnWidget};

    // 动力信息页: 功率 1200 → horse_power 原子字段值文本 (无节流 — 组件每帧)
    {
        let h = page_of(&handles, "power-info-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("horse_power")
            .unwrap()
            .downcast_ref::<DataFieldWidget>()
            .expect("power 页 horse_power = DataFieldWidget");
        assert_eq!(w.value_text(), "1200", "PowerInfo 功率字段");
    }
    // 引擎控制页: throttle 55 (refreshInterval = 50×2 = 100, 首帧放行;
    // 原子仪表组件 — 面板拆解后逐仪表断言)
    {
        let h = page_of(&handles, "engine-control-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("throttle")
            .unwrap()
            .downcast_ref::<EngineGaugeWidget>()
            .expect("engine 页 throttle = EngineGaugeWidget");
        assert_eq!(w.cur_value(), 55);
    }
    // 起落襟翼页 (拆解后两原子组件): gear=100 + airbrake=100 → "起落架 减速板"
    // 告警; flaps=25 → flap_pix (fontAdd 0/dpi 1 → fs=24, barHeight=96,
    // 25·96/100 = 24)
    {
        let h = page_of(&handles, "gear-flaps-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("warn")
            .unwrap()
            .downcast_ref::<GearWarnWidget>()
            .expect("gear 页 warn = GearWarnWidget");
        assert_eq!(w.warn_text(), "起落架 减速板");
        let b = page
            .cells
            .get("flapbar")
            .unwrap()
            .downcast_ref::<FlapBarWidget>()
            .expect("gear 页 flapbar = FlapBarWidget");
        assert_eq!(b.flap_pix(), 24);
        assert_eq!(b.flap_text(), " 25");
    }
    // 操纵面页: aileron=100 → 十字 px = (100+100)·144/200 = 144; rudder 未设
    // (默认 0) → 游标中位 72; elevator 行字段 0 (frame 在场 = live 形态)
    {
        let h = page_of(&handles, "axis-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("cross")
            .unwrap()
            .downcast_ref::<AxesWidget>()
            .expect("axis 页 cross = AxesWidget");
        assert_eq!(w.state().px, 144);
        let r = page
            .cells
            .get("rudderbar")
            .unwrap()
            .downcast_ref::<RudderBarWidget>()
            .expect("axis 页 rudderbar = RudderBarWidget");
        assert_eq!(r.rudder_pix(), 72);
        let e = page
            .cells
            .get("elevator")
            .unwrap()
            .downcast_ref::<DataFieldWidget>()
            .expect("axis 页 elevator = DataFieldWidget");
        assert_eq!(e.value_text(), "0");
    }
    // 地平仪页: aoa=10 → AoA = round((10+30)·300/60) = 200 (默认几何 150×300)
    {
        let h = page_of(&handles, "attitude-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("gauge")
            .unwrap()
            .downcast_ref::<AttitudeWidget>()
            .expect("attitude 页 gauge = AttitudeWidget");
        assert_eq!(w.state().aoa_y, 200);
        // 40ms 节流闩 (组件化): last_ms 已推进, 窗口内第二帧不重算
        assert!(w.last_ms() > 0);
    }
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
    feed_overlays_live(
        &handles,
        &payload,
        &shared,
        &fm,
        &settings,
        &lang,
    );
    {
        let h = page_of(&handles, "engine-control-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("throttle")
            .unwrap()
            .downcast_ref::<EngineGaugeWidget>()
            .unwrap();
        assert_eq!(
            w.cur_value(),
            55,
            "preview 期不喂入 (Java initPreview 不订阅)"
        );
    }
}

/// 畸形 s_state (引擎数组空, update 未跑) 的保真 panic 点: catch_unwind 吞帧不杀线程
/// (W3 改写: 8 页组件全在场 — 比"PowerInfo 句柄悬空"的旧形态覆盖更宽的
/// 组件消费面, 断言面不变: 不 panic 即通过)
#[test]
fn feed_overlays_live_swallows_malformed_frame() {
    let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts");
    let lang = Lang::init_lang();
    let inputs = test_overlay_inputs();
    let handles = OverlayHandles {
        minihud: None,
        pages: test_pages(&fonts, &inputs),
    };
    let shared = ControllerShared::new();
    shared.overlay_ctx_preview.store(false, Ordering::SeqCst);
    // State::new() 的 pitch/thrust 空 Vec — 取数链的保真 panic 点
    *shared.live.write().unwrap() = Some(frame_store_of(&live_service_data("bad")));
    let fm = FMManager::new(Arc::new(EventBus::new()));
    let settings = inputs.hud;
    let payload = EventPayload::builder().build();
    // 不 panic 即通过 (吞帧 + ERROR 留痕; Java NPE 由 EDT 吞的同位形态)
    feed_overlays_live(
        &handles,
        &payload,
        &shared,
        &fm,
        &settings,
        &lang,
    );
}

/// 注册键 ↔ ui_layout.cfg 核对: 9 个激活键 (ACTIVATION_KEYS) 全部以 panel switch
/// 形式存在 (Java 端第 10 键 thrustdFS 无 cfg 项 — 策略读 enableFMPrint,
/// DrawFrameSimpl 无独立开关, Java 同形态)
#[test]
fn activation_keys_match_factory_default() {
    let keys = factory_target_keys();
    for key in ACTIVATION_KEYS {
        assert!(
            keys.iter().any(|k| k == key),
            "激活键 {key} 应以行绑定键存在于 factory_default.json"
        );
    }
    // 6 个窗口条目键 (注册面) 与出厂面板一一对应 (thrustdFS 例外 — 策略读
    // enableFMPrint, 无独立开关)
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
        assert!(keys.iter().any(|k| k == key), "键 {key} 缺失");
    }
}

/// 出厂默认树的全量行绑定键 (含 HEADER 嵌套)
fn factory_target_keys() -> Vec<String> {
    fn walk(rows: &[vm_core::config::json_model::RowConfig], out: &mut Vec<String>) {
        for r in rows {
            if let Some(p) = &r.property {
                out.push(p.clone());
            }
            walk(&r.children, out);
        }
    }
    let mut out = Vec::new();
    for p in &vm_core::config::json_store::factory_default().panels {
        walk(&p.rows, &mut out);
    }
    out
}

/// MiniHUD 兴趣键 ↔ ui_layout.cfg 键空间核对 (审查 W1 回归锚): with_interest
/// 为前缀匹配 (host is_interested_in), 死键不命中任何 cfg 键 → WYSIWYG 开关
/// 切换时 MiniHUD 不刷新 (Java 会刷新)。曾笔误 "showAttitudeIndicator"
/// (正确键 showAttitudeGauge, ui_layout.cfg:63 / Java Controller.java:676)。
/// 注: "S." (PowerInfo) 为 Java 原样搬移的死前缀, cfg 无此键族 — 不在本测试面。
#[test]
fn minihud_interest_keys_hit_factory_default() {
    let keys = factory_target_keys();
    assert!(!keys.is_empty(), "出厂键空间非空 (解析自检)");
    for p in MINIHUD_INTEREST_KEYS {
        assert!(
            keys.iter().any(|k| k.starts_with(p)),
            "MiniHUD 兴趣键 {p} 应命中 factory_default.json 的行绑定键 (前缀匹配)"
        );
    }
}

/// FocusMonitor 通道桥: coordinator 回调 → UiCommand 命令 + shared 镜像
#[test]
fn focus_bridge_sends_commands_and_mirrors_hidden() {
    use vm_core::platform::focus_monitor::AlwaysOnTopCoordinatorApi as _;
    let (tx, rx) = std::sync::mpsc::channel::<UiCommand>();
    let shared = ControllerShared::default();
    let bridge = ChannelFocusBridge {
        tx,
        shared: Arc::new(shared),
    };
    assert!(!bridge.is_overlays_hidden(), "初始未隐藏");
    bridge.hide_all_overlays();
    assert_eq!(
        rx.recv_timeout(Duration::from_millis(200)).unwrap(),
        UiCommand::HideAllOverlays
    );
    bridge.show_all_overlays();
    assert_eq!(
        rx.recv_timeout(Duration::from_millis(200)).unwrap(),
        UiCommand::ShowAllOverlays
    );
}

/// 出厂页位置自检: 每页 pos 有值 (R2: 位置真源 = PageDoc.pos, host 条目键
/// 由 host_key() 派生 — 快照/落盘链两端同一式)
#[test]
fn factory_pages_host_keys_unique() {
    let factory = vm_core::config::json_store::factory_default();
    let keys: Vec<String> = factory.pages.iter().map(|p| p.host_key()).collect();
    let uniq: std::collections::HashSet<&String> = keys.iter().collect();
    assert_eq!(keys.len(), uniq.len(), "出厂页 host 键不得重复 (位置档将撞键)");
    assert!(keys.contains(&"crosshairSwitch".to_string()));
    assert!(keys.contains(&"thrustdFS".to_string()));
}

/// 渲染线程 CloseAllOverlays 数据面重置 (reset_handles_preview_values 接线面):
/// 组件 reset_preview 的页面级接线 — live 残留 → preview 静态初值。
/// (W3 改写: 旧 handle 直灌 → pages 构造 + UpdateEnv/sidecar 喂 live 残留,
/// 重置后经组件 downcast 断言回 preview 态; 托盘 live→preview 后重开的预览窗
/// 不得显示上次 live 数据 — TODO 项根治的回归面)
#[test]
fn reset_handles_preview_values_clears_live_residue() {
    let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts");
    let lang = Lang::init_lang();
    let inputs = test_overlay_inputs();
    let handles = OverlayHandles {
        minihud: None,
        pages: test_pages(&fonts, &inputs),
    };
    let page_of = |id: &str| {
        handles
            .pages
            .iter()
            .find(|(pid, _)| pid == id)
            .map(|(_, h)| h.clone())
            .unwrap_or_else(|| panic!("页面 {id} 应在 pages"))
    };
    use vm_overlay::widgets::data_field::DataFieldWidget;
    use vm_overlay::widgets::fm_field::FmFieldWidget;
    use vm_overlay::widgets::gauges_composite::{AttitudeWidget, AxesWidget};

    // ---- live 残留注入 (喂入面与生产同源: 通用页 UpdateEnv / fm 页 sidecar tick) ----
    let mut d = live_service_data("residue-plane");
    {
        let st = d.s_state.as_mut().unwrap();
        st.aileron = 100; // 操纵面: 游标偏离中心
        st.aoa = 10.0; // 地平仪: 姿态点集非零
        st.pitch = [0.0; 16];
        st.thrust = [0; 16];
    }
    d.engine.total_hp = 1200; // 动力信息: buffer 进 live 值 + 节流基准推进
    // W-C: 派生量唯一真相 = 公式槽 (mach 经槽 0 注入 → 飞行信息行进 live 值)
    {
        let mut slots = std::collections::HashMap::new();
        slots.insert("mach".to_string(), 0u16);
        d.formula_slots = std::sync::Arc::new(slots);
        d.formula_values = vm_core::formula::FormulaResults { values: vec![0.72] };
    }
    let frame = vm_data::frame::Frame::from_service_data(&d);
    let payload = EventPayload::builder().build();
    let empty_data = vm_core::derived::hud_data::HUDData::empty();
    let env = vm_overlay::widgets::UpdateEnv {
        data: &empty_data,
        frame: Some(&frame),
        fmdata: None,
        payload: Some(&payload),
        compressor_stages: None,
        now_ms: 10_000,
        maneuver_len: 0,
        maneuver_ticks: Default::default(),
        lang: Some(&lang),
    };
    for (_, page) in &handles.pages {
        page.borrow_mut().feed(&env);
    }
    // fm-list live 残留: live 帧 + 无 FM → 字段行归零 (渲染线程 sidecar 节拍
    // 同款 tick; 原子字段面 — fm.list 黑盒已退役)
    {
        let fm_mgr = FMManager::new(Arc::new(EventBus::new()));
        let h = page_of("fm-list-default");
        let page = h.borrow();
        let cell = page
            .cells
            .get("weight_empty")
            .expect("fm-list 页 weight_empty 组件");
        let mut sctx = vm_overlay::widgets::SidecarCtx {
            now_ms: 10_000,
            page_id: "fm-list-default",
            fm: &fm_mgr,
            fm_field_config: &|_| None,
            display_fm_key: 0,
            frame: Some(&frame),
            is_jet: false,
            toggle_pulse: false,
            game_mode_pulse: true,
            fm_changed: None,
        };
        let mut sc = cell.sidecar().expect("fm.field sidecar 面");
        sc.tick(&mut sctx);
        drop(sc); // RefMut 守卫先放, 再借 downcast 断言面
        let w = cell
            .downcast_ref::<FmFieldWidget>()
            .expect("fm.field 具体类型");
        assert!(!w.shown(), "live 残留: 无 FM → 字段行已归零");
    }
    // 残留到位自检 (注入确实生效 — 否则后续复位断言平凡通过)
    {
        let h = page_of("axis-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("cross")
            .unwrap()
            .downcast_ref::<AxesWidget>()
            .unwrap();
        assert_eq!(w.state().px, 144, "live 残留: 游标已偏离中心");
    }
    {
        // thrust: live 数组 [0;16] → 值文本 "0" (preview 为 "1000", 可区分)
        let h = page_of("power-info-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("thrust")
            .unwrap()
            .downcast_ref::<DataFieldWidget>()
            .unwrap();
        assert_eq!(w.value_text(), "0", "live 残留: thrust 已进 live 值");
    }

    // 重置 (渲染线程 CloseAllOverlays 处理点同款)
    reset_handles_preview_values(&handles);
    // 五路断言: 全部回 preview 态
    {
        let h = page_of("power-info-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("thrust")
            .unwrap()
            .downcast_ref::<DataFieldWidget>()
            .unwrap();
        assert_eq!(w.value_text(), "1000", "动力信息回 preview 静态值");
    }
    {
        let h = page_of("axis-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("cross")
            .unwrap()
            .downcast_ref::<AxesWidget>()
            .unwrap();
        let cs = w.state();
        assert_eq!(
            (cs.px, cs.py),
            (cs.width / 2, cs.height / 2),
            "舵面值游标回几何中心 (live 位置清除)"
        );
    }
    {
        let h = page_of("attitude-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("gauge")
            .unwrap()
            .downcast_ref::<AttitudeWidget>()
            .unwrap();
        assert_eq!(w.state().pitch_y, 0, "地平仪姿态点集复位");
    }
    {
        // mach: live 槽 0.72 → 复位回 preview "0.45" (原子字段; 槽注入链的复位面)
        let h = page_of("flight-info-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("mach")
            .unwrap()
            .downcast_ref::<DataFieldWidget>()
            .unwrap();
        assert_eq!(w.value_text(), "0.45", "飞行信息回 preview 静态值");
    }
    {
        let h = page_of("fm-list-default");
        let page = h.borrow();
        let w = page
            .cells
            .get("weight_empty")
            .unwrap()
            .downcast_ref::<FmFieldWidget>()
            .unwrap();
        assert!(
            w.shown() && w.text() == "3050.0",
            "FM拆包字段回 preview 静态值"
        );
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

/// FM show* 配置键快照: 构造期全量落 + CONFIG_CHANGED 逐键同步 (渲染线程
/// generate_lines 的跨线程读面; voice_config 同族)
#[test]
fn fm_field_config_snapshot_syncs_config_changed() {
    let cfg = vec![tpanel(
        "T",
        vec![
            trow("showWeight", true),
            trow("enableFMPrint", true),
            trow("autoStartGameMode", false),
        ],
    )];
    let mut shell = fixture_full(30, cfg);
    // 构造期: 16 键全量落 (无 cfg 项的键 = 空串, isFieldEnabled 空串→默认启用,
    // Java getConfig 返回 null 的对位)
    assert_eq!(
        shell.config_snapshots.fm_field.lock().unwrap().len(),
        FM_FIELD_KEYS.len()
    );
    assert_eq!(
        shell
            .config_snapshots
            .fm_field
            .lock()
            .unwrap()
            .get("showWeight")
            .map(|s| s.as_str()),
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
        shell
            .config_snapshots
            .fm_field
            .lock()
            .unwrap()
            .get("showWeight")
            .map(|s| s.as_str()),
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
        !shell
            .config_snapshots
            .fm_field
            .lock()
            .unwrap()
            .contains_key("enableFMPrint"),
        "enableFMPrint 走激活缓存, 不入 show* 快照"
    );
}

// ------------------------------------------------------------------
// 语音子系统装配: 共享 VoiceResourceManager (Java getInstance() 单例落位)
// ------------------------------------------------------------------
