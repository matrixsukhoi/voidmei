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
 * <h3>WAPC Compatibility</h3>
 * <p>This implementation follows the wt-aircraft-performance-calculator (WAPC)
 * formulas, including:
 * <ul>
 *   <li>Deck power: Uses Main.Power for stage 0, 0.8× previous stage deck power for later stages</li>
 *   <li>WEP multiplier: 4-factor formula (octane, throttle, stage, RPM)</li>
 *   <li>WEP critical altitude: Supercharger pressure model</li>
 *   <li>ExactAltitudes detection: Old FM format handling</li>
 *   <li>definition_alt_power_adjuster: RPM-based power/altitude correction</li>
 *   <li>ConstRPM and Ceiling parameter propagation</li>
 * </ul>
 */
public final class FMPowerExtractor {

    private FMPowerExtractor() {}

    /**
     * Extracts compressor stage parameters from a parsed Blkx FM file.
     *
     * <p>Performs a multi-pass extraction:
     * <ol>
     *   <li>Basic parameter mapping (critAlt, critPower, curvature, etc.)</li>
     *   <li>Deck power calculation (WAPC deck_power_maker)</li>
     *   <li>ExactAltitudes detection</li>
     *   <li>RPM-based power/altitude adjustment (definition_alt_power_adjuster)</li>
     *   <li>WEP parameter calculation (multiplier, critical altitude, deck altitude)</li>
     * </ol>
     *
     * @param blkx parsed FM file data
     * @return array of CompressorStageParams, or null if not a piston engine
     */
    public static CompressorStageParams[] extractStages(Blkx blkx) {
        if (blkx == null || blkx.compNumSteps <= 0) {
            return null;
        }

        // Detect ExactAltitudes (old FM format: no CompressorOmegaFactorSq)
        boolean exactAltitudes = (blkx.compOmegaFactorSq <= 0);

        // Determine "default RPM" — the RPM at which FM power values are defined
        double defaultRpm = determineDefaultRpm(blkx);

        CompressorStageParams[] stages = new CompressorStageParams[blkx.compNumSteps];

        // --- Pass 1: Basic parameter extraction ---
        // We need deck power values stored per-stage for the deck_power_maker logic
        double[] stageDeckPower = new double[blkx.compNumSteps];

        for (int i = 0; i < blkx.compNumSteps; i++) {
            stages[i] = new CompressorStageParams();
            stages[i].stageIndex = i;
            stages[i].exactAltitudes = exactAltitudes;

            // Critical altitude and power
            stages[i].critAlt = blkx.compAlt[i];
            stages[i].critPower = blkx.compPower[i];

            // Store originals before adjustment
            stages[i].oldAltitude = blkx.compAlt[i];
            stages[i].oldPower = blkx.compPower[i];
            stages[i].oldPowerNewRpm = blkx.compPower[i];

            // ConstRPM parameters
            if (blkx.compConstRpmAlt != null && i < blkx.compConstRpmAlt.length) {
                stages[i].constRpmAlt = blkx.compConstRpmAlt[i];
                stages[i].constRpmPower = blkx.compConstRpmPower[i];
            }

            // Ceiling parameters
            stages[i].ceilingAlt = blkx.compCeil[i];
            stages[i].ceilingPower = blkx.compCeilPwr[i];

            // Power curve curvature
            stages[i].curvature = blkx.compRpmRatio[i] > 0 ? blkx.compRpmRatio[i] : 1.0;

            // RAM effect coefficient
            stages[i].speedManifoldMult = blkx.speedToManifoldMultiplier > 0
                ? blkx.speedToManifoldMultiplier : 1.0;

            // Deck power: WAPC deck_power_maker logic
            // Stage 0: Main.Power; Stage 1+: 0.8× previous stage DECK power
            if (i == 0) {
                stageDeckPower[i] = blkx.deckPower > 0 ? blkx.deckPower : blkx.compPower[0] * 0.8;
            } else {
                stageDeckPower[i] = 0.8 * stageDeckPower[i - 1];
                double minDeck = 0.8 * blkx.compPower[i];
                if (stageDeckPower[i] < minDeck) {
                    stageDeckPower[i] = minDeck;
                }
            }
            stages[i].deckPower = stageDeckPower[i];
        }

        // --- Pass 2: definition_alt_power_adjuster ---
        // If FM power/altitude is defined for a higher RPM (WEP or default RPM) rather
        // than military RPM, adjust to military RPM baseline
        boolean needsRpmAdjustment = needsRpmAdjustment(blkx, defaultRpm);

        for (int i = 0; i < blkx.compNumSteps; i++) {
            if (needsRpmAdjustment) {
                adjustPowerAndAltitude(stages[i], blkx, i, defaultRpm, stageDeckPower);
                // After adjustment, recalculate deck power for later stages
                if (i == 0) {
                    stageDeckPower[0] = stages[0].deckPower;
                }
            }

            stages[i].oldPowerNewRpm = stages[i].oldPower;
            if (needsRpmAdjustment) {
                stages[i].oldPowerNewRpm = stages[i].oldPower
                        / torqueRpmBoost(blkx.militaryRPM, defaultRpm);
            }
        }

        // --- Pass 3: WEP parameters ---
        for (int i = 0; i < blkx.compNumSteps; i++) {
            stages[i].wepPowerMult = calculateWepMultiplier(blkx, i);
            stages[i].wepCritAlt = calculateWepCriticalAltitude(blkx, stages[i], i);
            stages[i].wepDeckAlt = calculateWepDeckAltitude(blkx, stages[i], i);

            // WEP ConstRPM altitude (for non-ExactAltitudes FMs like F2G-1)
            if (!exactAltitudes && stages[i].constRpmAlt != 0 && stages[i].constRpmPower > 0) {
                stages[i].wepConstRpmAlt = calculateWepConstRpmAltitude(blkx, stages[i], i);
            }

            // Handle AfterburnerBoostMul == 0 (no WEP for this stage)
            if (blkx.compBoost[i] == 0) {
                stages[i].wepDeckAlt = 0;
                stages[i].wepCritAlt = stages[i].critAlt;
                stages[i].wepPowerMult = 1.0;
            }
        }

        return stages;
    }

    // ==================== RPM Adjustment (definition_alt_power_adjuster) ====================

    /**
     * Checks if the FM defines power values at a higher RPM than military RPM.
     * If so, the power/altitude values need to be adjusted down to military RPM baseline.
     */
    private static boolean needsRpmAdjustment(Blkx blkx, double defaultRpm) {
        return (defaultRpm - blkx.militaryRPM) > 5;
    }

    /**
     * Determines the "default RPM" — the RPM at which FM file power values are defined.
     *
     * <p>WAPC checks ShaftRPMMax, RPMNom, and GovernorMaxParam in order.
     * Since Blkx doesn't have all of these, we approximate using available data.
     */
    private static double determineDefaultRpm(Blkx blkx) {
        // If WEP RPM is close to maxRPM and significantly higher than military RPM,
        // the FM likely defines power at WEP RPM
        if (blkx.wepRPM > 0 && blkx.militaryRPM > 0
                && (blkx.wepRPM - blkx.militaryRPM) > 5) {
            return blkx.wepRPM;
        }
        return blkx.militaryRPM;
    }

    /**
     * Adjusts power and critical altitude from default RPM to military RPM.
     * Port of WAPC definition_alt_power_adjuster().
     */
    private static void adjustPowerAndAltitude(CompressorStageParams stage, Blkx blkx,
                                                int i, double defaultRpm, double[] stageDeckPower) {
        double militaryMP = (blkx.compATA != null && i < blkx.compATA.length && blkx.compATA[i] > 0)
            ? blkx.compATA[i] : 0;
        if (militaryMP <= 0) return;

        double rpmBoost = torqueRpmBoost(blkx.militaryRPM, defaultRpm);
        if (rpmBoost <= 0 || Math.abs(rpmBoost - 1.0) < 0.001) return;

        // Calculate supercharger effect to find adjusted critical altitude
        double pressureAtRPM0 = blkx.compPressureAtRPM0 > 0 ? blkx.compPressureAtRPM0 : 0.3;
        double omegaFactorSq = blkx.compOmegaFactorSq > 0 ? blkx.compOmegaFactorSq : 1.0;
        double defaultMilRpmEffect = superchargerRpmEffect(blkx.militaryRPM, defaultRpm,
                pressureAtRPM0, omegaFactorSq);

        // Adjust critical altitude: remove the extra supercharger boost from higher RPM
        double fakeSuperchargerStrength = militaryMP / pressure(stage.critAlt);
        double realSuperchargerStrength = fakeSuperchargerStrength / defaultMilRpmEffect;
        double adjustedCritAlt = Math.round(altitudeAtPressure(militaryMP / realSuperchargerStrength));

        // Adjust deck altitude similarly
        double fakeDeckStrength = militaryMP / pressure(0);
        double realDeckStrength = fakeDeckStrength / defaultMilRpmEffect;
        double adjustedDeckAlt = altitudeAtPressure(militaryMP / realDeckStrength);
        stage.deckAlt = adjustedDeckAlt;

        // Adjust power: interpolate on original curve at new crit alt, then divide by RPM boost
        double deckPowerRatio = (blkx.deckPower > 0 && stage.oldPower > 0)
            ? blkx.deckPower / blkx.compPower[0] : 0.8;
        double adjustedPower = interpolatePower(
                stage.oldPower, stage.oldAltitude,
                stage.oldPower * deckPowerRatio,
                stage.oldAltitude - blkx.compAlt[0],
                adjustedCritAlt, 1.0) / rpmBoost;

        // Adjust ConstRPM power
        if (stage.constRpmPower > 0) {
            if (stage.constRpmPower == stage.oldPower) {
                // Special case (Hornet Mk3): keep constRPM aligned with adjusted power
                stage.constRpmPower = adjustedPower;
            } else {
                stage.constRpmPower = stage.constRpmPower / rpmBoost;
            }
        }

        // Adjust ceiling altitude
        if (stage.ceilingAlt > 0) {
            double fakeCeilStrength = militaryMP / pressure(stage.ceilingAlt);
            double realCeilStrength = fakeCeilStrength / defaultMilRpmEffect;
            stage.ceilingAlt = Math.round(altitudeAtPressure(militaryMP / realCeilStrength));
        }

        stage.critAlt = adjustedCritAlt;
        stage.critPower = adjustedPower;

        // Recalculate deck power after adjustment
        if (i == 0) {
            // Deck power is interpolated on the original curve at the adjusted deck altitude, then /rpmBoost
            stage.deckPower = interpolatePower(
                    stage.oldPower, stage.oldAltitude,
                    stageDeckPower[0], 0,
                    adjustedDeckAlt, 1.0) / rpmBoost;
            stageDeckPower[0] = stage.deckPower;
        }
    }

    // ==================== WEP Parameter Calculations ====================

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
     */
    private static double calculateWepMultiplier(Blkx blkx, int stageIndex) {
        double afterburnerBoost = blkx.aftbCoff > 0 ? blkx.aftbCoff : 1.0;
        double octaneMult = blkx.octaneAfterburnerMult > 0 ? blkx.octaneAfterburnerMult : 1.0;
        double boostEffect = 1.0 + (afterburnerBoost - 1.0) * octaneMult;

        double throttleBoost = blkx.throttleBoost > 0 ? blkx.throttleBoost : 1.0;
        double stageMult = blkx.compBoost[stageIndex] > 0 ? blkx.compBoost[stageIndex] : 1.0;
        double rpmBoost = torqueRpmBoost(blkx.militaryRPM, blkx.wepRPM);

        return boostEffect * throttleBoost * stageMult * rpmBoost;
    }

    /**
     * Calculates the WEP critical altitude using supercharger pressure model.
     */
    private static double calculateWepCriticalAltitude(Blkx blkx, CompressorStageParams stage, int stageIndex) {
        double militaryMP = (blkx.compATA != null && stageIndex < blkx.compATA.length)
            ? blkx.compATA[stageIndex] : 0;
        double wepMP = blkx.wepManifoldPressure;

        if (militaryMP <= 0 || wepMP <= 0) {
            return stage.critAlt * 0.9;
        }

        // Use the adjusted (military) critical altitude for strength calculation
        double critPressure = pressure(stage.critAlt);
        double superchargerStrength = militaryMP / critPressure;

        double rpmEffect = superchargerRpmEffect(
            blkx.militaryRPM, blkx.wepRPM,
            blkx.compPressureAtRPM0 > 0 ? blkx.compPressureAtRPM0 : 0.3,
            blkx.compOmegaFactorSq > 0 ? blkx.compOmegaFactorSq : 0.0
        );

        double pressureBoost = (blkx.compAfterburnerPressureBoost != null &&
                               stageIndex < blkx.compAfterburnerPressureBoost.length &&
                               blkx.compAfterburnerPressureBoost[stageIndex] > 0)
            ? blkx.compAfterburnerPressureBoost[stageIndex] : 1.0;

        double wepSuperchargerStrength = superchargerStrength * rpmEffect * pressureBoost;
        double wepCritPressure = wepMP / wepSuperchargerStrength;
        return Math.round(altitudeAtPressure(wepCritPressure));
    }

    /**
     * Calculates the WEP deck altitude.
     */
    private static double calculateWepDeckAltitude(Blkx blkx, CompressorStageParams stage, int stageIndex) {
        double militaryMP = (blkx.compATA != null && stageIndex < blkx.compATA.length)
            ? blkx.compATA[stageIndex] : 0;
        double wepMP = blkx.wepManifoldPressure;

        if (militaryMP <= 0 || wepMP <= 0) {
            return 0;
        }

        double deckStrength = militaryMP / pressure(stage.deckAlt);
        double rpmEffect = superchargerRpmEffect(
            blkx.militaryRPM, blkx.wepRPM,
            blkx.compPressureAtRPM0 > 0 ? blkx.compPressureAtRPM0 : 0.3,
            blkx.compOmegaFactorSq > 0 ? blkx.compOmegaFactorSq : 0.0
        );
        double pressureBoost = (blkx.compAfterburnerPressureBoost != null &&
                               stageIndex < blkx.compAfterburnerPressureBoost.length &&
                               blkx.compAfterburnerPressureBoost[stageIndex] > 0)
            ? blkx.compAfterburnerPressureBoost[stageIndex] : 1.0;

        double wepDeckStrength = deckStrength * rpmEffect * pressureBoost;
        return Math.round(altitudeAtPressure(wepMP / wepDeckStrength));
    }

    /**
     * Calculates the WEP ConstRPM altitude for non-ExactAltitudes FMs.
     */
    private static double calculateWepConstRpmAltitude(Blkx blkx, CompressorStageParams stage, int stageIndex) {
        double militaryMP = (blkx.compATA != null && stageIndex < blkx.compATA.length)
            ? blkx.compATA[stageIndex] : 0;
        double wepMP = blkx.wepManifoldPressure;

        if (militaryMP <= 0 || wepMP <= 0 || stage.constRpmAlt == 0) {
            return 0;
        }

        double constRpmStrength = militaryMP / pressure(stage.constRpmAlt);
        double rpmEffect = superchargerRpmEffect(
            blkx.militaryRPM, blkx.wepRPM,
            blkx.compPressureAtRPM0 > 0 ? blkx.compPressureAtRPM0 : 0.3,
            blkx.compOmegaFactorSq > 0 ? blkx.compOmegaFactorSq : 0.0
        );
        double pressureBoost = (blkx.compAfterburnerPressureBoost != null &&
                               stageIndex < blkx.compAfterburnerPressureBoost.length &&
                               blkx.compAfterburnerPressureBoost[stageIndex] > 0)
            ? blkx.compAfterburnerPressureBoost[stageIndex] : 1.0;

        double wepConstRpmStrength = constRpmStrength * rpmEffect * pressureBoost;
        return altitudeAtPressure(wepMP / wepConstRpmStrength);
    }

    // ==================== Public Utility Methods ====================

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
        if (blkx == null) return 1.0;
        return blkx.aftbCoff > 0 ? blkx.aftbCoff : 1.0;
    }

    /**
     * Gets the RAM effect coefficient from FM data.
     *
     * @param blkx parsed FM file data
     * @return SpeedManifoldMultiplier, or 1.0 if not available
     */
    public static double getSpeedManifoldMultiplier(Blkx blkx) {
        if (blkx == null) return 1.0;
        return blkx.speedToManifoldMultiplier > 0 ? blkx.speedToManifoldMultiplier : 1.0;
    }
}
