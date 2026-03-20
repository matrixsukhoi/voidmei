package prog.util;

/**
 * Physical constants used throughout the application.
 * Centralizes physics-related constants for consistency and maintainability.
 */
public final class PhysicsConstants {

    // === Gravitational Constants ===

    /**
     * Standard gravitational acceleration (m/s²).
     * Using 9.80 as the standard value for flight simulation calculations.
     */
    public static final double G = 9.80;

    /**
     * Alias for gravitational acceleration, matching physics notation.
     * Use this in formulas where lowercase 'g' is conventional.
     */
    public static final double g = G;

    // === Atmospheric Model Constants (ISA - International Standard Atmosphere) ===

    /**
     * Pressure-altitude coefficient (1/m).
     * Used in the barometric formula: P = P₀ × (1 - PRESSURE_ALTITUDE_COEFF × h)^PRESSURE_ALTITUDE_EXP
     */
    public static final double PRESSURE_ALTITUDE_COEFF = 0.0000225577;

    /**
     * Pressure-altitude exponent.
     * Derived from: g / (R × L) where L is the temperature lapse rate.
     */
    public static final double PRESSURE_ALTITUDE_EXP = 5.25588;

    /**
     * Tropospheric temperature lapse rate (K/m).
     * Temperature decreases by 6.5°C per 1000m of altitude gain.
     */
    public static final double TEMP_LAPSE_RATE = 0.0065;

    /**
     * Sea level standard atmospheric pressure (Pa).
     */
    public static final double SEA_LEVEL_PRESSURE = 101325.0;

    /**
     * Sea level standard air density (kg/m³).
     */
    public static final double SEA_LEVEL_DENSITY = 1.225;

    /**
     * Specific gas constant for dry air (J/(kg·K)).
     * R_specific = R_universal / M_air ≈ 8314.5 / 28.97
     */
    public static final double R_SPECIFIC_AIR = 287.0500676;

    /**
     * Kelvin zero point offset (°C → K conversion).
     * T(K) = T(°C) + KELVIN_OFFSET
     */
    public static final double KELVIN_OFFSET = 273.15;

    // Prevent instantiation
    private PhysicsConstants() {}
}
