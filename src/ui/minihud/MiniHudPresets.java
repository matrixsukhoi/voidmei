package ui.minihud;

import prog.config.MiniHudFieldCatalog;

/**
 * 编辑器“预设”宏：一次性创建一组普通原子组件并自动成组。
 * 预设不是组件类型；插入后与手摆的组件完全等价，可解组、单删、单改。
 */
public final class MiniHudPresets {

    public enum Preset {
        SPEED_AOA("速度 / AoA"),
        ALT_ENERGY("高度 / 能量"),
        MECHANIZATION("机械状态"),
        SEP("SEP"),
        MANEUVER("G / 机动"),
        INSTRUMENTS("姿态仪"),
        FLAP_BAR("襟翼角条"),
        SPEED_BAR("速度条"),
        THROTTLE_BAR("油门条");

        public final String label;

        Preset(String label) {
            this.label = label;
        }
    }

    private MiniHudPresets() {
    }

    /** 在画布坐标 (x,y) 处插入预设，返回新建的组。 */
    public static MiniHudGroup insert(MiniHudDocument doc, Preset preset, double x, double y) {
        MiniHudDocument template = MiniHudDefaultLayout.create();
        String[] ids = idsFor(preset);
        MiniHudGroup group = new MiniHudGroup(doc.nextGroupId(), preset.label);
        doc.groups.add(group);

        double minX = Double.MAX_VALUE;
        double minY = Double.MAX_VALUE;
        for (String id : ids) {
            MiniHudWidget t = template.find(id);
            if (t != null) {
                minX = Math.min(minX, t.x);
                minY = Math.min(minY, t.y);
            }
        }
        if (minX == Double.MAX_VALUE) {
            minX = 0;
            minY = 0;
        }
        int baseZ = maxZ(doc) + 1;
        int z = 0;
        for (String id : ids) {
            MiniHudWidget t = template.find(id);
            if (t == null)
                continue;
            MiniHudWidget w = t.copy(doc.nextWidgetId());
            w.x = x + (t.x - minX);
            w.y = y + (t.y - minY);
            w.groupId = group.id;
            w.visible = true;
            w.z = baseZ + z++;
            doc.widgets.add(w);
        }
        doc.validate();
        return group;
    }

    public static MiniHudWidget addStaticText(MiniHudDocument doc, String text, double x, double y) {
        MiniHudWidget w = newWidget(doc, MiniHudProps.TYPE_TEXT, x, y);
        w.set(MiniHudProps.SOURCE, MiniHudProps.SRC_STATIC);
        w.set(MiniHudProps.TEXT, text != null ? text : "");
        w.set(MiniHudProps.TEMPLATE, text != null && !text.isEmpty() ? text : "TEXT");
        w.set(MiniHudProps.PREVIEW, text != null ? text : "");
        return w;
    }

    public static MiniHudWidget addTextField(MiniHudDocument doc, String field, double x, double y,
            MiniHudFieldCatalog catalog) {
        MiniHudWidget w = newWidget(doc, MiniHudProps.TYPE_TEXT, x, y);
        w.set(MiniHudProps.SOURCE, MiniHudProps.SRC_FIELD);
        w.set(MiniHudProps.FIELD, field != null ? field : "");
        MiniHudFieldCatalog.FieldDef def = catalog != null && field != null ? catalog.get(field) : null;
        String label = def != null && def.label != null ? def.label : "";
        String unit = def != null && def.unit != null ? def.unit : "";
        String sample = def != null && def.previewValue != null && !def.previewValue.isEmpty()
                ? def.previewValue
                : "8888";
        int precision = def != null ? def.precision : 2;
        w.set(MiniHudProps.LABEL, label);
        w.set(MiniHudProps.UNIT, unit);
        w.set(MiniHudProps.PRECISION, Integer.toString(precision));
        w.set(MiniHudProps.TEMPLATE, label + sample + unit);
        w.set(MiniHudProps.PREVIEW, label + sample + unit);
        return w;
    }

    public static MiniHudWidget addBar(MiniHudDocument doc, String barSource, double x, double y) {
        MiniHudWidget w = newWidget(doc, MiniHudProps.TYPE_BAR, x, y);
        w.set(MiniHudProps.BAR_SOURCE, barSource);
        switch (barSource) {
            case MiniHudProps.BAR_FLAP_ANGLE:
                w.set(MiniHudProps.LENGTH_UNITS, "6.0");
                break;
            case MiniHudProps.BAR_SPEED_RATIO:
            case MiniHudProps.BAR_MANEUVER:
                w.set(MiniHudProps.LENGTH_UNITS, "5.5");
                break;
            case MiniHudProps.BAR_THROTTLE:
                w.set(MiniHudProps.LENGTH_UNITS, "4.8");
                break;
            case MiniHudProps.BAR_AOA:
                w.set(MiniHudProps.LENGTH_UNITS, "4.8332");
                break;
            default:
                w.set(MiniHudProps.MIN, "0");
                w.set(MiniHudProps.MAX, "1");
                w.set(MiniHudProps.LENGTH_UNITS, "4");
                break;
        }
        return w;
    }

    public static MiniHudWidget addCompass(MiniHudDocument doc, double x, double y) {
        MiniHudWidget w = newWidget(doc, MiniHudProps.TYPE_COMPASS, x, y);
        w.set(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY);
        return w;
    }

    public static MiniHudWidget addAttitude(MiniHudDocument doc, double x, double y) {
        MiniHudWidget w = newWidget(doc, MiniHudProps.TYPE_ATTITUDE, x, y);
        w.set(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY);
        return w;
    }

    public static MiniHudWidget addPanel(MiniHudDocument doc, double x, double y) {
        MiniHudWidget w = newWidget(doc, MiniHudProps.TYPE_PANEL, x, y);
        w.set(MiniHudProps.WIDTH_UNITS, "6");
        w.set(MiniHudProps.HEIGHT_UNITS, "3");
        w.set(MiniHudProps.RADIUS_UNITS, "0.25");
        w.set(MiniHudProps.PANEL_COLOR, "#60202020");
        return w;
    }

    /** 机场距离面板是普通画布组件，可多实例。 */
    public static MiniHudWidget addAirfield(MiniHudDocument doc, double x, double y) {
        return newWidget(doc, MiniHudProps.TYPE_AIRFIELD, x, y);
    }

    private static MiniHudWidget newWidget(MiniHudDocument doc, String type, double x, double y) {
        MiniHudWidget w = new MiniHudWidget(doc.nextWidgetId(), type);
        MiniHudWidgets.applyDefaults(w);
        w.x = x;
        w.y = y;
        w.z = maxZ(doc) + 1;
        doc.widgets.add(w);
        return w;
    }

    private static int maxZ(MiniHudDocument doc) {
        int max = 0;
        for (MiniHudWidget w : doc.widgets)
            max = Math.max(max, w.z);
        return max;
    }

    private static String[] idsFor(Preset preset) {
        switch (preset) {
            case SPEED_AOA:
                return new String[] { "speed.text", "aoa.bar", "aoa.text" };
            case ALT_ENERGY:
                return new String[] { "alt.text", "energy.text" };
            case MECHANIZATION:
                return new String[] { "mech.flap.text", "mech.brk.text", "mech.gear.text" };
            case SEP:
                return new String[] { "sep.text" };
            case MANEUVER:
                return new String[] { "g.text", "maneuver.bar" };
            case INSTRUMENTS:
                return new String[] { "attitude" };
            case FLAP_BAR:
                return new String[] { "flap.text", "flap.bar" };
            case SPEED_BAR:
                return new String[] { "speed.bar" };
            case THROTTLE_BAR:
                return new String[] { "throttle.bar" };
            default:
                return new String[] {};
        }
    }
}
