import parser.Blkx;
import prog.i18n.Lang;
import prog.util.FMPowerExtractor;
import prog.util.Logger;
import prog.util.PistonPowerModel;
import prog.util.PistonPowerModel.CompressorStageParams;

import static prog.util.PistonPowerModel.*;

/**
 * Quick verification: F4U-4B power curve vs WAPC reference.
 *
 * Usage:
 *   javac -encoding UTF-8 -d bin -classpath bin test/VerifyF4U4B.java
 *   java -classpath bin VerifyF4U4B
 */
public class VerifyF4U4B {

    private static final String FM_PATH =
        "/home/tu10ng/Downloads/voidmei/data/aces/gamedata/flightmodels/fm/f4u-4b.blkx";

    // WAPC reference (300 km/h IAS, octane=true, alt_tick=10)
    private static final double[][] WAPC_MIL = {
        {0, 2300.0}, {1000, 2160.8}, {2000, 2000.0}, {3000, 2000.0},
        {4000, 2000.0}, {5000, 2000.0}, {6000, 1903.3}, {7000, 1850.0},
        {8000, 1768.7}, {9000, 1541.0}, {10000, 1337.6}
    };
    private static final double[][] WAPC_WEP = {
        {0, 2688.0}, {1000, 2602.6}, {2000, 2602.6}, {3000, 2602.6},
        {4000, 2602.6}, {5000, 2407.4}, {6000, 2407.4}, {7000, 2241.9},
        {8000, 1957.3}, {9000, 1705.3}, {10000, 1480.2}
    };

    public static void main(String[] args) {
        Logger.setMinLevel(Logger.Level.INFO);
        initLang();

        System.out.println("=== F4U-4B Power Curve Verification ===\n");

        Blkx blkx = new Blkx(FM_PATH, "f4u-4b");
        if (!blkx.valid) {
            System.err.println("ERROR: Failed to parse FM file");
            return;
        }

        System.out.println("Parsed FM: compNumSteps=" + blkx.compNumSteps
            + ", deckPower=" + blkx.deckPower
            + ", speedManifoldMult=" + blkx.speedToManifoldMultiplier
            + ", aftbCoff=" + blkx.aftbCoff);

        // Print raw compressor data
        for (int i = 0; i < blkx.compNumSteps; i++) {
            System.out.printf("  Stage %d: Alt=%.0f, Pwr=%.0f, Ceil=%.0f, CeilPwr=%.0f, ConstAlt=%.0f, ConstPwr=%.0f%n",
                i, blkx.compAlt[i], blkx.compPower[i], blkx.compCeil[i], blkx.compCeilPwr[i],
                blkx.compConstRpmAlt[i], blkx.compConstRpmPower[i]);
        }

        // No fuel modification for F4U-4B
        CompressorStageParams[] stages = FMPowerExtractor.extractStages(blkx);
        if (stages == null || stages.length == 0) {
            System.err.println("ERROR: extractStages returned null");
            return;
        }

        System.out.println("\nExtracted " + stages.length + " stages:");
        for (int i = 0; i < stages.length; i++) {
            CompressorStageParams s = stages[i];
            System.out.printf("  Stage %d: critAlt=%.0f critPwr=%.1f deckAlt=%.0f deckPwr=%.1f " +
                "ceilAlt=%.0f ceilPwr=%.1f wepMult=%.4f wepCritAlt=%.0f wepDeckAlt=%.0f " +
                "oldAlt=%.0f oldPwr=%.1f constAlt=%.0f constPwr=%.1f exact=%s curv=%.2f smm=%.2f%n",
                i, s.critAlt, s.critPower, s.deckAlt, s.deckPower,
                s.ceilingAlt, s.ceilingPower, s.wepPowerMult, s.wepCritAlt, s.wepDeckAlt,
                s.oldAltitude, s.oldPower, s.constRpmAlt, s.constRpmPower,
                s.exactAltitudes, s.curvature, s.speedManifoldMult);
        }

        // === MILITARY comparison ===
        System.out.println("\n--- MILITARY (300 km/h IAS) ---");
        System.out.printf("%-8s %-12s %-12s %-10s%n", "Alt(m)", "WAPC(hp)", "VoidMei(hp)", "Diff(hp)");
        System.out.println("-------------------------------------------");

        double milMaxDiff = 0;
        for (double[] ref : WAPC_MIL) {
            int alt = (int) ref[0];
            double wapcPwr = ref[1];
            double ourPwr = optimalPowerAdvanced(stages, alt, false, 300, true, 15);
            double diff = ourPwr - wapcPwr;
            if (Math.abs(diff) > milMaxDiff) milMaxDiff = Math.abs(diff);
            String status = Math.abs(diff) < 1.0 ? "" : (Math.abs(diff) < 5.0 ? " ~" : " !!!");
            System.out.printf("%-8d %-12.1f %-12.1f %-10.1f%s%n", alt, wapcPwr, ourPwr, diff, status);
        }

        // === WEP comparison ===
        System.out.println("\n--- WEP (300 km/h IAS) ---");
        System.out.printf("%-8s %-12s %-12s %-10s%n", "Alt(m)", "WAPC(hp)", "VoidMei(hp)", "Diff(hp)");
        System.out.println("-------------------------------------------");

        double wepMaxDiff = 0;
        for (double[] ref : WAPC_WEP) {
            int alt = (int) ref[0];
            double wapcPwr = ref[1];
            double ourPwr = optimalPowerAdvanced(stages, alt, true, 300, true, 15);
            double diff = ourPwr - wapcPwr;
            if (Math.abs(diff) > wepMaxDiff) wepMaxDiff = Math.abs(diff);
            String status = Math.abs(diff) < 1.0 ? "" : (Math.abs(diff) < 5.0 ? " ~" : " !!!");
            System.out.printf("%-8d %-12.1f %-12.1f %-10.1f%s%n", alt, wapcPwr, ourPwr, diff, status);
        }

        System.out.printf("%nMilitary max diff: %.1f hp%n", milMaxDiff);
        System.out.printf("WEP max diff: %.1f hp%n", wepMaxDiff);

        if (milMaxDiff < 1.0 && wepMaxDiff < 1.0) {
            System.out.println("\n>>> ALL VALUES MATCH WAPC (< 1 hp tolerance) <<<");
        } else if (milMaxDiff < 5.0 && wepMaxDiff < 5.0) {
            System.out.println("\n>>> CLOSE MATCH (< 5 hp tolerance) <<<");
        } else {
            System.out.println("\n>>> SIGNIFICANT DIFFERENCES DETECTED <<<");
        }
    }

    private static void initLang() {
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
