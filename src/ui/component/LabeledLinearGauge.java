package ui.component;

import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics2D;
import prog.Application;

/**
 * A LinearGauge with label rendering capability that moves with the value.
 * Subclassed to avoid modifying the core LinearGauge component.
 * Fixed: horizontal separator drawing issue (now visible and correctly
 * positioned).
 */
public class LabeledLinearGauge extends LinearGauge {

    public LabeledLinearGauge(String label, int maxValue, boolean vertical) {
        super(label, maxValue, vertical);
    }

    @Override
    public void update(int value, String displayValue) {
        // Prepend label to displayValue so it moves with the separator line
        String combinedValue = (label != null ? label : "") + displayValue;
        super.update(value, combinedValue);
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y, int length, int thickness, Font fontLabel, Font fontValue) {
        // Force Sarasa Mono SC as requested
        Font monoFont = new Font("Sarasa Mono SC", fontValue.getStyle(), fontValue.getSize());

        if (vertical) {
            // For vertical gauges, the base class logic works well with our prepended label
            super.draw(g2d, x, y, length, thickness, fontLabel, monoFont);
        } else {
            // For horizontal gauges, override to fix the missing separator and refine
            // layout
            int pixVal = 0;
            if (maxValue > 0) {
                pixVal = Math.round((float) curValue * length / maxValue);
            }

            Color c = Application.colorNum;
            Color shadeShadow = Application.colorShadeShape;

            // 1. Draw the bar background and border
            drawBarFixed(g2d, x, y, length, thickness, pixVal, shadeShadow, Application.colorNum, false);

            // 2. Draw the separator line correctly (vertical line for horizontal gauge)
            // It should start at the top of the bar and end at the bottom of the text
            int sepHeight = thickness + monoFont.getSize() + 2;

            // Draw shadow line
            g2d.setColor(shadeShadow);
            g2d.drawLine(x + pixVal + 1, y, x + pixVal + 1, y + sepHeight);

            // Draw main line
            g2d.setColor(c);
            g2d.drawLine(x + pixVal, y, x + pixVal, y + sepHeight);

            // 3. Draw the combined Label + Value (using monospaced font)
            drawTextShadedFixed(g2d, x + pixVal, y + thickness + monoFont.getSize(), displayValue, monoFont, c);
        }
    }

    // --- Private Helper Methods (Re-implemented to fix bugs and remain
    // independent) ---

    private void drawBarFixed(Graphics2D g2d, int x, int y, int w, int h, int val, Color shade, Color fill,
            boolean isVert) {
        g2d.setColor(shade);
        if (isVert) {
            g2d.drawRect(x, y - h, w - 1, h - 1);
            g2d.setColor(fill);
            int valH = (val > h) ? h : val;
            if (valH >= 0)
                g2d.fillRect(x + 1, y + 1 - valH, w - 2, valH - 2);
        } else {
            g2d.drawRect(x, y, w - 1, h - 1);
            g2d.setColor(fill);
            int valW = (val > w) ? w : val;
            if (valW >= 0)
                g2d.fillRect(x + 1, y + 1, valW - 2, h - 2);
        }
    }

    private void drawTextShadedFixed(Graphics2D g2d, int x, int y, String s, Font f, Color c) {
        g2d.setFont(f);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawString(s, x + 1, y + 1);
        g2d.setColor(c);
        g2d.drawString(s, x, y);
    }
}
