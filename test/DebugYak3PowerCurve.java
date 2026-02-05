import parser.Blkx;
import prog.i18n.Lang;
import prog.util.FMPowerExtractor;
import prog.util.Logger;
import prog.util.PistonPowerModel;
import prog.util.PistonPowerModel.CompressorStageParams;
import prog.util.PowerCurveHelper;

import static prog.util.AtmosphereModel.*;
import static prog.util.PistonPowerModel.*;
import static prog.util.PowerCurveHelper.*;

/**
 * Debug script for VoidMei Yak-3 power curve calculation.
 * Outputs intermediate values for comparison with WAPC.
 *
 * Usage:
 *   javac -encoding UTF-8 -d bin -classpath bin test/DebugYak3PowerCurve.java
 *   java -classpath bin DebugYak3PowerCurve [path/to/yak-3.blkx]
 *
 * If no path is provided, uses default path:
 *   /home/tu10ng/Downloads/voidmei/data/aces/gamedata/flightmodels/fm/yak-3.blkx
 *
 * Or use: ./script/test.sh (after adding to test runner)
 */
public class DebugYak3PowerCurve {

    private static final String DEFAULT_BLKX_PATH =
        "/home/tu10ng/Downloads/voidmei/data/aces/gamedata/flightmodels/fm/yak-3.blkx";

    public static void main(String[] args) {
        // Enable debug logging
        Logger.setMinLevel(Logger.Level.DEBUG);

        // Initialize language system with default values for standalone test
        initLangForStandaloneTest();

        System.out.println("======================================================================");
        System.out.println("VoidMei Yak-3 Power Curve Debug Output");
        System.out.println("======================================================================");

        // Determine blkx file path
        String blkxPath = args.length > 0 ? args[0] : DEFAULT_BLKX_PATH;
        System.out.println("\nUsing blkx file: " + blkxPath);

        // === PARSE BLKX FILE ===
        System.out.println("\n=== PARSING BLKX FILE ===\n");

        Blkx blkx = new Blkx(blkxPath, "yak-3");
        if (!blkx.valid) {
            System.err.println("ERROR: Failed to parse blkx file: " + blkxPath);
            System.err.println("Falling back to hardcoded parameters...\n");
            runWithHardcodedParams();
            return;
        }

        System.out.println("Blkx file parsed successfully!");
        System.out.println("  isJet: " + blkx.isJet);
        System.out.println("  compNumSteps: " + blkx.compNumSteps);
        System.out.println("  deckPower (Main.Power): " + blkx.deckPower);
        System.out.println("  speedManifoldMult: " + blkx.speedToManifoldMultiplier);
        System.out.println("  militaryRPM: " + blkx.militaryRPM);
        System.out.println("  wepRPM: " + blkx.wepRPM);
        System.out.println("  shaftRPMMax: " + blkx.shaftRPMMax);
        System.out.println("  governorMaxParam: " + blkx.governorMaxParam);
        System.out.println("  aftbCoff (AfterburnerBoost): " + blkx.aftbCoff);
        System.out.println("  explicitExactAltitudes: " + blkx.explicitExactAltitudes);
        System.out.println("  hasCompOmegaFactorSq: " + blkx.hasCompOmegaFactorSq);
        System.out.println("  compOmegaFactorSq: " + blkx.compOmegaFactorSq);
        System.out.println("  compPressureAtRPM0: " + blkx.compPressureAtRPM0);
        System.out.println("  militaryMP: " + blkx.militaryMP);
        System.out.println("  wepManifoldPressure: " + blkx.wepManifoldPressure);

        // Print raw compressor arrays
        System.out.println("\n--- Raw Compressor Arrays from Blkx ---");
        for (int i = 0; i < blkx.compNumSteps; i++) {
            System.out.printf("  Stage %d: Altitude=%s, Power=%s, Ceiling=%s, PowerAtCeiling=%s%n",
                i, blkx.compAlt[i], blkx.compPower[i], blkx.compCeil[i], blkx.compCeilPwr[i]);
            System.out.printf("           ConstRpmAlt=%s, ConstRpmPower=%s, Curvature=%s%n",
                blkx.compConstRpmAlt[i], blkx.compConstRpmPower[i], blkx.compRpmRatio[i]);
            System.out.printf("           AfterburnerBoostMul=%s (hasCompBoost=%s)%n",
                blkx.compBoost[i], blkx.hasCompBoost != null && blkx.hasCompBoost[i]);
        }

        // === COMPARE WITH HARDCODED VALUES ===
        System.out.println("\n=== COMPARISON: Blkx vs Hardcoded Values ===\n");
        compareWithHardcoded(blkx);

        // === EXTRACT STAGES USING FMPowerExtractor ===
        System.out.println("\n=== EXTRACTING STAGES (FMPowerExtractor) ===\n");

        CompressorStageParams[] stages = FMPowerExtractor.extractStages(blkx);
        if (stages == null || stages.length == 0) {
            System.err.println("ERROR: FMPowerExtractor returned null/empty stages!");
            System.err.println("Falling back to hardcoded parameters...\n");
            runWithHardcodedParams();
            return;
        }

        System.out.println("Extracted " + stages.length + " compressor stages");

        // === PARAMETER EXTRACTION (from parsed stages) ===
        System.out.println("\n=== PARAMETER EXTRACTION (parsed) ===\n");

        for (int i = 0; i < stages.length; i++) {
            printStageParams(stages[i], i);
        }

        // Ceiling_is_useful checks
        System.out.println("--- Ceiling_is_useful checks ---");
        for (int i = 0; i < stages.length; i++) {
            CompressorStageParams p = stages[i];
            boolean useful = ceilingIsUseful(p);
            double refAlt = p.oldAltitude > 0 ? p.oldAltitude : p.critAlt;
            double refPower = p.oldPower > 0 ? p.oldPower : p.critPower;
            System.out.printf("[Ceiling_is_useful] stage=%d: %s%n", i, useful);
            System.out.printf("  ceilingAlt=%.0f, referenceAlt=%.0f, diff=%.0f%n",
                    p.ceilingAlt, refAlt, p.ceilingAlt - refAlt);
            System.out.printf("  referencePower=%.0f, ceilingPower=%.0f, diff=%.0f%n",
                    refPower, p.ceilingPower, refPower - p.ceilingPower);
        }
        System.out.println();

        // === RUN POWER CALCULATIONS ===
        runPowerCalculations(stages);

        System.out.println("\n======================================================================");
        System.out.println("Debug output complete (using parsed blkx)");
        System.out.println("======================================================================");
    }

    /**
     * Run power calculations with given stages.
     */
    private static void runPowerCalculations(CompressorStageParams[] stages) {
        // === POWER CURVE (static, speed=0) ===
        System.out.println("=== POWER CURVE (military, static, stage 0) ===\n");

        int[] altitudes = {0, 100, 200, 300, 400, 500, 675, 1000, 1500, 2000, 2600, 3000, 5000, 7000, 9000, 10000};

        CompressorStageParams stage0 = stages[0];
        for (int alt : altitudes) {
            double altRam = alt;  // No RAM effect

            System.out.printf("[alt=%d] ramAlt=%.0f%n", alt, altRam);

            // Call powerAtAltitudeAdvanced which will trigger Logger.debug in variabler
            double power = powerAtAltitudeAdvanced(stage0, alt, false, 0, false, 15);

            System.out.printf("  equationer: power=%.1f%n%n", power);
        }

        // === POWER CURVE (with RAM, 301 km/h IAS) ===
        System.out.println("=== POWER CURVE (military, 301km/h IAS, stage 0) ===\n");

        double speed = 301;
        double airTemp = 15;
        boolean isIAS = true;

        for (int alt : altitudes) {
            double altRam = ramEffectAltitude(alt, airTemp, speed, isIAS, stage0.speedManifoldMult);

            // Calculate intermediate RAM values for debug
            double p = pressure(alt);
            double rho = density(p, airTemp, alt);
            double tasKmh = isIAS ? iasToTas(speed, rho) : speed;
            double tasMs = tasKmh / 3.6;
            double dynamicPressure = (0.5 * rho * tasMs * tasMs * stage0.speedManifoldMult) / 101325.0;
            double totalPressure = p + dynamicPressure;

            System.out.printf("[alt=%d] ramAlt=%.1f (TAS=%.1f, q=%.6f, p_total=%.6f)%n",
                    alt, altRam, tasKmh, dynamicPressure, totalPressure);

            double power = powerAtAltitudeAdvanced(stage0, alt, false, speed, isIAS, airTemp);

            System.out.printf("  equationer: power=%.1f%n%n", power);
        }

        // === OPTIMAL POWER (both stages) ===
        System.out.println("=== OPTIMAL POWER (military, static, both stages) ===\n");

        // Disable debug logging for cleaner output in this section
        Logger.setMinLevel(Logger.Level.INFO);

        for (int alt : altitudes) {
            double power0 = powerAtAltitudeAdvanced(stages[0], alt, false, 0, false, 15);
            double power1 = stages.length > 1
                ? powerAtAltitudeAdvanced(stages[1], alt, false, 0, false, 15)
                : 0;
            double optimal = Math.max(power0, power1);
            int optimalStage = power0 >= power1 ? 0 : 1;

            System.out.printf("[alt=%d] stage0=%.1fhp, stage1=%.1fhp -> optimal=%.1fhp (stage %d)%n",
                    alt, power0, power1, optimal, optimalStage);
        }
    }

    /**
     * Compare parsed blkx values with hardcoded expected values.
     */
    private static void compareWithHardcoded(Blkx blkx) {
        // Expected hardcoded values for Yak-3
        double[][] expected = {
            // Stage 0: Altitude, Power, Ceiling, PowerAtCeiling, ConstRpmAlt, ConstRpmPower
            {300, 1310, 5000, 670, 18300, 1310},
            // Stage 1
            {2600, 1240, 9000, 510, 18300, 1240}
        };

        boolean allMatch = true;
        for (int i = 0; i < Math.min(blkx.compNumSteps, expected.length); i++) {
            System.out.printf("Stage %d comparisons:%n", i);

            allMatch &= checkValue("Altitude" + i, blkx.compAlt[i], expected[i][0]);
            allMatch &= checkValue("Power" + i, blkx.compPower[i], expected[i][1]);
            allMatch &= checkValue("Ceiling" + i, blkx.compCeil[i], expected[i][2]);
            allMatch &= checkValue("PowerAtCeiling" + i, blkx.compCeilPwr[i], expected[i][3]);
            allMatch &= checkValue("AltitudeConstRPM" + i, blkx.compConstRpmAlt[i], expected[i][4]);
            allMatch &= checkValue("PowerConstRPM" + i, blkx.compConstRpmPower[i], expected[i][5]);
        }

        // Other global parameters
        System.out.println("\nGlobal parameters:");
        allMatch &= checkValue("Main.Power", blkx.deckPower, 1290);
        allMatch &= checkValue("ExactAltitudes", blkx.explicitExactAltitudes != null && blkx.explicitExactAltitudes ? 1 : 0, 1);
        allMatch &= checkValue("SpeedManifoldMultiplier", blkx.speedToManifoldMultiplier, 1.0);
        allMatch &= checkValue("AfterburnerBoost", blkx.aftbCoff, 1.0);

        System.out.println();
        if (allMatch) {
            System.out.println(">>> ALL VALUES MATCH HARDCODED EXPECTATIONS <<<");
        } else {
            System.out.println(">>> SOME VALUES DIFFER FROM HARDCODED - SEE MISMATCHES ABOVE <<<");
        }
    }

    /**
     * Check a single value and print comparison result.
     */
    private static boolean checkValue(String name, double actual, double expected) {
        boolean match = Math.abs(actual - expected) < 0.001;
        String status = match ? "OK" : "MISMATCH";
        System.out.printf("  %s: actual=%.1f, expected=%.1f [%s]%n", name, actual, expected, status);
        return match;
    }

    /**
     * Fallback: Run with hardcoded parameters if blkx parsing fails.
     */
    private static void runWithHardcodedParams() {
        System.out.println("=== USING HARDCODED PARAMETERS ===\n");
        CompressorStageParams[] stages = setupYak3ParamsHardcoded();

        System.out.println("=== PARAMETER EXTRACTION (hardcoded) ===\n");
        for (int i = 0; i < stages.length; i++) {
            printStageParams(stages[i], i);
        }

        runPowerCalculations(stages);

        System.out.println("\n======================================================================");
        System.out.println("Debug output complete (using hardcoded params)");
        System.out.println("======================================================================");
    }

    /**
     * Set up Yak-3 FM parameters from hardcoded values (fallback).
     */
    private static CompressorStageParams[] setupYak3ParamsHardcoded() {
        // Stage 0 - from EngineType0.Compressor
        CompressorStageParams stage0 = new CompressorStageParams();
        stage0.critAlt = 300;          // Altitude0
        stage0.critPower = 1310;       // Power0
        stage0.deckPower = 1290;       // Main.Power
        stage0.deckAlt = 0;
        stage0.curvature = 1.0;        // PowerConstRPMCurvature0
        stage0.ceilingAlt = 5000;      // Ceiling0
        stage0.ceilingPower = 670;     // PowerAtCeiling0
        stage0.oldAltitude = 300;      // No adjustment for Yak-3
        stage0.oldPower = 1310;
        stage0.oldPowerNewRpm = 1310;
        stage0.exactAltitudes = true;  // ExactAltitudes=true in FM
        stage0.stageIndex = 0;
        // ConstRPM parameters (exist but "useless" - AltitudeConstRPM=18300 >> Ceiling=5000)
        stage0.constRpmAlt = 18300;    // AltitudeConstRPM0 - ACTUAL VALUE
        stage0.constRpmPower = 1310;   // PowerConstRPM0 - equals Power0
        // No WEP: Afterburner.IsControllable=false, AfterburnerBoost=1
        stage0.wepPowerMult = 1.0;
        stage0.wepCritAlt = 300;
        stage0.wepDeckAlt = 0;
        stage0.stage0DeckAlt = 0;
        stage0.speedManifoldMult = 1.0;  // SpeedManifoldMultiplier

        // Stage 1 - from EngineType0.Compressor
        CompressorStageParams stage1 = new CompressorStageParams();
        stage1.critAlt = 2600;         // Altitude1
        stage1.critPower = 1240;       // Power1
        stage1.deckPower = 1290 * 0.8; // WAPC: deck_power_maker uses 0.8 * prev stage
        stage1.deckAlt = 0;
        stage1.curvature = 1.0;        // PowerConstRPMCurvature1
        stage1.ceilingAlt = 9000;      // Ceiling1
        stage1.ceilingPower = 510;     // PowerAtCeiling1
        stage1.oldAltitude = 2600;
        stage1.oldPower = 1240;
        stage1.oldPowerNewRpm = 1240;
        stage1.exactAltitudes = true;
        stage1.stageIndex = 1;
        // ConstRPM parameters
        stage1.constRpmAlt = 18300;    // AltitudeConstRPM1
        stage1.constRpmPower = 1240;   // PowerConstRPM1 - equals Power1
        // No WEP
        stage1.wepPowerMult = 1.0;
        stage1.wepCritAlt = 2600;
        stage1.wepDeckAlt = 0;
        stage1.stage0DeckAlt = 0;
        stage1.speedManifoldMult = 1.0;

        return new CompressorStageParams[] { stage0, stage1 };
    }

    /**
     * Print all stage parameters for verification.
     */
    private static void printStageParams(CompressorStageParams p, int i) {
        System.out.printf("--- Stage %d Parameters ---%n", i);
        System.out.printf("  critAlt=%.0f, critPower=%.0f%n", p.critAlt, p.critPower);
        System.out.printf("  deckAlt=%.0f, deckPower=%.0f%n", p.deckAlt, p.deckPower);
        System.out.printf("  ceilingAlt=%.0f, ceilingPower=%.0f%n", p.ceilingAlt, p.ceilingPower);
        System.out.printf("  constRpmAlt=%.0f, constRpmPower=%.0f%n", p.constRpmAlt, p.constRpmPower);
        System.out.printf("  oldAltitude=%.0f, oldPower=%.0f, oldPowerNewRpm=%.0f%n",
                p.oldAltitude, p.oldPower, p.oldPowerNewRpm);
        System.out.printf("  wepCritAlt=%.0f, wepPowerMult=%.4f%n", p.wepCritAlt, p.wepPowerMult);
        System.out.printf("  exactAltitudes=%s, curvature=%.1f%n", p.exactAltitudes, p.curvature);
        System.out.printf("  speedManifoldMult=%.1f%n", p.speedManifoldMult);
        System.out.println();
    }

    /**
     * Initialize Lang with minimal default values for standalone test.
     * This avoids the need for ./lang/cur.properties to exist.
     */
    private static void initLangForStandaloneTest() {
        // Set minimal language strings that Blkx.getload() uses
        Lang.bFmVersion = "FM: %s - %s\n";
        Lang.bWeight = "Empty: %.1f kg, Max Fuel: %.1f kg\n";
        Lang.bCritSpeed = "Critical Speed: [%.0f, %.0f] km/h\n";
        Lang.bAllowLoadFactor = "G-limits: [%.1f, %.1f] (full), [%.1f, %.1f] (half fuel)\n";
        Lang.bFlapRestrict = "Flap %d: [%.0f%%, %.0f km/h]\n";
        Lang.bEffSpeedAndPowerLoss = "Eff Speed (E/A/R): %.0f/%.0f/%.0f, Power Loss: %.1f/%.1f/%.1f\n";
        Lang.bNitro = "Nitro: %.1f kg, Duration: %.1f min\n";
        Lang.bAverageHeatRecovery = "Avg Heat Recovery: %.2f\n";
        Lang.bMaxLiftLoad350 = "Max Lift Load: %.2f / %.2f\n";
        Lang.bInertia = "Inertia (P/R/Y): %.0f / %.0f / %.0f\n";
        Lang.bLift = "Wing: %.1f m2, Fuse: %.1f m2, WLL: %.2f / %.2f, Oswald: %.2f, AR: %.1f, Sweep: %.1f\n";
        Lang.bDrag = "CdS: %.3f / %.3f, IndCd: %.4f / %.0f, Rad/Oil: %.3f / %.3f\n";
        Lang.bFmParts = "--- %s ---\n";
        Lang.bCdMin = "CdMin: %.4f\n";
        Lang.bCl0 = "Cl0: %.4f\n";
        Lang.bAoACrit = "AoACrit: [%.1f, %.1f]\n";
        Lang.bAoACritCl = "ClCrit: [%.2f, %.2f]\n";
        Lang.noblkx = "No FM loaded";
    }
}
