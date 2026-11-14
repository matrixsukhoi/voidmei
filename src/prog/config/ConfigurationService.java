package prog.config;

import java.awt.Color;
import java.awt.RenderingHints;

import prog.Application;
import prog.util.ColorHelper;
import prog.Controller;
import prog.i18n.Lang;
import java.net.InetSocketAddress;
import prog.event.UIStateBus;
import prog.event.UIStateEvents;

/**
 * Main configuration Service implementing ConfigProvider.
 * Handles loading, saving, and accessing application configuration.
 */
public class ConfigurationService implements ConfigProvider {
    private java.util.List<ConfigLoader.GroupConfig> layoutConfigs;
    // 数据字段目录缓存：配置重载/导入/出厂后必须清空
    private MiniHudFieldCatalog fieldCatalogCache;
    // private Timer saveTimer; // Removed
    // private final long SAVE_DELAY = 2000; // Removed

    public ConfigurationService() {
        // Initialize config class (property loader)
        // Note: Actual loading happens in initConfig()
    }

    public void initConfig() {
        // Load layout config using ConfigManager (handles first-run, upgrade, errors)
        layoutConfigs = ConfigManager.initialize();
        fieldCatalogCache = null;
        prog.util.Logger.info("ConfigurationService", "Loaded layout config with " + layoutConfigs.size() + " groups.");

        // Subscribe to global reset requests (EDA implementation)
        UIStateBus.getInstance().subscribe(UIStateEvents.CONFIG_CHANGED, key -> {
            if (UIStateEvents.ACTION_RESET_REQUEST.equals(key)) {
                resetAllLayoutDefaults();
            }
        });
    }

    public void loadLayout(String path) {
        layoutConfigs = ConfigLoader.loadConfig(path);
        fieldCatalogCache = null;
        prog.util.Logger.info("ConfigurationService", "Loaded layout config with " + layoutConfigs.size() + " groups.");
    }

    public void saveLayoutConfig() {
        if (layoutConfigs != null) {
            prog.util.Logger.info("ConfigurationService", "ACTION: ConfigurationService: Saving to " + ConfigManager.getUserConfigPath());
            ConfigLoader.saveConfig(ConfigManager.getUserConfigPath(), layoutConfigs);
        }
    }

    /**
     * Imports configuration from an external file.
     *
     * @param sourcePath Path to the config file to import
     * @return true if import was successful
     */
    public boolean importConfig(String sourcePath) {
        boolean success = ConfigManager.importConfig(sourcePath);
        if (success) {
            // Reload configuration
            layoutConfigs = ConfigLoader.loadConfig(ConfigManager.getUserConfigPath());
            fieldCatalogCache = null;
            // Notify all subscribers about the change
            UIStateBus.getInstance().publish(UIStateEvents.CONFIG_CHANGED,
                    "ConfigurationService", UIStateEvents.ACTION_RESET_COMPLETED);
        }
        return success;
    }

    /**
     * Resets configuration to factory defaults.
     *
     * @return true if reset was successful
     */
    public boolean resetToFactory() {
        boolean success = ConfigManager.resetToFactory();
        if (success) {
            // Reload configuration
            layoutConfigs = ConfigLoader.loadConfig(ConfigManager.getUserConfigPath());
            fieldCatalogCache = null;
            // Notify all subscribers about the change
            UIStateBus.getInstance().publish(UIStateEvents.CONFIG_CHANGED,
                    "ConfigurationService", UIStateEvents.ACTION_RESET_COMPLETED);
        }
        return success;
    }

    /** 数据字段目录（懒加载，配置重载后自动重建）。 */
    public MiniHudFieldCatalog getMiniHudFieldCatalog() {
        if (fieldCatalogCache == null) {
            fieldCatalogCache = MiniHudFieldCatalog.from(layoutConfigs);
        }
        return fieldCatalogCache;
    }

    public java.util.List<ConfigLoader.GroupConfig> getLayoutConfigs() {
        return layoutConfigs;
    }

    /**
     * 根据 groupTitle 查找最新的 GroupConfig
     * 用于 UI 组件在 rebuild 时获取最新配置，解决导入配置后引用陈旧的问题
     *
     * @param groupTitle 要查找的 GroupConfig 的标题
     * @return 找到的 GroupConfig，如果未找到则返回 null
     */
    public ConfigLoader.GroupConfig findGroupByTitle(String groupTitle) {
        if (layoutConfigs == null || groupTitle == null) {
            return null;
        }
        for (ConfigLoader.GroupConfig gc : layoutConfigs) {
            if (groupTitle.equals(gc.title)) {
                return gc;
            }
        }
        return null;
    }

    public void loadAppCheck(Controller c) {
        // Parse and apply config to app and Controller State
        // This replaces loadFromConfig() in Controller

        long serviceLoopIntervalMs = 50;
        try {
            // Try new config key first, fallback to legacy key for backward compatibility
            String intervalStr = getConfig("dataPollIntervalMs");
            if (intervalStr == null || intervalStr.isEmpty()) {
                intervalStr = getConfig("Interval");  // Legacy key fallback
            }
            if (intervalStr != null && !intervalStr.isEmpty()) {
                serviceLoopIntervalMs = Long.parseLong(intervalStr);
            }
        } catch (NumberFormatException e) {
            serviceLoopIntervalMs = 50;
        }
        c.serviceLoopIntervalMs = serviceLoopIntervalMs;
        c.engineInfoIntervalMs = (long) (serviceLoopIntervalMs * 2f);
        c.flightInfoIntervalMs = (long) (serviceLoopIntervalMs * 1.5f);
        c.altitudeIntervalMs = (long) (serviceLoopIntervalMs * 1.5f);
        c.gearFlapsIntervalMs = (long) (serviceLoopIntervalMs * 2f);
        c.controlInputIntervalMs = (long) (serviceLoopIntervalMs * 1f);
        Application.threadSleepTime = (long) (serviceLoopIntervalMs / 3);

        Application.colorNum = getColorConfig("fontNum");
        Application.colorLabel = getColorConfig("fontLabel");
        Application.colorUnit = getColorConfig("fontUnit");
        Application.colorWarning = getColorConfig("fontWarn");
        Application.colorShadeShape = getColorConfig("fontShade");

        try {
            String vol = getConfig("voiceVolume");
            if (vol != null && !vol.isEmpty()) {
                Application.voiceVolumn = Integer.parseInt(vol);
            }
        } catch (NumberFormatException e) {
            Application.voiceVolumn = 0; // Default
        }

        // Application.drawFontShape = !Boolean.parseBoolean(getConfig("simpleFont"));
        String aa = getConfig("AAEnable");
        if (aa != null && !aa.isEmpty()) {
            Application.aaEnable = Boolean.parseBoolean(aa);
        } else {
            Application.aaEnable = false; // Default
        }

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

        // --- HTTP Port Sync ---
        try {
            String portStr = getConfig("httpPort");
            if (portStr != null && !portStr.isEmpty()) {
                int port = Integer.parseInt(portStr);
                Application.appPort = port;
                Application.appPortBkp = port + 1111;
                // Assuming httpIp is still from Lang or static 127.0.0.1
                String ip = "127.0.0.1";
                if (Lang.httpIp != null && !Lang.httpIp.isEmpty()) {
                    ip = Lang.httpIp;
                }
                Application.requestDest = new InetSocketAddress(ip, Application.appPort);
                Application.requestDestBkp = new InetSocketAddress(ip, Application.appPortBkp);
                prog.util.Logger.info("ConfigurationService", "HTTP Port synchronized: " + port);
            }
        } catch (NumberFormatException e) {
            // Ignore
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
    public boolean isFieldDisabled(String key) {
        if (key == null || key.isEmpty())
            return false;

        if (layoutConfigs != null) {
            for (ConfigLoader.GroupConfig gc : layoutConfigs) {
                ConfigLoader.RowConfig row = findRowRecursive(gc.rows, key);
                if (row != null) {
                    if ("SWITCH_INV".equals(row.type)) {
                        // For SWITCH_INV, the key is usually "disableXXX".
                        // Logic must match getConfig() which returns !row.getBool().
                        // If getConfig() returns "true", it is DISABLED.
                        // Thus isFieldDisabled should return !row.getBool().
                        return !row.getBool();
                    }
                    if (row.value instanceof Boolean) {
                        // For 'data' or 'switch', if value is false, it means it is HIDDEN/DISABLED.
                        return !((Boolean) row.value);
                    }
                }
            }
        }
        return false;
    }

    @Override
    public String getConfig(String key) {
        // Priority 1: Check in-memory LayoutConfigs (Source of Truth)
        if (layoutConfigs != null) {
            for (ConfigLoader.GroupConfig gc : layoutConfigs) {
                ConfigLoader.RowConfig row = findRowRecursive(gc.rows, key);
                if (row != null) {
                    // Handle inversion for SWITCH_INV
                    if ("SWITCH_INV".equals(row.type)) {
                        return String.valueOf(!row.getBool());
                    }
                    return row.getStr();
                }

                // Check Group SwitchKey (e.g. flightInfoSwitch maps to Group visibility)
                if (key.equals(gc.switchKey)) {
                    return String.valueOf(gc.visible);
                }
            }
        }
        return "";
    }

    private ConfigLoader.RowConfig findRowRecursive(java.util.List<ConfigLoader.RowConfig> rows, String key) {
        for (ConfigLoader.RowConfig row : rows) {
            // Priority 1: Match property target exactly
            if (key.equals(row.property)) {
                return row;
            }
            // Priority 2: Match label if property is missing
            if (row.property == null && key.equals(row.label)) {
                return row;
            }
            // Recurse
            if (row.children != null && !row.children.isEmpty()) {
                ConfigLoader.RowConfig found = findRowRecursive(row.children, key);
                if (found != null)
                    return found;
            }
        }
        return null;
    }

    @Override
    public void setConfig(String key, String value) {
        // 1. Update LayoutConfigs (if exists)
        if (layoutConfigs != null) {
            for (ConfigLoader.GroupConfig gc : layoutConfigs) {
                // Check Rows - recursive update ALL matching instances
                updateRowsRecursive(gc.rows, key, value);

                // Check Group SwitchKey
                if (key.equals(gc.switchKey)) {
                    gc.visible = Boolean.parseBoolean(value);
                    UIStateBus.getInstance().publish(UIStateEvents.CONFIG_CHANGED, "ConfigurationService", key);
                }
            }
        }
    }

    private void updateRowsRecursive(java.util.List<ConfigLoader.RowConfig> rows, String key, String value) {
        for (ConfigLoader.RowConfig row : rows) {
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
            if (row.children != null && !row.children.isEmpty()) {
                updateRowsRecursive(row.children, key, value);
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
                collectResetCandidatesRecursive(g.rows, pendingChanges);
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

    private void collectResetCandidatesRecursive(java.util.List<ConfigLoader.RowConfig> rows,
            java.util.List<ConfigLoader.RowConfig> pendingChanges) {
        for (ConfigLoader.RowConfig r : rows) {
            // Check self
            if (r.defaultValue != null && !r.defaultValue.equals(r.value)) {
                pendingChanges.add(r);
            }
            // Recurse
            if (r.children != null && !r.children.isEmpty()) {
                collectResetCandidatesRecursive(r.children, pendingChanges);
            }
        }
    }

    // private synchronized void scheduleBackgroundSave() { ... } // Removed

    // --- Helpers ---

    public void saveConfig() {
        // No longer using config.properties, all settings in ui_layout.cfg
    }

    public Color getColorConfig(String key) {
        String val = getConfig(key);
        return ColorHelper.parseColor(val, Color.WHITE);
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
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null) {
                ConfigLoader.RowConfig row = findRowRecursive(gc.rows, key);
                if (row != null) {
                    // Handle inversion for SWITCH_INV
                    if ("SWITCH_INV".equals(row.type)) {
                        return !row.getBool();
                    }
                    return row.getBool();
                }
            }
            String val = getConfig(key);
            if (val == null || val.isEmpty())
                return def;
            return Boolean.parseBoolean(val);
        }

        @Override
        public int getInt(String key, int def) {
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null) {
                ConfigLoader.RowConfig row = findRowRecursive(gc.rows, key);
                if (row != null) {
                    return row.getInt();
                }
            }
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
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null) {
                ConfigLoader.RowConfig row = findRowRecursive(gc.rows, key);
                if (row != null) {
                    return row.getStr();
                }
            }
            String val = getConfig(key);
            if (val == null || val.isEmpty())
                return def;
            return val;
        }

        @Override
        public boolean autoHideOnFocusLoss() {
            // 默认开启：失焦时隐藏所有 overlay
            return getBool("autoHideOnFocusLoss", true);
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
        public int getMiniHUDScale() {
            // 历史键名保持 crosshairScale：改名会让所有老用户的 MiniHUD 尺寸被重置
            int scale = getInt("crosshairScale", 113);
            return scale == 0 ? 1 : scale;
        }

        private double getDoubleFromLayoutFirst(String section, String property, double defaultVal) {
            // Priority 1: Check in-memory LayoutConfigs
            if (layoutConfigs != null) {
                for (ConfigLoader.GroupConfig gc : layoutConfigs) {
                    if (section.equalsIgnoreCase(gc.title)) {
                        ConfigLoader.RowConfig row = findRowRecursive(gc.rows, property);
                        if (row != null && row.value != null) {
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
        public boolean isGPUCompatibilityMode() {
            return getBool("gpuCompatibilityMode", false);
        }

        @Override
        public boolean isMiniHudContentVisible() {
            // 迁移期兼容：新键不存在时沿用旧 drawHUDtext
            ConfigLoader.GroupConfig gc = getGroupConfig();
            if (gc != null && findRowRecursive(gc.rows, "miniHudContentVisible") == null) {
                return getBool("drawHUDtext", true);
            }
            return getBool("miniHudContentVisible", true);
        }

        @Override
        public String getMiniHudLayout() {
            String s = getString("miniHudLayout", "");
            return s != null ? s : "";
        }

        @Override
        public void saveMiniHudLayout(String encoded) {
            setConfig("miniHudLayout", encoded != null ? encoded : "");
            saveLayoutConfig();
        }

        @Override
        public MiniHudFieldCatalog getMiniHudFieldCatalog() {
            return ConfigurationService.this.getMiniHudFieldCatalog();
        }

        @Override
        public boolean isMiniHudAirfieldArrow() {
            return getBool("miniHudAirfieldArrow", true);
        }

        @Override
        public boolean isMiniHudAirfieldShowAll() {
            // combo 存中文标签；缺省/未知值按“全部”处理
            return !"最近".equals(getString("miniHudAirfieldRange", "全部"));
        }

        @Override
        public boolean isMiniHudAirfieldShowGrid() {
            return getBool("miniHudAirfieldShowGrid", true);
        }

        @Override
        public Color getMiniHudAirfieldFriendlyColor() {
            return ColorHelper.parseColor(getString("miniHudAirfieldFriendlyColor", "#1BFF80F0"),
                    new Color(27, 255, 128, 240));
        }

        @Override
        public Color getMiniHudAirfieldEnemyColor() {
            return ColorHelper.parseColor(getString("miniHudAirfieldEnemyColor", "#FF4444F0"),
                    new Color(255, 68, 68, 240));
        }

        @Override
        public double getMiniHudAirfieldWarnKm() {
            return Math.max(0, getInt("miniHudAirfieldWarnKm", 5));
        }
    }
}
