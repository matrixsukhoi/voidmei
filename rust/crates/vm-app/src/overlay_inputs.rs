//! 激活缓存 + 注册参数快照 (渲染线程的配置面)。重构波2 自 app_shell.rs 拆出。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use vm_core::config::config_api::{ConfigProvider, HudSettingsSnapshot, OverlaySettings};
use vm_core::config::configuration_service::{ConfigurationService, GlobalColors};

use crate::controller_shared::ControllerShared;
use crate::env::Env;

/// 激活策略引用的非页面配置键 (R3 声明式后仅剩语音告警; 页面激活键由
/// 页文档 activation.key 派生, 见 refresh_activation_cache)
pub const ACTIVATION_KEYS: [&str; 1] = ["enableVoiceWarn"];

/// key → 原始配置串 (get_config 值域, Some("") 表缺失 — ConfigurationService 先例)。
/// 主线程刷新 (rebuild + 每次 CONFIG_CHANGED), 渲染线程激活探测读。
pub type ActivationCache = Arc<Mutex<HashMap<String, String>>>;

/// 主线程从配置服务重建激活缓存 (Java: shouldActivate 经 ctx.get_bool →
/// configProvider.getConfig 实时读; Rust 以"每次配置变更即刷新缓存"等价,
/// 配置写点必发 CONFIG_CHANGED, 最后写胜出)
pub(crate) fn refresh_activation_cache(config: &ConfigurationService, cache: &ActivationCache) {
    let mut m = cache.lock().expect("激活缓存锁中毒");
    for key in ACTIVATION_KEYS {
        m.insert(key.to_string(), config.get_config(key).unwrap_or_default());
    }
    // 页激活键并集 (R3 声明式: 全部页 (出厂+用户) 的 activation.key —
    // 不入缓存则激活探测 get_bool 恒 false, 页面永不激活)
    for page in config.pages().iter() {
        if let Some(a) = &page.activation {
            if !a.key.is_empty() {
                m.insert(a.key.clone(), config.get_config(&a.key).unwrap_or_default());
            }
        }
    }
}

/// overlay 注册面的 Send 参数快照 (渲染线程一次性注册用, D8: 字体→渲染线程)。
/// PORT(WYSIWYG 收口, 原审查 A-W4): 本快照仍只喂 spawn 期初始注册; 配置变更后的
/// 重建经 [`vm_overlay::platform::reinit::ReinitParams`] 走 `UiCommand::ReinitOverlays`
/// (见 vm-overlay reinit.rs 头注) — 主线程 CONFIG_CHANGED 时即时重建参数包直送
/// 渲染线程的线程局部仓, 各 spec 工厂的 reinit 闭包消费, 不再冻结在 spawn 时刻。
pub struct OverlayInputs {
    pub dpi_scale: f64,
    /// MiniHUD 全量设置快照
    pub hud: HudSettingsSnapshot,
    /// HUD 页面清单 (PageDoc 驱动建树; 主线程出厂 ⊕ delta 后快照)
    pub pages: std::sync::Arc<Vec<vm_core::config::json_model::PageDoc>>,
    /// 引擎控制面板字号增量 (getOverlaySettings("引擎控制").get_font_size_add)
    pub font_add_engine: i32,
    /// 动力信息字号增量 (getOverlaySettings("动力信息"))
    pub font_add_power: i32,
    /// 飞行信息字号增量 (getOverlaySettings("飞行信息"))
    pub font_add_flight: i32,
    /// 起落襟翼字号增量 + 边缘模式 (getOverlaySettings("起落襟翼"))
    pub font_add_gear: i32,
    pub gear_show_edge: bool,
    /// 舵面值字号增量 + 边缘模式 (getOverlaySettings("舵面值"))
    pub font_add_axis: i32,
    pub axis_show_edge: bool,
    /// FM拆包数据字号增量 (getOverlaySettings("FM拆包数据");
    /// cfg 该组无字号滑条, 恒默认 0, setupFont 的 14+add 面)
    pub font_add_fm: i32,
    /// 地平仪几何/开关 (getOverlaySettings("地平仪"); 缺省 = Java reinitConfig 默认:
    /// 150×300 / 40ms / direction false / AoA 极限 true)
    pub attitude_width: i32,
    pub attitude_height: i32,
    pub attitude_freq_ms: i64,
    pub attitude_show_direction: bool,
    pub attitude_show_aoa_limits: bool,
    /// Service 轮询间隔 (MiniHUD blinkTicks/refreshInterval 同源;
    /// EngineControl loadRefreshInterval 读的 dataPollIntervalMs 亦同源)
    pub service_loop_interval_ms: i64,
    /// 全局五色快照 (Java Application.colorNum 族静态; cfg fontNum/fontLabel/
    /// fontUnit/fontWarn/fontShade → 渲染线程 global_colors 仓)
    pub colors: GlobalColors,
    /// AA 开关快照 (cfg AAEnable, Java cfg 缺省 false; → global_aa 仓)
    pub aa: bool,
}

impl OverlayInputs {
    /// 主线程构建 (调用点持 ConfigurationService + Env + shared)
    pub fn build(config: &ConfigurationService, env: &Env, shared: &ControllerShared) -> Self {
        let interval = shared
            .intervals
            .lock()
            .expect("intervals 锁中毒")
            .service_loop_interval_ms;
        let engine = config.get_overlay_settings("引擎控制");
        let power = config.get_overlay_settings("动力信息");
        let gear = config.get_overlay_settings("起落襟翼");
        let axis = config.get_overlay_settings("舵面值");
        let fm_print = config.get_overlay_settings("FM拆包数据");
        let attitude = config.get_overlay_settings("地平仪");
        let flight = config.get_overlay_settings("飞行信息");
        // bool 全键面 (页面组件 visibleWhen 的配置键求值源 — 只靠 build 的
        // enableLayoutDebug 单键会令带条件的组件在真窗恒消失/恒显)
        let mut hud = HudSettingsSnapshot::build(&config.get_hud_settings());
        hud.bools = all_bool_rows(config);
        OverlayInputs {
            dpi_scale: env.dpi.get_scale(),
            hud,
            pages: config.pages(),
            font_add_engine: engine.get_font_size_add(),
            font_add_power: power.get_font_size_add(),
            font_add_flight: flight.get_font_size_add(),
            font_add_gear: gear.get_font_size_add(),
            gear_show_edge: gear.get_bool("enablegearAndFlapsEdge", false),
            font_add_axis: axis.get_font_size_add(),
            axis_show_edge: axis.get_bool("enableAxisEdge", false),
            font_add_fm: fm_print.get_font_size_add(),
            attitude_width: attitude.get_int("attitudeIndicatorWidth", 150),
            attitude_height: attitude.get_int("attitudeIndicatorHeight", 300),
            attitude_freq_ms: attitude.get_int("attitudeIndicatorFreqMs", 40) as i64,
            attitude_show_direction: attitude.get_bool("attitudeIndicatorDisplayDirection", false),
            attitude_show_aoa_limits: attitude.get_bool("attitudeIndicatorDisplayAoALimits", true),
            // load_app_check 缺省 50 (ConfigurationService.java 同源)
            service_loop_interval_ms: if interval > 0 { interval } else { 50 },
            colors: config.global_colors(),
            aa: config.application_state().aa_enable,
        }
    }
}

/// 配置树全部 bool 行收集 (property → value; 嵌套 HEADER 递归) —
/// 面板标题集 = 出厂标题 (面板集固定, 用户不可增删)
fn all_bool_rows(config: &ConfigurationService) -> HashMap<String, bool> {
    let mut out = HashMap::new();
    fn walk(rows: &[vm_core::config::json_model::RowConfig], out: &mut HashMap<String, bool>) {
        for r in rows {
            if let (Some(key), Some(vm_core::config::json_model::ConfigValue::Bool(v))) =
                (&r.property, &r.value)
            {
                out.insert(key.clone(), *v);
            }
            walk(&r.children, out);
        }
    }
    for title in vm_core::config::json_store::factory()
        .panels
        .iter()
        .map(|p| p.title.clone())
    {
        if let Some(gc) = config.get_overlay_settings(&title).get_group_config() {
            walk(&gc.rows, &mut out);
        }
    }
    out
}

/// 注册快照 → WYSIWYG reinit 参数包 (同源配置键的子集投影; 颜色/AA 有专命令不入包)。
/// F15: ReinitParams 分组嵌套, 本快照保持平铺 (spawn 期一次性构建, 无分组收益)
impl From<&OverlayInputs> for vm_overlay::platform::reinit::ReinitParams {
    fn from(i: &OverlayInputs) -> Self {
        use vm_overlay::platform::reinit::{
            AttitudeGroup, EdgeGroup, EngineGroup, FmGroup, ListGroup,
        };
        vm_overlay::platform::reinit::ReinitParams {
            dpi_scale: i.dpi_scale,
            service_loop_interval_ms: i.service_loop_interval_ms,
            attitude_freq_ms: i.attitude_freq_ms,
            engine: EngineGroup {
                font_add: i.font_add_engine,
            },
            power: ListGroup {
                font_add: i.font_add_power,
            },
            flight: ListGroup {
                font_add: i.font_add_flight,
            },
            gear: EdgeGroup {
                font_add: i.font_add_gear,
                show_edge: i.gear_show_edge,
            },
            axis: EdgeGroup {
                font_add: i.font_add_axis,
                show_edge: i.axis_show_edge,
            },
            fm: FmGroup {
                font_add: i.font_add_fm,
            },
            attitude: AttitudeGroup {
                width: i.attitude_width,
                height: i.attitude_height,
                show_direction: i.attitude_show_direction,
                show_aoa_limits: i.attitude_show_aoa_limits,
            },
            pages: std::sync::Arc::clone(&i.pages),

            hud: i.hud.clone(),
        }
    }
}
