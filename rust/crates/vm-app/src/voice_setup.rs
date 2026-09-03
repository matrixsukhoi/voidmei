//! VoiceWarning 装配 (Java Controller 注册 → OverlayManager
//! .open/close 的线程启停; 语音子系统装配批)。重构波2 自 app_shell.rs 拆出。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use vm_core::audio::voice_resource_manager::VoiceResourceManager;
use vm_core::audio::voice_warning::{VoiceWarning, VoiceWarningService};
use vm_core::base::bus::flight_data_bus::FlightDataBus;
use vm_core::base::bus::ui_state_bus::UIStateBus;
use vm_core::config::config_api::ConfigProvider;
use vm_core::config::configuration_service::ConfigurationService;
use vm_core::fm::FMManager;
use vm_core::formula::registry::FormulaView as _; // var_value 取数唯一接口

use crate::keys::FM_FIELD_KEYS;

/// 配置跨线程快照对 (E9b): AppShell 原平铺的 voice_config/fm_field_config
/// 双字段收敛体。配置服务 !Send 恒留主线程, VoiceWarning 告警线程与渲染线程
/// (FMUnpackedData generate_lines) 经值快照跨线程读; 常规写值经 write_hook 在
/// CONFIG_CHANGED 广播前直写 (快照新值先于订阅者), 托盘 rebuild 随新配置树
/// 全量重刷。
#[derive(Clone)]
pub struct ConfigSnapshots {
    /// voice_* 配置键快照 ([`SnapshotConfigProvider`] 数据面; VoiceWarning
    /// 的 reload 链跨线程读)
    pub voice: Arc<Mutex<HashMap<String, String>>>,
    /// FM拆包数据 show* 配置键快照 (FMUnpackedData 的 generate_lines 每
    /// tick 读, CONFIG_CHANGED 逐键刷新)
    pub fm_field: Arc<Mutex<HashMap<String, String>>>,
}

impl ConfigSnapshots {
    /// 全量初建 (AppShell 构造点): 两快照填充 + 挂写值钩子
    pub fn new(config: &ConfigurationService) -> Self {
        let voice = Arc::new(Mutex::new(HashMap::new()));
        refresh_voice_config_snapshot(config, &voice);
        let fm_field = Arc::new(Mutex::new(HashMap::new()));
        refresh_fm_field_config_snapshot(config, &fm_field);
        let me = ConfigSnapshots { voice, fm_field };
        me.attach_hooks(config);
        me
    }

    /// 两快照全量重刷 (调用点: 托盘 rebuild 新配置树)
    pub fn refresh(&self, config: &ConfigurationService) {
        refresh_voice_config_snapshot(config, &self.voice);
        refresh_fm_field_config_snapshot(config, &self.fm_field);
    }

    /// 挂配置写值钩子: set_config 广播 CONFIG_CHANGED 前直写跨线程快照,
    /// 保证 VoiceWarning reload (publish 栈内同步读快照) 拿到新值。
    /// 调用点: 构造 (initial_config) / 托盘 rebuild (新配置树)。
    pub fn attach_hooks(&self, config: &ConfigurationService) {
        let voice = Arc::clone(&self.voice);
        let fm = Arc::clone(&self.fm_field);
        config.set_write_hook(Box::new(move |key, value| {
            if key.starts_with("voice_") {
                voice
                    .lock()
                    .expect("voice 配置快照锁中毒")
                    .insert(key.to_string(), value.to_string());
            } else if FM_FIELD_KEYS.contains(&key) {
                fm.lock()
                    .expect("FM 字段快照锁中毒")
                    .insert(key.to_string(), value.to_string());
            }
        }));
    }
}

/// ConfigurationService (!Send, 主线程独占) 的快照适配器 (重构波2 三合一:
/// 原 FlightLogConfig 单键快照 / VoiceConfigSnapshot voice_* 全键版 /
/// FmFieldConfigSnapshot FM show* 版 — 同型只读 ConfigProvider, 合一收敛)。
/// get = 查 map, set/is_field_disabled 无调用方 (Java 侧 VoiceWarning.reload 与
/// generateLines 经 configProvider 只读)。
pub(crate) struct SnapshotConfigProvider(Arc<Mutex<HashMap<String, String>>>);

impl SnapshotConfigProvider {
    /// 现有快照 map 直接包装 (voice_*/FM show* 场景: map 由 AppShell 持有刷新)
    pub(crate) fn new(map: Arc<Mutex<HashMap<String, String>>>) -> Self {
        SnapshotConfigProvider(map)
    }

    /// 键值对构造 (FlightLogConfig 单键场景: None 值不落键 = get 返回 None,
    /// 与原 Option<String> 语义一致)
    pub(crate) fn from_pairs(
        pairs: impl IntoIterator<Item = (&'static str, Option<String>)>,
    ) -> Self {
        let mut m = HashMap::new();
        for (k, v) in pairs {
            if let Some(v) = v {
                m.insert(k.to_string(), v);
            }
        }
        SnapshotConfigProvider::new(Arc::new(Mutex::new(m)))
    }
}

impl ConfigProvider for SnapshotConfigProvider {
    fn get_config(&self, key: &str) -> Option<String> {
        self.0.lock().expect("配置快照锁中毒").get(key).cloned()
    }
    fn set_config(&self, _key: &str, _value: &str) {
        // Java 侧经 configProvider 只读 (reload 的 getConfig / generateLines)
    }
    fn is_field_disabled(&self, _key: &str) -> bool {
        false
    }
}

/// 全量刷新 voice_* 快照: 键集 = VoiceAlertType 全部告警键 (含 start1) 加前缀。
/// 调用点: [`ConfigSnapshots::new`] / [`ConfigSnapshots::refresh`] — 均主线程。
fn refresh_voice_config_snapshot(
    config: &ConfigurationService,
    snapshot: &Arc<Mutex<HashMap<String, String>>>,
) {
    let mut m = snapshot.lock().expect("voice 配置快照锁中毒");
    for ty in vm_core::audio::voice_alert_type::ALL {
        // with_voice_prefix: "voice_" + key (无前缀时补, 有则原样)
        let cfg_key = vm_core::audio::VoicePackConfig::with_voice_prefix(Some(ty.get_key()))
            .expect("告警键非 null");
        m.insert(
            cfg_key.clone(),
            config.get_config(&cfg_key).unwrap_or_default(),
        );
    }
}

/// 全量刷新 FM show* 快照 (调用点同 voice: 构造 / 托盘 rebuild)
fn refresh_fm_field_config_snapshot(
    config: &ConfigurationService,
    snapshot: &Arc<Mutex<HashMap<String, String>>>,
) {
    let mut m = snapshot.lock().expect("FM 字段快照锁中毒");
    for key in FM_FIELD_KEYS {
        m.insert(key.to_string(), config.get_config(key).unwrap_or_default());
    }
}

/// VoiceWarning 对 Service 消费面的生产实现 (VoiceWarningService trait):
/// 波4 起持帧仓 — 每 trait 方法一次零锁 clone Arc 取整帧 (原 ~20 次读锁/
/// tick + State/Indicators 整结构深拷的 B-W2 备案形态消亡); fatal_warn 写
/// 真相源原子 (Service 帧发布时镜像入帧)。
struct LiveVoiceService {
    frames: Arc<vm_data::frame::FrameStore>,
}

impl VoiceWarningService for LiveVoiceService {
    fn current_time_ms(&self) -> i64 {
        self.frames.latest().map(|f| f.current_time_ms).unwrap_or(0)
    }
    fn player_live(&self) -> bool {
        self.frames.latest().is_some_and(|f| f.player_live)
    }
    fn set_fatal_warn(&self, v: bool) {
        self.frames.set_fatal_warn(v);
    }
    fn is_downing_flap(&self) -> bool {
        // W-C: 直读公式槽, None→false
        self.frames
            .latest()
            .is_some_and(|f| f.var_value("is_downing_flap").unwrap_or(0.0) != 0.0)
    }
    fn flap_allow_angle(&self) -> f64 {
        // W-C: 直读公式槽, None→MAX(无限制, 不触发告警)
        self.frames
            .latest()
            .and_then(|f| f.var_value("flap_allow_angle"))
            .unwrap_or(f64::MAX)
    }
    fn flap_allow_speed(&self) -> f64 {
        self.frames
            .latest()
            .and_then(|f| f.var_value("flap_allow_speed"))
            .unwrap_or(f64::MAX)
    }
    fn total_fuel(&self) -> f64 {
        // 波17 F14: 派生标量按语义分组 (fuel/engine/altm), 字段路径随组
        self.frames
            .latest()
            .map(|f| f.fuel.total_fuel)
            .unwrap_or(0.0)
    }
    fn fuel_percent(&self) -> i32 {
        self.frames
            .latest()
            .map(|f| f.fuel.fuel_percent)
            .unwrap_or(0)
    }
    fn radio_alt(&self) -> f64 {
        self.frames
            .latest()
            .map(|f| f.altm.radio_alt)
            .unwrap_or(0.0)
    }
    fn d_radio_alt(&self) -> f64 {
        self.frames
            .latest()
            .map(|f| f.altm.d_radio_alt)
            .unwrap_or(0.0)
    }
    fn cur_load_min_work_time(&self) -> f64 {
        self.frames
            .latest()
            .map(|f| f.cur_load_min_work_time)
            .unwrap_or(0.0)
    }
    fn maximum_thr_rpm(&self) -> f64 {
        self.frames
            .latest()
            .map(|f| f.engine.maximum_thr_rpm)
            .unwrap_or(0.0)
    }
    fn get_maximum_rpm(&self) -> bool {
        self.frames
            .latest()
            .is_some_and(|f| f.engine.maximum_rpm_learned)
    }
    fn is_eng_jet(&self) -> bool {
        // Java Service.isEngJet() = iEngType == ENGINE_TYPE_JET;
        // 波17 F1: i32 常量族 → EngineType 枚举
        self.frames
            .latest()
            .is_some_and(|f| f.engine.engine_type == vm_data::service_fields::EngineType::Jet)
    }
    fn get_stall_speed(&self) -> f64 {
        // W-C: 直读公式槽, None→0(永不触发失速告警)
        self.frames
            .latest()
            .and_then(|f| f.var_value("stall_speed"))
            .unwrap_or(0.0)
    }
    fn s_state(&self) -> vm_core::game_api::parser::State {
        // Java st 恒非 null (Service 构造即建); 无帧/槽内 None 仅畸形帧窗口 — 零值让步
        self.frames
            .latest()
            .and_then(|f| f.s_state.clone())
            .unwrap_or_default()
    }
    fn s_indic(&self) -> vm_core::game_api::parser::Indicators {
        self.frames
            .latest()
            .and_then(|f| f.s_indic.clone())
            .unwrap_or_default()
    }
}

/// VoiceWarning 会话句柄 (Java OverlayEntry 的 instance+thread 二位一体):
/// OpenAllOverlays 建 / CloseAllOverlays 停; Drop 兜底停 (渲染线程局部声明,
/// Shutdown return 时逆序 drop 自动收线程)。
pub(crate) struct VoiceWarnSession {
    pub(crate) doit: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl VoiceWarnSession {
    /// Java OverlayEntry.close: thread.interrupt() 的电平形态 — doit 翻 false
    /// + join (run 的分片睡眠 10ms 轮询, 退出时延 ≤ 一片)
    pub(crate) fn stop(&mut self) {
        self.doit.store(false, Ordering::SeqCst);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for VoiceWarnSession {
    fn drop(&mut self) {
        self.stop();
    }
}

/// voice_warn 条目的 refreshPreviews 触达判定 (审查 W1 修复的配套):
/// Java OverlayManager.refreshPreviews 对 `isGlobalConfig(key) ||
/// entry.isInterestedIn(key)` 的条目调 refreshPreview。
/// voice_warn 非 host 条目无注册面可挂 interest — 此处复刻同款
/// 判定: 全局键集/前缀 (含 None 恒真) 走 host::is_global_config 全库唯一
/// 真相; interest 集 = Java 默认 own key ("enableVoiceWarn",
/// registerWithStrategy 无 withInterest 追加)。
pub(crate) fn voice_warn_refresh_reaches(changed_key: Option<&str>) -> bool {
    vm_overlay::platform::host::is_global_config(changed_key)
        || changed_key == Some("enableVoiceWarn")
}

/// Java OverlayEntry.open 的 VoiceWarning 专项:
/// factory.get() + init(this, S) + needsThread → new Thread(instance).start()。
/// `live`: shared.live 槽现值 (openpad 必在 start() 之后, Some 是生产形态;
/// None = Java init(S=null) 的 doit=false 短路, 不起线程)。
/// 线程名对位 Java 默认 "Thread-N" — 取语义名便于排障。
///
/// PORT(备案, 审查 W3): init 在渲染线程同步执行 ~20 个 new_alert→reload→
/// load_clip (文件读 + waveOutOpen 每路开设备), OpenAllOverlays 处理期间事件
/// 泵阻塞几十至百 ms (一次性) — Java 对位 OverlayEntry.open 在 UI 线程调 init 同
/// 样阻塞 UI 线程, 形态保真; 若后续观察到 openpad 卡顿再议预加载 (偏离 Java 时
/// 序, 需裁决)。
pub(crate) fn open_voice_warning(
    voice: &Arc<VoiceResourceManager>,
    ui_bus: &Arc<UIStateBus>,
    voice_config: &Arc<Mutex<HashMap<String, String>>>,
    fm: &Arc<FMManager>,
    flight_bus: &Arc<FlightDataBus>,
    live: Option<Arc<vm_data::frame::FrameStore>>,
) -> Option<VoiceWarnSession> {
    let frames = live?;
    let mut vw = VoiceWarning::new(
        // 原 VoiceConfigSnapshot (voice_* 全键快照) — SnapshotConfigProvider 三合一
        Arc::new(SnapshotConfigProvider::new(Arc::clone(voice_config)))
            as Arc<dyn ConfigProvider + Send + Sync>,
        Arc::clone(voice),
        Arc::clone(fm),
        Arc::clone(ui_bus),
        Arc::clone(flight_bus),
        // legacy_player: playWav/getClip 直开面 (全库无调用方), 独立 winmm 实例
        // 与 resource_manager 注入同一实现即等价 (voice_warning.rs PORT 注)
        Arc::from(crate::winmm_player::make_player())
            as Arc<dyn vm_core::audio::voice_resource_manager::SoundPlayer>,
    );
    let doit = Arc::clone(&vw.doit);
    vw.init(Some(Arc::new(LiveVoiceService { frames })));
    let join = std::thread::Builder::new()
        .name("VoiceWarning".to_string())
        .spawn(move || vw.run())
        .expect("VoiceWarning 线程创建失败");
    Some(VoiceWarnSession {
        doit,
        join: Some(join),
    })
}
