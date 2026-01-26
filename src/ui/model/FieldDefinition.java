package ui.model;

/**
 * Definition of a flight info field.
 * Used to externalize field configuration from the FlightInfo class.
 */
public class FieldDefinition {
    public final String key;
    public final String label;
    public final String unit;
    public final String configKey;
    public final boolean hideWhenNA;
    public final boolean hideWhenZero;
    public final String previewValue;

    public FieldDefinition(String key, String label, String unit, String configKey, boolean hideWhenNA,
            boolean hideWhenZero, String previewValue) {
        this.key = key;
        this.label = label;
        this.unit = unit;
        this.configKey = configKey;
        this.hideWhenNA = hideWhenNA;
        this.hideWhenZero = hideWhenZero;
        this.previewValue = previewValue;
    }

    public FieldDefinition(String key, String label, String unit, String configKey, boolean hideWhenNA,
            String previewValue) {
        this(key, label, unit, configKey, hideWhenNA, false, previewValue);
    }

    public FieldDefinition(String key, String label, String unit, String configKey, boolean hideWhenNA) {
        this(key, label, unit, configKey, hideWhenNA, "-");
    }
}
