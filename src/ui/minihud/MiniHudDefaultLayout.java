package ui.minihud;

/**
 * 出厂默认布局：由原子组件拼成，坐标单位 = 基准行高。
 * 与旧版出厂状态一致：三个文字标签全关、右列 3.5 行高、数据区不显示襟翼文字；
 * 演示值也复刻旧版预览（α 20、0/100、BRK/GEA）。
 */
public final class MiniHudDefaultLayout {

    private MiniHudDefaultLayout() {
    }

    public static MiniHudDocument create() {
        // 出厂默认与旧版一致：三个文字标签全关，右列按 3.5 行高布局
        return create(false);
    }

    /**
     * @param labelsVisible 旧版三个标签至少一个可见时，右列按 5.5 行高布局；
     *                      三个全关时按旧版的 3.5 行高布局。
     */
    public static MiniHudDocument create(boolean labelsVisible) {
        MiniHudDocument d = new MiniHudDocument();

        MiniHudGroup gSpeed = group(d, "g.speed", "速度 / AoA");
        MiniHudGroup gAlt = group(d, "g.alt", "高度 / 能量");
        MiniHudGroup gMech = group(d, "g.mech", "机械状态");
        MiniHudGroup gSep = group(d, "g.sep", "SEP");
        MiniHudGroup gManeuver = group(d, "g.maneuver", "G / 机动");
        MiniHudGroup gBars = group(d, "g.bars", "条");
        MiniHudGroup gInstruments = group(d, "g.instruments", "仪表");

        MiniHudWidget speed = widget(d, "speed.text", MiniHudProps.TYPE_TEXT, 2.085, 3.50, gSpeed.id, 30);
        speed.set(MiniHudProps.SOURCE, MiniHudProps.SRC_MACH);
        speed.set(MiniHudProps.LABEL, "M");
        speed.set(MiniHudProps.TEMPLATE, "M 0.85");
        speed.set(MiniHudProps.PREVIEW, "M 0.85");
        speed.set(MiniHudProps.COLOR, MiniHudProps.COLOR_AUTO);

        MiniHudWidget aoaBar = widget(d, "aoa.bar", MiniHudProps.TYPE_BAR, 2.73, 4.58, gSpeed.id, 20);
        aoaBar.set(MiniHudProps.BAR_SOURCE, MiniHudProps.BAR_AOA);
        aoaBar.set(MiniHudProps.LENGTH_UNITS, "4.8332");

        MiniHudWidget aoaText = widget(d, "aoa.text", MiniHudProps.TYPE_TEXT, 7.585, 3.50, gSpeed.id, 21);
        aoaText.set(MiniHudProps.SOURCE, MiniHudProps.SRC_AOA);
        aoaText.set(MiniHudProps.LABEL, "α");
        aoaText.set(MiniHudProps.FONT, MiniHudProps.FONT_SMALL);
        aoaText.set(MiniHudProps.BASELINE, MiniHudProps.BASELINE_MAIN);
        aoaText.set(MiniHudProps.TEMPLATE, "α 20");
        aoaText.set(MiniHudProps.PREVIEW, "α 20");
        aoaText.set(MiniHudProps.COLOR, MiniHudProps.COLOR_AOA);

        MiniHudWidget flapText = widget(d, "flap.text", MiniHudProps.TYPE_TEXT, 2.085, 2.36, gSpeed.id, 10);
        flapText.set(MiniHudProps.SOURCE, MiniHudProps.SRC_FLAP_ANGLE);
        flapText.set(MiniHudProps.FONT, MiniHudProps.FONT_SMALL);
        flapText.set(MiniHudProps.ALIGN, MiniHudProps.ALIGN_CENTER);
        flapText.set(MiniHudProps.WIDTH_UNITS, "6.0");
        flapText.set(MiniHudProps.TEMPLATE, "  0/100");
        flapText.set(MiniHudProps.PREVIEW, "  0/100");

        MiniHudWidget flapBar = widget(d, "flap.bar", MiniHudProps.TYPE_BAR, 2.085, 3.20, gSpeed.id, 11);
        flapBar.set(MiniHudProps.BAR_SOURCE, MiniHudProps.BAR_FLAP_ANGLE);
        flapBar.set(MiniHudProps.LENGTH_UNITS, "6.0");

        MiniHudWidget alt = widget(d, "alt.text", MiniHudProps.TYPE_TEXT, 2.085, 4.58, gAlt.id, 30);
        alt.set(MiniHudProps.SOURCE, MiniHudProps.SRC_ALTITUDE);
        alt.set(MiniHudProps.LABEL, "ALT");
        alt.set(MiniHudProps.ALT_SOURCE, MiniHudProps.ALT_AUTO);
        alt.set(MiniHudProps.TEMPLATE, "ALT  1024");
        alt.set(MiniHudProps.PREVIEW, "ALT  1024");
        alt.set(MiniHudProps.COLOR, MiniHudProps.COLOR_AUTO);

        MiniHudWidget energy = widget(d, "energy.text", MiniHudProps.TYPE_TEXT, 7.585, 4.58, gAlt.id, 31);
        energy.set(MiniHudProps.SOURCE, MiniHudProps.SRC_ENERGY);
        energy.set(MiniHudProps.LABEL, "E");
        energy.set(MiniHudProps.FONT, MiniHudProps.FONT_SMALL);
        energy.set(MiniHudProps.BASELINE, MiniHudProps.BASELINE_MAIN);
        energy.set(MiniHudProps.TEMPLATE, "E114514");
        energy.set(MiniHudProps.PREVIEW, "E114514");

        MiniHudWidget mechFlap = widget(d, "mech.flap.text", MiniHudProps.TYPE_TEXT, 2.085, 5.66, gMech.id, 30);
        mechFlap.set(MiniHudProps.SOURCE, MiniHudProps.SRC_FLAPS);
        mechFlap.set(MiniHudProps.TEMPLATE, "F100 ");
        // 旧版默认预览在襟翼条开启时不显示数据区襟翼文字
        mechFlap.set(MiniHudProps.PREVIEW, "");
        mechFlap.set(MiniHudProps.COLOR, MiniHudProps.COLOR_AUTO);

        MiniHudWidget mechBrk = widget(d, "mech.brk.text", MiniHudProps.TYPE_TEXT, 4.68, 5.66, gMech.id, 30);
        mechBrk.set(MiniHudProps.SOURCE, MiniHudProps.SRC_AIRBRAKE);
        mechBrk.set(MiniHudProps.TEMPLATE, "BRK ");
        mechBrk.set(MiniHudProps.PREVIEW, "BRK ");
        mechBrk.set(MiniHudProps.COLOR, MiniHudProps.COLOR_AUTO);

        MiniHudWidget mechGear = widget(d, "mech.gear.text", MiniHudProps.TYPE_TEXT, 7.07, 5.66, gMech.id, 30);
        mechGear.set(MiniHudProps.SOURCE, MiniHudProps.SRC_GEAR);
        mechGear.set(MiniHudProps.TEMPLATE, "GEA");
        mechGear.set(MiniHudProps.PREVIEW, "GEA");
        mechGear.set(MiniHudProps.COLOR, MiniHudProps.COLOR_AUTO);

        MiniHudWidget sep = widget(d, "sep.text", MiniHudProps.TYPE_TEXT, 2.085, 6.74, gSep.id, 30);
        sep.set(MiniHudProps.SOURCE, MiniHudProps.SRC_SEP);
        sep.set(MiniHudProps.LABEL, "SEP");
        sep.set(MiniHudProps.ARROW, "1");
        sep.set(MiniHudProps.TEMPLATE, "SEP↑30  ");
        sep.set(MiniHudProps.PREVIEW, "SEP↑30  ");

        MiniHudWidget g = widget(d, "g.text", MiniHudProps.TYPE_TEXT, 2.085, 7.82, gManeuver.id, 30);
        g.set(MiniHudProps.SOURCE, MiniHudProps.SRC_GLOAD);
        g.set(MiniHudProps.LABEL, "G");
        g.set(MiniHudProps.TEMPLATE, "G  2.0");
        g.set(MiniHudProps.PREVIEW, "G  2.0");

        MiniHudWidget maneuver = widget(d, "maneuver.bar", MiniHudProps.TYPE_BAR, 2.085, 7.82, gManeuver.id, 20);
        maneuver.set(MiniHudProps.BAR_SOURCE, MiniHudProps.BAR_MANEUVER);
        maneuver.set(MiniHudProps.LENGTH_UNITS, "5.5");

        MiniHudWidget speedBar = widget(d, "speed.bar", MiniHudProps.TYPE_BAR, 1.55, 3.32, gBars.id, 20);
        speedBar.set(MiniHudProps.BAR_SOURCE, MiniHudProps.BAR_SPEED_RATIO);
        speedBar.set(MiniHudProps.LENGTH_UNITS, "5.5");

        MiniHudWidget throttle = widget(d, "throttle.bar", MiniHudProps.TYPE_BAR, 0.545, 4.03, gBars.id, 21);
        throttle.set(MiniHudProps.BAR_SOURCE, MiniHudProps.BAR_THROTTLE);
        throttle.set(MiniHudProps.LENGTH_UNITS, "4.8");
        throttle.visible = false;

        MiniHudWidget attitude = widget(d, "attitude", MiniHudProps.TYPE_ATTITUDE, 7.975, 7.16,
                gInstruments.id, 40);
        attitude.set(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY);

        MiniHudWidget compass = widget(d, "compass", MiniHudProps.TYPE_COMPASS, 7.63, 6.74,
                gInstruments.id, 41);
        compass.set(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY);
        compass.visible = false;

        if (!labelsVisible) {
            // 旧版三个标签全关时 rightDraw 为 3.5 行高：右列/AoA 条/机动条同步收窄
            if (!MiniHudProps.SRC_MACH.equals(speed.get(MiniHudProps.SOURCE, ""))) {
                speed.set(MiniHudProps.LABEL, "");
                speed.set(MiniHudProps.TEMPLATE, "  360");
                speed.set(MiniHudProps.PREVIEW, "  360");
            }
            alt.set(MiniHudProps.LABEL, "");
            alt.set(MiniHudProps.TEMPLATE, "  1024");
            alt.set(MiniHudProps.PREVIEW, "  1024");
            sep.set(MiniHudProps.LABEL, "");
            sep.set(MiniHudProps.TEMPLATE, "↑30  ");
            sep.set(MiniHudProps.PREVIEW, "↑30  ");
            aoaText.x = 5.585;
            energy.x = 5.585;
            aoaBar.set(MiniHudProps.LENGTH_UNITS, "2.8332");
            maneuver.set(MiniHudProps.LENGTH_UNITS, "3.5");
        }

        d.validate();
        return d;
    }

    private static MiniHudGroup group(MiniHudDocument d, String id, String name) {
        MiniHudGroup g = new MiniHudGroup(id, name);
        d.groups.add(g);
        return g;
    }

    private static MiniHudWidget widget(MiniHudDocument d, String id, String type, double x, double y,
            String groupId, int z) {
        MiniHudWidget w = new MiniHudWidget(id, type);
        w.x = x;
        w.y = y;
        w.groupId = groupId;
        w.z = z;
        d.widgets.add(w);
        return w;
    }
}
