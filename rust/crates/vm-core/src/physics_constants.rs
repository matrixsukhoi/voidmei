//! Physical constants used throughout the application.
//! Centralizes physics-related constants for consistency and maintainability.
//!
//! 对应 Java: `prog.util.PhysicsConstants` (final 类, 纯常量容器)。
//! PORT: Java 用 private 构造器阻止实例化——Rust 模块即命名空间,
//! 不定义 struct 天然无法实例化, 原语义由模块边界保真。
//! PORT: lib.rs 已收敛为 `pub use physics_constants::{g, G};` 转发 (主 agent P1 裁决)。

// === Gravitational Constants ===

/// Standard gravitational acceleration (m/s²).
/// Using 9.80 as the standard value for flight simulation calculations.
pub const G: f64 = 9.80;

/// Alias for gravitational acceleration, matching physics notation.
/// Use this in formulas where lowercase 'g' is conventional.
// PORT: Java 允许小写字段名 `g`; Rust 常量规范要求大写, 这里为保持
// 公式书写惯例 (E = v²/2g) 原样保留小写, 仅抑制命名 lint。
#[allow(non_upper_case_globals)]
pub const g: f64 = G;

// === Atmospheric Model Constants (ISA - International Standard Atmosphere) ===

/// Pressure-altitude coefficient (1/m).
/// Used in the barometric formula: P = P₀ × (1 - PRESSURE_ALTITUDE_COEFF × h)^PRESSURE_ALTITUDE_EXP
pub const PRESSURE_ALTITUDE_COEFF: f64 = 0.0000225577;

/// Pressure-altitude exponent.
/// Derived from: g / (R × L) where L is the temperature lapse rate.
pub const PRESSURE_ALTITUDE_EXP: f64 = 5.25588;

/// Tropospheric temperature lapse rate (K/m).
/// Temperature decreases by 6.5°C per 1000m of altitude gain.
pub const TEMP_LAPSE_RATE: f64 = 0.0065;

/// Sea level standard atmospheric pressure (Pa).
pub const SEA_LEVEL_PRESSURE: f64 = 101325.0;

/// Sea level standard air density (kg/m³).
pub const SEA_LEVEL_DENSITY: f64 = 1.225;

/// Specific gas constant for dry air (J/(kg·K)).
/// R_specific = R_universal / M_air ≈ 8314.5 / 28.97
pub const R_SPECIFIC_AIR: f64 = 287.0500676;

/// Kelvin zero point offset (°C → K conversion).
/// T(K) = T(°C) + KELVIN_OFFSET
pub const KELVIN_OFFSET: f64 = 273.15;

// PORT: Java `private PhysicsConstants() {}` (Prevent instantiation)
// 在 Rust 中无对应物也不需要——本模块只含常量, 无实例化入口。

#[cfg(test)]
mod tests {
    use super::*;

    // 断言值 = Java 字面量原值; 同一十进制字面量在 Java/Rust 解析为
    // 完全相同的 IEEE 754 f64 位模式, 故可安全用精确相等。
    #[test]
    fn test_gravitational_constants() {
        assert_eq!(G, 9.80);
        assert_eq!(g, 9.80);
        // Java `g = G` 别名语义: 两者恒等
        assert_eq!(g, G);
    }

    #[test]
    fn test_isa_constants() {
        assert_eq!(PRESSURE_ALTITUDE_COEFF, 0.0000225577);
        assert_eq!(PRESSURE_ALTITUDE_EXP, 5.25588);
        assert_eq!(TEMP_LAPSE_RATE, 0.0065);
    }

    #[test]
    fn test_sea_level_constants() {
        assert_eq!(SEA_LEVEL_PRESSURE, 101325.0);
        assert_eq!(SEA_LEVEL_DENSITY, 1.225);
        assert_eq!(R_SPECIFIC_AIR, 287.0500676);
        assert_eq!(KELVIN_OFFSET, 273.15);
    }
}
