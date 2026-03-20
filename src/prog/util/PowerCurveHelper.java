package prog.util;

import prog.util.PistonPowerModel.CompressorStageParams;

/**
 * Helper functions for determining power curve shape based on FM parameters.
 *
 * <p>Ported from WAPC plane_power_calculator.py helper functions:
 * ConstRPM_is, ConstRPM_bends_below_critalt, Ceiling_is_useful, etc.
 *
 * <p>These functions determine which branch of the power curve calculation
 * to use based on the relationship between various FM parameters.
 */
public final class PowerCurveHelper {

    private PowerCurveHelper() {}

    /**
     * Checks if the stage has ConstRPM parameters defined.
     *
     * <p>Note: constRpmAlt=0 is a valid value (ConstRPM at sea level), so we only
     * check constRpmPower. WAPC's ConstRPM_is() checks key existence, not altitude value.
     * When FM doesn't define ConstRPM, both constRpmAlt and constRpmPower default to 0.
     */
    public static boolean hasConstRpm(CompressorStageParams p) {
        return p.constRpmPower > 0;
    }

    /**
     * ConstRPM bend point is below the critical altitude.
     * This creates a two-segment curve below crit alt: deck→constRPM then constRPM→crit.
     */
    public static boolean constRpmBelowCritAlt(CompressorStageParams p) {
        return hasConstRpm(p) && (p.constRpmAlt - p.critAlt) < -1;
    }

    /**
     * ConstRPM bend point is below the original (pre-adjustment) critical altitude.
     */
    public static boolean constRpmBelowOldCritAlt(CompressorStageParams p) {
        return hasConstRpm(p) && (p.constRpmAlt - p.oldAltitude) < -1;
    }

    /**
     * ConstRPM bend point is below the WEP critical altitude.
     */
    public static boolean constRpmBelowWepCritAlt(CompressorStageParams p) {
        return hasConstRpm(p) && (p.constRpmAlt - p.wepCritAlt) < -1;
    }

    /**
     * ConstRPM bends above critical altitude — used with ceiling parameters
     * to create a curved decay above crit alt (e.g., P-63).
     */
    public static boolean constRpmAboveCritAlt(CompressorStageParams p) {
        return hasConstRpm(p)
            && p.constRpmAlt == p.critAlt
            && p.critPower - p.ceilingPower > 1
            && p.curvature > 1;
    }

    /** ConstRPM altitude is at or below sea level. */
    public static boolean constRpmBelowDeck(CompressorStageParams p) {
        return hasConstRpm(p) && p.constRpmAlt <= 0;
    }

    /** Checks if ceiling parameters exist. */
    public static boolean hasCeiling(CompressorStageParams p) {
        return p.ceilingAlt > 0 && p.ceilingPower > 0;
    }

    /**
     * Ceiling parameters are meaningful — altitude gap and power gap are both
     * significant enough to affect the curve shape.
     *
     * <p>Uses the original FM altitude/power (before definition_alt_power_adjuster)
     * to match WAPC's Ceiling_is_useful() which compares against Altitude[i] / Power[i],
     * not the adjusted critAlt / critPower.
     */
    public static boolean ceilingIsUseful(CompressorStageParams p) {
        double referenceAlt = p.oldAltitude > 0 ? p.oldAltitude : p.critAlt;
        double referencePower = p.oldPower > 0 ? p.oldPower : p.critPower;
        return hasCeiling(p)
            && (p.ceilingAlt - referenceAlt) >= 2
            && (referencePower - p.ceilingPower) >= 2;
    }

    /**
     * Critical altitude equals deck altitude — the power curve is flat from sea level.
     * Deck→crit interpolation is skipped; curve goes directly to ceiling.
     */
    public static boolean powerIsDeckPower(CompressorStageParams p) {
        return Math.abs(p.critAlt - p.deckAlt) < 1;
    }
}
