//! L0 原子变量注册表: 名字 → VarId → 取值 fn 指针 (编译期显式代码, 非反射, D4/D7)。
//! 覆盖: TelemetrySource 全量 getter (getter 别名向后兼容 :target) + FM 字段
//! (fm.* 前缀, 无 FM 时 NaN) + 元变量 + 物理常量。
//! state/indicators 原始字段直通在阶段 2-4 实际外置引用时按需增补 (见设计文档 §5)。
//! 设计: doc/formula_system_design.md §5

use super::definition::VarLookup;
use super::functions::Value;
use crate::ui_model::fm_data_source::FMDataSource;
use crate::ui_model::telemetry_source::TelemetrySource;
use std::collections::HashMap;
use std::sync::OnceLock;

/// 变量类别 (编辑器目录分组)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarCategory {
    /// 飞行数据 (速度/高度/姿态/机动)
    Flight,
    /// 引擎 (功率/温度/油量/增压器)
    Engine,
    /// 操纵面与载具状态
    State,
    /// 速度/结构限制
    Limit,
    /// FM 文件字段 (换机重载)
    Fm,
    /// 运行元信息 (帧间隔/会话时间/标志)
    Meta,
    /// 物理常量
    Const,
}

/// 数据来源 (编辑器目录标注: 变量从哪来)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarOrigin {
    /// 8111 /state 接口 (遥测原始字段, 直通或浅聚合/单位换算)
    State,
    /// 8111 /indicators 接口 (座舱仪表; **可用字段随机型而异**, 部分机型无此数据)
    Indicators,
    /// 内部派生计算 (物理模型/滤波/状态机/多接口聚合)
    Derived,
    /// FM 文件 (.blkx, 换机重载)
    Fm,
    /// 运行时元信息
    Meta,
    /// 物理常量
    Const,
}

impl VarOrigin {
    /// 编辑器目录展示文案
    pub fn label(self) -> &'static str {
        match self {
            VarOrigin::State => "8111 /state",
            VarOrigin::Indicators => "8111 /indicators",
            VarOrigin::Derived => "内部计算",
            VarOrigin::Fm => "FM 文件",
            VarOrigin::Meta => "运行时",
            VarOrigin::Const => "常量",
        }
    }
}

/// 元变量 (接线层每帧供值)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaVar {
    IntervalMs,
    Freq,
    ElapsedMs,
    SessionMs,
    FmLoaded,
    EngineCount,
}

/// 取值源 (fn 指针, 非捕获闭包强转)
pub enum VarSrc {
    Tel(fn(&dyn TelemetrySource) -> f64),
    TelBool(fn(&dyn TelemetrySource) -> bool),
    Fm(fn(&dyn FMDataSource) -> f64),
    Const(f64),
    Meta(MetaVar),
}

/// 变量元数据
pub struct VarMeta {
    /// 短名 (公式引用主名, snake_case)
    pub name: &'static str,
    /// getter 别名 (Java 驼峰, :target 兼容; 空串 = 无)
    pub getter: &'static str,
    pub unit: &'static str,
    pub desc: &'static str,
    pub category: VarCategory,
    /// 数据来源 (编辑器标注; 逐 getter 核对 service_fields.rs 实现)
    pub origin: VarOrigin,
    pub src: VarSrc,
}

/// 元变量输入 (Service 线程每帧组装)
#[derive(Debug, Clone, Copy, Default)]
pub struct MetaInputs {
    pub interval_ms: f64,
    pub freq: f64,
    pub elapsed_ms: f64,
    pub session_ms: f64,
    pub fm_loaded: bool,
    pub engine_count: f64,
}

/// 一帧变量快照 (VarId 下标平坦 Vec; 求值器零查表)
pub struct VarSnapshot {
    pub values: Vec<f64>,
}

impl VarSnapshot {
    pub fn get(&self, id: u16) -> Option<Value> {
        self.values.get(id as usize).map(|&v| Value::Num(v))
    }

    /// 全 NaN 快照 (无数据帧; 公式自然降级 "-")
    pub fn empty(len: usize) -> Self {
        VarSnapshot { values: vec![f64::NAN; len] }
    }
}

/// 注册表 (名字+别名 → VarId)
pub struct Registry {
    pub vars: Vec<VarMeta>,
    pub index: HashMap<&'static str, u16>,
}

impl Registry {
    pub fn lookup(&self, name: &str) -> Option<u16> {
        self.index.get(name).copied()
    }

    pub fn len(&self) -> usize {
        self.vars.len()
    }

    /// 编辑器目录 DTO 用 (名字/单位/描述/类别/来源)
    pub fn catalog(&self) -> Vec<(&'static str, &'static str, &'static str, VarCategory, VarOrigin)> {
        self.vars
            .iter()
            .map(|v| (v.name, v.unit, v.desc, v.category, v.origin))
            .collect()
    }
}

impl VarLookup for Registry {
    fn lookup(&self, name: &str) -> Option<u16> {
        Registry::lookup(self, name)
    }
    fn version(&self) -> u32 {
        // 注册表内容变化 = 代码变化, 版本号静态即可
        1
    }
}

/// 全局注册表 (首访问构建一次)
pub fn registry() -> &'static Registry {
    static REG: OnceLock<Registry> = OnceLock::new();
    REG.get_or_init(build_registry)
}

/// 组装一帧快照 (Service 线程调用; fm=None 时 fm.* 全 NaN)
pub fn assemble_snapshot(
    tel: &dyn TelemetrySource,
    fm: Option<&dyn FMDataSource>,
    meta: &MetaInputs,
) -> VarSnapshot {
    let reg = registry();
    let mut values = Vec::with_capacity(reg.vars.len());
    for v in &reg.vars {
        let x = match &v.src {
            VarSrc::Tel(f) => f(tel),
            VarSrc::TelBool(f) => f(tel) as u8 as f64,
            VarSrc::Fm(f) => match fm {
                Some(src) => f(src),
                // FM 未加载: 隔离为 NaN (设计 §3.6), 不用哨兵污染
                None => f64::NAN,
            },
            VarSrc::Const(c) => *c,
            VarSrc::Meta(m) => match m {
                MetaVar::IntervalMs => meta.interval_ms,
                MetaVar::Freq => meta.freq,
                MetaVar::ElapsedMs => meta.elapsed_ms,
                MetaVar::SessionMs => meta.session_ms,
                MetaVar::FmLoaded => meta.fm_loaded as u8 as f64,
                MetaVar::EngineCount => meta.engine_count,
            },
        };
        values.push(x);
    }
    VarSnapshot { values }
}

fn build_registry() -> Registry {
    use VarCategory as C;
    use VarSrc::{Const as K, Fm as F, Meta as M, Tel as T, TelBool as TB};

    let vars: Vec<VarMeta> = vec![
        // ===== 飞行数据 (TelemetrySource) =====
        VarMeta { name: "ias", getter: "getIAS", unit: "km/h", desc: "指示空速", category: C::Flight, origin: VarOrigin::State, src: T(|s| s.get_ias()) },
        VarMeta { name: "tas", getter: "getTAS", unit: "km/h", desc: "真空速", category: C::Flight, origin: VarOrigin::State, src: T(|s| s.get_tas()) },
        VarMeta { name: "mach", getter: "getMach", unit: "Ma", desc: "马赫数", category: C::Flight, origin: VarOrigin::Derived, src: T(|s| s.get_mach()) },
        VarMeta { name: "aoa", getter: "getAoA", unit: "°", desc: "迎角", category: C::Flight, origin: VarOrigin::State, src: T(|s| s.get_aoa()) },
        VarMeta { name: "aos", getter: "getAoS", unit: "°", desc: "侧滑角", category: C::Flight, origin: VarOrigin::State, src: T(|s| s.get_aos()) },
        VarMeta { name: "ny", getter: "getNy", unit: "G", desc: "过载", category: C::Flight, origin: VarOrigin::Derived, src: T(|s| s.get_ny()) },
        VarMeta { name: "vario", getter: "getVario", unit: "m/s", desc: "升降速度", category: C::Flight, origin: VarOrigin::Indicators, src: T(|s| s.get_vario()) },
        VarMeta { name: "altitude", getter: "getAltitude", unit: "m", desc: "气压高度", category: C::Flight, origin: VarOrigin::State, src: T(|s| s.get_altitude()) }, // state.heightm 直通 (FlightValues 回写, 无滤波)
        VarMeta { name: "radio_altitude", getter: "getRadioAltitude", unit: "m", desc: "雷达高度", category: C::Flight, origin: VarOrigin::Indicators, src: T(|s| s.get_radio_altitude()) },
        VarMeta { name: "compass", getter: "getCompass", unit: "°", desc: "航向", category: C::Flight, origin: VarOrigin::Derived, src: T(|s| s.get_compass()) },
        VarMeta { name: "sep", getter: "getSep", unit: "m", desc: "单位能量高度", category: C::Flight, origin: VarOrigin::Derived, src: T(|s| s.get_sep()) },
        VarMeta { name: "acceleration", getter: "getAcceleration", unit: "m/s²", desc: "加速度", category: C::Flight, origin: VarOrigin::Derived, src: T(|s| s.get_acceleration()) },
        VarMeta { name: "turn_rate", getter: "getTurnRate", unit: "°/s", desc: "盘旋率", category: C::Flight, origin: VarOrigin::Derived, src: T(|s| s.get_turn_rate()) },
        VarMeta { name: "turn_radius", getter: "getTurnRadius", unit: "m", desc: "盘旋半径", category: C::Flight, origin: VarOrigin::Derived, src: T(|s| s.get_turn_radius()) },
        VarMeta { name: "roll_rate", getter: "getRollRate", unit: "°/s", desc: "滚转率", category: C::Flight, origin: VarOrigin::State, src: T(|s| s.get_roll_rate()) },
        VarMeta { name: "energy_jkg", getter: "getEnergyJkg", unit: "J/kg", desc: "单位动能(比能量)", category: C::Flight, origin: VarOrigin::Derived, src: T(|s| s.get_energy_jkg()) },
        VarMeta { name: "aviahorizon_pitch", getter: "getAviahorizonPitch", unit: "°", desc: "地平仪俯仰", category: C::Flight, origin: VarOrigin::Indicators, src: T(|s| s.get_aviahorizon_pitch()) },
        VarMeta { name: "aviahorizon_roll", getter: "getAviahorizonRoll", unit: "°", desc: "地平仪滚转", category: C::Flight, origin: VarOrigin::Indicators, src: T(|s| s.get_aviahorizon_roll()) },
        // ===== 引擎 (TelemetrySource) =====
        VarMeta { name: "mass_fuel", getter: "getMassFuel", unit: "kg", desc: "当前油量", category: C::Engine, origin: VarOrigin::Derived, src: T(|s| s.get_mass_fuel()) },
        VarMeta { name: "total_weight", getter: "getTotalWeight", unit: "kg", desc: "全机重量", category: C::Engine, origin: VarOrigin::Derived, src: T(|s| s.get_total_weight()) },
        VarMeta { name: "fuel_time_mili", getter: "getFuelTimeMili", unit: "ms", desc: "剩余油量时间", category: C::Engine, origin: VarOrigin::Derived, src: T(|s| s.get_fuel_time_mili() as f64) },
        VarMeta { name: "throttle", getter: "getThrottle", unit: "%", desc: "油门", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_throttle()) },
        VarMeta { name: "rpm", getter: "getRPM", unit: "rpm", desc: "转速", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_rpm()) },
        VarMeta { name: "manifold_pressure", getter: "getManifoldPressure", unit: "ata", desc: "进气压", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_manifold_pressure()) },
        VarMeta { name: "water_temp", getter: "getWaterTemp", unit: "°C", desc: "水温", category: C::Engine, origin: VarOrigin::Indicators, src: T(|s| s.get_water_temp()) }, // indic.waterTemp 优先, state 兜底 (updateTemp)
        VarMeta { name: "oil_temp", getter: "getOilTemp", unit: "°C", desc: "油温", category: C::Engine, origin: VarOrigin::Indicators, src: T(|s| s.get_oil_temp()) }, // indic.oilTemp 优先, state 兜底 (updateTemp)
        VarMeta { name: "prop_pitch", getter: "getPitch", unit: "", desc: "桨距", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_pitch()) },
        VarMeta { name: "eff_hp", getter: "getEffHp", unit: "hp", desc: "有效功率", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_eff_hp()) }, // state.thrust[] 聚合 (speedv 校正因子来自 Deriver)
        VarMeta { name: "thrust", getter: "getThrust", unit: "kgf", desc: "推力", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_thrust()) },
        VarMeta { name: "horse_power", getter: "getHorsePower", unit: "hp", desc: "功率", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_horse_power()) },
        VarMeta { name: "engine_response", getter: "getEngineResponse", unit: "", desc: "引擎响应速率", category: C::Engine, origin: VarOrigin::Derived, src: T(|s| s.get_engine_response()) },
        VarMeta { name: "prop_efficiency", getter: "getPropEfficiency", unit: "%", desc: "螺旋桨效率", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_prop_efficiency()) },
        VarMeta { name: "wep_kg", getter: "getWepKg", unit: "kg", desc: "WEP 剩余工质", category: C::Engine, origin: VarOrigin::Derived, src: T(|s| s.get_wep_kg()) }, // fm.nitro 减 wep_time 状态机消耗 (updateWepTime)
        VarMeta { name: "wep_time", getter: "getWepTime", unit: "s", desc: "WEP 剩余时间", category: C::Engine, origin: VarOrigin::Derived, src: T(|s| s.get_wep_time()) }, // fm.nitro/nitro_decr 与 wep_time 状态机 (formatStrings)
        VarMeta { name: "heat_tolerance", getter: "getHeatTolerance", unit: "", desc: "耐热阈值", category: C::Engine, origin: VarOrigin::Derived, src: T(|s| s.get_heat_tolerance()) },
        VarMeta { name: "power_percent", getter: "getPowerPercent", unit: "%", desc: "推力/功率百分比", category: C::Engine, origin: VarOrigin::Derived, src: T(|s| s.get_power_percent()) },
        VarMeta { name: "manifold_pressure_pounds", getter: "getManifoldPressurePounds", unit: "psi", desc: "进气压(英制)", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_manifold_pressure_pounds()) },
        VarMeta { name: "manifold_pressure_inch_hg", getter: "getManifoldPressureInchHg", unit: "inHg", desc: "进气压(英寸汞柱)", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_manifold_pressure_inch_hg()) },
        VarMeta { name: "manifold_pressure_display", getter: "getManifoldPressureDisplay", unit: "", desc: "进气压显示值(公/英制)", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_manifold_pressure_display()) }, // 两分支均为 state.manifoldpressure 换算
        VarMeta { name: "manifold_pressure_display_precision", getter: "getManifoldPressureDisplayPrecision", unit: "", desc: "进气压显示精度", category: C::Engine, origin: VarOrigin::Derived, src: T(|s| s.get_manifold_pressure_display_precision() as f64) }, // 纯 is_imperial (英制检测状态机) 的函数
        VarMeta { name: "mixture_state", getter: "getUnknownMixture", unit: "", desc: "混合比状态", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_unknown_mixture()) },
        VarMeta { name: "radiator", getter: "getRadiator", unit: "", desc: "散热器", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_radiator()) },
        VarMeta { name: "compressor_stage", getter: "getCompressorStage", unit: "", desc: "增压器档位", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_compressor_stage()) },
        VarMeta { name: "fuel_percent", getter: "getFuelPercent", unit: "%", desc: "油量百分比", category: C::Engine, origin: VarOrigin::Derived, src: T(|s| s.get_fuel_percent()) }, // total_fuel (indic.fuel/state.mfuel 混合) / state.mfuel0
        VarMeta { name: "rpm_throttle", getter: "getRPMThrottle", unit: "", desc: "转速油门", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_rpm_throttle()) },
        VarMeta { name: "booster_fuel_kg", getter: "getBoosterFuelKg", unit: "kg", desc: "助推器剩余燃料", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_booster_fuel_kg()) },
        VarMeta { name: "booster_fuel_percent", getter: "getBoosterFuelPercent", unit: "%", desc: "助推器燃料百分比", category: C::Engine, origin: VarOrigin::State, src: T(|s| s.get_booster_fuel_percent()) },
        // ===== 操纵面与状态 (TelemetrySource) =====
        VarMeta { name: "gear", getter: "getGear", unit: "", desc: "起落架", category: C::State, origin: VarOrigin::State, src: T(|s| s.get_gear()) },
        VarMeta { name: "flaps", getter: "getFlaps", unit: "", desc: "襟翼", category: C::State, origin: VarOrigin::State, src: T(|s| s.get_flaps()) },
        VarMeta { name: "airbrake", getter: "getAirbrake", unit: "", desc: "空气刹车", category: C::State, origin: VarOrigin::State, src: T(|s| s.get_airbrake()) },
        VarMeta { name: "aileron", getter: "getAileron", unit: "", desc: "副翼", category: C::State, origin: VarOrigin::State, src: T(|s| s.get_aileron()) },
        VarMeta { name: "elevator", getter: "getElevator", unit: "", desc: "升降舵", category: C::State, origin: VarOrigin::State, src: T(|s| s.get_elevator()) },
        VarMeta { name: "rudder", getter: "getRudder", unit: "", desc: "方向舵", category: C::State, origin: VarOrigin::State, src: T(|s| s.get_rudder()) },
        VarMeta { name: "wing_sweep", getter: "getWingSweep", unit: "", desc: "变后掠翼", category: C::State, origin: VarOrigin::Indicators, src: T(|s| s.get_wing_sweep()) },
        // ===== 限制 (TelemetrySource) =====
        VarMeta { name: "speed_limit_ratio", getter: "getSpeedLimitRatio", unit: "", desc: "速度限制比值", category: C::Limit, origin: VarOrigin::Derived, src: T(|s| s.get_speed_limit_ratio()) },
        VarMeta { name: "aileron_lock_ratio", getter: "getAileronLockRatio", unit: "", desc: "副翼锁止比值", category: C::Limit, origin: VarOrigin::Derived, src: T(|s| s.get_aileron_lock_ratio()) },
        VarMeta { name: "rudder_lock_ratio", getter: "getRudderLockRatio", unit: "", desc: "方向舵锁止比值", category: C::Limit, origin: VarOrigin::Derived, src: T(|s| s.get_rudder_lock_ratio()) },
        VarMeta { name: "unit_mach_limit_ratio", getter: "getUnitMachLimitRatio", unit: "", desc: "马赫限制比值", category: C::Limit, origin: VarOrigin::Derived, src: T(|s| s.get_unit_mach_limit_ratio()) },
        VarMeta { name: "stall_speed", getter: "getStallSpeed", unit: "km/h", desc: "失速速度", category: C::Limit, origin: VarOrigin::Derived, src: T(|s| s.get_stall_speed()) },
        // ===== 布尔标志 (TelemetrySource, 0/1) =====
        VarMeta { name: "radio_altitude_valid", getter: "isRadioAltitudeValid", unit: "", desc: "雷达高度有效", category: C::Flight, origin: VarOrigin::Indicators, src: TB(|s| s.is_radio_altitude_valid()) },
        VarMeta { name: "turn_radius_valid", getter: "isTurnRadiusValid", unit: "", desc: "盘旋半径有效", category: C::Flight, origin: VarOrigin::Derived, src: TB(|s| s.is_turn_radius_valid()) },
        VarMeta { name: "wing_sweep_valid", getter: "isWingSweepValid", unit: "", desc: "变后掠翼有效", category: C::State, origin: VarOrigin::Indicators, src: TB(|s| s.is_wing_sweep_valid()) },
        VarMeta { name: "is_imperial", getter: "isImperial", unit: "", desc: "英制单位", category: C::Meta, origin: VarOrigin::Derived, src: TB(|s| s.is_imperial()) },
        VarMeta { name: "is_jet_engine", getter: "isJetEngine", unit: "", desc: "喷气引擎", category: C::Engine, origin: VarOrigin::Derived, src: TB(|s| s.is_jet_engine()) },
        VarMeta { name: "is_prop_engine", getter: "isPropEngine", unit: "", desc: "螺旋桨引擎", category: C::Engine, origin: VarOrigin::Derived, src: TB(|s| s.is_prop_engine()) },
        VarMeta { name: "is_piston_engine", getter: "isPistonEngine", unit: "", desc: "活塞引擎", category: C::Engine, origin: VarOrigin::Derived, src: TB(|s| s.is_piston_engine()) },
        VarMeta { name: "is_turboprop_engine", getter: "isTurbopropEngine", unit: "", desc: "涡桨引擎", category: C::Engine, origin: VarOrigin::Derived, src: TB(|s| s.is_turboprop_engine()) },
        VarMeta { name: "is_engine_check_done", getter: "isEngineCheckDone", unit: "", desc: "引擎检测完成", category: C::Engine, origin: VarOrigin::Derived, src: TB(|s| s.is_engine_check_done()) },
        VarMeta { name: "has_wep", getter: "hasWep", unit: "", desc: "有加力系统", category: C::Engine, origin: VarOrigin::Fm, src: TB(|s| s.has_wep()) }, // Tel 源但读 fm.blkx.nitro, 按数据本质标 Fm
        VarMeta { name: "has_booster", getter: "hasBooster", unit: "", desc: "有火箭助推器", category: C::Engine, origin: VarOrigin::State, src: TB(|s| s.has_booster()) },
        // ===== FM 字段 (fm.* 前缀, 换机重载; 无 FM = NaN) =====
        VarMeta { name: "fm.empty_weight", getter: "", unit: "kg", desc: "空重", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_empty_weight()) },
        VarMeta { name: "fm.max_fuel_weight", getter: "", unit: "kg", desc: "最大油量", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_max_fuel_weight()) },
        VarMeta { name: "fm.critical_speed", getter: "", unit: "km/h", desc: "临界速度", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_critical_speed()) },
        VarMeta { name: "fm.vne", getter: "", unit: "km/h", desc: "最大速度(VNE)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_vne()) },
        VarMeta { name: "fm.vne_mach", getter: "", unit: "Ma", desc: "最大马赫数", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_vne_mach()) },
        VarMeta { name: "fm.full_fuel_pos_g", getter: "", unit: "G", desc: "满油正过载限制", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_full_fuel_pos_g()) },
        VarMeta { name: "fm.full_fuel_neg_g", getter: "", unit: "G", desc: "满油负过载限制", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_full_fuel_neg_g()) },
        VarMeta { name: "fm.half_fuel_pos_g", getter: "", unit: "G", desc: "半油正过载限制", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_half_fuel_pos_g()) },
        VarMeta { name: "fm.half_fuel_neg_g", getter: "", unit: "G", desc: "半油负过载限制", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_half_fuel_neg_g()) },
        VarMeta { name: "fm.elevator_eff_speed", getter: "", unit: "km/h", desc: "升降舵生效速度", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_elevator_eff_speed()) },
        VarMeta { name: "fm.aileron_eff_speed", getter: "", unit: "km/h", desc: "副翼生效速度", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_aileron_eff_speed()) },
        VarMeta { name: "fm.rudder_eff_speed", getter: "", unit: "km/h", desc: "方向舵生效速度", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_rudder_eff_speed()) },
        VarMeta { name: "fm.elevator_power_loss", getter: "", unit: "km/h", desc: "升降舵锁速", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_elevator_power_loss()) },
        VarMeta { name: "fm.aileron_power_loss", getter: "", unit: "km/h", desc: "副翼锁速", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_aileron_power_loss()) },
        VarMeta { name: "fm.rudder_power_loss", getter: "", unit: "km/h", desc: "方向舵锁速", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_rudder_power_loss()) },
        VarMeta { name: "fm.nitro_amount", getter: "", unit: "kg", desc: "氧化亚氮量", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_nitro_amount()) },
        VarMeta { name: "fm.nitro_time", getter: "", unit: "s", desc: "氧化亚氮时间", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_nitro_time()) },
        VarMeta { name: "fm.avg_eng_recovery_rate", getter: "", unit: "", desc: "平均引擎恢复率", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_avg_eng_recovery_rate()) },
        VarMeta { name: "fm.no_flap_wing_load", getter: "", unit: "kg/m²", desc: "翼载(无襟翼)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_no_flap_wing_load()) },
        VarMeta { name: "fm.full_flap_wing_load", getter: "", unit: "kg/m²", desc: "翼载(满襟翼)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_full_flap_wing_load()) },
        VarMeta { name: "fm.moi_pitch", getter: "", unit: "kg·m²", desc: "俯仰转动惯量", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_moi_pitch()) },
        VarMeta { name: "fm.moi_roll", getter: "", unit: "kg·m²", desc: "滚转转动惯量", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_moi_roll()) },
        VarMeta { name: "fm.moi_yaw", getter: "", unit: "kg·m²", desc: "偏航转动惯量", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_moi_yaw()) },
        VarMeta { name: "fm.wing_area", getter: "", unit: "m²", desc: "机翼面积", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_wing_area()) },
        VarMeta { name: "fm.fuselage_area", getter: "", unit: "m²", desc: "机身面积", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_fuselage_area()) },
        VarMeta { name: "fm.oswalds_efficiency", getter: "", unit: "", desc: "奥斯瓦尔德效率", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_oswalds_efficiency()) },
        VarMeta { name: "fm.aspect_ratio", getter: "", unit: "", desc: "展弦比", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_aspect_ratio()) },
        VarMeta { name: "fm.swept_wing_angle", getter: "", unit: "°", desc: "后掠角", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_swept_wing_angle()) },
        VarMeta { name: "fm.cd_s", getter: "", unit: "", desc: "寄生阻力系数", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_cd_s()) },
        VarMeta { name: "fm.ind_cd_f", getter: "", unit: "", desc: "诱导阻力系数", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_ind_cd_f()) },
        VarMeta { name: "fm.radiator_cd", getter: "", unit: "", desc: "散热器阻力系数", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_radiator_cd()) },
        VarMeta { name: "fm.oil_radiator_cd", getter: "", unit: "", desc: "滑油散热器阻力系数", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_oil_radiator_cd()) },
        VarMeta { name: "fm.no_flaps_wing_cd_min", getter: "", unit: "", desc: "最小阻力系数(无襟翼)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_no_flaps_wing_cd_min()) },
        VarMeta { name: "fm.no_flaps_wing_cl0", getter: "", unit: "", desc: "零升力系数(无襟翼)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_no_flaps_wing_cl0()) },
        VarMeta { name: "fm.no_flaps_wing_aoa_crit_high", getter: "", unit: "°", desc: "临界迎角上限(无襟翼)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_no_flaps_wing_aoa_crit_high()) },
        VarMeta { name: "fm.no_flaps_wing_aoa_crit_low", getter: "", unit: "°", desc: "临界迎角下限(无襟翼)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_no_flaps_wing_aoa_crit_low()) },
        VarMeta { name: "fm.no_flaps_wing_cl_crit_high", getter: "", unit: "", desc: "临界升力系数上限(无襟翼)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_no_flaps_wing_cl_crit_high()) },
        VarMeta { name: "fm.no_flaps_wing_cl_crit_low", getter: "", unit: "", desc: "临界升力系数下限(无襟翼)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_no_flaps_wing_cl_crit_low()) },
        VarMeta { name: "fm.full_flaps_wing_cd_min", getter: "", unit: "", desc: "最小阻力系数(满襟翼)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_full_flaps_wing_cd_min()) },
        VarMeta { name: "fm.full_flaps_wing_cl0", getter: "", unit: "", desc: "零升力系数(满襟翼)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_full_flaps_wing_cl0()) },
        VarMeta { name: "fm.full_flaps_wing_aoa_crit_high", getter: "", unit: "°", desc: "临界迎角上限(满襟翼)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_full_flaps_wing_aoa_crit_high()) },
        VarMeta { name: "fm.full_flaps_wing_aoa_crit_low", getter: "", unit: "°", desc: "临界迎角下限(满襟翼)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_full_flaps_wing_aoa_crit_low()) },
        VarMeta { name: "fm.fuselage_cd_min", getter: "", unit: "", desc: "机身最小阻力系数", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_fuselage_cd_min()) },
        VarMeta { name: "fm.fin_cd_min", getter: "", unit: "", desc: "垂尾最小阻力系数", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_fin_cd_min()) },
        VarMeta { name: "fm.stab_cd_min", getter: "", unit: "", desc: "平尾最小阻力系数", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_stab_cd_min()) },
        VarMeta { name: "fm.flap0_speed", getter: "", unit: "km/h", desc: "襟翼档位0速度", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_flap0_speed()) },
        VarMeta { name: "fm.flap1_speed", getter: "", unit: "km/h", desc: "襟翼档位1速度", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_flap1_speed()) },
        VarMeta { name: "fm.flap2_speed", getter: "", unit: "km/h", desc: "襟翼档位2速度", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_flap2_speed()) },
        VarMeta { name: "fm.flap3_speed", getter: "", unit: "km/h", desc: "襟翼档位3速度", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_flap3_speed()) },
        VarMeta { name: "fm.gear_destruction_speed", getter: "", unit: "km/h", desc: "起落架损毁速度", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_gear_destruction_speed()) },
        VarMeta { name: "fm.engine_num", getter: "", unit: "", desc: "引擎数(FM)", category: C::Fm, origin: VarOrigin::Fm, src: F(|f| f.get_engine_num() as f64) },
        // ===== 元变量 =====
        VarMeta { name: "interval_ms", getter: "", unit: "ms", desc: "本帧轮询间隔", category: C::Meta, origin: VarOrigin::Meta, src: M(MetaVar::IntervalMs) },
        VarMeta { name: "freq", getter: "", unit: "Hz", desc: "轮询频率", category: C::Meta, origin: VarOrigin::Meta, src: M(MetaVar::Freq) },
        VarMeta { name: "elapsed_ms", getter: "", unit: "ms", desc: "会话经过时间", category: C::Meta, origin: VarOrigin::Meta, src: M(MetaVar::ElapsedMs) },
        VarMeta { name: "session_ms", getter: "", unit: "ms", desc: "会话开始至今", category: C::Meta, origin: VarOrigin::Meta, src: M(MetaVar::SessionMs) },
        VarMeta { name: "fm_loaded", getter: "", unit: "", desc: "FM 已加载", category: C::Meta, origin: VarOrigin::Meta, src: M(MetaVar::FmLoaded) },
        VarMeta { name: "engine_count", getter: "", unit: "", desc: "引擎数(遥测)", category: C::Meta, origin: VarOrigin::Meta, src: M(MetaVar::EngineCount) },
        // ===== 物理常量 (physics_constants 单一来源) =====
        VarMeta { name: "g", getter: "", unit: "m/s²", desc: "重力加速度", category: C::Const, origin: VarOrigin::Const, src: K(crate::physics_constants::g) },
        VarMeta { name: "rho0", getter: "", unit: "kg/m³", desc: "海平面空气密度", category: C::Const, origin: VarOrigin::Const, src: K(crate::physics_constants::SEA_LEVEL_DENSITY) },
        VarMeta { name: "P0", getter: "", unit: "Pa", desc: "海平面气压", category: C::Const, origin: VarOrigin::Const, src: K(crate::physics_constants::SEA_LEVEL_PRESSURE) },
    ];

    let mut index: HashMap<&'static str, u16> = HashMap::with_capacity(vars.len() * 2);
    for (i, v) in vars.iter().enumerate() {
        index.insert(v.name, i as u16);
        if !v.getter.is_empty() {
            index.insert(v.getter, i as u16);
        }
    }
    Registry { vars, index }
}
