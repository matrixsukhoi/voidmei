//! 语音子系统装配与会话 (波11 自 tests.rs 分片; 含 Counting* 测试替身)

use super::*;

/// Java 静态 final INSTANCE 语义: 托盘重建核后管理器**不**随核销毁重建
/// (跨核存活的共享实例; 表单 IPC/告警线程复用同一 Arc)
#[test]
fn voice_共享实例跨核重建不变() {
    let mut shell = fixture();
    let before = Arc::clone(&shell.voice);
    shell.rebuild_controller(false); // 托盘 Activate 路径
    assert!(
        Arc::ptr_eq(&before, &shell.voice),
        "VoiceResourceManager 应跨核重建存活 (Java getInstance 单例语义)"
    );
}

/// 音量同步写点: loadAppCheck 读 cfg voiceVolume → Application.voiceVolumn 的
/// 消费面收敛 (app_shell.rs load_from_config 的 PORT 注)
#[test]
fn voice_volume_经load_from_config同步进管理器() {
    let cfg = fixture_cfg(
        "(panel \"T\" :visible true\n\
             \x20 (item \"vol\" :type slider :target \"voiceVolume\" :min 0 :max 200 :value 42)\n\
             \x20 (item \"auto\" :type switch :target \"autoStartGameMode\" :value false))\n\
            ",
    );
    let shell = fixture_full(30, cfg);
    assert_eq!(
        shell.voice.voice_volumn(),
        42,
        "cfg voiceVolume=42 应经 Controller 构造链同步进共享管理器"
    );
}

/// 共享管理器错误路径 (真实播放器, 文件解析失败面 — 不触音频设备):
/// Java loadClip 的 catch→null 语义 (resolve 失败先于 open_clip, 音频会话无关)
#[test]
fn voice_共享管理器_缺失告警文件_load_clip_返_none() {
    let shell = fixture();
    assert!(
        shell
            .voice
            .load_clip("no_such_warning_zz", Some("default"))
            .is_none(),
        "缺失 wav 必须返 None (Java catch→null), 不得假成功"
    );
}

// ------------------------------------------------------------------
// VoiceWarning 装配 (Java Controller.java:716-723 → OverlayManager.java:294-312
// 的 open→init(this,S)→new Thread().start() 链; 批1 审查 A-B1 收口)
// ------------------------------------------------------------------

/// 游戏模式会话时序核查点: open_voice_warning (live 在位) → 告警线程跑 100ms
/// tick → xS.fatalWarn 有人写 (Java VoiceWarning.run() 的唯一外显副作用);
/// stop (Java OverlayEntry.close 的 interrupt 形态) 后线程退出。
/// CWD 无 voice/ 目录 → start1/告警 wav 全部 load 失败 → available=false →
/// 无声 (不触音频设备, 无声卡环境安全)。
/// tick 信号修正 (假通过根治): ServiceData::default 的 fatal_warn 初值即
/// Some(false) (Java `= false` 初始化器), is_some() 从 t=0 恒真 — 曾令本
/// 测试的等待环 0ms 空转后假过; 改为放置起落架超速遥测 (gear=100, IAS=500
/// ≥ 默认限速 450) 令 checkGearWarning 置 fatal, 轮询 Some(true) 才是真
/// "至少一轮 tick 已跑" 的信号
#[test]
fn voice_warning_游戏模式会话_启动tick写fatal_warn并停机() {
    let shell = fixture();
    // openpad 前提: start() 已建 live (生产时序); fixture 手塞 (先例);
    // 遥测含致命告警形态 (gear 放下 + 超速) — 见函数头 tick 信号修正注
    let mut sd = live_service_data("spitfire");
    sd.s_state.as_mut().unwrap().ias = 500;
    sd.s_state.as_mut().unwrap().gear = 100;
    let data = frame_store_of(&sd);
    let mut session = open_voice_warning(
        &shell.voice,
        &shell.ui_bus,
        &shell.config_snapshots.voice,
        &shell.fm,
        &shell.flight_bus,
        Some(Arc::clone(&data)),
    )
    .expect("live 在位时应启动会话 (Java init(S) 非短路)");
    // run(): 启动延迟 1s + 100ms tick; 轮询等待 (固定 sleep 的调度余量坑,
    // voice_warning/tests.rs 同款修法), 超时即失败 — 不假通过
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if data.fatal_warn() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "8s 内至少一轮 tick 应写 fatalWarn=true (线程未跑 = 装配失败)"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    session.stop();
    assert!(!session.doit.load(Ordering::SeqCst), "停机后 doit 应为 false");
    session.stop(); // 幂等 (Drop 兜底同款)
}

/// live 缺失 (Java init(S=null) 的 doit=false 短路): 不起线程
#[test]
fn voice_warning_live缺失_不起会话() {
    let shell = fixture();
    assert!(
        open_voice_warning(
            &shell.voice,
            &shell.ui_bus,
            &shell.config_snapshots.voice,
            &shell.fm,
            &shell.flight_bus,
            None,
        )
        .is_none(),
        "live=None 应返 None (Java init(null) 短路形态)"
    );
}

/// 激活策略 (config("enableVoiceWarn") + live_only): cfg 开关 + 会话窗口形态
/// 双门控 — openpad (preview=false) 且 cfg true 才激活; 生产消费点 = win32 线程
/// OpenAllOverlays 命令处理 (同 host 窗口条目同源探测)
#[test]
fn voice_warning_激活判定_配置开关与live门控() {
    let cfg = fixture_cfg(
        "(panel \"T\" :visible true\n\
             \x20 (item \"vw\" :type switch :target \"enableVoiceWarn\" :value true)\n\
             \x20 (item \"auto\" :type switch :target \"autoStartGameMode\" :value false))\n\
            ",
    );
    let mut shell = fixture_full(30, cfg);
    let mk_ctx = |shell: &AppShell| HostActivationCtx {
        activation: Arc::clone(&shell.activation),
        fm: Arc::clone(&shell.fm),
        shared: Arc::clone(&shell.shared),
        debug: false,
    };
    // cfg=true + 预览态 (CloseAll/重建核初值) → live_only 拦截
    assert!(
        !strategy_for("enableVoiceWarn").should_activate(&mk_ctx(&shell)),
        "预览态 (overlay_ctx_preview=true) 不得激活 (Java gameModeOnly)"
    );
    // openpad: 会话窗口形态翻 false (forGameMode ctx) → 激活
    shell.shared.overlay_ctx_preview.store(false, Ordering::SeqCst);
    assert!(
        strategy_for("enableVoiceWarn").should_activate(&mk_ctx(&shell)),
        "cfg=true + 游戏模式应激活"
    );
    // 游戏模式但 cfg 改关 (WYSIWYG): 经 CONFIG_CHANGED 链刷新激活缓存后拦截
    shell
        .controller
        .as_ref()
        .unwrap()
        .config
        .set_config("enableVoiceWarn", "false");
    pump_events(&mut shell);
    assert!(
        !strategy_for("enableVoiceWarn").should_activate(&mk_ctx(&shell)),
        "cfg 改关后不得激活 (激活缓存应已刷新)"
    );
}

/// configHandler 触发链 (重构波1): 配置写值 → write_hook 广播前直写快照 →
/// CONFIG_CHANGED 直达统一路由总线 (VoiceWarning 订阅面; 原转发桥退役)
#[test]
fn voice_config_变更同步快照并直达总线() {
    let cfg = fixture_cfg(
        "(panel \"T\" :visible true\n\
             \x20 (item \"vw\" :type combo :target \"voice_aoaCrit\" :value \"default|false\")\n\
             \x20 (item \"auto\" :type switch :target \"autoStartGameMode\" :value false))\n\
            ",
    );
    let mut shell = fixture_full(30, cfg);
    // 快照初值 = 配置树当前值 (with_parts 全量填充)
    assert_eq!(
        shell
            .config_snapshots
            .voice
            .lock()
            .unwrap()
            .get("voice_aoaCrit")
            .map(|s| s.as_str()),
        Some("default|false"),
        "初始快照应含配置树现值"
    );
    // 探针挂统一总线 (Java VoiceWarning.configHandler 的送达面)
    let hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let h2 = Arc::clone(&hits);
    let sub = shell.ui_bus.subscribe(
        ui_state_events::CONFIG_CHANGED,
        move |msg: &vm_core::base::bus::ui_state_bus::UiStateEvent| {
            if msg.data.as_deref() == Some("voice_aoaCrit") {
                h2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        },
    );
    // 发布方写配置树: write_hook 同步直写快照 (广播前) → publish 直达订阅者
    shell
        .controller
        .as_ref()
        .unwrap()
        .config
        .set_config("voice_aoaCrit", "default|true");
    // 快照在 publish 栈内已被 VoiceWarning reload 可见 — 此刻即新值, 无需 pump
    assert_eq!(
        shell
            .config_snapshots
            .voice
            .lock()
            .unwrap()
            .get("voice_aoaCrit")
            .map(|s| s.as_str()),
        Some("default|true"),
        "voice_* 变更应经 write_hook 同步进快照 (reload 链读到新值的前提)"
    );
    assert_eq!(
        hits.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "统一总线应收到恰好一次 CONFIG_CHANGED(voice_aoaCrit)"
    );
    pump_events(&mut shell); // Controller 转发链照常 (无额外快照补写面)
    drop(sub);
}

// ------------------------------------------------------------------
// VoiceWarning 装配面验收 (任务书: mock SoundPlayer 计数断言 + 订阅生命周期)
// "=false 不建" 的判定点 (=OpenAllOverlays 处理器消费的 strategy_for 门)
// 由 voice_warning_激活判定_配置开关与live门控 覆盖, 此处钉会话级全链
// ------------------------------------------------------------------

impl vm_core::audio::voice_resource_manager::SoundPlayer for CountingPlayer {
    fn open_clip(
        &self,
        path: &std::path::Path,
    ) -> Result<
        Box<dyn vm_core::audio::voice_resource_manager::SoundClip>,
        vm_core::audio::voice_resource_manager::SoundError,
    > {
        let key = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Box::new(CountingClip {
            key,
            plays: Arc::clone(&self.plays),
        }))
    }
}

struct CountingClip {
    key: String,
    plays: Arc<Mutex<HashMap<String, usize>>>,
}

impl vm_core::audio::voice_resource_manager::SoundClip for CountingClip {
    fn start(&self) {
        *self
            .plays
            .lock()
            .unwrap()
            .entry(self.key.clone())
            .or_insert(0) += 1;
    }
    fn stop(&self) {}
    fn is_running(&self) -> bool {
        false // 恒停: 冷却由时间项压制 (本测试 current_time_ms 恒 0)
    }
    fn set_frame_position(&self, _frame: i32) {}
    fn close(&self) {}
    fn master_gain_range(&self) -> Option<(f32, f32)> {
        None // Control not supported → applyVolume 跳过 (Java 空 catch 面)
    }
    fn set_master_gain(&self, _value: f32) {}
}

/// 装配面全链 (mock SoundPlayer, 不触音频设备):
/// 1) enableVoiceWarn=true 会话建 → FlightDataBus 订阅在 (+1) +
///    统一总线 configHandler 在 (发布送达 = Controller 常驻 1 + configHandler 1);
/// 2) 告警键触发 play 路径: playerLive+IAS 200+AoA 20 (>默认线 15-1) →
///    aoaCrit start 计 1, init 的 start1 计 1;
/// 3) stop (Java OverlayEntry.close) 后无僵尸订阅: 总线计数回基线 +
///    configHandler 送达 0 (Java 泄漏点的根治面)。
#[test]
fn voice_warning_装配面_播放计数与订阅生命周期() {
    let shell = fixture();
    // tmp voice 目录 + 两个告警 wav (内容任意 — mock 播放器不解析,
    // 仅供 resolve_audio_file 的 exists 探测命中)
    let dir = std::env::temp_dir().join(format!("vm_app_voice_assembly_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(dir.join("start1.wav"), b"mock").unwrap();
    std::fs::write(dir.join("aoaCrit.wav"), b"mock").unwrap();
    let plays: Arc<Mutex<HashMap<String, usize>>> = Arc::new(Mutex::new(HashMap::new()));
    // W1 共享管理器接口匹配消费: open_voice_warning 的 manager 参数直接注入
    // mock 装配的管理器 (无需适配层)
    let mgr = Arc::new(VoiceResourceManager::new_with_voice_dir(
        Box::new(CountingPlayer {
            plays: Arc::clone(&plays),
        }),
        dir.to_string_lossy().into_owned(),
    ));
    // 遥测: playerLive + IAS 200 + AoA 20 → checkAoAWarning 的 aoaCrit 腿
    let mut d = live_service_data("spitfire");
    d.s_state.as_mut().unwrap().ias = 200;
    d.s_state.as_mut().unwrap().aoa = 20.0;
    let data = frame_store_of(&d);

    // 订阅在: 装配后 FlightDataBus +1 (initCompressorWarning 的 register)
    let base = shell.flight_bus.subscriber_count();
    let mut session = open_voice_warning(
        &mgr,
        &shell.ui_bus,
        &shell.config_snapshots.voice,
        &shell.fm,
        &shell.flight_bus,
        Some(Arc::clone(&data)),
    )
    .expect("live 在位应建会话 (Java init(S) 非短路)");
    assert_eq!(
        shell.flight_bus.subscriber_count(),
        base + 1,
        "会话存活期 FlightDataBus 订阅应在"
    );
    // configHandler 在: 发布送达数 = 2 (统一总线上 Controller 常驻订阅 1 +
    // VoiceWarning configHandler 1; 原独立 voice_bus 时代 VoiceWarning 独占为 1)
    assert_eq!(
        shell.ui_bus.publish(
            ui_state_events::CONFIG_CHANGED,
            Some("test"),
            Some("voice_aoaCrit")
        ),
        2,
        "configHandler 应在订阅中 (Controller 1 + configHandler 1)"
    );

    // 等 tick (启动延迟 1s + 100ms 节拍; 轮询 8s 超时即失败 — 不假通过)。
    // 信号 = fatal_warn==Some(true): AoA 遥测同 tick 先触发 aoaCrit 播放
    // (checkAoAWarning 的 play 先于 fatal 累积), 故此处成立时播放计数已就绪;
    // 不能用 is_some() — ServiceData 初值即 Some(false), 0ms 即真 (假通过面)
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if data.fatal_warn() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "8s 内至少一轮 tick (fatalWarn=true 未写 = 线程未跑)"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    // 播放计数: current_time_ms 恒 0 → 冷却期内不重播, 计数确定性为 1
    {
        let p = plays.lock().unwrap();
        assert_eq!(p.get("start1"), Some(&1), "启动音效应播一次: {p:?}");
        assert_eq!(p.get("aoaCrit"), Some(&1), "AoA 告警键应触发 play 路径: {p:?}");
    }

    // 销毁: 停机 (join = VoiceWarning Drop) 后无僵尸订阅 (双总线注销)
    session.stop();
    assert_eq!(
        shell.flight_bus.subscriber_count(),
        base,
        "停机后 FlightDataBus 订阅应注销 (RAII)"
    );
    assert_eq!(
        shell.ui_bus.publish(
            ui_state_events::CONFIG_CHANGED,
            Some("test"),
            Some("voice_aoaCrit")
        ),
        1,
        "configHandler 应随线程退出注销, 仅剩 Controller 常驻订阅 (Java 泄漏点的根治面)"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// W1 (审查): 游戏会话中 RefreshPreviews(enableVoiceWarn) 即时停语音告警 —
/// Java refreshPreviews 对触达条目调 refreshPreview(forPreviewMode ctx):
/// gameModeOnly(preview)=false → 在场即 close (Controller.java:498-536 +
/// OverlayManager.java:320-340); Rust 原实现只走 host 窗口条目, voice_warn
/// 无重估面 — 关掉开关后告警继续响到会话结束。观测面 = voice_bus 订阅计数
/// (VoiceWarning 起线程订阅 +1, 停机退订回落)。开方向不重建: Java preview-ctx
/// 下 shouldBeOpen 恒 false 同样不 open (怪癖保真), 重起等 OpenAllOverlays。
#[test]
fn refresh_previews_stop_voice_warn_session() {
    // 观测探针: CONFIG_CHANGED 的送达数 (UIStateBus 路由总线无订阅计数面,
    // publish 返回送达数 — 本文件 VoiceWarnSession 会话测试同款; 探针触发的
    // configHandler→reload 无副作用: 测试 CWD voice/ 无音频文件, load_clip
    // 静默 None)
    fn probe_deliveries(bus: &vm_core::base::bus::ui_state_bus::UIStateBus) -> usize {
        bus.publish(ui_state_events::CONFIG_CHANGED, Some("W1Probe"), None)
    }
    fn wait_deliveries(bus: &vm_core::base::bus::ui_state_bus::UIStateBus, want: usize) -> bool {
        let start = Instant::now();
        loop {
            if probe_deliveries(bus) == want {
                return true;
            }
            if start.elapsed() > Duration::from_millis(3000) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let cfg = fixture_cfg(
        "(panel \"T\" :visible true\n\
         \x20 (item \"v\" :type switch :target \"enableVoiceWarn\" :value true))\n\
        ",
    );
    let mut shell = fixture_full(30, cfg);
    // live 槽手工装填 (openpad 前提; 不起真 Service — 零值数据 player_live=false,
    // 告警静默, 只驱动会话生命周期; open_voice_warning 测试同款先例)
    *shell.shared.live.write().unwrap() = Some(frame_store_of(&ServiceData::default()));
    shell.spawn_win32_thread().expect("win32 线程启动");
    let base = probe_deliveries(&shell.ui_bus); // 无会话期送达 0
    shell.send_ui(UiCommand::OpenAllOverlays);
    assert!(
        wait_deliveries(&shell.ui_bus, base + 1),
        "OpenAllOverlays 应起 VoiceWarning 线程 (configHandler 送达 +1, 实测 {})",
        probe_deliveries(&shell.ui_bus)
    );
    // 游戏稳态 (openpad 后 overlay_ctx_preview=false) + WYSIWYG 键控刷新
    // (State=Preview 是 Java 同名态, 防过期守卫放行)
    *shell.shared.state.write().unwrap() = ControllerState::Preview;
    let gen = shell.shared.preview_generation.load(std::sync::atomic::Ordering::SeqCst);
    shell.send_ui(UiCommand::RefreshPreviews {
        changed_key: Some("enableVoiceWarn".to_string()),
        generation: gen,
    });
    assert!(
        wait_deliveries(&shell.ui_bus, base),
        "RefreshPreviews(enableVoiceWarn) 应即时停语音会话 (退订回落, 实测 {})",
        probe_deliveries(&shell.ui_bus)
    );
    // 开方向不重建 (Java preview-ctx 怪癖同形态): 再次键控刷新计数不动
    let gen2 = shell.shared.preview_generation.load(std::sync::atomic::Ordering::SeqCst);
    shell.send_ui(UiCommand::RefreshPreviews {
        changed_key: Some("enableVoiceWarn".to_string()),
        generation: gen2,
    });
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        probe_deliveries(&shell.ui_bus),
        base,
        "开方向不得经 RefreshPreviews 重建 (Java 同形态, 重起等 OpenAllOverlays)"
    );
    shell.send_ui(UiCommand::Shutdown);
    let join = shell.win32.take().unwrap();
    assert!(join.join().is_ok(), "win32 线程应干净退出");
}
