//! Helper functions for determining power curve shape based on FM parameters.
//!
//! <p>Ported from WAPC plane_power_calculator.py helper functions:
//! ConstRPM_is, ConstRPM_bends_below_critalt, Ceiling_is_useful, etc.
//!
//! <p>These functions determine which branch of the power curve calculation
//! to use based on the relationship between various FM parameters.
//!
//! 对应 Java: `src/prog/util/PowerCurveHelper.java` (一比一翻译)

// PORT: Java `private PowerCurveHelper() {}` (final 工具类, 私有构造器防实例化)
// → Rust 自由函数模块无实例化概念, 天然满足

use crate::piston_power_model::CompressorStageParams;

/// Checks if the stage has ConstRPM parameters defined.
///
/// <p>Note: constRpmAlt=0 is a valid value (ConstRPM at sea level), so we only
/// check constRpmPower. WAPC's ConstRPM_is() checks key existence, not altitude value.
/// When FM doesn't define ConstRPM, both constRpmAlt and constRpmPower default to 0.
pub fn has_const_rpm(p: &CompressorStageParams) -> bool {
    p.const_rpm_power > 0.0
}

/// ConstRPM bend point is below the critical altitude.
/// This creates a two-segment curve below crit alt: deck→constRPM then constRPM→crit.
pub fn const_rpm_below_crit_alt(p: &CompressorStageParams) -> bool {
    has_const_rpm(p) && (p.const_rpm_alt - p.crit_alt) < -1.0
}

/// ConstRPM bend point is below the original (pre-adjustment) critical altitude.
pub fn const_rpm_below_old_crit_alt(p: &CompressorStageParams) -> bool {
    has_const_rpm(p) && (p.const_rpm_alt - p.old_altitude) < -1.0
}

/// ConstRPM bend point is below the WEP critical altitude.
pub fn const_rpm_below_wep_crit_alt(p: &CompressorStageParams) -> bool {
    has_const_rpm(p) && (p.const_rpm_alt - p.wep_crit_alt) < -1.0
}

/// ConstRPM bends above critical altitude — used with ceiling parameters
/// to create a curved decay above crit alt (e.g., P-63).
pub fn const_rpm_above_crit_alt(p: &CompressorStageParams) -> bool {
    has_const_rpm(p)
        && p.const_rpm_alt == p.crit_alt
        && p.crit_power - p.ceiling_power > 1.0
        && p.curvature > 1.0
}

/// ConstRPM altitude is at or below sea level.
pub fn const_rpm_below_deck(p: &CompressorStageParams) -> bool {
    has_const_rpm(p) && p.const_rpm_alt <= 0.0
}

/// Checks if ceiling parameters exist.
pub fn has_ceiling(p: &CompressorStageParams) -> bool {
    p.ceiling_alt > 0.0 && p.ceiling_power > 0.0
}

/// Ceiling parameters are meaningful — altitude gap and power gap are both
/// significant enough to affect the curve shape.
///
/// <p>Uses the original FM altitude/power (before definition_alt_power_adjuster)
/// to match WAPC's Ceiling_is_useful() which compares against Altitude[i] / Power[i],
/// not the adjusted critAlt / critPower.
pub fn ceiling_is_useful(p: &CompressorStageParams) -> bool {
    let reference_alt = if p.old_altitude > 0.0 { p.old_altitude } else { p.crit_alt };
    let reference_power = if p.old_power > 0.0 { p.old_power } else { p.crit_power };
    has_ceiling(p)
        && (p.ceiling_alt - reference_alt) >= 2.0
        && (reference_power - p.ceiling_power) >= 2.0
}

/// Critical altitude equals deck altitude — the power curve is flat from sea level.
/// Deck→crit interpolation is skipped; curve goes directly to ceiling.
pub fn power_is_deck_power(p: &CompressorStageParams) -> bool {
    (p.crit_alt - p.deck_alt).abs() < 1.0
}

#[cfg(test)]
mod tests;
