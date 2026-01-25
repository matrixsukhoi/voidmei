package ui.component;

import java.awt.BasicStroke;
import java.awt.Dimension;
import java.awt.Font;
import java.awt.Graphics2D;

import prog.Application;
import ui.UIBaseElements;

/**
 * High-performance Flap Angle Bar component.
 * Displays a tri-color bar showing current flap angle vs allowable.
 */
public class FlapAngleBar extends AbstractHUDComponent {

    // Cached stroke
    private BasicStroke tickStroke;
    private int cachedStrokeWidth = -1;

    // Styling Context
    private int totalWidth;
    private int barHeight;
    private Font font;

    // State
    private double currentAngle;
    private double maxSafeAngle;
    private String displayText;

    // Tick marks at these positions
    private static final int[] TICK_POSITIONS = { 20, 33, 60, 100 };
    private static final int MAX_SCALE = 125;

    public FlapAngleBar() {
        this.currentAngle = 0;
        this.maxSafeAngle = 100;
        this.displayText = "  0/100";
    }

    @Override
    public String getId() {
        return "bar.flaps";
    }

    @Override
    public Dimension getPreferredSize() {
        int w = totalWidth > 0 ? totalWidth : 200;
        int h = (font != null ? font.getSize() : 12) + barHeight + 5;
        return new Dimension(w, h);
    }

    public void setStyleContext(int totalWidth, int barHeight, Font font) {
        this.totalWidth = totalWidth;
        this.barHeight = barHeight;
        this.font = font;
    }

    @Override
    public void update(Object data) {
        if (data instanceof Object[]) {
            Object[] params = (Object[]) data;
            if (params.length >= 2) {
                this.currentAngle = (Double) params[0];
                this.maxSafeAngle = (Double) params[1];
                this.displayText = String.format("%3.0f/%3.0f", currentAngle, maxSafeAngle);
            }
        }
    }

    @Override
    public void onDataUpdate(ui.overlay.model.HUDData data) {
        if (data == null)
            return;

        this.currentAngle = data.flaps;
        this.maxSafeAngle = data.flapAllowAngle;
        this.displayText = String.format("%3.0f/%3.0f", currentAngle, maxSafeAngle);
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        if (font == null)
            return;

        // Adjust Y to treat input (x,y) as Top-Left
        // 1. Text Baseline
        int ascent = g2d.getFontMetrics(font).getAscent();
        int textY = y + ascent;

        // Draw text
        int strWidth = g2d.getFontMetrics(font).stringWidth(displayText);
        int strX = x + (totalWidth - strWidth) / 2;
        UIBaseElements.__drawStringShade(g2d, strX, textY, 1, displayText, font, Application.colorNum);

        // Bar position below text (using font size as line height approximation)
        int barY = y + font.getSize() + 2;

        // Calculate section widths
        int blueWidth = (int) (currentAngle * totalWidth / MAX_SCALE);
        int greenWidth = (int) ((maxSafeAngle - currentAngle) * totalWidth / MAX_SCALE);
        int redWidth = totalWidth - blueWidth - greenWidth;

        // Ensure non-negative
        blueWidth = Math.max(0, blueWidth);
        greenWidth = Math.max(0, greenWidth);
        redWidth = Math.max(0, redWidth);

        // Cache stroke
        if (cachedStrokeWidth != 2) {
            tickStroke = new BasicStroke(2);
            cachedStrokeWidth = 2;
        }

        // Draw tick marks
        g2d.setColor(Application.colorLabel);
        g2d.setStroke(tickStroke);

        for (int tick : TICK_POSITIONS) {
            int tx = x + (int) (tick * totalWidth / MAX_SCALE);
            int ext = (tick == 100) ? barHeight : barHeight / 4;
            g2d.drawLine(tx, barY - ext - 2, tx, barY); // Draw ticks strictly above barY? or attached?
            // Original: barY - ext - 4 to barY.
            // Let's keep it attached to the bar top.
        }

        // Draw blue section (0 → currentAngle)
        if (blueWidth > 0) {
            g2d.setColor(Application.colorShadeShape);
            g2d.fillRect(x, barY, blueWidth, barHeight);
        }

        // Draw green section (currentAngle → maxSafeAngle)
        if (greenWidth > 0) {
            g2d.setColor(Application.colorNum);
            g2d.fillRect(x + blueWidth, barY, greenWidth, barHeight);
        }

        // Draw red section (maxSafeAngle → 125)
        if (redWidth > 0) {
            g2d.setColor(Application.colorWarning);
            g2d.fillRect(x + blueWidth + greenWidth, barY, redWidth, barHeight);
        }
    }
}
