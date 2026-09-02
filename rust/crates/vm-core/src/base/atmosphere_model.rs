//! Atmospheric model calculation utilities.
//! Based on the International Standard Atmosphere (ISA) model, providing
//! pressure, density, and temperature calculations at various altitudes.
//!
//! Ported from: wt-aircraft-performance-calculator/ram_pressure_density_calculator.py
//!
//! All methods are pure functions, thread-safe, and zero-allocation.
//!
//! Physical Background
//! ==================
//! The ISA model assumes:
//! - Sea level temperature: 15°C (288.15 K)
//! - Sea level pressure: 101325 Pa
//! - Temperature lapse rate: -6.5°C per 1000m (in troposphere)
//! - Tropopause at ~11000m where temperature stabilizes at -56.5°C
//!
//! Key Formulas
//! ------------
//! ```text
//! Pressure ratio:     P/P₀ = (1 - 0.0000225577 × h)^5.25588
//! Density:            ρ = P / (R × T)
//! IAS to TAS:         TAS = IAS × √(ρ₀/ρ)
//! Dynamic pressure:   q = ½ρv²
//! ```
//!
//! 对应 Java: `src/prog/util/AtmosphereModel.java` (一比一翻译)

// PORT: Java `private AtmosphereModel() {}` (final 工具类, 私有构造器防实例化)
// → Rust 自由函数模块无实例化概念, 天然满足

use crate::base::physics_constants::{
    KELVIN_OFFSET, PRESSURE_ALTITUDE_COEFF, PRESSURE_ALTITUDE_EXP, R_SPECIFIC_AIR,
    SEA_LEVEL_DENSITY, SEA_LEVEL_PRESSURE, TEMP_LAPSE_RATE,
};

/// Calculates relative atmospheric pressure at a given altitude.
/// Based on the ISA barometric formula.
///
/// Formula: P/P₀ = (1 - 0.0000225577 × h)^5.25588
///
/// - `altitude_m`: altitude in meters
///
/// Returns relative pressure (sea level = 1.0), range approximately [0, 1.2] for [-4000m, 20000m]
pub fn pressure(altitude_m: f64) -> f64 {
    (1.0 - PRESSURE_ALTITUDE_COEFF * altitude_m).powf(PRESSURE_ALTITUDE_EXP)
}

/// Calculates altitude from relative pressure (inverse of [`pressure`]).
///
/// - `relative_pressure`: relative pressure (0 to ~1.2)
///
/// Returns altitude in meters
pub fn altitude_at_pressure(relative_pressure: f64) -> f64 {
    if relative_pressure <= 0.0 {
        return 20000.0; // Avoid NaN, return approximate stratosphere altitude
    }
    (1.0 - relative_pressure.powf(1.0 / PRESSURE_ALTITUDE_EXP)) / PRESSURE_ALTITUDE_COEFF
}

/// Calculates air density using the ideal gas law.
///
/// Formula: ρ = P_abs / (R × T)
/// where P_abs = relativePressure × 101325 Pa
///
/// - `relative_pressure`: relative pressure (from [`pressure`])
/// - `sea_level_temp_c`: sea level temperature in Celsius (ISA standard: 15°C)
/// - `altitude_m`: altitude in meters
///
/// Returns air density in kg/m³
pub fn density(relative_pressure: f64, sea_level_temp_c: f64, altitude_m: f64) -> f64 {
    let temp_k = KELVIN_OFFSET + sea_level_temp_c - TEMP_LAPSE_RATE * altitude_m;
    SEA_LEVEL_PRESSURE * relative_pressure / (temp_k * R_SPECIFIC_AIR)
}

/// Converts Indicated Airspeed (IAS) to True Airspeed (TAS).
///
/// IAS is what the airspeed indicator shows, based on dynamic pressure.
/// TAS is the actual speed through the air mass.
///
/// Formula: TAS = IAS × √(ρ₀/ρ)
///
/// - `ias_kmh`: indicated airspeed in km/h
/// - `density`: air density in kg/m³
///
/// Returns true airspeed in km/h
pub fn ias_to_tas(ias_kmh: f64, density: f64) -> f64 {
    if density <= 0.0 {
        return ias_kmh;
    }
    ias_kmh * (SEA_LEVEL_DENSITY / density).sqrt()
}

/// Converts True Airspeed (TAS) to Indicated Airspeed (IAS).
///
/// Formula: IAS = TAS × √(ρ/ρ₀)
///
/// - `tas_kmh`: true airspeed in km/h
/// - `density`: air density in kg/m³
///
/// Returns indicated airspeed in km/h
pub fn tas_to_ias(tas_kmh: f64, density: f64) -> f64 {
    if density <= 0.0 {
        return tas_kmh;
    }
    tas_kmh * (density / SEA_LEVEL_DENSITY).sqrt()
}

/// Calculates RAM effect equivalent altitude.
///
/// At high speeds, the intake captures additional dynamic pressure (RAM air),
/// which increases the total pressure available to the supercharger. This is
/// equivalent to flying at a lower altitude in still air.
///
/// The RAM effect is significant for high-speed aircraft with well-designed
/// intakes, providing "free" supercharging from forward motion.
///
/// Formula:
/// ```text
/// q = ½ρv² × speedManifoldMult    (dynamic pressure with intake efficiency)
/// P_total = P_static + q
/// alt_RAM = altitude where P_static equals P_total
/// ```
///
/// - `altitude_m`: actual altitude in meters
/// - `sea_level_temp_c`: sea level temperature in Celsius
/// - `speed_kmh`: aircraft speed in km/h
/// - `is_ias`: true if speedKmh is IAS, false if TAS
/// - `speed_manifold_mult`: RAM coefficient (FM file's SpeedManifoldMultiplier), typically 0.8-1.0
///
/// Returns equivalent altitude in meters (lower than actual due to RAM effect)
pub fn ram_effect_altitude(
    altitude_m: f64,
    sea_level_temp_c: f64,
    speed_kmh: f64,
    is_ias: bool,
    speed_manifold_mult: f64,
) -> f64 {
    if speed_kmh <= 0.0 || speed_manifold_mult <= 0.0 {
        return altitude_m;
    }

    let p = pressure(altitude_m);
    let rho = density(p, sea_level_temp_c, altitude_m);
    // PORT: Java 三目 `isIAS ? iasToTas(...) : speedKmh` 两臂均为 double, 无数值提升
    let tas_kmh = if is_ias { ias_to_tas(speed_kmh, rho) } else { speed_kmh };
    let tas_ms = tas_kmh / 3.6; // Convert km/h to m/s

    // Dynamic pressure: q = ½ρv² (as relative pressure)
    let dynamic_pressure = (0.5 * rho * tas_ms * tas_ms * speed_manifold_mult) / SEA_LEVEL_PRESSURE;
    let total_pressure = p + dynamic_pressure;

    altitude_at_pressure(total_pressure)
}

/// Calculates temperature at a given altitude.
///
/// In the troposphere, temperature decreases linearly with altitude
/// at the lapse rate of 6.5°C per 1000m.
///
/// - `sea_level_temp_c`: sea level temperature in Celsius
/// - `altitude_m`: altitude in meters
///
/// Returns temperature at altitude in Celsius
pub fn temperature_at_altitude(sea_level_temp_c: f64, altitude_m: f64) -> f64 {
    sea_level_temp_c - TEMP_LAPSE_RATE * altitude_m
}

/// Calculates air density at a given altitude using ISA standard temperature.
///
/// Convenience method that uses the ISA standard sea level temperature of 15°C.
///
/// - `altitude_m`: altitude in meters
///
/// Returns air density in kg/m³
pub fn density_at_altitude(altitude_m: f64) -> f64 {
    density(pressure(altitude_m), 15.0, altitude_m)
}

// =====================================================================
// Tests — 对应 Java: test/TestAtmosphereModel.java (一比一移植)
// =====================================================================

#[cfg(test)]
mod tests;
