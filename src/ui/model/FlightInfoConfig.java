package ui.model;

import java.util.ArrayList;
import java.util.List;

import prog.lang;
import prog.config.ConfigProvider;

/**
 * Configuration for FlightInfo overlay.
 * Externalizes all configuration from the flightInfo class.
 */
public class FlightInfoConfig {

    // Field definitions
    private List<FieldDefinition> fieldDefinitions = new ArrayList<>();

    // Overlay title
    public String title = "flightInfo";

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

    public List<FieldDefinition> getFieldDefinitions() {
        return fieldDefinitions;
    }

    public void addFieldDefinition(String key, String label, String unit, String configKey, boolean hideWhenNA) {
        fieldDefinitions.add(new FieldDefinition(key, label, unit, configKey, hideWhenNA));
    }

    /**
     * Create default configuration with standard flight info fields.
     * This factory method contains the field definitions that were previously
     * hardcoded in flightInfo.
     */
    public static FlightInfoConfig createDefault(ConfigProvider config) {
        FlightInfoConfig cfg = new FlightInfoConfig();

        // Load style settings from config
        if (config != null) {
            cfg.showEdge = "true".equals(config.getConfig("flightInfoEdge"));
            String colStr = config.getConfig("flightInfoColumn");
            if (colStr != null && !colStr.isEmpty()) {
                try {
                    cfg.columnNum = Integer.parseInt(colStr);
                } catch (NumberFormatException e) {
                    cfg.columnNum = 3;
                }
            }
        }

        // Add all standard flight info fields
        // These were previously hardcoded in flightInfo.initFields()
        cfg.addFieldDefinition("ias", lang.fIAS, "Km/h", "disableFlightInfoIAS", false);
        cfg.addFieldDefinition("tas", lang.fTAS, "Km/h", "disableFlightInfoTAS", false);
        cfg.addFieldDefinition("mach", lang.fMach, "Mach", "disableFlightInfoMach", false);
        cfg.addFieldDefinition("dir", lang.fCompass, "Deg", "disableFlightInfoCompass", false);
        cfg.addFieldDefinition("height", lang.fAlt, "M", "disableFlightInfoHeight", false);
        cfg.addFieldDefinition("rda", lang.fRa, "M", "disableFlightInfoRadioAlt", true);
        cfg.addFieldDefinition("vario", lang.fVario, "M/s", "disableFlightInfoVario", false);
        cfg.addFieldDefinition("sep", lang.fSEP, "M/s", "disableFlightInfoSEP", false);
        cfg.addFieldDefinition("acc", lang.fAcc, "M/s^2", "disableFlightInfoAcc", false);
        cfg.addFieldDefinition("wx", lang.fWx, "Deg/s", "disableFlightInfoWx", false);
        cfg.addFieldDefinition("ny", lang.fGL, "G", "disableFlightInfoNy", false);
        cfg.addFieldDefinition("turn", lang.fTRr, "Deg/s", "disableFlightInfoTurn", false);
        cfg.addFieldDefinition("rds", lang.fTR, "M", "disableFlightInfoTurnRadius", false);
        cfg.addFieldDefinition("aoa", lang.fAoA, "Deg", "disableFlightInfoAoA", false);
        cfg.addFieldDefinition("aos", lang.fAoS, "Deg", "disableFlightInfoAoS", false);
        cfg.addFieldDefinition("ws", lang.fWs, "%", "disableFlightInfoWingSweep", true);

        return cfg;
    }
}
