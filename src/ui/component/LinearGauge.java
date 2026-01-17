package ui.component;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics2D;

import prog.Application;

public class LinearGauge {
    public String label;
    public String unit;
    public int maxValue;
    public int curValue;
    public String displayValue;
    public boolean vertical;

    // Cached graphics objects for performance
    private BasicStroke borderStroke;
    private BasicStroke separatorStroke;
    private int cachedThickness = -1;

    public LinearGauge(String label, int maxValue, boolean vertical) {
        this.label = label;
        this.maxValue = maxValue;
        this.vertical = vertical;
        this.displayValue = "";
    }

    public void update(int value, String displayValue) {
        this.curValue = value;
        this.displayValue = displayValue;
    }

    /**
     * Draws the gauge.
     * Optimized to minimize object allocation in the render loop.
     */
    public void draw(Graphics2D g2d, int x, int y, int length, int thickness, Font fontLabel, Font fontNum) {
        // Update cached strokes if parameters change
        if (thickness != cachedThickness) {
            borderStroke = new BasicStroke(1, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            separatorStroke = new BasicStroke(1, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            cachedThickness = thickness;
        }

        int pixVal = 0;
        if (maxValue > 0) {
            pixVal = Math.round((float) curValue * length / maxValue);
        }

        Color c = Application.colorNum; // Can be parameterized if needed
        Color shade = Application.colorShadeShape;
        Color lblColor = Application.colorLabel;

        g2d.setStroke(borderStroke);

        if (vertical) {
            // Vertical Bar: width=thickness, height=length
            drawBar(g2d, x, y, thickness, length, pixVal, shade, c, true);

            // Separator Line
            g2d.setStroke(separatorStroke);
            drawRect(g2d, x, y - pixVal - 1, thickness + 3 * fontNum.getSize(), 3, shade, lblColor, false);

            // Text Number
            drawTextShaded(g2d, x + thickness, y - pixVal - 2, displayValue, fontNum, lblColor);
        } else {
            // Horizontal Bar: width=length, height=thickness
            drawBar(g2d, x, y, length, thickness, pixVal, shade, c, false);

            // Separator Line
            g2d.setStroke(separatorStroke);
            // drawVRect behavior for horizontal separator
            drawRect(g2d, x + pixVal - 2, y, 3, -thickness - 1 * fontNum.getSize(), shade, lblColor, true); // true for
                                                                                                            // "vertical"
                                                                                                            // rect
                                                                                                            // (width <
                                                                                                            // height
                                                                                                            // logic)

            // Text Number
            drawTextShaded(g2d, x + pixVal, y + thickness + 1 * numFontHeight(fontNum), displayValue, fontNum,
                    lblColor);
        }
    }

    // Helper for height calculation without Metrics if possible, or just
    // approximated
    private int numFontHeight(Font f) {
        return f.getSize();
    }

    private void drawBar(Graphics2D g2d, int x, int y, int w, int h, int val, Color shade, Color fill, boolean isVert) {
        // Outer Border
        g2d.setColor(shade);
        if (isVert) {
            // Draw growing upwards from y
            g2d.drawRect(x, y - h, w - 1, h - 1);
            g2d.setColor(fill);
            // Fill based on val
            int valH = (val > h) ? h : val;
            if (valH >= 0)
                g2d.fillRect(x + 1, y + 1 - valH, w - 2, valH - 2);
        } else {
            // Horizontal
            g2d.drawRect(x, y, w - 1, h - 1);
            g2d.setColor(fill);
            int valW = (val > w) ? w : val;
            if (valW >= 0)
                g2d.fillRect(x + 1, y + 1, valW - 2, h - 2);
        }
    }

    private void drawRect(Graphics2D g2d, int x, int y, int w, int h, Color shade, Color fill, boolean flipLogic) {
        g2d.setColor(shade);
        // Simplified rect drawing based on UIBaseElements logic
        if (!flipLogic) {
            // Standard
            g2d.drawRect(x, y, w - 1, h - 1);
            g2d.setColor(fill);
            g2d.fillRect(x + 1, y + 1, w - 2, h - 2);
        } else {
            // Flip (used for vertical separator)
            g2d.drawRect(x + w, y, -w - 1, h - 1);
            g2d.setColor(fill);
            g2d.fillRect(x + 1 + w, y + 1, -w - 2, h - 2);
        }
    }

    // Custom shaded text drawing to avoid UIBaseElements.__drawStringShade overhead
    // (if any)
    // For now, simple shadow
    private void drawTextShaded(Graphics2D g2d, int x, int y, String s, Font f, Color c) {
        g2d.setFont(f);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawString(s, x + 1, y + 1);
        g2d.setColor(c);
        g2d.drawString(s, x, y);
    }
}
