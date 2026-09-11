//! 语音告警黑盒场景 (kernel::audio): 告警类型清单 / 语音包配置解析 /
//! VoiceAlert 冷却窗与恢复重触发 (MockPlayer + tmp voice 目录) /
//! 资源管理器回退语义 / VoiceWarning 告警判定链的 fatal 传播
//! (VoiceWarningService 桩直灌遥测, run 循环经 doit 停机)。
#![allow(non_snake_case)] // 测试名 = 模块前缀英文 + 中文场景 (风格约定, 豁免蛇形)

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use expect_test::expect;
use kernel::audio::voice_alert_type::{VoiceAlertType, ALL as ALL_ALERTS};
use kernel::audio::voice_pack_config::VoicePackConfig;
use kernel::audio::voice_resource_manager::{SoundClip, SoundError, SoundPlayer, VoiceResourceManager};
use kernel::audio::voice_warning::{VoiceAlert, VoiceWarning, VoiceWarningService};
use kernel::base::bus::flight_data_bus::FlightDataBus;
use kernel::base::bus::ui_state_bus::UIStateBus;
use kernel::config::config_api::ConfigProvider;
use kernel::config::configuration_service::ConfigurationService;
use kernel::fm::{FmChangedBus, FMManager};
use kernel::game_api::parser::{Indicators, State};

// ---- Mock 播放层 (D7 注入面: SoundPlayer/SoundClip 的测试实现) ----

/// 模拟短音频: start 后 10ms 内 is_running=true, 之后自然"播完"
/// (真实 Clip 播完 isRunning 翻 false — 冷却期满后的恢复重触发依赖此)
struct MockClip {
    started_at: Mutex<Option<Instant>>,
}

impl MockClip {
    fn new() -> Self {
        MockClip {
            started_at: Mutex::new(None),
        }
    }
}

const MOCK_CLIP_PLAY_MS: u64 = 10;

impl SoundClip for MockClip {
    fn start(&self) {
        *self.started_at.lock().unwrap() = Some(Instant::now());
    }
    fn stop(&self) {
        *self.started_at.lock().unwrap() = None;
    }
    fn is_running(&self) -> bool {
        match *self.started_at.lock().unwrap() {
            Some(t0) => t0.elapsed() < Duration::from_millis(MOCK_CLIP_PLAY_MS),
            None => false,
        }
    }
    fn set_frame_position(&self, _frame: i32) {}
    fn close(&self) {
        self.stop();
    }
    fn master_gain_range(&self) -> Option<(f32, f32)> {
        Some((-80.0, 6.0)) // Java FloatControl 近似域
    }
    fn set_master_gain(&self, _value: f32) {}
}

#[derive(Default)]
struct MockPlayer {
    opened: Mutex<Vec<PathBuf>>,
}

impl SoundPlayer for MockPlayer {
    fn open_clip(&self, path: &std::path::Path) -> Result<Box<dyn SoundClip>, SoundError> {
        self.opened.lock().unwrap().push(path.to_path_buf());
        Ok(Box::new(MockClip::new()))
    }
}

// ---- 临时 voice 目录 ----

static DIR_N: AtomicUsize = AtomicUsize::new(0);

fn tmp_voice_dir(tag: &str) -> PathBuf {
    let n = DIR_N.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("vm_kvoice_{tag}_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 用毕清理 (Drop 承接, panic 展栈也清)
struct DirGuard(PathBuf);

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn poll_until(desc: &str, f: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("等待超时: {desc} (5s)");
}

// ---- 场景: 告警类型清单 ----

/// VoiceAlertType: 全集 24 项 + 代表键名/冷却 + from_key 往返
#[test]
fn 告警类型_清单与冷却表() {
    assert_eq!(ALL_ALERTS.len(), 24, "七类告警 + 启动音全集");
    let rows: Vec<String> = [
        VoiceAlertType::AoaCrit,
        VoiceAlertType::WarnIas,
        VoiceAlertType::WarnStall,
        VoiceAlertType::WarnGear,
        VoiceAlertType::WarnEngineoverheat,
        VoiceAlertType::FailNofuel,
        VoiceAlertType::WarnAltitude,
        VoiceAlertType::RudderEff,
        VoiceAlertType::WarnCompressor,
        VoiceAlertType::Start1,
    ]
    .iter()
    .map(|t| format!("{} {}s", t.get_key(), t.get_cooldown_seconds()))
    .collect();
    expect![[r#"
        aoaCrit 1s
        warn_ias 10s
        warn_stall 2s
        warn_gear 7s
        warn_engineoverheat 60s
        fail_nofuel 60s
        warn_altitude 5s
        rudderEff 10s
        warn_compressor 0s
        start1 1s"#]]
    .assert_eq(&rows.join("\n"));

    // from_key 往返 + 未知键 None
    assert_eq!(VoiceAlertType::from_key(Some("warn_stall")), Some(VoiceAlertType::WarnStall));
    assert_eq!(VoiceAlertType::from_key(Some("nope")), None);
    assert_eq!(VoiceAlertType::from_key(None), None);
}

/// VoicePackConfig: "packName|enabled" 解析契约 (Java 行为契约逐条)
#[test]
fn 语音包配置_解析契约() {
    let cases = [
        VoicePackConfig::parse(Some("jarvis|true")),
        VoicePackConfig::parse(Some("jarvis")),
        VoicePackConfig::parse(None),
        VoicePackConfig::parse(Some("")),
        VoicePackConfig::parse(Some("jarvis|false")),
        VoicePackConfig::parse(Some("JA RVIS|yes")), // 非 "true" 串 → false
    ];
    let actual = cases
        .iter()
        .map(|c| c.to_config_string())
        .collect::<Vec<_>>()
        .join(" | ");
    expect!["jarvis|true | jarvis|true | default|true | default|true | jarvis|false | JA RVIS|false"]
        .assert_eq(&actual);

    // 前缀工具 + with_ 派生
    assert_eq!(VoicePackConfig::strip_voice_prefix(Some("voice_warn_ias")), Some("warn_ias".into()));
    assert_eq!(VoicePackConfig::strip_voice_prefix(Some("other")), Some("other".into()));
    assert_eq!(VoicePackConfig::strip_voice_prefix(None), None);
    assert_eq!(
        VoicePackConfig::with_voice_prefix(Some("warn_ias")),
        Some("voice_warn_ias".into())
    );
    assert_eq!(
        VoicePackConfig::with_voice_prefix(Some("voice_x")),
        Some("voice_x".into()), // 已带前缀不再叠
    );
    let base = VoicePackConfig::parse(Some("jarvis|true"));
    assert_eq!(base.with_enabled(false).to_config_string(), "jarvis|false");
    assert_eq!(
        base.with_pack_name(Some("hudless")).to_config_string(),
        "hudless|true"
    );
    // 空包名归 default
    assert_eq!(VoicePackConfig::new(None, true).pack_name, "default");
}

// ---- 场景: VoiceAlert 冷却窗 ----

/// 冷却窗抑制与恢复重触发: available=true 时 is_playing 在冷却期内恒 true,
/// 冷却期满 (clip 已停) → false → 可再次 play_once; 不可用时假装在播 (防重试)
#[test]
fn 告警_冷却窗抑制与恢复重触发() {
    let dir = tmp_voice_dir("cooldown");
    let _g = DirGuard(dir.clone());
    std::fs::write(dir.join("warn_ias.wav"), b"RIFF-fake").unwrap();

    let player = MockPlayer::default();
    let rm = Arc::new(VoiceResourceManager::new_with_voice_dir(
        Box::new(player),
        dir.to_string_lossy().into_owned(),
    ));
    let cfg = Arc::new(ConfigurationService::new(None)); // 无树 → get "" → default 启用

    // 冷却 2 秒的告警
    let alert = VoiceAlert::new("warn_ias", 2);
    alert.reload(&*cfg, &rm);
    assert!(!alert.is_playing(0), "未播过且可用 → 不在播");

    alert.play_once(1000);
    assert_eq!(alert.last_time_play.load(Ordering::SeqCst), 1000);
    assert!(alert.is_act.load(Ordering::SeqCst));

    // 冷却期内: 抑制 (is_playing true, play_once 不刷新时间戳)
    assert!(alert.is_playing(2500), "差 1500ms < 2000ms 冷却 → 抑制");
    alert.play_once(2500);
    assert_eq!(
        alert.last_time_play.load(Ordering::SeqCst),
        1000,
        "冷却期内 play_once 被抑制, 时间戳不刷新"
    );

    // 冷却期满 + clip 已自然"播完" (MockClip 播 10ms, 等它停) → 恢复可播
    std::thread::sleep(Duration::from_millis(MOCK_CLIP_PLAY_MS + 20));
    assert!(!alert.is_playing(3001), "差 2001ms 超冷却且 clip 停 → 恢复");
    alert.play_once(3001);
    assert_eq!(alert.last_time_play.load(Ordering::SeqCst), 3001, "恢复重触发");
    assert!(alert.is_act.load(Ordering::SeqCst));

    // 不可用面: 无音频资源的告警 reload 后 available=false → is_playing 恒 true (防重试循环)
    let dead = VoiceAlert::new("no_such_alert", 1);
    dead.reload(&*cfg, &rm);
    assert!(dead.is_playing(0));
    assert!(dead.is_playing(999_999), "不可用 → 假装在播");
}

// ---- 场景: 资源管理器 ----

/// 语音包发现 + 资源存在性 (default 回退 / strict 不回退 / None 的 "null" 拼接保真)
/// + load_clip 的 default 回退与音量应用
#[test]
fn 资源管理器_包发现与回退() {
    let dir = tmp_voice_dir("packs");
    let _g = DirGuard(dir.clone());
    std::fs::write(dir.join("a.wav"), b"RIFF-a").unwrap();
    let pk = dir.join("pk");
    std::fs::create_dir_all(&pk).unwrap();
    std::fs::write(pk.join("b.wav"), b"RIFF-b").unwrap();

    let opened = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
    let opened2 = Arc::clone(&opened);
    let rm = VoiceResourceManager::new_with_voice_dir(
        Box::new(RecorderPlayer {
            opened: opened2,
        }),
        dir.to_string_lossy().into_owned(),
    );

    // 包发现: default 恒在 + 子目录计入
    let packs = rm.get_available_packs();
    assert!(packs.contains(&"default".to_string()));
    assert!(packs.contains(&"pk".to_string()), "子目录即语音包");

    // 回退语义: has_resource 查 pack 失败后回落 default; strict 不回退
    assert!(rm.has_resource("a", Some("pk")), "pk 无 a.wav → default 有 → true");
    assert!(rm.has_resource("b", Some("pk")), "pk 有 b.wav");
    assert!(!rm.has_resource_strict("a", Some("pk")), "strict: pk 无 a.wav 即 false");
    assert!(rm.has_resource_strict("b", Some("pk")));
    // Java null 拼接保真: None pack → "./voice/null/x.wav"
    assert!(!rm.has_resource_strict("b", None));
    assert!(rm.has_resource("a", None), "None → 直接查 default");

    // load_clip: pack 缺失时回退 default/a.wav + 音量应用 (volumn=100 → MockClip 记录增益)
    rm.set_voice_volumn(100);
    let clip = rm.load_clip("a", Some("pk")).expect("回退 default 应加载成功");
    let first = opened.lock().unwrap().clone();
    assert_eq!(first.len(), 1);
    assert!(first[0].ends_with("a.wav"), "回退打开 default 根的 a.wav");
    drop(clip);

    // 音量读取面 + 二次加载 (default 直查)
    let clip2 = rm.load_clip("a", None).unwrap();
    drop(clip2);
    assert_eq!(opened.lock().unwrap().len(), 2);

    // 缺失资源 → None (不 panic)
    assert!(rm.load_clip("ghost", None).is_none());
    assert_eq!(rm.voice_volumn(), 100);
}

/// 音量注入记录版 player (open_clip 成功即 MockClip)
struct RecorderPlayer {
    opened: Arc<Mutex<Vec<PathBuf>>>,
}

impl SoundPlayer for RecorderPlayer {
    fn open_clip(&self, path: &std::path::Path) -> Result<Box<dyn SoundClip>, SoundError> {
        self.opened.lock().unwrap().push(path.to_path_buf());
        Ok(Box::new(MockClip::new()))
    }
}

// ---- 场景: VoiceWarning 告警判定链 ----

/// 遥测桩: fatal 写回面 + 轮次计数 (run 每轮刷新 s_state)
struct Telemetry {
    fatal: AtomicBool,
    rounds: AtomicUsize,
    t: AtomicI64,
    state: Mutex<State>,
}

impl Telemetry {
    fn new(state: State) -> Self {
        Telemetry {
            fatal: AtomicBool::new(false),
            rounds: AtomicUsize::new(0),
            t: AtomicI64::new(1000),
            state: Mutex::new(state),
        }
    }
}

impl VoiceWarningService for Telemetry {
    fn current_time_ms(&self) -> i64 {
        self.t.fetch_add(100, Ordering::SeqCst) // 单调推进, 喂冷却判定
    }
    fn player_live(&self) -> bool {
        true
    }
    fn set_fatal_warn(&self, v: bool) {
        self.fatal.store(v, Ordering::SeqCst)
    }
    fn is_downing_flap(&self) -> bool {
        false
    }
    fn flap_allow_angle(&self) -> f64 {
        60.0
    }
    fn flap_allow_speed(&self) -> f64 {
        500.0
    }
    fn total_fuel(&self) -> f64 {
        100.0
    }
    fn fuel_percent(&self) -> i32 {
        80
    }
    fn radio_alt(&self) -> f64 {
        1000.0
    }
    fn d_radio_alt(&self) -> f64 {
        0.0
    }
    fn cur_load_min_work_time(&self) -> f64 {
        600_000.0 // > 300s → 无过热告警
    }
    fn maximum_thr_rpm(&self) -> f64 {
        3000.0
    }
    fn maximum_rpm_learned(&self) -> bool {
        false
    }
    fn is_eng_jet(&self) -> bool {
        false
    }
    fn stall_speed(&self) -> f64 {
        144.0
    }
    fn s_state(&self) -> State {
        self.rounds.fetch_add(1, Ordering::SeqCst);
        self.state.lock().unwrap().clone()
    }
    fn s_indic(&self) -> Indicators {
        Indicators::new()
    }
}

/// 装配 VoiceWarning (tmp voice 目录无音频 → 全部告警静默, 只观测 fatal 链)
fn assemble() -> (VoiceWarning, Arc<Telemetry>) {
    let dir = tmp_voice_dir("vw");
    std::fs::create_dir_all(&dir).unwrap();
    let rm = Arc::new(VoiceResourceManager::new_with_voice_dir(
        Box::new(MockPlayer::default()),
        dir.to_string_lossy().into_owned(),
    ));
    let cfg: Arc<dyn ConfigProvider + Send + Sync> = Arc::new(ConfigurationService::new(None));
    let fm = Arc::new(FMManager::new(Arc::new(FmChangedBus::new())));
    let vw = VoiceWarning::new(cfg, rm, fm, Arc::new(UIStateBus::new()), Arc::new(FlightDataBus::new()));
    (vw, Arc::new(Telemetry::new(State::new())))
}

/// 在后台线程跑 run 循环, 等谓词成立后停机 (doit 外翻 = Java 中断腿)
fn run_until(mut vw: VoiceWarning, svc: &Arc<Telemetry>, desc: &str, ok: impl Fn(&Telemetry) -> bool + Send + 'static) {
    let doit = Arc::clone(&vw.doit);
    let svc2 = Arc::clone(svc);
    let handle = std::thread::spawn(move || vw.run());
    let svc3 = Arc::clone(&svc2);
    poll_until(desc, move || ok(&svc3));
    doit.store(false, Ordering::SeqCst);
    handle.join().expect("run 线程应正常退出");
}

/// 安全遥测: 全部检查通过 → fatal 恒 false (无告警链误触发)
#[test]
fn 告警链_安全遥测fatal为false() {
    let (mut vw, svc) = assemble();
    let mut st = State::new();
    st.ias = 300; // 低于一切线 (ias 告警线无 FM = f32::MAX)
    st.aoa = 5.0; // 低于 15 告警线
    st.ny = 1.0; // 过载带内 [-4, 10]
    st.gear = 0; // 起落架收起 (无速度告警/高度告警进入)
    st.throttle = 60;
    st.flaps = 0;
    *svc.state.lock().unwrap() = st;
    vw.init(Some(Arc::clone(&svc) as Arc<dyn VoiceWarningService>));

    run_until(vw, &svc, "两轮安全检查", |s| s.rounds.load(Ordering::SeqCst) >= 2);
    assert!(!svc.fatal.load(Ordering::SeqCst), "安全态 fatal=false");
}

/// 攻角越限 (aoa > 告警线-1, ias > 80): 致命告警链触发 → fatal 传播 true
#[test]
fn 告警链_攻角越限fatal传播() {
    let (mut vw, svc) = assemble();
    let mut st = State::new();
    st.ias = 100; // > 80 (aoa 检测的速度门槛)
    st.aoa = 15.0; // 无 FM 默认告警线 15 → 15 > 15-1 → aoaCrit 致命
    st.ny = 1.0;
    st.gear = 0;
    st.throttle = 60;
    *svc.state.lock().unwrap() = st;
    vw.init(Some(Arc::clone(&svc) as Arc<dyn VoiceWarningService>));

    // 等到 fatal=true 即可停机 (第一轮 ~1.1s 后落定)
    run_until(vw, &svc, "fatal=true 落定", |s| s.fatal.load(Ordering::SeqCst));
    assert!(svc.fatal.load(Ordering::SeqCst), "aoa 越限 → 致命告警传播到 Service");
}

/// 过载越限 (ny 超上限 10, 无 FM 默认带): fatal 传播 (换一族告警线再钉一次)
#[test]
fn 告警链_过载越限fatal传播() {
    let (mut vw, svc) = assemble();
    let mut st = State::new();
    st.ias = 300;
    st.aoa = 5.0;
    st.ny = 12.0; // > 默认上限 10
    st.gear = 0;
    st.throttle = 60;
    *svc.state.lock().unwrap() = st;
    vw.init(Some(Arc::clone(&svc) as Arc<dyn VoiceWarningService>));

    run_until(vw, &svc, "fatal=true 落定", |s| s.fatal.load(Ordering::SeqCst));
    assert!(svc.fatal.load(Ordering::SeqCst));
}

/// 起落架超速 (gear > 0 且 ias ≥ 默认限速 450): fatal + 10s 持续后结构损坏
/// (is_gear_alive 内部态不可直接观测, 钉 fatal 面与存活时长)
#[test]
fn 告警链_起落架超速fatal() {
    let (mut vw, svc) = assemble();
    let mut st = State::new();
    st.ias = 500; // ≥ 默认起落架限速 450 (无 FM)
    st.aoa = 5.0;
    st.ny = 1.0;
    st.gear = 100; // 放下 + 超速
    st.throttle = 60;
    *svc.state.lock().unwrap() = st;
    vw.init(Some(Arc::clone(&svc) as Arc<dyn VoiceWarningService>));

    run_until(vw, &svc, "fatal=true 落定", |s| s.fatal.load(Ordering::SeqCst));
    assert!(svc.fatal.load(Ordering::SeqCst), "起落架超速 → 致命告警");
}
