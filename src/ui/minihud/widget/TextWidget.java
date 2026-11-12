package ui.minihud.widget;

import java.awt.Color;
import java.awt.Dimension;
import java.awt.Font;
import java.awt.Graphics2D;
import java.util.function.DoubleSupplier;
import java.util.function.IntSupplier;
import java.util.function.Supplier;

import prog.Application;
import prog.config.MiniHudFieldCatalog;
import ui.UIBaseElements;
import ui.component.AbstractHUDComponent;
import ui.component.HUDComponent;
import ui.minihud.MiniHudProps;
import ui.minihud.MiniHudWidget;
import ui.minihud.WidgetBuildContext;
import ui.model.TelemetrySource;
import ui.overlay.MinimalHUDContext;
import ui.overlay.logic.HUDCalculator;
import ui.overlay.model.HUDData;
import ui.util.FastNumberFormatter;
import ui.util.ReflectBinder;
import ui.util.VisibilityExpressionEvaluator;

/**
 * 原子文本组件：一个实例只表达一个读数或一段静态文本。
 * label/unit/precision/format 是它的格式化属性，不是可独立移动的子组件。
 *
 * 同时支持两类来源：
 * - 内置语义来源（IAS/Ma/高度/能量/攻角/SEP/G/襟翼/时间…）
 * - 配置字段目录里的任意 TelemetrySource 字段（反射绑定）
 */
public final class TextWidget extends AbstractHUDComponent implements HUDComponent {

    private final String id;
    private final Font font;
    private final int height;
    private final int baselineAscent;
    private final int width;
    private final String align;
    private final String source;
    private final String label;
    private final String unit;
    private final String altSource;
    private final boolean arrow;
    private final String colorMode;
    private final String staticText;

    // 配置字段绑定
    private final boolean fieldMode;
    private final String field;
    private final int precision;
    private final String format;
    private final DoubleSupplier valueSupplier;
    private final IntSupplier precisionSupplier;
    private final Supplier<String> unitSupplier;
    private final VisibilityExpressionEvaluator visibleEval;
    private final VisibilityExpressionEvaluator naEval;
    private final char[] valueBuf = new char[48];
    private final String previewText;
    private final boolean previewMode;

    private String currentText = "";
    private Color currentColor = Application.colorNum;

    public TextWidget(WidgetBuildContext c) {
        MiniHudWidget w = c.widget;
        MinimalHUDContext ctx = c.ctx;
        this.id = w.id;
        this.previewMode = c.preview;
        this.source = w.get(MiniHudProps.SOURCE, MiniHudProps.SRC_STATIC);
        this.staticText = w.get(MiniHudProps.TEXT, "");
        this.altSource = w.get(MiniHudProps.ALT_SOURCE, MiniHudProps.ALT_AUTO);
        this.arrow = w.getBool(MiniHudProps.ARROW, true);
        this.colorMode = w.get(MiniHudProps.COLOR, MiniHudProps.COLOR_NORMAL);
        this.align = w.get(MiniHudProps.ALIGN, MiniHudProps.ALIGN_LEFT);
        String fontMode = w.get(MiniHudProps.FONT, MiniHudProps.FONT_MAIN);
        if (MiniHudProps.FONT_SMALL.equals(fontMode))
            this.font = ctx.drawFontSmall;
        else if (MiniHudProps.FONT_TINY.equals(fontMode))
            this.font = ctx.drawFontSSmall;
        else
            this.font = ctx.drawFont;
        this.height = ctx.hudFontSize;
        // 默认按自身字体对齐；旧版行内小字原本与主字体共基线，通过 baseline=main 指定
        int mainAscent = METRICS.getFontMetrics(ctx.drawFont).getAscent();
        String baseline = w.get(MiniHudProps.BASELINE, MiniHudProps.BASELINE_FONT);
        this.baselineAscent = MiniHudProps.BASELINE_MAIN.equals(baseline)
                ? mainAscent : METRICS.getFontMetrics(font).getAscent();

        this.fieldMode = MiniHudProps.SRC_FIELD.equals(source);
        MiniHudFieldCatalog.FieldDef def = null;
        if (fieldMode) {
            this.field = w.get(MiniHudProps.FIELD, "");
            def = (c.catalog != null && !field.isEmpty()) ? c.catalog.get(field) : null;
        } else {
            this.field = "";
        }

        String l = w.get(MiniHudProps.LABEL, null);
        if (l == null)
            l = fieldMode ? (def != null && def.label != null ? def.label : field)
                    : defaultLabel(source);
        this.label = l;

        String u = w.get(MiniHudProps.UNIT, null);
        if (u == null)
            u = fieldMode ? (def != null && def.unit != null ? def.unit : "") : defaultUnit(source);
        this.unit = u;

        int p = w.getInt(MiniHudProps.PRECISION, -1);
        if (p < 0)
            p = fieldMode ? (def != null ? def.precision : 2) : defaultPrecision(source);
        this.precision = p;
        this.format = fieldMode && def != null ? def.format : "";

        String template = w.get(MiniHudProps.TEMPLATE, "");
        if (template.isEmpty())
            template = defaultTemplate(fieldMode, def, source, label, unit, staticText);
        // 预览值优先取组件自己的属性；老布局没有该属性时按来源生成默认演示值
        if (w.props.containsKey(MiniHudProps.PREVIEW))
            this.previewText = w.get(MiniHudProps.PREVIEW, "");
        else
            this.previewText = fieldMode ? buildPreview(def)
                    : defaultPreview(source, label, staticText);

        double widthUnits = w.getDouble(MiniHudProps.WIDTH_UNITS, 0);
        if (widthUnits > 0)
            this.width = Math.max(1, Math.round((float) (widthUnits * ctx.hudFontSize)));
        else
            this.width = Math.max(1, HUDCalculator.getStringWidth(template, font));

        if (fieldMode && c.source != null) {
            this.valueSupplier = ReflectBinder.resolveDouble(c.source, field);
            this.precisionSupplier = def != null && def.precisionSource != null
                    ? ReflectBinder.resolveInt(c.source, def.precisionSource)
                    : null;
            this.unitSupplier = def != null && def.unitSource != null
                    ? ReflectBinder.resolveString(c.source, def.unitSource)
                    : null;
            this.visibleEval = def != null && def.visibleWhen != null
                    ? new VisibilityExpressionEvaluator(def.visibleWhen, c.source)
                    : null;
            this.naEval = def != null && def.naWhen != null
                    ? new VisibilityExpressionEvaluator(def.naWhen, c.source)
                    : null;
        } else {
            this.valueSupplier = null;
            this.precisionSupplier = null;
            this.unitSupplier = null;
            this.visibleEval = null;
            this.naEval = null;
        }
        this.currentText = previewText;
    }

    private static final java.awt.image.BufferedImage METRICS_IMG = new java.awt.image.BufferedImage(1, 1,
            java.awt.image.BufferedImage.TYPE_INT_ARGB);
    private static final Graphics2D METRICS = METRICS_IMG.createGraphics();

    @Override
    public String getId() {
        return id;
    }

    @Override
    public Dimension getPreferredSize() {
        return new Dimension(width, height);
    }

    public String getCurrentText() {
        return currentText;
    }

    /** 配置字段每帧更新（EDT 上读取 TelemetrySource，沿用 FieldOverlay 先例）。 */
    public void updateData(TelemetrySource source) {
        if (previewMode || !fieldMode || valueSupplier == null || source == null)
            return;
        double v = valueSupplier.getAsDouble();
        if (visibleEval != null && !visibleEval.evaluate(v)) {
            currentText = "";
            return;
        }
        String value;
        if (isSentinel(v) || (naEval != null && naEval.evaluate(v))) {
            value = "-";
        } else if ("TIME_MM_SS".equals(format)) {
            int len = FastNumberFormatter.formatTime(v, valueBuf);
            value = new String(valueBuf, 0, len);
        } else {
            int p = precisionSupplier != null ? precisionSupplier.getAsInt() : precision;
            int len = FastNumberFormatter.format(v, valueBuf, p);
            value = new String(valueBuf, 0, len);
        }
        String u = unit;
        if (unitSupplier != null) {
            String dynamic = unitSupplier.get();
            if (dynamic != null)
                u = dynamic;
        }
        currentText = label + value + u;
        currentColor = Application.colorNum;
    }

    @Override
    public void onDataUpdate(HUDData data) {
        if (data == null || fieldMode || previewMode)
            return;
        switch (source) {
            case MiniHudProps.SRC_IAS:
                currentText = label + String.format("%5.0f", data.ias);
                break;
            case MiniHudProps.SRC_MACH:
                currentText = label + String.format("%5.2f", data.mach);
                break;
            case MiniHudProps.SRC_ALTITUDE:
                boolean radar = MiniHudProps.ALT_RADAR.equals(altSource) && data.radioAltitudeValid;
                if (MiniHudProps.ALT_AUTO.equals(altSource))
                    radar = data.warnAltitude;
                currentText = label + (radar ? String.format("R%5.0f", data.radioAltitude)
                        : String.format("%6.0f", data.altitude));
                break;
            case MiniHudProps.SRC_ENERGY:
                currentText = label + String.format("%5.0f", data.energyM);
                break;
            case MiniHudProps.SRC_AOA:
                currentText = label + String.format("%3.0f", data.aoa);
                break;
            case MiniHudProps.SRC_SEP:
                String arrowChar = arrow ? (data.verticalSpeed > 0 ? "↑" : "↓") : "";
                currentText = label + arrowChar + String.format("%-4.0f", data.verticalSpeed);
                break;
            case MiniHudProps.SRC_GLOAD:
                if (data.gLoad > 1.5 || data.gLoad < -0.5)
                    currentText = label + String.format("%5.1f", data.gLoad);
                else
                    currentText = (data.timeStr != null && !data.timeStr.isEmpty()) ? "L" + data.timeStr : "";
                break;
            case MiniHudProps.SRC_FLAPS:
                currentText = data.flapsWingStr;
                break;
            case MiniHudProps.SRC_AIRBRAKE:
                currentText = data.airbrakeStr;
                break;
            case MiniHudProps.SRC_GEAR:
                currentText = data.gearStr;
                break;
            case MiniHudProps.SRC_TIME:
                currentText = data.timeStr != null ? data.timeStr : "";
                break;
            case MiniHudProps.SRC_FLAP_ANGLE:
                currentText = String.format("%3.0f/%3.0f", data.flaps, data.flapAllowAngle);
                break;
            case MiniHudProps.SRC_THROTTLE:
                currentText = String.format("%3d", data.throttle);
                break;
            case MiniHudProps.SRC_STATIC:
            default:
                currentText = staticText;
                break;
        }
        currentColor = colorFor(data);
    }

    private Color colorFor(HUDData data) {
        if (MiniHudProps.COLOR_AOA.equals(colorMode))
            return data.aoaColor != null ? data.aoaColor : Application.colorNum;
        if (MiniHudProps.COLOR_WARNING.equals(colorMode))
            return Application.colorWarning;
        if (!MiniHudProps.COLOR_AUTO.equals(colorMode))
            return Application.colorNum;
        switch (source) {
            case MiniHudProps.SRC_IAS:
            case MiniHudProps.SRC_MACH:
                return data.warnVne ? Application.colorWarning : Application.colorNum;
            case MiniHudProps.SRC_ALTITUDE:
                return data.warnAltitude ? Application.colorWarning : Application.colorNum;
            case MiniHudProps.SRC_FLAPS:
            case MiniHudProps.SRC_AIRBRAKE:
            case MiniHudProps.SRC_GEAR:
                return data.warnConfiguration ? Application.colorWarning : Application.colorNum;
            default:
                return Application.colorNum;
        }
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        if (font == null || currentText == null || currentText.isEmpty())
            return;
        g2d.setFont(font);
        int baseline = y + baselineAscent;
        int textWidth = g2d.getFontMetrics(font).stringWidth(currentText);
        int dx = 0;
        if (MiniHudProps.ALIGN_CENTER.equals(align))
            dx = Math.max(0, (width - textWidth) / 2);
        else if (MiniHudProps.ALIGN_RIGHT.equals(align))
            dx = Math.max(0, width - textWidth);
        UIBaseElements.__drawStringShade(g2d, x + dx, baseline, 1, currentText, font, currentColor);
    }

    /** 8111 退化帧常见哨兵值：-65535、绝对零度水温/油温；不直接显示。 */
    private boolean isSentinel(double v) {
        if (Double.isNaN(v) || Double.isInfinite(v))
            return true;
        if (v == -65535)
            return true;
        String f = field != null ? field.toLowerCase() : "";
        return f.contains("temp") && v < -100;
    }

    private String buildPreview(MiniHudFieldCatalog.FieldDef def) {
        String sample = def != null && def.previewValue != null && !def.previewValue.isEmpty()
                ? def.previewValue
                : "8888";
        return label + sample + unit;
    }

    private static String defaultLabel(String source) {
        switch (source) {
            case MiniHudProps.SRC_MACH:
                return "M";
            case MiniHudProps.SRC_AOA:
                return "α";
            case MiniHudProps.SRC_SEP:
                return "SEP";
            case MiniHudProps.SRC_GLOAD:
                return "G";
            case MiniHudProps.SRC_ENERGY:
                return "E";
            case MiniHudProps.SRC_ALTITUDE:
                return "ALT";
            case MiniHudProps.SRC_IAS:
                return "SPD";
            default:
                return "";
        }
    }

    private static String defaultUnit(String source) {
        return "";
    }

    private static int defaultPrecision(String source) {
        switch (source) {
            case MiniHudProps.SRC_MACH:
                return 2;
            case MiniHudProps.SRC_GLOAD:
                return 1;
            default:
                return 0;
        }
    }

    /** 预览演示值：与旧版默认预览一致，避免组件在无数据时显示宽度模板里的占位值。 */
    private static String defaultPreview(String source, String label, String staticText) {
        switch (source) {
            case MiniHudProps.SRC_IAS:
                return label + "  360";
            case MiniHudProps.SRC_MACH:
                return label + " 0.85";
            case MiniHudProps.SRC_ALTITUDE:
                return label + "  1024";
            case MiniHudProps.SRC_ENERGY:
                return label + "114514";
            case MiniHudProps.SRC_AOA:
                return label + "  4";
            case MiniHudProps.SRC_SEP:
                return label + "↑30  ";
            case MiniHudProps.SRC_GLOAD:
                return label + "  2.0";
            case MiniHudProps.SRC_FLAPS:
                return "F 20 ";
            case MiniHudProps.SRC_AIRBRAKE:
                return "";
            case MiniHudProps.SRC_GEAR:
                return "GEA";
            case MiniHudProps.SRC_TIME:
                return "12:00";
            case MiniHudProps.SRC_FLAP_ANGLE:
                return " 20/100";
            case MiniHudProps.SRC_THROTTLE:
                return "100";
            case MiniHudProps.SRC_STATIC:
            default:
                return staticText;
        }
    }

    private static String defaultTemplate(boolean fieldMode, MiniHudFieldCatalog.FieldDef def, String source,
            String label, String unit, String staticText) {
        if (!fieldMode) {
            switch (source) {
                case MiniHudProps.SRC_IAS:
                    return label + "  360";
                case MiniHudProps.SRC_MACH:
                    return label + " 0.85";
                case MiniHudProps.SRC_ALTITUDE:
                    return label + "  1024";
                case MiniHudProps.SRC_ENERGY:
                    return label + "114514";
                case MiniHudProps.SRC_AOA:
                    return label + " 20";
                case MiniHudProps.SRC_SEP:
                    return label + "↑30  ";
                case MiniHudProps.SRC_GLOAD:
                    return label + "  2.0";
                case MiniHudProps.SRC_FLAPS:
                    return "F100 ";
                case MiniHudProps.SRC_AIRBRAKE:
                    return "BRK ";
                case MiniHudProps.SRC_GEAR:
                    return "GEA";
                case MiniHudProps.SRC_TIME:
                    return "12:00";
                case MiniHudProps.SRC_FLAP_ANGLE:
                    return "  0/100";
                case MiniHudProps.SRC_THROTTLE:
                    return "100";
                default:
                    return staticText;
            }
        }
        String sample = def != null && def.previewValue != null && !def.previewValue.isEmpty()
                ? def.previewValue
                : "8888";
        return label + sample + unit;
    }
}
