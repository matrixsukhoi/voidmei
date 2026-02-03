package prog.util;

import static prog.util.PhysicsConstants.*;

/**
 * Atmospheric model calculation utilities.
 * Based on the International Standard Atmosphere (ISA) model, providing
 * pressure, density, and temperature calculations at various altitudes.
 *
 * <p>Ported from: wt-aircraft-performance-calculator/ram_pressure_density_calculator.py
 *
 * <p>All methods are pure functions, thread-safe, and zero-allocation.
 *
 * <h3>Physical Background</h3>
 * <p>The ISA model assumes:
 * <ul>
 *   <li>Sea level temperature: 15°C (288.15 K)</li>
 *   <li>Sea level pressure: 101325 Pa</li>
 *   <li>Temperature lapse rate: -6.5°C per 1000m (in troposphere)</li>
 *   <li>Tropopause at ~11000m where temperature stabilizes at -56.5°C</li>
 * </ul>
 *
 * <h3>Key Formulas</h3>
 * <pre>
 * Pressure ratio:     P/P₀ = (1 - 0.0000225577 × h)^5.25588
 * Density:            ρ = P / (R × T)
 * IAS to TAS:         TAS = IAS × √(ρ₀/ρ)
 * Dynamic pressure:   q = ½ρv²
 * </pre>
 */
public final class AtmosphereModel {

    private AtmosphereModel() {}

    /**
     * Calculates relative atmospheric pressure at a given altitude.
     * Based on the ISA barometric formula.
     *
     * <p>Formula: P/P₀ = (1 - 0.0000225577 × h)^5.25588
     *
     * @param altitudeM altitude in meters
     * @return relative pressure (sea level = 1.0), range approximately [0, 1.2] for [-4000m, 20000m]
     */
    public static double pressure(double altitudeM) {
        return Math.pow(1 - PRESSURE_ALTITUDE_COEFF * altitudeM, PRESSURE_ALTITUDE_EXP);
    }

    /**
     * Calculates altitude from relative pressure (inverse of {@link #pressure(double)}).
     *
     * @param relativePressure relative pressure (0 to ~1.2)
     * @return altitude in meters
     */
    public static double altitudeAtPressure(double relativePressure) {
        if (relativePressure <= 0) {
            return 20000; // Avoid NaN, return approximate stratosphere altitude
        }
        return (1 - Math.pow(relativePressure, 1.0 / PRESSURE_ALTITUDE_EXP)) / PRESSURE_ALTITUDE_COEFF;
    }

    /**
     * Calculates air density using the ideal gas law.
     *
     * <p>Formula: ρ = P_abs / (R × T)
     * <p>where P_abs = relativePressure × 101325 Pa
     *
     * @param relativePressure relative pressure (from {@link #pressure(double)})
     * @param seaLevelTempC    sea level temperature in Celsius (ISA standard: 15°C)
     * @param altitudeM        altitude in meters
     * @return air density in kg/m³
     */
    public static double density(double relativePressure, double seaLevelTempC, double altitudeM) {
        double tempK = KELVIN_OFFSET + seaLevelTempC - TEMP_LAPSE_RATE * altitudeM;
        return SEA_LEVEL_PRESSURE * relativePressure / (tempK * R_SPECIFIC_AIR);
    }

    /**
     * Converts Indicated Airspeed (IAS) to True Airspeed (TAS).
     *
     * <p>IAS is what the airspeed indicator shows, based on dynamic pressure.
     * TAS is the actual speed through the air mass.
     *
     * <p>Formula: TAS = IAS × √(ρ₀/ρ)
     *
     * @param iasKmh  indicated airspeed in km/h
     * @param density air density in kg/m³
     * @return true airspeed in km/h
     */
    public static double iasToTas(double iasKmh, double density) {
        if (density <= 0) {
            return iasKmh;
        }
        return iasKmh * Math.sqrt(SEA_LEVEL_DENSITY / density);
    }

    /**
     * Converts True Airspeed (TAS) to Indicated Airspeed (IAS).
     *
     * <p>Formula: IAS = TAS × √(ρ/ρ₀)
     *
     * @param tasKmh  true airspeed in km/h
     * @param density air density in kg/m³
     * @return indicated airspeed in km/h
     */
    public static double tasToIas(double tasKmh, double density) {
        if (density <= 0) {
            return tasKmh;
        }
        return tasKmh * Math.sqrt(density / SEA_LEVEL_DENSITY);
    }

    /**
     * Calculates RAM effect equivalent altitude.
     *
     * <p>At high speeds, the intake captures additional dynamic pressure (RAM air),
     * which increases the total pressure available to the supercharger. This is
     * equivalent to flying at a lower altitude in still air.
     *
     * <p>The RAM effect is significant for high-speed aircraft with well-designed
     * intakes, providing "free" supercharging from forward motion.
     *
     * <p>Formula:
     * <pre>
     * q = ½ρv² × speedManifoldMult    (dynamic pressure with intake efficiency)
     * P_total = P_static + q
     * alt_RAM = altitude where P_static equals P_total
     * </pre>
     *
     * @param altitudeM         actual altitude in meters
     * @param seaLevelTempC     sea level temperature in Celsius
     * @param speedKmh          aircraft speed in km/h
     * @param isIAS             true if speedKmh is IAS, false if TAS
     * @param speedManifoldMult RAM coefficient (FM file's SpeedManifoldMultiplier), typically 0.8-1.0
     * @return equivalent altitude in meters (lower than actual due to RAM effect)
     */
    public static double ramEffectAltitude(double altitudeM, double seaLevelTempC,
                                           double speedKmh, boolean isIAS,
                                           double speedManifoldMult) {
        if (speedKmh <= 0 || speedManifoldMult <= 0) {
            return altitudeM;
        }

        double p = pressure(altitudeM);
        double rho = density(p, seaLevelTempC, altitudeM);
        double tasKmh = isIAS ? iasToTas(speedKmh, rho) : speedKmh;
        double tasMs = tasKmh / 3.6; // Convert km/h to m/s

        // Dynamic pressure: q = ½ρv² (as relative pressure)
        double dynamicPressure = (0.5 * rho * tasMs * tasMs * speedManifoldMult) / SEA_LEVEL_PRESSURE;
        double totalPressure = p + dynamicPressure;

        return altitudeAtPressure(totalPressure);
    }

    /**
     * Calculates temperature at a given altitude.
     *
     * <p>In the troposphere, temperature decreases linearly with altitude
     * at the lapse rate of 6.5°C per 1000m.
     *
     * @param seaLevelTempC sea level temperature in Celsius
     * @param altitudeM     altitude in meters
     * @return temperature at altitude in Celsius
     */
    public static double temperatureAtAltitude(double seaLevelTempC, double altitudeM) {
        return seaLevelTempC - TEMP_LAPSE_RATE * altitudeM;
    }

    /**
     * Calculates air density at a given altitude using ISA standard temperature.
     *
     * <p>Convenience method that uses the ISA standard sea level temperature of 15°C.
     *
     * @param altitudeM altitude in meters
     * @return air density in kg/m³
     */
    public static double densityAtAltitude(double altitudeM) {
        return density(pressure(altitudeM), 15.0, altitudeM);
    }
}
