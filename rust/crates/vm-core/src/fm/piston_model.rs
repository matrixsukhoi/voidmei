//! Piston engine power curve calculation model.
//!
//! Ported from: wt-aircraft-performance-calculator/plane_power_calculator.py
//!
//! This model calculates engine power output at any altitude by considering:
//! - Supercharger staging and critical altitudes
//! - WEP (War Emergency Power) boost effects
//! - RPM effects on torque and supercharger efficiency
//! - RAM air effect from forward motion
//!
//! Physical Background
//! ==================
//!
//! Supercharger Critical Altitude: The altitude up to which the supercharger
//! can maintain rated manifold pressure. Above this altitude, power drops
//! proportionally with ambient pressure.
//!
//! Torque Curve Model: Engine torque follows an inverted parabola with
//! peak torque at approximately 75% of maximum RPM. This affects how power
//! scales with RPM changes between military and WEP settings.
//!
//! Multi-Stage Superchargers: Some engines have 2-3 supercharger speeds.
//! Each stage is optimized for a different altitude band. The model selects
//! the stage providing maximum power at each altitude.
//!
//! Accuracy: The original Python implementation claims ±1% accuracy for 95%+
//! of aircraft when compared against War Thunder actual flight model
//! calculations.

use crate::base::atmosphere_model::{altitude_at_pressure, pressure, ram_effect_altitude};
use crate::base::format::java_round;
// PowerCurveHelper 判定函数集 (fm/power_curve.rs, 逐函数同实现)
use crate::fm::power_curve::{
    ceiling_is_useful, const_rpm_above_crit_alt, const_rpm_below_crit_alt, const_rpm_below_deck,
    const_rpm_below_wep_crit_alt, has_const_rpm, power_is_deck_power,
};

// ==================== Torque/RPM Calculations ====================

/// Calculates the power multiplier due to RPM change.
///
/// Based on a parabolic torque curve model where maximum torque occurs
/// at 75% of the higher RPM value. The torque curve is:
/// `τ = -rpm² + 2×b×rpm`
/// where b = 0.75 × higherRPM
///
/// This models real piston engine behavior where torque peaks at
/// moderate RPM and falls off at both low and high RPM extremes.
///
/// - `lower_rpm`: lower RPM value (e.g., military power RPM)
/// - `higher_rpm`: higher RPM value (e.g., WEP RPM)
///
/// Returns power ratio (higherRPM power / lowerRPM power), typically 1.0-1.15
pub fn torque_rpm_boost(lower_rpm: f64, higher_rpm: f64) -> f64 {
    if lower_rpm <= 0.0 || higher_rpm <= 0.0 {
        return 1.0;
    }

    // Peak torque occurs at 75% of higher RPM
    let torque_max_rpm = 0.75 * higher_rpm;

    // Torque at each RPM: τ = rpm × (2×b×rpm - rpm²) = rpm × τ_curve
    // Power = torque × rpm, so we compute rpm × (2b - rpm) × rpm
    let high_term = higher_rpm * (2.0 * torque_max_rpm * higher_rpm - higher_rpm * higher_rpm);
    let low_term = lower_rpm * (2.0 * torque_max_rpm * lower_rpm - lower_rpm * lower_rpm);

    if low_term <= 0.0 {
        return 1.0;
    }
    high_term / low_term
}

/// Calculates propeller shaft torque from horsepower.
///
/// Derived from: P = τ × ω = τ × (2π × RPM / 60)
/// Rearranged: τ = P × 60 / (2π × RPM)
/// With unit conversion (hp to W, N·m to kgf·m): τ = 726.115 × P / RPM
///
/// - `power_hp`: engine power in horsepower
/// - `reduct_rpm`: propeller RPM after reduction gear
///
/// Returns torque in kgf·m
pub fn torque_from_hp(power_hp: f64, reduct_rpm: f64) -> f64 {
    if reduct_rpm <= 0.0 {
        return 0.0;
    }
    726.115 * power_hp / reduct_rpm
}

// ==================== Supercharger Efficiency ====================

/// Calculates the supercharger efficiency boost from increased RPM.
///
/// The supercharger is mechanically driven by the engine crankshaft.
/// Higher RPM means the supercharger spins faster, producing more boost.
/// This effect is non-linear due to compressor characteristics.
///
/// The formula models this relationship:
/// ```text
/// effect = (1 + (1 - pressureAtRPM0) / milRPM × (wepRPM - milRPM))^(1 + omegaFactorSq)
/// ```
///
/// - `military_rpm`: military power RPM
/// - `wep_rpm`: WEP mode RPM
/// - `compressor_pressure_at_rpm0`: supercharger pressure ratio at RPM=0 (typically 0.1-0.3)
/// - `compressor_omega_factor_sq`: supercharger angular velocity factor squared
///
/// Returns supercharger efficiency multiplier, typically 1.0-1.3
pub fn supercharger_rpm_effect(
    military_rpm: f64,
    wep_rpm: f64,
    compressor_pressure_at_rpm0: f64,
    compressor_omega_factor_sq: f64,
) -> f64 {
    if military_rpm <= 0.0 {
        return 1.0;
    }

    let rpm_diff = wep_rpm - military_rpm;
    let base = 1.0 + ((1.0 - compressor_pressure_at_rpm0) / military_rpm) * rpm_diff;
    // Math.pow 与 f64::powf 语义一致 (§2.3)
    base.powf(1.0 + compressor_omega_factor_sq)
}

// ==================== Power Interpolation ====================

/// Interpolates power between two altitude points.
///
/// This is the core interpolation formula used throughout the model.
/// Power varies with altitude based on a pressure ratio raised to a
/// curvature exponent:
///
/// ```text
/// power = P_lower + ΔP × |((p_target - p_lower) / (p_higher - p_lower))|^curvature
/// ```
///
/// The curvature parameter (typically 1.0) controls how quickly power
/// transitions between the two reference points.
///
/// - `higher_power`: power at the higher altitude point (hp)
/// - `higher_alt`: higher altitude (m)
/// - `lower_power`: power at the lower altitude point (hp)
/// - `lower_alt`: lower altitude (m)
/// - `target_alt`: target altitude for interpolation (m)
/// - `curvature`: interpolation curvature (typically 1.0)
///
/// Returns interpolated power at target altitude (hp)
pub fn interpolate_power(
    higher_power: f64,
    higher_alt: f64,
    lower_power: f64,
    lower_alt: f64,
    target_alt: f64,
    curvature: f64,
) -> f64 {
    let p_target = pressure(target_alt);
    let p_lower = pressure(lower_alt);
    let p_higher = pressure(higher_alt);

    let p_denom = p_higher - p_lower;
    if p_denom.abs() < 1e-9 {
        return lower_power;
    }

    // Determine power difference based on altitude direction
    // PORT: Java 三目两臂均 double, 无数值提升
    let power_diff = if target_alt >= lower_alt {
        higher_power - lower_power
    } else {
        lower_power - higher_power
    };

    let ratio = ((p_target - p_lower) / p_denom).abs();
    lower_power + power_diff * ratio.powf(curvature)
}

// ==================== WEP Calculations ====================

/// Calculates the total WEP power multiplier.
///
/// WEP (War Emergency Power) combines several boost mechanisms:
/// - Afterburner boost (higher fuel flow, ADI injection, etc.)
/// - Throttle boost (over-boost manifold pressure)
/// - RPM increase effect on torque curve
/// - Octane rating modifications (fuel quality upgrades)
///
/// - `afterburner_boost`: base afterburner boost factor (FM: AfterburnerBoost)
/// - `throttle_boost`: throttle boost factor (FM: ThrottleBoost, usually 1.0)
/// - `afterburner_boost_mul`: stage-specific boost multiplier (FM: AfterburnerBoostMul)
/// - `octane_afterburner_mult`: octane rating correction (fuel upgrade effect)
/// - `military_rpm`: military power RPM
/// - `wep_rpm`: WEP mode RPM
///
/// Returns total WEP power multiplier
pub fn wep_power_multiplier(
    afterburner_boost: f64,
    throttle_boost: f64,
    afterburner_boost_mul: f64,
    octane_afterburner_mult: f64,
    military_rpm: f64,
    wep_rpm: f64,
) -> f64 {
    // Boost effect modified by octane rating
    let boost_effect = 1.0 + (afterburner_boost - 1.0) * octane_afterburner_mult;

    // RPM effect on torque/power
    let rpm_boost = torque_rpm_boost(military_rpm, wep_rpm);

    boost_effect * throttle_boost * afterburner_boost_mul * rpm_boost
}

/// Calculates the WEP critical altitude.
///
/// WEP mode typically has a higher manifold pressure than military power.
/// This higher pressure can only be maintained up to a lower altitude
/// (the WEP critical altitude). Above this, WEP power drops but may still
/// exceed military power due to the higher base multiplier.
///
/// The calculation determines where the supercharger can no longer
/// maintain the WEP manifold pressure based on:
/// - Military critical altitude and manifold pressure
/// - WEP manifold pressure requirement
/// - RPM effect on supercharger efficiency
/// - Afterburner pressure boost factor
///
/// - `military_crit_alt`: military mode critical altitude (m)
/// - `military_mp`: military mode manifold pressure (ata)
/// - `wep_mp`: WEP mode manifold pressure (ata)
/// - `supercharger_rpm_effect`: supercharger RPM efficiency multiplier
/// - `afterburner_pressure_boost`: afterburner pressure boost (FM: AfterburnerPressureBoost)
///
/// Returns WEP critical altitude (m)
pub fn wep_critical_altitude(
    military_crit_alt: f64,
    military_mp: f64,
    wep_mp: f64,
    supercharger_rpm_effect: f64,
    afterburner_pressure_boost: f64,
) -> f64 {
    // Calculate supercharger "strength" at military critical altitude
    let crit_pressure = pressure(military_crit_alt);
    let supercharger_strength = military_mp / crit_pressure;

    // WEP mode supercharger strength is boosted
    // 注: power_extractor::wep_supercharger_strength 是本公式的 fmdata 直取版
    // (RPM 效率/级压力增益由其内部回退推导); 本函数两系数是公开参数
    // (Java 原签名如此, 调用方自算), 形态不同不并轨, 仅共享乘法骨架。
    let wep_supercharger_strength =
        supercharger_strength * supercharger_rpm_effect * afterburner_pressure_boost;

    // Find altitude where ambient pressure matches WEP requirement
    let wep_crit_pressure = wep_mp / wep_supercharger_strength;
    altitude_at_pressure(wep_crit_pressure)
}

// ==================== Main Power Calculation (WAPC variabler port) ====================

/// Calculates engine power at altitude using the advanced WAPC variabler algorithm.
///
/// This is the high-fidelity version that handles:
/// - ConstRPM regions (variable-speed supercharger bends)
/// - Ceiling parameters for above-critical-altitude decay
/// - ExactAltitudes flag for old-format FM files
/// - Recursive power calculation for WEP critical altitude
///
/// - `params`: supercharger stage parameters (with advanced fields populated)
/// - `altitude_m`: target altitude (m)
/// - `is_wep`: true for WEP mode
/// - `speed_kmh`: aircraft speed for RAM effect (km/h), 0 to ignore
/// - `is_ias`: true if speed is IAS
/// - `sea_level_temp_c`: sea level temperature (°C)
///
/// Returns engine power at altitude (hp)
pub fn power_at_altitude_advanced(
    params: &CompressorStageParams,
    altitude_m: f64,
    is_wep: bool,
    speed_kmh: f64,
    is_ias: bool,
    sea_level_temp_c: f64,
) -> f64 {
    let mut effective_alt = altitude_m;
    if speed_kmh > 0.0 && params.speed_manifold_mult > 0.0 {
        effective_alt = ram_effect_altitude(
            altitude_m,
            sea_level_temp_c,
            speed_kmh,
            is_ias,
            params.speed_manifold_mult,
        );
    }

    // PORT: Java 三目 `isWep ? params.wepPowerMult : 1.0` 两臂均 double
    let wep_mult = if is_wep { params.wep_power_mult } else { 1.0 };
    let bounds = variabler(params, effective_alt, is_wep, wep_mult);

    interpolate_power(
        bounds.higher_power,
        bounds.higher_alt,
        bounds.lower_power,
        bounds.lower_alt,
        effective_alt,
        bounds.curvature,
    )
}

/// Calculates optimal power from multiple stages using the advanced algorithm.
///
/// - `stages`: array of supercharger stage parameters
/// - `altitude_m`: target altitude (m)
/// - `is_wep`: true for WEP mode
/// - `speed_kmh`: aircraft speed for RAM effect (km/h)
/// - `is_ias`: true if speed is IAS
/// - `sea_level_temp_c`: sea level temperature (°C)
///
/// Returns maximum available power from any stage (hp)
// null 入参在切片模型下对应空切片, 同走提前返回 (0)
pub fn optimal_power_advanced(
    stages: &[CompressorStageParams],
    altitude_m: f64,
    is_wep: bool,
    speed_kmh: f64,
    is_ias: bool,
    sea_level_temp_c: f64,
) -> f64 {
    if stages.is_empty() {
        return 0.0;
    }
    let mut max_power = 0.0f64;
    for stage in stages {
        let power = power_at_altitude_advanced(
            stage,
            altitude_m,
            is_wep,
            speed_kmh,
            is_ias,
            sea_level_temp_c,
        );
        if power > max_power {
            max_power = power;
        }
    }
    max_power
}

/// Finds the supercharger stage index that provides maximum power at the given altitude.
///
/// This is used for supercharger gear switching notifications. Multi-stage
/// superchargers have different gear ratios optimized for different altitude bands.
/// This method identifies which stage should be active for best performance.
///
/// - `stages`: array of supercharger stage parameters
/// - `altitude_m`: target altitude (m)
/// - `is_wep`: true for WEP mode
/// - `speed_kmh`: aircraft speed for RAM effect (km/h)
/// - `is_ias`: true if speed is IAS
/// - `sea_level_temp_c`: sea level temperature (°C)
///
/// Returns optimal stage index (0-based), or 0 if single stage or invalid data
// null 检查在切片模型下退化为长度判断 (同上)
pub fn find_optimal_stage_index(
    stages: &[CompressorStageParams],
    altitude_m: f64,
    is_wep: bool,
    speed_kmh: f64,
    is_ias: bool,
    sea_level_temp_c: f64,
) -> usize {
    if stages.len() <= 1 {
        return 0;
    }

    let mut max_power = 0.0f64;
    let mut optimal_index = 0usize;
    for (i, stage) in stages.iter().enumerate() {
        let power = power_at_altitude_advanced(
            stage,
            altitude_m,
            is_wep,
            speed_kmh,
            is_ias,
            sea_level_temp_c,
        );
        if power > max_power {
            max_power = power;
            optimal_index = i;
        }
    }
    optimal_index
}

/// Generates a power curve using the advanced algorithm (0m to 10000m).
///
/// - `stages`: supercharger stage parameters
/// - `is_wep`: true for WEP mode
/// - `speed_kmh`: aircraft speed for RAM effect (0 for static)
/// - `is_ias`: true if speed is IAS
/// - `sea_level_temp_c`: sea level temperature (°C)
/// - `alt_step`: altitude step in meters (recommend 50)
///
/// Returns power array where index i corresponds to altitude (i × altStep)
pub fn generate_power_curve_advanced(
    stages: &[CompressorStageParams],
    is_wep: bool,
    speed_kmh: f64,
    is_ias: bool,
    sea_level_temp_c: f64,
    alt_step: i32,
) -> Vec<f64> {
    let min_alt = 0i32;
    let max_alt = 10000i32;
    let count = ((max_alt - min_alt) / alt_step + 1) as usize;
    let mut curve = vec![0.0f64; count];
    for (i, slot) in curve.iter_mut().enumerate() {
        let alt = (min_alt + i as i32 * alt_step) as f64;
        *slot = optimal_power_advanced(stages, alt, is_wep, speed_kmh, is_ias, sea_level_temp_c);
    }
    curve
}

// ==================== Peak Power Calculation ====================

/// Calculates peak WEP power by traversing altitude × speed combinations.
///
/// Traverses the power surface from 0m to 10000m altitude and 0-800 km/h IAS
/// to find the maximum value, accounting for RAM effect at high speeds.
///
/// This is useful for performance calculations (energy, climb rate, etc.)
/// where a single peak power value is needed regardless of flight conditions.
///
/// Search Grid:
/// - Altitude: 0-10000m, step 100m (101 points)
/// - Speed: 0-800 km/h IAS, step 50 km/h (17 points)
/// - Total iterations: 1717
///
/// - `stages`: compressor stage parameters array
///
/// Returns peak WEP power (hp)
pub fn peak_wep_power(stages: &[CompressorStageParams]) -> f64 {
    // 空切片同走提前返回 (0)
    if stages.is_empty() {
        return 0.0;
    }

    // Traverse altitude × speed to find peak
    let mut peak = 0.0f64;
    for alt in (0..=10000i32).step_by(100) {
        for speed in (0..=800i32).step_by(50) {
            let power = optimal_power_advanced(stages, alt as f64, true, speed as f64, true, 15.0);
            if power > peak {
                peak = power;
            }
        }
    }
    peak
}

/// variabler 的插值边界产出 (五元组具名形态, 消魔法下标)。
#[derive(Debug, Clone, Copy)]
struct InterpBounds {
    /// 高参考点功率 (hp)
    higher_power: f64,
    /// 高参考点高度 (m)
    higher_alt: f64,
    /// 低参考点功率 (hp)
    lower_power: f64,
    /// 低参考点高度 (m)
    lower_alt: f64,
    /// 插值曲率指数 (典型 1.0)
    curvature: f64,
}

/// ceil_scaled_alt — 天花板高度按 (参考高度与临界高度的气压比) 缩放。
/// variabler 军用/WEP 两分支共 10 处同构计算收敛于此; 军用分支参考高度即
/// crit_alt (Java 源字面保留 `pressure(crit)/pressure(crit)` 自比, 逐运算符
/// 同序, 位级等价), WEP 分支参考 wep_crit_alt。
fn ceil_scaled_alt(p: &CompressorStageParams, ref_alt: f64) -> f64 {
    altitude_at_pressure(pressure(p.ceiling_alt) * (pressure(ref_alt) / pressure(p.crit_alt)))
}

/// WAPC variabler() port — determines interpolation bounds for a given altitude.
///
/// This is the core logic that determines the shape of the power curve by
/// selecting the correct pair of (altitude, power) reference points for
/// interpolation, based on the relationship between the target altitude and
/// the various FM-defined altitudes (critical, constRPM, ceiling, old/adjusted).
///
/// 波14 拆解: 军用/WEP 两大对称分支提取为 [`variabler_military`]/
/// [`variabler_wep`], 语句序零变化。
///
/// - `p`: stage parameters
/// - `alt_ram`: effective altitude after RAM effect (m)
/// - `is_wep`: true for WEP mode
/// - `wep_mult`: WEP power multiplier (1.0 for military)
///
/// Returns [`InterpBounds`]
fn variabler(p: &CompressorStageParams, alt_ram: f64, is_wep: bool, wep_mult: f64) -> InterpBounds {
    if !is_wep {
        variabler_military(p, alt_ram)
    } else {
        variabler_wep(p, alt_ram, wep_mult)
    }
}

/// variabler 军用功率 (military) 分支 — 临界高度以下 / 调整临界~原始临界 /
/// 原始临界以上三段选点 (WAPC variabler 的 !isWep 半区)。
// let 无初值: 各路径恰好单次赋值 (无 WEP 的 swap 段), 除 curvature 外无需 mut
fn variabler_military(p: &CompressorStageParams, alt_ram: f64) -> InterpBounds {
    let higher_power: f64;
    let higher_alt: f64;
    let lower_power: f64;
    let lower_alt: f64;
    let mut curvature: f64 = 1.0;

    {
        // ======================== MILITARY MODE ========================
        if alt_ram <= p.crit_alt {
            // --- Below or at critical altitude ---
            if has_const_rpm(p) && const_rpm_below_deck(p) && alt_ram < p.const_rpm_alt {
                // ConstRPM is below deck — zero power zone
                higher_alt = p.const_rpm_alt;
                higher_power = 0.0;
                lower_alt = p.const_rpm_alt - 10.0;
                lower_power = 0.0;
            } else if !const_rpm_below_crit_alt(p) && !power_is_deck_power(p) {
                // Normal case: interpolate between deck and crit alt
                higher_alt = p.crit_alt;
                higher_power = p.crit_power;
                lower_alt = p.deck_alt;
                lower_power = p.deck_power;
            } else if const_rpm_below_crit_alt(p) && alt_ram < p.const_rpm_alt {
                // Below constRPM bend: deck → constRPM
                higher_alt = p.const_rpm_alt;
                higher_power = p.const_rpm_power;
                lower_alt = p.deck_alt;
                lower_power = p.deck_power;
            } else if const_rpm_below_crit_alt(p) && alt_ram >= p.const_rpm_alt {
                // Above constRPM bend: constRPM → crit (with curvature)
                curvature = p.curvature;
                higher_alt = p.crit_alt;
                higher_power = p.crit_power;
                lower_alt = p.const_rpm_alt;
                lower_power = p.const_rpm_power;
            } else {
                // powerIsDeckPower: crit alt == deck alt, use ceiling
                higher_alt = p.ceiling_alt;
                higher_power = p.ceiling_power;
                lower_alt = p.crit_alt;
                lower_power = p.crit_power;
            }
        } else if alt_ram <= p.old_altitude {
            // --- Between adjusted crit alt and original crit alt ---
            lower_alt = p.crit_alt;
            lower_power = p.crit_power;

            if !ceiling_is_useful(p) {
                // No useful ceiling: pressure decay from crit
                higher_alt = p.old_altitude;
                higher_power = interpolate_power(
                    p.old_power_new_rpm,
                    p.crit_alt,
                    p.deck_power,
                    p.deck_alt,
                    p.crit_alt,
                    curvature,
                ) * (pressure(p.old_altitude) / pressure(p.crit_alt));
            } else if !const_rpm_above_crit_alt(p) {
                // Ceiling useful, no constRPM above crit
                if p.exact_altitudes {
                    higher_alt = p.old_altitude;
                    let ceil_scaled = ceil_scaled_alt(p, p.crit_alt);
                    higher_power = interpolate_power(
                        p.ceiling_power,
                        ceil_scaled,
                        p.old_power_new_rpm,
                        p.crit_alt,
                        p.old_altitude,
                        curvature,
                    );
                } else {
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                }
            } else {
                // Ceiling useful + constRPM above crit (P-63 style)
                curvature = p.curvature;
                if p.exact_altitudes {
                    higher_alt = p.old_altitude;
                    let ceil_scaled = ceil_scaled_alt(p, p.crit_alt);
                    higher_power = interpolate_power(
                        p.ceiling_power,
                        ceil_scaled,
                        p.old_power_new_rpm,
                        p.crit_alt,
                        p.old_altitude,
                        curvature,
                    );
                } else {
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                }
            }
        } else {
            // --- Above original critical altitude ---
            if !ceiling_is_useful(p) {
                // Pressure decay above old altitude
                lower_alt = p.old_altitude;
                lower_power = interpolate_power(
                    p.old_power_new_rpm,
                    p.crit_alt,
                    p.deck_power,
                    p.deck_alt,
                    p.crit_alt,
                    curvature,
                ) * (pressure(p.old_altitude) / pressure(p.crit_alt));
                higher_alt = alt_ram;
                higher_power = lower_power * (pressure(alt_ram) / pressure(lower_alt));
            } else if !const_rpm_above_crit_alt(p) {
                if p.exact_altitudes {
                    lower_alt = p.old_altitude;
                    let ceil_scaled = ceil_scaled_alt(p, p.crit_alt);
                    lower_power = interpolate_power(
                        p.ceiling_power,
                        ceil_scaled,
                        p.old_power_new_rpm,
                        p.crit_alt,
                        p.old_altitude,
                        curvature,
                    );
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                } else {
                    lower_alt = p.crit_alt;
                    lower_power = p.crit_power;
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                }
            } else {
                // constRPM above crit with ceiling
                curvature = p.curvature;
                if p.exact_altitudes {
                    let ceil_scaled = ceil_scaled_alt(p, p.crit_alt);
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                    lower_alt = p.old_altitude;
                    lower_power = interpolate_power(
                        p.ceiling_power,
                        ceil_scaled,
                        p.old_power_new_rpm,
                        p.crit_alt,
                        p.old_altitude,
                        curvature,
                    );
                } else {
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                    lower_alt = p.old_altitude;
                    lower_power = p.crit_power;
                }
            }
        }
    }

    InterpBounds {
        higher_power,
        higher_alt,
        lower_power,
        lower_alt,
        curvature,
    }
}

/// variabler WEP 分支 — WEP 临界高度/原始高度的四段选点 + 上下界倒置安全交换
/// (WAPC variabler 的 isWep 半区)。
fn variabler_wep(p: &CompressorStageParams, alt_ram: f64, wep_mult: f64) -> InterpBounds {
    let mut higher_power: f64;
    let mut higher_alt: f64;
    let mut lower_power: f64;
    let mut lower_alt: f64;
    let mut curvature: f64 = 1.0;

    {
        // ======================== WEP MODE ========================
        let wep_crit_alt = p.wep_crit_alt;

        if alt_ram <= wep_crit_alt && alt_ram <= p.old_altitude {
            // --- Below both WEP crit alt and old altitude ---
            if has_const_rpm(p) && const_rpm_below_deck(p) && alt_ram < p.const_rpm_alt {
                // ConstRPM below deck — zero power
                higher_alt = p.const_rpm_alt;
                higher_power = 0.0;
                lower_alt = p.const_rpm_alt - 10.0;
                lower_power = 0.0;
            } else if !const_rpm_below_crit_alt(p) && !power_is_deck_power(p) {
                // Normal WEP below crit alt
                if p.exact_altitudes {
                    // Recursive: compute WEP power at wepCritAlt from military curve × wepMult
                    higher_alt = wep_crit_alt;
                    higher_power = interpolate_power(
                        p.crit_power * wep_mult,
                        p.crit_alt,
                        p.deck_power * wep_mult,
                        p.deck_alt,
                        higher_alt,
                        curvature,
                    );
                    lower_alt = p.wep_deck_alt;
                    lower_power = interpolate_power(
                        p.crit_power * wep_mult,
                        p.crit_alt,
                        p.deck_power * wep_mult,
                        p.deck_alt,
                        lower_alt,
                        curvature,
                    );
                } else {
                    higher_alt = wep_crit_alt;
                    higher_power = p.crit_power * wep_mult;
                    lower_alt = p.stage0_deck_alt; // WAPC: Deck_Altitude{0}
                    lower_power = p.deck_power * wep_mult;
                }
            } else if p.exact_altitudes && has_const_rpm(p) && alt_ram < p.const_rpm_alt {
                // ExactAltitudes + constRPM below crit: deck → constRPM
                higher_power = p.const_rpm_power * wep_mult;
                lower_alt = p.deck_alt;
                lower_power = p.deck_power * wep_mult;
                higher_alt = p.const_rpm_alt; // Doesn't change with WEP
            } else if !p.exact_altitudes && has_const_rpm(p) && alt_ram < p.wep_const_rpm_alt {
                // Non-ExactAltitudes + constRPM: deck → WEP constRPM alt
                higher_power = p.const_rpm_power * wep_mult;
                lower_alt = p.deck_alt;
                lower_power = p.deck_power * wep_mult;
                higher_alt = p.wep_const_rpm_alt;
            } else if p.exact_altitudes && has_const_rpm(p) && alt_ram >= p.const_rpm_alt {
                // ExactAltitudes + above constRPM: constRPM → wep crit (with curvature + recursion)
                curvature = p.curvature;
                higher_alt = wep_crit_alt;
                lower_power = p.const_rpm_power * wep_mult;
                lower_alt = p.const_rpm_alt;
                higher_power = interpolate_power(
                    p.crit_power * wep_mult,
                    p.crit_alt,
                    p.const_rpm_power * wep_mult,
                    p.const_rpm_alt,
                    higher_alt,
                    curvature,
                );
            } else if !p.exact_altitudes && has_const_rpm(p) && alt_ram >= p.wep_const_rpm_alt {
                // Non-ExactAltitudes + above WEP constRPM
                curvature = p.curvature;
                higher_alt = wep_crit_alt;
                lower_power = p.const_rpm_power * wep_mult;
                lower_alt = p.wep_const_rpm_alt;
                higher_power = p.crit_power * wep_mult;
            } else if power_is_deck_power(p) {
                // Power == deck power case
                if p.exact_altitudes {
                    higher_alt = p.ceiling_alt;
                    let ceil_scaled = ceil_scaled_alt(p, wep_crit_alt);
                    higher_power = interpolate_power(
                        p.ceiling_power * wep_mult,
                        ceil_scaled,
                        p.crit_power * wep_mult,
                        wep_crit_alt,
                        p.ceiling_alt,
                        curvature,
                    );
                    lower_alt = wep_crit_alt;
                    lower_power = p.crit_power * wep_mult;
                } else {
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                    lower_alt = wep_crit_alt;
                    lower_power = p.crit_power * wep_mult;
                }
            } else {
                // Fallback: simple WEP curve
                higher_alt = wep_crit_alt;
                higher_power = p.crit_power * wep_mult;
                lower_alt = p.deck_alt;
                lower_power = p.deck_power * wep_mult;
            }
        } else if p.old_altitude < alt_ram && alt_ram <= wep_crit_alt {
            // --- WEP crit alt higher than old mil altitude (rare: Fw-190A-1) ---
            // Power constant between old altitude and WEP crit alt
            higher_alt = wep_crit_alt;
            higher_power = interpolate_power(
                p.crit_power * wep_mult,
                p.crit_alt,
                p.deck_power * wep_mult,
                p.deck_alt,
                p.old_altitude,
                curvature,
            );
            lower_alt = p.old_altitude;
            lower_power = higher_power;
        } else if (java_round(wep_crit_alt) as f64) < alt_ram
            && alt_ram <= (java_round(p.old_altitude) as f64)
        {
            // PORT: Java Math.round(double)=floor(x+0.5) (§2.3), 返回 long 后与 double
            // 比较时提升回 double — 此处 as f64 复刻该提升
            // --- Above WEP crit alt but below old mil altitude ---
            // Determine lower bound power at WEP crit alt
            if !const_rpm_below_wep_crit_alt(p) {
                lower_alt = wep_crit_alt;
                if p.exact_altitudes {
                    lower_power = interpolate_power(
                        p.crit_power * wep_mult,
                        p.crit_alt,
                        p.deck_power * wep_mult,
                        p.deck_alt,
                        lower_alt,
                        curvature,
                    );
                } else {
                    lower_power = p.crit_power * wep_mult;
                }
            } else {
                lower_alt = wep_crit_alt;
                if p.exact_altitudes {
                    lower_power = interpolate_power(
                        p.crit_power * wep_mult,
                        p.crit_alt,
                        p.const_rpm_power * wep_mult,
                        p.const_rpm_alt,
                        lower_alt,
                        p.curvature,
                    );
                } else {
                    lower_power = p.crit_power * wep_mult;
                }
            }

            // Determine upper bound
            if !ceiling_is_useful(p) {
                higher_alt = p.old_altitude;
                higher_power = interpolate_power(
                    p.crit_power * wep_mult,
                    p.crit_alt,
                    p.deck_power * wep_mult,
                    p.deck_alt,
                    higher_alt,
                    curvature,
                ) * (pressure(p.old_altitude) / pressure(lower_alt));
            } else if !const_rpm_above_crit_alt(p) {
                if p.exact_altitudes {
                    higher_alt = p.old_altitude;
                    let ceil_scaled = ceil_scaled_alt(p, wep_crit_alt);
                    higher_power = interpolate_power(
                        p.ceiling_power * wep_mult,
                        ceil_scaled,
                        p.old_power_new_rpm * wep_mult,
                        wep_crit_alt,
                        p.old_altitude,
                        curvature,
                    );
                } else {
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                }
            } else {
                // constRPM above crit + ceiling
                curvature = p.curvature;
                if p.exact_altitudes {
                    higher_alt = p.old_altitude;
                    let ceil_scaled = ceil_scaled_alt(p, wep_crit_alt);
                    higher_power = interpolate_power(
                        p.ceiling_power * wep_mult,
                        ceil_scaled,
                        p.old_power_new_rpm * wep_mult,
                        wep_crit_alt,
                        p.old_altitude,
                        curvature,
                    );
                } else {
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                }
            }
        } else {
            // --- Above both WEP crit alt and old altitude ---
            if wep_crit_alt < p.crit_alt {
                // WEP crit alt below military crit alt
                lower_alt = p.old_altitude;
                if !ceiling_is_useful(p) {
                    lower_power = interpolate_power(
                        p.crit_power * wep_mult,
                        p.crit_alt,
                        p.deck_power * wep_mult,
                        p.deck_alt,
                        lower_alt,
                        curvature,
                    ) * (pressure(p.old_altitude) / pressure(wep_crit_alt));
                } else {
                    if p.exact_altitudes {
                        let ceil_scaled = ceil_scaled_alt(p, wep_crit_alt);
                        lower_power = interpolate_power(
                            p.ceiling_power * wep_mult,
                            ceil_scaled,
                            p.old_power_new_rpm * wep_mult,
                            wep_crit_alt,
                            lower_alt,
                            curvature,
                        );
                    } else {
                        lower_alt = wep_crit_alt;
                        lower_power = p.crit_power * wep_mult;
                    }
                }
            } else if !const_rpm_below_crit_alt(p) {
                lower_alt = wep_crit_alt;
                if p.exact_altitudes {
                    lower_power = interpolate_power(
                        p.crit_power * wep_mult,
                        p.crit_alt,
                        p.deck_power * wep_mult,
                        p.deck_alt,
                        p.old_altitude,
                        curvature,
                    );
                } else {
                    lower_power = p.crit_power * wep_mult;
                }
            } else {
                // constRPM below crit alt
                lower_alt = wep_crit_alt;
                lower_power = interpolate_power(
                    p.crit_power * wep_mult,
                    p.crit_alt,
                    p.const_rpm_power * wep_mult,
                    p.const_rpm_alt,
                    lower_alt,
                    curvature,
                );
            }

            // Upper bound for above-everything case
            if !ceiling_is_useful(p) {
                higher_alt = alt_ram;
                higher_power = lower_power * (pressure(alt_ram) / pressure(lower_alt));
            } else if !const_rpm_above_crit_alt(p) {
                if p.exact_altitudes {
                    higher_alt = ceil_scaled_alt(p, wep_crit_alt);
                    higher_power = p.ceiling_power * wep_mult;
                } else {
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                }
            } else {
                curvature = p.curvature;
                if p.exact_altitudes {
                    higher_alt = ceil_scaled_alt(p, wep_crit_alt);
                    higher_power = p.ceiling_power;
                } else {
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                }
            }

            // Safety: swap if higher < lower and powers are inverted
            if higher_alt < lower_alt && higher_power > lower_power {
                let tmp_alt = lower_alt;
                let tmp_pwr = lower_power;
                lower_alt = higher_alt;
                lower_power = higher_power;
                higher_alt = tmp_alt;
                higher_power = tmp_pwr;
            }
        }
    }

    InterpBounds {
        higher_power,
        higher_alt,
        lower_power,
        lower_alt,
        curvature,
    }
}

// ==================== Parameter Data Class ====================

/// Parameters for a single supercharger stage.
///
/// These values are extracted from War Thunder FM (Flight Model) files.
/// Each supercharger stage has its own critical altitude and power curve.
///
/// Key Concepts
/// ============
///
/// Critical Altitude: The altitude up to which the supercharger can maintain
/// rated manifold pressure. Power is relatively constant below this point.
///
/// Deck Power: Power output at sea level (or "deck altitude"). May be slightly
/// less than critical altitude power due to exhaust back-pressure.
///
/// WEP Critical Altitude: Usually lower than military critical altitude
/// because WEP demands higher manifold pressure that the supercharger
/// cannot maintain as high.
// 公开可变字段, 不造 getter (PORTING §0.7)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressorStageParams {
    /// Critical altitude in meters - altitude where power starts dropping
    pub crit_alt: f64,

    /// Power at critical altitude in horsepower
    pub crit_power: f64,

    /// Sea level (deck) power in horsepower
    pub deck_power: f64,

    /// Deck altitude in meters (usually 0)
    pub deck_alt: f64,

    /// Power curve curvature coefficient (typically 1.0)
    pub curvature: f64,

    /// WEP critical altitude in meters
    pub wep_crit_alt: f64,

    /// WEP power multiplier relative to military power
    pub wep_power_mult: f64,

    /// RAM effect coefficient (FM: SpeedManifoldMultiplier)
    pub speed_manifold_mult: f64,

    /// Stage index (0 = first stage, 1 = second stage, etc.)
    pub stage_index: i32,

    // === Advanced fields for WAPC-compatible variabler() ===
    /// ConstRPM altitude - altitude where variable-speed supercharger bends the curve (m)
    pub const_rpm_alt: f64,

    /// ConstRPM power - power at the ConstRPM bend point (hp)
    pub const_rpm_power: f64,

    /// Ceiling altitude - service ceiling for this stage (m)
    pub ceiling_alt: f64,

    /// Power at ceiling altitude (hp)
    pub ceiling_power: f64,

    /// Original (pre-adjustment) critical altitude, before definition_alt_power_adjuster (m)
    pub old_altitude: f64,

    /// Original (pre-adjustment) critical power (hp)
    pub old_power: f64,

    /// Pre-adjustment power scaled to military RPM (hp)
    pub old_power_new_rpm: f64,

    /// WEP deck altitude (m)
    pub wep_deck_alt: f64,

    /// WEP ConstRPM altitude (m), for non-ExactAltitudes FMs
    pub wep_const_rpm_alt: f64,

    /// Stage 0 deck altitude (m), used for WEP non-ExactAltitudes mode
    pub stage0_deck_alt: f64,

    /// True if this is an old-format FM (no CompressorOmegaFactorSq)
    pub exact_altitudes: bool,
}

// Default 非全零: curvature/wep_power_mult/speed_manifold_mult 默认 1.0, 其余 0/false (§2.10)
impl Default for CompressorStageParams {
    fn default() -> Self {
        Self {
            crit_alt: 0.0,
            crit_power: 0.0,
            deck_power: 0.0,
            deck_alt: 0.0,
            curvature: 1.0,
            wep_crit_alt: 0.0,
            wep_power_mult: 1.0,
            speed_manifold_mult: 1.0,
            stage_index: 0,
            const_rpm_alt: 0.0,
            const_rpm_power: 0.0,
            ceiling_alt: 0.0,
            ceiling_power: 0.0,
            old_altitude: 0.0,
            old_power: 0.0,
            old_power_new_rpm: 0.0,
            wep_deck_alt: 0.0,
            wep_const_rpm_alt: 0.0,
            stage0_deck_alt: 0.0,
            exact_altitudes: false,
        }
    }
}

impl CompressorStageParams {
    /// Creates a parameter set with basic values.
    ///
    /// - `crit_alt`: critical altitude (m)
    /// - `crit_power`: power at critical altitude (hp)
    /// - `deck_power`: sea level power (hp)
    pub fn new(crit_alt: f64, crit_power: f64, deck_power: f64) -> Self {
        Self {
            crit_alt,
            crit_power,
            deck_power,
            wep_crit_alt: crit_alt,
            ..Default::default()
        }
    }

    /// Creates a complete parameter set.
    ///
    /// - `crit_alt`: critical altitude (m)
    /// - `crit_power`: power at critical altitude (hp)
    /// - `deck_power`: sea level power (hp)
    /// - `wep_crit_alt`: WEP critical altitude (m)
    /// - `wep_power_mult`: WEP power multiplier
    /// - `speed_manifold_mult`: RAM effect coefficient
    /// - `stage_index`: supercharger stage index
    pub fn new_full(
        crit_alt: f64,
        crit_power: f64,
        deck_power: f64,
        wep_crit_alt: f64,
        wep_power_mult: f64,
        speed_manifold_mult: f64,
        stage_index: i32,
    ) -> Self {
        Self {
            crit_alt,
            crit_power,
            deck_power,
            wep_crit_alt,
            wep_power_mult,
            speed_manifold_mult,
            stage_index,
            ..Default::default()
        }
    }
}

// Display 取整语义: Java String.format %.0f 用 HALF_UP, Rust {:.0} 在 .5 精确界处
// 取整规则不同 (half-even); 非有限值输出亦不同 — 仅展示用途, 非平界值行为等价
impl std::fmt::Display for CompressorStageParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stage{}[critAlt={:.0}m, critPower={:.0}hp, deckPower={:.0}hp, wepMult={:.2}]",
            self.stage_index, self.crit_alt, self.crit_power, self.deck_power, self.wep_power_mult
        )
    }
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests;
