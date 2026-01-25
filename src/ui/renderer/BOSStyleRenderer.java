package ui.renderer;

import java.awt.Dimension;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.util.List;

import prog.Application;
import ui.model.DataField;

/**
 * BOS-style renderer for overlay fields.
 * Displays fields in a multi-column grid layout with number, label, and unit.
 */
public class BOSStyleRenderer implements OverlayRenderer {

    // Cache TextGauges to maintain stroke caching
    private java.util.Map<String, ui.component.TextGauge> gaugeCache = new java.util.HashMap<>();

    @Override
    public void render(Graphics2D g2d, List<DataField> fields, RenderContext ctx, int[] offset) {
        g2d.setPaintMode();
        g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
        g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);
        g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
                RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
        g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING,
                RenderingHints.VALUE_COLOR_RENDER_SPEED);

        offset[0] = ctx.fontSize >> 1;
        offset[1] = ctx.fontSize >> 1;

        int visibleIndex = 0;
        // int fieldWidth = ctx.getFieldWidth(); // TextGauge handles layout internally
        // based on font size

        for (DataField field : fields) {
            if (!field.visible) {
                continue;
            }

            ui.component.TextGauge gauge = gaugeCache.get(field.label);
            if (gauge == null) {
                gauge = new ui.component.TextGauge(field.label, field.unit);
                gaugeCache.put(field.label, gauge);
            }

            // Update gauge state
            gauge.update(field.currentValue);

            // Sync dynamic unit
            if (field.unit != null && !field.unit.equals(gauge.unit)) {
                gauge.setUnit(field.unit);
            }
            // gauge.label/unit are final in current TextGauge, assuming they don't change
            // frequently.
            // If they do (e.g. dynamic unit change), we might need to update them too.
            // For now, assume static metadata.

            // Draw using zero-GC char buffer if available
            gauge.draw(g2d, offset[0], offset[1], ctx.numFont, ctx.labelFont, ctx.unitFont, 1, field.buffer,
                    field.length);

            visibleIndex++;
            updateOffset(visibleIndex, offset, ctx);
        }
    }

    private void updateOffset(int visibleIndex, int[] offset, RenderContext ctx) {
        if (visibleIndex % ctx.columnNum == 0) {
            offset[1] += Math.round(1 * ctx.numHeight);
            offset[0] = ctx.fontSize >> 1;
        } else {
            offset[0] += Math.round(5 * ctx.fontSize);
        }
    }

    @Override
    public Dimension calculatePreferredSize(List<DataField> fields, RenderContext ctx) {
        int visibleCount = 0;
        for (DataField field : fields) {
            if (field.visible)
                visibleCount++;
        }

        int width = ctx.getTotalWidth();
        int height = ctx.getTotalHeight(visibleCount);

        return new Dimension(width, height);
    }
}
