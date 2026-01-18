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
    private Config cfg;
    private java.util.List<ConfigLoader.GroupConfig> layoutConfigs;
    // private Timer saveTimer; // Removed
    // private final long SAVE_DELAY = 2000; // Removed

    public ConfigurationService() {
        // Initialize config class (property loader)
        // Note: Actual loading happens in initConfig()
    }

    public void initConfig() {
        cfg = new Config("./config/config.properties");

        // Load layout config
        loadLayout("./ui_layout.cfg");

        boolean changed = false;

        changed |= checkDefault("Interval", "300");
        changed |= checkDefault("voiceVolume", "100");
        changed |= checkDefault("enableVoiceWarn", "true");
        changed |= checkDefault("enableFMPrint", "false");
        changed |= checkDefault("enableEngineControl", "false");
        changed |= checkDefault("engineInfoSwitch", "false");
        changed |= checkDefault("flightInfoSwitch", "false");
        changed |= checkDefault("crosshairSwitch", "false");
        changed |= checkDefault("enableAttitudeIndicator", "false");
        changed |= checkDefault("enablegearAndFlaps", "false");
        changed |= checkDefault("enableAxis", "false");
        changed |= checkDefault("thrustdFS", "false");
        changed |= checkDefault("lock", "false");
        changed |= checkDefault("simpleFont", "false");
        changed |= checkDefault("AAEnable", "true");
        changed |= checkDefault("enableStatusBar", "true");
        changed |= checkDefault("AlwaysOnTop", "true");
        changed |= checkDefault("enableLogging", "false");
        changed |= checkDefault("GlobalNumFont", "Sarasa Mono SC");

        // Colors defaults
        changed |= checkColorDefault("fontNum", new Color(32, 222, 64, 140));
        changed |= checkColorDefault("fontLabel", new Color(166, 166, 166, 220));
        changed |= checkColorDefault("fontUnit", new Color(166, 166, 166, 220));
        changed |= checkColorDefault("fontWarn", new Color(216, 33, 13, 100));
        changed |= checkColorDefault("fontShade", new Color(0, 0, 0, 42));

        if (changed) {
            saveConfig();
        }
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

    private boolean checkDefault(String key, String defaultValue) {
        if (cfg.getValue(key).equals("")) {
            cfg.setValue(key, defaultValue);
            return true;
        }
        return false;
    }

    private boolean checkColorDefault(String key, Color defaultColor) {
        if (cfg.getValue(key + "R").equals("")) {
            setColorConfig(key, defaultColor);
            return true;
        }
        return false;
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
    }

    // --- ConfigProvider Implementation ---

    @Override
    public String getConfig(String key) {
        if (cfg == null)
            return "";
        return cfg.getValue(key);
    }

    @Override
    public void setConfig(String key, String value) {
        if (cfg != null) {
            String oldVal = cfg.getValue(key);
            if (!value.equals(oldVal)) {
                cfg.setValue(key, value);

                // Publish event for live updates
                UIStateBus.getInstance().publish(UIStateEvents.CONFIG_CHANGED, "ConfigurationService", key);

                // Schedule debounced save - Removed for manual save strategy
                // scheduleBackgroundSave();
            }
        }
    }

    // private synchronized void scheduleBackgroundSave() { ... } // Removed

    // --- Helpers ---

    public void saveConfig() {
        if (cfg != null) {
            prog.util.Logger.info("ConfigurationService", "ACTION: ConfigurationService: Saving to config.properties");
            cfg.saveFile("./config/config.properties", "8111");
        }
    }

    public Color getColorConfig(String key) {
        int R, G, B, A;
        try {
            R = Integer.parseInt(getConfig(key + "R"));
            G = Integer.parseInt(getConfig(key + "G"));
            B = Integer.parseInt(getConfig(key + "B"));
            A = Integer.parseInt(getConfig(key + "A"));
            return new Color(R, G, B, A);
        } catch (Exception e) {
            return Color.WHITE;
        }
    }

    public void setColorConfig(String key, Color c) {
        int R = c.getRed();
        int G = c.getGreen();
        int B = c.getBlue();
        int A = c.getAlpha();

        setConfig(key + "R", Integer.toString(R));
        setConfig(key + "G", Integer.toString(G));
        setConfig(key + "B", Integer.toString(B));
        setConfig(key + "A", Integer.toString(A));
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

        protected ConfigLoader.GroupConfig getGroupConfig() {
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
                return (int) (gc.x * Application.screenWidth);
            }
            return (Application.screenWidth - width) / 2;
        }

        @Override
        public int getWindowY(int height) {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null) {
                return (int) (gc.y * Application.screenHeight);
            }
            return (Application.screenHeight - height) / 2;
        }

        @Override
        public void saveWindowPosition(double x, double y) {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null) {
                gc.x = x / Application.screenWidth;
                gc.y = y / Application.screenHeight;
                saveLayoutConfig();
            }
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

        private boolean getBool(String key, boolean def) {
            String val = getConfig(key);
            return val.isEmpty() ? def : Boolean.parseBoolean(val);
        }

        private int getInt(String key, int def) {
            String val = getConfig(key);
            try {
                return val.isEmpty() ? def : Integer.parseInt(val);
            } catch (NumberFormatException e) {
                return def;
            }
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
            return font.isEmpty() ? Application.defaultNumfontName : font;
        }

        @Override
        public int getWindowX(int canvasWidth) {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null) {
                return (int) (gc.x * Application.screenWidth);
            }
            return getInt("crosshairX", (Application.screenWidth - canvasWidth) / 2);
        }

        @Override
        public int getWindowY(int canvasHeight) {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null) {
                return (int) (gc.y * Application.screenHeight);
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

        @Override
        public double getAoAWarningRatio() {
            return getDouble("miniHUDaoaWarningRatio", 0.25);
        }

        @Override
        public double getAoABarWarningRatio() {
            return getDouble("miniHUDaoaBarWarningRatio", 0);
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
