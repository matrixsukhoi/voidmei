package ui.minihud;

import java.util.Map;

/**
 * 旧 MiniHUD 逐项开关 → 自由画布文档的一次性映射。
 * 缺失键按旧代码默认值处理：showHUD* 默认 true，hudMach 默认 true，
 * 雷达高度/随体模式默认 false。
 */
public final class MiniHudLegacyMigrator {

    public static final String[] LEGACY_KEYS = {
            "showHUDSpeed", "showHUDAoA", "showHUDAltitude", "showHUDEnergy",
            "showHUDFlaps", "showHUDAirbrake", "showHUDGear", "showHUDSep",
            "showHUDGLoad", "showHUDManeuverBar",
            "enableFlapAngleBar", "showSpeedBar", "showAttitudeGauge",
            "hudMach", "disableHUDSpeedLabel", "disableHUDHeightLabel", "disableHUDSEPLabel",
            "alwaysShowRadarAltitude", "attitudeIndicatorInertialMode"
    };

    private MiniHudLegacyMigrator() {
    }

    public static MiniHudDocument fromLegacy(Map<String, String> legacy) {
        // 旧配置里 disableHUD*Label 存的是“是否显示标签”：false=隐藏，true=显示
        boolean showSpeedLabel = bool(legacy, "disableHUDSpeedLabel", false);
        boolean showAltLabel = bool(legacy, "disableHUDHeightLabel", false);
        boolean showSepLabel = bool(legacy, "disableHUDSEPLabel", false);
        // 旧版 rightDraw：三个标签全关才收窄到 3.5 行高
        boolean labelsVisible = showSpeedLabel || showAltLabel || showSepLabel;
        MiniHudDocument d = MiniHudDefaultLayout.create(labelsVisible);

        boolean mach = bool(legacy, "hudMach", true);
        boolean showSpeed = bool(legacy, "showHUDSpeed", true);
        boolean showAoa = bool(legacy, "showHUDAoA", true);
        boolean showAlt = bool(legacy, "showHUDAltitude", true);
        boolean showEnergy = bool(legacy, "showHUDEnergy", true);
        boolean showFlapsText = bool(legacy, "showHUDFlaps", true);
        boolean showBrk = bool(legacy, "showHUDAirbrake", true);
        boolean showGear = bool(legacy, "showHUDGear", true);
        boolean showSep = bool(legacy, "showHUDSep", true);
        boolean showG = bool(legacy, "showHUDGLoad", true);
        boolean showManeuverBar = bool(legacy, "showHUDManeuverBar", true);
        boolean showAttitude = bool(legacy, "showAttitudeGauge", true);
        boolean showSpeedBar = bool(legacy, "showSpeedBar", true);
        boolean radarAlt = bool(legacy, "alwaysShowRadarAltitude", false);
        boolean inertial = bool(legacy, "attitudeIndicatorInertialMode", false);

        MiniHudWidget speed = d.find("speed.text");
        if (speed != null) {
            speed.visible = showSpeed;
            speed.set(MiniHudProps.SOURCE, mach ? MiniHudProps.SRC_MACH : MiniHudProps.SRC_IAS);
            speed.set(MiniHudProps.LABEL, mach ? "M" : (showSpeedLabel ? "SPD" : ""));
            speed.set(MiniHudProps.TEMPLATE, mach ? "M 0.85"
                    : (speed.get(MiniHudProps.LABEL, "SPD") + "  360"));
            speed.set(MiniHudProps.PREVIEW, mach ? "M 0.85"
                    : (speed.get(MiniHudProps.LABEL, "SPD") + "  360"));
        }
        MiniHudWidget aoaText = d.find("aoa.text");
        if (aoaText != null)
            aoaText.visible = showAoa;
        MiniHudWidget aoaBar = d.find("aoa.bar");
        if (aoaBar != null)
            aoaBar.visible = showAoa;

        MiniHudWidget flapBar = d.find("flap.bar");
        if (flapBar != null)
            flapBar.visible = bool(legacy, "enableFlapAngleBar", true);
        MiniHudWidget flapText = d.find("flap.text");
        if (flapText != null)
            flapText.visible = bool(legacy, "enableFlapAngleBar", true);

        MiniHudWidget alt = d.find("alt.text");
        if (alt != null) {
            alt.visible = showAlt;
            alt.set(MiniHudProps.LABEL, showAltLabel ? "ALT" : "");
            alt.set(MiniHudProps.ALT_SOURCE, radarAlt ? MiniHudProps.ALT_RADAR : MiniHudProps.ALT_AUTO);
            String prefix = alt.get(MiniHudProps.LABEL, "ALT");
            alt.set(MiniHudProps.TEMPLATE,
                    radarAlt ? prefix + "R 1024" : prefix + "  1024");
            alt.set(MiniHudProps.PREVIEW,
                    radarAlt ? prefix + "R 1024" : prefix + "  1024");
        }
        MiniHudWidget energy = d.find("energy.text");
        if (energy != null)
            energy.visible = showEnergy;

        MiniHudWidget mechFlap = d.find("mech.flap.text");
        if (mechFlap != null)
            mechFlap.visible = showFlapsText;
        MiniHudWidget mechBrk = d.find("mech.brk.text");
        if (mechBrk != null)
            mechBrk.visible = showBrk;
        MiniHudWidget mechGear = d.find("mech.gear.text");
        if (mechGear != null)
            mechGear.visible = showGear;

        MiniHudWidget sep = d.find("sep.text");
        if (sep != null) {
            sep.visible = showSep;
            sep.set(MiniHudProps.LABEL, showSepLabel ? "SEP" : "");
            sep.set(MiniHudProps.TEMPLATE,
                    sep.get(MiniHudProps.LABEL, "SEP") + "↑30  ");
            sep.set(MiniHudProps.PREVIEW,
                    sep.get(MiniHudProps.LABEL, "SEP") + "↑30  ");
        }

        MiniHudWidget g = d.find("g.text");
        if (g != null)
            g.visible = showG;
        MiniHudWidget maneuver = d.find("maneuver.bar");
        if (maneuver != null)
            maneuver.visible = showManeuverBar;

        String mode = inertial ? MiniHudProps.MODE_INERTIAL : MiniHudProps.MODE_BODY;
        MiniHudWidget attitude = d.find("attitude");
        if (attitude != null) {
            attitude.visible = showAttitude;
            attitude.set(MiniHudProps.COORDINATE_MODE, mode);
        }
        MiniHudWidget compass = d.find("compass");
        if (compass != null) {
            compass.visible = !showAttitude;
            compass.set(MiniHudProps.COORDINATE_MODE, mode);
        }

        MiniHudWidget speedBar = d.find("speed.bar");
        if (speedBar != null)
            speedBar.visible = showSpeedBar;
        MiniHudWidget throttle = d.find("throttle.bar");
        if (throttle != null)
            throttle.visible = !showSpeedBar;

        d.validate();
        return d;
    }


    /**
     * 相对父子布局（V1，未发布）→ 自由画布 V2。
     * 只提取旧默认链的显示偏好/可见性/标签，几何按当前旧版标签状态重建；
     * V1 里用户自定义的绝对位置/额外 text.row 不保留（该实现未发布）。
     */
    public static MiniHudDocument fromV1(String text) {
        if (text == null || !text.startsWith("V1;"))
            return null;
        Map<String, String[]> entries = new java.util.LinkedHashMap<>();
        String[] records = text.split(";", -1);
        for (int i = 1; i < records.length; i++) {
            if (records[i].isEmpty())
                continue;
            String[] f = records[i].split("\\|", -1);
            if (f.length != 12)
                continue;
            String id = MiniHudTextCodec.decode(f[0]);
            entries.put(id, f);
        }
        if (entries.isEmpty())
            return null;

        Map<String, String> legacy = new java.util.HashMap<>();
        String[] akb = entry(entries, "row.akb");
        if (akb != null) {
            put(legacy, "hudMach", "getMach".equals(prop(akb, "speedField", "getMach")));
            put(legacy, "disableHUDSpeedLabel", !isEmpty(prop(akb, "speedLabel", "SPD")));
            put(legacy, "showHUDSpeed", boolProp(akb, "showSpeed", true));
            put(legacy, "showHUDAoA", boolProp(akb, "showAoa", true));
        }
        String[] energy = entry(entries, "row.energy");
        if (energy != null) {
            put(legacy, "disableHUDHeightLabel", !isEmpty(prop(energy, "altLabel", "ALT")));
            put(legacy, "alwaysShowRadarAltitude", "RADAR".equals(prop(energy, "altSource", "AUTO")));
            put(legacy, "showHUDAltitude", boolProp(energy, "showAltitude", true));
            put(legacy, "showHUDEnergy", boolProp(energy, "showEnergy", true));
        }
        String[] mech = entry(entries, "row.mechanization");
        if (mech != null) {
            put(legacy, "showHUDFlaps", boolProp(mech, "flapText", true));
            put(legacy, "showHUDAirbrake", boolProp(mech, "airbrakeText", true));
            put(legacy, "showHUDGear", boolProp(mech, "gearText", true));
        }
        String[] sep = entry(entries, "row.sep");
        if (sep != null) {
            put(legacy, "disableHUDSEPLabel", !isEmpty(prop(sep, "label", "SEP")));
            put(legacy, "showHUDSep", visible(sep));
        }
        String[] maneuver = entry(entries, "row.maneuver");
        if (maneuver != null) {
            put(legacy, "showHUDGLoad", boolProp(maneuver, "showGLoad", true));
            put(legacy, "showHUDManeuverBar", boolProp(maneuver, "showBar", true));
        }
        String[] flapBar = entry(entries, "flap.bar");
        if (flapBar != null)
            put(legacy, "enableFlapAngleBar", visible(flapBar));
        String[] speedBar = entry(entries, "bar.speed");
        if (speedBar != null)
            put(legacy, "showSpeedBar", visible(speedBar));
        String[] attitude = entry(entries, "gauge.attitude");
        if (attitude != null)
            put(legacy, "showAttitudeGauge", visible(attitude));
        String[] compass = entry(entries, "gauge.compass");
        if (attitude != null) {
            put(legacy, "attitudeIndicatorInertialMode",
                    "INERTIAL".equals(prop(attitude, "coordinateMode", "BODY")));
        } else if (compass != null) {
            put(legacy, "attitudeIndicatorInertialMode",
                    "INERTIAL".equals(prop(compass, "coordinateMode", "BODY")));
        }
        return fromLegacy(legacy);
    }

    private static String[] entry(Map<String, String[]> entries, String id) {
        return entries.get(id);
    }

    private static String prop(String[] fields, String key, String def) {
        if (fields.length < 12)
            return def;
        String props = fields[11];
        if (props == null || props.isEmpty())
            return def;
        for (String kv : props.split(",", -1)) {
            int eq = kv.indexOf('=');
            if (eq > 0 && key.equals(kv.substring(0, eq)))
                return MiniHudTextCodec.decode(kv.substring(eq + 1));
        }
        return def;
    }

    private static boolean boolProp(String[] fields, String key, boolean def) {
        String v = prop(fields, key, null);
        if (v == null || v.isEmpty())
            return def;
        return "1".equals(v) || "true".equalsIgnoreCase(v);
    }

    private static boolean visible(String[] fields) {
        return fields.length > 8 && "1".equals(fields[8]);
    }

    private static boolean isEmpty(String s) {
        return s == null || s.isEmpty();
    }

    private static void put(Map<String, String> map, String key, boolean value) {
        map.put(key, value ? "true" : "false");
    }

    private static boolean bool(Map<String, String> map, String key, boolean def) {
        String v = map.get(key);
        if (v == null || v.isEmpty())
            return def;
        return "true".equalsIgnoreCase(v) || "1".equals(v);
    }
}
