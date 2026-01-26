package ui.model;

import java.util.ArrayList;
import java.util.List;

import prog.config.ConfigProvider;

/**
 * Configuration for FlightInfo overlay.
 * Externalizes all configuration from the FlightInfo class.
 */
public class FlightInfoConfig {

    // Field definitions
    private List<FieldDefinition> fieldDefinitions = new ArrayList<>();

    // Overlay title
    public String title = "FlightInfo";

    // Style configuration
    public boolean showEdge = false;
    public int columnNum = 3;

    // Font configuration keys
    public String numFontKey = "GlobalNumFont";
    public String labelFontKey = "flightInfoFontC";
    // public String fontAddKey = "flightInfoFontaddC"; // Legacy, removed
    public String columnKey = "flightInfoColumn";

    // Position keys (inherited by DraggableOverlay)
    public String posXKey = "flightInfoX";
    public String posYKey = "flightInfoY";

    // Edge style key
    public String edgeKey = "flightInfoEdge";

    // Layout Config
    public prog.config.ConfigLoader.GroupConfig groupConfig;

    public List<FieldDefinition> getFieldDefinitions() {
        return fieldDefinitions;
    }

    public void addFieldDefinition(String key, String label, String unit, String configKey, boolean hideWhenNA,
            String exampleValue) {
        fieldDefinitions.add(new FieldDefinition(key, label, unit, configKey, hideWhenNA, exampleValue));
    }

    /**
     * Create default configuration with standard flight info fields.
     * This factory method contains the field definitions that were previously
     * hardcoded in FlightInfo.
     */
    public static FlightInfoConfig createDefault(ConfigProvider configProvider,
            prog.config.ConfigLoader.GroupConfig groupConfig) {
        FlightInfoConfig cfg = new FlightInfoConfig();
        cfg.groupConfig = groupConfig;

        if (configProvider != null) {
            String edgeVal = configProvider.getConfig("flightInfoEdge");
            if (edgeVal != null) {
                cfg.showEdge = "true".equals(edgeVal);
            }

            String colStr = configProvider.getConfig("flightInfoColumn");
            if (colStr != null && !colStr.isEmpty()) {
                try {
                    cfg.columnNum = Integer.parseInt(colStr);
                } catch (NumberFormatException e) {
                    cfg.columnNum = 3;
                }
            }
        }

        // Dynamically populate fields from the loaded configuration groups
        if (groupConfig != null) {
            cfg.populateFromGroup(groupConfig.rows);
        }

        return cfg;
    }

    private void populateFromGroup(List<prog.config.ConfigLoader.RowConfig> rows) {
        if (rows == null)
            return;
        for (prog.config.ConfigLoader.RowConfig row : rows) {
            if ("DATA".equals(row.type) && row.property != null && !row.property.isEmpty()) {
                // Use the property as key, label from config
                String defVal = row.previewValue != null ? row.previewValue : "-";
                addFieldDefinition(row.property, row.label, row.unit, row.property, true, row.hideWhenZero, defVal);
            }
            if (row.children != null) {
                populateFromGroup(row.children);
            }
        }
    }

    public void addFieldDefinition(String key, String label, String unit, String configKey, boolean hideWhenNA,
            boolean hideWhenZero, String previewValue) {
        fieldDefinitions.add(new FieldDefinition(key, label, unit, configKey, hideWhenNA, hideWhenZero, previewValue));
    }
}
