import parser.Blkx;
import prog.util.FMPowerExtractor;
import prog.util.PistonPowerModel.CompressorStageParams;
import static prog.util.PistonPowerModel.*;
import static prog.util.AtmosphereModel.*;

import java.io.*;

/**
 * Verifies power curve calculations for Tempest Mk V (150 octane fuel aircraft).
 *
 * This test validates:
 * 1. invertEnableLogic detection for British 150 octane fuel (inverted case)
 * 2. Correct behavior when 150-octane is the DEFAULT fuel (no bonus applied)
 * 3. Power curve accuracy vs wtapc reference values
 *
 * The Tempest Mk V FM file already contains 150-octane power values (invertEnableLogic=true),
 * so VoidMei should NOT apply any fuel bonus.
 *
 * Run with: ./script/test.sh tempest
 */
public class TestTempestMk5Power {

    // wtapc reference values at 300 km/h IAS, 15C
    // Command: python wtapc.py --fm ... --central ... --ias 300
    private static final double[][] WTAPC_MIL = {
        {0, 1982.4}, {1000, 2031.5}, {1730, 2064.7},  // Peak at ~1730m
        {2000, 2001.8}, {3000, 1773.7}, {4000, 1704.3},
        {5000, 1726.7}, {6000, 1615.6}, {7000, 1432.2},
        {8000, 1269.0}, {9000, 1124.1}, {10000, 994.2}
    };

    private static final double[][] WTAPC_WEP = {
        {0, 2439.9},  // Peak at sea level
        {1000, 2223.0}, {2000, 2041.6}, {3000, 2075.9},
        {4000, 2045.9}, {5000, 1844.6}, {6000, 1650.3},
        {7000, 1466.0}, {8000, 1302.0}, {9000, 1156.3},
        {10000, 1025.8}
    };

    private static int passed = 0;
    private static int failed = 0;

    private static String centralPath;
    private static String fmPath;

    public static void main(String[] args) {
        System.out.println("=== Tempest Mk V Power Curve Verification ===\n");

        // Parse command line arguments
        parseArgs(args);

        if (centralPath == null || fmPath == null) {
            System.out.println("Usage: java TestTempestMk5Power --central <path> --fm <path>");
            System.out.println();
            System.out.println("Required files from War Thunder datamine:");
            System.out.println("  --central: flightmodels/tempest_mkv.blkx");
            System.out.println("  --fm:      flightmodels/fm/tempest_mkv.blkx");
            System.exit(1);
        }

        // Run tests
        testInvertEnableLogicDetection();
        testFuelModificationBehavior();
        testParameterExtraction();
        testPowerCurveMilitary();
        testPowerCurveWEP();

        // Summary
        System.out.println("\n=== Results ===");
        System.out.printf("Passed: %d, Failed: %d%n", passed, failed);

        if (failed > 0) {
            System.exit(1);
        }
    }

    private static void parseArgs(String[] args) {
        for (int i = 0; i < args.length - 1; i++) {
            if ("--central".equals(args[i])) {
                centralPath = args[i + 1];
            } else if ("--fm".equals(args[i])) {
                fmPath = args[i + 1];
            }
        }
    }

    // ==================== Test Cases ====================

    /**
     * Tests that invertEnableLogic is correctly parsed as a boolean value,
     * not just detected by keyword presence.
     */
    private static void testInvertEnableLogicDetection() {
        System.out.println("Testing invertEnableLogic detection...");

        String centralData = readFile(centralPath);
        if (centralData == null) {
            System.out.println("  SKIP: Cannot read central file");
            return;
        }

        Blkx.FuelModification fuelMod = Blkx.extractFuelModifications(centralData);

        // Verify fuel type detected
        assertTrue("detected British 150 octane fuel",
            fuelMod.type == Blkx.FuelModification.FuelType.BRITISH_150_OCTANE);

        // Tempest Mk V has invertEnableLogic:b = true
        // This means 150-octane is the DEFAULT fuel state
        assertTrue("invertEnableLogic is true (150 octane is default)",
            fuelMod.britishInvertLogic);

        System.out.printf("  Fuel type: %s%n", fuelMod.type);
        System.out.printf("  invertEnableLogic: %b%n", fuelMod.britishInvertLogic);
    }

    /**
     * Tests that fuel modification is NOT applied when invertEnableLogic=true.
     * Since 150-octane is default, the modification represents REMOVING it.
     */
    private static void testFuelModificationBehavior() {
        System.out.println("\nTesting fuel modification behavior (invertEnableLogic=true)...");

        Blkx blkx = new Blkx(fmPath, "tempest_mkv", true);
        String centralData = readFile(centralPath);
        Blkx.FuelModification fuelMod = Blkx.extractFuelModifications(centralData);

        if (!blkx.valid || fuelMod == null) {
            System.out.println("  SKIP: Cannot load files");
            return;
        }

        // Extract stages with and without fuel modification
        CompressorStageParams[] stagesNoFuel = FMPowerExtractor.extractStages(blkx);
        CompressorStageParams[] stagesWithFuel = FMPowerExtractor.extractStages(blkx, fuelMod);

        if (stagesNoFuel != null && stagesWithFuel != null) {
            // With invertEnableLogic=true, fuel mod should NOT change WEP params
            boolean wepUnchanged = true;
            for (int i = 0; i < stagesNoFuel.length; i++) {
                double noFuelMult = stagesNoFuel[i].wepPowerMult;
                double withFuelMult = stagesWithFuel[i].wepPowerMult;
                double noFuelWepAlt = stagesNoFuel[i].wepCritAlt;
                double withFuelWepAlt = stagesWithFuel[i].wepCritAlt;

                System.out.printf("  Stage %d: wepMult %.4f → %.4f, wepCritAlt %.0f → %.0f%n",
                    i, noFuelMult, withFuelMult, noFuelWepAlt, withFuelWepAlt);

                if (Math.abs(noFuelMult - withFuelMult) > 0.001 ||
                    Math.abs(noFuelWepAlt - withFuelWepAlt) > 1) {
                    wepUnchanged = false;
                }
            }

            assertTrue("WEP parameters unchanged (invertEnableLogic=true means no bonus)", wepUnchanged);
        }
    }

    private static void testParameterExtraction() {
        System.out.println("\nTesting parameter extraction...");

        Blkx blkx = new Blkx(fmPath, "tempest_mkv", true);
        if (!blkx.valid) {
            System.out.println("  SKIP: Cannot parse FM file");
            return;
        }

        // Since invertEnableLogic=true, we can extract with or without fuel mod - same result
        CompressorStageParams[] stages = FMPowerExtractor.extractStages(blkx);
        assertNotNull("extracted stages", stages);
        assertTrue("has 2 stages", stages != null && stages.length == 2);

        if (stages != null) {
            System.out.println("\n  === Extracted Stage Parameters ===");
            for (int i = 0; i < stages.length; i++) {
                CompressorStageParams s = stages[i];
                System.out.printf("  Stage %d:%n", i);
                System.out.printf("    critAlt: %.0fm, critPower: %.1fhp%n", s.critAlt, s.critPower);
                System.out.printf("    deckPower: %.1fhp%n", s.deckPower);
                System.out.printf("    wepCritAlt: %.0fm, wepPowerMult: %.4f%n", s.wepCritAlt, s.wepPowerMult);
            }
        }

        // Verify expected FM parameters (specific to Tempest Mk V)
        assertClose("compressor NumSteps", blkx.compNumSteps, 2, 0);
        // Stage 0 critical altitude should be around 1730m
        assertClose("Stage 0 altitude", blkx.compAlt[0], 1730, 50);
        // Stage 1 critical altitude should be around 5000m
        assertClose("Stage 1 altitude", blkx.compAlt[1], 5000, 200);
    }

    private static void testPowerCurveMilitary() {
        System.out.println("\nTesting military power curve vs wtapc...");

        Blkx blkx = new Blkx(fmPath, "tempest_mkv", true);
        if (!blkx.valid) {
            System.out.println("  SKIP: Cannot parse FM file");
            return;
        }

        // For Tempest Mk V, since invertEnableLogic=true, fuel mod doesn't change anything
        // Use stages directly without fuel modification (or with - same result)
        CompressorStageParams[] stages = FMPowerExtractor.extractStages(blkx);
        if (stages == null) {
            System.out.println("  SKIP: Cannot extract stages");
            return;
        }

        double speedKmh = 300.0;
        boolean isIAS = true;
        double seaLevelTemp = 15.0;

        System.out.println("\n  === Military Power Curve (300 km/h IAS) ===");
        System.out.println("  Alt(m)    VoidMei    wtapc    Diff");
        System.out.println("  ------    -------    -----    ----");

        int errors = 0;
        double maxError = 0;
        for (double[] ref : WTAPC_MIL) {
            double alt = ref[0];
            double expected = ref[1];
            double actual = optimalPowerAdvanced(stages, alt, false, speedKmh, isIAS, seaLevelTemp);
            double diff = actual - expected;
            double absDiff = Math.abs(diff);

            String status = absDiff < 5.0 ? "OK" : (absDiff < 20.0 ? "~" : "X");
            System.out.printf("  %5.0f    %7.1f    %5.1f    %+.1f %s%n",
                alt, actual, expected, diff, status);

            if (absDiff > 5.0) errors++;
            if (absDiff > maxError) maxError = absDiff;
        }

        System.out.printf("\n  Max error: %.1f hp%n", maxError);
        assertTrue("Military max error < 5 hp", maxError < 5.0);
    }

    private static void testPowerCurveWEP() {
        System.out.println("\nTesting WEP power curve vs wtapc...");

        Blkx blkx = new Blkx(fmPath, "tempest_mkv", true);
        if (!blkx.valid) {
            System.out.println("  SKIP: Cannot parse FM file");
            return;
        }

        CompressorStageParams[] stages = FMPowerExtractor.extractStages(blkx);
        if (stages == null) {
            System.out.println("  SKIP: Cannot extract stages");
            return;
        }

        double speedKmh = 300.0;
        boolean isIAS = true;
        double seaLevelTemp = 15.0;

        System.out.println("\n  === WEP Power Curve (300 km/h IAS) ===");
        System.out.println("  Alt(m)    VoidMei    wtapc    Diff");
        System.out.println("  ------    -------    -----    ----");

        int errors = 0;
        double maxError = 0;
        for (double[] ref : WTAPC_WEP) {
            double alt = ref[0];
            double expected = ref[1];
            double actual = optimalPowerAdvanced(stages, alt, true, speedKmh, isIAS, seaLevelTemp);
            double diff = actual - expected;
            double absDiff = Math.abs(diff);

            String status = absDiff < 10.0 ? "OK" : (absDiff < 50.0 ? "~" : "X");
            System.out.printf("  %5.0f    %7.1f    %5.1f    %+.1f %s%n",
                alt, actual, expected, diff, status);

            if (absDiff > 10.0) errors++;
            if (absDiff > maxError) maxError = absDiff;
        }

        // Find peak values
        double wepPeakPower = 0, wepPeakAlt = 0;
        for (int alt = 0; alt <= 10000; alt += 50) {
            double wepPower = optimalPowerAdvanced(stages, alt, true, speedKmh, isIAS, seaLevelTemp);
            if (wepPower > wepPeakPower) {
                wepPeakPower = wepPower;
                wepPeakAlt = alt;
            }
        }

        System.out.printf("\n  Max error: %.1f hp%n", maxError);
        System.out.printf("  VoidMei WEP peak: %.1f hp @ %.0fm (wtapc: 2439.9 hp @ 0m)%n", wepPeakPower, wepPeakAlt);

        // Acceptance criteria
        assertTrue("WEP max error < 10 hp", maxError < 10.0);
        assertClose("WEP peak power", wepPeakPower, 2439.9, 10.0);
        assertClose("WEP peak altitude", wepPeakAlt, 0, 100);
    }

    // ==================== Utility Methods ====================

    private static String readFile(String path) {
        try {
            StringBuilder sb = new StringBuilder();
            BufferedReader br = new BufferedReader(new FileReader(path));
            String line;
            while ((line = br.readLine()) != null) {
                sb.append(line).append("\n");
            }
            br.close();
            return sb.toString();
        } catch (IOException e) {
            System.err.println("Error reading file: " + path);
            return null;
        }
    }

    private static void assertClose(String name, double actual, double expected, double tolerance) {
        if (Math.abs(actual - expected) <= tolerance) {
            System.out.printf("  PASS: %s = %.2f (expected %.2f +/- %.2f)%n", name, actual, expected, tolerance);
            passed++;
        } else {
            System.out.printf("  FAIL: %s = %.2f (expected %.2f +/- %.2f)%n",
                name, actual, expected, tolerance);
            failed++;
        }
    }

    private static void assertTrue(String name, boolean condition) {
        if (condition) {
            System.out.printf("  PASS: %s%n", name);
            passed++;
        } else {
            System.out.printf("  FAIL: %s%n", name);
            failed++;
        }
    }

    private static void assertNotNull(String name, Object obj) {
        assertTrue(name + " not null", obj != null);
    }
}
