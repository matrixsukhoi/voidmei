package ui.model;

/**
 * Represents a single data field displayed in an overlay.
 * Generic version - can be used for any type of data display.
 */
public class DataField {
    /** Unique identifier for this field (e.g., "ias", "tas", "mach") */
    public final String key;

    /** Display label (e.g., "指示空速", "真空速") */
    public final String label;

    /** Unit string (e.g., "Km/h", "M/s", "Deg") */
    public String unit;

    /** Configuration key to check if this field is disabled */
    public final String configKey;

    /** Whether this field should be hidden when its value is N/A */
    public final boolean hideWhenNA;

    /** Whether this field is currently visible */
    public boolean visible = true;

    /** Current formatted value to display */
    public String currentValue = "---";

    public DataField(String key, String label, String unit, String configKey, boolean hideWhenNA) {
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

    public void setUnit(String unit) {
        this.unit = unit;
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

    // --- Zero-GC Pipeline Support ---
    public final char[] buffer = new char[32];
    public int length = 0;
    public java.util.function.DoubleSupplier valueSupplier;
    public java.util.function.BooleanSupplier visibilitySupplier;
    public int precision = 0; // Default to integer
}
