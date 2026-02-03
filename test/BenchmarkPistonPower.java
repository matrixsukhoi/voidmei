import prog.util.PistonPowerModel;
import prog.util.PistonPowerModel.CompressorStageParams;
import static prog.util.PistonPowerModel.*;

/**
 * Performance benchmark for PistonPowerModel.generatePowerCurveAdvanced.
 *
 * Run with:
 *   javac -encoding UTF-8 -d bin -classpath 'dep/*' @sources.txt test/BenchmarkPistonPower.java
 *   java -cp bin BenchmarkPistonPower
 */
public class BenchmarkPistonPower {

    public static void main(String[] args) {
        System.out.println("=== PistonPowerModel Performance Benchmark ===\n");

        // Test configurations
        int warmupIterations = 100;
        int benchmarkIterations = 1000;

        // Create realistic test data (bf-109f-4 style single-stage)
        CompressorStageParams[] singleStage = createBf109F4Params();

        // Create multi-stage test data (Spitfire style two-stage)
        CompressorStageParams[] twoStage = createTwoStageParams();

        // Create three-stage test data (hypothetical)
        CompressorStageParams[] threeStage = createThreeStageParams();

        System.out.println("Configuration:");
        System.out.println("  Warmup iterations: " + warmupIterations);
        System.out.println("  Benchmark iterations: " + benchmarkIterations);
        System.out.println("  Altitude range: 0m to 10000m");
        System.out.println("  Altitude step: 50m (201 points per curve)");
        System.out.println();

        // Warmup JIT
        System.out.println("Warming up JIT...");
        for (int i = 0; i < warmupIterations; i++) {
            generatePowerCurveAdvanced(singleStage, false, 0, false, 15, 50);
            generatePowerCurveAdvanced(singleStage, true, 0, false, 15, 50);
            generatePowerCurveAdvanced(twoStage, false, 400, true, 15, 50);
        }
        System.out.println("Warmup complete.\n");

        // Benchmark 1: Single stage, military mode, static
        benchmarkMethod("Single stage, Military, Static", () -> {
            generatePowerCurveAdvanced(singleStage, false, 0, false, 15, 50);
        }, benchmarkIterations);

        // Benchmark 2: Single stage, WEP mode, static
        benchmarkMethod("Single stage, WEP, Static", () -> {
            generatePowerCurveAdvanced(singleStage, true, 0, false, 15, 50);
        }, benchmarkIterations);

        // Benchmark 3: Single stage, military, with RAM effect
        benchmarkMethod("Single stage, Military, RAM 400km/h", () -> {
            generatePowerCurveAdvanced(singleStage, false, 400, true, 15, 50);
        }, benchmarkIterations);

        // Benchmark 4: Single stage, WEP, with RAM effect
        benchmarkMethod("Single stage, WEP, RAM 400km/h", () -> {
            generatePowerCurveAdvanced(singleStage, true, 400, true, 15, 50);
        }, benchmarkIterations);

        // Benchmark 5: Two stage supercharger
        benchmarkMethod("Two stage, Military, Static", () -> {
            generatePowerCurveAdvanced(twoStage, false, 0, false, 15, 50);
        }, benchmarkIterations);

        // Benchmark 6: Two stage, WEP, RAM
        benchmarkMethod("Two stage, WEP, RAM 400km/h", () -> {
            generatePowerCurveAdvanced(twoStage, true, 400, true, 15, 50);
        }, benchmarkIterations);

        // Benchmark 7: Three stage supercharger
        benchmarkMethod("Three stage, Military, Static", () -> {
            generatePowerCurveAdvanced(threeStage, false, 0, false, 15, 50);
        }, benchmarkIterations);

        // Benchmark 8: Three stage, WEP, RAM
        benchmarkMethod("Three stage, WEP, RAM 400km/h", () -> {
            generatePowerCurveAdvanced(threeStage, true, 400, true, 15, 50);
        }, benchmarkIterations);

        // Also benchmark the simpler generatePowerCurve for comparison
        System.out.println("\n--- Comparison with simpler generatePowerCurve ---\n");

        benchmarkMethod("generatePowerCurve (simple)", () -> {
            generatePowerCurve(singleStage, false, 0, false, 15, 50);
        }, benchmarkIterations);

        benchmarkMethod("generatePowerCurveAdvanced", () -> {
            generatePowerCurveAdvanced(singleStage, false, 0, false, 15, 50);
        }, benchmarkIterations);

        // Memory allocation check
        System.out.println("\n--- Memory Analysis ---");
        System.gc();
        long beforeMem = Runtime.getRuntime().totalMemory() - Runtime.getRuntime().freeMemory();

        double[][] results = new double[1000][];
        for (int i = 0; i < 1000; i++) {
            results[i] = generatePowerCurveAdvanced(singleStage, i % 2 == 0, 0, false, 15, 50);
        }

        long afterMem = Runtime.getRuntime().totalMemory() - Runtime.getRuntime().freeMemory();
        System.out.printf("Memory used for 1000 curves: %.2f KB%n", (afterMem - beforeMem) / 1024.0);
        System.out.printf("Per curve allocation: ~%.0f bytes (201 doubles * 8 = 1608 bytes expected)%n",
            (afterMem - beforeMem) / 1000.0);

        // Keep results alive to prevent GC
        System.out.println("Results array length: " + results.length);
    }

    private static void benchmarkMethod(String name, Runnable method, int iterations) {
        long startTime = System.nanoTime();
        for (int i = 0; i < iterations; i++) {
            method.run();
        }
        long endTime = System.nanoTime();

        double totalMs = (endTime - startTime) / 1_000_000.0;
        double avgUs = (endTime - startTime) / 1_000.0 / iterations;
        double avgMs = totalMs / iterations;

        System.out.printf("%-38s: %8.3f ms total, %8.3f µs/call (%7.4f ms/call)%n",
            name, totalMs, avgUs, avgMs);
    }

    /**
     * Creates bf-109f-4 style single-stage parameters with full WAPC fields.
     */
    private static CompressorStageParams[] createBf109F4Params() {
        CompressorStageParams stage = new CompressorStageParams();
        stage.critAlt = 5200;
        stage.critPower = 1200;
        stage.deckPower = 1120;
        stage.deckAlt = 0;
        stage.curvature = 1.0;
        stage.wepCritAlt = 4620;
        stage.wepPowerMult = 1.144;  // After the bug fix!
        stage.speedManifoldMult = 0.9;
        stage.stageIndex = 0;

        // Advanced WAPC fields
        stage.oldAltitude = 5200;
        stage.oldPower = 1200;
        stage.oldPowerNewRpm = 1200;
        stage.ceilingAlt = 11000;
        stage.ceilingPower = 550;
        stage.constRpmAlt = 0;
        stage.constRpmPower = 0;
        stage.wepDeckAlt = 0;
        stage.wepConstRpmAlt = 0;
        stage.stage0DeckAlt = 0;
        stage.exactAltitudes = false;

        return new CompressorStageParams[] { stage };
    }

    /**
     * Creates Spitfire style two-stage supercharger parameters.
     */
    private static CompressorStageParams[] createTwoStageParams() {
        CompressorStageParams stage1 = new CompressorStageParams();
        stage1.critAlt = 3000;
        stage1.critPower = 1400;
        stage1.deckPower = 1350;
        stage1.deckAlt = 0;
        stage1.curvature = 1.0;
        stage1.wepCritAlt = 2500;
        stage1.wepPowerMult = 1.1;
        stage1.speedManifoldMult = 0.85;
        stage1.stageIndex = 0;
        stage1.oldAltitude = 3000;
        stage1.oldPower = 1400;
        stage1.oldPowerNewRpm = 1400;
        stage1.ceilingAlt = 8000;
        stage1.ceilingPower = 800;
        stage1.exactAltitudes = false;

        CompressorStageParams stage2 = new CompressorStageParams();
        stage2.critAlt = 6500;
        stage2.critPower = 1300;
        stage2.deckPower = 1100;
        stage2.deckAlt = 0;
        stage2.curvature = 1.0;
        stage2.wepCritAlt = 6000;
        stage2.wepPowerMult = 1.1;
        stage2.speedManifoldMult = 0.85;
        stage2.stageIndex = 1;
        stage2.oldAltitude = 6500;
        stage2.oldPower = 1300;
        stage2.oldPowerNewRpm = 1300;
        stage2.ceilingAlt = 12000;
        stage2.ceilingPower = 600;
        stage2.exactAltitudes = false;

        return new CompressorStageParams[] { stage1, stage2 };
    }

    /**
     * Creates hypothetical three-stage supercharger parameters.
     */
    private static CompressorStageParams[] createThreeStageParams() {
        CompressorStageParams stage1 = new CompressorStageParams();
        stage1.critAlt = 2000;
        stage1.critPower = 1600;
        stage1.deckPower = 1550;
        stage1.deckAlt = 0;
        stage1.curvature = 1.0;
        stage1.wepCritAlt = 1800;
        stage1.wepPowerMult = 1.12;
        stage1.speedManifoldMult = 0.9;
        stage1.stageIndex = 0;
        stage1.oldAltitude = 2000;
        stage1.oldPower = 1600;
        stage1.oldPowerNewRpm = 1600;
        stage1.ceilingAlt = 6000;
        stage1.ceilingPower = 900;
        stage1.exactAltitudes = false;

        CompressorStageParams stage2 = new CompressorStageParams();
        stage2.critAlt = 5000;
        stage2.critPower = 1450;
        stage2.deckPower = 1200;
        stage2.deckAlt = 0;
        stage2.curvature = 1.0;
        stage2.wepCritAlt = 4500;
        stage2.wepPowerMult = 1.12;
        stage2.speedManifoldMult = 0.9;
        stage2.stageIndex = 1;
        stage2.oldAltitude = 5000;
        stage2.oldPower = 1450;
        stage2.oldPowerNewRpm = 1450;
        stage2.ceilingAlt = 10000;
        stage2.ceilingPower = 700;
        stage2.exactAltitudes = false;

        CompressorStageParams stage3 = new CompressorStageParams();
        stage3.critAlt = 8000;
        stage3.critPower = 1250;
        stage3.deckPower = 1000;
        stage3.deckAlt = 0;
        stage3.curvature = 1.0;
        stage3.wepCritAlt = 7500;
        stage3.wepPowerMult = 1.12;
        stage3.speedManifoldMult = 0.9;
        stage3.stageIndex = 2;
        stage3.oldAltitude = 8000;
        stage3.oldPower = 1250;
        stage3.oldPowerNewRpm = 1250;
        stage3.ceilingAlt = 14000;
        stage3.ceilingPower = 500;
        stage3.exactAltitudes = false;

        return new CompressorStageParams[] { stage1, stage2, stage3 };
    }
}
