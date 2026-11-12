package ui.minihud;

import ui.layout.Anchor;

/**
 * 出厂默认布局：复刻拆分前 MiniHUD 的硬编码几何（MiniHUDOverlay.initModernLayout）。
 * 坐标单位 = hudFontSize；改动这里等于改变所有新用户的默认 HUD。
 */
public final class MiniHudDefaultLayout {

    private MiniHudDefaultLayout() {
    }

    public static MiniHudLayoutDoc create() {
        MiniHudLayoutDoc d = new MiniHudLayoutDoc();

        MiniHudEntry akb = entry(d, "row.akb", "legacy.akb", null, 2.1, 3.5,
                Anchor.TOP_LEFT, Anchor.TOP_LEFT);
        akb.set(MiniHudProps.SPEED_FIELD, MiniHudProps.FIELD_MACH);
        akb.set(MiniHudProps.SPEED_LABEL, "SPD");
        akb.set(MiniHudProps.SHOW_SPEED, "1");
        akb.set(MiniHudProps.SHOW_AOA, "1");

        MiniHudEntry flap = entry(d, "flap.bar", "gauge.flapBar", "row.akb", 0, -0.1,
                Anchor.TOP_LEFT, Anchor.BOTTOM_LEFT);
        flap.visible = true;

        MiniHudEntry energy = entry(d, "row.energy", "legacy.energy", "row.akb", 0, 0.1,
                Anchor.BOTTOM_LEFT, Anchor.TOP_LEFT);
        energy.set(MiniHudProps.ALT_SOURCE, MiniHudProps.ALT_AUTO);
        energy.set(MiniHudProps.ALT_LABEL, "ALT");
        energy.set(MiniHudProps.ENERGY_LABEL, "E");
        energy.set(MiniHudProps.SHOW_ALTITUDE, "1");
        energy.set(MiniHudProps.SHOW_ENERGY, "1");

        MiniHudEntry mech = entry(d, "row.mechanization", "legacy.mechanization", "row.energy", 0, 0.1,
                Anchor.BOTTOM_LEFT, Anchor.TOP_LEFT);
        mech.set(MiniHudProps.FLAP_TEXT, "1");
        mech.set(MiniHudProps.AIRBRAKE_TEXT, "1");
        mech.set(MiniHudProps.GEAR_TEXT, "1");

        MiniHudEntry sep = entry(d, "row.sep", "legacy.sep", "row.mechanization", 0, 0.1,
                Anchor.BOTTOM_LEFT, Anchor.TOP_LEFT);
        sep.set(MiniHudProps.SEP_LABEL, "SEP");
        sep.set(MiniHudProps.SEP_ARROW, "1");

        MiniHudEntry maneuver = entry(d, "row.maneuver", "legacy.maneuver", "row.sep", 0, 0.1,
                Anchor.BOTTOM_LEFT, Anchor.TOP_LEFT);
        maneuver.set(MiniHudProps.G_LABEL, "G");
        maneuver.set(MiniHudProps.SHOW_GLOAD, "1");
        maneuver.set(MiniHudProps.SHOW_BAR, "1");

        MiniHudEntry attitude = entry(d, "gauge.attitude", "gauge.attitude", "row.mechanization", 0, 0.5,
                Anchor.BOTTOM_RIGHT, Anchor.TOP_RIGHT);
        attitude.set(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY);
        attitude.visible = true;

        MiniHudEntry compass = entry(d, "gauge.compass", "gauge.compass", "row.mechanization", 0, 0.1,
                Anchor.BOTTOM_RIGHT, Anchor.TOP_RIGHT);
        compass.set(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY);
        compass.visible = false;

        MiniHudEntry speedBar = entry(d, "bar.speed", "gauge.speedBar", "row.maneuver", -0.3, 0,
                Anchor.BOTTOM_LEFT, Anchor.BOTTOM_RIGHT);
        speedBar.visible = true;

        MiniHudEntry throttle = entry(d, "bar.throttle", "gauge.throttle", "row.maneuver", -0.3, 0,
                Anchor.BOTTOM_LEFT, Anchor.BOTTOM_RIGHT);
        throttle.visible = false;

        return d;
    }

    private static MiniHudEntry entry(MiniHudLayoutDoc d, String id, String type, String parent,
            double x, double y, Anchor pa, Anchor sa) {
        MiniHudEntry e = new MiniHudEntry(id, type);
        e.parentId = parent;
        e.unitX = x;
        e.unitY = y;
        e.parentAnchor = pa;
        e.selfAnchor = sa;
        d.entries.add(e);
        return e;
    }
}
