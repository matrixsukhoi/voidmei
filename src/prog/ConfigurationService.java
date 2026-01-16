package prog;

import java.awt.Color;
import java.awt.RenderingHints;
import ui.model.ConfigProvider;

public class ConfigurationService implements ConfigProvider {
    private config cfg;

    public ConfigurationService() {
        // Initialize config class (property loader)
        // Note: Actual loading happens in initConfig()
    }

    public void initConfig() {
        cfg = new config("./config/config.properties");
        if (cfg.getValue("Interval") == "") {
            // Defaults
            cfg.setValue("Interval", "300");
            cfg.setValue("voiceVolume", "100");
            cfg.setValue("enableVoiceWarn", "true");
            cfg.setValue("enableFMPrint", "false");
            cfg.setValue("enableEngineControl", "false");
            cfg.setValue("engineInfoSwitch", "false");
            cfg.setValue("flightInfoSwitch", "false");
            cfg.setValue("crosshairSwitch", "false");
            cfg.setValue("enableAttitudeIndicator", "false");
            cfg.setValue("enablegearAndFlaps", "false");
            cfg.setValue("enableAxis", "false");
            cfg.setValue("thrustdFS", "false");
            cfg.setValue("lock", "false"); // lock overlay
            cfg.setValue("simpleFont", "false");
            cfg.setValue("AAEnable", "true");
            cfg.setValue("enableStatusBar", "true");
            cfg.setValue("AlwaysOnTop", "true");
            cfg.setValue("enableLogging", "false");
            cfg.setValue("GlobalNumFont", "");

            // Colors defaults
            setColorConfig("fontNum", new Color(32, 222, 64, 140));
            setColorConfig("fontLabel", new Color(166, 166, 166, 220));
            setColorConfig("fontUnit", new Color(166, 166, 166, 220));
            setColorConfig("fontWarn", new Color(216, 33, 13, 100));
            setColorConfig("fontShade", new Color(0, 0, 0, 42));

            saveConfig();
        }
    }

    public void loadAppCheck(controller c) {
        // Parse and apply config to app and controller state
        // This replaces loadFromConfig() in controller

        long freqService = Long.parseLong(getConfig("Interval"));
        c.freqService = freqService;
        c.freqEngineInfo = (long) (freqService * 2f);
        c.freqFlightInfo = (long) (freqService * 1.5f);
        c.freqAltitude = (long) (freqService * 1.5f);
        c.freqGearAndFlap = (long) (freqService * 2f);
        c.freqStickValue = (long) (freqService * 1f);
        app.threadSleepTime = (long) (freqService / 3);

        app.colorNum = getColorConfig("fontNum");
        app.colorLabel = getColorConfig("fontLabel");
        app.colorUnit = getColorConfig("fontUnit");
        app.colorWarning = getColorConfig("fontWarn");
        app.colorShadeShape = getColorConfig("fontShade");

        app.voiceVolumn = Integer.parseInt(getConfig("voiceVolume"));

        // app.drawFontShape = !Boolean.parseBoolean(getConfig("simpleFont"));
        app.aaEnable = Boolean.parseBoolean(getConfig("AAEnable"));

        if (app.aaEnable) {
            app.textAASetting = RenderingHints.VALUE_TEXT_ANTIALIAS_ON;
            app.graphAASetting = RenderingHints.VALUE_ANTIALIAS_ON;
        } else {
            app.textAASetting = RenderingHints.VALUE_TEXT_ANTIALIAS_OFF;
            app.graphAASetting = RenderingHints.VALUE_ANTIALIAS_OFF;
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
