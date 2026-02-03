package prog.util;

/**
 * Physical constants used throughout the application.
 * Centralizes physics-related constants for consistency and maintainability.
 */
public final class PhysicsConstants {

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

    // Prevent instantiation
    private PhysicsConstants() {}
}
