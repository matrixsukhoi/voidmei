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
     * Extracts compressor stage parameters from a parsed Blkx FM file,
     * applying fuel quality modifications from the Central file.
     *
     * <p>This overload supports fuel upgrades (e.g., Soviet B-100, British 150 octane)
     * that modify engine power output. If fuelMod is null, no fuel bonus is applied.
     *
     * @param blkx    parsed FM file data
     * @param fuelMod fuel modification data from Central file, or null
     * @return array of CompressorStageParams, or null if not a piston engine
     * @see #extractStages(Blkx)
     */
    public static CompressorStageParams[] extractStages(Blkx blkx, Blkx.FuelModification fuelMod) {
        CompressorStageParams[] stages = extractStages(blkx);
        if (stages == null) return null;

        // Apply fuel modifications if present
        if (fuelMod != null && fuelMod.type != Blkx.FuelModification.FuelType.NONE) {
            applyFuelModifications(stages, fuelMod, blkx);
        }

        return stages;
    }

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

        // Detect ExactAltitudes:
        // 1. If explicitly defined in FM file, use that
        // 2. Otherwise, if CompressorOmegaFactorSq is missing, set true (old format)
        boolean exactAltitudes;
        if (blkx.explicitExactAltitudes != null) {
            exactAltitudes = blkx.explicitExactAltitudes;
        } else {
            exactAltitudes = !blkx.hasCompOmegaFactorSq;
        }

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
                // After adjustment, update stageDeckPower and cascade to subsequent stages
                stageDeckPower[i] = stages[i].deckPower;
                // Recalculate deck power for subsequent stages based on adjusted values
                for (int j = i + 1; j < blkx.compNumSteps; j++) {
                    double newDeck = 0.8 * stageDeckPower[j - 1];
                    double minDeck = 0.8 * blkx.compPower[j];
                    if (newDeck < minDeck) newDeck = minDeck;
                    stageDeckPower[j] = newDeck;
                    stages[j].deckPower = newDeck;
                }
            }

            stages[i].oldPowerNewRpm = stages[i].oldPower;
            if (needsRpmAdjustment) {
                stages[i].oldPowerNewRpm = stages[i].oldPower
                        / torqueRpmBoost(blkx.militaryRPM, defaultRpm);
            }
        }

        // Set stage0DeckAlt for all stages (used in WEP non-ExactAltitudes mode)
        double stage0DeckAlt = stages[0].deckAlt;
        for (int i = 0; i < blkx.compNumSteps; i++) {
            stages[i].stage0DeckAlt = stage0DeckAlt;
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

            // Handle AfterburnerBoostMul explicitly set to 0 (no WEP for this stage)
            // Only disable WEP if the field EXISTS and is explicitly 0
            // If field is missing, WEP uses global AfterburnerBoost (handled by calculateWepMultiplier)
            if (blkx.hasCompBoost != null && blkx.hasCompBoost[i] && blkx.compBoost[i] == 0) {
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
     * <p>Port of WAPC wep_rpm_ratioer priority logic:
     * <ol>
     *   <li>ShaftRPMMax: if far from military AND close to WEP RPM</li>
     *   <li>RPMNom: if far from military RPM</li>
     *   <li>GovernorMaxParam: if far from military RPM</li>
     *   <li>Fallback: military RPM (no adjustment needed)</li>
     * </ol>
     */
    private static double determineDefaultRpm(Blkx blkx) {
        // Priority 1: ShaftRPMMax close to WEP but far from military
        if (blkx.shaftRPMMax > 0
                && (blkx.shaftRPMMax - blkx.militaryRPM) > 5
                && (blkx.shaftRPMMax - blkx.wepRPM) < 5) {
            return blkx.shaftRPMMax;
        }
        // Priority 2: RPMNom far from military
        if (blkx.rpmNom > 0 && (blkx.rpmNom - blkx.militaryRPM) > 5) {
            return blkx.rpmNom;
        }
        // Priority 3: GovernorMaxParam far from military
        if (blkx.governorMaxParam > 0 && (blkx.governorMaxParam - blkx.militaryRPM) > 5) {
            return blkx.governorMaxParam;
        }
        return blkx.militaryRPM;
    }

    /**
     * Adjusts power and critical altitude from default RPM to military RPM.
     * Port of WAPC definition_alt_power_adjuster().
     */
    private static void adjustPowerAndAltitude(CompressorStageParams stage, Blkx blkx,
                                                int i, double defaultRpm, double[] stageDeckPower) {
        double militaryMP = blkx.militaryMP;
        if (militaryMP <= 0) return;

        double rpmBoost = torqueRpmBoost(blkx.militaryRPM, defaultRpm);
        if (rpmBoost <= 0 || Math.abs(rpmBoost - 1.0) < 0.001) return;

        // Calculate supercharger effect to find adjusted critical altitude
        double pressureAtRPM0 = blkx.compPressureAtRPM0 > 0 ? blkx.compPressureAtRPM0 : 0.3;
        // WAPC: missing → 1.0; explicit 0 → 0
        double omegaFactorSq = blkx.hasCompOmegaFactorSq ? blkx.compOmegaFactorSq : 1.0;
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
        // If WEP power multiplier ≈ 1.0, WEP is effectively military — no altitude shift
        double wepMult = calculateWepMultiplier(blkx, stageIndex);
        if (Math.abs(wepMult - 1.0) < 0.001) {
            return stage.critAlt;
        }

        double militaryMP = blkx.militaryMP;
        double wepMP = blkx.wepManifoldPressure;

        if (militaryMP <= 0 || wepMP <= 0) {
            return stage.critAlt * 0.9;
        }

        // Use the adjusted (military) critical altitude for strength calculation
        double critPressure = pressure(stage.critAlt);
        double superchargerStrength = militaryMP / critPressure;

        // WAPC: missing → 1.0; explicit 0 → 0
        double omegaFactorSq = blkx.hasCompOmegaFactorSq ? blkx.compOmegaFactorSq : 1.0;
        double rpmEffect = superchargerRpmEffect(
            blkx.militaryRPM, blkx.wepRPM,
            blkx.compPressureAtRPM0 > 0 ? blkx.compPressureAtRPM0 : 0.3,
            omegaFactorSq
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
        // If WEP power multiplier ≈ 1.0, WEP is effectively military — no deck shift
        double wepMult = calculateWepMultiplier(blkx, stageIndex);
        if (Math.abs(wepMult - 1.0) < 0.001) {
            return stage.deckAlt;
        }

        double militaryMP = blkx.militaryMP;
        double wepMP = blkx.wepManifoldPressure;

        if (militaryMP <= 0 || wepMP <= 0) {
            return 0;
        }

        double deckStrength = militaryMP / pressure(stage.deckAlt);
        double omegaFactorSq = blkx.hasCompOmegaFactorSq ? blkx.compOmegaFactorSq : 1.0;
        double rpmEffect = superchargerRpmEffect(
            blkx.militaryRPM, blkx.wepRPM,
            blkx.compPressureAtRPM0 > 0 ? blkx.compPressureAtRPM0 : 0.3,
            omegaFactorSq
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
        double militaryMP = blkx.militaryMP;
        double wepMP = blkx.wepManifoldPressure;

        if (militaryMP <= 0 || wepMP <= 0 || stage.constRpmAlt == 0) {
            return 0;
        }

        double constRpmStrength = militaryMP / pressure(stage.constRpmAlt);
        double omegaFactorSq = blkx.hasCompOmegaFactorSq ? blkx.compOmegaFactorSq : 1.0;
        double rpmEffect = superchargerRpmEffect(
            blkx.militaryRPM, blkx.wepRPM,
            blkx.compPressureAtRPM0 > 0 ? blkx.compPressureAtRPM0 : 0.3,
            omegaFactorSq
        );
        double pressureBoost = (blkx.compAfterburnerPressureBoost != null &&
                               stageIndex < blkx.compAfterburnerPressureBoost.length &&
                               blkx.compAfterburnerPressureBoost[stageIndex] > 0)
            ? blkx.compAfterburnerPressureBoost[stageIndex] : 1.0;

        double wepConstRpmStrength = constRpmStrength * rpmEffect * pressureBoost;
        return altitudeAtPressure(wepMP / wepConstRpmStrength);
    }

    // ==================== Fuel Modification Application ====================

    /**
     * Soviet octane power multiplier: 1.8% power increase.
     * Applied when addHorsePowers == 50 (WAPC soviet_octane_adder convention).
     */
    private static final double SOVIET_OCTANE_POWER_MULT = 1.018;

    /**
     * Applies fuel quality modifications to extracted compressor stages.
     *
     * <p>Dispatches to the appropriate handler based on fuel type:
     * <ul>
     *   <li>Soviet B-95/B-100: Multiplicative power gain across all stages</li>
     *   <li>British 150 octane / 100 spitfire: WEP multiplier and critical altitude changes</li>
     * </ul>
     */
    private static void applyFuelModifications(CompressorStageParams[] stages,
                                                Blkx.FuelModification fuelMod, Blkx blkx) {
        switch (fuelMod.type) {
            case SOVIET_B95:
            case SOVIET_B100:
                applySovietOctaneBonus(stages, fuelMod);
                break;
            case BRITISH_150_OCTANE:
            case BRITISH_100_SPITFIRE:
                applyBritishOctaneBonus(stages, fuelMod, blkx);
                break;
            default:
                break;
        }
    }

    /**
     * Applies Soviet fuel octane bonus to all compressor stages.
     *
     * <p>Port of WAPC soviet_octane_adder():
     * <ul>
     *   <li>Condition: addHorsePowers == 50 (the standard Soviet fuel bonus value)</li>
     *   <li>Effect: 1.8% multiplicative power gain on deckPower (stage 0 only),
     *       critPower, constRpmPower, and ceilingPower for all stages</li>
     * </ul>
     *
     * <p>The 1.8% figure comes from WAPC's empirical fit: the +50 HP is not added
     * directly (as it would be altitude-independent), but rather modeled as a
     * percentage boost that scales with the power curve.
     *
     * @param stages  compressor stages to modify in-place
     * @param fuelMod fuel modification containing sovietOctaneHpBonus
     */
    private static void applySovietOctaneBonus(CompressorStageParams[] stages,
                                                Blkx.FuelModification fuelMod) {
        // Only apply when addHorsePowers is exactly 50 (WAPC convention)
        if (Math.abs(fuelMod.sovietOctaneHpBonus - 50) > 0.01) {
            return;
        }

        Logger.info("FMPowerExtractor", String.format(
                "Applying Soviet octane bonus: %.1f%% power increase (addHorsePowers=%.0f)",
                (SOVIET_OCTANE_POWER_MULT - 1) * 100, fuelMod.sovietOctaneHpBonus));

        for (int i = 0; i < stages.length; i++) {
            // Deck power: only apply to stage 0
            // (later stages derive from stage 0 via 0.8× cascade)
            if (i == 0) {
                stages[i].deckPower *= SOVIET_OCTANE_POWER_MULT;
            }
            stages[i].critPower *= SOVIET_OCTANE_POWER_MULT;
            if (stages[i].constRpmPower > 0) {
                stages[i].constRpmPower *= SOVIET_OCTANE_POWER_MULT;
            }
            if (stages[i].ceilingPower > 0) {
                stages[i].ceilingPower *= SOVIET_OCTANE_POWER_MULT;
            }
        }
    }

    /**
     * Applies British fuel octane bonus to all compressor stages.
     *
     * <p>Port of WAPC brrritish_octane_adder():
     * <ul>
     *   <li>If invertEnableLogic is true: high octane is the default, so
     *       the modification represents REMOVING it — no bonus applied</li>
     *   <li>Otherwise: modifies the WEP power multiplier using the fuel's
     *       afterburnerMult and afterburnerCompressorMult values, and
     *       recalculates WEP critical altitude</li>
     * </ul>
     *
     * @param stages  compressor stages to modify in-place
     * @param fuelMod fuel modification containing British fuel parameters
     * @param blkx    parsed FM data for recalculating WEP altitudes
     */
    private static void applyBritishOctaneBonus(CompressorStageParams[] stages,
                                                 Blkx.FuelModification fuelMod, Blkx blkx) {
        // invertEnableLogic means the high-octane fuel is the DEFAULT state
        // The "modification" represents removing it, so we don't apply any bonus
        if (fuelMod.britishInvertLogic) {
            return;
        }

        Logger.info("FMPowerExtractor", String.format(
                "Applying British octane bonus: afterburnerMult=%.3f, compressorMult=%.3f",
                fuelMod.britishAfterburnerMult, fuelMod.britishAfterburnerCompressorMult));

        for (int i = 0; i < stages.length; i++) {
            // Modify WEP power multiplier with fuel's afterburnerMult
            stages[i].wepPowerMult *= fuelMod.britishAfterburnerMult;

            // Recalculate WEP critical altitude with fuel's compressor boost
            if (blkx.militaryMP > 0 && blkx.wepManifoldPressure > 0
                    && Math.abs(stages[i].wepPowerMult - 1.0) > 0.001) {
                double critPressure = pressure(stages[i].critAlt);
                double superchargerStrength = blkx.militaryMP / critPressure;

                double omegaFactorSq = blkx.hasCompOmegaFactorSq ? blkx.compOmegaFactorSq : 1.0;
                double rpmEffect = superchargerRpmEffect(
                        blkx.militaryRPM, blkx.wepRPM,
                        blkx.compPressureAtRPM0 > 0 ? blkx.compPressureAtRPM0 : 0.3,
                        omegaFactorSq);

                double basePressureBoost = (blkx.compAfterburnerPressureBoost != null
                        && i < blkx.compAfterburnerPressureBoost.length
                        && blkx.compAfterburnerPressureBoost[i] > 0)
                        ? blkx.compAfterburnerPressureBoost[i] : 1.0;

                // Apply fuel's compressor multiplier on top of base pressure boost
                double totalPressureBoost = basePressureBoost * fuelMod.britishAfterburnerCompressorMult;
                double wepSuperchargerStrength = superchargerStrength * rpmEffect * totalPressureBoost;
                double wepCritPressure = blkx.wepManifoldPressure / wepSuperchargerStrength;
                stages[i].wepCritAlt = Math.round(altitudeAtPressure(wepCritPressure));
            }
        }
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
