package ui.component.row;

import java.awt.Font;
import java.awt.Graphics2D;

import prog.Application;
import ui.UIBaseElements;

public class HUDTextRow implements HUDRow {

    protected String text;
    protected Font font;
    protected int height;
    protected int index; // For debug logging
    protected boolean isWarning;

    public HUDTextRow(int index, Font font, int height) {
        this.index = index;
        this.font = font;
        this.height = height;
        this.text = "";
    }

    public void setStyle(Font font, int height) {
        this.font = font;
        this.height = height;
    }

    @Override
    public String getId() {
        return "row." + index;
    }

    @Override
    public void update(Object data) {
        if (data instanceof Object[]) {
            Object[] params = (Object[]) data;
            if (params.length >= 2) {
                this.text = (String) params[0];
                this.isWarning = (Boolean) params[1];
            }
        }
    }

    public void update(String text, boolean isWarning) {
        this.text = text;
        this.isWarning = isWarning;
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        // Convert Top-Left y to Baseline y
        int ascent = g2d.getFontMetrics(font).getAscent();
        int baseY = y + ascent;

        if (isWarning) {
            UIBaseElements.__drawStringShade(g2d, x, baseY, 1, text, font, Application.colorWarning);
        } else {
            UIBaseElements.__drawStringShade(g2d, x, baseY, 1, text, font, Application.colorNum);
        }
    }

    @Override
    public int getHeight() {
        return height;
    }

    // Layout Stability Template
    protected String templateText;

    public void setTemplate(String template) {
        this.templateText = template;
    }

    @Override
    public java.awt.Dimension getPreferredSize() {
        int w = 200;
        // Use template text for stable width if available
        String textToMeasure = (templateText != null && !templateText.isEmpty()) ? templateText : text;

        if (textToMeasure != null && font != null) {
            // Precise measurement using shared/dummy context is ideal,
            // but for performance and valid estimation we can use a heuristic or Toolkit.
            // Since this runs every frame, we want to be fast.
            // But layout is only recalculated if dirty.
            // Let's use a robust estimation for Monospace font (Sarasa Mono SC):
            // Width ~= fontSize * 0.6 * ascii_len + fontSize * 1.0 * cjk_len.
            // Or just Measure it properly.
            // Let's rely on standard AWT measurement helper.
            w = ui.overlay.logic.HUDCalculator.getStringWidth(textToMeasure, font);
        }
        return new java.awt.Dimension(w, height);
    }
}
