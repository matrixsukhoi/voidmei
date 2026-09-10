//! 对应 Java: `src/parser/Blkx.java` 的内部类区 (D4 拆分: types.rs)。
//! 覆盖 4 个内部类:
//! - `FuelModification` (static 内部类; 提取逻辑在 json.rs 树版)
//! - `engineLoad` / `fm_parts` / `SweepLevel`
//!   — 非静态内部类, 均未引用 Blkx.this 外部状态 (纯语法糖) → 独立 struct
//!
//! `XY` (PASSPORT 曲线容器) 已随曲线链删除 — Java DrawFrame
//! 的消费未迁移至 Rust, Rust 生产零消费 (2026-09 死代码清理)。

use std::fmt;

// ==================== Fuel Modification Support ====================

/// 对应 Java `public static class FuelModification`。
/// Represents fuel quality modifications extracted from Central file's
/// "modifications" section. These upgrades affect engine power output.
///
/// <p>War Thunder Central files (e.g., flightmodels/yak-3.blkx) contain:
/// <pre>
/// modifications {
///   ussr_fuel_b-100 {
///     effects {
///       addHorsePowers:r = 50
///     }
///   }
/// }
/// </pre>
// 刻意不 derive PartialEq — Java 无 equals 覆写, 语义只有引用同一性,
// 全库使用点 (FMLoader/FMPowerExtractor/PowerCurveWindow) 均只比较 type 枚举
// 与读数值字段, 从不比较 FuelModification 整体 (FMHandle 同款先例)。
#[derive(Debug, Clone)]
pub struct FuelModification {
    /// Soviet fuel addHorsePowers value (typically 50)
    pub soviet_octane_hp_bonus: f64,
    /// British afterburnerMult from fuel modification
    pub british_afterburner_mult: f64,
    /// British afterburnerCompressorMult from fuel modification
    pub british_afterburner_compressor_mult: f64,
    /// Whether British fuel has invertEnableLogic (means high octane is default)
    pub british_invert_logic: bool,
    /// Fuel modification type
    // Java 字段名 `type` 是 Rust 关键字 → `r#type` (gauge_marker.rs 先例)
    pub r#type: FuelType,
}

/// 对应 Java 字段初始化器 (`= 0` / `= 1.0` / `= false` / `= FuelType.NONE`)。
impl Default for FuelModification {
    fn default() -> Self {
        FuelModification {
            soviet_octane_hp_bonus: 0.0,
            british_afterburner_mult: 1.0,
            british_afterburner_compressor_mult: 1.0,
            british_invert_logic: false,
            r#type: FuelType::None,
        }
    }
}

impl FuelModification {
    /// 对应 Java `new FuelModification()` — 无参构造, 全字段取初始化器值。
    pub fn new() -> Self {
        Self::default()
    }
}

/// 对应 Java `public enum FuelModification.FuelType`。
// Java 枚举常量全大写 → Rust 驼峰 (FMStatus 先例); Java 枚举默认
// toString()=常量名 的字符串形态由 Display 保留 (FMLoader 日志
// "Fuel modification detected: " + fuelMod.type 依赖, 历史基线 对拍)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuelType {
    None,
    SovietB95,
    SovietB100,
    British150Octane,
    British100Spitfire,
}

/// 对应 Java 枚举默认 `toString()` = 常量名 (`name()`)。
impl fmt::Display for FuelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FuelType::None => "NONE",
            FuelType::SovietB95 => "SOVIET_B95",
            FuelType::SovietB100 => "SOVIET_B100",
            FuelType::British150Octane => "BRITISH_150_OCTANE",
            FuelType::British100Spitfire => "BRITISH_100_SPITFIRE",
        };
        f.write_str(s)
    }
}

// ==================== End Fuel Modification Support ====================

/// 对应 Java `public class engineLoad` (源文件类名即小驼峰)。
/// 非静态内部类, 六个 double 字段无任何 Blkx 外部引用 → 独立 struct;
/// Java 字段隐式零初始化 → Default 复刻。类名 engineLoad → EngineLoad。
#[derive(Debug, Clone, Default)]
pub struct EngineLoad {
    pub water_limit: f64,
    pub oil_limit: f64,
    pub work_time: f64,
    pub recover_time: f64,
    pub cur_water_work_time_mili: f64,
    pub cur_oil_work_time_mili: f64,
}

/// 对应 Java `public class fm_parts` (源文件类名即小写下划线)。
/// 非静态内部类, 未引用 Blkx 外部状态 → 独立 struct;
/// 类名 fm_parts → FmParts。Java 字段隐式零初始化/null → Default;
/// name 未赋值的 null ↔ None (唯一赋值点是 getPartsFm, 后续波次)。
#[derive(Debug, Clone, Default)]
pub struct FmParts {
    pub name: Option<String>,

    pub sq: f64,
    pub cd_min: f64,

    pub cl0: f64,

    pub cl_crit_high: f64,
    pub cl_crit_low: f64,

    pub cl_after_crit: f64,

    pub aoa_crit_high: f64,
    pub aoa_crit_low: f64,

    pub line_cl_coeff: f64,
    // 翼展效率因数，影响诱导阻力，因数越大阻力越小
    // public double oswaldEff;
}

/// 对应 Java `public class SweepLevel`。
/// Represents a single sweep level with its associated aerodynamic data.
/// Used for variable-sweep wing aircraft (e.g., F-14 with 4 sweep levels).
/// 非静态内部类, 持有的两个 fm_parts 是构造后即赋值的值字段而非
/// Blkx 外部引用 → 独立 struct; noFlaps/fullFlaps 在 `new SweepLevel()` 后、
/// 构造器赋值前为 null ↔ Option<FmParts> + Default (None)。
/// Java 的引用共享 (`NoFlapsWing_V50 = sweepLevels.get(1).noFlaps`) 由后续
/// 字段波次以值克隆承接 — 解析完成后 fm_parts 只读, 无行为差异。
#[derive(Debug, Clone, Default)]
pub struct SweepLevel {
    /// 0.0 ~ 1.0 sweep ratio (from Sweep:r field)
    pub sweep: f64,
    /// VNE limit speed for this sweep level
    pub vne: f64,
    /// MNE limit (Mach) for this sweep level
    pub vne_mach: f64,
    /// No-flaps aerodynamic data
    pub no_flaps: Option<FmParts>,
    /// Full-flaps aerodynamic data
    pub full_flaps: Option<FmParts>,
}

// =====================================================================
// Tests — Java 无内部类独立测试; 公共面按"每个公共项写边界测试"规则补齐,
// 期望值来自 历史基线 对拍: Blkx.java L34-218 逐字提取为独立
// 可编译类 (该段零外部依赖), OpenJDK 1.8.0_342 实测 dump, 临时文件用完已删。
// =====================================================================
#[cfg(test)]
mod tests;
