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

    // Aircraft State
    double getMassFuel();

    long getFuelTimeMili();

    double getThrottle();

    double getRPM();

    double getManifoldPressure();

    double getWaterTemp();

    double getOilTemp();

    double getPitch();

    double getThrust();

    double getHorsePower();

    double getEngineResponse();

    double getPropEfficiency();

    double getManifoldPressurePounds(); // Imperial

    double getManifoldPressureInchHg(); // Imperial

    // Engine Control
    double getUnknownMixture(); // For mixture state

    double getRadiator();

    double getCompressorStage();

    double getFuelPercent();

    double getRPMThrottle();

    double getThrustPercent();

    // Component State (0.0 - 1.0 or percent)
    double getGear();

    double getFlaps();

    double getAirbrake();

    double getAileron();

    double getElevator();

    double getRudder();

    double getWingSweep();
}
