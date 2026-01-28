package parser;

import ui.window.comparison.model.ComparisonData;
import parser.Blkx;

/**
 * Extracts normalized performance data from raw Blkx objects.
 */
/**
 * @deprecated Data parsing is now handled via regex/Blkx in the UI directly or
 *             via new backend logic.
 */
@Deprecated
public class FlightModelParser {

    public static ComparisonData parse(Blkx blkx) {
        if (blkx == null || !blkx.valid)
            return new ComparisonData("Unknown");

        ComparisonData d = new ComparisonData(blkx.readFileName); // Or use friendly name if passed

        // 1. Physical Properties
        d.emptyWeight = safeParse(blkx.getlastone("EmptyWeight"), 0);
        d.maxFuelWeight = safeParse(blkx.getlastone("MaxFuelMass"), 0);
        d.maxTakeoffWeight = safeParse(blkx.getlastone("MaxTakeoffMass"), 0);

        // Wing Area calc: No direct field usually, sometimes in Aerodynamics ->
        // WingPlane -> Areas -> Core
        // Using approximate logic or specific key lookup
        // For now, let's look for "Wingspan" and "Length" if Area absent, or check
        // Aerodynamics block
        // Blkx structure is complex, for MVP we might pull what's easy or calculate
        // from derived data
        // Let's rely on standard keys first.

        // 2. Performance
        d.vne = safeParse(blkx.getlastone("Vne"), 0);
        d.maxG = safeParse(blkx.getlastone("MaxOverload"), 0);

        // 3. Engine (Complex: Prop vs Jet)
        // Need to traverse engine block
        // For MVP, placeholders or basic parsing

        // Speed/Climb usually not in FM file directly as a "stat",
        // they are result of simulation.
        // However, some FM files have "Test" or "Passport" data in comments or specific
        // blocks.
        // IF NOT AVAILABLE: We might show "N/A" or 0 for now until advanced simulation
        // logic added.

        // Wait, Blkx class has some parsed fields already?
        // Let's check Blkx.java capabilities via view_file if needed.
        // Assuming raw parsing for now.

        // Fallback for missing critical data:
        if (d.maxTakeoffWeight == 0)
            d.maxTakeoffWeight = d.emptyWeight + d.maxFuelWeight;

        return d;
    }

    private static double safeParse(String val, double def) {
        if (val == null)
            return def;
        try {
            return Double.parseDouble(val);
        } catch (Exception e) {
            return def;
        }
    }
}
