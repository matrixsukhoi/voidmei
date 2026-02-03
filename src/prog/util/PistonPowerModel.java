package prog.util;

import static prog.util.AtmosphereModel.*;

/**
 * Piston engine power curve calculation model.
 *
 * <p>Ported from: wt-aircraft-performance-calculator/plane_power_calculator.py
 *
 * <p>This model calculates engine power output at any altitude by considering:
 * <ul>
 *   <li>Supercharger staging and critical altitudes</li>
 *   <li>WEP (War Emergency Power) boost effects</li>
 *   <li>RPM effects on torque and supercharger efficiency</li>
 *   <li>RAM air effect from forward motion</li>
 * </ul>
 *
 * <h3>Physical Background</h3>
 *
 * <p><b>Supercharger Critical Altitude:</b> The altitude up to which the supercharger
 * can maintain rated manifold pressure. Above this altitude, power drops proportionally
 * with ambient pressure.
 *
 * <p><b>Torque Curve Model:</b> Engine torque follows an inverted parabola with
 * peak torque at approximately 75% of maximum RPM. This affects how power scales
 * with RPM changes between military and WEP settings.
 *
 * <p><b>Multi-Stage Superchargers:</b> Some engines have 2-3 supercharger speeds.
 * Each stage is optimized for a different altitude band. The model selects the
 * stage providing maximum power at each altitude.
 *
 * <h3>Accuracy</h3>
 * <p>The original Python implementation claims ±1% accuracy for 95%+ of aircraft
 * when compared against War Thunder's actual flight model calculations.
 */
public final class PistonPowerModel {

    private PistonPowerModel() {}

    // ==================== Torque/RPM Calculations ====================

    /**
     * Calculates the power multiplier due to RPM change.
     *
     * <p>Based on a parabolic torque curve model where maximum torque occurs
     * at 75% of the higher RPM value. The torque curve is:
     * <pre>τ = -rpm² + 2×b×rpm</pre>
     * where b = 0.75 × higherRPM
     *
     * <p>This models real piston engine behavior where torque peaks at
     * moderate RPM and falls off at both low and high RPM extremes.
     *
     * @param lowerRPM  lower RPM value (e.g., military power RPM)
     * @param higherRPM higher RPM value (e.g., WEP RPM)
     * @return power ratio (higherRPM power / lowerRPM power), typically 1.0-1.15
     */
    public static double torqueRpmBoost(double lowerRPM, double higherRPM) {
        if (lowerRPM <= 0 || higherRPM <= 0) {
            return 1.0;
        }

        // Peak torque occurs at 75% of higher RPM
        double torqueMaxRPM = 0.75 * higherRPM;

        // Torque at each RPM: τ = rpm × (2×b×rpm - rpm²) = rpm × τ_curve
        // Power = torque × rpm, so we compute rpm × (2b - rpm) × rpm
        double highTerm = higherRPM * (2 * torqueMaxRPM * higherRPM - higherRPM * higherRPM);
        double lowTerm = lowerRPM * (2 * torqueMaxRPM * lowerRPM - lowerRPM * lowerRPM);

        if (lowTerm <= 0) {
            return 1.0;
        }
        return highTerm / lowTerm;
    }

    /**
     * Calculates propeller shaft torque from horsepower.
     *
     * <p>Derived from: P = τ × ω = τ × (2π × RPM / 60)
     * <p>Rearranged: τ = P × 60 / (2π × RPM)
     * <p>With unit conversion (hp to W, N·m to kgf·m): τ = 726.115 × P / RPM
     *
     * @param powerHp   engine power in horsepower
     * @param reductRPM propeller RPM after reduction gear
     * @return torque in kgf·m
     */
    public static double torqueFromHp(double powerHp, double reductRPM) {
        if (reductRPM <= 0) {
            return 0;
        }
        return 726.115 * powerHp / reductRPM;
    }

    // ==================== Supercharger Efficiency ====================

    /**
     * Calculates the supercharger efficiency boost from increased RPM.
     *
     * <p>The supercharger is mechanically driven by the engine crankshaft.
     * Higher RPM means the supercharger spins faster, producing more boost.
     * This effect is non-linear due to compressor characteristics.
     *
     * <p>The formula models this relationship:
     * <pre>
     * effect = (1 + (1 - pressureAtRPM0) / milRPM × (wepRPM - milRPM))^(1 + omegaFactorSq)
     * </pre>
     *
     * @param militaryRPM              military power RPM
     * @param wepRPM                   WEP mode RPM
     * @param compressorPressureAtRPM0 supercharger pressure ratio at RPM=0 (typically 0.1-0.3)
     * @param compressorOmegaFactorSq  supercharger angular velocity factor squared
     * @return supercharger efficiency multiplier, typically 1.0-1.3
     */
    public static double superchargerRpmEffect(double militaryRPM, double wepRPM,
                                               double compressorPressureAtRPM0,
                                               double compressorOmegaFactorSq) {
        if (militaryRPM <= 0) {
            return 1.0;
        }

        double rpmDiff = wepRPM - militaryRPM;
        double base = 1.0 + ((1.0 - compressorPressureAtRPM0) / militaryRPM) * rpmDiff;
        return Math.pow(base, 1.0 + compressorOmegaFactorSq);
    }

    // ==================== Power Interpolation ====================

    /**
     * Interpolates power between two altitude points.
     *
     * <p>This is the core interpolation formula used throughout the model.
     * Power varies with altitude based on a pressure ratio raised to a
     * curvature exponent:
     *
     * <pre>
     * power = P_lower + ΔP × |((p_target - p_lower) / (p_higher - p_lower))|^curvature
     * </pre>
     *
     * <p>The curvature parameter (typically 1.0) controls how quickly power
     * transitions between the two reference points.
     *
     * @param higherPower power at the higher altitude point (hp)
     * @param higherAlt   higher altitude (m)
     * @param lowerPower  power at the lower altitude point (hp)
     * @param lowerAlt    lower altitude (m)
     * @param targetAlt   target altitude for interpolation (m)
     * @param curvature   interpolation curvature (typically 1.0)
     * @return interpolated power at target altitude (hp)
     */
    public static double interpolatePower(double higherPower, double higherAlt,
                                          double lowerPower, double lowerAlt,
                                          double targetAlt, double curvature) {
        double pTarget = pressure(targetAlt);
        double pLower = pressure(lowerAlt);
        double pHigher = pressure(higherAlt);

        double pDenom = pHigher - pLower;
        if (Math.abs(pDenom) < 1e-9) {
            return lowerPower;
        }

        // Determine power difference based on altitude direction
        double powerDiff;
        if (targetAlt >= lowerAlt) {
            powerDiff = higherPower - lowerPower;
        } else {
            powerDiff = lowerPower - higherPower;
        }

        double ratio = Math.abs((pTarget - pLower) / pDenom);
        return lowerPower + powerDiff * Math.pow(ratio, curvature);
    }

    // ==================== WEP Calculations ====================

    /**
     * Calculates the total WEP power multiplier.
     *
     * <p>WEP (War Emergency Power) combines several boost mechanisms:
     * <ul>
     *   <li>Afterburner boost (higher fuel flow, ADI injection, etc.)</li>
     *   <li>Throttle boost (over-boost manifold pressure)</li>
     *   <li>RPM increase effect on torque curve</li>
     *   <li>Octane rating modifications (fuel quality upgrades)</li>
     * </ul>
     *
     * @param afterburnerBoost      base afterburner boost factor (FM: AfterburnerBoost)
     * @param throttleBoost         throttle boost factor (FM: ThrottleBoost, usually 1.0)
     * @param afterburnerBoostMul   stage-specific boost multiplier (FM: AfterburnerBoostMul)
     * @param octaneAfterburnerMult octane rating correction (fuel upgrade effect)
     * @param militaryRPM           military power RPM
     * @param wepRPM                WEP mode RPM
     * @return total WEP power multiplier
     */
    public static double wepPowerMultiplier(double afterburnerBoost, double throttleBoost,
                                            double afterburnerBoostMul, double octaneAfterburnerMult,
                                            double militaryRPM, double wepRPM) {
        // Boost effect modified by octane rating
        double boostEffect = 1.0 + (afterburnerBoost - 1.0) * octaneAfterburnerMult;

        // RPM effect on torque/power
        double rpmBoost = torqueRpmBoost(militaryRPM, wepRPM);

        return boostEffect * throttleBoost * afterburnerBoostMul * rpmBoost;
    }

    /**
     * Calculates the WEP critical altitude.
     *
     * <p>WEP mode typically has a higher manifold pressure than military power.
     * This higher pressure can only be maintained up to a lower altitude
     * (the WEP critical altitude). Above this, WEP power drops but may still
     * exceed military power due to the higher base multiplier.
     *
     * <p>The calculation determines where the supercharger can no longer
     * maintain the WEP manifold pressure based on:
     * <ul>
     *   <li>Military critical altitude and manifold pressure</li>
     *   <li>WEP manifold pressure requirement</li>
     *   <li>RPM effect on supercharger efficiency</li>
     *   <li>Afterburner pressure boost factor</li>
     * </ul>
     *
     * @param militaryCritAlt          military mode critical altitude (m)
     * @param militaryMP               military mode manifold pressure (ata)
     * @param wepMP                    WEP mode manifold pressure (ata)
     * @param superchargerRpmEffect    supercharger RPM efficiency multiplier
     * @param afterburnerPressureBoost afterburner pressure boost (FM: AfterburnerPressureBoost)
     * @return WEP critical altitude (m)
     */
    public static double wepCriticalAltitude(double militaryCritAlt, double militaryMP, double wepMP,
                                             double superchargerRpmEffect, double afterburnerPressureBoost) {
        // Calculate supercharger "strength" at military critical altitude
        double critPressure = pressure(militaryCritAlt);
        double superchargerStrength = militaryMP / critPressure;

        // WEP mode supercharger strength is boosted
        double wepSuperchargerStrength = superchargerStrength * superchargerRpmEffect * afterburnerPressureBoost;

        // Find altitude where ambient pressure matches WEP requirement
        double wepCritPressure = wepMP / wepSuperchargerStrength;
        return altitudeAtPressure(wepCritPressure);
    }

    // ==================== Main Calculation Methods ====================

    /**
     * Calculates engine power at a specific altitude for one supercharger stage.
     *
     * <p>The power curve has three regions:
     * <ol>
     *   <li><b>Below deck altitude:</b> Power interpolates from deck to critical altitude</li>
     *   <li><b>Between deck and critical:</b> Power may increase slightly due to
     *       reduced exhaust back-pressure and optimized mixture</li>
     *   <li><b>Above critical altitude:</b> Power drops proportionally with ambient pressure</li>
     * </ol>
     *
     * @param params        supercharger stage parameters
     * @param altitudeM     target altitude (m)
     * @param isWep         true for WEP mode, false for military power
     * @param speedKmh      aircraft speed for RAM effect (km/h), 0 to ignore
     * @param isIAS         true if speed is IAS, false if TAS
     * @param seaLevelTempC sea level temperature (°C), use 15 for ISA standard
     * @return engine power at altitude (hp)
     */
    public static double powerAtAltitude(CompressorStageParams params, double altitudeM,
                                         boolean isWep, double speedKmh, boolean isIAS,
                                         double seaLevelTempC) {
        // Apply RAM effect to get effective altitude
        double effectiveAlt = altitudeM;
        if (speedKmh > 0 && params.speedManifoldMult > 0) {
            effectiveAlt = ramEffectAltitude(altitudeM, seaLevelTempC, speedKmh, isIAS, params.speedManifoldMult);
        }

        // Select power parameters based on mode
        double powerMult = isWep ? params.wepPowerMult : 1.0;
        double critAlt = isWep ? params.wepCritAlt : params.critAlt;
        double critPower = params.critPower * powerMult;
        double deckPower = params.deckPower * powerMult;
        double deckAlt = params.deckAlt;

        // Below or at critical altitude: interpolate from deck
        if (effectiveAlt <= critAlt) {
            return interpolatePower(critPower, critAlt, deckPower, deckAlt, effectiveAlt, params.curvature);
        }

        // Above critical altitude: power drops with pressure
        double pAtCrit = pressure(critAlt);
        double pAtAlt = pressure(effectiveAlt);
        return critPower * (pAtAlt / pAtCrit);
    }

    /**
     * Calculates optimal power from multiple supercharger stages.
     *
     * <p>Multi-stage superchargers have different gear ratios optimized for
     * different altitude bands. This method evaluates all stages and returns
     * the maximum available power.
     *
     * <p>Typically:
     * <ul>
     *   <li>Stage 1 (low gear): Best power at low altitude</li>
     *   <li>Stage 2 (high gear): Best power at medium altitude</li>
     *   <li>Stage 3 (if present): Best power at high altitude</li>
     * </ul>
     *
     * @param stages        array of supercharger stage parameters
     * @param altitudeM     target altitude (m)
     * @param isWep         true for WEP mode
     * @param speedKmh      aircraft speed for RAM effect (km/h)
     * @param isIAS         true if speed is IAS
     * @param seaLevelTempC sea level temperature (°C)
     * @return maximum available power from any stage (hp)
     */
    public static double optimalPowerAtAltitude(CompressorStageParams[] stages, double altitudeM,
                                                boolean isWep, double speedKmh, boolean isIAS,
                                                double seaLevelTempC) {
        if (stages == null || stages.length == 0) {
            return 0;
        }

        double maxPower = 0;
        for (CompressorStageParams stage : stages) {
            double power = powerAtAltitude(stage, altitudeM, isWep, speedKmh, isIAS, seaLevelTempC);
            if (power > maxPower) {
                maxPower = power;
            }
        }
        return maxPower;
    }

    /**
     * Generates a complete power curve from -4000m to 20000m.
     *
     * <p>This produces an array suitable for display or export, matching
     * the format used by wt-aircraft-performance-calculator.
     *
     * @param stages        supercharger stage parameters
     * @param isWep         true for WEP mode
     * @param speedKmh      aircraft speed for RAM effect (0 for static)
     * @param isIAS         true if speed is IAS
     * @param seaLevelTempC sea level temperature (°C)
     * @param altStep       altitude step in meters (recommend 100)
     * @return power array where index i corresponds to altitude (-4000 + i × altStep)
     */
    public static double[] generatePowerCurve(CompressorStageParams[] stages, boolean isWep,
                                              double speedKmh, boolean isIAS, double seaLevelTempC,
                                              int altStep) {
        int minAlt = -4000;
        int maxAlt = 20000;
        int count = (maxAlt - minAlt) / altStep + 1;
        double[] curve = new double[count];

        for (int i = 0; i < count; i++) {
            double alt = minAlt + i * altStep;
            curve[i] = optimalPowerAtAltitude(stages, alt, isWep, speedKmh, isIAS, seaLevelTempC);
        }
        return curve;
    }

    /**
     * Finds the critical altitude where power begins to drop.
     *
     * <p>Searches the power curve to find the altitude with maximum power.
     * This is useful for determining the effective ceiling of each
     * supercharger stage.
     *
     * @param stages        supercharger stage parameters
     * @param isWep         true for WEP mode
     * @param seaLevelTempC sea level temperature (°C)
     * @return altitude of peak power (m)
     */
    public static double findPeakPowerAltitude(CompressorStageParams[] stages, boolean isWep,
                                               double seaLevelTempC) {
        double maxPower = 0;
        double peakAlt = 0;

        // Search from sea level to 15000m in 100m increments
        for (int alt = 0; alt <= 15000; alt += 100) {
            double power = optimalPowerAtAltitude(stages, alt, isWep, 0, false, seaLevelTempC);
            if (power > maxPower) {
                maxPower = power;
                peakAlt = alt;
            }
        }
        return peakAlt;
    }

    // ==================== Parameter Data Class ====================

    /**
     * Parameters for a single supercharger stage.
     *
     * <p>These values are extracted from War Thunder FM (Flight Model) files.
     * Each supercharger stage has its own critical altitude and power curve.
     *
     * <h3>Key Concepts</h3>
     * <dl>
     *   <dt>Critical Altitude</dt>
     *   <dd>The altitude up to which the supercharger can maintain rated
     *       manifold pressure. Power is relatively constant below this point.</dd>
     *
     *   <dt>Deck Power</dt>
     *   <dd>Power output at sea level (or "deck altitude"). May be slightly
     *       less than critical altitude power due to exhaust back-pressure.</dd>
     *
     *   <dt>WEP Critical Altitude</dt>
     *   <dd>Usually lower than military critical altitude because WEP demands
     *       higher manifold pressure that the supercharger cannot maintain as high.</dd>
     * </dl>
     */
    public static class CompressorStageParams {

        /** Critical altitude in meters - altitude where power starts dropping */
        public double critAlt;

        /** Power at critical altitude in horsepower */
        public double critPower;

        /** Sea level (deck) power in horsepower */
        public double deckPower;

        /** Deck altitude in meters (usually 0) */
        public double deckAlt = 0;

        /** Power curve curvature coefficient (typically 1.0) */
        public double curvature = 1.0;

        /** WEP critical altitude in meters */
        public double wepCritAlt;

        /** WEP power multiplier relative to military power */
        public double wepPowerMult = 1.0;

        /** RAM effect coefficient (FM: SpeedManifoldMultiplier) */
        public double speedManifoldMult = 1.0;

        /** Stage index (0 = first stage, 1 = second stage, etc.) */
        public int stageIndex;

        /**
         * Creates an empty parameter set.
         */
        public CompressorStageParams() {}

        /**
         * Creates a parameter set with basic values.
         *
         * @param critAlt   critical altitude (m)
         * @param critPower power at critical altitude (hp)
         * @param deckPower sea level power (hp)
         */
        public CompressorStageParams(double critAlt, double critPower, double deckPower) {
            this.critAlt = critAlt;
            this.critPower = critPower;
            this.deckPower = deckPower;
            this.wepCritAlt = critAlt;
        }

        /**
         * Creates a complete parameter set.
         *
         * @param critAlt           critical altitude (m)
         * @param critPower         power at critical altitude (hp)
         * @param deckPower         sea level power (hp)
         * @param wepCritAlt        WEP critical altitude (m)
         * @param wepPowerMult      WEP power multiplier
         * @param speedManifoldMult RAM effect coefficient
         * @param stageIndex        supercharger stage index
         */
        public CompressorStageParams(double critAlt, double critPower, double deckPower,
                                     double wepCritAlt, double wepPowerMult,
                                     double speedManifoldMult, int stageIndex) {
            this.critAlt = critAlt;
            this.critPower = critPower;
            this.deckPower = deckPower;
            this.wepCritAlt = wepCritAlt;
            this.wepPowerMult = wepPowerMult;
            this.speedManifoldMult = speedManifoldMult;
            this.stageIndex = stageIndex;
        }

        @Override
        public String toString() {
            return String.format("Stage%d[critAlt=%.0fm, critPower=%.0fhp, deckPower=%.0fhp, wepMult=%.2f]",
                    stageIndex, critAlt, critPower, deckPower, wepPowerMult);
        }
    }
}
