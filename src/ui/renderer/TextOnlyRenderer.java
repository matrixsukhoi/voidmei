package ui.renderer;

import java.awt.Color;
import java.awt.Dimension;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.util.List;

import prog.Application;
import ui.model.DataField;

/**
 * Renderer for displaying a list of text fields.
 * Ignores Label and Unit, only renders CurrentValue.
 * Useful for FM Data display or simple logs.
 */
public class TextOnlyRenderer implements OverlayRenderer {

    @Override
    public void render(Graphics2D g2d, List<DataField> fields, RenderContext ctx, int[] offset) {
        g2d.setPaintMode();
        g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
        g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);

        // Font setup
        g2d.setFont(ctx.labelFont); // Use label font for text
        g2d.setColor(Color.WHITE); // Default color

        int x = ctx.fontSize >> 1;
        int y = ctx.fontSize; // Start slightly lower to account for ascent
        int lineHeight = (int) (ctx.fontSize * 1.5);

        for (DataField field : fields) {
            if (!field.visible) {
                continue;
            }

            g2d.drawString(field.currentValue, x, y);
            y += lineHeight;
        }

        // Update offset for next pass (though typically this renderer consumes full
        // space)
        offset[1] = y;
    }

    @Override
    public Dimension calculatePreferredSize(List<DataField> fields, RenderContext ctx) {
        int visibleCount = 0;
        int maxWidth = 0;
        // Estimate width (rough)
        int charWidth = ctx.fontSize / 2;

        for (DataField field : fields) {
            if (field.visible) {
                visibleCount++;
                if (field.currentValue != null) {
                    maxWidth = Math.max(maxWidth, field.currentValue.length() * charWidth);
                }
            }
        }

        int lineHeight = (int) (ctx.fontSize * 1.5);
        int height = visibleCount * lineHeight + ctx.fontSize;

        return new Dimension(maxWidth + ctx.fontSize, height);
    }
}
