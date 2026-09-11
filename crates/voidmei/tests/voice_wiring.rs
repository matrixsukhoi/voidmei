//! VoiceWarning 装配黑盒场景: open_voice_warning 会话生命周期 (起线程/tick
//! 副作用/停机注销)、告警帧播放路径 (CountingPlayer 计数替身)、静音门控。
//! 播放器经 VoiceResourceManager 注入面 (new_with_voice_dir) 换 mock — 全程
//! 不触音频设备; 遥测经手造 live 帧仓 (fixture 先例), 不 spawn 渲染线程。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


mod common;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::*;
use expect_test::expect;
use kernel::audio::voice_resource_manager::{
    SoundClip, SoundPlayer, VoiceResourceManager,
};
use kernel::base::event::ui_state_events;
use voidmei::open_voice_warning;

// ------------------------------------------------------------------
// 计数替身 (mock SoundPlayer — open_clip 永成功, start 计数按文件名键)
// ------------------------------------------------------------------

struct CountingPlayer {
    plays: Arc<Mutex<HashMap<String, usize>>>,
}

impl SoundPlayer for CountingPlayer {
    fn open_clip(&self, path: &Path) -> Result<Box<dyn SoundClip>, kernel::audio::voice_resource_manager::SoundError> {
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

impl SoundClip for CountingClip {
    fn start(&self) {
        *self.plays.lock().unwrap().entry(self.key.clone()).or_insert(0) += 1;
    }
    fn stop(&self) {}
    fn is_running(&self) -> bool {
        false // 恒停: 冷却由时间项压制 (本文件遥测 current_time_ms 恒 0)
    }
    fn set_frame_position(&self, _frame: i32) {}
    fn close(&self) {}
    fn master_gain_range(&self) -> Option<(f32, f32)> {
        None // Control not supported → applyVolume 跳过 (Java 空 catch 面)
    }
    fn set_master_gain(&self, _value: f32) {}
}

// ------------------------------------------------------------------
// 辅助
// ------------------------------------------------------------------

/// 致命告警形态帧: 起落架放下 (gear=100) + 超速 (IAS=500 ≥ 无 FM 默认限速 450)
/// → checkGearWarning 置 fatal — 用作 "至少一轮 tick 已跑" 的信号
/// (fatal_warn 是 run() 唯一稳定外显副作用; 不能用初值判定, 恒 false)
fn fatal_gear_frame() -> Arc<data::frame::FrameStore> {
    let mut d = live_service_data("spitfire");
    d.s_state.as_mut().unwrap().ias = 500;
    d.s_state.as_mut().unwrap().gear = 100;
    live_store_of(&d)
}

/// 等 tick 写 fatalWarn (启动延迟 1s + 100ms 节拍; 轮询 8s 超时即失败 — 不假通过)
fn wait_fatal(store: &data::frame::FrameStore, desc: &str) {
    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    while !store.fatal_warn() {
        assert!(
            std::time::Instant::now() < deadline,
            "{desc}: 8s 内至少一轮 tick 应写 fatalWarn (线程未跑 = 装配失败)"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// CONFIG_CHANGED 探测送达数 (configHandler 在位的观测面; 探针键触发 reload
/// 无副作用 — 测试 CWD voice/ 无音频文件, load_clip 静默 None)
fn delivery_of(shell: &voidmei::AppShell) -> usize {
    shell
        .ui_bus
        .publish(ui_state_events::CONFIG_CHANGED, Some("probe"), Some("voice_aoaCrit"))
}

/// tmp voice 目录 (mock wav 摆放点; 用例名隔离并行测试)
fn tmp_voice_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("vm_it_voice_{tag}_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// 计数快照 (HashMap 迭代序不稳定 → BTreeMap 定序后入快照)
fn counted(plays: &Arc<Mutex<HashMap<String, usize>>>) -> BTreeMap<String, usize> {
    plays.lock().unwrap().clone().into_iter().collect()
}

// ------------------------------------------------------------------
// 场景
// ------------------------------------------------------------------

/// open_voice_warning 起线程跑 live: 会话句柄在 (Some) + 装配订阅在位
/// (FlightDataBus +1 / configHandler 送达 +1) + fatalWarn 有人写 (tick 真在跑)。
/// live 缺失 (Java init(S=null) 短路) → None 不起会话。
#[test]
fn 语音告警_live在位起线程跑tick() {
    let shell = fixture();
    let store = fatal_gear_frame();
    let base_sub = shell.flight_bus.subscriber_count();
    let base_delivery = delivery_of(&shell);

    let mut session = open_voice_warning(
        &shell.voice,
        &shell.ui_bus,
        &shell.config_snapshots.voice,
        &shell.fm,
        &shell.flight_bus,
        Some(Arc::clone(&store)),
    )
    .expect("live 在位时应启动会话 (Java init(S) 非短路)");
    assert_eq!(
        shell.flight_bus.subscriber_count(),
        base_sub + 1,
        "会话存活期 FlightDataBus 订阅应在"
    );
    assert_eq!(
        delivery_of(&shell),
        base_delivery + 1,
        "configHandler 应在订阅中 (+1 送达)"
    );
    wait_fatal(&store, "live 会话");
    session.stop();

    // live=None: 不起线程 (openpad 前提缺失的短路形态)
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

/// 告警帧触发播放路径 (CountingPlayer 替身): init 的 start1 启动音 + tick 的
/// warn_gear 告警音各播一次 (current_time_ms 恒 0 → 冷却压制重播, 计数恒 1)。
#[test]
fn 语音告警_告警帧触发播放路径() {
    let shell = fixture();
    // tmp voice 目录 + 两个告警 wav (内容任意 — mock 播放器不解析,
    // 仅供 resolve_audio_file 的 exists 探测命中)
    let dir = tmp_voice_dir("play");
    std::fs::write(dir.join("start1.wav"), b"mock").unwrap();
    std::fs::write(dir.join("warn_gear.wav"), b"mock").unwrap();
    let plays: Arc<Mutex<HashMap<String, usize>>> = Arc::new(Mutex::new(HashMap::new()));
    let mgr = Arc::new(VoiceResourceManager::new_with_voice_dir(
        Box::new(CountingPlayer { plays: Arc::clone(&plays) }),
        dir.to_string_lossy().into_owned(),
    ));

    let store = fatal_gear_frame();
    let mut session = open_voice_warning(
        &mgr,
        &shell.ui_bus,
        &shell.config_snapshots.voice,
        &shell.fm,
        &shell.flight_bus,
        Some(Arc::clone(&store)),
    )
    .expect("live 在位应建会话");
    wait_fatal(&store, "播放路径");
    // fatalWarn=true 时同 tick 的 warn_gear 播放已发生 (check 内 play 先于
    // fatal 累积写回), 此刻计数就绪
    let snap = expect![[r#"{"start1": 1, "warn_gear": 1}"#]];
    snap.assert_eq(&format!("{:?}", counted(&plays)));
    session.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

/// 会话关闭停线程 (Java OverlayEntry.close 的 interrupt 形态): doit 翻 false
/// + join 收线程 + 双总线订阅注销回落 (Java 泄漏点的根治面) + 幂等。
#[test]
fn 语音告警_会话停止收线程并注销订阅() {
    let shell = fixture();
    let store = fatal_gear_frame(); // live 槽在位即可 (openpad 前提), 不等 tick
    let base_sub = shell.flight_bus.subscriber_count();
    let base_delivery = delivery_of(&shell);

    let mut session = open_voice_warning(
        &shell.voice,
        &shell.ui_bus,
        &shell.config_snapshots.voice,
        &shell.fm,
        &shell.flight_bus,
        Some(Arc::clone(&store)),
    )
    .expect("live 在位应建会话");
    assert_eq!(shell.flight_bus.subscriber_count(), base_sub + 1);

    session.stop();
    assert!(!session.doit.load(Ordering::SeqCst), "停机后 doit 应为 false");
    assert_eq!(
        shell.flight_bus.subscriber_count(),
        base_sub,
        "停机后 FlightDataBus 订阅应注销 (RAII)"
    );
    assert_eq!(
        delivery_of(&shell),
        base_delivery,
        "configHandler 应随线程退出注销, 回到 Controller 常驻基线"
    );
    session.stop(); // 幂等 (Drop 兜底同款)
}

/// 静音门控: 全告警键配置 "default|false" (enabled=false) → reload 短路
/// (available=false, 不触 load_clip), tick 照跑且告警条件成立, 但零播放。
/// wav 文件在位 — 排除 "文件缺失导致不播" 的混淆, 门控唯一变量是配置。
#[test]
fn 语音告警_静音配置关闭时不播() {
    let shell = fixture();
    let dir = tmp_voice_dir("mute");
    std::fs::write(dir.join("start1.wav"), b"mock").unwrap();
    std::fs::write(dir.join("warn_gear.wav"), b"mock").unwrap();
    let plays: Arc<Mutex<HashMap<String, usize>>> = Arc::new(Mutex::new(HashMap::new()));
    let mgr = Arc::new(VoiceResourceManager::new_with_voice_dir(
        Box::new(CountingPlayer { plays: Arc::clone(&plays) }),
        dir.to_string_lossy().into_owned(),
    ));
    // 全告警键 (含 start1) 静音: voice_<key> = "default|false"
    let mut m = HashMap::new();
    for ty in kernel::audio::voice_alert_type::ALL {
        let key = kernel::audio::VoicePackConfig::with_voice_prefix(Some(ty.get_key()))
            .expect("告警键非 null");
        m.insert(key, "default|false".to_string());
    }
    let voice_config = Arc::new(Mutex::new(m));

    let store = fatal_gear_frame();
    let mut session = open_voice_warning(
        &mgr,
        &shell.ui_bus,
        &voice_config,
        &shell.fm,
        &shell.flight_bus,
        Some(Arc::clone(&store)),
    )
    .expect("live 在位应建会话 (静音只门播放, 不门装配)");
    wait_fatal(&store, "静音门控"); // tick 在跑 + 告警条件成立 (fatal=true)
    let counted = counted(&plays);
    assert!(
        counted.is_empty(),
        "静音配置下不应有任何播放: {counted:?}"
    );
    session.stop();
    let _ = std::fs::remove_dir_all(&dir);
}
