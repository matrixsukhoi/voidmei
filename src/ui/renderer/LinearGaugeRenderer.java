package ui.renderer;

import java.awt.Dimension;
import java.awt.Font;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.util.List;

import prog.Application;
import ui.model.DataField;
import ui.model.GaugeField;

/**
 * Renderer for linear gauge displays (vertical/horizontal bars).
 * Designed for EngineControl-style overlays.
 */
public class LinearGaugeRenderer implements OverlayRenderer {

    @Override
    public void render(Graphics2D g2d, List<DataField> fields, RenderContext ctx, int[] offset) {
        g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
        g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);

        int fontsize = ctx.numFont.getSize();
        int x = offset[0];
        int y = offset[1];
        int dx = 0;
        int dy = fontsize >> 1;

        for (DataField field : fields) {
            if (!field.visible || !(field instanceof GaugeField)) {
                continue;
            }

            GaugeField gf = (GaugeField) field;
            ui.component.LinearGauge gauge = gf.gauge;

            if (gauge == null)
                continue;

            if (gf.isHorizontal) {
                gauge.vertical = false;
                gauge.draw(g2d, x, y + dy, 4 * fontsize, fontsize >> 1, ctx.labelFont, ctx.labelFont);
                dy += 1 * fontsize + (fontsize >> 2);
            } else {
                gauge.vertical = true;
                gauge.draw(g2d, x + dx, y, 4 * fontsize, fontsize >> 1, ctx.labelFont, ctx.labelFont);
                dx += (5 * fontsize) >> 1;
            }
        }
    }

    @Override
    public Dimension calculatePreferredSize(List<DataField> fields, RenderContext ctx) {
        int fontsize = ctx.numFont.getSize();
        int rowNum = 0;
        int columnNum = 0;

        for (DataField field : fields) {
            if (!field.visible || !(field instanceof GaugeField))
                continue;
            GaugeField gf = (GaugeField) field;
            if (gf.isHorizontal) {
                rowNum++;
            } else {
                columnNum++;
            }
        }

        int width = fontsize * 8;
        int height = (int) ((fontsize * 4 + (fontsize * 9) >> 1) + (rowNum + 1) * (1 * fontsize + (fontsize >> 2)));

        return new Dimension(width, height);
    }
}
