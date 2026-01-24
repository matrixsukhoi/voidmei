package ui.component;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics2D;

import prog.Application;

public class LinearGauge implements HUDComponent {
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

    @Override
    public String getId() {
        return "gauge.linear." + label;
    }

    @Override
    public java.awt.Dimension getPreferredSize() {
        // Estimate dimensions including text
        int textMetric = 30; // Default fallback
        if (fontNumCache != null) {
            textMetric = (int) (fontNumCache.getSize() * 2.0); // Tighter estimate for 3 digits
        }

        if (vertical) {
            // Width = Text + Spacing + Bar
            int estimatedWidth = textMetric + 5 - 5 + thicknessCache;
            return new java.awt.Dimension(estimatedWidth, lengthCache);
        } else {
            // Height = Bar + Text
            int estimatedHeight = thicknessCache + textMetric;
            return new java.awt.Dimension(lengthCache, estimatedHeight);
        }
    }

    private int lengthCache = 100;
    private int thicknessCache = 10;
    private Font fontLabelCache;
    private Font fontNumCache;

    public void setStyleContext(int length, int thickness, Font fontLabel, Font fontNum) {
        this.lengthCache = length;
        this.thicknessCache = thickness;
        this.fontLabelCache = fontLabel;
        this.fontNumCache = fontNum;
    }

    public Color valueColor = null;

    @Override
    public void update(Object data) {
        // Legacy
        if (data instanceof Object[]) {
            Object[] params = (Object[]) data;
            if (params.length >= 2) {
                update((Integer) params[0], (String) params[1]);
            }
        }
    }

    @Override
    public void onDataUpdate(ui.overlay.model.HUDData data) {
        if (data == null)
            return;

        if ("ThrottleBar".equals(this.label)) {
            this.curValue = data.throttle;
            this.displayValue = String.format("%d", data.throttle);
            this.valueColor = data.throttleColor;
        }
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        draw(g2d, x, y, lengthCache, thicknessCache, fontLabelCache, fontNumCache);
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

        Color c = valueColor != null ? valueColor : Application.colorNum;
        Color shade = Application.colorShadeShape;

        g2d.setStroke(borderStroke);

        if (vertical) {
            // (x, y) is the Top-Left of the combined gauge area
            int textWidth = g2d.getFontMetrics(fontNum).stringWidth(displayValue);
            int labelSpacing = 2; // Spacing between label and bar
            int barX = x + textWidth + labelSpacing;

            // Draw the background and fixed bar border/fill
            // Bar should act as 'structure', maybe keep it white? User said "vbar should
            // keep white".
            drawBar(g2d, barX, y, thickness, length, pixVal, shade, Application.colorNum, true);

            // Fix separator position for Top-Left Y
            // Bar goes from y to y+length.
            // Low values are at bottom (y+length).
            // Value height is pixVal.
            // So separator is at (y + length) - pixVal.
            int sepY = y + length - pixVal;

            // Separator Line (moving with value)
            g2d.setStroke(separatorStroke);
            int totalWidth = textWidth + labelSpacing + thickness;
            drawRect(g2d, x, sepY, totalWidth, 3, shade, c, false);

            // Text Number (moving with separator)
            drawTextShaded(g2d, x, sepY - 1, displayValue, fontNum, c);
        } else {
            // Horizontal Bar: width=length, height=thickness
            // Bar should act as 'structure', maybe keep it white? User said "vbar should
            // keep white".
            // So we use Application.colorNum (default white) for the bar fill.
            drawBar(g2d, x, y, length, thickness, pixVal, shade, Application.colorNum, false);

            // Separator Line
            g2d.setStroke(separatorStroke);
            // drawVRect behavior for horizontal separator
            drawRect(g2d, x + pixVal - 2, y, 3, -thickness - 1 * fontNum.getSize(), shade, c, true); // true for
                                                                                                     // "vertical"
                                                                                                     // rect
                                                                                                     // (width <
                                                                                                     // height
                                                                                                     // logic)

            // Text Number
            drawTextShaded(g2d, x + pixVal, y + thickness + 1 * numFontHeight(fontNum), displayValue, fontNum,
                    c);
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
            // Fix: Draw downwards from y (Top-Left)
            // Original was: g2d.drawRect(x, y - h, w - 1, h - 1); (Growing UP)
            // New: g2d.drawRect(x, y, w - 1, h - 1); (Growing DOWN)
            g2d.drawRect(x, y, w - 1, h - 1);
            g2d.setColor(fill);
            // Fill based on val (from Bottom up? or Top down?)
            // Usually throttle is Bottom=0.
            // So if Rect is y to y+h. Bottom is y+h.
            // Fill should be from y+h-val upwards.
            int valH = (val > h) ? h : val;
            if (valH >= 0) {
                // Fill Rect: x+1, (y+h-1) - valH, ...
                // Let's calculate bottom Y: y + h.
                // Fill top Y: (y + h) - valH.
                // Wait, border height is h-1.
                // Inner height is h-2.
                // Bottom Inner Y is y + 1 + (h-2) = y + h - 1.
                // Fill Rect Y = (y + h - 1) - valH.
                // height = valH.
                // Let's verify existing logic: y + 1 - valH. (This assumed y was bottom).

                // Correct for Top-Left Y:
                // Bottom of bar is y + h.
                // We want to fill from bottom up.
                g2d.fillRect(x + 1, y + h - 1 - valH, w - 2, valH);
                // Using w-2, valH (approx).
            }
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
