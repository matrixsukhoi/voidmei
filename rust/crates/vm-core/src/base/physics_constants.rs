//! Physical constants used throughout the application.
//! Centralizes physics-related constants for consistency and maintainability.
//!
//! 对应 Java: `prog.util.PhysicsConstants` (final 类, 纯常量容器)。
//! Java 用 private 构造器阻止实例化——Rust 模块即命名空间,
//! 不定义 struct 天然无法实例化, 原语义由模块边界保真。
//! lib.rs 已收敛为 `pub use physics_constants::{g, G};` 转发 (主 agent P1 裁决)。

// === Gravitational Constants ===

/// Standard gravitational acceleration (m/s²).
/// Using 9.80 as the standard value for flight simulation calculations.
pub const G: f64 = 9.80;

/// Alias for gravitational acceleration, matching physics notation.
/// Use this in formulas where lowercase 'g' is conventional.
// Java 允许小写字段名 `g`; Rust 常量规范要求大写, 这里为保持
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

// Java `private PhysicsConstants() {}` (Prevent instantiation)
// 在 Rust 中无对应物也不需要——本模块只含常量, 无实例化入口。

#[cfg(test)]
mod tests;
