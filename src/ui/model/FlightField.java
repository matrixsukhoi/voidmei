package ui.model;

/**
 * Represents a single flight data field displayed in the FlightInfo overlay.
 * Separates data definition from view logic.
 */
public class FlightField {
    /** Unique identifier for this field (e.g., "ias", "tas", "mach") */
    public final String key;

    /** Display label (e.g., "指示空速", "真空速") */
    public final String label;

    /** Unit string (e.g., "Km/h", "M/s", "Deg") */
    public final String unit;

    /**
     * Configuration key to check if this field is disabled (e.g.,
     * "disableFlightInfoIAS")
     */
    public final String configKey;

    /**
     * Whether this field should be hidden when its value is N/A (like WingSweep,
     * RadioAlt)
     */
    public final boolean hideWhenNA;

    /**
     * Whether this field is currently visible (respects config and runtime
     * conditions)
     */
    public boolean visible = true;

    /** Current formatted value to display */
    public String currentValue = "---";

    public FlightField(String key, String label, String unit, String configKey, boolean hideWhenNA) {
        this.key = key;
        this.label = label;
        this.unit = unit;
        this.configKey = configKey;
        this.hideWhenNA = hideWhenNA;
    }

    /**
     * Update the value with right-aligned formatting.
     */
    public void setValue(String value) {
        this.currentValue = String.format("%5s", value);
    }

    /**
     * Update the value and visibility based on hideWhenNA setting.
     */
    public void setValueWithVisibility(String value, String naString) {
        setValue(value);
        if (hideWhenNA) {
            this.visible = !value.equals(naString);
        }
    }
}
