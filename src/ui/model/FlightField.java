package ui.model;

/**
 * Represents a single flight data field displayed in the FlightInfo overlay.
 * Separates data definition from view logic.
 */
public class FlightField {
    /** Unique identifier for this field (e.g., "ias", "tas", "mach") */
    public String key;

    /** Display label (e.g., "指示空速", "真空速") */
    public String label;

    /** Unit string (e.g., "Km/h", "M/s", "Deg") */
    public String unit;

    /**
     * Configuration key to check if this field is disabled (e.g.,
     * "disableFlightInfoIAS")
     */
    public String configKey;

    /**
     * Whether this field is currently visible (respects config and runtime
     * conditions)
     */
    public boolean visible = true;

    /** Current formatted value to display */
    public String currentValue = "";

    /**
     * Whether this field should be hidden when its value is N/A (like WingSweep,
     * RadioAlt)
     */
    public boolean hideWhenNA = false;

    public FlightField(String key, String label, String unit, String configKey) {
        this.key = key;
        this.label = label;
        this.unit = unit;
        this.configKey = configKey;
    }

    public FlightField(String key, String label, String unit, String configKey, boolean hideWhenNA) {
        this(key, label, unit, configKey);
        this.hideWhenNA = hideWhenNA;
    }

    /**
     * Update the value with right-aligned formatting.
     */
    public void setValue(String value) {
        this.currentValue = String.format("%5s", value);
    }
}
