package prog.config;

import java.awt.Color;
import java.awt.RenderingHints;
import prog.Application;
import prog.Controller;

/**
 * Main configuration Service implementing ConfigProvider.
 * Handles loading, saving, and accessing application configuration.
 */
public class ConfigurationService implements ConfigProvider {
    private Config cfg;

    public ConfigurationService() {
        // Initialize config class (property loader)
        // Note: Actual loading happens in initConfig()
    }

    public void initConfig() {
        cfg = new Config("./config/config.properties");
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
        changed |= checkDefault("GlobalNumFont", "");

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
        if (cfg != null)
            cfg.setValue(key, value);
    }

    // --- Helpers ---

    public void saveConfig() {
        if (cfg != null)
            cfg.saveFile("./config/config.properties", "8111");
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
}
