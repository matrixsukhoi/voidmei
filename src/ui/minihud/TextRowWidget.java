package ui.minihud;

import java.awt.Dimension;
import java.awt.Font;
import java.awt.Graphics2D;
import java.util.ArrayList;
import java.util.List;
import java.util.function.DoubleSupplier;
import java.util.function.IntSupplier;
import java.util.function.Supplier;

import prog.Application;
import prog.config.MiniHudFieldCatalog;
import ui.UIBaseElements;
import ui.component.AbstractHUDComponent;
import ui.overlay.MinimalHUDContext;
import ui.overlay.model.HUDData;
import ui.model.TelemetrySource;
import ui.util.FastNumberFormatter;
import ui.util.ReflectBinder;
import ui.util.VisibilityExpressionEvaluator;

/**
 * 最基础的文本行 / grid 行：一行 N 个单元格，每格是数据字段或静态文本。
 * 预设行（速度+AoA 等）是它的特化；本组件负责用户自由搭建 HUD 的文本部分。
 */
public final class TextRowWidget extends AbstractHUDComponent {

    private static final class CellRuntime {
        MiniHudCells.Cell cell;
        String template;
        int width;
        DoubleSupplier valueSupplier;
        IntSupplier precisionSupplier;
        Supplier<String> unitSupplier;
        VisibilityExpressionEvaluator visibleEval;
        VisibilityExpressionEvaluator naEval;
        final char[] valueBuf = new char[32];
        int valueLen;
        String previewValue;
        String unit;
        String format;
        boolean cellVisible = true;
        boolean na;
    }

    private final String id;
    private final Font font;
    private final int height;
    private final List<CellRuntime> cells = new ArrayList<>();
    private final int gap;
    private int preferredWidth;

    public TextRowWidget(String id, MinimalHUDContext ctx, MiniHudEntry entry, boolean preview,
            TelemetrySource source, MiniHudFieldCatalog catalog) {
        this.id = id;
        this.font = ctx.drawFont;
        this.height = ctx.hudFontSize;
        this.gap = Math.max(2, ctx.hudFontSize / 2);

        List<MiniHudCells.Cell> parsed = MiniHudCells.parse(entry.get(MiniHudProps.CELLS, ""));
        if (parsed.isEmpty()) {
            MiniHudCells.Cell c = new MiniHudCells.Cell();
            c.text = "";
            parsed.add(c);
        }
        for (MiniHudCells.Cell c : parsed) {
            CellRuntime rt = new CellRuntime();
            rt.cell = c;
            MiniHudFieldCatalog.FieldDef def = null;
            if (c.isData() && catalog != null) {
                def = catalog.get(c.field);
                if (def != null) {
                    if (c.label.isEmpty())
                        c.label = def.label != null ? def.label : "";
                    if (c.unit.isEmpty())
                        c.unit = def.unit != null ? def.unit : "";
                    if (c.precision < 0)
                        c.precision = def.precision;
                }
            }
            rt.unit = c.unit;
            if (!c.isData()) {
                // 静态文本格：显示 text 本身，不套用数值占位
                rt.previewValue = c.text != null ? c.text : "";
                rt.template = c.label + rt.previewValue + c.unit;
            } else {
                String sample = def != null && def.previewValue != null && !def.previewValue.isEmpty()
                        ? def.previewValue
                        : "8888";
                rt.previewValue = sample;
                rt.template = c.label + sample + c.unit;
            }
            rt.width = Math.max(1, ui.overlay.logic.HUDCalculator.getStringWidth(rt.template, font));
            if (source != null && c.isData()) {
                rt.valueSupplier = ReflectBinder.resolveDouble(source, c.field);
                if (def != null) {
                    rt.format = def.format;
                    if (def.visibleWhen != null)
                        rt.visibleEval = new VisibilityExpressionEvaluator(def.visibleWhen, source);
                    if (def.naWhen != null)
                        rt.naEval = new VisibilityExpressionEvaluator(def.naWhen, source);
                    if (def.unitSource != null)
                        rt.unitSupplier = ReflectBinder.resolveString(source, def.unitSource);
                    if (def.precisionSource != null)
                        rt.precisionSupplier = ReflectBinder.resolveInt(source, def.precisionSource);
                }
            }
            cells.add(rt);
        }
        recomputeWidth();
    }

    private void recomputeWidth() {
        int w = 0;
        for (CellRuntime rt : cells)
            w += rt.width + gap;
        preferredWidth = Math.max(1, w - gap);
    }

    @Override
    public String getId() {
        return id;
    }

    @Override
    public Dimension getPreferredSize() {
        return new Dimension(preferredWidth, height);
    }

    /** EDT 每帧更新：直接读 TelemetrySource，格式化到复用的 char[]。 */
    public void updateData(TelemetrySource source) {
        if (source == null)
            return;
        for (CellRuntime rt : cells) {
            if (rt.valueSupplier == null)
                continue;
            double v = rt.valueSupplier.getAsDouble();
            rt.cellVisible = rt.visibleEval == null || rt.visibleEval.evaluate(v);
            rt.na = rt.naEval != null && rt.naEval.evaluate(v);
            if (rt.na) {
                rt.valueBuf[0] = '-';
                rt.valueLen = 1;
            } else if ("TIME_MM_SS".equals(rt.format)) {
                rt.valueLen = FastNumberFormatter.formatTime(v, rt.valueBuf);
            } else {
                int p = rt.precisionSupplier != null ? rt.precisionSupplier.getAsInt() : rt.cell.precision;
                rt.valueLen = FastNumberFormatter.format(v, rt.valueBuf, p);
            }
            if (rt.unitSupplier != null) {
                String u = rt.unitSupplier.get();
                if (u != null)
                    rt.unit = u;
            }
        }
    }

    @Override
    public void onDataUpdate(HUDData data) {
        // 自定义字段由 updateData(TelemetrySource) 驱动
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        if (font == null)
            return;
        g2d.setFont(font);
        int baseline = y + g2d.getFontMetrics(font).getAscent();
        int cursor = x;
        for (CellRuntime rt : cells) {
            if (rt.cellVisible) {
                int labelW = rt.cell.label.isEmpty() ? 0
                        : ui.overlay.logic.HUDCalculator.getStringWidth(rt.cell.label, font);
                int valueW = (rt.valueSupplier == null || rt.valueLen == 0)
                        ? ui.overlay.logic.HUDCalculator.getStringWidth(rt.previewValue, font)
                        : measureBuffer(rt.valueBuf, rt.valueLen, g2d);
                int unitW = rt.unit == null || rt.unit.isEmpty() ? 0
                        : ui.overlay.logic.HUDCalculator.getStringWidth(rt.unit, font);
                int contentW = labelW + valueW + unitW;
                int start = cursor;
                if (rt.cell.align == 'R')
                    start = cursor + Math.max(0, rt.width - contentW);
                else if (rt.cell.align == 'C')
                    start = cursor + Math.max(0, (rt.width - contentW) / 2);

                int cx = start;
                if (labelW > 0) {
                    UIBaseElements.__drawStringShade(g2d, cx, baseline, 1, rt.cell.label, font, Application.colorNum);
                    cx += labelW;
                }
                if (rt.valueSupplier == null || rt.valueLen == 0) {
                    UIBaseElements.__drawStringShade(g2d, cx, baseline, 1, rt.previewValue, font,
                            Application.colorNum);
                } else {
                    UIBaseElements.__drawStringShade(g2d, cx, baseline, 1, rt.valueBuf, rt.valueLen, font,
                            Application.colorNum);
                }
                cx += valueW;
                if (unitW > 0) {
                    UIBaseElements.__drawStringShade(g2d, cx, baseline, 1, rt.unit, font, Application.colorNum);
                }
            }
            cursor += rt.width + gap;
        }
    }

    private static int measureBuffer(char[] buf, int len, Graphics2D g2d) {
        if (len <= 0)
            return 0;
        return g2d.getFontMetrics().charsWidth(buf, 0, len);
    }
}
