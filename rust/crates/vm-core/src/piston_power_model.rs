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
//!
//! 对应 Java: `src/prog/util/PistonPowerModel.java` (一比一翻译)

// PORT: Java `private PistonPowerModel() {}` (final 工具类, 私有构造器防实例化)
// → Rust 自由函数模块无实例化概念, 天然满足

use crate::atmosphere_model::{altitude_at_pressure, pressure, ram_effect_altitude};

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

    let higher_power = bounds[0];
    let higher_alt = bounds[1];
    let lower_power = bounds[2];
    let lower_alt = bounds[3];
    let curvature = bounds[4];

    interpolate_power(higher_power, higher_alt, lower_power, lower_alt, effective_alt, curvature)
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
// PORT: Java `CompressorStageParams[]` 只读数组入参 → &[CompressorStageParams];
// `stages == null` 在 Rust 切片模型下无对应, 空切片走同一提前返回路径 (0)
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
        let power =
            power_at_altitude_advanced(stage, altitude_m, is_wep, speed_kmh, is_ias, sea_level_temp_c);
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
// PORT: Java `int` 索引返回值 → usize (非负索引域, 行为等价); Java null 检查同上退化为长度判断
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
        let power =
            power_at_altitude_advanced(stage, altitude_m, is_wep, speed_kmh, is_ias, sea_level_temp_c);
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
    // PORT: Java `stages == null || stages.length == 0` → 空切片同走提前返回 (0)
    if stages.is_empty() {
        return 0.0;
    }

    // Traverse altitude × speed to find peak
    // PORT: Java `for (int alt = 0; alt <= 10000; alt += 100)` int 步进循环
    let mut peak = 0.0f64;
    for alt in (0..=10000i32).step_by(100) {
        for speed in (0..=800i32).step_by(50) {
            let power = optimal_power_advanced(
                stages,
                alt as f64,
                true,
                speed as f64,
                true,
                15.0,
            );
            if power > peak {
                peak = power;
            }
        }
    }
    peak
}

/// WAPC variabler() port — determines interpolation bounds for a given altitude.
///
/// This is the core logic that determines the shape of the power curve by
/// selecting the correct pair of (altitude, power) reference points for
/// interpolation, based on the relationship between the target altitude and
/// the various FM-defined altitudes (critical, constRPM, ceiling, old/adjusted).
///
/// - `p`: stage parameters
/// - `alt_ram`: effective altitude after RAM effect (m)
/// - `is_wep`: true for WEP mode
/// - `wep_mult`: WEP power multiplier (1.0 for military)
///
/// Returns [f64; 5]: {higherPower, higherAlt, lowerPower, lowerAlt, curvature}
// PORT: Java private static → 模块私有函数; 返回 double[5] → [f64; 5]
fn variabler(p: &CompressorStageParams, alt_ram: f64, is_wep: bool, wep_mult: f64) -> [f64; 5] {
    // PORT: Java 声明未初始化的局部变量 (所有分支先赋值后使用) → Rust let mut 无初值,
    // 编译器静态检查所有路径均已赋值, 语义一致
    let mut higher_power: f64;
    let mut higher_alt: f64;
    let mut lower_power: f64;
    let mut lower_alt: f64;
    let mut curvature: f64 = 1.0;

    if !is_wep {
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
                    p.old_power_new_rpm, p.crit_alt,
                    p.deck_power, p.deck_alt, p.crit_alt, curvature,
                )
                    * (pressure(p.old_altitude) / pressure(p.crit_alt));
            } else if !const_rpm_above_crit_alt(p) {
                // Ceiling useful, no constRPM above crit
                if p.exact_altitudes {
                    higher_alt = p.old_altitude;
                    let ceil_scaled_alt = altitude_at_pressure(
                        pressure(p.ceiling_alt) * (pressure(p.crit_alt) / pressure(p.crit_alt)));
                    higher_power = interpolate_power(p.ceiling_power, ceil_scaled_alt,
                        p.old_power_new_rpm, p.crit_alt, p.old_altitude, curvature);
                } else {
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                }
            } else {
                // Ceiling useful + constRPM above crit (P-63 style)
                curvature = p.curvature;
                if p.exact_altitudes {
                    higher_alt = p.old_altitude;
                    let ceil_scaled_alt = altitude_at_pressure(
                        pressure(p.ceiling_alt) * (pressure(p.crit_alt) / pressure(p.crit_alt)));
                    higher_power = interpolate_power(p.ceiling_power, ceil_scaled_alt,
                        p.old_power_new_rpm, p.crit_alt, p.old_altitude, curvature);
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
                    p.old_power_new_rpm, p.crit_alt,
                    p.deck_power, p.deck_alt, p.crit_alt, curvature,
                )
                    * (pressure(p.old_altitude) / pressure(p.crit_alt));
                higher_alt = alt_ram;
                higher_power = lower_power * (pressure(alt_ram) / pressure(lower_alt));
            } else if !const_rpm_above_crit_alt(p) {
                if p.exact_altitudes {
                    lower_alt = p.old_altitude;
                    let ceil_scaled_alt = altitude_at_pressure(
                        pressure(p.ceiling_alt) * (pressure(p.crit_alt) / pressure(p.crit_alt)));
                    lower_power = interpolate_power(p.ceiling_power, ceil_scaled_alt,
                        p.old_power_new_rpm, p.crit_alt, p.old_altitude, curvature);
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
                    let ceil_scaled_alt = altitude_at_pressure(
                        pressure(p.ceiling_alt) * (pressure(p.crit_alt) / pressure(p.crit_alt)));
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                    lower_alt = p.old_altitude;
                    lower_power = interpolate_power(p.ceiling_power, ceil_scaled_alt,
                        p.old_power_new_rpm, p.crit_alt, p.old_altitude, curvature);
                } else {
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                    lower_alt = p.old_altitude;
                    lower_power = p.crit_power;
                }
            }
        }
    } else {
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
                        p.crit_power * wep_mult, p.crit_alt,
                        p.deck_power * wep_mult, p.deck_alt,
                        higher_alt, curvature);
                    lower_alt = p.wep_deck_alt;
                    lower_power = interpolate_power(
                        p.crit_power * wep_mult, p.crit_alt,
                        p.deck_power * wep_mult, p.deck_alt,
                        lower_alt, curvature);
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
                    p.crit_power * wep_mult, p.crit_alt,
                    p.const_rpm_power * wep_mult, p.const_rpm_alt,
                    higher_alt, curvature);
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
                    let ceil_scaled_alt = altitude_at_pressure(
                        pressure(p.ceiling_alt) * (pressure(wep_crit_alt) / pressure(p.crit_alt)));
                    higher_power = interpolate_power(
                        p.ceiling_power * wep_mult, ceil_scaled_alt,
                        p.crit_power * wep_mult, wep_crit_alt,
                        p.ceiling_alt, curvature);
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
                p.crit_power * wep_mult, p.crit_alt,
                p.deck_power * wep_mult, p.deck_alt,
                p.old_altitude, curvature);
            lower_alt = p.old_altitude;
            lower_power = higher_power;
        } else if (java_round(wep_crit_alt) as f64) < alt_ram && alt_ram <= (java_round(p.old_altitude) as f64) {
            // PORT: Java Math.round(double)=floor(x+0.5) (§2.3), 返回 long 后与 double
            // 比较时提升回 double — 此处 as f64 复刻该提升
            // --- Above WEP crit alt but below old mil altitude ---
            // Determine lower bound power at WEP crit alt
            if !const_rpm_below_wep_crit_alt(p) {
                lower_alt = wep_crit_alt;
                if p.exact_altitudes {
                    lower_power = interpolate_power(
                        p.crit_power * wep_mult, p.crit_alt,
                        p.deck_power * wep_mult, p.deck_alt,
                        lower_alt, curvature);
                } else {
                    lower_power = p.crit_power * wep_mult;
                }
            } else {
                lower_alt = wep_crit_alt;
                if p.exact_altitudes {
                    lower_power = interpolate_power(
                        p.crit_power * wep_mult, p.crit_alt,
                        p.const_rpm_power * wep_mult, p.const_rpm_alt,
                        lower_alt, p.curvature);
                } else {
                    lower_power = p.crit_power * wep_mult;
                }
            }

            // Determine upper bound
            if !ceiling_is_useful(p) {
                higher_alt = p.old_altitude;
                higher_power = interpolate_power(
                    p.crit_power * wep_mult, p.crit_alt,
                    p.deck_power * wep_mult, p.deck_alt,
                    higher_alt, curvature)
                    * (pressure(p.old_altitude) / pressure(lower_alt));
            } else if !const_rpm_above_crit_alt(p) {
                if p.exact_altitudes {
                    higher_alt = p.old_altitude;
                    let ceil_scaled_alt = altitude_at_pressure(
                        pressure(p.ceiling_alt) * (pressure(wep_crit_alt) / pressure(p.crit_alt)));
                    higher_power = interpolate_power(
                        p.ceiling_power * wep_mult, ceil_scaled_alt,
                        p.old_power_new_rpm * wep_mult, wep_crit_alt,
                        p.old_altitude, curvature);
                } else {
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                }
            } else {
                // constRPM above crit + ceiling
                curvature = p.curvature;
                if p.exact_altitudes {
                    higher_alt = p.old_altitude;
                    let ceil_scaled_alt = altitude_at_pressure(
                        pressure(p.ceiling_alt) * (pressure(wep_crit_alt) / pressure(p.crit_alt)));
                    higher_power = interpolate_power(
                        p.ceiling_power * wep_mult, ceil_scaled_alt,
                        p.old_power_new_rpm * wep_mult, wep_crit_alt,
                        p.old_altitude, curvature);
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
                        p.crit_power * wep_mult, p.crit_alt,
                        p.deck_power * wep_mult, p.deck_alt,
                        lower_alt, curvature)
                        * (pressure(p.old_altitude) / pressure(wep_crit_alt));
                } else {
                    if p.exact_altitudes {
                        let ceil_scaled_alt = altitude_at_pressure(
                            pressure(p.ceiling_alt) * (pressure(wep_crit_alt) / pressure(p.crit_alt)));
                        lower_power = interpolate_power(
                            p.ceiling_power * wep_mult, ceil_scaled_alt,
                            p.old_power_new_rpm * wep_mult, wep_crit_alt,
                            lower_alt, curvature);
                    } else {
                        lower_alt = wep_crit_alt;
                        lower_power = p.crit_power * wep_mult;
                    }
                }
            } else if !const_rpm_below_crit_alt(p) {
                lower_alt = wep_crit_alt;
                if p.exact_altitudes {
                    lower_power = interpolate_power(
                        p.crit_power * wep_mult, p.crit_alt,
                        p.deck_power * wep_mult, p.deck_alt,
                        p.old_altitude, curvature);
                } else {
                    lower_power = p.crit_power * wep_mult;
                }
            } else {
                // constRPM below crit alt
                lower_alt = wep_crit_alt;
                lower_power = interpolate_power(
                    p.crit_power * wep_mult, p.crit_alt,
                    p.const_rpm_power * wep_mult, p.const_rpm_alt,
                    lower_alt, curvature);
            }

            // Upper bound for above-everything case
            if !ceiling_is_useful(p) {
                higher_alt = alt_ram;
                higher_power = lower_power * (pressure(alt_ram) / pressure(lower_alt));
            } else if !const_rpm_above_crit_alt(p) {
                if p.exact_altitudes {
                    higher_alt = altitude_at_pressure(
                        pressure(p.ceiling_alt) * (pressure(wep_crit_alt) / pressure(p.crit_alt)));
                    higher_power = p.ceiling_power * wep_mult;
                } else {
                    higher_alt = p.ceiling_alt;
                    higher_power = p.ceiling_power;
                }
            } else {
                curvature = p.curvature;
                if p.exact_altitudes {
                    higher_alt = altitude_at_pressure(
                        pressure(p.ceiling_alt) * (pressure(wep_crit_alt) / pressure(p.crit_alt)));
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

    // TODO(port): Logger 未译 (B 类, CLASSIFY 裁决 → tracing/log); 原调用为 DEBUG 级
    // 调试日志, 不影响计算结果:
    // Logger.debug("variabler", String.format("altRam=%.1f, isWep=%s | lower=(%.1f, %.1f), higher=(%.1f, %.1f), curv=%.1f",
    //         altRam, isWep, lowerAlt, lowerPower, higherAlt, higherPower, curvature));
    [higher_power, higher_alt, lower_power, lower_alt, curvature]
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
// PORT: Java public 可变字段内部类 → pub struct + pub 字段 (§0.7, 不造 getter)
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

// PORT: Java 字段显式初始化器 (deckAlt=0, curvature=1.0, wepPowerMult=1.0,
// speedManifoldMult=1.0; 其余 int=0/double=0.0/boolean=false 隐式初始化, §2.10)
// → 手写 Default 覆盖派生默认值
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
    // PORT: Java 无参构造器 `new CompressorStageParams()` → `CompressorStageParams::default()`
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

// PORT: Java toString() 覆写 → Display trait (使 .to_string() 可用)。
// 取整语义差异: Java String.format %.0f 用 HALF_UP, Rust {:.0} 在 .5 精确界处
// 取整规则不同 (half-even); 非有限值输出亦不同 (Rust "inf" vs Java "Infinity") —
// 仅展示用途, 测试断言值均非 .5 界, 非平界值行为等价
impl std::fmt::Display for CompressorStageParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stage{}[critAlt={:.0}m, critPower={:.0}hp, deckPower={:.0}hp, wepMult={:.2}]",
            self.stage_index, self.crit_alt, self.crit_power, self.deck_power, self.wep_power_mult
        )
    }
}

// ==================== Java Math.round 复刻 (§2.3) ====================

/// Java `Math.round(double)` = `floor(x + 0.5)`
// PORT: Rust f64::round 是半偶舍入, 不可用; 与 format.rs 的 java_round 同源实现
fn java_round(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

// ==================== PowerCurveHelper 静态导入内联 ====================
// PORT: Java 源文件 `import static prog.util.PowerCurveHelper.*;` — 该工具类在
// 流水线后续波次翻译 (rust 侧 power_curve_helper.rs 当前为占位), 其被本文件用到的
// 8 个函数按 PowerCurveHelper.java 逐字内联为私有函数 (7 个被直接调用 +
// hasCeiling 作为 ceilingIsUseful 的内部依赖)。待 power_curve_helper.rs
// 落地后应切换为 crate::power_curve_helper 调用并删除这些内联副本 (Rust 同 crate
// 模块互相引用合法, 无循环依赖障碍)。constRpmBelowOldCritAlt 未被本文件使用, 不内联。

/// Checks if the stage has ConstRPM parameters defined.
///
/// Note: constRpmAlt=0 is a valid value (ConstRPM at sea level), so we only
/// check constRpmPower. WAPC's ConstRPM_is() checks key existence, not altitude value.
/// When FM doesn't define ConstRPM, both constRpmAlt and constRpmPower default to 0.
fn has_const_rpm(p: &CompressorStageParams) -> bool {
    p.const_rpm_power > 0.0
}

/// ConstRPM bend point is below the critical altitude.
/// This creates a two-segment curve below crit alt: deck→constRPM then constRPM→crit.
fn const_rpm_below_crit_alt(p: &CompressorStageParams) -> bool {
    has_const_rpm(p) && (p.const_rpm_alt - p.crit_alt) < -1.0
}

/// ConstRPM bend point is below the WEP critical altitude.
fn const_rpm_below_wep_crit_alt(p: &CompressorStageParams) -> bool {
    has_const_rpm(p) && (p.const_rpm_alt - p.wep_crit_alt) < -1.0
}

/// ConstRPM bends above critical altitude — used with ceiling parameters
/// to create a curved decay above crit alt (e.g., P-63).
fn const_rpm_above_crit_alt(p: &CompressorStageParams) -> bool {
    has_const_rpm(p)
        && p.const_rpm_alt == p.crit_alt
        && p.crit_power - p.ceiling_power > 1.0
        && p.curvature > 1.0
}

/// ConstRPM altitude is at or below sea level.
fn const_rpm_below_deck(p: &CompressorStageParams) -> bool {
    has_const_rpm(p) && p.const_rpm_alt <= 0.0
}

/// Checks if ceiling parameters exist.
fn has_ceiling(p: &CompressorStageParams) -> bool {
    p.ceiling_alt > 0.0 && p.ceiling_power > 0.0
}

/// Ceiling parameters are meaningful — altitude gap and power gap are both
/// significant enough to affect the curve shape.
///
/// Uses the original FM altitude/power (before definition_alt_power_adjuster)
/// to match WAPC's Ceiling_is_useful() which compares against Altitude[i] / Power[i],
/// not the adjusted critAlt / critPower.
fn ceiling_is_useful(p: &CompressorStageParams) -> bool {
    // PORT: Java 三目两臂均 double
    let reference_alt = if p.old_altitude > 0.0 { p.old_altitude } else { p.crit_alt };
    let reference_power = if p.old_power > 0.0 { p.old_power } else { p.crit_power };
    has_ceiling(p) && (p.ceiling_alt - reference_alt) >= 2.0 && (reference_power - p.ceiling_power) >= 2.0
}

/// Critical altitude equals deck altitude — the power curve is flat from sea level.
/// Deck→crit interpolation is skipped; curve goes directly to ceiling.
fn power_is_deck_power(p: &CompressorStageParams) -> bool {
    (p.crit_alt - p.deck_alt).abs() < 1.0
}

// =====================================================================
// Tests — 对应 Java: test/TestPistonPowerModel.java (一比一移植)
// =====================================================================

#[cfg(test)]
mod tests {
    // PORT: Java 保真 — 测试构造沿用 Java `new X(); x.f = v;` 逐字段赋值形态,
    // 不改成 struct 字面量以保持与 Java 测试源逐行对应
    #![allow(clippy::field_reassign_with_default)]

    use super::*;

    /// Tests for PistonPowerModel.
    ///
    /// Validates piston engine power curve calculations.
    ///
    /// Run with: ./script/test.sh
    ///
    /// PORT: Java 断言助手 assertClose/assertTrue (计数式 pass/fail) → Rust
    /// assert! 宏 (失败即 panic); `Math.abs(a-e) <= tol` 判定式逐字保留。
    /// Java 的 printf 信息输出 (非断言) 移植不保留
    fn assert_close(name: &str, actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "FAIL: {name} = {actual:.4} (expected {expected:.4}, tolerance {tolerance:.4})"
        );
    }

    fn assert_true(name: &str, condition: bool) {
        assert!(condition, "FAIL: {name}");
    }

    fn test_torque_rpm_boost() {
        // Testing torqueRpmBoost()...

        // Same RPM = no boost
        assert_close("same RPM", torque_rpm_boost(2400.0, 2400.0), 1.0, 0.001);

        // Higher RPM = boost > 1
        let boost = torque_rpm_boost(2400.0, 2600.0);
        assert_true("higher RPM gives boost", boost > 1.0);

        // Invalid inputs
        assert_close("zero lowerRPM", torque_rpm_boost(0.0, 2600.0), 1.0, 0.001);
        assert_close("zero higherRPM", torque_rpm_boost(2400.0, 0.0), 1.0, 0.001);

        // Boost is monotonic with RPM difference
        let boost1 = torque_rpm_boost(2400.0, 2500.0);
        let boost2 = torque_rpm_boost(2400.0, 2600.0);
        let boost3 = torque_rpm_boost(2400.0, 2700.0);
        assert_true("boost increases with RPM diff", boost1 < boost2 && boost2 < boost3);
    }

    fn test_torque_from_hp() {
        // Testing torqueFromHp()...

        // Known conversion: 726.115 * hp / rpm
        let torque = torque_from_hp(1000.0, 2400.0);
        assert_close("torque at 1000hp/2400rpm", torque, 302.55, 0.1);

        // Zero RPM handling
        assert_close("zero RPM", torque_from_hp(1000.0, 0.0), 0.0, 0.001);

        // Torque proportional to power
        let torque2 = torque_from_hp(2000.0, 2400.0);
        assert_close("double power = double torque", torque2, torque * 2.0, 0.1);

        // Torque inversely proportional to RPM
        let torque3 = torque_from_hp(1000.0, 4800.0);
        assert_close("double RPM = half torque", torque3, torque / 2.0, 0.1);
    }

    fn test_supercharger_rpm_effect() {
        // Testing superchargerRpmEffect()...

        // Typical parameters
        let effect = supercharger_rpm_effect(2400.0, 2600.0, 0.2, 0.1);
        assert_true("effect > 1 with RPM increase", effect > 1.0);

        // No RPM difference = no effect
        assert_close("same RPM", supercharger_rpm_effect(2400.0, 2400.0, 0.2, 0.1), 1.0, 0.001);

        // Invalid input
        assert_close("zero milRPM", supercharger_rpm_effect(0.0, 2600.0, 0.2, 0.1), 1.0, 0.001);

        // Higher compressor factors = more effect
        let effect1 = supercharger_rpm_effect(2400.0, 2600.0, 0.1, 0.1);
        let effect2 = supercharger_rpm_effect(2400.0, 2600.0, 0.3, 0.1);
        assert_true("lower pressureAtRPM0 = more effect", effect1 > effect2);
    }

    fn test_interpolate_power() {
        // Testing interpolatePower()...

        // At lower altitude point
        let at_lower = interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 0.0, 1.0);
        assert_close("at lower point", at_lower, 2000.0, 1.0);

        // At higher altitude point
        let at_higher = interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 5000.0, 1.0);
        assert_close("at higher point", at_higher, 1800.0, 1.0);

        // In between (should be between the two values)
        let at_mid = interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 2500.0, 1.0);
        assert_true("mid point between", at_mid > 1800.0 && at_mid < 2000.0);

        // Curvature effect
        let linear = interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 2500.0, 1.0);
        let curved = interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 2500.0, 2.0);
        assert_true("curvature affects interpolation", (linear - curved).abs() > 1.0);
    }

    fn test_wep_power_multiplier() {
        // Testing wepPowerMultiplier()...

        // All factors = 1 and same RPM = multiplier of 1
        assert_close("baseline", wep_power_multiplier(1.0, 1.0, 1.0, 1.0, 2400.0, 2400.0), 1.0, 0.01);

        // Typical WEP parameters
        let mult = wep_power_multiplier(1.15, 1.0, 1.0, 1.0, 2400.0, 2600.0);
        assert_true("WEP mult > 1", mult > 1.0);

        // AfterburnerBoost effect
        let mult1 = wep_power_multiplier(1.10, 1.0, 1.0, 1.0, 2400.0, 2400.0);
        let mult2 = wep_power_multiplier(1.20, 1.0, 1.0, 1.0, 2400.0, 2400.0);
        assert_true("higher boost = higher mult", mult2 > mult1);
    }

    fn test_wep_critical_altitude() {
        // Testing wepCriticalAltitude()...

        // WEP requires higher manifold pressure, so critical altitude is lower
        let wep_crit_alt = wep_critical_altitude(7000.0, 1.42, 1.65, 1.0, 1.0);
        assert_true("WEP crit alt < mil crit alt", wep_crit_alt < 7000.0);

        // With supercharger RPM effect boost
        let wep_crit_alt2 = wep_critical_altitude(7000.0, 1.42, 1.65, 1.1, 1.0);
        assert_true("RPM effect raises WEP crit alt", wep_crit_alt2 > wep_crit_alt);

        // With pressure boost
        let wep_crit_alt3 = wep_critical_altitude(7000.0, 1.42, 1.65, 1.0, 1.1);
        assert_true("pressure boost raises WEP crit alt", wep_crit_alt3 > wep_crit_alt);
    }

    fn test_power_at_altitude() {
        // Testing powerAtAltitudeAdvanced()...

        // Create test engine (simplified P-47D-like)
        let mut stage = CompressorStageParams::default();
        stage.crit_alt = 7000.0;
        stage.crit_power = 2000.0;
        stage.deck_power = 1850.0;
        stage.deck_alt = 0.0;
        stage.curvature = 1.0;
        stage.wep_crit_alt = 6000.0;
        stage.wep_power_mult = 1.15;
        stage.speed_manifold_mult = 0.9;
        stage.old_altitude = 7000.0;
        stage.old_power = 2000.0;
        stage.old_power_new_rpm = 2000.0;

        // At sea level
        let p0 = power_at_altitude_advanced(&stage, 0.0, false, 0.0, false, 15.0);
        assert_close("power at sea level", p0, 1850.0, 10.0);

        // At critical altitude
        let p_crit = power_at_altitude_advanced(&stage, 7000.0, false, 0.0, false, 15.0);
        assert_close("power at crit alt", p_crit, 2000.0, 10.0);

        // Above critical altitude (power drops)
        let p_high = power_at_altitude_advanced(&stage, 9000.0, false, 0.0, false, 15.0);
        assert_true("power drops above crit alt", p_high < p_crit);

        // WEP gives more power
        let p_wep = power_at_altitude_advanced(&stage, 5000.0, true, 0.0, false, 15.0);
        let p_mil = power_at_altitude_advanced(&stage, 5000.0, false, 0.0, false, 15.0);
        assert_true("WEP > military", p_wep > p_mil);
    }

    fn test_multi_stage_supercharger() {
        // Testing multi-stage supercharger...

        // Two-stage supercharger (like Spitfire Merlin)
        let mut stage1 = CompressorStageParams::default();
        stage1.crit_alt = 3000.0;
        stage1.crit_power = 1400.0;
        stage1.deck_power = 1350.0;
        stage1.wep_crit_alt = 2500.0;
        stage1.wep_power_mult = 1.1;
        stage1.stage_index = 0;
        stage1.old_altitude = 3000.0;
        stage1.old_power = 1400.0;
        stage1.old_power_new_rpm = 1400.0;

        let mut stage2 = CompressorStageParams::default();
        stage2.crit_alt = 6500.0;
        stage2.crit_power = 1300.0;
        stage2.deck_power = 1100.0;
        stage2.wep_crit_alt = 6000.0;
        stage2.wep_power_mult = 1.1;
        stage2.stage_index = 1;
        stage2.old_altitude = 6500.0;
        stage2.old_power = 1300.0;
        stage2.old_power_new_rpm = 1300.0;

        let stages = [stage1, stage2];

        // At low altitude, stage 1 is better
        let p_low = optimal_power_advanced(&stages, 1000.0, false, 0.0, false, 15.0);
        let p1_low = power_at_altitude_advanced(&stage1, 1000.0, false, 0.0, false, 15.0);
        let p2_low = power_at_altitude_advanced(&stage2, 1000.0, false, 0.0, false, 15.0);
        assert_true("stage 1 better at low alt", p1_low > p2_low);
        assert_close("optimal selects stage 1", p_low, p1_low, 1.0);

        // At high altitude, stage 2 is better
        let p_high = optimal_power_advanced(&stages, 6000.0, false, 0.0, false, 15.0);
        let p1_high = power_at_altitude_advanced(&stage1, 6000.0, false, 0.0, false, 15.0);
        let p2_high = power_at_altitude_advanced(&stage2, 6000.0, false, 0.0, false, 15.0);
        assert_true("stage 2 better at high alt", p2_high > p1_high);
        assert_close("optimal selects stage 2", p_high, p2_high, 1.0);
    }

    fn test_generate_power_curve() {
        // Testing generatePowerCurveAdvanced()...

        let mut stage = CompressorStageParams::new(5000.0, 1500.0, 1400.0);
        stage.wep_crit_alt = 4500.0;
        stage.wep_power_mult = 1.1;
        stage.old_altitude = 5000.0;
        stage.old_power = 1500.0;
        stage.old_power_new_rpm = 1500.0;
        let stages = [stage];

        let curve = generate_power_curve_advanced(&stages, false, 0.0, false, 15.0, 50);

        // Check array size (range: 0m to 10000m, step 50m)
        let expected_size = 10000 / 50 + 1; // 201 points
        assert_true("curve size correct", curve.len() == expected_size);

        // Check values at known altitudes (index = altitude / step)
        let sea_level_idx = 0usize;
        assert_close("curve at sea level", curve[sea_level_idx], 1400.0, 10.0);

        // Index at 5000m = 5000 / 50 = 100
        let crit_alt_idx = 5000 / 50;
        assert_close("curve at crit alt", curve[crit_alt_idx], 1500.0, 10.0);

        // Power decreases after critical altitude (8000m = index 160)
        let high_alt_idx = 8000 / 50;
        assert_true("power decreases at high alt", curve[high_alt_idx] < curve[crit_alt_idx]);
    }

    fn test_ram_effect_integration() {
        // Testing RAM effect integration...

        let mut stage = CompressorStageParams::default();
        stage.crit_alt = 7000.0;
        stage.crit_power = 2000.0;
        stage.deck_power = 1850.0;
        stage.wep_crit_alt = 6500.0;
        stage.wep_power_mult = 1.1;
        stage.speed_manifold_mult = 0.9;
        stage.old_altitude = 7000.0;
        stage.old_power = 2000.0;
        stage.old_power_new_rpm = 2000.0;

        // Test RAM effect well ABOVE critical altitude where power drops with altitude
        // Use higher altitude to avoid effective altitude crossing critical altitude
        let p_static_12k = power_at_altitude_advanced(&stage, 12000.0, false, 0.0, false, 15.0);
        let p_moving_12k = power_at_altitude_advanced(&stage, 12000.0, false, 400.0, true, 15.0);

        // RAM effect should increase power above critical altitude
        assert_true("RAM increases power above crit alt", p_moving_12k > p_static_12k);

        // Higher speed = more RAM effect (use moderate speeds to stay above crit alt)
        let p_faster_12k = power_at_altitude_advanced(&stage, 12000.0, false, 500.0, true, 15.0);
        assert_true("faster = more RAM above crit alt", p_faster_12k > p_moving_12k);

        // Note: Below critical altitude, RAM effect may DECREASE power because
        // in this model deck power < crit power (power increases toward crit alt).
        // This is physically correct for supercharged engines where the
        // supercharger maintains constant boost up to critical altitude.
        let _p_static_low = power_at_altitude_advanced(&stage, 5000.0, false, 0.0, false, 15.0);
        let _p_moving_low = power_at_altitude_advanced(&stage, 5000.0, false, 500.0, true, 15.0);
        // (Below crit alt, RAM lowers effective alt, which may reduce power)
    }

    fn test_no_wep_aircraft_identical_curves() {
        // Testing no-WEP aircraft (Yak-3 style)...

        // Yak-3 Stage 0: critAlt=300m, Power=1310hp, Ceiling=5000m/670hp
        // AfterburnerBoost=1 → wepPowerMult=1.0, so WEP == military
        let mut stage0 = CompressorStageParams::default();
        stage0.crit_alt = 300.0;
        stage0.crit_power = 1310.0;
        stage0.deck_power = 1290.0;
        stage0.deck_alt = 0.0;
        stage0.curvature = 1.0;
        stage0.ceiling_alt = 5000.0;
        stage0.ceiling_power = 670.0;
        stage0.old_altitude = 300.0;
        stage0.old_power = 1310.0;
        stage0.old_power_new_rpm = 1310.0;
        stage0.exact_altitudes = true;
        stage0.stage_index = 0;
        // No WEP: wepPowerMult=1.0, wepCritAlt == critAlt
        stage0.wep_power_mult = 1.0;
        stage0.wep_crit_alt = 300.0; // Must equal critAlt when no WEP
        stage0.wep_deck_alt = 0.0;
        stage0.speed_manifold_mult = 1.0;

        // Yak-3 Stage 1: critAlt=2600m, Power=1240hp, Ceiling=9000m/510hp
        let mut stage1 = CompressorStageParams::default();
        stage1.crit_alt = 2600.0;
        stage1.crit_power = 1240.0;
        stage1.deck_power = 1290.0 * 0.8;
        stage1.deck_alt = 0.0;
        stage1.curvature = 1.0;
        stage1.ceiling_alt = 9000.0;
        stage1.ceiling_power = 510.0;
        stage1.old_altitude = 2600.0;
        stage1.old_power = 1240.0;
        stage1.old_power_new_rpm = 1240.0;
        stage1.exact_altitudes = true;
        stage1.stage_index = 1;
        stage1.wep_power_mult = 1.0;
        stage1.wep_crit_alt = 2600.0;
        stage1.wep_deck_alt = 0.0;
        stage1.speed_manifold_mult = 1.0;

        let stages = [stage0, stage1];

        // WEP and military curves must be identical at every altitude
        let mut all_match = true;
        // PORT: Java `for (int alt = 0; alt <= 10000; alt += 500)` int 步进循环
        for alt in (0..=10000i32).step_by(500) {
            let alt_f = alt as f64;
            let mil_power = optimal_power_advanced(&stages, alt_f, false, 0.0, false, 15.0);
            let wep_power = optimal_power_advanced(&stages, alt_f, true, 0.0, false, 15.0);
            if (mil_power - wep_power).abs() > 0.1 {
                all_match = false;
            }
        }
        assert_true("no-WEP: WEP curve equals military curve", all_match);

        // Also verify with RAM effect (301 km/h IAS)
        let mut all_match_ram = true;
        for alt in (0..=10000i32).step_by(500) {
            let alt_f = alt as f64;
            let mil_power = optimal_power_advanced(&stages, alt_f, false, 301.0, true, 15.0);
            let wep_power = optimal_power_advanced(&stages, alt_f, true, 301.0, true, 15.0);
            if (mil_power - wep_power).abs() > 0.1 {
                all_match_ram = false;
            }
        }
        assert_true("no-WEP with RAM: WEP curve equals military curve", all_match_ram);
    }

    fn test_peak_wep_power() {
        // Testing peakWepPower()...

        // Test 1: Single-stage supercharger
        let mut stage = CompressorStageParams::default();
        stage.crit_alt = 5000.0;
        stage.crit_power = 1500.0;
        stage.deck_power = 1400.0;
        stage.wep_crit_alt = 4500.0;
        stage.wep_power_mult = 1.15; // 15% WEP boost
        stage.old_altitude = 5000.0;
        stage.old_power = 1500.0;
        stage.old_power_new_rpm = 1500.0;
        let single_stage = [stage];

        let peak_single = peak_wep_power(&single_stage);
        // Peak should be critPower × wepPowerMult = 1500 × 1.15 = 1725 hp (approximately)
        let expected_peak = stage.crit_power * stage.wep_power_mult;
        assert_close("single-stage peak WEP power", peak_single, expected_peak, 20.0);

        // Test 2: Multi-stage supercharger (peak is max across all stages)
        let mut stage1 = CompressorStageParams::default();
        stage1.crit_alt = 3000.0;
        stage1.crit_power = 1400.0;
        stage1.deck_power = 1350.0;
        stage1.wep_crit_alt = 2500.0;
        stage1.wep_power_mult = 1.1;
        stage1.stage_index = 0;
        stage1.old_altitude = 3000.0;
        stage1.old_power = 1400.0;
        stage1.old_power_new_rpm = 1400.0;

        let mut stage2 = CompressorStageParams::default();
        stage2.crit_alt = 6500.0;
        stage2.crit_power = 1300.0;
        stage2.deck_power = 1100.0;
        stage2.wep_crit_alt = 6000.0;
        stage2.wep_power_mult = 1.1;
        stage2.stage_index = 1;
        stage2.old_altitude = 6500.0;
        stage2.old_power = 1300.0;
        stage2.old_power_new_rpm = 1300.0;

        let multi_stage = [stage1, stage2];
        let peak_multi = peak_wep_power(&multi_stage);
        // Stage 1 should give higher peak (1400 × 1.1 = 1540hp vs 1300 × 1.1 = 1430hp)
        let expected_multi_peak = stage1.crit_power * stage1.wep_power_mult;
        assert_close("multi-stage peak WEP power", peak_multi, expected_multi_peak, 20.0);

        // Test 3: Empty array returns 0
        let peak_empty = peak_wep_power(&[]);
        assert_close("empty array returns 0", peak_empty, 0.0, 0.001);

        // Test 4: Null array returns 0
        // PORT: Java null 参数在 Rust 切片模型下无对应 (无 null 切片),
        // 空切片走同一 `stages.length == 0` 提前返回路径, 行为等价 (0)
        // (与 Test 3 同路径, 保留用例编号以对齐 Java 测试文本)

        // Test 5: No-WEP aircraft (wepPowerMult = 1.0)
        let mut no_wep_stage = CompressorStageParams::default();
        no_wep_stage.crit_alt = 3000.0;
        no_wep_stage.crit_power = 1200.0;
        no_wep_stage.deck_power = 1150.0;
        no_wep_stage.wep_crit_alt = 3000.0;
        no_wep_stage.wep_power_mult = 1.0; // No WEP
        no_wep_stage.old_altitude = 3000.0;
        no_wep_stage.old_power = 1200.0;
        no_wep_stage.old_power_new_rpm = 1200.0;
        let no_wep_array = [no_wep_stage];

        let peak_no_wep = peak_wep_power(&no_wep_array);
        // With no WEP, peak should equal critPower
        assert_close("no-WEP peak equals critPower", peak_no_wep, no_wep_stage.crit_power, 10.0);
    }

    /// Java 8 oracle 对拍 (PORTING.md §5.1 A 类策略):
    /// 期望值 = build/oracle/PistonPowerOracle{,2}.java 在 OpenJDK 1.8.0_342 上
    /// dump 的 %.17g 实测值 (临时文件, 用完已删除)。容差取混合式 1e-12·max(|expected|,1)
    /// (|expected|<1 时退化为绝对容差): Math.pow 跨 libm 实现允许最后几位 ULP 差异,
    /// 远小于业务断言容差。覆盖 variabler 军用/WEP 全部分支形态。
    #[test]
    fn java8_oracle_parity() {
        let tol = 1e-12;
        let check = |name: &str, actual: f64, expected: f64| {
            // 混合容差: expected 为 0 时退化为绝对容差
            let diff = (actual - expected).abs();
            assert!(
                diff <= tol * expected.abs().max(1.0),
                "oracle mismatch {name}: rust={actual:?} java={expected:?}"
            );
        };

        // === Part A: 纯函数 ===
        check("tq_same", torque_rpm_boost(2400.0, 2400.0), 1.0);
        check("tq_2600", torque_rpm_boost(2400.0, 2600.0), 1.0171296296296297);
        check("tq_2700", torque_rpm_boost(2400.0, 2700.0), 1.0355113636363635);
        check("tq_2500", torque_rpm_boost(2400.0, 2500.0), 1.0046939300411524);
        check("tq_zero_lo", torque_rpm_boost(0.0, 2600.0), 1.0);
        check("tq_zero_hi", torque_rpm_boost(2400.0, 0.0), 1.0);
        check("tqf_1000_2400", torque_from_hp(1000.0, 2400.0), 302.54791666666665);
        check("tqf_1000_4800", torque_from_hp(1000.0, 4800.0), 151.27395833333333);
        check("tqf_2000_2400", torque_from_hp(2000.0, 2400.0), 605.0958333333333);
        check("tqf_zero_rpm", torque_from_hp(1000.0, 0.0), 0.0);
        check("sce_typ", supercharger_rpm_effect(2400.0, 2600.0, 0.2, 0.1), 1.0735730379653925);
        check("sce_same", supercharger_rpm_effect(2400.0, 2400.0, 0.2, 0.1), 1.0);
        check("sce_p03", supercharger_rpm_effect(2400.0, 2600.0, 0.3, 0.1), 1.0643506320619338);
        check("sce_om05", supercharger_rpm_effect(2400.0, 2600.0, 0.2, 0.5), 1.1016485962545541);
        check("ip_low", interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 0.0, 1.0), 2000.0);
        check("ip_high", interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 5000.0, 1.0), 1800.0);
        check("ip_mid1", interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 2500.0, 1.0), 1887.3589678159094);
        check("ip_mid2", interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 2500.0, 2.0), 1936.5599893425133);
        check("ip_below", interpolate_power(1800.0, 5000.0, 2000.0, 0.0, -100.0, 1.0), 2005.1034456173313);
        check("ip_degen", interpolate_power(1000.0, 3000.0, 990.0, 3000.0, 2500.0, 1.0), 990.0);
        check("wepm_115", wep_power_multiplier(1.15, 1.0, 1.0, 1.0, 2400.0, 2600.0), 1.169_699_074_074_074);
        check("wepm_110", wep_power_multiplier(1.10, 1.0, 1.0, 1.0, 2400.0, 2400.0), 1.1);
        check("wepm_120", wep_power_multiplier(1.20, 1.0, 1.0, 1.0, 2400.0, 2400.0), 1.2);
        check("wepm_oct18", wep_power_multiplier(1.15, 1.0, 1.1, 1.8, 2400.0, 2600.0), 1.4209300925925925);
        check("wca_base", wep_critical_altitude(7000.0, 1.42, 1.65, 1.0, 1.0), 5_918.386_017_652_959);
        check("wca_rpm11", wep_critical_altitude(7000.0, 1.42, 1.65, 1.1, 1.0), 6_608.678_596_256_166);
        check("wca_pb11", wep_critical_altitude(7000.0, 1.42, 1.65, 1.0, 1.1), 6_608.678_596_256_166);

        // === Part B: P-47D-like 单级 ===
        let mut p47 = CompressorStageParams::default();
        p47.crit_alt = 7000.0;
        p47.crit_power = 2000.0;
        p47.deck_power = 1850.0;
        p47.deck_alt = 0.0;
        p47.curvature = 1.0;
        p47.wep_crit_alt = 6000.0;
        p47.wep_power_mult = 1.15;
        p47.speed_manifold_mult = 0.9;
        p47.old_altitude = 7000.0;
        p47.old_power = 2000.0;
        p47.old_power_new_rpm = 2000.0;
        check("p47_0", power_at_altitude_advanced(&p47, 0.0, false, 0.0, false, 15.0), 1850.0);
        check("p47_5000m", power_at_altitude_advanced(&p47, 5000.0, false, 0.0, false, 15.0), 1_967.744_152_587_75);
        check("p47_5000w", power_at_altitude_advanced(&p47, 5000.0, true, 0.0, false, 15.0), 2278.2116615819164);
        check("p47_7000", power_at_altitude_advanced(&p47, 7000.0, false, 0.0, false, 15.0), 2000.0);
        check("p47_7000w", power_at_altitude_advanced(&p47, 7000.0, true, 0.0, false, 15.0), 2001.6456091340815);
        check("p47_9000", power_at_altitude_advanced(&p47, 9000.0, false, 0.0, false, 15.0), 1497.4132153786454);
        check("p47_10000", power_at_altitude_advanced(&p47, 10000.0, false, 0.0, false, 15.0), 1287.6657433610794);
        check("p47_12000s", power_at_altitude_advanced(&p47, 12000.0, false, 0.0, false, 15.0), 939.280_857_552_052_6);
        check("p47_12000v400", power_at_altitude_advanced(&p47, 12000.0, false, 400.0, true, 15.0), 1270.7683498691983);
        check("p47_12000v500", power_at_altitude_advanced(&p47, 12000.0, false, 500.0, true, 15.0), 1_457.230_064_297_593);
        check("p47_5000v500", power_at_altitude_advanced(&p47, 5000.0, false, 500.0, true, 15.0), 1941.2765375362055);

        // === Part C: 两级机械增压器 ===
        let mut s1 = CompressorStageParams::default();
        s1.crit_alt = 3000.0;
        s1.crit_power = 1400.0;
        s1.deck_power = 1350.0;
        s1.wep_crit_alt = 2500.0;
        s1.wep_power_mult = 1.1;
        s1.stage_index = 0;
        s1.old_altitude = 3000.0;
        s1.old_power = 1400.0;
        s1.old_power_new_rpm = 1400.0;
        let mut s2 = CompressorStageParams::default();
        s2.crit_alt = 6500.0;
        s2.crit_power = 1300.0;
        s2.deck_power = 1100.0;
        s2.wep_crit_alt = 6000.0;
        s2.wep_power_mult = 1.1;
        s2.stage_index = 1;
        s2.old_altitude = 6500.0;
        s2.old_power = 1300.0;
        s2.old_power_new_rpm = 1300.0;
        let two = [s1, s2];
        check("two_1000opt", optimal_power_advanced(&two, 1000.0, false, 0.0, false, 15.0), 1368.3403761790846);
        check("two_1000s1", power_at_altitude_advanced(&s1, 1000.0, false, 0.0, false, 15.0), 1368.3403761790846);
        check("two_1000s2", power_at_altitude_advanced(&s2, 1000.0, false, 0.0, false, 15.0), 1_139.973_474_805_542);
        check("two_6000opt", optimal_power_advanced(&two, 6000.0, false, 0.0, false, 15.0), 1_289.016_680_952_667);
        check("two_6000s1", power_at_altitude_advanced(&s1, 6000.0, false, 0.0, false, 15.0), 942.159_246_548_316);
        check("two_6000s2", power_at_altitude_advanced(&s2, 6000.0, false, 0.0, false, 15.0), 1_289.016_680_952_667);
        check("two_4000w", optimal_power_advanced(&two, 4000.0, true, 0.0, false, 15.0), 1371.2487721441946);
        assert_eq!(find_optimal_stage_index(&two, 1000.0, false, 0.0, false, 15.0), 0);
        assert_eq!(find_optimal_stage_index(&two, 6000.0, false, 0.0, false, 15.0), 1);
        assert_eq!(find_optimal_stage_index(&two, 4000.0, true, 0.0, false, 15.0), 1);

        // === Part D: generatePowerCurveAdvanced ===
        let mut gc = CompressorStageParams::new(5000.0, 1500.0, 1400.0);
        gc.wep_crit_alt = 4500.0;
        gc.wep_power_mult = 1.1;
        gc.old_altitude = 5000.0;
        gc.old_power = 1500.0;
        gc.old_power_new_rpm = 1500.0;
        let gc_arr = [gc];
        let curve = generate_power_curve_advanced(&gc_arr, false, 0.0, false, 15.0, 50);
        assert_eq!(curve.len(), 201); // gc_size
        check("gc_0", curve[0], 1400.0);
        check("gc_100", curve[100], 1500.0);
        check("gc_160", curve[160], 988.518_743_244_433_6);
        check("gc_200", curve[200], 734.069_573_327_452);

        // === Part E: Yak-3 无 WEP (exactAltitudes=true) ===
        let mut y0 = CompressorStageParams::default();
        y0.crit_alt = 300.0;
        y0.crit_power = 1310.0;
        y0.deck_power = 1290.0;
        y0.deck_alt = 0.0;
        y0.curvature = 1.0;
        y0.ceiling_alt = 5000.0;
        y0.ceiling_power = 670.0;
        y0.old_altitude = 300.0;
        y0.old_power = 1310.0;
        y0.old_power_new_rpm = 1310.0;
        y0.exact_altitudes = true;
        y0.stage_index = 0;
        y0.wep_power_mult = 1.0;
        y0.wep_crit_alt = 300.0;
        y0.wep_deck_alt = 0.0;
        y0.speed_manifold_mult = 1.0;
        let mut y1 = CompressorStageParams::default();
        y1.crit_alt = 2600.0;
        y1.crit_power = 1240.0;
        y1.deck_power = 1290.0 * 0.8;
        y1.deck_alt = 0.0;
        y1.curvature = 1.0;
        y1.ceiling_alt = 9000.0;
        y1.ceiling_power = 510.0;
        y1.old_altitude = 2600.0;
        y1.old_power = 1240.0;
        y1.old_power_new_rpm = 1240.0;
        y1.exact_altitudes = true;
        y1.stage_index = 1;
        y1.wep_power_mult = 1.0;
        y1.wep_crit_alt = 2600.0;
        y1.wep_deck_alt = 0.0;
        y1.speed_manifold_mult = 1.0;
        let yak = [y0, y1];
        let yak_m = [
            1290.0, 1_194.470_459_653_116, 1196.6571288157202, 1_178.207_276_690_934,
            1034.4645783240283, 905.115_774_247_377_9, 789.031_271_730_281_6, 685.144_435_518_207_6,
            592.449_584_215_098, 510.0, 436.90595194307707,
        ];
        // Java dump: WEP 曲线与军用逐点相同 (无 WEP 机型)
        for (k, &exp) in yak_m.iter().enumerate() {
            let alt = (k as i32 * 1000) as f64;
            check(&format!("yak_{k}m"), optimal_power_advanced(&yak, alt, false, 0.0, false, 15.0), exp);
            check(&format!("yak_{k}w"), optimal_power_advanced(&yak, alt, true, 0.0, false, 15.0), exp);
        }
        let yakr = [
            1265.8932300069234, 1200.6608991925395, 977.797_114_068_640_6,
            710.139_880_165_562_4, 509.587_291_764_339_5,
        ];
        for (k, &exp) in yakr.iter().enumerate() {
            let alt = (k as i32 * 2500) as f64;
            check(&format!("yakr_{k}m"), optimal_power_advanced(&yak, alt, false, 301.0, true, 15.0), exp);
            check(&format!("yakr_{k}w"), optimal_power_advanced(&yak, alt, true, 301.0, true, 15.0), exp);
        }

        // === Part F1: ConstRPM 低于甲板零功率区 ===
        let mut f1 = CompressorStageParams::default();
        f1.crit_alt = 7000.0;
        f1.crit_power = 2000.0;
        f1.deck_power = 1850.0;
        f1.wep_crit_alt = 6000.0;
        f1.wep_power_mult = 1.15;
        f1.old_altitude = 7000.0;
        f1.old_power = 2000.0;
        f1.old_power_new_rpm = 2000.0;
        f1.const_rpm_alt = 0.0;
        f1.const_rpm_power = 100.0;
        check("f1_m100m", power_at_altitude_advanced(&f1, -100.0, false, 0.0, false, 15.0), 0.0);
        check("f1_m100w", power_at_altitude_advanced(&f1, -100.0, true, 0.0, false, 15.0), 0.0);

        // === Part F2: ConstRPM 弯折低于临界高度 (两段式) ===
        let mut f2 = CompressorStageParams::default();
        f2.crit_alt = 7000.0;
        f2.crit_power = 2000.0;
        f2.deck_power = 1850.0;
        f2.curvature = 1.5;
        f2.wep_crit_alt = 6500.0;
        f2.wep_power_mult = 1.15;
        f2.old_altitude = 7000.0;
        f2.old_power = 2000.0;
        f2.old_power_new_rpm = 2000.0;
        f2.const_rpm_alt = 3000.0;
        f2.const_rpm_power = 1900.0;
        f2.wep_const_rpm_alt = 2800.0;
        f2.stage0_deck_alt = 0.0;
        f2.wep_deck_alt = 0.0;
        check("f2_1500m", power_at_altitude_advanced(&f2, 1500.0, false, 0.0, false, 15.0), 1876.8592257962787);
        check("f2_5000m", power_at_altitude_advanced(&f2, 5000.0, false, 0.0, false, 15.0), 1941.2200836990635);
        check("f2_9000m", power_at_altitude_advanced(&f2, 9000.0, false, 0.0, false, 15.0), 1497.4132153786454);
        f2.exact_altitudes = true;
        check("f2e_1500w", power_at_altitude_advanced(&f2, 1500.0, true, 0.0, false, 15.0), 2158.3881096657205);
        check("f2e_5000w", power_at_altitude_advanced(&f2, 5000.0, true, 0.0, false, 15.0), 2_232.403_096_253_923);
        f2.exact_altitudes = false;
        check("f2n_1500w", power_at_altitude_advanced(&f2, 1500.0, true, 0.0, false, 15.0), 2160.2798813437057);
        check("f2n_5000w", power_at_altitude_advanced(&f2, 5000.0, true, 0.0, false, 15.0), 2_244.127_818_748_27);

        // === Part F3: powerIsDeckPower (critAlt==deckAlt) + ceiling ===
        let mut f3 = CompressorStageParams::default();
        f3.crit_alt = 5000.0;
        f3.crit_power = 2000.0;
        f3.deck_power = 1800.0;
        f3.deck_alt = 5000.0;
        f3.wep_crit_alt = 4500.0;
        f3.wep_power_mult = 1.1;
        f3.old_altitude = 5000.0;
        f3.old_power = 2000.0;
        f3.old_power_new_rpm = 2000.0;
        f3.ceiling_alt = 10000.0;
        f3.ceiling_power = 900.0;
        check("f3_2000m", power_at_altitude_advanced(&f3, 2000.0, false, 0.0, false, 15.0), 3015.9225747678397);
        check("f3_6000m", power_at_altitude_advanced(&f3, 6000.0, false, 0.0, false, 15.0), 1727.2740837849933);
        check("f3_8000w", power_at_altitude_advanced(&f3, 8000.0, true, 0.0, false, 15.0), 1280.6909993159848);
        f3.exact_altitudes = true;
        check("f3e_2000m", power_at_altitude_advanced(&f3, 2000.0, false, 0.0, false, 15.0), 3015.9225747678397);
        check("f3e_6000m", power_at_altitude_advanced(&f3, 6000.0, false, 0.0, false, 15.0), 1727.2740837849933);
        check("f3e_8000w", power_at_altitude_advanced(&f3, 8000.0, true, 0.0, false, 15.0), 1_291.654_865_726_723);

        // === Part F4: Fw-190A-1 式 oldAltitude < altRam <= wepCritAlt ===
        let mut f4 = CompressorStageParams::default();
        f4.crit_alt = 5000.0;
        f4.crit_power = 1800.0;
        f4.deck_power = 1750.0;
        f4.wep_crit_alt = 6500.0;
        f4.wep_power_mult = 1.1;
        f4.old_altitude = 5000.0;
        f4.old_power = 1800.0;
        f4.old_power_new_rpm = 1800.0;
        f4.ceiling_alt = 10000.0;
        f4.ceiling_power = 800.0;
        check("f4_6000w", power_at_altitude_advanced(&f4, 6000.0, true, 0.0, false, 15.0), 1980.0000000000002);
        f4.exact_altitudes = true;
        check("f4e_6000w", power_at_altitude_advanced(&f4, 6000.0, true, 0.0, false, 15.0), 1980.0000000000002);

        // === Part F5: Math.round 分支 (round(wepCritAlt) < altRam <= round(oldAltitude)) ===
        let mut f5 = CompressorStageParams::default();
        f5.crit_alt = 5000.0;
        f5.crit_power = 1800.0;
        f5.deck_power = 1750.0;
        f5.wep_crit_alt = 4400.6;
        f5.wep_power_mult = 1.1;
        f5.old_altitude = 5000.7;
        f5.old_power = 1800.0;
        f5.old_power_new_rpm = 1800.0;
        f5.ceiling_alt = 10000.0;
        f5.ceiling_power = 800.0;
        check("f5_4450w", power_at_altitude_advanced(&f5, 4450.0, true, 0.0, false, 15.0), 1966.0358146615285);
        f5.exact_altitudes = true;
        check("f5e_4450w", power_at_altitude_advanced(&f5, 4450.0, true, 0.0, false, 15.0), 1_961.272_999_158_355);
        // 无 ceiling 变体 (ceilingIsUseful=false)
        let mut f5b = CompressorStageParams::default();
        f5b.crit_alt = 5000.0;
        f5b.crit_power = 1800.0;
        f5b.deck_power = 1750.0;
        f5b.wep_crit_alt = 4400.6;
        f5b.wep_power_mult = 1.1;
        f5b.old_altitude = 5000.7;
        f5b.old_power = 1800.0;
        f5b.old_power_new_rpm = 1800.0;
        check("f5b_4450w", power_at_altitude_advanced(&f5b, 4450.0, true, 0.0, false, 15.0), 1_967.159_626_018_502);
        // constRpmBelowWepCritAlt=true 变体
        let mut b = CompressorStageParams::default();
        b.crit_alt = 5000.0;
        b.crit_power = 1800.0;
        b.deck_power = 1750.0;
        b.wep_crit_alt = 4400.6;
        b.wep_power_mult = 1.1;
        b.old_altitude = 5000.7;
        b.old_power = 1800.0;
        b.old_power_new_rpm = 1800.0;
        b.const_rpm_alt = 3000.0;
        b.const_rpm_power = 1650.0;
        b.curvature = 1.3;
        check("b_4450w", power_at_altitude_advanced(&b, 4450.0, true, 0.0, false, 15.0), 1_967.159_626_018_502);
        b.exact_altitudes = true;
        check("be_4450w", power_at_altitude_advanced(&b, 4450.0, true, 0.0, false, 15.0), 1915.0609690787915);

        // === Part F6: WEP 高于双临界高度 ===
        let mut f6 = CompressorStageParams::default();
        f6.crit_alt = 7000.0;
        f6.crit_power = 2000.0;
        f6.deck_power = 1850.0;
        f6.wep_crit_alt = 4500.0;
        f6.wep_power_mult = 1.15;
        f6.old_altitude = 7000.0;
        f6.old_power = 2000.0;
        f6.old_power_new_rpm = 2000.0;
        check("f6_8000w", power_at_altitude_advanced(&f6, 8000.0, true, 0.0, false, 15.0), 1418.3597215439743);
        f6.ceiling_alt = 11000.0;
        f6.ceiling_power = 700.0;
        check("f6c_8000w", power_at_altitude_advanced(&f6, 8000.0, true, 0.0, false, 15.0), 1291.1852371464227);
        f6.exact_altitudes = true;
        check("f6ce_8000w", power_at_altitude_advanced(&f6, 8000.0, true, 0.0, false, 15.0), 1_023.158_284_819_586);
        let mut f6d = CompressorStageParams::default();
        f6d.crit_alt = 7000.0;
        f6d.crit_power = 2000.0;
        f6d.deck_power = 1850.0;
        f6d.wep_crit_alt = 6500.0;
        f6d.wep_power_mult = 1.15;
        f6d.old_altitude = 7000.0;
        f6d.old_power = 2000.0;
        f6d.old_power_new_rpm = 2000.0;
        check("f6d_8000w", power_at_altitude_advanced(&f6d, 8000.0, true, 0.0, false, 15.0), 1859.4262634513013);
        f6d.exact_altitudes = true;
        check("f6de_8000w", power_at_altitude_advanced(&f6d, 8000.0, true, 0.0, false, 15.0), 1859.4262634513013);
        f6d.exact_altitudes = false;
        f6d.const_rpm_alt = 3000.0;
        f6d.const_rpm_power = 1900.0;
        check("f6dr_8000w", power_at_altitude_advanced(&f6d, 8000.0, true, 0.0, false, 15.0), 1859.4262634513013);

        // wepCritAlt == critAlt, 高于双临界 (无 constRPM → !constRpmBelowCritAlt 分支)
        let mut a = CompressorStageParams::default();
        a.crit_alt = 7000.0;
        a.crit_power = 2000.0;
        a.deck_power = 1850.0;
        a.wep_crit_alt = 7000.0;
        a.wep_power_mult = 1.15;
        a.old_altitude = 7000.0;
        a.old_power = 2000.0;
        a.old_power_new_rpm = 2000.0;
        check("a_8000w", power_at_altitude_advanced(&a, 8000.0, true, 0.0, false, 15.0), 1994.1079618146214);
        a.exact_altitudes = true;
        check("ae_8000w", power_at_altitude_advanced(&a, 8000.0, true, 0.0, false, 15.0), 1994.1079618146214);
        a.exact_altitudes = false;
        a.ceiling_alt = 11000.0;
        a.ceiling_power = 700.0;
        check("ac_8000w", power_at_altitude_advanced(&a, 8000.0, true, 0.0, false, 15.0), 1_825.875_275_812_053);
        a.exact_altitudes = true;
        check("ace_8000w", power_at_altitude_advanced(&a, 8000.0, true, 0.0, false, 15.0), 1856.9897108368868);
        a.exact_altitudes = false;
        a.curvature = 1.5;
        a.const_rpm_alt = 7000.0;
        a.const_rpm_power = 1700.0;
        check("acr_8000w", power_at_altitude_advanced(&a, 8000.0, true, 0.0, false, 15.0), 2041.9054028586359);
        a.exact_altitudes = true;
        check("acre_8000w", power_at_altitude_advanced(&a, 8000.0, true, 0.0, false, 15.0), 2041.9054028586359);

        // === Part F7: constRpmAboveCritAlt (P-63 弯折高于临界) ===
        let mut f7 = CompressorStageParams::default();
        f7.crit_alt = 5000.0;
        f7.crit_power = 1800.0;
        f7.deck_power = 1750.0;
        f7.curvature = 1.5;
        f7.wep_crit_alt = 4500.0;
        f7.wep_power_mult = 1.1;
        f7.old_altitude = 5000.0;
        f7.old_power = 1800.0;
        f7.old_power_new_rpm = 1800.0;
        f7.ceiling_alt = 10000.0;
        f7.ceiling_power = 800.0;
        f7.const_rpm_alt = 5000.0;
        f7.const_rpm_power = 1700.0;
        check("f7_5200m", power_at_altitude_advanced(&f7, 5200.0, false, 0.0, false, 15.0), 1788.2179935134372);
        check("f7_6000m", power_at_altitude_advanced(&f7, 6000.0, false, 0.0, false, 15.0), 1676.5473017913841);
        f7.exact_altitudes = true;
        check("f7e_5200m", power_at_altitude_advanced(&f7, 5200.0, false, 0.0, false, 15.0), 1788.2179935134372);
        check("f7e_6000m", power_at_altitude_advanced(&f7, 6000.0, false, 0.0, false, 15.0), 1676.5473017913841);

        // === Part F8: 无 ceiling 高空衰减 (military) ===
        let mut f8 = CompressorStageParams::default();
        f8.crit_alt = 5000.0;
        f8.crit_power = 1500.0;
        f8.deck_power = 1400.0;
        f8.wep_crit_alt = 4500.0;
        f8.wep_power_mult = 1.1;
        f8.old_altitude = 5000.0;
        f8.old_power = 1500.0;
        f8.old_power_new_rpm = 1500.0;
        check("f8_9000m", power_at_altitude_advanced(&f8, 9000.0, false, 0.0, false, 15.0), 853.641_937_572_037_8);
        check("f8_5200m", power_at_altitude_advanced(&f8, 5200.0, false, 0.0, false, 15.0), 1_460.341_572_137_396);

        // === RAM 使 effectiveAlt 落入 WEP 低于临界分支 ===
        let mut c = CompressorStageParams::default();
        c.crit_alt = 7000.0;
        c.crit_power = 2000.0;
        c.deck_power = 1850.0;
        c.wep_crit_alt = 6000.0;
        c.wep_power_mult = 1.15;
        c.speed_manifold_mult = 0.9;
        c.old_altitude = 7000.0;
        c.old_power = 2000.0;
        c.old_power_new_rpm = 2000.0;
        check("c_6500w_tas600", power_at_altitude_advanced(&c, 6500.0, true, 600.0, false, 15.0), 2_285.179_163_632_901);
        check("c_6500w_ias600", power_at_altitude_advanced(&c, 6500.0, true, 600.0, true, 15.0), 2261.2387414765312);
        check("c_6500w_ias600_t30", power_at_altitude_advanced(&c, 6500.0, true, 600.0, true, 30.0), 2261.2387414765312);

        // === Part G: peakWepPower ===
        let mut pk = CompressorStageParams::default();
        pk.crit_alt = 5000.0;
        pk.crit_power = 1500.0;
        pk.deck_power = 1400.0;
        pk.wep_crit_alt = 4500.0;
        pk.wep_power_mult = 1.15;
        pk.old_altitude = 5000.0;
        pk.old_power = 1500.0;
        pk.old_power_new_rpm = 1500.0;
        let pk_arr = [pk];
        check("pk_single", peak_wep_power(&pk_arr), 1724.9999999999998);
        check("pk_multi", peak_wep_power(&two), 1540.0000000000002);
        check("pk_empty", peak_wep_power(&[]), 0.0);
        let mut nw = CompressorStageParams::default();
        nw.crit_alt = 3000.0;
        nw.crit_power = 1200.0;
        nw.deck_power = 1150.0;
        nw.wep_crit_alt = 3000.0;
        nw.wep_power_mult = 1.0;
        nw.old_altitude = 3000.0;
        nw.old_power = 1200.0;
        nw.old_power_new_rpm = 1200.0;
        let nw_arr = [nw];
        check("pk_nowep", peak_wep_power(&nw_arr), 1200.0);

        // toString 对拍 (Java String.format %.0f/%.2f 输出)
        assert_eq!(
            pk.to_string(),
            "Stage0[critAlt=5000m, critPower=1500hp, deckPower=1400hp, wepMult=1.15]"
        );
        assert_eq!(
            s2.to_string(),
            "Stage1[critAlt=6500m, critPower=1300hp, deckPower=1100hp, wepMult=1.10]"
        );
    }

    #[test]
    fn run_test_torque_rpm_boost() {
        test_torque_rpm_boost();
    }

    #[test]
    fn run_test_torque_from_hp() {
        test_torque_from_hp();
    }

    #[test]
    fn run_test_supercharger_rpm_effect() {
        test_supercharger_rpm_effect();
    }

    #[test]
    fn run_test_interpolate_power() {
        test_interpolate_power();
    }

    #[test]
    fn run_test_wep_power_multiplier() {
        test_wep_power_multiplier();
    }

    #[test]
    fn run_test_wep_critical_altitude() {
        test_wep_critical_altitude();
    }

    #[test]
    fn run_test_power_at_altitude() {
        test_power_at_altitude();
    }

    #[test]
    fn run_test_multi_stage_supercharger() {
        test_multi_stage_supercharger();
    }

    #[test]
    fn run_test_generate_power_curve() {
        test_generate_power_curve();
    }

    #[test]
    fn run_test_ram_effect_integration() {
        test_ram_effect_integration();
    }

    #[test]
    fn run_test_no_wep_aircraft_identical_curves() {
        test_no_wep_aircraft_identical_curves();
    }

    #[test]
    fn run_test_peak_wep_power() {
        test_peak_wep_power();
    }
}
