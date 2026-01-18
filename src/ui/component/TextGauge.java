package ui.component;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics2D;

import prog.Application;

/**
 * A high-performance component for drawing text gauges (Value + Label + Unit).
 * Replaces UIBaseElements.drawLabelBOSType pattern.
 */
public class TextGauge {

    public String label;
    public String unit;
    public String value;

    // Cached output
    private BasicStroke stroke;
    private int cachedShadeWidth = -1;

    public TextGauge(String label, String unit) {
        this.label = label;
        this.unit = unit;
        this.value = "";
    }

    public void update(String value) {
        this.value = value;
    }

    public void setUnit(String unit) {
        this.unit = unit;
    }

    public void draw(Graphics2D g2d, int x, int y, Font fontNum, Font fontLabel, Font fontUnit, int shadeWidth) {
        if (shadeWidth != cachedShadeWidth) {
            stroke = new BasicStroke(shadeWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            cachedShadeWidth = shadeWidth;
        }

        // Calculate layout
        // Label width approximation or calculation could be cached if fonts don't
        // change often
        // For dynamic layout, we assume standard BOS layout logic

        int lwidth = (13 * fontNum.getSize()) >> 2; // Derived from UIBaseElements logic

        // Y offset calculation
        int centerY = (y + y + fontLabel.getSize() + fontUnit.getSize()) >> 1;

        drawTextShaded(g2d, x, centerY, value, fontNum, Application.colorNum);

        drawTextShaded(g2d, x + lwidth, y, label, fontLabel, Application.colorLabel);

        drawTextShaded(g2d, x + lwidth, y + fontLabel.getSize(), unit, fontUnit, Application.colorUnit);
    }

    private void drawTextShaded(Graphics2D g2d, int x, int y, String s, Font f, Color c) {
        g2d.setFont(f);
        if (stroke != null) {
            g2d.setStroke(stroke);
            g2d.setColor(Application.colorShadeShape);
            // Simplistic shade drawing: offset 1px or stroke?
            // UIBaseElements uses stroke drawing if Application.drawFontShape is true,
            // else string doubling. To keep it simple and fast, we use string doubling
            // here.
            // If full stroking is needed, we should cache GlyphVectors (expensive).
            g2d.drawString(s, x + 1, y + 1);
        }
        g2d.setColor(c);
        g2d.drawString(s, x, y);
    }
}
