//! 引擎控制面板 7 仪表定义表 (ui/overlay/EngineControlOverlay.java 溯源)。
//! 运行态 (量程闩锁/隐藏门控/optimal 标记/节流) 见 widgets::engine_gauge
//! 的 EngineGaugeWidget; 本表仅静态定义源 (widgets 与 voidmei 表单共用)。

use kernel::lang::Lang;

/// EngineControlOverlay GaugeType 枚举 (ordinal 即 gaugeType 字段值)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeType {
    Throttle,
    Pitch,
    Power,
    Mixture,
    Radiator,
    Compressor,
    Fuel,
}

/// Lang 标签访问器 (cfg 无 lang 快照, EngineControl 标签全部来自 Lang 静态字段)
fn lbl_throttle(l: &Lang) -> &'static str {
    l.e_throttle
}
fn lbl_proppitch(l: &Lang) -> &'static str {
    l.e_proppitch
}
fn lbl_power_percent(l: &Lang) -> &'static str {
    l.e_power_percent
}
fn lbl_mixture(l: &Lang) -> &'static str {
    l.e_mixture
}
fn lbl_radiator(l: &Lang) -> &'static str {
    l.e_radiator
}
fn lbl_compressor(l: &Lang) -> &'static str {
    l.e_compressor
}
fn lbl_fuel_per(l: &Lang) -> &'static str {
    l.e_fuel_per
}

/// 单个仪表定义 (EngineControlOverlay.initGaugeFields 的 addGaugeIfEnabled 参数快照,
/// ui_layout.cfg "引擎控制"→"发动机元素" 组的 switch-inv :target 即 disableKey)。
/// 无 PartialEq — label 是 fn 指针, 地址比较无意义 (rustc 同款告警)
#[derive(Debug, Clone, Copy)]
pub struct EngineGaugeDef {
    /// 开关键 ("true" 时该仪表不建)
    pub disable_key: &'static str,
    /// 字段键 (GaugeField key)
    pub key: &'static str,
    /// Lang 标签访问器
    pub label: fn(&Lang) -> &'static str,
    pub unit: &'static str,
    pub gauge_type: GaugeType,
    pub max_value: i32,
    pub is_horizontal: bool,
}

/// initGaugeFields 的 7 条定义, 顺序原样
pub const ENGINE_GAUGE_DEFS: &[EngineGaugeDef] = &[
    EngineGaugeDef {
        disable_key: "disableEngineInfoThrottle",
        key: "throttle",
        label: lbl_throttle,
        unit: "%",
        gauge_type: GaugeType::Throttle,
        max_value: 110,
        is_horizontal: false,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoPitch",
        key: "pitch",
        label: lbl_proppitch,
        unit: "%",
        gauge_type: GaugeType::Pitch,
        max_value: 100,
        is_horizontal: false,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoPower",
        key: "power",
        label: lbl_power_percent,
        unit: "%",
        gauge_type: GaugeType::Power,
        max_value: 100,
        is_horizontal: false,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoMixture",
        key: "mixture",
        label: lbl_mixture,
        unit: "%",
        gauge_type: GaugeType::Mixture,
        max_value: 120,
        is_horizontal: true,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoRadiator",
        key: "radiator",
        label: lbl_radiator,
        unit: "%",
        gauge_type: GaugeType::Radiator,
        max_value: 100,
        is_horizontal: true,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoCompressor",
        key: "compressor",
        label: lbl_compressor,
        unit: "",
        gauge_type: GaugeType::Compressor,
        max_value: 1,
        is_horizontal: true,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoLFuel",
        key: "fuel",
        label: lbl_fuel_per,
        unit: "%",
        gauge_type: GaugeType::Fuel,
        max_value: 100,
        is_horizontal: true,
    },
];

/// serviceLoopIntervalMs × 2 — EngineControl 节流间隔倍率
/// (widgets::engine_gauge 消费; 默认间隔 100ms 见 layout::ui_constants::ENGINE_DEFAULT_REFRESH_MS)
pub const ENGINE_REFRESH_MULTIPLIER: f64 = 2.0;
