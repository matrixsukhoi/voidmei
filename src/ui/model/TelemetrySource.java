package ui.model;

/**
 * Interface for providing raw telemetry data without object allocation.
 * This allows UI components to pull data directly as primitives.
 */
public interface TelemetrySource {
    // Flight Data
    double getIAS();

    double getTAS();

    double getMach();

    double getAoA();

    double getAoS();

    double getNy(); // G-Force

    double getVario(); // Climb Rate

    // Altitude & Position
    double getAltitude();

    double getRadioAltitude();

    boolean isRadioAltitudeValid();

    double getCompass();

    // Performance
    double getSEP();

    double getAcceleration();

    double getTurnRate();

    double getTurnRadius();

    double getRollRate(); // Wx

    double getEnergyJKg(); // Specific Energy

    // Aircraft State
    double getMassFuel();

    long getFuelTimeMili();

    double getThrottle();

    double getRPM();

    double getManifoldPressure();

    boolean isManifoldPressureValid();

    double getWaterTemp();

    double getOilTemp();

    double getPitch();

    double getEffHp();

    double getThrust();

    double getHorsePower();

    double getEngineResponse();

    double getPropEfficiency();

    double getWepKg();

    double getWepTime();

    double getHeatTolerance();

    double getPowerPercent();

    double getManifoldPressurePounds(); // Imperial

    double getManifoldPressureInchHg(); // Imperial

    /**
     * Get manifold pressure display value (Ata for metric, psi for imperial).
     */
    double getManifoldPressureDisplay();

    /**
     * Get manifold pressure display unit.
     * Returns "Ata" for metric, "P/XX.X''" (with live inHg) for imperial.
     */
    String getManifoldPressureDisplayUnit();

    /**
     * Get manifold pressure display precision.
     * Returns 2 for metric (Ata), 1 for imperial (psi).
     */
    int getManifoldPressureDisplayPrecision();

    // Engine Control
    double getUnknownMixture(); // For mixture state

    double getRadiator();

    double getCompressorStage();

    double getFuelPercent();

    double getRPMThrottle();

    // Component State (0.0 - 1.0 or percent)
    double getGear();

    double getFlaps();

    double getAirbrake();

    double getAileron();

    double getElevator();

    double getRudder();

    double getWingSweep();

    boolean isWingSweepValid();

    // Speed Indicator & Limits
    double getSpeedLimitRatio();

    double getAileronLockRatio();

    double getRudderLockRatio();

    double getUnitMachLimitRatio();

    double getStallSpeed();

    boolean isImperial();

    // Attitude Indicator Data
    /**
     * Get aviahorizon pitch (degrees).
     * Used by AttitudeOverlay for artificial horizon display.
     */
    double getAviahorizonPitch();

    /**
     * Get aviahorizon roll (degrees).
     * Used by AttitudeOverlay for artificial horizon rotation.
     */
    double getAviahorizonRoll();
}
