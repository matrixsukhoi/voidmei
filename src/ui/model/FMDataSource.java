package ui.model;

/**
 * Interface for providing FM (Flight Model) data for display in overlays.
 * Designed for use with ReflectBinder for zero-GC data access.
 *
 * <p>This interface abstracts Blkx data access, allowing overlays to bind
 * to getter methods via reflection without directly depending on Blkx.
 */
public interface FMDataSource {

    // ==================== Basic Info ====================

    /** Get FM file name and version string */
    String getFmVersion();

    /** Get empty aircraft weight (kg) */
    double getEmptyWeight();

    /** Get maximum fuel weight (kg) */
    double getMaxFuelWeight();

    // ==================== Speed Limits ====================

    /** Get critical speed (km/h) - compressibility limit */
    double getCriticalSpeed();

    /** Get VNE - never exceed speed (km/h) */
    double getVne();

    /** Get MNE - never exceed Mach number */
    double getVneMach();

    // ==================== G-Load Limits ====================

    /** Get positive G limit at full fuel */
    double getFullFuelPosG();

    /** Get negative G limit at full fuel */
    double getFullFuelNegG();

    /** Get positive G limit at half fuel */
    double getHalfFuelPosG();

    /** Get negative G limit at half fuel */
    double getHalfFuelNegG();

    // ==================== Control Surface Effectiveness ====================

    /** Get elevator effective speed (km/h) */
    double getElevatorEffSpeed();

    /** Get aileron effective speed (km/h) */
    double getAileronEffSpeed();

    /** Get rudder effective speed (km/h) */
    double getRudderEffSpeed();

    /** Get elevator power loss factor */
    double getElevatorPowerLoss();

    /** Get aileron power loss factor */
    double getAileronPowerLoss();

    /** Get rudder power loss factor */
    double getRudderPowerLoss();

    // ==================== WEP/Nitro System ====================

    /** Get nitro amount (kg) */
    double getNitroAmount();

    /** Get nitro duration (seconds) */
    double getNitroTime();

    /** Check if nitro is available (for hide-when-zero) */
    boolean isNitroAmountValid();

    // ==================== Heat Management ====================

    /** Get average engine heat recovery rate */
    double getAvgEngRecoveryRate();

    // ==================== Lift Performance ====================

    /** Get wing loading limit without flaps */
    double getNoFlapWingLoad();

    /** Get wing loading limit with full flaps */
    double getFullFlapWingLoad();

    // ==================== Inertia ====================

    /** Get pitch moment of inertia (kg*m^2) */
    double getMoiPitch();

    /** Get roll moment of inertia (kg*m^2) */
    double getMoiRoll();

    /** Get yaw moment of inertia (kg*m^2) */
    double getMoiYaw();

    // ==================== Wing Geometry ====================

    /** Get wing area (m^2) */
    double getWingArea();

    /** Get fuselage area (m^2) */
    double getFuselageArea();

    /** Get Oswald's efficiency number */
    double getOswaldsEfficiency();

    /** Get aspect ratio */
    double getAspectRatio();

    /** Get swept wing angle (degrees) */
    double getSweptWingAngle();

    // ==================== Drag Parameters ====================

    /** Get drag area coefficient (CdS) */
    double getCdS();

    /** Get induced drag factor */
    double getIndCdF();

    /** Get radiator drag coefficient */
    double getRadiatorCd();

    /** Get oil radiator drag coefficient */
    double getOilRadiatorCd();

    // ==================== No-Flaps Wing (fm_parts) ====================

    /** Get NoFlapsWing CdMin */
    double getNoFlapsWing_CdMin();

    /** Get NoFlapsWing Cl0 (zero-angle lift coefficient) */
    double getNoFlapsWing_Cl0();

    /** Get NoFlapsWing critical AoA high (degrees) */
    double getNoFlapsWing_AoACritHigh();

    /** Get NoFlapsWing critical AoA low (degrees) */
    double getNoFlapsWing_AoACritLow();

    /** Get NoFlapsWing ClCritHigh */
    double getNoFlapsWing_ClCritHigh();

    /** Get NoFlapsWing ClCritLow */
    double getNoFlapsWing_ClCritLow();

    // ==================== Full-Flaps Wing (fm_parts) ====================

    /** Get FullFlapsWing CdMin */
    double getFullFlapsWing_CdMin();

    /** Get FullFlapsWing Cl0 */
    double getFullFlapsWing_Cl0();

    /** Get FullFlapsWing critical AoA high (degrees) */
    double getFullFlapsWing_AoACritHigh();

    /** Get FullFlapsWing critical AoA low (degrees) */
    double getFullFlapsWing_AoACritLow();

    // ==================== Other fm_parts ====================

    /** Get Fuselage CdMin */
    double getFuselage_CdMin();

    /** Get Fin CdMin */
    double getFin_CdMin();

    /** Get Stab CdMin */
    double getStab_CdMin();

    // ==================== Flap Speed Limits ====================

    /** Get flap position 0 speed limit (km/h) */
    double getFlap0Speed();

    /** Get flap position 1 speed limit (km/h) */
    double getFlap1Speed();

    /** Get flap position 2 speed limit (km/h) */
    double getFlap2Speed();

    /** Get flap position 3 speed limit (km/h) */
    double getFlap3Speed();

    /** Check if flap 0 speed is valid (for hide-when-zero) */
    boolean isFlap0SpeedValid();

    /** Check if flap 1 speed is valid (for hide-when-zero) */
    boolean isFlap1SpeedValid();

    /** Check if flap 2 speed is valid (for hide-when-zero) */
    boolean isFlap2SpeedValid();

    /** Check if flap 3 speed is valid (for hide-when-zero) */
    boolean isFlap3SpeedValid();

    // ==================== Gear ====================

    /** Get gear destruction speed (km/h) */
    double getGearDestructionSpeed();

    // ==================== Engine Info ====================

    /** Check if aircraft is jet-powered */
    boolean isJet();

    /** Get number of engines */
    int getEngineNum();
}
