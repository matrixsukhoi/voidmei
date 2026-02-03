import prog.util.PistonPowerModel;
import prog.util.PistonPowerModel.CompressorStageParams;
import static prog.util.PistonPowerModel.*;

/**
 * Tests for PistonPowerModel.
 *
 * Validates piston engine power curve calculations.
 *
 * Run with: ./script/test.sh
 */
public class TestPistonPowerModel {

    private static int passed = 0;
    private static int failed = 0;

    public static void main(String[] args) {
        System.out.println("=== PistonPowerModel Tests ===\n");

        testTorqueRpmBoost();
        testTorqueFromHp();
        testSuperchargerRpmEffect();
        testInterpolatePower();
        testWepPowerMultiplier();
        testWepCriticalAltitude();
        testPowerAtAltitude();
        testMultiStageSupercharger();
        testGeneratePowerCurve();
        testRamEffectIntegration();

        System.out.println("\n=== Results ===");
        System.out.printf("Passed: %d, Failed: %d%n", passed, failed);

        if (failed > 0) {
            System.exit(1);
        }
    }

    private static void testTorqueRpmBoost() {
        System.out.println("Testing torqueRpmBoost()...");

        // Same RPM = no boost
        assertClose("same RPM", torqueRpmBoost(2400, 2400), 1.0, 0.001);

        // Higher RPM = boost > 1
        double boost = torqueRpmBoost(2400, 2600);
        assertTrue("higher RPM gives boost", boost > 1.0);
        System.out.printf("  torqueRpmBoost(2400, 2600) = %.4f%n", boost);

        // Invalid inputs
        assertClose("zero lowerRPM", torqueRpmBoost(0, 2600), 1.0, 0.001);
        assertClose("zero higherRPM", torqueRpmBoost(2400, 0), 1.0, 0.001);

        // Boost is monotonic with RPM difference
        double boost1 = torqueRpmBoost(2400, 2500);
        double boost2 = torqueRpmBoost(2400, 2600);
        double boost3 = torqueRpmBoost(2400, 2700);
        assertTrue("boost increases with RPM diff", boost1 < boost2 && boost2 < boost3);
    }

    private static void testTorqueFromHp() {
        System.out.println("Testing torqueFromHp()...");

        // Known conversion: 726.115 * hp / rpm
        double torque = torqueFromHp(1000, 2400);
        assertClose("torque at 1000hp/2400rpm", torque, 302.55, 0.1);

        // Zero RPM handling
        assertClose("zero RPM", torqueFromHp(1000, 0), 0, 0.001);

        // Torque proportional to power
        double torque2 = torqueFromHp(2000, 2400);
        assertClose("double power = double torque", torque2, torque * 2, 0.1);

        // Torque inversely proportional to RPM
        double torque3 = torqueFromHp(1000, 4800);
        assertClose("double RPM = half torque", torque3, torque / 2, 0.1);
    }

    private static void testSuperchargerRpmEffect() {
        System.out.println("Testing superchargerRpmEffect()...");

        // Typical parameters
        double effect = superchargerRpmEffect(2400, 2600, 0.2, 0.1);
        assertTrue("effect > 1 with RPM increase", effect > 1.0);
        System.out.printf("  superchargerRpmEffect(2400, 2600, 0.2, 0.1) = %.4f%n", effect);

        // No RPM difference = no effect
        assertClose("same RPM", superchargerRpmEffect(2400, 2400, 0.2, 0.1), 1.0, 0.001);

        // Invalid input
        assertClose("zero milRPM", superchargerRpmEffect(0, 2600, 0.2, 0.1), 1.0, 0.001);

        // Higher compressor factors = more effect
        double effect1 = superchargerRpmEffect(2400, 2600, 0.1, 0.1);
        double effect2 = superchargerRpmEffect(2400, 2600, 0.3, 0.1);
        assertTrue("lower pressureAtRPM0 = more effect", effect1 > effect2);
    }

    private static void testInterpolatePower() {
        System.out.println("Testing interpolatePower()...");

        // At lower altitude point
        double atLower = interpolatePower(1800, 5000, 2000, 0, 0, 1.0);
        assertClose("at lower point", atLower, 2000, 1);

        // At higher altitude point
        double atHigher = interpolatePower(1800, 5000, 2000, 0, 5000, 1.0);
        assertClose("at higher point", atHigher, 1800, 1);

        // In between (should be between the two values)
        double atMid = interpolatePower(1800, 5000, 2000, 0, 2500, 1.0);
        assertTrue("mid point between", atMid > 1800 && atMid < 2000);

        // Curvature effect
        double linear = interpolatePower(1800, 5000, 2000, 0, 2500, 1.0);
        double curved = interpolatePower(1800, 5000, 2000, 0, 2500, 2.0);
        assertTrue("curvature affects interpolation", Math.abs(linear - curved) > 1);
    }

    private static void testWepPowerMultiplier() {
        System.out.println("Testing wepPowerMultiplier()...");

        // All factors = 1 and same RPM = multiplier of 1
        assertClose("baseline", wepPowerMultiplier(1.0, 1.0, 1.0, 1.0, 2400, 2400), 1.0, 0.01);

        // Typical WEP parameters
        double mult = wepPowerMultiplier(1.15, 1.0, 1.0, 1.0, 2400, 2600);
        assertTrue("WEP mult > 1", mult > 1.0);
        System.out.printf("  wepPowerMultiplier(1.15, 1.0, 1.0, 1.0, 2400, 2600) = %.4f%n", mult);

        // AfterburnerBoost effect
        double mult1 = wepPowerMultiplier(1.10, 1.0, 1.0, 1.0, 2400, 2400);
        double mult2 = wepPowerMultiplier(1.20, 1.0, 1.0, 1.0, 2400, 2400);
        assertTrue("higher boost = higher mult", mult2 > mult1);
    }

    private static void testWepCriticalAltitude() {
        System.out.println("Testing wepCriticalAltitude()...");

        // WEP requires higher manifold pressure, so critical altitude is lower
        double wepCritAlt = wepCriticalAltitude(7000, 1.42, 1.65, 1.0, 1.0);
        assertTrue("WEP crit alt < mil crit alt", wepCritAlt < 7000);
        System.out.printf("  wepCriticalAltitude(7000, 1.42ata, 1.65ata) = %.0fm%n", wepCritAlt);

        // With supercharger RPM effect boost
        double wepCritAlt2 = wepCriticalAltitude(7000, 1.42, 1.65, 1.1, 1.0);
        assertTrue("RPM effect raises WEP crit alt", wepCritAlt2 > wepCritAlt);

        // With pressure boost
        double wepCritAlt3 = wepCriticalAltitude(7000, 1.42, 1.65, 1.0, 1.1);
        assertTrue("pressure boost raises WEP crit alt", wepCritAlt3 > wepCritAlt);
    }

    private static void testPowerAtAltitude() {
        System.out.println("Testing powerAtAltitude()...");

        // Create test engine (simplified P-47D-like)
        CompressorStageParams stage = new CompressorStageParams();
        stage.critAlt = 7000;
        stage.critPower = 2000;
        stage.deckPower = 1850;
        stage.deckAlt = 0;
        stage.curvature = 1.0;
        stage.wepCritAlt = 6000;
        stage.wepPowerMult = 1.15;
        stage.speedManifoldMult = 0.9;

        // At sea level
        double p0 = powerAtAltitude(stage, 0, false, 0, false, 15);
        assertClose("power at sea level", p0, 1850, 10);

        // At critical altitude
        double pCrit = powerAtAltitude(stage, 7000, false, 0, false, 15);
        assertClose("power at crit alt", pCrit, 2000, 10);

        // Above critical altitude (power drops)
        double pHigh = powerAtAltitude(stage, 9000, false, 0, false, 15);
        assertTrue("power drops above crit alt", pHigh < pCrit);

        // WEP gives more power
        double pWep = powerAtAltitude(stage, 5000, true, 0, false, 15);
        double pMil = powerAtAltitude(stage, 5000, false, 0, false, 15);
        assertTrue("WEP > military", pWep > pMil);

        // Print power curve
        System.out.println("  Power curve (military):");
        for (int alt = 0; alt <= 10000; alt += 2000) {
            double power = powerAtAltitude(stage, alt, false, 0, false, 15);
            System.out.printf("    %5dm: %.0f hp%n", alt, power);
        }
    }

    private static void testMultiStageSupercharger() {
        System.out.println("Testing multi-stage supercharger...");

        // Two-stage supercharger (like Spitfire Merlin)
        CompressorStageParams stage1 = new CompressorStageParams();
        stage1.critAlt = 3000;
        stage1.critPower = 1400;
        stage1.deckPower = 1350;
        stage1.wepCritAlt = 2500;
        stage1.wepPowerMult = 1.1;
        stage1.stageIndex = 0;

        CompressorStageParams stage2 = new CompressorStageParams();
        stage2.critAlt = 6500;
        stage2.critPower = 1300;
        stage2.deckPower = 1100;
        stage2.wepCritAlt = 6000;
        stage2.wepPowerMult = 1.1;
        stage2.stageIndex = 1;

        CompressorStageParams[] stages = {stage1, stage2};

        // At low altitude, stage 1 is better
        double pLow = optimalPowerAtAltitude(stages, 1000, false, 0, false, 15);
        double p1Low = powerAtAltitude(stage1, 1000, false, 0, false, 15);
        double p2Low = powerAtAltitude(stage2, 1000, false, 0, false, 15);
        assertTrue("stage 1 better at low alt", p1Low > p2Low);
        assertClose("optimal selects stage 1", pLow, p1Low, 1);

        // At high altitude, stage 2 is better
        double pHigh = optimalPowerAtAltitude(stages, 6000, false, 0, false, 15);
        double p1High = powerAtAltitude(stage1, 6000, false, 0, false, 15);
        double p2High = powerAtAltitude(stage2, 6000, false, 0, false, 15);
        assertTrue("stage 2 better at high alt", p2High > p1High);
        assertClose("optimal selects stage 2", pHigh, p2High, 1);

        // Print combined curve
        System.out.println("  Two-stage power curve:");
        for (int alt = 0; alt <= 8000; alt += 1000) {
            double power = optimalPowerAtAltitude(stages, alt, false, 0, false, 15);
            System.out.printf("    %5dm: %.0f hp%n", alt, power);
        }
    }

    private static void testGeneratePowerCurve() {
        System.out.println("Testing generatePowerCurve()...");

        CompressorStageParams stage = new CompressorStageParams(5000, 1500, 1400);
        stage.wepCritAlt = 4500;
        stage.wepPowerMult = 1.1;
        CompressorStageParams[] stages = {stage};

        double[] curve = generatePowerCurve(stages, false, 0, false, 15, 50);

        // Check array size (range: 0m to 10000m, step 50m)
        int expectedSize = (10000 - 0) / 50 + 1;  // 201 points
        assertTrue("curve size correct", curve.length == expectedSize);

        // Check values at known altitudes (index = altitude / step)
        int seaLevelIdx = 0;
        assertClose("curve at sea level", curve[seaLevelIdx], 1400, 10);

        // Index at 5000m = 5000 / 50 = 100
        int critAltIdx = 5000 / 50;
        assertClose("curve at crit alt", curve[critAltIdx], 1500, 10);

        // Power decreases after critical altitude (8000m = index 160)
        int highAltIdx = 8000 / 50;
        assertTrue("power decreases at high alt", curve[highAltIdx] < curve[critAltIdx]);
    }

    private static void testRamEffectIntegration() {
        System.out.println("Testing RAM effect integration...");

        CompressorStageParams stage = new CompressorStageParams();
        stage.critAlt = 7000;
        stage.critPower = 2000;
        stage.deckPower = 1850;
        stage.wepCritAlt = 6500;
        stage.wepPowerMult = 1.1;
        stage.speedManifoldMult = 0.9;

        // Test RAM effect well ABOVE critical altitude where power drops with altitude
        // Use higher altitude to avoid effective altitude crossing critical altitude
        double pStatic12k = powerAtAltitude(stage, 12000, false, 0, false, 15);
        double pMoving12k = powerAtAltitude(stage, 12000, false, 400, true, 15);

        // RAM effect should increase power above critical altitude
        assertTrue("RAM increases power above crit alt", pMoving12k > pStatic12k);
        System.out.printf("  At 12000m: static=%.0fhp, 400km/h=%.0fhp (gain: %.0fhp)%n",
                pStatic12k, pMoving12k, pMoving12k - pStatic12k);

        // Higher speed = more RAM effect (use moderate speeds to stay above crit alt)
        double pFaster12k = powerAtAltitude(stage, 12000, false, 500, true, 15);
        assertTrue("faster = more RAM above crit alt", pFaster12k > pMoving12k);
        System.out.printf("  At 12000m: 400km/h=%.0fhp, 500km/h=%.0fhp%n", pMoving12k, pFaster12k);

        // Note: Below critical altitude, RAM effect may DECREASE power because
        // in this model deck power < crit power (power increases toward crit alt).
        // This is physically correct for supercharged engines where the
        // supercharger maintains constant boost up to critical altitude.
        double pStaticLow = powerAtAltitude(stage, 5000, false, 0, false, 15);
        double pMovingLow = powerAtAltitude(stage, 5000, false, 500, true, 15);
        System.out.printf("  At 5000m (below crit): static=%.0fhp, 500km/h=%.0fhp%n",
                pStaticLow, pMovingLow);
        System.out.println("  (Below crit alt, RAM lowers effective alt, which may reduce power)");
    }

    // === Assertion Helpers ===

    private static void assertClose(String name, double actual, double expected, double tolerance) {
        if (Math.abs(actual - expected) <= tolerance) {
            System.out.printf("  PASS: %s = %.4f (expected %.4f)%n", name, actual, expected);
            passed++;
        } else {
            System.out.printf("  FAIL: %s = %.4f (expected %.4f, tolerance %.4f)%n",
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
}
