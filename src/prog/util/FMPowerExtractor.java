package prog.util;

import parser.Blkx;
import prog.util.PistonPowerModel.CompressorStageParams;

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
 * </ul>
 */
public final class FMPowerExtractor {

    private FMPowerExtractor() {}

    /**
     * Extracts compressor stage parameters from a parsed Blkx FM file.
     *
     * <p>The method maps Blkx data to CompressorStageParams:
     * <ul>
     *   <li>critAlt ← compAlt[i]</li>
     *   <li>critPower ← compPower[i]</li>
     *   <li>deckPower ← estimated from compCeilPwr of previous stage or 92% of critPower</li>
     *   <li>wepPowerMult ← aftbCoff × compBoost[i]</li>
     *   <li>wepCritAlt ← estimated as 90% of military critAlt</li>
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

            // Deck (sea level) power estimation
            // First stage: assume deck power is slightly lower than critical power
            // Later stages: use previous stage's ceiling power as starting point
            if (i == 0) {
                // For first stage, deck power is typically 90-95% of critical power
                // due to exhaust back-pressure at sea level
                stages[i].deckPower = blkx.compPower[i] * 0.92;
            } else {
                // Later stages start where previous stage drops off
                stages[i].deckPower = blkx.compCeilPwr[i - 1];
            }

            // Power curve curvature (affects interpolation shape)
            stages[i].curvature = blkx.compRpmRatio[i] > 0 ? blkx.compRpmRatio[i] : 1.0;

            // RAM effect coefficient
            stages[i].speedManifoldMult = blkx.speedToManifoldMultiplier > 0
                ? blkx.speedToManifoldMultiplier : 1.0;

            // WEP parameters
            // Base WEP multiplier from global AfterburnerBoost
            double baseMult = blkx.aftbCoff > 0 ? blkx.aftbCoff : 1.0;
            // Per-stage multiplier
            double stageMult = blkx.compBoost[i] > 0 ? blkx.compBoost[i] : 1.0;
            stages[i].wepPowerMult = baseMult * stageMult;

            // WEP critical altitude is typically lower than military
            // (higher manifold pressure requires more boost)
            stages[i].wepCritAlt = blkx.compAlt[i] * 0.9;
        }

        return stages;
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
