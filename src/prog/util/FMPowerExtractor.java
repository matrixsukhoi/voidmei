package prog.util;

import parser.Blkx;
import prog.util.PistonPowerModel.CompressorStageParams;
import static prog.util.AtmosphereModel.*;
import static prog.util.PistonPowerModel.*;

/**
 * Extracts engine parameters from parsed FM (Flight Model) data and converts
 * them to the format required by {@link PistonPowerModel}.
 *
 * <p>This class bridges the gap between Blkx's raw data arrays and the
 * structured CompressorStageParams objects needed for power calculations.
 *
 * <h3>FM File Structure</h3>
 * <p>War Thunder FM files store compressor data in the following fields:
 * <ul>
 *   <li>Compressor.NumSteps - Number of supercharger stages</li>
 *   <li>Compressor.Altitude0/1/2 - Critical altitude per stage (m)</li>
 *   <li>Compressor.Power0/1/2 - Power at critical altitude (hp)</li>
 *   <li>Compressor.Ceiling0/1/2 - Ceiling altitude per stage (m)</li>
 *   <li>Compressor.PowerAtCeiling0/1/2 - Power at ceiling (hp)</li>
 *   <li>Compressor.AfterburnerBoostMul0/1/2 - Per-stage WEP multiplier</li>
 *   <li>Compressor.PowerConstRPMCurvature0/1/2 - Power curve shape</li>
 *   <li>Compressor.SpeedManifoldMultiplier - RAM effect coefficient</li>
 *   <li>EngineType0.Main.AfterburnerBoost - Global WEP boost factor</li>
 *   <li>EngineType0.Main.ThrottleBoost - Throttle boost factor</li>
 *   <li>EngineType0.Main.OctaneAfterburnerMult - Octane upgrade effect</li>
 *   <li>Compressor.ATA0/1/2 - Manifold pressure per stage (ata)</li>
 *   <li>Compressor.AfterburnerPressureBoost0/1/2 - WEP pressure boost</li>
 *   <li>AfterburnerManifoldPressure - WEP manifold pressure (ata)</li>
 *   <li>Main.Power - Sea level rated power (hp)</li>
 * </ul>
 *
 * <h3>WAPC Compatibility</h3>
 * <p>This implementation follows the wt-aircraft-performance-calculator (WAPC)
 * formulas for improved accuracy:
 * <ul>
 *   <li>Deck power: Uses Main.Power or 0.8× previous stage power</li>
 *   <li>WEP multiplier: 4-factor formula (octane, throttle, stage, RPM)</li>
 *   <li>WEP critical altitude: Supercharger pressure model</li>
 * </ul>
 */
public final class FMPowerExtractor {

    private FMPowerExtractor() {}

    /**
     * Extracts compressor stage parameters from a parsed Blkx FM file.
     *
     * <p>The method maps Blkx data to CompressorStageParams using WAPC-compatible formulas:
     * <ul>
     *   <li>critAlt ← compAlt[i]</li>
     *   <li>critPower ← compPower[i]</li>
     *   <li>deckPower ← Main.Power (stage 0) or 0.8× previous stage (WAPC deck_power_maker)</li>
     *   <li>wepPowerMult ← 4-factor formula (octane, throttle, stage, RPM boost)</li>
     *   <li>wepCritAlt ← supercharger pressure model or 0.9× fallback</li>
     *   <li>curvature ← compRpmRatio[i] or 1.0 if not specified</li>
     *   <li>speedManifoldMult ← speedToManifoldMultiplier</li>
     * </ul>
     *
     * @param blkx parsed FM file data
     * @return array of CompressorStageParams, or null if not a piston engine
     */
    public static CompressorStageParams[] extractStages(Blkx blkx) {
        if (blkx == null || blkx.compNumSteps <= 0) {
            return null;  // Not a piston engine or invalid data
        }

        CompressorStageParams[] stages = new CompressorStageParams[blkx.compNumSteps];

        for (int i = 0; i < blkx.compNumSteps; i++) {
            stages[i] = new CompressorStageParams();
            stages[i].stageIndex = i;

            // Critical altitude and power (primary parameters)
            stages[i].critAlt = blkx.compAlt[i];
            stages[i].critPower = blkx.compPower[i];

            // === IMPROVED: Deck power using WAPC deck_power_maker formula ===
            stages[i].deckPower = calculateDeckPower(blkx, i);

            // Power curve curvature (affects interpolation shape)
            stages[i].curvature = blkx.compRpmRatio[i] > 0 ? blkx.compRpmRatio[i] : 1.0;

            // RAM effect coefficient
            stages[i].speedManifoldMult = blkx.speedToManifoldMultiplier > 0
                ? blkx.speedToManifoldMultiplier : 1.0;

            // === IMPROVED: WEP multiplier using complete WAPC 4-factor formula ===
            stages[i].wepPowerMult = calculateWepMultiplier(blkx, i);

            // === IMPROVED: WEP critical altitude using supercharger pressure model ===
            stages[i].wepCritAlt = calculateWepCriticalAltitude(blkx, i);
        }

        return stages;
    }

    // ==================== WAPC-Compatible Helper Methods ====================

    /**
     * Calculates deck (sea level) power for a supercharger stage.
     *
     * <p>Implements WAPC deck_power_maker() logic:
     * <ul>
     *   <li>Stage 0: Use Main.Power from FM file, fallback to 0.8× critical power</li>
     *   <li>Stage 1+: Use 0.8× previous stage power, but not less than 0.8× current critical</li>
     * </ul>
     *
     * @param blkx       parsed FM file data
     * @param stageIndex supercharger stage index (0-based)
     * @return deck power in horsepower
     */
    private static double calculateDeckPower(Blkx blkx, int stageIndex) {
        if (stageIndex == 0) {
            // First stage: prefer Main.Power from FM
            if (blkx.deckPower > 0) {
                return blkx.deckPower;
            }
            // Fallback: WAPC uses 0.8× critical power as default
            return blkx.compPower[0] * 0.8;
        } else {
            // Later stages: 0.8× previous stage power
            double prevPower = blkx.compPower[stageIndex - 1];
            double deckPower = prevPower * 0.8;
            // But not less than 0.8× current stage critical power
            double minDeck = blkx.compPower[stageIndex] * 0.8;
            return Math.max(deckPower, minDeck);
        }
    }

    /**
     * Calculates the complete WEP power multiplier.
     *
     * <p>Implements WAPC WEP_power_mult formula:
     * <pre>
     * WEP_mult = (1 + (AfterburnerBoost - 1) × OctaneAfterburnerMult)
     *          × ThrottleBoost
     *          × AfterburnerBoostMul[i]
     *          × torque_rpm_boost(military_RPM, WEP_RPM)
     * </pre>
     *
     * @param blkx       parsed FM file data
     * @param stageIndex supercharger stage index
     * @return WEP power multiplier (typically 1.1-1.2)
     */
    private static double calculateWepMultiplier(Blkx blkx, int stageIndex) {
        // 1. Afterburner boost effect (modified by octane rating)
        double afterburnerBoost = blkx.aftbCoff > 0 ? blkx.aftbCoff : 1.0;
        double octaneMult = blkx.octaneAfterburnerMult > 0 ? blkx.octaneAfterburnerMult : 1.0;
        double boostEffect = 1.0 + (afterburnerBoost - 1.0) * octaneMult;

        // 2. Throttle boost factor
        double throttleBoost = blkx.throttleBoost > 0 ? blkx.throttleBoost : 1.0;

        // 3. Per-stage WEP multiplier
        double stageMult = blkx.compBoost[stageIndex] > 0 ? blkx.compBoost[stageIndex] : 1.0;

        // 4. RPM torque effect (higher WEP RPM produces more power)
        double rpmBoost = torqueRpmBoost(blkx.militaryRPM, blkx.wepRPM);

        return boostEffect * throttleBoost * stageMult * rpmBoost;
    }

    /**
     * Calculates the WEP critical altitude using supercharger pressure model.
     *
     * <p>Implements WAPC WEP critical altitude formula:
     * <pre>
     * supercharger_strength_mil = militaryMP / pressure(militaryCritAlt)
     * supercharger_strength_wep = supercharger_strength_mil × RPM_effect × AfterburnerPressureBoost
     * WEP_crit_altitude = altitude_at_pressure(wepMP / supercharger_strength_wep)
     * </pre>
     *
     * <p>Falls back to 0.9× military critical altitude if manifold pressure
     * parameters are not available in the FM file.
     *
     * @param blkx       parsed FM file data
     * @param stageIndex supercharger stage index
     * @return WEP critical altitude in meters
     */
    private static double calculateWepCriticalAltitude(Blkx blkx, int stageIndex) {
        double militaryCritAlt = blkx.compAlt[stageIndex];

        // Check if we have the manifold pressure data for accurate calculation
        double militaryMP = (blkx.compATA != null && stageIndex < blkx.compATA.length)
            ? blkx.compATA[stageIndex] : 0;
        double wepMP = blkx.wepManifoldPressure;

        // Fallback to simple estimate if parameters missing
        if (militaryMP <= 0 || wepMP <= 0) {
            return militaryCritAlt * 0.9;
        }

        // Calculate supercharger "strength" at military critical altitude
        double critPressure = pressure(militaryCritAlt);
        double superchargerStrength = militaryMP / critPressure;

        // RPM effect on supercharger efficiency
        double rpmEffect = superchargerRpmEffect(
            blkx.militaryRPM,
            blkx.wepRPM,
            blkx.compPressureAtRPM0 > 0 ? blkx.compPressureAtRPM0 : 0.3,  // Default ~0.3
            blkx.compOmegaFactorSq > 0 ? blkx.compOmegaFactorSq : 0.0     // Default 0
        );

        // Afterburner pressure boost for this stage
        double pressureBoost = (blkx.compAfterburnerPressureBoost != null &&
                               stageIndex < blkx.compAfterburnerPressureBoost.length &&
                               blkx.compAfterburnerPressureBoost[stageIndex] > 0)
            ? blkx.compAfterburnerPressureBoost[stageIndex] : 1.0;

        // WEP supercharger strength
        double wepSuperchargerStrength = superchargerStrength * rpmEffect * pressureBoost;

        // Calculate WEP critical altitude from pressure requirement
        double wepCritPressure = wepMP / wepSuperchargerStrength;
        return altitudeAtPressure(wepCritPressure);
    }

    /**
     * Checks if the aircraft uses piston engines.
     *
     * @param blkx parsed FM file data
     * @return true if piston engine (has compressor stages), false otherwise
     */
    public static boolean isPistonEngine(Blkx blkx) {
        return blkx != null && !blkx.isJet && blkx.compNumSteps > 0;
    }

    /**
     * Gets the global WEP boost factor from FM data.
     *
     * @param blkx parsed FM file data
     * @return WEP boost factor, or 1.0 if not available
     */
    public static double getWepBoostFactor(Blkx blkx) {
        if (blkx == null) {
            return 1.0;
        }
        return blkx.aftbCoff > 0 ? blkx.aftbCoff : 1.0;
    }

    /**
     * Gets the RAM effect coefficient from FM data.
     *
     * @param blkx parsed FM file data
     * @return SpeedManifoldMultiplier, or 1.0 if not available
     */
    public static double getSpeedManifoldMultiplier(Blkx blkx) {
        if (blkx == null) {
            return 1.0;
        }
        return blkx.speedToManifoldMultiplier > 0 ? blkx.speedToManifoldMultiplier : 1.0;
    }
}
