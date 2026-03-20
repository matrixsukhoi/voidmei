package ui.model;

import parser.Blkx;
import static prog.util.PhysicsConstants.g;

/**
 * Adapter that wraps a Blkx instance and implements FMDataSource.
 * Provides zero-allocation access to FM data for overlay display.
 *
 * <p>This adapter allows FMUnpackedDataOverlay to use ReflectBinder
 * for dynamic field binding without directly depending on Blkx structure.
 */
public class FMDataAdapter implements FMDataSource {

    private Blkx blkx;

    /**
     * Set the Blkx instance to read data from.
     * @param blkx The flight model data, or null if not loaded
     */
    public void setBlkx(Blkx blkx) {
        this.blkx = blkx;
    }

    /**
     * Get the current Blkx instance.
     */
    public Blkx getBlkx() {
        return blkx;
    }

    // ==================== Basic Info ====================

    @Override
    public String getFmVersion() {
        if (blkx == null) return "";
        String name = blkx.readFileName != null ? blkx.readFileName : "N/A";
        String ver = blkx.version != null ? blkx.version : "N/A";
        return name;
    }

    @Override
    public double getEmptyWeight() {
        return blkx != null ? blkx.emptyweight : 0;
    }

    @Override
    public double getMaxFuelWeight() {
        return blkx != null ? blkx.maxfuelweight : 0;
    }

    // ==================== Speed Limits ====================

    @Override
    public double getCriticalSpeed() {
        return blkx != null ? blkx.CriticalSpeed * 3.6 : 0;
    }

    @Override
    public double getVne() {
        return blkx != null ? blkx.vne : 0;
    }

    @Override
    public double getVneMach() {
        return blkx != null ? blkx.vneMach : 0;
    }

    // ==================== G-Load Limits ====================

    @Override
    public double getFullFuelPosG() {
        if (blkx == null || blkx.rawWingCritOverload == null) return 0;
        return 1.2 * (2 * blkx.rawWingCritOverload[1] / (g * blkx.grossweight) - 1);
    }

    @Override
    public double getFullFuelNegG() {
        if (blkx == null || blkx.rawWingCritOverload == null) return 0;
        return 1.2 * (2 * blkx.rawWingCritOverload[0] / (g * blkx.grossweight) + 1);
    }

    @Override
    public double getHalfFuelPosG() {
        if (blkx == null || blkx.rawWingCritOverload == null) return 0;
        return 1.2 * (2 * blkx.rawWingCritOverload[1] / (g * blkx.halfweight) - 1);
    }

    @Override
    public double getHalfFuelNegG() {
        if (blkx == null || blkx.rawWingCritOverload == null) return 0;
        return 1.2 * (2 * blkx.rawWingCritOverload[0] / (g * blkx.halfweight) + 1);
    }

    // ==================== Control Surface Effectiveness ====================

    @Override
    public double getElevatorEffSpeed() {
        return blkx != null ? blkx.elavEff : 0;
    }

    @Override
    public double getAileronEffSpeed() {
        return blkx != null ? blkx.aileronEff : 0;
    }

    @Override
    public double getRudderEffSpeed() {
        return blkx != null ? blkx.rudderEff : 0;
    }

    @Override
    public double getElevatorPowerLoss() {
        return blkx != null ? blkx.elavPowerLoss : 0;
    }

    @Override
    public double getAileronPowerLoss() {
        return blkx != null ? blkx.aileronPowerLoss : 0;
    }

    @Override
    public double getRudderPowerLoss() {
        return blkx != null ? blkx.rudderPowerLoss : 0;
    }

    // ==================== WEP/Nitro System ====================

    @Override
    public double getNitroAmount() {
        return blkx != null ? blkx.nitro : 0;
    }

    @Override
    public double getNitroTime() {
        if (blkx == null || blkx.nitroDecr <= 0) return 0;
        return blkx.nitro / (blkx.nitroDecr * 60);
    }

    @Override
    public boolean isNitroAmountValid() {
        return blkx != null && blkx.nitro > 0;
    }

    // ==================== Heat Management ====================

    @Override
    public double getAvgEngRecoveryRate() {
        return blkx != null ? blkx.avgEngRecoveryRate : 0;
    }

    // ==================== Lift Performance ====================

    @Override
    public double getNoFlapWingLoad() {
        return blkx != null ? blkx.NoFlapWLL : 0;
    }

    @Override
    public double getFullFlapWingLoad() {
        return blkx != null ? blkx.FullFlapWLL : 0;
    }

    // ==================== Inertia ====================

    @Override
    public double getMoiPitch() {
        if (blkx == null || blkx.MomentOfInertia == null || blkx.MomentOfInertia.length < 3) return 0;
        return blkx.MomentOfInertia[2];
    }

    @Override
    public double getMoiRoll() {
        if (blkx == null || blkx.MomentOfInertia == null || blkx.MomentOfInertia.length < 1) return 0;
        return blkx.MomentOfInertia[0];
    }

    @Override
    public double getMoiYaw() {
        if (blkx == null || blkx.MomentOfInertia == null || blkx.MomentOfInertia.length < 2) return 0;
        return blkx.MomentOfInertia[1];
    }

    // ==================== Wing Geometry ====================

    @Override
    public double getWingArea() {
        return blkx != null ? blkx.AWing : 0;
    }

    @Override
    public double getFuselageArea() {
        return blkx != null ? blkx.AFuselage : 0;
    }

    @Override
    public double getOswaldsEfficiency() {
        return blkx != null ? blkx.OswaldsEfficiencyNumber : 0;
    }

    @Override
    public double getAspectRatio() {
        return blkx != null ? blkx.AspectRatio : 0;
    }

    @Override
    public double getSweptWingAngle() {
        return blkx != null ? blkx.SweptWingAngle : 0;
    }

    // ==================== Drag Parameters ====================

    @Override
    public double getCdS() {
        return blkx != null ? blkx.CdS : 0;
    }

    @Override
    public double getIndCdF() {
        return blkx != null ? blkx.indCdF : 0;
    }

    @Override
    public double getRadiatorCd() {
        return blkx != null ? blkx.RadiatorCd : 0;
    }

    @Override
    public double getOilRadiatorCd() {
        return blkx != null ? blkx.OilRadiatorCd : 0;
    }

    // ==================== No-Flaps Wing (fm_parts) ====================

    @Override
    public double getNoFlapsWing_CdMin() {
        return blkx != null && blkx.NoFlapsWing != null ? blkx.NoFlapsWing.CdMin : 0;
    }

    @Override
    public double getNoFlapsWing_Cl0() {
        return blkx != null && blkx.NoFlapsWing != null ? blkx.NoFlapsWing.Cl0 : 0;
    }

    @Override
    public double getNoFlapsWing_AoACritHigh() {
        return blkx != null && blkx.NoFlapsWing != null ? blkx.NoFlapsWing.AoACritHigh : 0;
    }

    @Override
    public double getNoFlapsWing_AoACritLow() {
        return blkx != null && blkx.NoFlapsWing != null ? blkx.NoFlapsWing.AoACritLow : 0;
    }

    @Override
    public double getNoFlapsWing_ClCritHigh() {
        return blkx != null && blkx.NoFlapsWing != null ? blkx.NoFlapsWing.ClCritHigh : 0;
    }

    @Override
    public double getNoFlapsWing_ClCritLow() {
        return blkx != null && blkx.NoFlapsWing != null ? blkx.NoFlapsWing.ClCritLow : 0;
    }

    // ==================== Full-Flaps Wing (fm_parts) ====================

    @Override
    public double getFullFlapsWing_CdMin() {
        return blkx != null && blkx.FullFlapsWing != null ? blkx.FullFlapsWing.CdMin : 0;
    }

    @Override
    public double getFullFlapsWing_Cl0() {
        return blkx != null && blkx.FullFlapsWing != null ? blkx.FullFlapsWing.Cl0 : 0;
    }

    @Override
    public double getFullFlapsWing_AoACritHigh() {
        return blkx != null && blkx.FullFlapsWing != null ? blkx.FullFlapsWing.AoACritHigh : 0;
    }

    @Override
    public double getFullFlapsWing_AoACritLow() {
        return blkx != null && blkx.FullFlapsWing != null ? blkx.FullFlapsWing.AoACritLow : 0;
    }

    // ==================== Other fm_parts ====================

    @Override
    public double getFuselage_CdMin() {
        return blkx != null && blkx.Fuselage != null ? blkx.Fuselage.CdMin : 0;
    }

    @Override
    public double getFin_CdMin() {
        return blkx != null && blkx.Fin != null ? blkx.Fin.CdMin : 0;
    }

    @Override
    public double getStab_CdMin() {
        return blkx != null && blkx.Stab != null ? blkx.Stab.CdMin : 0;
    }

    // ==================== Flap Speed Limits ====================

    @Override
    public double getFlap0Speed() {
        if (blkx == null || blkx.FlapsDestructionIndSpeed == null) return 0;
        if (blkx.FlapsDestructionNum > 0) {
            return blkx.FlapsDestructionIndSpeed[0][1];
        }
        return 0;
    }

    @Override
    public double getFlap1Speed() {
        if (blkx == null || blkx.FlapsDestructionIndSpeed == null) return 0;
        if (blkx.FlapsDestructionNum > 1) {
            return blkx.FlapsDestructionIndSpeed[1][1];
        }
        return 0;
    }

    @Override
    public double getFlap2Speed() {
        if (blkx == null || blkx.FlapsDestructionIndSpeed == null) return 0;
        if (blkx.FlapsDestructionNum > 2) {
            return blkx.FlapsDestructionIndSpeed[2][1];
        }
        return 0;
    }

    @Override
    public double getFlap3Speed() {
        if (blkx == null || blkx.FlapsDestructionIndSpeed == null) return 0;
        if (blkx.FlapsDestructionNum > 3) {
            return blkx.FlapsDestructionIndSpeed[3][1];
        }
        return 0;
    }

    @Override
    public boolean isFlap0SpeedValid() {
        return getFlap0Speed() > 0;
    }

    @Override
    public boolean isFlap1SpeedValid() {
        return getFlap1Speed() > 0;
    }

    @Override
    public boolean isFlap2SpeedValid() {
        return getFlap2Speed() > 0;
    }

    @Override
    public boolean isFlap3SpeedValid() {
        return getFlap3Speed() > 0;
    }

    // ==================== Gear ====================

    @Override
    public double getGearDestructionSpeed() {
        return blkx != null ? blkx.GearDestructionIndSpeed : 0;
    }

    // ==================== Engine Info ====================

    @Override
    public boolean isJet() {
        return blkx != null && blkx.isJet;
    }

    @Override
    public int getEngineNum() {
        return blkx != null ? blkx.engineNum : 0;
    }
}
