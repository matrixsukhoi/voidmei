package ui.model;

import java.util.ArrayList;
import java.util.List;

import prog.i18n.Lang;
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
    public String fontAddKey = "flightInfoFontaddC";
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

        // Load style settings from config (Legacy properties or GroupConfig
        // extensions?)
        // For now, keep style in properties as requested, only X/Y moved to
        // ui_layout.cfg
        if (configProvider != null) {
            cfg.showEdge = "true".equals(configProvider.getConfig("flightInfoEdge"));
            String colStr = configProvider.getConfig("flightInfoColumn");
            if (colStr != null && !colStr.isEmpty()) {
                try {
                    cfg.columnNum = Integer.parseInt(colStr);
                } catch (NumberFormatException e) {
                    cfg.columnNum = 3;
                }
            }
        }

        // Ensure GroupConfig has default values if empty
        if (cfg.groupConfig != null) {
            if (cfg.groupConfig.x == 0 && cfg.groupConfig.y == 0) {
                // Try legacy keys as fallback or set default
                String legacyX = configProvider != null ? configProvider.getConfig("flightInfoX") : null;
                String legacyY = configProvider != null ? configProvider.getConfig("flightInfoY") : null;
                if (legacyX != null)
                    cfg.groupConfig.x = Double.parseDouble(legacyX) / prog.Application.screenWidth; // Approximate if
                                                                                                    // stored as pixels
                if (legacyY != null)
                    cfg.groupConfig.y = Double.parseDouble(legacyY) / prog.Application.screenHeight;
            }
        }

        // Add all standard flight info fields
        // These were previously hardcoded in FlightInfo.initFields()
        cfg.addFieldDefinition("ias", Lang.fIAS, "Km/h", "disableFlightInfoIAS", false, "500");
        cfg.addFieldDefinition("tas", Lang.fTAS, "Km/h", "disableFlightInfoTAS", false, "550");
        cfg.addFieldDefinition("mach", Lang.fMach, "Mach", "disableFlightInfoMach", false, "0.45");
        cfg.addFieldDefinition("dir", Lang.fCompass, "Deg", "disableFlightInfoCompass", false, "270");
        cfg.addFieldDefinition("height", Lang.fAlt, "M", "disableFlightInfoHeight", false, "1500");
        cfg.addFieldDefinition("rda", Lang.fRa, "M", "disableFlightInfoRadioAlt", true, "325");
        cfg.addFieldDefinition("vario", Lang.fVario, "M/s", "disableFlightInfoVario", false, "10");
        cfg.addFieldDefinition("sep", Lang.fSEP, "M/s", "disableFlightInfoSEP", false, "15");
        cfg.addFieldDefinition("acc", Lang.fAcc, "M/s^2", "disableFlightInfoAcc", false, "1.2");
        cfg.addFieldDefinition("wx", Lang.fWx, "Deg/s", "disableFlightInfoWx", false, "5.0");
        cfg.addFieldDefinition("ny", Lang.fGL, "G", "disableFlightInfoNy", false, "1.0");
        cfg.addFieldDefinition("turn", Lang.fTRr, "Deg/s", "disableFlightInfoTurn", false, "2.5");
        cfg.addFieldDefinition("rds", Lang.fTR, "M", "disableFlightInfoTurnRadius", false, "800");
        cfg.addFieldDefinition("aoa", Lang.fAoA, "Deg", "disableFlightInfoAoA", false, "4.2");
        cfg.addFieldDefinition("aos", Lang.fAoS, "Deg", "disableFlightInfoAoS", false, "0.5");
        cfg.addFieldDefinition("ws", Lang.fWs, "%", "disableFlightInfoWingSweep", true, "15");

        return cfg;
    }
}
