package ui.window.comparison.model;

/**
 * Standardized data model for aircraft comparison.
 * Holds performance metrics normalized from Flight Model data.
 */
public class ComparisonData {
    public String name;

    // Performance
    public double maxSpeedSeaLevel; // km/h
    public double maxSpeedBest; // km/h
    public double maxClimbSeaLevel; // m/s
    public double turnTime; // s (Virage time)
    public double maxRollRate; // deg/s
    public double vne; // km/h (Max Dive Speed)
    public double maxG; // G Limit

    // Physical Properties
    public double emptyWeight; // kg
    public double maxFuelWeight; // kg
    public double maxTakeoffWeight; // kg
    public double wingArea; // m^2

    // Engine
    public double takeoffThrust; // kgf (Static)
    public double takeoffPower; // hp (Static)

    // Derived Metrics
    public double wingLoading; // kg/m^2 (at full fuel)
    public double thrustToWeight; // ratio (at full fuel)

    public ComparisonData(String name) {
        this.name = name;
    }
}
