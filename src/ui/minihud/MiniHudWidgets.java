package ui.minihud;

import prog.Application;
import ui.component.AttitudeIndicatorGauge;
import ui.component.CompassGauge;
import ui.component.FlapAngleBar;
import ui.component.HUDComponent;
import ui.component.LinearGauge;
import ui.component.SpeedRatioBar;
import ui.component.row.HUDAkbRow;
import ui.component.row.HUDEnergyRow;
import ui.component.row.HUDManeuverRow;
import ui.component.row.HUDMechanizationRow;
import ui.component.row.HUDSepRow;
import ui.overlay.MinimalHUDContext;

/**
 * 内置条目类型注册。新增组件只需在这里加一行 register(...)，
 * 不改 MiniHUDOverlay / 场景构建。
 */
public final class MiniHudWidgets {

    private static boolean registered = false;

    private MiniHudWidgets() {
    }

    public static synchronized void registerDefaults() {
        if (registered)
            return;
        registered = true;

        reg("text.row", "文本", "文本行", false, true, EdgeResize.BOTH, c -> new TextRowWidget(
                c.entry.id, c.ctx, c.entry, c.preview, c.source, c.catalog));

        reg("legacy.akb", "复合行", "速度 + AoA", false, true, EdgeResize.NONE, MiniHudWidgets::createAkb);
        reg("legacy.energy", "复合行", "高度 + 能量", false, true, EdgeResize.NONE,
                MiniHudWidgets::createEnergy);
        reg("legacy.mechanization", "复合行", "襟翼/减速板/起落架", false, true, EdgeResize.NONE,
                MiniHudWidgets::createMechanization);
        reg("legacy.sep", "复合行", "SEP", false, true, EdgeResize.NONE, MiniHudWidgets::createSep);
        reg("legacy.maneuver", "复合行", "G + 机动条", false, true, EdgeResize.NONE,
                MiniHudWidgets::createManeuver);

        reg("gauge.flapBar", "条与仪表", "襟翼角条", false, true, EdgeResize.WIDTH,
                MiniHudWidgets::createFlapBar);
        reg("gauge.speedBar", "条与仪表", "速度条", false, true, EdgeResize.HEIGHT,
                MiniHudWidgets::createSpeedBar);
        reg("gauge.throttle", "条与仪表", "油门条", false, true, EdgeResize.HEIGHT,
                MiniHudWidgets::createThrottle);
        reg("gauge.compass", "条与仪表", "罗盘", false, true, EdgeResize.BOTH, MiniHudWidgets::createCompass);
        reg("gauge.attitude", "条与仪表", "姿态仪", false, true, EdgeResize.BOTH,
                MiniHudWidgets::createAttitude);
    }

    private static void reg(String type, String category, String name, boolean dataField, boolean scalable,
            EdgeResize edge, HUDWidgetFactory f) {
        HUDWidgetRegistry.register(new HUDWidgetSpec(type, category, name, dataField, scalable, edge, f));
    }

    /** 新建条目时的默认属性（与出厂布局一致）。 */
    public static void applyDefaults(MiniHudEntry e) {
        switch (e.type) {
            case "legacy.akb":
                e.set(MiniHudProps.SPEED_FIELD, MiniHudProps.FIELD_MACH);
                e.set(MiniHudProps.SPEED_LABEL, "SPD");
                e.set(MiniHudProps.SHOW_SPEED, "1");
                e.set(MiniHudProps.SHOW_AOA, "1");
                break;
            case "legacy.energy":
                e.set(MiniHudProps.ALT_SOURCE, MiniHudProps.ALT_AUTO);
                e.set(MiniHudProps.ALT_LABEL, "ALT");
                e.set(MiniHudProps.ENERGY_LABEL, "E");
                e.set(MiniHudProps.SHOW_ALTITUDE, "1");
                e.set(MiniHudProps.SHOW_ENERGY, "1");
                break;
            case "legacy.mechanization":
                e.set(MiniHudProps.FLAP_TEXT, "1");
                e.set(MiniHudProps.AIRBRAKE_TEXT, "1");
                e.set(MiniHudProps.GEAR_TEXT, "1");
                break;
            case "legacy.sep":
                e.set(MiniHudProps.SEP_LABEL, "SEP");
                e.set(MiniHudProps.SEP_ARROW, "1");
                break;
            case "legacy.maneuver":
                e.set(MiniHudProps.G_LABEL, "G");
                e.set(MiniHudProps.SHOW_GLOAD, "1");
                e.set(MiniHudProps.SHOW_BAR, "1");
                break;
            case "gauge.compass":
            case "gauge.attitude":
                e.set(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY);
                break;
            default:
                break;
        }
    }

    public static HUDComponent create(WidgetBuildContext c) {
        HUDWidgetSpec spec = HUDWidgetRegistry.find(c.entry.type);
        if (spec == null)
            return new TextRowWidget(c.entry.id, c.ctx, c.entry, c.preview, c.source, c.catalog);
        return spec.factory.create(c);
    }

    /** 复合行/多槽位条目的可见性：条目可见且至少一个槽位开启。 */
    public static boolean isSelfVisible(MiniHudEntry e) {
        String t = e.type;
        if ("legacy.akb".equals(t))
            return e.getBool(MiniHudProps.SHOW_SPEED, true) || e.getBool(MiniHudProps.SHOW_AOA, true);
        if ("legacy.energy".equals(t))
            return e.getBool(MiniHudProps.SHOW_ALTITUDE, true) || e.getBool(MiniHudProps.SHOW_ENERGY, true);
        if ("legacy.mechanization".equals(t))
            return e.getBool(MiniHudProps.FLAP_TEXT, true) || e.getBool(MiniHudProps.AIRBRAKE_TEXT, true)
                    || e.getBool(MiniHudProps.GEAR_TEXT, true);
        if ("legacy.sep".equals(t))
            return true;
        if ("legacy.maneuver".equals(t))
            return e.getBool(MiniHudProps.SHOW_GLOAD, true) || e.getBool(MiniHudProps.SHOW_BAR, true);
        if ("text.row".equals(t))
            return !e.get(MiniHudProps.CELLS, "").isEmpty();
        return true;
    }

    private static HUDComponent createAkb(WidgetBuildContext c) {
        MinimalHUDContext ctx = c.ctx;
        HUDAkbRow row = new HUDAkbRow(0, ctx.drawFont, ctx.hudFontSize, ctx.drawFontSmall, ctx.rightDraw,
                ctx.lineWidth);
        row.setShowSpeed(c.entry.getBool(MiniHudProps.SHOW_SPEED, true));
        row.setShowAoa(c.entry.getBool(MiniHudProps.SHOW_AOA, true));
        row.setSpeedFormat(c.entry.get(MiniHudProps.SPEED_FIELD, MiniHudProps.FIELD_MACH),
                c.entry.get(MiniHudProps.SPEED_LABEL, "SPD"));
        row.setTemplate(speedTemplate(c.entry), String.format("α%3.0f", 20.0));
        row.setStyle(ctx.drawFont, ctx.hudFontSize, ctx.drawFontSmall, ctx.rightDraw, ctx.lineWidth,
                (int) ctx.aoaLength);
        row.update(speedTemplate(c.entry), false, String.format("α%3.0f", 20.0), 10,
                Application.colorNum, Application.colorNum);
        return row;
    }

    /** 与旧版 refreshTemplates 完全一致的测宽模板。 */
    private static String speedTemplate(MiniHudEntry e) {
        String field = e.get(MiniHudProps.SPEED_FIELD, MiniHudProps.FIELD_MACH);
        if (MiniHudProps.FIELD_MACH.equals(field))
            return String.format("M%5.2f", 0.85);
        String label = e.get(MiniHudProps.SPEED_LABEL, "SPD");
        return label + String.format("%5s", "360");
    }

    private static HUDComponent createEnergy(WidgetBuildContext c) {
        MinimalHUDContext ctx = c.ctx;
        HUDEnergyRow row = new HUDEnergyRow(0, ctx.drawFont, ctx.hudFontSize, ctx.drawFontSmall, ctx.rightDraw);
        row.setShowAltitude(c.entry.getBool(MiniHudProps.SHOW_ALTITUDE, true));
        row.setShowEnergy(c.entry.getBool(MiniHudProps.SHOW_ENERGY, true));
        String label = c.entry.get(MiniHudProps.ALT_LABEL, "ALT");
        String altSource = c.entry.get(MiniHudProps.ALT_SOURCE, MiniHudProps.ALT_AUTO);
        String energyLabel = c.entry.get(MiniHudProps.ENERGY_LABEL, "E");
        row.setAltFormat(altSource, label, energyLabel);
        String altTemplate;
        if (MiniHudProps.ALT_RADAR.equals(c.entry.get(MiniHudProps.ALT_SOURCE, MiniHudProps.ALT_AUTO))) {
            altTemplate = label + String.format("R%5s", "1024");
        } else {
            altTemplate = label + String.format("%6s", "1024");
        }
        row.setTemplate(altTemplate, energyLabel + "114514");
        row.setStyle(ctx.drawFont, ctx.hudFontSize, ctx.drawFontSmall, ctx.rightDraw);
        row.update(altTemplate, false, energyLabel + "114514");
        return row;
    }

    private static HUDComponent createMechanization(WidgetBuildContext c) {
        MinimalHUDContext ctx = c.ctx;
        HUDMechanizationRow row = new HUDMechanizationRow(0, ctx.drawFont, ctx.hudFontSize);
        row.setShowFlaps(c.entry.getBool(MiniHudProps.FLAP_TEXT, true));
        row.setShowAirbrake(c.entry.getBool(MiniHudProps.AIRBRAKE_TEXT, true));
        row.setShowGear(c.entry.getBool(MiniHudProps.GEAR_TEXT, true));
        String template = String.format("%4s", "") + "BRK" + "GEA";
        row.setTemplate(template);
        row.setStyle(ctx.drawFont, ctx.hudFontSize);
        row.update(template, false);
        return row;
    }

    private static HUDComponent createSep(WidgetBuildContext c) {
        MinimalHUDContext ctx = c.ctx;
        HUDSepRow row = new HUDSepRow(0, ctx.drawFont, ctx.hudFontSize);
        String label = c.entry.get(MiniHudProps.SEP_LABEL, "SEP");
        boolean arrow = c.entry.getBool(MiniHudProps.SEP_ARROW, true);
        row.setProps(label, arrow);
        row.setStyle(ctx.drawFont, ctx.hudFontSize);
        row.update(label + (arrow ? "↑" : "") + "30", false);
        return row;
    }

    private static HUDComponent createManeuver(WidgetBuildContext c) {
        MinimalHUDContext ctx = c.ctx;
        HUDManeuverRow row = new HUDManeuverRow(0, ctx.drawFont, ctx.hudFontSize, ctx.rightDraw, ctx.halfLine,
                ctx.lineWidth, ctx.strokeThick, ctx.strokeThin);
        row.setShowGLoad(c.entry.getBool(MiniHudProps.SHOW_GLOAD, true));
        row.setShowManeuverBar(c.entry.getBool(MiniHudProps.SHOW_BAR, true));
        String gLabel = c.entry.get(MiniHudProps.G_LABEL, "G");
        row.setGLabel(gLabel);
        String template = gLabel + String.format("%5s", "2.0");
        row.setTemplate(template);
        row.setStyle(ctx.drawFont, ctx.hudFontSize, ctx.rightDraw, ctx.halfLine, ctx.lineWidth, ctx.strokeThick,
                ctx.strokeThin);
        row.update(template, false, 0.2,
                (int) Math.round(0.2 / 0.5 * ctx.rightDraw), (int) Math.round(0.1 / 0.5 * ctx.rightDraw),
                (int) Math.round(0.2 / 0.5 * ctx.rightDraw), (int) Math.round(0.3 / 0.5 * ctx.rightDraw),
                (int) Math.round(0.4 / 0.5 * ctx.rightDraw), ctx.rightDraw);
        return row;
    }

    private static HUDComponent createFlapBar(WidgetBuildContext c) {
        MinimalHUDContext ctx = c.ctx;
        FlapAngleBar bar = new FlapAngleBar();
        double units = c.entry.getDouble(MiniHudProps.LENGTH_UNITS, 6.0);
        bar.setStyleContext((int) (ctx.hudFontSize * units), ctx.lineWidth + 2, ctx.drawFontSmall);
        return bar;
    }

    private static HUDComponent createSpeedBar(WidgetBuildContext c) {
        MinimalHUDContext ctx = c.ctx;
        SpeedRatioBar bar = new SpeedRatioBar();
        double units = c.entry.getDouble(MiniHudProps.LENGTH_UNITS, 5.5);
        bar.setStyleContext((int) Math.max(6, ctx.hudFontSize * 0.25), (int) (ctx.hudFontSize * units),
                ctx.drawFontSSmall);
        return bar;
    }

    private static HUDComponent createThrottle(WidgetBuildContext c) {
        MinimalHUDContext ctx = c.ctx;
        LinearGauge gauge = new LinearGauge("ThrottleBar", 110, true, false);
        double units = c.entry.getDouble(MiniHudProps.LENGTH_UNITS, 4.8);
        gauge.setStyleContext((int) (ctx.hudFontSize * units), ctx.barWidth, ctx.drawFontSSmall, ctx.drawFontSSmall);
        return gauge;
    }

    private static HUDComponent createCompass(WidgetBuildContext c) {
        MinimalHUDContext ctx = c.ctx;
        CompassGauge gauge = new CompassGauge(ctx.roundCompass);
        gauge.setStyleContext(ctx.roundCompass, ctx.lineWidth, ctx.hudFontSize, ctx.hudFontSizeSmall,
                ctx.drawFontSmall);
        gauge.setInertialMode(MiniHudProps.MODE_INERTIAL.equals(
                c.entry.get(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY)));
        return gauge;
    }

    private static HUDComponent createAttitude(WidgetBuildContext c) {
        MinimalHUDContext ctx = c.ctx;
        AttitudeIndicatorGauge gauge = new AttitudeIndicatorGauge();
        gauge.setStyleContext(ctx.compassDiameter, ctx.compassRadius, ctx.compassInnerMarkRadius, ctx.lineWidth,
                ctx.halfLine, ctx.drawFontSmall);
        gauge.setInertialMode(MiniHudProps.MODE_INERTIAL.equals(
                c.entry.get(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY)));
        return gauge;
    }
}
