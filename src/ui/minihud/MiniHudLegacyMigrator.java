package ui.minihud;

import java.util.Map;

/**
 * 旧 MiniHUD 逐项开关 → 新布局文档 的一次性映射。
 * 缺失键按旧代码默认值处理（showHUD* 默认 true；hudMach/雷达高度/随体默认 false）。
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

    public static MiniHudLayoutDoc fromLegacy(Map<String, String> legacy) {
        MiniHudLayoutDoc d = MiniHudDefaultLayout.create();
        MiniHudEntry akb = d.find("row.akb");
        MiniHudEntry flap = d.find("flap.bar");
        MiniHudEntry energy = d.find("row.energy");
        MiniHudEntry mech = d.find("row.mechanization");
        MiniHudEntry sep = d.find("row.sep");
        MiniHudEntry maneuver = d.find("row.maneuver");
        MiniHudEntry attitude = d.find("gauge.attitude");
        MiniHudEntry compass = d.find("gauge.compass");
        MiniHudEntry speedBar = d.find("bar.speed");
        MiniHudEntry throttle = d.find("bar.throttle");

        boolean mach = bool(legacy, "hudMach", false);
        if (akb != null) {
            akb.set(MiniHudProps.SPEED_FIELD, mach ? MiniHudProps.FIELD_MACH : MiniHudProps.FIELD_IAS);
            akb.set(MiniHudProps.SPEED_LABEL,
                    bool(legacy, "disableHUDSpeedLabel", false) ? "" : "SPD");
            akb.set(MiniHudProps.SHOW_SPEED, b(bool(legacy, "showHUDSpeed", true)));
            akb.set(MiniHudProps.SHOW_AOA, b(bool(legacy, "showHUDAoA", true)));
        }
        if (flap != null) {
            flap.visible = bool(legacy, "enableFlapAngleBar", true);
        }
        if (energy != null) {
            energy.set(MiniHudProps.ALT_LABEL, bool(legacy, "disableHUDHeightLabel", false) ? "" : "ALT");
            energy.set(MiniHudProps.ALT_SOURCE,
                    bool(legacy, "alwaysShowRadarAltitude", false) ? MiniHudProps.ALT_RADAR : MiniHudProps.ALT_AUTO);
            energy.set(MiniHudProps.SHOW_ALTITUDE, b(bool(legacy, "showHUDAltitude", true)));
            energy.set(MiniHudProps.SHOW_ENERGY, b(bool(legacy, "showHUDEnergy", true)));
        }
        if (mech != null) {
            mech.set(MiniHudProps.FLAP_TEXT, b(bool(legacy, "showHUDFlaps", true)));
            mech.set(MiniHudProps.AIRBRAKE_TEXT, b(bool(legacy, "showHUDAirbrake", true)));
            mech.set(MiniHudProps.GEAR_TEXT, b(bool(legacy, "showHUDGear", true)));
        }
        if (sep != null) {
            sep.visible = bool(legacy, "showHUDSep", true);
            sep.set(MiniHudProps.SEP_LABEL, bool(legacy, "disableHUDSEPLabel", false) ? "" : "SEP");
        }
        if (maneuver != null) {
            maneuver.set(MiniHudProps.SHOW_GLOAD, b(bool(legacy, "showHUDGLoad", true)));
            maneuver.set(MiniHudProps.SHOW_BAR, b(bool(legacy, "showHUDManeuverBar", true)));
        }
        boolean inertial = bool(legacy, "attitudeIndicatorInertialMode", false);
        String mode = inertial ? MiniHudProps.MODE_INERTIAL : MiniHudProps.MODE_BODY;
        if (attitude != null) {
            attitude.visible = bool(legacy, "showAttitudeGauge", true);
            attitude.set(MiniHudProps.COORDINATE_MODE, mode);
        }
        if (compass != null) {
            compass.visible = !bool(legacy, "showAttitudeGauge", true);
            compass.set(MiniHudProps.COORDINATE_MODE, mode);
        }
        boolean showSpeedBar = bool(legacy, "showSpeedBar", true);
        if (speedBar != null)
            speedBar.visible = showSpeedBar;
        if (throttle != null)
            throttle.visible = !showSpeedBar;
        return d;
    }

    private static boolean bool(Map<String, String> map, String key, boolean def) {
        String v = map.get(key);
        if (v == null || v.isEmpty())
            return def;
        return "true".equalsIgnoreCase(v) || "1".equals(v);
    }

    private static String b(boolean v) {
        return v ? "1" : "0";
    }
}
