package ui.renderer;

import java.awt.Dimension;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.util.List;

import prog.app;
import ui.model.DataField;
import ui.uiBaseElem;

/**
 * BOS-style renderer for overlay fields.
 * Displays fields in a multi-column grid layout with number, label, and unit.
 */
public class BOSStyleRenderer implements OverlayRenderer {

    @Override
    public void render(Graphics2D g2d, List<DataField> fields, RenderContext ctx, int[] offset) {
        g2d.setPaintMode();
        g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, app.graphAASetting);
        g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, app.textAASetting);
        g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
                RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
        g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING,
                RenderingHints.VALUE_COLOR_RENDER_SPEED);

        offset[0] = ctx.fontSize >> 1;
        offset[1] = ctx.fontSize >> 1;

        int visibleIndex = 0;
        int fieldWidth = ctx.getFieldWidth();

        for (DataField field : fields) {
            if (!field.visible) {
                continue;
            }

            uiBaseElem._drawLabelBOSType(g2d, offset[0], offset[1], 1, fieldWidth,
                    ctx.numFont, ctx.labelFont, ctx.unitFont,
                    field.currentValue, field.label, field.unit);

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
