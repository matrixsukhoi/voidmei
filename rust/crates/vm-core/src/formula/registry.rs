//! L0 原子变量注册表: 名字 → VarId → 取值 fn 指针 (编译期显式代码, 非反射, D4/D7)。
//! 覆盖: state/indicators 直通 + C 级会话量 + FM 字段 (fm.* 前缀, 无 FM 时 NaN)
//! + 元变量 + 物理常量。单名制 (W10): 变量唯一短名, Java getter 名只在
//! 对拍文件边界存在 (vm-overlay fields.rs getter())。
//! 设计: doc/formula_system_design.md §5

use super::definition::VarLookup;
use super::functions::Value;
use crate::blkx::Blkx;
use crate::parser::{Indicators, State};
use crate::string_helper::F_INVALID;
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

/// 取值源 (fn 指针, 非捕获闭包强转) — W6 直通化:
/// 变量直接绑定 parser 原始字段 / blkx 字段, TelemetrySource/FMDataSource
/// 中间 getter 层消亡 (设计 §15 数据直通重构)。哨兵守卫按原 getter 逐一对齐。
pub enum VarSrc {
    /// /state 原始字段直通
    State(fn(&State) -> f64),
    /// /indicators 原始字段直通
    Indic(fn(&Indicators) -> f64),
    /// FM (blkx) 字段直通
    Blk(fn(&Blkx) -> f64),
    /// C 级会话量 (聚合/状态机产物, W8 消解)
    Session(fn(&SessionInputs) -> f64),
    Const(f64),
    Meta(MetaVar),
}

/// 一帧原始输入 (快照组装的三元组; None → 该源变量 NaN 隔离)
#[derive(Default)]
pub struct RawInputs<'a> {
    pub state: Option<&'a State>,
    pub indic: Option<&'a Indicators>,
    pub blkx: Option<&'a Blkx>,
}

/// C 级会话量暂存通道 (W6): 聚合/状态机产物经此供值, W8 公式化后逐项消亡。
/// 字段与 ServiceData 的 C 级字段一一对应 (计算留在原处, 本通道只搬运)。
#[derive(Debug, Clone, Copy, Default)]
pub struct SessionInputs {
    pub total_fuel: f64,
    pub fuel_time_mili: f64,
    pub total_hp: f64,
    pub total_hp_eff: f64,
    pub total_thrust: f64,
    pub n_water_temp: f64,
    pub n_oil_temp: f64,
    pub energy_j_kg: f64,
    pub radio_alt: f64,
    pub compass_delta: f64,
    pub nitro_kg: f64,
    pub wep_time: f64,
    pub heat_tolerance: f64,
    pub thurst_percent: f64,
    pub t_eng_response: f64,
    pub avgeff: f64,
    pub manifold_display: f64,
    pub fuel_percent: f64,
    pub is_imperial: bool,
    pub is_jet: bool,
    pub is_prop: bool,
    pub is_piston: bool,
    pub is_turboprop: bool,
    pub engine_check_done: bool,
}

/// 统一取值视图 (W6): resolve_target 求值面 — 实现方 (ServiceData) 持
/// 快照+公式槽, 按名字取变量或公式值
pub trait FormulaView {
    fn var_value(&self, name: &str) -> Option<f64>;
}

/// 变量元数据
pub struct VarMeta {
    /// 短名 (公式引用主名, snake_case)
    pub name: &'static str,
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

/// 注册表 (名字 → VarId, 单名制)
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

/// 组装一帧快照 (Service 线程调用; 源缺失 → 该源变量 NaN 隔离)
pub fn assemble_snapshot(raw: &RawInputs, session: &SessionInputs, meta: &MetaInputs) -> VarSnapshot {
    let reg = registry();
    let mut values = Vec::with_capacity(reg.vars.len());
    for v in &reg.vars {
        let x = match &v.src {
            VarSrc::State(f) => raw.state.map_or(f64::NAN, |s| f(s)),
            VarSrc::Indic(f) => raw.indic.map_or(f64::NAN, |i| f(i)),
            VarSrc::Blk(f) => raw.blkx.map_or(f64::NAN, |b| f(b)),
            VarSrc::Session(f) => f(session),
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
    use VarOrigin as O;
    use VarSrc::{Blk as B, Const as K, Indic as I, Meta as M, Session as SE, State as T};

    // 哨兵守卫小件 (与原 getter 实现逐一对齐)
    const FI: f64 = F_INVALID; // -65535 (float 域哨兵)
    let vars: Vec<VarMeta> = vec![
        // ===== /state 原始直通 (哨兵原样穿透, 守卫在消费侧/公式内联) =====
        VarMeta { name: "ias", unit: "km/h", desc: "指示空速", category: C::Flight, origin: O::State, src: T(|s| s.ias as f64) },
        VarMeta { name: "tas", unit: "km/h", desc: "真空速", category: C::Flight, origin: O::State, src: T(|s| s.tas as f64) },
        VarMeta { name: "aoa", unit: "°", desc: "迎角", category: C::Flight, origin: O::State, src: T(|s| s.aoa) },
        VarMeta { name: "aos", unit: "°", desc: "侧滑角", category: C::Flight, origin: O::State, src: T(|s| s.aos) },
        VarMeta { name: "altitude", unit: "m", desc: "气压高度", category: C::Flight, origin: O::State, src: T(|s| s.heightm) },
        VarMeta { name: "ny_raw", unit: "G", desc: "原始过载(state 直通)", category: C::Flight, origin: O::State, src: T(|s| s.ny) },
        VarMeta { name: "vy", unit: "m/s", desc: "垂直速度(state 直通)", category: C::Flight, origin: O::State, src: T(|s| s.vy) },
        VarMeta { name: "roll_rate", unit: "°/s", desc: "滚转率(Wx)", category: C::Flight, origin: O::State, src: T(|s| s.wx) },
        VarMeta { name: "mfuel", unit: "kg", desc: "主油量(state 直通)", category: C::Engine, origin: O::State, src: T(|s| s.mfuel) },
        VarMeta { name: "mfuel0", unit: "kg", desc: "初始主油量(state 直通)", category: C::Engine, origin: O::State, src: T(|s| s.mfuel0) },
        VarMeta { name: "mfuel_1", unit: "kg", desc: "助推器燃料(state 直通)", category: C::Engine, origin: O::State, src: T(|s| s.mfuel_1) },
        VarMeta { name: "mfuel0_1", unit: "kg", desc: "助推器初始燃料(state 直通)", category: C::Engine, origin: O::State, src: T(|s| s.mfuel0_1) },
        // 助推器两量 = 原 getBoosterFuelKg/Percent getter 直绑复刻 (守卫 NaN
        // 穿透原样 §2.12: `!(x <= 0.0)` 而非 `x > 0.0`, min 手写 NaN 传播)
        VarMeta { name: "booster_fuel_kg", unit: "kg", desc: "助推器燃料(守卫归零)", category: C::Engine, origin: O::State, src: T(|s| if !(s.mfuel_1 <= 0.0) { s.mfuel_1 } else { 0.0 }) },
        VarMeta { name: "booster_fuel_percent", unit: "%", desc: "助推器剩余百分比", category: C::Engine, origin: O::State, src: T(|s| if !(s.mfuel0_1 <= 0.0) {
            let v = 100.0 * s.mfuel_1 / s.mfuel0_1;
            if v.is_nan() { v } else { v.min(100.0) }
        } else { 0.0 }) },
        VarMeta { name: "has_booster", unit: "", desc: "有助推器(mfuel_1>0 且初始>0)", category: C::Engine, origin: O::State, src: T(|s| (s.mfuel_1 > 0.0 && s.mfuel0_1 > 0.0) as u8 as f64) },
        VarMeta { name: "engine_count", unit: "", desc: "引擎数(遥测)", category: C::Meta, origin: O::State, src: T(|s| s.engine_num as f64) },
        // ===== /indicators 原始直通 =====
        VarMeta { name: "indic_speed", unit: "m/s", desc: "校正速度(indicators.speed, m/s)", category: C::Flight, origin: O::Indicators, src: I(|i| i.speed) },
        VarMeta { name: "indic_vario", unit: "m/s", desc: "仪表升降速度(含哨兵)", category: C::Flight, origin: O::Indicators, src: I(|i| i.vario) },
        VarMeta { name: "radio_alt_raw", unit: "m", desc: "雷达高度原始值(含哨兵)", category: C::Flight, origin: O::Indicators, src: I(|i| i.radio_altitude) },
        VarMeta { name: "radio_altitude_valid", unit: "", desc: "雷达高度有效", category: C::Flight, origin: O::Indicators, src: I(|i| (i.radio_altitude != FI) as u8 as f64) },
        VarMeta { name: "wing_sweep", unit: "", desc: "变后掠翼(哨兵归零)", category: C::State, origin: O::Indicators, src: I(|i| if i.wsweep_indicator != FI { i.wsweep_indicator } else { 0.0 }) },
        VarMeta { name: "wing_sweep_valid", unit: "", desc: "变后掠翼有效", category: C::State, origin: O::Indicators, src: I(|i| (i.wsweep_indicator != FI) as u8 as f64) },
        VarMeta { name: "aviahorizon_pitch", unit: "°", desc: "地平仪俯仰", category: C::Flight, origin: O::Indicators, src: I(|i| i.aviahorizon_pitch) },
        VarMeta { name: "aviahorizon_roll", unit: "°", desc: "地平仪滚转", category: C::Flight, origin: O::Indicators, src: I(|i| i.aviahorizon_roll) },
        // ===== /state 引擎/操纵面直通 (int 拓宽) =====
        VarMeta { name: "throttle", unit: "%", desc: "油门", category: C::Engine, origin: O::State, src: T(|s| s.throttle as f64) },
        VarMeta { name: "rpm", unit: "rpm", desc: "转速", category: C::Engine, origin: O::State, src: T(|s| s.rpm as f64) },
        VarMeta { name: "manifold_pressure", unit: "ata", desc: "进气压", category: C::Engine, origin: O::State, src: T(|s| s.manifoldpressure) },
        VarMeta { name: "prop_pitch", unit: "", desc: "桨距(pitch[0])", category: C::Engine, origin: O::State, src: T(|s| s.pitch.first().copied().unwrap_or(0.0)) },
        VarMeta { name: "mixture_state", unit: "", desc: "混合比状态", category: C::Engine, origin: O::State, src: T(|s| s.mixture as f64) },
        VarMeta { name: "radiator", unit: "", desc: "散热器", category: C::Engine, origin: O::State, src: T(|s| s.radiator as f64) },
        VarMeta { name: "compressor_stage", unit: "", desc: "增压器档位(遥测)", category: C::Engine, origin: O::State, src: T(|s| s.compressorstage as f64) },
        VarMeta { name: "rpm_throttle", unit: "", desc: "转速油门", category: C::Engine, origin: O::State, src: T(|s| s.rpm_throttle as f64) },
        VarMeta { name: "gear", unit: "", desc: "起落架", category: C::State, origin: O::State, src: T(|s| s.gear as f64) },
        VarMeta { name: "flaps", unit: "", desc: "襟翼", category: C::State, origin: O::State, src: T(|s| s.flaps as f64) },
        VarMeta { name: "airbrake", unit: "", desc: "空气刹车", category: C::State, origin: O::State, src: T(|s| s.airbrake as f64) },
        VarMeta { name: "aileron", unit: "", desc: "副翼", category: C::State, origin: O::State, src: T(|s| s.aileron as f64) },
        VarMeta { name: "elevator", unit: "", desc: "升降舵", category: C::State, origin: O::State, src: T(|s| s.elevator as f64) },
        VarMeta { name: "rudder", unit: "", desc: "方向舵", category: C::State, origin: O::State, src: T(|s| s.rudder as f64) },
        // ===== C 级会话量 (聚合/状态机产物, W8 公式化后逐项消亡) =====
        VarMeta { name: "mass_fuel", unit: "kg", desc: "当前总油量(聚合)", category: C::Engine, origin: O::Derived, src: SE(|x| x.total_fuel) },
        VarMeta { name: "fuel_time_mili", unit: "ms", desc: "剩余油量时间(SMA 慢算)", category: C::Engine, origin: O::Derived, src: SE(|x| x.fuel_time_mili) },
        VarMeta { name: "horse_power", unit: "hp", desc: "总功率(引擎聚合)", category: C::Engine, origin: O::Derived, src: SE(|x| x.total_hp) },
        VarMeta { name: "eff_hp", unit: "hp", desc: "有效功率(引擎聚合)", category: C::Engine, origin: O::Derived, src: SE(|x| x.total_hp_eff) },
        VarMeta { name: "thrust", unit: "kgf", desc: "总推力(引擎聚合)", category: C::Engine, origin: O::Derived, src: SE(|x| x.total_thrust) },
        VarMeta { name: "water_temp", unit: "°C", desc: "水温(耐久状态机)", category: C::Engine, origin: O::Derived, src: SE(|x| x.n_water_temp) },
        VarMeta { name: "oil_temp", unit: "°C", desc: "油温(耐久状态机)", category: C::Engine, origin: O::Derived, src: SE(|x| x.n_oil_temp) },
        VarMeta { name: "energy_jkg", unit: "J/kg", desc: "单位动能(计算待移植, 现恒 0)", category: C::Flight, origin: O::Derived, src: SE(|x| x.energy_j_kg) },
        VarMeta { name: "radio_altitude", unit: "m", desc: "雷达高度(回退计算)", category: C::Flight, origin: O::Derived, src: SE(|x| x.radio_alt) },
        VarMeta { name: "compass", unit: "°", desc: "航向(罗盘回退链)", category: C::Flight, origin: O::Derived, src: SE(|x| x.compass_delta) },
        VarMeta { name: "wep_kg", unit: "kg", desc: "WEP 剩余工质(消耗状态机)", category: C::Engine, origin: O::Derived, src: SE(|x| x.nitro_kg) },
        VarMeta { name: "wep_time", unit: "s", desc: "WEP 剩余时间(消耗状态机)", category: C::Engine, origin: O::Derived, src: SE(|x| x.wep_time) },
        VarMeta { name: "heat_tolerance", unit: "", desc: "耐热阈值", category: C::Engine, origin: O::Derived, src: SE(|x| x.heat_tolerance) },
        VarMeta { name: "power_percent", unit: "%", desc: "推力/功率百分比", category: C::Engine, origin: O::Derived, src: SE(|x| x.thurst_percent) },
        VarMeta { name: "engine_response", unit: "", desc: "引擎响应速率(惯性)", category: C::Engine, origin: O::Derived, src: SE(|x| x.t_eng_response) },
        VarMeta { name: "prop_efficiency", unit: "%", desc: "螺旋桨效率(聚合比)", category: C::Engine, origin: O::Derived, src: SE(|x| x.avgeff) },
        VarMeta { name: "manifold_pressure_display", unit: "", desc: "进气压显示值(公/英制)", category: C::Engine, origin: O::Derived, src: SE(|x| x.manifold_display) },
        VarMeta { name: "fuel_percent", unit: "%", desc: "油量百分比(update_fuel 聚合)", category: C::Engine, origin: O::Derived, src: SE(|x| x.fuel_percent) },
        VarMeta { name: "is_imperial", unit: "", desc: "英制单位(检测状态机)", category: C::Meta, origin: O::Derived, src: SE(|x| x.is_imperial as u8 as f64) },
        VarMeta { name: "is_jet_engine", unit: "", desc: "喷气引擎(投票)", category: C::Engine, origin: O::Derived, src: SE(|x| x.is_jet as u8 as f64) },
        VarMeta { name: "is_prop_engine", unit: "", desc: "螺旋桨引擎(投票)", category: C::Engine, origin: O::Derived, src: SE(|x| x.is_prop as u8 as f64) },
        VarMeta { name: "is_piston_engine", unit: "", desc: "活塞引擎(投票)", category: C::Engine, origin: O::Derived, src: SE(|x| x.is_piston as u8 as f64) },
        VarMeta { name: "is_turboprop_engine", unit: "", desc: "涡桨引擎(投票)", category: C::Engine, origin: O::Derived, src: SE(|x| x.is_turboprop as u8 as f64) },
        VarMeta { name: "is_engine_check_done", unit: "", desc: "引擎检测完成(投票)", category: C::Engine, origin: O::Derived, src: SE(|x| x.engine_check_done as u8 as f64) },
        // ===== FM 字段直通 (blkx 直绑, None 守卫对齐 adapter) =====
        VarMeta { name: "fm.empty_weight", unit: "kg", desc: "空重", category: C::Fm, origin: O::Fm, src: B(|b| b.emptyweight) },
        VarMeta { name: "fm.nofuel_weight", unit: "kg", desc: "无油重量", category: C::Fm, origin: O::Fm, src: B(|b| b.nofuelweight) },
        VarMeta { name: "fm.max_fuel_weight", unit: "kg", desc: "最大油量", category: C::Fm, origin: O::Fm, src: B(|b| b.maxfuelweight) },
        VarMeta { name: "fm.critical_speed", unit: "km/h", desc: "临界速度", category: C::Fm, origin: O::Fm, src: B(|b| b.critical_speed * 3.6) },
        VarMeta { name: "fm.vne", unit: "km/h", desc: "最大速度(VNE)", category: C::Fm, origin: O::Fm, src: B(|b| b.vne) },
        VarMeta { name: "fm.vne_mach", unit: "Ma", desc: "最大马赫数", category: C::Fm, origin: O::Fm, src: B(|b| b.vne_mach) },
        VarMeta { name: "fm.full_fuel_pos_g", unit: "G", desc: "满油正过载限制", category: C::Fm, origin: O::Fm, src: B(|b| b.raw_wing_crit_overload.map_or(0.0, |r| 1.2 * (2.0 * r[1] / (9.80 * b.grossweight) - 1.0))) },
        VarMeta { name: "fm.full_fuel_neg_g", unit: "G", desc: "满油负过载限制", category: C::Fm, origin: O::Fm, src: B(|b| b.raw_wing_crit_overload.map_or(0.0, |r| 1.2 * (2.0 * r[0] / (9.80 * b.grossweight) + 1.0))) },
        VarMeta { name: "fm.half_fuel_pos_g", unit: "G", desc: "半油正过载限制", category: C::Fm, origin: O::Fm, src: B(|b| b.raw_wing_crit_overload.map_or(0.0, |r| 1.2 * (2.0 * r[1] / (9.80 * b.halfweight) - 1.0))) },
        VarMeta { name: "fm.half_fuel_neg_g", unit: "G", desc: "半油负过载限制", category: C::Fm, origin: O::Fm, src: B(|b| b.raw_wing_crit_overload.map_or(0.0, |r| 1.2 * (2.0 * r[0] / (9.80 * b.halfweight) + 1.0))) },
        VarMeta { name: "fm.elevator_eff_speed", unit: "km/h", desc: "升降舵生效速度", category: C::Fm, origin: O::Fm, src: B(|b| b.elav_eff) },
        VarMeta { name: "fm.aileron_eff_speed", unit: "km/h", desc: "副翼生效速度", category: C::Fm, origin: O::Fm, src: B(|b| b.aileron_eff) },
        VarMeta { name: "fm.rudder_eff_speed", unit: "km/h", desc: "方向舵生效速度", category: C::Fm, origin: O::Fm, src: B(|b| b.rudder_eff) },
        VarMeta { name: "fm.elevator_power_loss", unit: "km/h", desc: "升降舵锁速", category: C::Fm, origin: O::Fm, src: B(|b| b.elav_power_loss) },
        VarMeta { name: "fm.aileron_power_loss", unit: "km/h", desc: "副翼锁速", category: C::Fm, origin: O::Fm, src: B(|b| b.aileron_power_loss) },
        VarMeta { name: "fm.rudder_power_loss", unit: "km/h", desc: "方向舵锁速", category: C::Fm, origin: O::Fm, src: B(|b| b.rudder_power_loss) },
        VarMeta { name: "fm.nitro_amount", unit: "kg", desc: "氧化亚氮量", category: C::Fm, origin: O::Fm, src: B(|b| b.nitro) },
        // WEP 装配判定 = 原 hasWep() getter 直绑 (无 FM → NaN → var_value None → 恒 false, 对位原 false)
        VarMeta { name: "has_wep", unit: "", desc: "有 WEP(nitro>0)", category: C::Engine, origin: O::Fm, src: B(|b| (b.nitro > 0.0) as u8 as f64) },
        VarMeta { name: "fm.nitro_time", unit: "s", desc: "氧化亚氮时间", category: C::Fm, origin: O::Fm, src: B(|b| if b.nitro_decr <= 0.0 { 0.0 } else { b.nitro / (b.nitro_decr * 60.0) }) },
        VarMeta { name: "fm.avg_eng_recovery_rate", unit: "", desc: "平均引擎恢复率", category: C::Fm, origin: O::Fm, src: B(|b| b.avg_eng_recovery_rate) },
        VarMeta { name: "fm.no_flap_wing_load", unit: "kg/m²", desc: "翼载(无襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.no_flap_wll) },
        VarMeta { name: "fm.full_flap_wing_load", unit: "kg/m²", desc: "翼载(满襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.full_flap_wll) },
        VarMeta { name: "fm.moi_pitch", unit: "kg·m²", desc: "俯仰转动惯量", category: C::Fm, origin: O::Fm, src: B(|b| b.moment_of_inertia.map_or(0.0, |m| if m.len() >= 3 { m[2] } else { 0.0 })) },
        VarMeta { name: "fm.moi_roll", unit: "kg·m²", desc: "滚转转动惯量", category: C::Fm, origin: O::Fm, src: B(|b| b.moment_of_inertia.map_or(0.0, |m| if !m.is_empty() { m[0] } else { 0.0 })) },
        VarMeta { name: "fm.moi_yaw", unit: "kg·m²", desc: "偏航转动惯量", category: C::Fm, origin: O::Fm, src: B(|b| b.moment_of_inertia.map_or(0.0, |m| if m.len() >= 2 { m[1] } else { 0.0 })) },
        VarMeta { name: "fm.wing_area", unit: "m²", desc: "机翼面积", category: C::Fm, origin: O::Fm, src: B(|b| b.a_wing) },
        VarMeta { name: "fm.fuselage_area", unit: "m²", desc: "机身面积", category: C::Fm, origin: O::Fm, src: B(|b| b.a_fuselage) },
        VarMeta { name: "fm.oswalds_efficiency", unit: "", desc: "奥斯瓦尔德效率", category: C::Fm, origin: O::Fm, src: B(|b| b.oswalds_efficiency_number) },
        VarMeta { name: "fm.aspect_ratio", unit: "", desc: "展弦比", category: C::Fm, origin: O::Fm, src: B(|b| b.aspect_ratio) },
        VarMeta { name: "fm.swept_wing_angle", unit: "°", desc: "后掠角", category: C::Fm, origin: O::Fm, src: B(|b| b.swept_wing_angle) },
        VarMeta { name: "fm.cd_s", unit: "", desc: "寄生阻力系数", category: C::Fm, origin: O::Fm, src: B(|b| b.cd_s) },
        VarMeta { name: "fm.ind_cd_f", unit: "", desc: "诱导阻力系数", category: C::Fm, origin: O::Fm, src: B(|b| b.ind_cd_f) },
        VarMeta { name: "fm.radiator_cd", unit: "", desc: "散热器阻力系数", category: C::Fm, origin: O::Fm, src: B(|b| b.radiator_cd) },
        VarMeta { name: "fm.oil_radiator_cd", unit: "", desc: "滑油散热器阻力系数", category: C::Fm, origin: O::Fm, src: B(|b| b.oil_radiator_cd) },
        VarMeta { name: "fm.no_flaps_wing_cd_min", unit: "", desc: "最小阻力系数(无襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.no_flaps_wing.as_ref().map_or(0.0, |p| p.cd_min)) },
        VarMeta { name: "fm.no_flaps_wing_cl0", unit: "", desc: "零升力系数(无襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.no_flaps_wing.as_ref().map_or(0.0, |p| p.cl0)) },
        VarMeta { name: "fm.no_flaps_wing_aoa_crit_high", unit: "°", desc: "临界迎角上限(无襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.no_flaps_wing.as_ref().map_or(0.0, |p| p.aoa_crit_high)) },
        VarMeta { name: "fm.no_flaps_wing_aoa_crit_low", unit: "°", desc: "临界迎角下限(无襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.no_flaps_wing.as_ref().map_or(0.0, |p| p.aoa_crit_low)) },
        VarMeta { name: "fm.no_flaps_wing_cl_crit_high", unit: "", desc: "临界升力系数上限(无襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.no_flaps_wing.as_ref().map_or(0.0, |p| p.cl_crit_high)) },
        VarMeta { name: "fm.no_flaps_wing_cl_crit_low", unit: "", desc: "临界升力系数下限(无襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.no_flaps_wing.as_ref().map_or(0.0, |p| p.cl_crit_low)) },
        VarMeta { name: "fm.full_flaps_wing_cd_min", unit: "", desc: "最小阻力系数(满襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.full_flaps_wing.as_ref().map_or(0.0, |p| p.cd_min)) },
        VarMeta { name: "fm.full_flaps_wing_cl0", unit: "", desc: "零升力系数(满襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.full_flaps_wing.as_ref().map_or(0.0, |p| p.cl0)) },
        VarMeta { name: "fm.full_flaps_wing_aoa_crit_high", unit: "°", desc: "临界迎角上限(满襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.full_flaps_wing.as_ref().map_or(0.0, |p| p.aoa_crit_high)) },
        VarMeta { name: "fm.full_flaps_wing_aoa_crit_low", unit: "°", desc: "临界迎角下限(满襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.full_flaps_wing.as_ref().map_or(0.0, |p| p.aoa_crit_low)) },
        VarMeta { name: "fm.full_flaps_wing_cl_crit_high", unit: "", desc: "临界升力系数上限(满襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.full_flaps_wing.as_ref().map_or(0.0, |p| p.cl_crit_high)) },
        VarMeta { name: "fm.full_flaps_wing_cl_crit_low", unit: "", desc: "临界升力系数下限(满襟翼)", category: C::Fm, origin: O::Fm, src: B(|b| b.full_flaps_wing.as_ref().map_or(0.0, |p| p.cl_crit_low)) },
        VarMeta { name: "fm.fuse_cl_high", unit: "", desc: "机身最大升力因数", category: C::Fm, origin: O::Fm, src: B(|b| b.fuse_cl_high) },
        VarMeta { name: "fm.fuselage_aoa_crit_high", unit: "°", desc: "机身临界迎角上限", category: C::Fm, origin: O::Fm, src: B(|b| b.fuselage.as_ref().map_or(0.0, |p| p.aoa_crit_high)) },
        VarMeta { name: "fm.fuselage_cd_min", unit: "", desc: "机身最小阻力系数", category: C::Fm, origin: O::Fm, src: B(|b| b.fuselage.as_ref().map_or(0.0, |p| p.cd_min)) },
        VarMeta { name: "fm.fin_cd_min", unit: "", desc: "垂尾最小阻力系数", category: C::Fm, origin: O::Fm, src: B(|b| b.fin.as_ref().map_or(0.0, |p| p.cd_min)) },
        VarMeta { name: "fm.stab_cd_min", unit: "", desc: "平尾最小阻力系数", category: C::Fm, origin: O::Fm, src: B(|b| b.stab.as_ref().map_or(0.0, |p| p.cd_min)) },
        VarMeta { name: "fm.flap0_speed", unit: "km/h", desc: "襟翼档位0速度", category: C::Fm, origin: O::Fm, src: B(|b| flap_speed(b, 0)) },
        VarMeta { name: "fm.flap1_speed", unit: "km/h", desc: "襟翼档位1速度", category: C::Fm, origin: O::Fm, src: B(|b| flap_speed(b, 1)) },
        VarMeta { name: "fm.flap2_speed", unit: "km/h", desc: "襟翼档位2速度", category: C::Fm, origin: O::Fm, src: B(|b| flap_speed(b, 2)) },
        VarMeta { name: "fm.flap3_speed", unit: "km/h", desc: "襟翼档位3速度", category: C::Fm, origin: O::Fm, src: B(|b| flap_speed(b, 3)) },
        VarMeta { name: "fm.gear_destruction_speed", unit: "km/h", desc: "起落架损毁速度", category: C::Fm, origin: O::Fm, src: B(|b| b.gear_destruction_ind_speed) },
        VarMeta { name: "fm.engine_num", unit: "", desc: "引擎数(FM)", category: C::Fm, origin: O::Fm, src: B(|b| b.engine_num as f64) },
        // ===== 元变量 =====
        VarMeta { name: "interval_ms", unit: "ms", desc: "本帧轮询间隔", category: C::Meta, origin: O::Meta, src: M(MetaVar::IntervalMs) },
        VarMeta { name: "freq", unit: "Hz", desc: "轮询频率", category: C::Meta, origin: O::Meta, src: M(MetaVar::Freq) },
        VarMeta { name: "elapsed_ms", unit: "ms", desc: "会话经过时间", category: C::Meta, origin: O::Meta, src: M(MetaVar::ElapsedMs) },
        VarMeta { name: "session_ms", unit: "ms", desc: "会话开始至今", category: C::Meta, origin: O::Meta, src: M(MetaVar::SessionMs) },
        VarMeta { name: "fm_loaded", unit: "", desc: "FM 已加载", category: C::Meta, origin: O::Meta, src: M(MetaVar::FmLoaded) },
        // ===== 物理常量 =====
        VarMeta { name: "g", unit: "m/s²", desc: "重力加速度", category: C::Const, origin: O::Const, src: K(crate::physics_constants::g) },
        VarMeta { name: "rho0", unit: "kg/m³", desc: "海平面空气密度", category: C::Const, origin: O::Const, src: K(crate::physics_constants::SEA_LEVEL_DENSITY) },
        VarMeta { name: "P0", unit: "Pa", desc: "海平面气压", category: C::Const, origin: O::Const, src: K(crate::physics_constants::SEA_LEVEL_PRESSURE) },
    ];

    // 单名制 (W10): 变量只有一个名字; Java getter 名不进内核索引
    // (对拍文件边界映射见 vm-overlay fields.rs getter())
    let mut index: HashMap<&'static str, u16> = HashMap::with_capacity(vars.len());
    for (i, v) in vars.iter().enumerate() {
        index.insert(v.name, i as u16);
    }
    Registry { vars, index }
}

/// 襟翼档位速度表取值 (fm.flapN_speed 共用; adapter 同款守卫)
fn flap_speed(b: &Blkx, idx: usize) -> f64 {
    if b.flaps_destruction_num as usize > idx {
        b.flaps_destruction_ind_speed
            .as_ref()
            .and_then(|t| t.get(idx))
            .map(|r| r[1])
            .unwrap_or(0.0)
    } else {
        0.0
    }
}
