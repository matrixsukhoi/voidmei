package prog.config;

import java.awt.Color;
import java.awt.RenderingHints;

import prog.Application;
import prog.Controller;
import prog.event.UIStateBus;
import prog.event.UIStateEvents;

/**
 * Main configuration Service implementing ConfigProvider.
 * Handles loading, saving, and accessing application configuration.
 */
public class ConfigurationService implements ConfigProvider {
    private java.util.List<ConfigLoader.GroupConfig> layoutConfigs;
    // private Timer saveTimer; // Removed
    // private final long SAVE_DELAY = 2000; // Removed

    public ConfigurationService() {
        // Initialize config class (property loader)
        // Note: Actual loading happens in initConfig()
    }

    public void initConfig() {
        // Load layout config
        loadLayout("./ui_layout.cfg");

        // Subscribe to global reset requests (EDA implementation)
        UIStateBus.getInstance().subscribe(UIStateEvents.CONFIG_CHANGED, key -> {
            if (UIStateEvents.ACTION_RESET_REQUEST.equals(key)) {
                resetAllLayoutDefaults();
            }
        });
    }

    public void loadLayout(String path) {
        layoutConfigs = ConfigLoader.loadConfig(path);
        prog.util.Logger.info("ConfigurationService", "Loaded layout config with " + layoutConfigs.size() + " groups.");
    }

    public void saveLayoutConfig() {
        if (layoutConfigs != null) {
            prog.util.Logger.info("ConfigurationService", "ACTION: ConfigurationService: Saving to ui_layout.cfg");
            ConfigLoader.saveConfig("./ui_layout.cfg", layoutConfigs);
        }
    }

    public java.util.List<ConfigLoader.GroupConfig> getLayoutConfigs() {
        return layoutConfigs;
    }

    public void loadAppCheck(Controller c) {
        // Parse and apply config to app and Controller State
        // This replaces loadFromConfig() in Controller

        long freqService = Long.parseLong(getConfig("Interval"));
        c.freqService = freqService;
        c.freqEngineInfo = (long) (freqService * 2f);
        c.freqFlightInfo = (long) (freqService * 1.5f);
        c.freqAltitude = (long) (freqService * 1.5f);
        c.freqGearAndFlap = (long) (freqService * 2f);
        c.freqStickValue = (long) (freqService * 1f);
        Application.threadSleepTime = (long) (freqService / 3);

        Application.colorNum = getColorConfig("fontNum");
        Application.colorLabel = getColorConfig("fontLabel");
        Application.colorUnit = getColorConfig("fontUnit");
        Application.colorWarning = getColorConfig("fontWarn");
        Application.colorShadeShape = getColorConfig("fontShade");

        Application.voiceVolumn = Integer.parseInt(getConfig("voiceVolume"));

        // Application.drawFontShape = !Boolean.parseBoolean(getConfig("simpleFont"));
        Application.aaEnable = Boolean.parseBoolean(getConfig("AAEnable"));

        if (Application.aaEnable) {
            Application.textAASetting = RenderingHints.VALUE_TEXT_ANTIALIAS_ON;
            Application.graphAASetting = RenderingHints.VALUE_ANTIALIAS_ON;
        } else {
            Application.textAASetting = RenderingHints.VALUE_TEXT_ANTIALIAS_OFF;
            Application.graphAASetting = RenderingHints.VALUE_ANTIALIAS_OFF;
        }

        String fmKey = getConfig("displayFmKey");
        if (fmKey != null && !fmKey.isEmpty()) {
            try {
                Application.displayFmKey = Integer.parseInt(fmKey);
            } catch (NumberFormatException e) {
                // Ignore
            }
        }

        // --- Sync Global Fonts ---
        String globalNumFont = getConfig("GlobalNumFont");
        if (globalNumFont != null && !globalNumFont.isEmpty()) {
            Application.defaultNumfontName = globalNumFont;
            prog.util.Logger.info("ConfigurationService", "Global Num font synchronized: " + globalNumFont);
        }

        String globalTextFont = getConfig("GlobalTextFont");
        if (globalTextFont != null && !globalTextFont.isEmpty()) {
            Application.defaultFontName = globalTextFont;
            prog.util.Logger.info("ConfigurationService", "Global Text font synchronized: " + globalTextFont);
        }

        Application.initFont();
        Application.updateWebLafFonts();
    }

    // --- ConfigProvider Implementation ---

    @Override
    public String getConfig(String key) {
        // Priority 1: Check in-memory LayoutConfigs (Source of Truth)
        if (layoutConfigs != null) {
            for (ConfigLoader.GroupConfig gc : layoutConfigs) {
                for (ConfigLoader.RowConfig row : gc.rows) {
                    if (key.equals(row.property) || (row.property == null && key.equals(row.label))) {
                        // Handle inversion for SWITCH_INV
                        if ("SWITCH_INV".equals(row.type)) {
                            return String.valueOf(!row.getBool());
                        }
                        return row.getStr();
                    }
                }

                // Check Group SwitchKey (e.g. flightInfoSwitch maps to Group visibility)
                if (key.equals(gc.switchKey)) {
                    return String.valueOf(gc.visible);
                }
            }
        }
        return "";
    }

    @Override
    public void setConfig(String key, String value) {
        // 1. Update LayoutConfigs (if exists)
        if (layoutConfigs != null) {
            for (ConfigLoader.GroupConfig gc : layoutConfigs) {
                // Check Rows
                for (ConfigLoader.RowConfig row : gc.rows) {
                    if (key.equals(row.property) || (row.property == null && key.equals(row.label))) {
                        // Update typed value based on existing type to maintain consistency
                        // Handle inversion for SWITCH_INV
                        String valToStore = value;
                        if ("SWITCH_INV".equals(row.type)) {
                            valToStore = String.valueOf(!Boolean.parseBoolean(value));
                        }

                        if (row.value instanceof Boolean) {
                            row.value = Boolean.parseBoolean(valToStore);
                        } else if (row.value instanceof Integer) {
                            try {
                                row.value = Integer.parseInt(valToStore);
                            } catch (Exception e) {
                                row.value = valToStore;
                            }
                        } else {
                            row.value = valToStore;
                        }
                        UIStateBus.getInstance().publish(UIStateEvents.CONFIG_CHANGED, "ConfigurationService", key);
                    }
                }

                // Check Group SwitchKey
                if (key.equals(gc.switchKey)) {
                    gc.visible = Boolean.parseBoolean(value);
                    UIStateBus.getInstance().publish(UIStateEvents.CONFIG_CHANGED, "ConfigurationService", key);
                }
            }
        }
    }

    /**
     * Resets all configuration items in the layout to their default values.
     * 
     * @return true if any configuration was changed
     */
    public boolean resetAllLayoutDefaults() {
        boolean changed = false;
        java.util.List<ConfigLoader.RowConfig> pendingChanges = new java.util.ArrayList<>();

        // Phase 1: Collect changes (Prepare)
        if (layoutConfigs != null) {
            for (ConfigLoader.GroupConfig g : layoutConfigs) {
                for (ConfigLoader.RowConfig r : g.rows) {
                    if (r.defaultValue != null && !r.defaultValue.equals(r.value)) {
                        pendingChanges.add(r);
                    }
                }
            }
        }

        // Phase 2: Apply changes (Commit)
        if (!pendingChanges.isEmpty()) {
            for (ConfigLoader.RowConfig r : pendingChanges) {
                prog.util.Logger.info("ConfigReset", "Resetting " + r.label + " ("
                        + (r.property != null ? r.property : "no-key") + ") to default: "
                        + r.defaultValue);
                r.value = r.defaultValue;
            }
            changed = true;
        }

        // Phase 3: Persist and Notify
        if (changed) {
            saveLayoutConfig();
            // Broadcast global reset event so all components refresh
            UIStateBus.getInstance().publish(UIStateEvents.CONFIG_CHANGED,
                    "ConfigurationService", UIStateEvents.ACTION_RESET_COMPLETED);
        }
        return changed;
    }

    // private synchronized void scheduleBackgroundSave() { ... } // Removed

    // --- Helpers ---

    public void saveConfig() {
        // No longer using config.properties, all settings in ui_layout.cfg
    }

    public Color getColorConfig(String key) {
        String unifiedVal = getConfig(key);
        if (unifiedVal != null && !unifiedVal.isEmpty()) {
            try {
                String[] parts = unifiedVal.split(",");
                if (parts.length >= 3) {
                    int r = Integer.parseInt(parts[0].trim());
                    int g = Integer.parseInt(parts[1].trim());
                    int b = Integer.parseInt(parts[2].trim());
                    int a = (parts.length > 3) ? Integer.parseInt(parts[3].trim()) : 255;
                    return new Color(r, g, b, a);
                }
            } catch (Exception e) {
                // Formatting error
            }
        }
        return Color.WHITE;
    }

    public void setColorConfig(String key, Color c) {
        int R = c.getRed();
        int G = c.getGreen();
        int B = c.getBlue();
        int A = c.getAlpha();
        String unified = R + ", " + G + ", " + B + ", " + A;

        setConfig(key, unified);
    }

    // --- OverlaySettings Implementation ---

    public OverlaySettings getOverlaySettings(String sectionName) {
        return new GenericOverlaySettingsImpl(sectionName);
    }

    private class GenericOverlaySettingsImpl implements OverlaySettings {
        protected final String sectionName;

        public GenericOverlaySettingsImpl(String sectionName) {
            this.sectionName = sectionName;
        }

        @Override
        public ConfigLoader.GroupConfig getGroupConfig() {
            if (layoutConfigs == null)
                return null;
            for (ConfigLoader.GroupConfig gc : layoutConfigs) {
                if (sectionName.equalsIgnoreCase(gc.title)) {
                    return gc;
                }
            }
            return null;
        }

        @Override
        public int getWindowX(int width) {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null) {
                int px = (int) Math.round(gc.x * Application.screenWidth);
                prog.util.Logger.debug("OverlaySettings", String.format("[%s] getWindowX: gc.x=%.4f, screen=%d => %d",
                        sectionName, gc.x, Application.screenWidth, px));
                return px;
            }
            int cx = (Application.screenWidth - width) / 2;
            prog.util.Logger.debug("OverlaySettings",
                    String.format("[%s] getWindowX: gc=null => center %d", sectionName, cx));
            return cx;
        }

        @Override
        public int getWindowY(int height) {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null) {
                int py = (int) Math.round(gc.y * Application.screenHeight);
                prog.util.Logger.debug("OverlaySettings", String.format("[%s] getWindowY: gc.y=%.4f, screen=%d => %d",
                        sectionName, gc.y, Application.screenHeight, py));
                return py;
            }
            int cy = (Application.screenHeight - height) / 2;
            prog.util.Logger.debug("OverlaySettings",
                    String.format("[%s] getWindowY: gc=null => center %d", sectionName, cy));
            return cy;
        }

        @Override
        public void saveWindowPosition(double x, double y) {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null && Application.screenWidth > 0 && Application.screenHeight > 0) {
                gc.x = x / Application.screenWidth;
                gc.y = y / Application.screenHeight;
                prog.util.Logger.debug("OverlaySettings", String
                        .format("[%s] saveWindowPosition: %f,%f => rel %.4f,%.4f", sectionName, x, y, gc.x, gc.y));
                saveLayoutConfig();
            } else {
                prog.util.Logger.warn("OverlaySettings", String.format("[%s] CANNOT save position: gc=%s, screen=%dx%d",
                        sectionName, (gc != null ? "OK" : "null"), Application.screenWidth, Application.screenHeight));
            }
        }

        @Override
        public String getFontName() {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null && gc.fontName != null && !gc.fontName.isEmpty()) {
                return gc.fontName;
            }
            String globalFont = getConfig("GlobalTextFont");
            if (globalFont != null && !globalFont.isEmpty()) {
                return globalFont;
            }
            return Application.defaultFont.getFontName();
        }

        @Override
        public String getNumFontName() {
            String globalFont = getConfig("GlobalNumFont");
            if (globalFont != null && !globalFont.isEmpty()) {
                return globalFont;
            }
            return Application.defaultNumfontName;
        }

        @Override
        public int getFontSizeAdd() {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            return (gc != null) ? gc.fontSize : 0;
        }

        @Override
        public boolean getBool(String key, boolean def) {
            String val = getConfig(key);
            if (val == null || val.isEmpty())
                return def;
            return Boolean.parseBoolean(val);
        }

        @Override
        public int getInt(String key, int def) {
            String val = getConfig(key);
            if (val == null || val.isEmpty())
                return def;
            try {
                return Integer.parseInt(val);
            } catch (NumberFormatException e) {
                return def;
            }
        }

        @Override
        public String getString(String key, String def) {
            String val = getConfig(key);
            if (val == null || val.isEmpty())
                return def;
            return val;
        }
    }

    // --- HUDSettings Implementation ---

    public HUDSettings getHUDSettings() {
        return new HUDSettingsImpl();
    }

    private class HUDSettingsImpl extends GenericOverlaySettingsImpl implements HUDSettings {
        public HUDSettingsImpl() {
            super("MiniHUD");
        }

        private double getDouble(String key, double def) {
            String val = getConfig(key);
            try {
                return val.isEmpty() ? def : Double.parseDouble(val);
            } catch (NumberFormatException e) {
                return def;
            }
        }

        @Override
        public String getNumFont() {
            String font = getConfig("MonoNumFont");
            if (font == null || font.isEmpty()) {
                font = getConfig("GlobalNumFont");
            }
            return (font == null || font.isEmpty()) ? Application.defaultNumfontName : font;
        }

        @Override
        public String getNumFontName() {
            return getNumFont();
        }

        @Override
        public int getWindowX(int canvasWidth) {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null) {
                return (int) Math.round(gc.x * Application.screenWidth);
            }
            return getInt("crosshairX", (Application.screenWidth - canvasWidth) / 2);
        }

        @Override
        public int getWindowY(int canvasHeight) {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null) {
                return (int) Math.round(gc.y * Application.screenHeight);
            }
            return getInt("crosshairY", (Application.screenHeight - canvasHeight) / 2);
        }

        @Override
        public void saveWindowPosition(double x, double y) {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null) {
                gc.x = x / Application.screenWidth;
                gc.y = y / Application.screenHeight;
                saveLayoutConfig();
            } else {
                setConfig("crosshairX", Integer.toString((int) x));
                setConfig("crosshairY", Integer.toString((int) y));
            }
        }

        @Override
        public int getCrosshairScale() {
            int scale = getInt("crosshairScale", 70);
            return scale == 0 ? 1 : scale;
        }

        @Override
        public String getCrosshairName() {
            return getConfig("crosshairName").trim();
        }

        @Override
        public boolean isDisplayCrosshair() {
            return getBool("displayCrosshair", false);
        }

        @Override
        public boolean useTextureCrosshair() {
            return getBool("usetexturecrosshair", false);
        }

        @Override
        public boolean drawHUDText() {
            return getBool("drawHUDtext", true);
        }

        @Override
        public boolean drawHUDAttitude() {
            return getBool("drawHUDAttitude", true);
        }

        private double getDoubleFromLayoutFirst(String section, String property, double defaultVal) {
            // Priority 1: Check in-memory LayoutConfigs
            if (layoutConfigs != null) {
                for (ConfigLoader.GroupConfig gc : layoutConfigs) {
                    if (section.equalsIgnoreCase(gc.title)) {
                        for (ConfigLoader.RowConfig row : gc.rows) {
                            if (property.equals(row.property) && row.value != null) {
                                if (row.value instanceof Number) {
                                    return ((Number) row.value).doubleValue();
                                }
                                try {
                                    return Double.parseDouble(row.value.toString());
                                } catch (NumberFormatException e) {
                                    // ignore
                                }
                            }
                        }
                    }
                }
            }
            // Priority 2: Check global config.properties
            return getDouble(property, defaultVal);
        }

        @Override
        public double getAoAWarningRatio() {
            double val = getDoubleFromLayoutFirst(sectionName, "miniHUDaoaWarningRatio", 25);
            return (val > 1.0) ? val / 100.0 : val;
        }

        @Override
        public double getAoABarWarningRatio() {
            double val = getDoubleFromLayoutFirst(sectionName, "miniHUDaoaBarWarningRatio", 0);
            return (val > 1.0) ? val / 100.0 : val;
        }

        @Override
        public boolean enableFlapAngleBar() {
            return getBool("enableFlapAngleBar", true);
        }

        @Override
        public boolean drawHudMach() {
            return getBool("hudMach", false);
        }

        @Override
        public boolean isSpeedLabelDisabled() {
            return getBool("disableHUDSpeedLabel", false);
        }

        @Override
        public boolean isAltitudeLabelDisabled() {
            return getBool("disableHUDHeightLabel", false);
        }

        @Override
        public boolean isSEPLabelDisabled() {
            return getBool("disableHUDSEPLabel", false);
        }

        @Override
        public boolean isAoADisabled() {
            return getBool("disableHUDAoA", false);
        }
    }
}
