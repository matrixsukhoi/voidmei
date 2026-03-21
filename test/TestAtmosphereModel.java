import prog.util.AtmosphereModel;
import static prog.util.AtmosphereModel.*;

/**
 * Tests for AtmosphereModel.
 *
 * Validates the ISA standard atmosphere calculations against known values.
 *
 * Run with: ./script/test.sh
 */
public class TestAtmosphereModel {

    private static int passed = 0;
    private static int failed = 0;

    public static void main(String[] args) {
        System.out.println("=== AtmosphereModel Tests ===\n");

        testPressure();
        testAltitudeAtPressure();
        testDensity();
        testIasTasConversion();
        testRamEffect();
        testTemperature();

        System.out.println("\n=== Results ===");
        System.out.printf("Passed: %d, Failed: %d%n", passed, failed);

        if (failed > 0) {
            System.exit(1);
        }
    }

    private static void testPressure() {
        System.out.println("Testing pressure()...");

        // Sea level
        assertClose("pressure(0m)", pressure(0), 1.0, 0.001);

        // Standard ISA values
        assertClose("pressure(5000m)", pressure(5000), 0.5334, 0.01);
        assertClose("pressure(10000m)", pressure(10000), 0.2615, 0.01);
        assertClose("pressure(11000m)", pressure(11000), 0.2240, 0.01);

        // Below sea level (higher pressure)
        assertTrue("pressure(-1000m) > 1.0", pressure(-1000) > 1.0);
        assertClose("pressure(-1000m)", pressure(-1000), 1.127, 0.01);
    }

    private static void testAltitudeAtPressure() {
        System.out.println("Testing altitudeAtPressure()...");

        // Round-trip tests (using actual ISA formula values)
        assertClose("altitudeAtPressure(1.0)", altitudeAtPressure(1.0), 0, 1);
        assertClose("altitudeAtPressure(0.5)", altitudeAtPressure(0.5), 5477, 50);  // ISA formula result
        assertClose("altitudeAtPressure(0.25)", altitudeAtPressure(0.25), 10278, 50);  // ISA formula result

        // Inverse function property
        for (int alt = 0; alt <= 15000; alt += 1000) {
            double p = pressure(alt);
            double recovered = altitudeAtPressure(p);
            assertClose("round-trip at " + alt + "m", recovered, alt, 1);
        }
    }

    private static void testDensity() {
        System.out.println("Testing density()...");

        // Sea level ISA: 1.225 kg/m³
        double rho0 = density(1.0, 15.0, 0);
        assertClose("density at sea level", rho0, 1.225, 0.001);

        // At 5000m
        double p5000 = pressure(5000);
        double rho5000 = density(p5000, 15.0, 5000);
        assertClose("density at 5000m", rho5000, 0.736, 0.01);

        // Density decreases with altitude
        assertTrue("density decreases with altitude", rho5000 < rho0);

        // Temperature effect: higher temp = lower density
        double rhoHot = density(1.0, 30.0, 0);
        double rhoCold = density(1.0, 0.0, 0);
        assertTrue("hot air less dense", rhoHot < rho0);
        assertTrue("cold air more dense", rhoCold > rho0);
    }

    private static void testIasTasConversion() {
        System.out.println("Testing IAS/TAS conversion...");

        // At sea level, IAS = TAS
        double rho0 = 1.225;
        assertClose("TAS at sea level", iasToTas(400, rho0), 400, 0.1);
        assertClose("IAS at sea level", tasToIas(400, rho0), 400, 0.1);

        // At altitude, TAS > IAS
        double rho5000 = density(pressure(5000), 15.0, 5000);
        double tas = iasToTas(400, rho5000);
        assertTrue("TAS > IAS at altitude", tas > 400);
        assertClose("TAS at 5000m", tas, 516, 5);

        // Round-trip
        double iasBack = tasToIas(tas, rho5000);
        assertClose("IAS round-trip", iasBack, 400, 0.1);
    }

    private static void testRamEffect() {
        System.out.println("Testing ramEffectAltitude()...");

        // Zero speed: no effect
        double noRam = ramEffectAltitude(5000, 15.0, 0, true, 1.0);
        assertClose("no RAM at zero speed", noRam, 5000, 1);

        // With speed: effective altitude is lower
        double withRam = ramEffectAltitude(5000, 15.0, 500, true, 1.0);
        assertTrue("RAM lowers effective altitude", withRam < 5000);

        // Higher speed = more RAM effect
        double moreRam = ramEffectAltitude(5000, 15.0, 600, true, 1.0);
        assertTrue("more speed = more RAM", moreRam < withRam);

        // SpeedManifoldMult affects magnitude
        double lessEfficient = ramEffectAltitude(5000, 15.0, 500, true, 0.5);
        assertTrue("lower mult = less RAM", lessEfficient > withRam);

        // Typical values check
        double typical = ramEffectAltitude(5000, 15.0, 500, true, 0.9);
        System.out.printf("  RAM effect: 5000m @ 500km/h IAS -> %.0fm effective%n", typical);
        assertTrue("typical RAM ~1000-1500m reduction", typical > 3500 && typical < 4500);
    }

    private static void testTemperature() {
        System.out.println("Testing temperatureAtAltitude()...");

        // ISA sea level = 15°C
        assertClose("temp at sea level", temperatureAtAltitude(15.0, 0), 15.0, 0.01);

        // Lapse rate: -6.5°C per 1000m
        assertClose("temp at 1000m", temperatureAtAltitude(15.0, 1000), 8.5, 0.1);
        assertClose("temp at 5000m", temperatureAtAltitude(15.0, 5000), -17.5, 0.1);
        assertClose("temp at 10000m", temperatureAtAltitude(15.0, 10000), -50.0, 0.1);
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
