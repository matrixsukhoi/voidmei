//! fmdata — 对应 Java `src/parser/Blkx.java` 的 D4 拆分模块 (JSON 数据源终态:
//! BlkText 文本链已删, FM 数据为 wt_ext_cli `--format Json` 产物, 迁移期
//! 2832 对全量位级对拍验证等价):
//! - `types.rs` — 4 个数据类 (EngineLoad/FmParts/SweepLevel/FuelModification);
//!   燃油修正只有 JSON 版 (json.rs)
//! - 本文件 (`mod.rs`) — Blkx 聚合 struct: 完整字段区 (结构体字段不可跨文件拆分)
//! - `model.rs` — getter/计算方法 (findmax*/getVersion/peakThrust)
//! - `reader.rs` — getload_from 装载主体 (JSON 直供, 抽取原语在 json.rs)
//! - `json.rs` — JSON 后端: 树寻址原语 + get_f64 数值直读族 + parse_named_json
//!   构造入口 + 燃油修正树版
//!
//! 2026-09 死代码清理: Java 遗留死存储与无消费面字段已删 —
//! PASSPORT 曲线链 (loc..loc3/XY/transUnit/getplotdata, Java DrawFrame 的
//! 消费未迁移至 Rust)、MIL 推力表与 peak MIL、平铺 aoa/clmax 快照族、
//! v50/v100 快照字段 (摘要串用局部变量)、oilload/wtload/tmload 装箱遗留
//! (会话状态由 FMHandle.eng_load_state 承接)、wx* 死存储、cl_a/aileron_defl、
//! 原始文本 data 串 (init/finalizeLoading 一并退役)。
//!
//! PORT: 反射段 (getValue/dumpVariables, L1908-2000) 按 D4 裁决不迁移。
//! interpolateSweepDouble (L718) 由 crate::base::interpolation::interp_sweep_level 承接
//! (单一来源规约, 见 model.rs 函数级注)。

mod model;
// FM JSON 数据源 (wt_ext_cli --format Json 产物): 树寻址原语 + get_f64 数值
// 直读族 + 中央文件燃油修正树版 + parse_named_json 族入口
pub mod flap_limits;
pub mod json;
mod reader;
mod types;

// 真机 FM 集成测试 (D4 验收项): TestSpitfireF24Power/TestTempestMk5Power/
// FMParserFuzzer 三套 Java 测试的一比一移植; data/ 缺失自动跳过 (对齐 build.py)
#[cfg(test)]
mod realtests;


pub use flap_limits::{get_flap_allow_angle, get_flap_allow_speed};
pub use types::{EngineLoad, FuelModification, FuelType, FmParts, SweepLevel};

/// 对应 Java `public class Blkx` (L14) 的聚合 struct — 字段区宿主 (D4)。
///
/// 字段区覆盖 Java 实例字段的存活子集 (死字段见模块头注), 类型对齐:
/// double→f64 / int→i32 / String 与对象引用的 null-未赋值态→Option;
/// 定长数组 `new double[N]`→`[f64; N]`, 变长/jagged→Vec。
// PORT: Java private 字段 → 无 pub (fmdata 模块树内可见); 刻意不 derive
// PartialEq — Java 无 equals 覆写, 语义只有引用同一性 (FMHandle 同款先例)。
#[derive(Debug, Clone, Default)]
pub struct FmData {
    // ---- L15 / L234-241 ----
    pub valid: bool,
    pub read_file_name: Option<String>,

    // ---- L243-244 ----
    // 发动机负载相关
    pub fmdata: Option<String>,

    // ---- L255-273 ----
    pub eng_load: Option<Vec<EngineLoad>>,
    pub max_eng_load: i32,

    // ---- L275-281 ----
    pub vne: f64,
    pub vne_mach: f64,

    // ---- L292-293 ----
    pub emptyweight: f64,

    // ---- L295 ----
    pub max_allow_gload: Option<[f64; 2]>,

    // ---- L297-298 ----
    /// Raw wing critical overload values (Newtons) for dynamic G-load calculation
    pub raw_wing_crit_overload: Option<[f64; 2]>,

    // ---- L300-327 ----
    pub aileron_eff: f64,
    pub aileron_power_loss: f64,
    pub rudder_eff: f64,
    pub rudder_power_loss: f64,
    pub elav_eff: f64,
    pub elav_power_loss: f64,
    pub nitro: f64,
    pub grossweight: f64,
    pub oil: f64,
    pub nofuelweight: f64,
    pub nitro_decr: f64,
    pub maxfuelweight: f64,
    pub wing_angle: f64,
    pub stab_angle: f64,
    pub keel_angle: f64,
    pub radiator_cd: f64,
    pub oil_radiator_cd: f64,
    pub oswalds_efficiency_number: f64,

    // ---- L364-379 ----
    /// Dynamic list of sweep levels, ordered by sweep ratio (0.0 to 1.0)
    // PORT: Java List<SweepLevel> null-未赋值 → Option<Vec<..>>
    pub sweep_levels: Option<Vec<SweepLevel>>,
    pub no_flaps_wing: Option<FmParts>,
    pub full_flaps_wing: Option<FmParts>,
    // PORT: Java Boolean 装箱 (getload 前为 null, 拆箱 NPE) → Option<bool>
    pub is_v_wing: Option<bool>,
    pub fuselage: Option<FmParts>,
    pub fin: Option<FmParts>,
    pub stab: Option<FmParts>,

    // ---- L380-404 ----
    pub swept_wing_angle: f64,
    pub critical_speed: f64,
    pub a_wing_left_in: f64,
    pub a_wing_left_mid: f64,
    pub a_wing_left_out: f64,
    pub a_wing_right_in: f64,
    pub a_wing_right_mid: f64,
    pub a_wing_right_out: f64,
    pub a_fuselage: f64,
    pub a_wing: f64,
    pub no_flap_wll: f64,
    pub full_flap_wll: f64,
    pub cd_s: f64,
    pub moment_of_inertia: Option<[f64; 3]>,
    pub a_aileron: f64,
    pub wingspan: f64,
    pub aspect_ratio: f64,
    pub ind_cd_f: f64,
    pub version: Option<String>,
    pub avg_eng_recovery_rate: f64,
    pub flaps_destruction_num: i32,
    // PORT: Java new double[6][2] (+1 行是 1.25x 襟翼插值哨兵, 见 getload) →
    // 定长 [[f64; 2]; 6]
    pub flaps_destruction_ind_speed: Option<[[f64; 2]; 6]>,
    pub halfweight: f64,

    // ---- L633-648 喷气推力表 ----
    // PORT: Java new double[30] 定长缓冲 → [f64; N]; 有效数据前缀长度由
    // alt_thr_num/vel_thr_num/mode_engine_num 记录
    pub altitude_thr: Option<[f64; 30]>,
    pub velocity_thr: Option<[f64; 30]>,
    pub max_thr_aft_coff: Option<Vec<Vec<f64>>>,
    pub max_thr_aft: Option<Vec<Vec<f64>>>,
    pub thr_max0: f64, // 静推力
    pub aftb_coff: f64,
    pub alt_thr_num: i32,
    pub vel_thr_num: i32,
    pub is_jet: bool,
    // 加力峰值推力缓存（在 getload() 中预计算; 军用表无消费方已删）
    pub peak_thr_aft: f64, // 加力峰值推力 (kgf)
    pub engine_num: i32,

    // ---- L650-674 增压器 ----
    // PORT: new double[compNumSteps] 运行时长度 → Vec
    pub comp_num_steps: i32,
    pub comp_alt: Option<Vec<f64>>,
    pub comp_power: Option<Vec<f64>>,
    pub comp_ceil: Option<Vec<f64>>,
    pub comp_ceil_pwr: Option<Vec<f64>>,
    pub comp_const_rpm_alt: Option<Vec<f64>>,
    pub comp_const_rpm_power: Option<Vec<f64>>,
    pub comp_boost: Option<Vec<f64>>,
    pub has_comp_boost: Option<Vec<bool>>, // Whether AfterburnerBoostMul exists in FM file (vs defaulting to 0)
    pub comp_rpm_ratio: Option<Vec<f64>>,
    // 冲压系数
    pub speed_to_manifold_multiplier: f64,
    mode_engine_num: i32,
    pub a_wing_right_cut: f64,
    pub a_wing_left_cut: f64,
    pub gear_destruction_ind_speed: f64,
    pub max_rpm: f64,

    // ---- L676-715 === Piston Engine Extended Parameters (for WAPC-compatible power calculations) === ----
    // RPM parameters
    pub military_rpm: f64, // Military power RPM (from Propeller.ThrottleRPMAuto)
    pub wep_rpm: f64,      // WEP power RPM (from Propeller.ThrottleRPMAuto)
    pub shaft_rpm_max: f64, // Main.ShaftRPMMax (may not exist)
    pub rpm_nom: f64,      // Main.RPMNom
    pub governor_max_param: f64, // Propeller.GovernorMaxParam

    // Supercharger pressure parameters
    pub comp_pressure_at_rpm0: f64, // Compressor.CompressorPressureAtRPM0
    pub comp_omega_factor_sq: f64,  // Compressor.CompressorOmegaFactorSq
    pub has_comp_omega_factor_sq: bool, // Whether CompressorOmegaFactorSq exists in FM file
    pub comp_ata: Option<Vec<f64>>, // Compressor.ATA0/1/2 (manifold pressure, ata)
    pub military_mp: f64, // Max of all ATA values (true military manifold pressure)
    pub comp_afterburner_pressure_boost: Option<Vec<f64>>, // Compressor.AfterburnerPressureBoost0/1/2

    // ExactAltitudes flag (may be explicitly set in FM file)
    pub explicit_exact_altitudes: Option<bool>, // null = not defined in FM

    // WEP parameters
    pub throttle_boost: f64,         // Main.ThrottleBoost (typically 1.0)
    pub octane_afterburner_mult: f64, // Main.OctaneAfterburnerMult (typically 1.0)
    pub wep_manifold_pressure: f64,  // AfterburnerManifoldPressure (WEP manifold pressure, ata)

    // Sea level power
    pub deck_power: f64, // Main.Power (sea level rated power)

    // getload 内部中转 (Java private 字段保真): WEP 档 RPM 乘数 (幻影2000C 修复)
    engine_rpm_mult_wep: f64,
    pub fuse_cl_high: f64,
}
