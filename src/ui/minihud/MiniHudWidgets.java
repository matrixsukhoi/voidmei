package ui.minihud;

import ui.component.HUDComponent;
import ui.minihud.widget.AttitudeWidget;
import ui.minihud.widget.BarWidget;
import ui.minihud.widget.CompassWidget;
import ui.minihud.widget.PanelWidget;
import ui.minihud.widget.TextWidget;

/**
 * 内置原子组件注册。注册表里只有原子组件；
 * “速度行/高度行”等预设是编辑器宏（MiniHudPresets），不注册成组件类型。
 */
public final class MiniHudWidgets {

    private static boolean registered = false;

    private MiniHudWidgets() {
    }

    public static synchronized void registerDefaults() {
        if (registered)
            return;
        registered = true;

        reg(MiniHudProps.TYPE_TEXT, "文本", "文本 / 数据字段", true, true, EdgeResize.BOTH,
                TextWidget::new);
        reg(MiniHudProps.TYPE_BAR, "条与仪表", "条 / 量表", true, true, EdgeResize.BOTH,
                BarWidget::new);
        reg(MiniHudProps.TYPE_COMPASS, "条与仪表", "罗盘", false, true, EdgeResize.BOTH,
                CompassWidget::new);
        reg(MiniHudProps.TYPE_ATTITUDE, "条与仪表", "姿态仪", false, true, EdgeResize.BOTH,
                AttitudeWidget::new);
        reg(MiniHudProps.TYPE_PANEL, "背景", "背景板", false, true, EdgeResize.BOTH,
                PanelWidget::new);
    }

    private static void reg(String type, String category, String name, boolean dataField, boolean scalable,
            EdgeResize edge, HUDWidgetFactory f) {
        HUDWidgetRegistry.register(new HUDWidgetSpec(type, category, name, dataField, scalable, edge, f));
    }

    public static HUDComponent create(WidgetBuildContext c) {
        HUDWidgetSpec spec = HUDWidgetRegistry.find(c.widget.type);
        if (spec == null)
            return new TextWidget(c);
        return spec.factory.create(c);
    }

    /** 新建原子组件时的默认属性。 */
    public static void applyDefaults(MiniHudWidget w) {
        switch (w.type) {
            case MiniHudProps.TYPE_TEXT:
                w.set(MiniHudProps.SOURCE, MiniHudProps.SRC_STATIC);
                w.set(MiniHudProps.TEXT, "TEXT");
                w.set(MiniHudProps.TEMPLATE, "TEXT");
                w.set(MiniHudProps.PREVIEW, "TEXT");
                break;
            case MiniHudProps.TYPE_BAR:
                w.set(MiniHudProps.BAR_SOURCE, MiniHudProps.BAR_FIELD);
                w.set(MiniHudProps.MIN, "0");
                w.set(MiniHudProps.MAX, "1");
                w.set(MiniHudProps.LENGTH_UNITS, "4");
                break;
            case MiniHudProps.TYPE_COMPASS:
            case MiniHudProps.TYPE_ATTITUDE:
                w.set(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY);
                break;
            case MiniHudProps.TYPE_PANEL:
                w.set(MiniHudProps.WIDTH_UNITS, "6");
                w.set(MiniHudProps.HEIGHT_UNITS, "3");
                w.set(MiniHudProps.RADIUS_UNITS, "0.25");
                w.set(MiniHudProps.PANEL_COLOR, "#60202020");
                break;
            default:
                break;
        }
    }
}
