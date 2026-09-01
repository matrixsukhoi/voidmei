//! blkx — 对应 Java `src/parser/Blkx.java` (2029 行, B 类) 的 D4 拆分四模块:
//! - `types.rs` — 5 个内部类 + Fuel Modification Support 静态函数区 (L34-660)【已译】
//! - 本文件 (`mod.rs`) — Blkx 聚合 struct: **完整字段区** (L15 / L234-404 / L633-715,
//!   D4: "Blx 聚合 struct" 的宿主; Rust 结构体字段不可跨文件拆分, 故字段区落此)【本波】
//! - `model.rs` — getter/计算方法 (L523-631 的 findmax*/getVersion、L676-1660 的纯字段
//!   计算面、L1978-2028 的 finalizeLoading/calculatePeakThrust/peakThrust)【本波】
//! - `reader.rs` — 构造器 + 原语 + getload 全量装载 (L408-575/L817-1590/L1665-1906)
//!   → `parse/parse_named/parse_named_opts -> Result<Blkx>`【getload 批次 +
//!   getAllplotdata 批次 (transUnit/getAllplotdata/getplotdata) 均已译,
//!   真机/合成英制位级对拍】
//!
//! PORT: 反射段 (getValue/dumpVariables/getVariableMap, L1908-2000) 按 D4 裁决
//! **不迁移** (getVariableMap 唯一下游 FormulaEvaluator 归 C 类; FMPowerExtractor
//! 直读字段; dumpVariables 是调试工具) — 具体标注职责落在 reader.rs 波次。
//!
//! PORT (方法波次边界): 依赖原始文本抽取原语 getone/cut/getArray/getlastone/
//! getoneinData (L1728-1900, 归 reader.rs 波次) 的方法族已随 getload 批次在
//! reader.rs 落地 (真机 spitfire 位级对拍): getPartsFm (L408) /
//! extractRpmFromThrottleAuto (L431) / getEngineLoad (L477) / showEngineLoad (L496) /
//! WritePartsFm (L502) / getdoubles (L523) / getdouble (L543) / getdouble_exc (L557) /
//! initEngineLoad (L817) / getload (L855); getAllplotdata 批次补齐曲线族:
//! transUnit (L1590, 喂入 sub_st 的 PASSPORT.UNITSYSTEM 行值保持 ASCII 域 §2.1,
//! 见 model.rs sub_st 函数级注) / getAllplotdata (L1618) / getplotdata (L1627)
//! — fm_loader 接线 + fuzz 腿1 管线恢复, 真机 bf-109e-4 metric 路径 + 合成英制
//! 变体 (DumpPlot oracle) 位级对拍。
//! interpolateSweepDouble (L718) 由 crate::interpolation::interp_sweep_level 承接
//! (单一来源规约, 见 model.rs 函数级注)。
//!
//! 字段波次陷阱落地记录:
//! 1. `oilload0~5`/`wtload0~5` (L257-268) 是 Java `Float` 装箱、可 null →
//!    `Option<f32>` (是 f32 不是 f64, 全库其余 double 才是 f64);
//! 2. `getdoubles`/`getdouble`/`getdouble_exc` (L523-569) 用 `Float.parseFloat`
//!    赋值 double (24-bit 尾数, 如 1.42f != 1.42) → reader 波次必须
//!    `parse::<f32>() as f64`, 勿照抄 types.rs 的 `parse::<f64>` (那边对应
//!    `Double.parseDouble`);
//! 3. `extract_fuel_modifications` 的 &str 收窄依赖调用方判空 (FMLoader.java L76
//!    `lookupBlkx.valid && lookupBlkx.data != null`; PowerCurveWindow 读文件失败
//!    早退) — 两调用方波次的 Option 层须承接 null 分支, 见 types.rs 函数级注;
//! 4. `FuelType` 已由 Java 嵌套类型提升为 blkx 顶层导出 (§0.6 扁平化), 未来
//!    crate 引入其他 FuelType 时注意命名冲突;
//! 5. engLoad 会话状态的就地改写语义 (Service 线程 ~10Hz 递减 curWaterWorkTimeMili,
//!    见 FMHandle 类注) → reader 波次落地 Blkx 构造时需以内部可变性承接
//!    (fm/handle.rs PORT 注预留; 其 BlkxPlaceholder → Blkx 切换亦在该波次)。

mod model;
mod reader;
mod types;

// 真机 FM 集成测试 (D4 验收项): TestSpitfireF24Power/TestTempestMk5Power/
// FMParserFuzzer 三套 Java 测试的一比一移植; data/ 缺失自动跳过 (对齐 build.py)
#[cfg(test)]
mod realtests;

pub use types::{
    extract_fuel_modifications, EngineLoad, FuelModification, FuelType, FmParts, SweepLevel, XY,
};

/// 对应 Java `public class Blkx` (L14) 的聚合 struct — 字段区宿主 (D4)。
///
/// 字段区完整覆盖 Java 全部实例字段 (L15 / L234-404 / L633-715), 声明顺序与 Java
/// 一致, 注释逐字保留。类型对齐: double→f64 / int→i32 / `Float` 装箱→Option<f32> /
/// String 与对象引用的 null-未赋值态→Option (§1, §2.10 隐式初始化由 Default 承接);
/// 定长数组 `new double[N]`→`[f64; N]`, 变长/jagged→Vec (§1)。
// PORT: Java private 字段 → 无 pub (blkx 模块树内可见, 供 model/reader 子模块的
// impl 块访问, 对应 Java "类内可见"); 其中数个在本波无读写方 (写入方在 reader 波次
// getload, Wx* 族在 Java 里本就只剩注释引用) — 逐字段 allow(dead_code) 标注, reader
// 波次落地后即可去除。
// PORT: 刻意不 derive PartialEq — Java 无 equals 覆写, 语义只有引用同一性 (FMHandle
// 同款先例)。
#[derive(Debug, Clone, Default)]
pub struct Blkx {
    // ---- L15 ----
    pub valid: bool,

    // ---- L234-241 ----
    pub data: Option<String>,
    pub read_file_name: Option<String>,
    pub loc: Option<XY>,  // WEP爬升
    pub loc0: Option<XY>, // NOM爬升
    pub loc1: Option<XY>, // WEP速度
    pub loc2: Option<XY>, // NOM速度
    pub loc3: Option<XY>, // 滚转
    pub plotdata: Option<Vec<XY>>,

    // ---- L243-244 ----
    // 发动机负载相关
    pub fmdata: Option<String>,

    // ---- L255-273 ----
    pub eng_load: Option<Vec<EngineLoad>>,
    pub max_eng_load: i32,
    // PORT: Java Float 装箱 (可 null) → Option<f32>, 注意是 f32 (见模块注 1)
    pub oilload0: Option<f32>,
    pub oilload1: Option<f32>,
    pub oilload2: Option<f32>,
    pub oilload3: Option<f32>,
    pub oilload4: Option<f32>,
    pub oilload5: Option<f32>,
    pub wtload0: Option<f32>,
    pub wtload1: Option<f32>,
    pub wtload2: Option<f32>,
    pub wtload3: Option<f32>,
    pub wtload4: Option<f32>,
    pub wtload5: Option<f32>,
    pub tmload1: i32,
    pub tmload2: i32,
    pub tmload3: i32,
    pub tmload4: i32,
    pub tmload5: i32,

    // ---- L275-281 ----
    pub vne: f64,
    pub vne_v50: f64,
    pub vne_v100: f64,
    pub vne_mach: f64,
    pub vne_mach_v50: f64,
    pub vne_mach_v100: f64,

    // ---- L283-287 ----
    pub clmax: f64,
    pub aoa_high: f64,
    pub aoa_low: f64,
    pub flap_aoa_high: f64,
    pub flap_aoa_low: f64,

    // ---- L289-290 ----
    pub aoa_fuselage_high: f64,
    pub aoa_fuselage_low: f64,

    // ---- L292-293 ----
    pub flap_clmax: f64,
    pub emptyweight: f64,

    // ---- L295 ----
    pub max_allow_gload: Option<[f64; 2]>,

    // ---- L297-298 ----
    /// Raw wing critical overload values (Newtons) for dynamic G-load calculation
    pub raw_wing_crit_overload: Option<[f64; 2]>,

    // ---- L300-327 ----
    pub emptyweight_to_load: i32,
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
    pub fm_cd_min: f64,
    pub wing_angle: f64,
    pub stab_angle: f64,
    pub keel_angle: f64,
    pub radiator_cd: f64,
    pub oil_radiator_cd: f64,
    pub airbrake_cd: f64,
    pub oswalds_efficiency_number: f64,

    // ---- L364-379 ----
    /// Dynamic list of sweep levels, ordered by sweep ratio (0.0 to 1.0)
    // PORT: Java List<SweepLevel> null-未赋值 → Option<Vec<..>>; NoFlapsWing_V50 等
    // 在 Java 里与 sweepLevels 元素共享引用, Rust 以值克隆承接 (解析后只读,
    // types.rs SweepLevel 注同款裁决)
    pub sweep_levels: Option<Vec<SweepLevel>>,
    pub no_flaps_wing: Option<FmParts>,
    pub no_flaps_wing_v50: Option<FmParts>,
    pub no_flaps_wing_v100: Option<FmParts>,
    pub full_flaps_wing: Option<FmParts>,
    pub full_flaps_wing_v50: Option<FmParts>,
    pub full_flaps_wing_v100: Option<FmParts>,
    // PORT: Java Boolean 装箱 (getload 前为 null, 拆箱 NPE) → Option<bool>
    pub is_v_wing: Option<bool>,
    pub fuselage: Option<FmParts>,
    pub fin: Option<FmParts>,
    pub stab: Option<FmParts>,

    // ---- L380-404 ----
    pub swept_wing_angle: f64,
    pub wing_taper_ratio: f64,
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
    // PORT: Java new double[6][2] (+1 行是 1.25x 襟翼插值哨兵, 见 getload L1188) →
    // 定长 [[f64; 2]; 6]
    pub flaps_destruction_ind_speed: Option<[[f64; 2]; 6]>,
    pub halfweight: f64,

    // ---- L633-648 喷气推力表 ----
    // PORT: Java new double[30] / new double[10] 定长缓冲 → [f64; N]; 有效数据
    // 前缀长度由 alt_thr_num/vel_thr_num/mode_engine_num 记录
    pub altitude_thr: Option<[f64; 30]>,
    pub velocity_thr: Option<[f64; 30]>,
    pub max_thr_coff: Option<Vec<Vec<f64>>>,
    pub max_thr_aft_coff: Option<Vec<Vec<f64>>>,
    pub max_thr: Option<Vec<Vec<f64>>>,
    pub max_thr_aft: Option<Vec<Vec<f64>>>,
    pub thr_max0: f64, // 静推力
    pub aftb_coff: f64,
    pub alt_thr_num: i32,
    pub vel_thr_num: i32,
    pub is_jet: bool,
    // 峰值推力缓存（在 getload() 中预计算）
    pub peak_thr_mil: f64, // 军用峰值推力 (kgf)
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
    pub mode_engine_mult: Option<[f64; 10]>,
    // 冲压系数
    pub speed_to_manifold_multiplier: f64,
    mode_engine_num: i32,
    a_wing_right_cut: f64,
    a_wing_left_cut: f64,
    pub gear_destruction_ind_speed: f64,
    pub max_rpm: f64,
    pub max_allowed_rpm: f64,

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

    // PORT: 以下均为 Java private 字段 (本波无读取方: 写入方在 reader 波次 getload,
    // Wx* 族在 Java 里仅剩注释引用即死字段, 保真保留)
    #[allow(dead_code)] // getload 写入后无读取 (Java 亦然, 死存储保真保留)
    cl_a: f64,
    #[allow(dead_code)] // getload 写入; 读取方全在被注释的滚转率代码里 (Java 亦死)
    aileron_defl: Option<[f64; 2]>,
    #[allow(dead_code)]
    wx100: f64,
    #[allow(dead_code)]
    wx_vcoff: f64,
    #[allow(dead_code)]
    wx250: f64,
    #[allow(dead_code)]
    wx300: f64,
    #[allow(dead_code)]
    wx350: f64,
    #[allow(dead_code)]
    wx_max: f64,
    #[allow(dead_code)]
    wx600: f64,
    mode_engine_rpm_mult: Option<[f64; 10]>,
    engine_rpm_mult_wep: f64,
    #[allow(dead_code)]
    full_flaps_wing_s: Option<FmParts>,
    #[allow(dead_code)]
    no_flaps_wing_s: Option<FmParts>,
    pub fuse_cl_high: f64,
}
