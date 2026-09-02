//! 激活缓存 + 注册参数快照 (win32 线程的配置面)。重构波2 自 app_shell.rs 拆出。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use vm_core::config::config_api::{ConfigProvider, HudSettingsSnapshot, OverlaySettings};
use vm_core::config::configuration_service::{ConfigurationService, GlobalColors};

use crate::controller_shared::ControllerShared;
use crate::env::{java_parse_boolean, Env};

/// 激活策略引用的全部配置键 (Java registerGameModeOverlays 的
/// ActivationStrategy.config(...) 实参 + 复合策略依赖键)
pub const ACTIVATION_KEYS: [&str; 9] = [
    "enableEngineControl",
    "engineInfoSwitch",
    "crosshairSwitch",
    "flightInfoSwitch",
    "enableAxis",
    "enableAttitudeIndicator",
    "enablegearAndFlaps",
    "enableVoiceWarn",
    "enableFMPrint",
];

/// key → 原始配置串 (get_config 值域, Some("") 表缺失 — ConfigurationService 先例)。
/// 主线程刷新 (rebuild + 每次 CONFIG_CHANGED), win32 激活探测读。
pub type ActivationCache = Arc<Mutex<HashMap<String, String>>>;

/// 主线程从配置服务重建激活缓存 (Java: shouldActivate 经 ctx.get_bool →
/// configProvider.getConfig 实时读; Rust 以"每次配置变更即刷新缓存"等价,
/// 配置写点必发 CONFIG_CHANGED, 最后写胜出)
pub(crate) fn refresh_activation_cache(config: &ConfigurationService, cache: &ActivationCache) {
    let mut m = cache.lock().expect("激活缓存锁中毒");
    for key in ACTIVATION_KEYS {
        m.insert(key.to_string(), config.get_config(key).unwrap_or_default());
    }
}

/// overlay 注册面的 Send 参数快照 (win32 线程一次性注册用, D8: 字体→win32 线程)。
/// PORT(WYSIWYG 收口, 原审查 A-W4): 本快照仍只喂 spawn 期初始注册; 配置变更后的
/// 重建经 [`vm_overlay::ReinitParams`] 走 `UiCommand::ReinitOverlays` (见 vm-overlay reinit.rs
/// 头注) — 主线程 CONFIG_CHANGED 时即时重建参数包直送 win32 线程的线程局部仓,
/// 各 spec 工厂的 reinit 闭包消费, 不再冻结在 spawn 时刻。
pub struct OverlayInputs {
    pub dpi_scale: f64,
    /// MiniHUD 全量设置快照
    pub hud: HudSettingsSnapshot,
    /// 引擎控制面板字号增量 (getOverlaySettings("引擎控制").get_font_size_add)
    pub font_add_engine: i32,
    /// 动力信息字号增量 + 列数 (getOverlaySettings("动力信息"))
    pub font_add_power: i32,
    pub power_columns: i32,
    /// 飞行信息字号增量 + 列数 (getOverlaySettings("飞行信息"); Java Controller:683)
    pub font_add_flight: i32,
    pub flight_columns: i32,
    /// 起落襟翼字号增量 + 边缘模式 (getOverlaySettings("起落襟翼"))
    pub font_add_gear: i32,
    pub gear_show_edge: bool,
    /// 舵面值字号增量 + 边缘模式 (getOverlaySettings("舵面值"); Java :683)
    pub font_add_axis: i32,
    pub axis_show_edge: bool,
    /// FM拆包数据字号增量 (getOverlaySettings("FM拆包数据"); Java :731-736 —
    /// cfg 该组无字号滑条, 恒默认 0, setupFont 的 14+add 面)
    pub font_add_fm: i32,
    /// 地平仪几何/开关 (getOverlaySettings("地平仪"); 缺省 = Java reinitConfig 默认:
    /// 150×300 / 40ms / direction false / AoA 极限 true, AttitudeOverlay.java:232-248)
    pub attitude_width: i32,
    pub attitude_height: i32,
    pub attitude_freq_ms: i64,
    pub attitude_show_direction: bool,
    pub attitude_show_aoa_limits: bool,
    /// Service 轮询间隔 (MiniHUD blinkTicks/refreshInterval 同源;
    /// EngineControl loadRefreshInterval 读的 dataPollIntervalMs 亦同源)
    pub service_loop_interval_ms: i64,
    /// 全局五色快照 (Java Application.colorNum 族静态; cfg fontNum/fontLabel/
    /// fontUnit/fontWarn/fontShade → win32 线程 global_colors 仓)
    pub colors: GlobalColors,
    /// AA 开关快照 (cfg AAEnable, Java cfg 缺省 false; → global_aa 仓)
    pub aa: bool,
    /// 引擎控制 7 仪表 disable 开关 (ENGINE_DISABLE_KEYS 序; 曾 never-wired
    /// 恒 false — 用户关仪表 Rust 恒显全部, 启动首帧即错, 审查轮 1-B)
    pub engine_disables: [bool; 7],
    /// W-D cfg 驱动行定义 (行开关过滤后)
    pub flight_rows: std::sync::Arc<Vec<vm_core::ui_support::row_def::RowDef>>,
    pub power_rows: std::sync::Arc<Vec<vm_core::ui_support::row_def::RowDef>>,
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
        let compile_rows = |title: &str| -> std::sync::Arc<Vec<vm_core::ui_support::row_def::RowDef>> {
            match config.get_overlay_settings(title).get_group_config() {
                Some(gc) => {
                    // 行开关 (is_field_disabled = Java isFieldDisabled): value=false
                    // 的 data 行不进面板 — Rust 侧此前 no-op, W-D 接线修复
                    let rows = vm_core::ui_support::row_def::rows_from_group(gc, &|r| {
                        let key = r.property.clone().unwrap_or_else(|| r.label.clone());
                        ConfigProvider::is_field_disabled(config, &key)
                    });
                    std::sync::Arc::new(rows)
                }
                None => std::sync::Arc::new(Vec::new()),
            }
        };
        OverlayInputs {
            dpi_scale: env.dpi.get_scale(),
            hud: HudSettingsSnapshot::build(&config.get_hud_settings()),
            font_add_engine: engine.get_font_size_add(),
            font_add_power: power.get_font_size_add(),
            power_columns: power.get_int("hudColumns", 1),
            font_add_flight: flight.get_font_size_add(),
            flight_columns: flight.get_int("flightInfoColumn", 1),
            font_add_gear: gear.get_font_size_add(),
            gear_show_edge: gear.get_bool("enablegearAndFlapsEdge", false),
            font_add_axis: axis.get_font_size_add(),
            axis_show_edge: axis.get_bool("enableAxisEdge", false),
            font_add_fm: fm_print.get_font_size_add(),
            attitude_width: attitude.get_int("attitudeIndicatorWidth", 150),
            attitude_height: attitude.get_int("attitudeIndicatorHeight", 300),
            attitude_freq_ms: attitude.get_int("attitudeIndicatorFreqMs", 40) as i64,
            attitude_show_direction: attitude
                .get_bool("attitudeIndicatorDisplayDirection", false),
            attitude_show_aoa_limits: attitude
                .get_bool("attitudeIndicatorDisplayAoALimits", true),
            // load_app_check 缺省 50 (ConfigurationService.java 同源)
            service_loop_interval_ms: if interval > 0 { interval } else { 50 },
            colors: config.global_colors(),
            aa: config.application_state().aa_enable,
            engine_disables: std::array::from_fn(|i| {
                config
                    .get_config(vm_overlay::ENGINE_DISABLE_KEYS[i])
                    .map(|v| java_parse_boolean(&v))
                    .unwrap_or(false)
            }),
            flight_rows: compile_rows("飞行信息"),
            power_rows: compile_rows("动力信息"),
        }
    }
}

/// 注册快照 → WYSIWYG reinit 参数包 (同源配置键的子集投影; 颜色/AA 有专命令不入包)
impl From<&OverlayInputs> for vm_overlay::ReinitParams {
    fn from(i: &OverlayInputs) -> Self {
        vm_overlay::ReinitParams {
            dpi_scale: i.dpi_scale,
            font_add_engine: i.font_add_engine,
            engine_disables: i.engine_disables,
            service_loop_interval_ms: i.service_loop_interval_ms,
            font_add_power: i.font_add_power,
            power_columns: i.power_columns,
            font_add_flight: i.font_add_flight,
            flight_columns: i.flight_columns,
            font_add_gear: i.font_add_gear,
            gear_show_edge: i.gear_show_edge,
            font_add_axis: i.font_add_axis,
            axis_show_edge: i.axis_show_edge,
            font_add_fm: i.font_add_fm,
            attitude_width: i.attitude_width,
            attitude_height: i.attitude_height,
            attitude_freq_ms: i.attitude_freq_ms,
            attitude_show_direction: i.attitude_show_direction,
            attitude_show_aoa_limits: i.attitude_show_aoa_limits,
            hud: i.hud.clone(),
            flight_rows: std::sync::Arc::clone(&i.flight_rows),
            power_rows: std::sync::Arc::clone(&i.power_rows),
        }
    }
}
