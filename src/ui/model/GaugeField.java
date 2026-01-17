package ui.model;

/**
 * Data model for a linear gauge field.
 * Extends DataField to add gauge-specific properties like max value and
 * orientation.
 */
public class GaugeField extends DataField {

    public int gaugeType;
    public int maxValue;
    public int currentIntValue;
    public boolean isHorizontal;

    // Reference to the visual component for direct updates
    public ui.component.LinearGauge gauge;

    public GaugeField(String key, String label, String unit, int gaugeType, int maxValue, boolean isHorizontal) {
        super(key, label, unit, "disable" + key, false); // Use key-based configKey
        this.gaugeType = gaugeType;
        this.maxValue = maxValue;
        this.currentIntValue = 0;
        this.isHorizontal = isHorizontal;
        this.gauge = new ui.component.LinearGauge(label, maxValue, !isHorizontal);
    }

    public void updateGauge(int value, String displayText) {
        this.currentIntValue = value;
        this.currentValue = displayText;
        if (gauge != null) {
            gauge.update(value, displayText);
        }
    }
}
