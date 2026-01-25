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
        // Legacy String draw
        draw(g2d, x, y, fontNum, fontLabel, fontUnit, shadeWidth, null, 0);
    }

    public void draw(Graphics2D g2d, int x, int y, Font fontNum, Font fontLabel, Font fontUnit, int shadeWidth,
            char[] valBuffer, int valLen) {
        if (shadeWidth != cachedShadeWidth) {
            stroke = new BasicStroke(shadeWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            cachedShadeWidth = shadeWidth;
        }

        int lwidth = (13 * fontNum.getSize()) >> 2;
        int centerY = (y + y + fontLabel.getSize() + fontUnit.getSize()) >> 1;

        if (valBuffer != null && valLen > 0) {
            drawTextShaded(g2d, x, centerY, valBuffer, valLen, fontNum, Application.colorNum);
        } else {
            drawTextShaded(g2d, x, centerY, value, fontNum, Application.colorNum);
        }

        drawTextShaded(g2d, x + lwidth, y, label, fontLabel, Application.colorLabel);
        drawTextShaded(g2d, x + lwidth, y + fontLabel.getSize(), unit, fontUnit, Application.colorUnit);
    }

    private void drawTextShaded(Graphics2D g2d, int x, int y, String s, Font f, Color c) {
        g2d.setFont(f);
        if (stroke != null) {
            drawShade(g2d, x, y, s);
        }
        g2d.setColor(c);
        g2d.drawString(s, x, y);
    }

    private void drawTextShaded(Graphics2D g2d, int x, int y, char[] buf, int len, Font f, Color c) {
        g2d.setFont(f);
        if (stroke != null) {
            drawShade(g2d, x, y, buf, len);
        }
        g2d.setColor(c);
        g2d.drawChars(buf, 0, len, x, y);
    }

    private void drawShade(Graphics2D g2d, int x, int y, String s) {
        g2d.setColor(Application.colorShadeShape);
        g2d.drawString(s, x + 1, y + 1);
    }

    private void drawShade(Graphics2D g2d, int x, int y, char[] buf, int len) {
        g2d.setColor(Application.colorShadeShape);
        g2d.drawChars(buf, 0, len, x + 1, y + 1);
    }
}
