use super::*;
use crate::audio::voice_alert_type;
use crate::audio::voice_resource_manager::SoundError;
use crate::base::bus::EventBus;
use crate::base::event::event_payload::EventPayload;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicUsize;

static DIR_N: AtomicUsize = AtomicUsize::new(0);

/// 每测试独立临时 voice 根目录 (voice_resource_manager 测试先例)
fn tmp_dir() -> PathBuf {
    let n = DIR_N.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("vm_core_vw_{}_{n}", std::process::id()))
}

/// State 逐字段快照 (State 未 derive Clone; vm-data service_loop.rs 的
/// snapshot_state 同款逐字段副本, 本文件内自备 mock 用)
fn snapshot_state(s: &State) -> State {
    State {
        valid: s.valid.clone(),
        flag: s.flag,
        engine_num: s.engine_num,
        aileron: s.aileron,
        elevator: s.elevator,
        rudder: s.rudder,
        flaps: s.flaps,
        gear: s.gear,
        tas: s.tas,
        ias: s.ias,
        m: s.m,
        aoa: s.aoa,
        heightm: s.heightm,
        aos: s.aos,
        ny: s.ny,
        vy: s.vy,
        wx: s.wx,
        throttle: s.throttle,
        rpm_throttle: s.rpm_throttle,
        radiator: s.radiator,
        oilradiator: s.oilradiator,
        mixture: s.mixture,
        compressorstage: s.compressorstage,
        magneto: s.magneto,
        power: s.power.clone(),
        rpm: s.rpm,
        manifoldpressure: s.manifoldpressure,
        watertemp: s.watertemp,
        oiltemp: s.oiltemp,
        mfuel: s.mfuel,
        mfuel_1: s.mfuel_1,
        mfuel0: s.mfuel0,
        mfuel0_1: s.mfuel0_1,
        pitch: s.pitch.clone(),
        thrust: s.thrust.clone(),
        efficiency: s.efficiency.clone(),
        airbrake: s.airbrake,
        total_thr: s.total_thr,
        throttles: s.throttles.clone(),
    }
}

/// Indicators 逐字段快照 (army 为 parser 模块私有, 经 new() 落默认值 —
/// VoiceWarning 无 army 读者; vm-data snapshot_indicators 同款形状)
fn snapshot_indicators(i: &Indicators) -> Indicators {
    let mut s = Indicators::new();
    s.valid = i.valid.clone();
    s.r#type = i.r#type.clone();
    s.flag = i.flag;
    s.speed = i.speed;
    s.pedals = i.pedals;
    s.stick_elevator = i.stick_elevator;
    s.stick_ailerons = i.stick_ailerons;
    s.altitude_hour = i.altitude_hour;
    s.altitude_min = i.altitude_min;
    s.altitude_10k = i.altitude_10k;
    s.bank = i.bank;
    s.turn = i.turn;
    s.compass = i.compass;
    s.clock_hour = i.clock_hour;
    s.clock_min = i.clock_min;
    s.clock_sec = i.clock_sec;
    s.manifold_pressure = i.manifold_pressure;
    s.rpm = i.rpm;
    s.oil_pressure = i.oil_pressure;
    s.water_temperature = i.water_temperature;
    s.engine_temperature = i.engine_temperature;
    s.mixture = i.mixture;
    s.fuel = i.fuel;
    s.fuel_pressure = i.fuel_pressure;
    s.oxygen = i.oxygen;
    s.gears_lamp = i.gears_lamp;
    s.flaps = i.flaps;
    s.trimmer = i.trimmer;
    s.throttle = i.throttle;
    s.weapon1 = i.weapon1;
    s.weapon2 = i.weapon2;
    s.weapon3 = i.weapon3;
    s.prop_pitch_hour = i.prop_pitch_hour;
    s.prop_pitch_min = i.prop_pitch_min;
    s.ammo_counter1 = i.ammo_counter1;
    s.ammo_counter2 = i.ammo_counter2;
    s.ammo_counter3 = i.ammo_counter3;
    s.oil_temp = i.oil_temp;
    s.water_temp = i.water_temp;
    s.fuelnum = i.fuelnum;
    s.vario = i.vario;
    s.aviahorizon_pitch = i.aviahorizon_pitch;
    s.aviahorizon_roll = i.aviahorizon_roll;
    s.wsweep_indicator = i.wsweep_indicator;
    s.radio_altitude = i.radio_altitude;
    s.mach = i.mach;
    s
}

/// 播放日志中指定 key 的 start 次数
fn starts(log: &Mutex<Vec<String>>, key: &str) -> usize {
    let target = format!("{key}:start");
    log.lock()
        .unwrap()
        .iter()
        .filter(|s| s.as_str() == target)
        .count()
}

// ---- mock SoundClip/SoundPlayer (D7 trait 的测试替身) ----

struct MockClip {
    key: String,
    log: Arc<Mutex<Vec<String>>>,
    running: Arc<AtomicBool>,
}

impl SoundClip for MockClip {
    fn start(&self) {
        self.log.lock().unwrap().push(format!("{}:start", self.key));
        self.running.store(true, Ordering::SeqCst);
    }
    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    fn set_frame_position(&self, _frame: i32) {}
    fn close(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
    fn master_gain_range(&self) -> Option<(f32, f32)> {
        None // 控件不支持 (不走音量路径)
    }
    fn set_master_gain(&self, _value: f32) {}
}

struct MockPlayer {
    /// 每次 open_clip 尝试的文件 stem 序列
    calls: Mutex<Vec<String>>,
    /// key → 最近一次创建 clip 的 running 句柄 (测试侧翻转模拟播放结束)
    running_handles: Mutex<HashMap<String, Arc<AtomicBool>>>,
    log: Arc<Mutex<Vec<String>>>,
}

impl MockPlayer {
    fn open_clip(&self, path: &Path) -> Result<Box<dyn SoundClip>, SoundError> {
        let key = path.file_stem().unwrap().to_string_lossy().into_owned();
        self.calls.lock().unwrap().push(key.clone());
        if !path.exists() {
            return Err("mock: file missing".into());
        }
        let running = Arc::new(AtomicBool::new(false));
        self.running_handles
            .lock()
            .unwrap()
            .insert(key.clone(), Arc::clone(&running));
        Ok(Box::new(MockClip {
            key,
            log: Arc::clone(&self.log),
            running,
        }))
    }

    /// 模拟指定 key 的 clip 播放结束 (is_running → false)
    fn finish(&self, key: &str) {
        if let Some(r) = self.running_handles.lock().unwrap().get(key) {
            r.store(false, Ordering::SeqCst);
        }
    }
}

/// Box<dyn SoundPlayer> 的转发壳 (让测试保留 mock 句柄)
struct PlayerForward(Arc<MockPlayer>);

impl SoundPlayer for PlayerForward {
    fn open_clip(&self, path: &Path) -> Result<Box<dyn SoundClip>, SoundError> {
        self.0.open_clip(path)
    }
}

// ---- mock ConfigProvider ----

struct MapConfig {
    values: Mutex<HashMap<String, String>>,
}

impl ConfigProvider for MapConfig {
    fn get_config(&self, key: &str) -> Option<String> {
        self.values.lock().unwrap().get(key).cloned()
    }
    fn set_config(&self, key: &str, value: &str) {
        self.values
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
    }
    fn is_field_disabled(&self, key: &str) -> bool {
        self.get_config(key).as_deref() == Some("true")
    }
}

// ---- mock VoiceWarningService ----

struct MockSvcData {
    current_time_ms: i64,
    player_live: bool,
    is_downing_flap: bool,
    flap_allow_angle: f64,
    flap_allow_speed: f64,
    total_fuel: f64,
    fuel_percent: i32,
    radio_alt: f64,
    d_radio_alt: f64,
    cur_load_min_work_time: f64,
    maximum_thr_rpm: f64,
    maximum_rpm_learned: bool,
    is_eng_jet: bool,
    stall_speed: f64,
    st: State,
    indic: Indicators,
    fatal_warn: Option<bool>,
}

impl MockSvcData {
    fn new() -> Self {
        let mut st = State::new();

        MockSvcData {
            current_time_ms: 0,
            player_live: true,
            is_downing_flap: false,
            // Java Service 字段缺省 (service_fields.rs 文档): Float.MAX_VALUE 拓宽
            flap_allow_angle: f32::MAX as f64,
            flap_allow_speed: f32::MAX as f64,
            total_fuel: 100.0,
            fuel_percent: 100,
            radio_alt: 0.0,
            d_radio_alt: 0.0,
            // Java Service 缺省 99999*1000
            cur_load_min_work_time: 99999.0 * 1000.0,
            maximum_thr_rpm: 1.0,
            maximum_rpm_learned: false,
            is_eng_jet: false,
            stall_speed: 0.0,
            st,
            indic: Indicators::new(),
            fatal_warn: None,
        }
    }
}

struct MockSvc {
    data: Mutex<MockSvcData>,
}

impl VoiceWarningService for MockSvc {
    fn current_time_ms(&self) -> i64 {
        self.data.lock().unwrap().current_time_ms
    }
    fn player_live(&self) -> bool {
        self.data.lock().unwrap().player_live
    }
    fn set_fatal_warn(&self, v: bool) {
        self.data.lock().unwrap().fatal_warn = Some(v);
    }
    fn is_downing_flap(&self) -> bool {
        self.data.lock().unwrap().is_downing_flap
    }
    fn flap_allow_angle(&self) -> f64 {
        self.data.lock().unwrap().flap_allow_angle
    }
    fn flap_allow_speed(&self) -> f64 {
        self.data.lock().unwrap().flap_allow_speed
    }
    fn total_fuel(&self) -> f64 {
        self.data.lock().unwrap().total_fuel
    }
    fn fuel_percent(&self) -> i32 {
        self.data.lock().unwrap().fuel_percent
    }
    fn radio_alt(&self) -> f64 {
        self.data.lock().unwrap().radio_alt
    }
    fn d_radio_alt(&self) -> f64 {
        self.data.lock().unwrap().d_radio_alt
    }
    fn cur_load_min_work_time(&self) -> f64 {
        self.data.lock().unwrap().cur_load_min_work_time
    }
    fn maximum_thr_rpm(&self) -> f64 {
        self.data.lock().unwrap().maximum_thr_rpm
    }
    fn maximum_rpm_learned(&self) -> bool {
        self.data.lock().unwrap().maximum_rpm_learned
    }
    fn is_eng_jet(&self) -> bool {
        self.data.lock().unwrap().is_eng_jet
    }
    fn stall_speed(&self) -> f64 {
        self.data.lock().unwrap().stall_speed
    }
    fn s_state(&self) -> State {
        snapshot_state(&self.data.lock().unwrap().st)
    }
    fn s_indic(&self) -> Indicators {
        snapshot_indicators(&self.data.lock().unwrap().indic)
    }
}

impl MockSvc {
    fn edit(&self, f: impl FnOnce(&mut MockSvcData)) {
        f(&mut self.data.lock().unwrap());
    }
}

// ---- 测试环境 ----

struct TestEnv {
    vw: VoiceWarning,
    player: Arc<MockPlayer>,
    svc: Arc<MockSvc>,
    ui_bus: Arc<UIStateBus>,
    fd_bus: Arc<FlightDataBus>,
    config: Arc<MapConfig>,
    log: Arc<Mutex<Vec<String>>>,
    _dir: PathBuf,
}

fn env() -> TestEnv {
    let dir = tmp_dir();
    std::fs::create_dir_all(&dir).unwrap();
    // 为全部告警 key 造 wav, 避开 load_clip 缺失文件的错误日志噪音
    for ty in voice_alert_type::ALL {
        let _ = std::fs::write(dir.join(format!("{}.wav", ty.get_key())), b"wav");
    }
    let player = Arc::new(MockPlayer {
        calls: Mutex::new(Vec::new()),
        running_handles: Mutex::new(HashMap::new()),
        log: Arc::new(Mutex::new(Vec::new())),
    });
    let log = Arc::clone(&player.log);
    let mgr = Arc::new(VoiceResourceManager::new_with_voice_dir(
        Box::new(PlayerForward(Arc::clone(&player))),
        dir.to_str().unwrap().to_string(),
    ));
    let fm = Arc::new(FMManager::new(Arc::new(EventBus::new())));
    let ui_bus = Arc::new(UIStateBus::new());
    let fd_bus = Arc::new(FlightDataBus::new());
    let config = Arc::new(MapConfig {
        values: Mutex::new(HashMap::new()),
    });
    let svc = Arc::new(MockSvc {
        data: Mutex::new(MockSvcData::new()),
    });
    let mut vw = VoiceWarning::new(
        Arc::clone(&config) as Arc<dyn ConfigProvider + Send + Sync>,
        mgr,
        fm,
        Arc::clone(&ui_bus),
        Arc::clone(&fd_bus),
        Arc::new(PlayerForward(Arc::clone(&player))) as Arc<dyn SoundPlayer>,
    );
    vw.init(Some(Arc::clone(&svc) as Arc<dyn VoiceWarningService>));
    TestEnv {
        vw,
        player,
        svc,
        ui_bus,
        fd_bus,
        config,
        log,
        _dir: dir,
    }
}

fn fd_event(compressor_stage_mismatch: bool) -> FlightDataEvent {
    FlightDataEvent::new(
        EventPayload::builder()
            .compressor_stage_mismatch(compressor_stage_mismatch)
            .build(),
    )
}

// ---- init / 注册表 ----

// Java init(S=null): doit=false 短路
#[test]
fn init_none_service_disables() {
    let TestEnv { mut vw, .. } = env();
    vw.init(None);
    assert!(!vw.doit.load(Ordering::SeqCst));
}

// init 注册全部告警; "fail_engine" 同 key 后写覆盖 → map 指向 invert 实例
// (Java ConcurrentHashMap.put 覆盖语义保真); start1 启动音效播放一次
#[test]
fn init_registers_alerts_and_fail_engine_overwrite() {
    let TestEnv { vw, log, .. } = env();
    let map = vw.alerts.lock().unwrap();
    // 25 个告警实例 - 1 个 fail_engine 重键 = 24 个去重注册项
    assert_eq!(map.len(), 24, "alerts map 应含 24 个去重 key");
    assert!(Arc::ptr_eq(
        map.get("fail_engine").unwrap(),
        vw.eng_fail_invert.as_ref().unwrap()
    ));
    assert!(!Arc::ptr_eq(
        map.get("fail_engine").unwrap(),
        vw.eng_fail.alert.as_ref().unwrap()
    ));
    assert!(map.contains_key("start1"));
    drop(map);
    // 启动音效: init 尾部 playOnce(currentTimeMs=0)
    assert_eq!(starts(&log, "start1"), 1);
}

// init 后的默认告警线 (无 FM): aoa=15, ias/mach=Float.MAX_VALUE 拓宽,
// gear=450, ny=-4/10, 舵效 65535, 低油量线 10
#[test]
fn init_default_lines_without_fm() {
    let TestEnv { vw, .. } = env();
    assert_eq!(vw.aoa_warning_line, 15.0);
    assert_eq!(vw.ias.line, f32::MAX as f64);
    assert_eq!(vw.mach.line, f32::MAX as f64);
    assert_eq!(vw.gear.line, 450.0);
    assert_eq!(vw.ny_warning_line0, -4.0);
    assert_eq!(vw.ny_warning_line1, 10.0);
    assert_eq!(vw.rudder.line, 65535.0);
    assert_eq!(vw.elevator.line, 65535.0);
    assert_eq!(vw.aileron.line, 65535.0);
    assert_eq!(vw.fuel.line, 10.0);
    assert!(vw.is_gear_alive && vw.is_flap_alive && !vw.eng_damage);
}

// ---- VoiceAlert 冷却/播放 ----

// 冷却期内不重播; 冷却过后 clip 仍在播放则继续压制; 播放结束后可重播
#[test]
fn voice_alert_cooldown_and_running_gate() {
    let TestEnv {
        vw, player, log, ..
    } = env();
    let alert = vw.aoa_crit.as_ref().unwrap(); // 冷却 1s

    assert!(!alert.is_playing(0)); // 初始静默
    alert.play_once(0);
    assert_eq!(starts(&log, "aoaCrit"), 1);
    assert!(alert.is_act.load(Ordering::SeqCst));

    alert.play_once(500); // 冷却期内 (500-0 <= 1000)
    assert_eq!(starts(&log, "aoaCrit"), 1);

    player.finish("aoaCrit"); // 模拟播放结束
    alert.play_once(1001); // 冷却过 + 不再 running → 重播
    assert_eq!(starts(&log, "aoaCrit"), 2);

    alert.play_once(1002); // 刚播过, 冷却重新计时
    assert_eq!(starts(&log, "aoaCrit"), 2);
}

// 不可用 (资源缺失/配置禁用) 时 is_playing 恒 true — 防重试循环; play_once 无操作
#[test]
fn voice_alert_unavailable_pretends_playing() {
    let TestEnv { vw, log, .. } = env();
    // 无 wav 的 key → reload 后 available=false
    let a = VoiceAlert::new("no_such_key", 1);
    a.reload(&*vw.config_provider, &vw.resource_manager);
    assert!(a.is_playing(0), "不可用时假装在播放");
    a.play_once(0); // 无操作, 不 panic
    assert_eq!(starts(&log, "no_such_key"), 0);
}

// ---- configHandler 触发链 (UIStateBus 注入) ----

// CONFIG_CHANGED(voice_<key>) → 命中 alerts 注册表 → reload; 非语音键/未知键忽略
#[test]
fn config_changed_reloads_registered_alert() {
    let TestEnv {
        vw,
        config,
        ui_bus,
        player,
        ..
    } = env();
    let opens = |key: &str| {
        player
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|k| k.as_str() == key)
            .count()
    };
    let before = opens("aoaCrit");

    // 命中: voice_aoaCrit
    assert_eq!(
        ui_bus.publish(
            ui_state_events::CONFIG_CHANGED,
            Some("test"),
            Some("voice_aoaCrit")
        ),
        1
    );
    assert_eq!(opens("aoaCrit"), before + 1, "应触发一次 reload 重开");

    // 非语音键: handler 前缀过滤
    let delivered = ui_bus.publish(
        ui_state_events::CONFIG_CHANGED,
        Some("test"),
        Some("showSpeedBar"),
    );
    assert_eq!(delivered, 1);
    assert_eq!(opens("aoaCrit"), before + 1);

    // 未知语音键: 注册表未命中, 无 reload
    ui_bus.publish(
        ui_state_events::CONFIG_CHANGED,
        Some("test"),
        Some("voice_nope"),
    );
    assert_eq!(opens("nope"), 0);

    // 配置禁用: voice_aoaHigh = "default|false" → reload 后 available=false
    config.set_config("voice_aoaHigh", "default|false");
    ui_bus.publish(
        ui_state_events::CONFIG_CHANGED,
        Some("test"),
        Some("voice_aoaHigh"),
    );
    assert!(vw.aoa_high.as_ref().unwrap().is_playing(0));
}

// ---- 订阅注销 (Java 泄漏根治 — LIFETIMES §2.1) ----

// dispose 显式注销两条总线订阅
#[test]
fn dispose_unsubscribes_both_buses() {
    let TestEnv {
        mut vw,
        ui_bus,
        fd_bus,
        ..
    } = env();
    assert_eq!(
        ui_bus.publish(
            ui_state_events::CONFIG_CHANGED,
            Some("t"),
            Some("voice_aoaCrit")
        ),
        1
    );
    assert_eq!(fd_bus.subscriber_count(), 1);
    vw.dispose();
    assert_eq!(
        ui_bus.publish(
            ui_state_events::CONFIG_CHANGED,
            Some("t"),
            Some("voice_aoaCrit")
        ),
        0,
        "Java 版此处仍会送达 (configHandler 泄漏), Rust 版已根治"
    );
    assert_eq!(fd_bus.subscriber_count(), 0);
}

// 忘记 dispose 时 Drop 兜底注销 (泄漏在类型层面不可能)
#[test]
fn drop_unsubscribes_both_buses() {
    let TestEnv {
        vw, ui_bus, fd_bus, ..
    } = env();
    assert_eq!(fd_bus.subscriber_count(), 1);
    drop(vw); // 不调 dispose 直接 drop
    assert_eq!(fd_bus.subscriber_count(), 0);
    assert_eq!(
        ui_bus.publish(
            ui_state_events::CONFIG_CHANGED,
            Some("t"),
            Some("voice_aoaCrit")
        ),
        0
    );
}

// ---- check* 触发链 (mock Service 数据 + mock 播放日志) ----

// 临界攻角: aoaCrit 播放 + aoaHigh 被标记已播放 (不重复触发) + fatal;
// 高攻角带: 仅 aoaHigh 播放, 非 fatal
#[test]
fn check_aoa_warning_crit_marks_high() {
    let TestEnv { mut vw, log, .. } = env();
    vw.st.ias = 100;
    vw.st.aoa = 14.5; // > 15-1

    assert!(vw.check_aoa_warning(1000));
    assert_eq!(starts(&log, "aoaCrit"), 1);
    assert_eq!(starts(&log, "aoaHigh"), 0, "aoaHigh 不应播放");
    // 同时标记 aoaHigh 为已播放 (Java 包内直写字段)
    assert!(vw.aoa_high.as_ref().unwrap().is_act.load(Ordering::SeqCst));
    assert_eq!(
        vw.aoa_high
            .as_ref()
            .unwrap()
            .last_time_play
            .load(Ordering::SeqCst),
        1000
    );

    // 高攻角预警带: 15*0.75=11.25 < aoa <= 14。
    // t 须 > 1000+8000: crit 分支已把 aoaHigh 标记为已播放 (t=1000),
    // 8s 冷却期内 playOnce 被压制 (Java 防双告警语义)
    vw.st.aoa = 12.0;
    assert!(!vw.check_aoa_warning(10_000));
    assert_eq!(starts(&log, "aoaHigh"), 1);

    // IAS 门禁: <= 80 直接短路
    vw.st.ias = 80;
    vw.st.aoa = 30.0;
    assert!(!vw.check_aoa_warning(3000));
    assert_eq!(starts(&log, "aoaCrit"), 1, "IAS 不足不触发");
}

// IAS / Mach 双超限各自播放并置 fatal
#[test]
fn check_speed_warning_ias_mach_fatal() {
    let TestEnv { mut vw, log, .. } = env();
    vw.ias.line = 500.0;
    vw.mach.line = 0.8;
    vw.st.ias = 600;
    vw.st.m = 0.9;

    assert!(vw.check_speed_warning(0));
    assert_eq!(starts(&log, "warn_ias"), 1);
    assert_eq!(starts(&log, "warn_mach"), 1);

    // 均未超限
    let TestEnv {
        vw: mut vw2,
        log: log2,
        ..
    } = env();
    vw2.ias.line = 500.0;
    vw2.mach.line = 0.8;
    vw2.st.ias = 400;
    vw2.st.m = 0.5;
    assert!(!vw2.check_speed_warning(0));
    assert_eq!(starts(&log2, "warn_ias"), 0);
    assert_eq!(starts(&log2, "warn_mach"), 0);
}

// 起落架超速 fatal / 减速板组合 / 大下降率
#[test]
fn check_gear_brake_vario_warnings() {
    let TestEnv { mut vw, log, .. } = env();
    // gear: 放下 (gear>0) 且 IAS >= 450
    vw.st.ias = 500;
    vw.st.gear = 100;
    assert!(vw.check_gear_warning(0));
    assert_eq!(starts(&log, "warn_gear"), 1);

    // brake: 起落架未放下但减速板展开
    vw.st.gear = 0;
    vw.st.airbrake = 90;
    vw.check_brake_warning(0);
    assert_eq!(starts(&log, "warn_brake"), 1);

    // vario: 起落架放下且下降率过高
    vw.st.gear = 50;
    vw.st.vy = -9.0;
    vw.check_vario_warning(0);
    assert_eq!(starts(&log, "warn_highvario"), 1);

    // 起落架损坏后不再告警
    vw.is_gear_alive = false;
    vw.st.gear = 100;
    vw.st.vy = -20.0;
    assert!(!vw.check_gear_warning(1000));
    assert_eq!(starts(&log, "warn_gear"), 1);
}

// 襟翼告警两条路径 (静止超限 / 正在下襟翼的宽限)
#[test]
fn check_flap_warning_conditions() {
    let TestEnv {
        mut vw,
        svc,
        player,
        log,
        ..
    } = env();
    // cond1: 非下襟翼态, 允许角-当前角 < 2 且襟翼非 0
    svc.edit(|d| {
        d.is_downing_flap = false;
        d.flap_allow_angle = 10.0;
    });
    vw.st.flaps = 9;
    assert!(vw.check_flap_warning(0));
    assert_eq!(starts(&log, "warn_flap"), 1);

    // cond2: 下襟翼态, 允许角-当前角 < 8
    player.finish("warn_flap"); // 首Frame clip 已停 (否则 isRunning 压制重播)
    svc.edit(|d| {
        d.is_downing_flap = true;
        d.flap_allow_angle = 10.0;
    });
    vw.st.flaps = 5; // 10-5=5 < 8
    assert!(vw.check_flap_warning(1500)); // warn_flap 冷却 1s (须严格大于)
    assert_eq!(starts(&log, "warn_flap"), 2);

    // 均不满足: 差值足够大
    svc.edit(|d| {
        d.is_downing_flap = true;
        d.flap_allow_angle = 20.0;
    });
    vw.st.flaps = 5; // 20-5=15 >= 8
    assert!(!vw.check_flap_warning(2000));
    assert_eq!(starts(&log, "warn_flap"), 2);
}

// 无油/低油量 16 tick 门限 (后自增语义: 第 18 次调用才播放)
#[test]
fn check_fuel_warning_tick_gates() {
    let TestEnv {
        mut vw, svc, log, ..
    } = env();
    svc.edit(|d| {
        d.total_fuel = 0.0;
        d.fuel_percent = 5; // <= 10
    });
    for t in 0..17 {
        vw.check_fuel_warning(t * 100);
    }
    assert_eq!(starts(&log, "fail_nofuel"), 0, "17 次未过门限");
    assert_eq!(starts(&log, "warn_lowfuel"), 0);
    vw.check_fuel_warning(1700); // 第 18 次: old=17 > 16
    assert_eq!(starts(&log, "fail_nofuel"), 1);
    assert_eq!(starts(&log, "warn_lowfuel"), 1);
    // 计数已清零, 立即再调不重播
    vw.check_fuel_warning(1800);
    assert_eq!(starts(&log, "fail_nofuel"), 1);
    assert_eq!(starts(&log, "warn_lowfuel"), 1);
}

// 油压低 → 2s 后 warn_lowpressure + 引擎损坏标记; 损坏后油压 0 → 1s 后 fail_engine
#[test]
fn check_fuel_pressure_warning_chain() {
    let TestEnv { mut vw, log, .. } = env();
    vw.st.throttle = 100;
    vw.indic.fuel_pressure = 0.1; // 100 - 0.1*10 = 99 > 2

    for _ in 0..19 {
        vw.check_fuel_pressure_warning(0);
    }
    assert!(!vw.eng_damage, "1900ms 未到 2000 门限");
    vw.check_fuel_pressure_warning(0); // 第 20 次: 2000
    assert!(vw.eng_damage);
    assert_eq!(starts(&log, "warn_lowpressure"), 1);

    // 引擎损坏后油压归零 → fail_engine, 损坏标记清除。
    // A6 拆分后两计数器独立: fp=0 时低油压块条件仍真 (throttle-0 > 2) 但只记
    // 自己的计数, fail_engine 由独立计数满 1000 → 第 10 次调用触发
    vw.indic.fuel_pressure = 0.0;
    for _ in 0..9 {
        vw.check_fuel_pressure_warning(0);
    }
    assert_eq!(starts(&log, "fail_engine"), 0);
    vw.check_fuel_pressure_warning(0); // 第 10 次: 独立累计 1000
    assert!(!vw.eng_damage);
    assert_eq!(starts(&log, "fail_engine"), 1);

    // 条件消失 → 油压低计数清零 (损坏腿计数已随播放清零)
    vw.indic.fuel_pressure = 10.0; // 100 - 100 = 0, 不 > 2
    vw.check_fuel_pressure_warning(0);
    assert_eq!(vw.fuel_prs.check, 0);
}

// 倒飞断油 (fail_engine 冷却 5s 的 invert 实例) + 转速低/高
#[test]
fn check_inverted_flight_and_rpm_warnings() {
    let TestEnv {
        mut vw, svc, log, ..
    } = env();
    // 倒飞: Ny<0, 油门>50, 推力<50
    vw.st.ny = -1.0;
    vw.st.throttle = 60;
    vw.st.thrust[0] = 10;
    vw.check_inverted_flight_warning(0);
    assert_eq!(
        starts(&log, "fail_engine"),
        1,
        "invert 实例共用 fail_engine 键"
    );

    // 转速低: 非喷气 + 桨距有效, 油门-30 > RPM*100/maxRPM。
    // (须先解除倒飞态 — Ny<0+油门大+推力低 会短路转速检测)
    svc.edit(|d| {
        d.is_eng_jet = false;
        d.maximum_thr_rpm = 1000.0;
    });
    vw.st.ny = 1.0;
    vw.st.thrust[0] = 100;
    vw.st.throttle = 100;
    vw.st.rpm = 100; // 100*100/1000 = 10 < 70
    vw.check_rpm_warning(0);
    assert_eq!(starts(&log, "warn_lowrpm"), 1);

    // 转速高: 自适应已学习 + RPM*100/maxRPM >= 105
    svc.edit(|d| d.maximum_rpm_learned = true);
    vw.st.rpm = 1100; // 110 >= 105; 70 > 110 为假 → 低转速不再触发
    vw.check_rpm_warning(10_000); // warn_lowrpm 冷却 10s
    assert_eq!(starts(&log, "warn_highrpm"), 1);
    assert_eq!(starts(&log, "warn_lowrpm"), 1);
}

// 失速: 存活 + 未放起落架 + 有下降率 + IAS <= 失速速度
#[test]
fn check_stall_warning() {
    let TestEnv {
        mut vw, svc, log, ..
    } = env();
    svc.edit(|d| d.stall_speed = 150.0);
    vw.st.gear = 0;
    vw.st.vy = -1.0;
    vw.st.ias = 140;
    vw.check_stall_warning(0);
    assert_eq!(starts(&log, "warn_stall"), 1);

    // 速度高于失速速度: 不触发
    vw.st.ias = 160;
    vw.check_stall_warning(2000); // 冷却 2s
    assert_eq!(starts(&log, "warn_stall"), 1);

    // 无下降率 (Vy==0): 不触发
    vw.st.ias = 140;
    vw.st.vy = 0.0;
    vw.check_stall_warning(4000);
    assert_eq!(starts(&log, "warn_stall"), 1);
}

// 高度告警 (下降率 > 高度/10) 优先; 其次地形告警 (无线电高度变化率)
#[test]
fn check_altitude_and_terrain_warning() {
    let TestEnv {
        mut vw, svc, log, ..
    } = env();
    vw.st.gear = 0; // 且 playerLive=true
    vw.st.heightm = 100.0;
    vw.st.vy = -20.0; // -20 < -100/10 = -10
    assert!(vw.check_altitude_warning(0));
    assert_eq!(starts(&log, "warn_altitude"), 1);

    // 地形: 下降率未触高度线, 但无线电高度骤降
    svc.edit(|d| {
        d.radio_alt = 100.0;
        d.d_radio_alt = -15.0; // < -100/10
    });
    vw.st.vy = -5.0;
    assert!(vw.check_altitude_warning(5000)); // 冷却 5s
    assert_eq!(starts(&log, "warn_terrain"), 1);

    // 起落架放下 → 短路
    vw.st.gear = 100;
    assert!(!vw.check_altitude_warning(10_000));
    assert_eq!(starts(&log, "warn_altitude"), 1);
}

// 过载动态阈值: rawWingCritOverload 存在时按当前重量重算上下限
#[test]
fn check_load_factor_dynamic_limits() {
    let TestEnv { mut vw, log, .. } = env();
    // 静态线上限拉高到 20 — 动态计算应压回 ~12, ny=15 触发
    vw.ny_warning_line1 = 20.0;
    vw.st.ny = 15.0;
    assert!(!vw.check_load_factor_warning(0), "静态线内不触发");
    assert_eq!(starts(&log, "warn_loadfactor"), 0);

    let mut b = FmData::default();
    // 正 g 限 = 1.2*(2*raw1/(g*6000) - 1) = 12 → raw1 = 11*g*6000/2
    b.raw_wing_crit_overload = Some([0.0, 11.0 * crate::base::physics_constants::g * 6000.0 / 2.0]);
    vw.fmdata = Some(b);
    vw.nofuelweight = 5000.0;
    vw.st.mfuel = 1000.0; // currentWeight = 6000
    assert!(
        vw.check_load_factor_warning(3000),
        "动态上限 ~12, ny=15 应触发"
    );
    assert_eq!(starts(&log, "warn_loadfactor"), 1);
}

// 舵效告警边沿锁存: 越线首帧播放, 持续越线不重播, 回线复位
#[test]
fn control_effectiveness_edge_latch() {
    let TestEnv {
        mut vw,
        player,
        log,
        ..
    } = env();
    vw.aileron.line = 300.0;
    vw.rudder.line = 400.0;
    vw.st.ias = 350;

    vw.check_control_effectiveness_warning(0);
    assert_eq!(starts(&log, "aileronEff"), 1);
    assert_eq!(starts(&log, "rudderEff"), 0, "方向舵未越线");

    vw.check_control_effectiveness_warning(100);
    assert_eq!(starts(&log, "aileronEff"), 1, "锁存期内不重播");

    vw.st.ias = 200; // 回线复位
    vw.check_control_effectiveness_warning(200);
    player.finish("aileronEff"); // 首Frame clip 已停 (否则 isRunning 压制重播)
    vw.st.ias = 350; // 再次越线 → 重新播放 (冷却 10s 已过)
    vw.check_control_effectiveness_warning(30_000);
    assert_eq!(starts(&log, "aileronEff"), 2);
}

// 增压器档位不匹配: FlightDataBus 事件 → currentMismatch → 3s 延迟告警,
// 每 3s 重复, 状态解除即取消
#[test]
fn compressor_mismatch_delayed_and_repeating() {
    let TestEnv {
        mut vw,
        fd_bus,
        player,
        log,
        ..
    } = env();
    // 总线注入断言: 发布事件 → 订阅闭包写 currentMismatch
    fd_bus.publish(&fd_event(true));
    assert!(vw.current_mismatch.load(Ordering::SeqCst));

    vw.check_compressor_warning(1000); // false→true: 定时器 = 1000+3000
    assert_eq!(vw.pending_compressor_warn_time, 4000);
    assert_eq!(starts(&log, "warn_compressor"), 0, "3s 延迟内不告警");
    vw.check_compressor_warning(3999);
    assert_eq!(starts(&log, "warn_compressor"), 0);

    vw.check_compressor_warning(4000); // 到点告警, 重排 7000
    assert_eq!(starts(&log, "warn_compressor"), 1);
    assert_eq!(vw.pending_compressor_warn_time, 7000);

    player.finish("warn_compressor"); // 冷却 0, 仅 running 压制需解除
    vw.check_compressor_warning(7000); // 每 3s 重复
    assert_eq!(starts(&log, "warn_compressor"), 2);

    fd_bus.publish(&fd_event(false)); // 一致了 → 取消定时器
    vw.check_compressor_warning(7200);
    assert_eq!(vw.pending_compressor_warn_time, 0);
    player.finish("warn_compressor");
    vw.check_compressor_warning(20_000);
    assert_eq!(starts(&log, "warn_compressor"), 2, "解除后不再重复");
}

// 起落架/襟翼超速计时损坏 (10s/15s), 收起后恢复。
// gear 须为放下态 (>0): 同方法尾部的 "收起后恢复" 分支在 gear==0 时会把
// 刚标记的损坏立即复原 (Java 同序行为)
#[test]
fn structure_health_damage_and_restore() {
    let TestEnv {
        mut vw, svc, log, ..
    } = env();
    vw.st.ias = 500; // > gear 线 450
    vw.st.gear = 100; // 放下态, 阻断同帧复原
    for _ in 0..99 {
        vw.update_structure_health(); // 9900ms
    }
    assert!(vw.is_gear_alive, "9900ms 未到 10s 门限");
    vw.update_structure_health(); // 第 100 次: 10000
    assert!(!vw.is_gear_alive, "超速 10s 标记损坏");

    // 收起后恢复
    vw.st.gear = 0;
    vw.update_structure_health();
    assert!(vw.is_gear_alive);

    // 襟翼: 允许速度 300, 超速 15s 损坏
    svc.edit(|d| d.flap_allow_speed = 300.0);
    vw.st.flaps = 50;
    for _ in 0..150 {
        vw.update_structure_health();
    }
    assert!(!vw.is_flap_alive, "超速 15s 标记损坏");
    vw.st.flaps = 0;
    vw.update_structure_health();
    assert!(vw.is_flap_alive);

    // 损坏期间的 gear 告警不再触发 (checkGearWarning 的 isGearAlive 门禁)
    assert_eq!(starts(&log, "warn_gear"), 0);
}

// 无 FM 时 updateDynamicParameters 保持既有告警线 (R1 快照降级路径)
#[test]
fn update_dynamic_parameters_keeps_lines_without_fm() {
    let TestEnv { mut vw, svc, .. } = env();
    svc.edit(|d| d.indic.wsweep_indicator = 0.5); // 有 sweep 指示也无 FM 可用
    vw.st.flaps = 30;
    let (aoa, ias, mach) = (vw.aoa_warning_line, vw.ias.line, vw.mach.line);
    vw.update_dynamic_parameters();
    assert_eq!(vw.aoa_warning_line, aoa);
    assert_eq!(vw.ias.line, ias);
    assert_eq!(vw.mach.line, mach);
}

// ---- run() 主循环 (§2.13 线程映射) ----

// 启动延迟后正常打点 (写 fatalWarn), 停机标志翻转后循环退出
#[test]
fn run_loop_ticks_and_exits_on_stop() {
    let TestEnv { mut vw, svc, .. } = env();
    let doit = Arc::clone(&vw.doit);
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let handle = std::thread::spawn(move || {
        vw.run();
        let _ = tx.send(());
    });

    // 启动延迟 1s + 至少一个 100ms tick。轮询等待而非固定 sleep(1500):
    // 理论 1.1s 出结果但断言余量仅 ~400ms, 线程调度重负载下间歇假失败
    // (审查轮 A-W 复现面) — 轮询超时 8s 是等待手法的鲁棒化, 断言语义不变
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if svc.data.lock().unwrap().fatal_warn.is_some() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "8s 内至少一轮 tick 应写 fatalWarn (线程未打点 = 行为缺失)"
        );
        std::thread::sleep(Duration::from_millis(25));
    }

    doit.store(false, Ordering::SeqCst); // ≈ Java OverlayEntry.close 的 interrupt
    let done = rx.recv_timeout(Duration::from_secs(2));
    assert!(done.is_ok(), "停机后 run() 应及时退出");
    let _ = handle.join();
}
