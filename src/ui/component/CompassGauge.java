package ui.component;

import java.awt.BasicStroke;
import java.awt.Dimension;
import java.awt.Graphics2D;

import prog.Application;
import ui.UIBaseElements;

/**
 * High-performance Compass Gauge component.
 * Extracted from MinimalHUD.
 */
public class CompassGauge implements HUDComponent {

    private int radius;
    private BasicStroke outBs;
    private BasicStroke inBs;
    private int cachedStrokeWidth = -1;

    // Static/Style context
    private int lineWidth;
    private int HUDFontSize;
    private int HUDFontSizeSmall;
    private java.awt.Font fontSmall;

    // State
    private float compassRads;
    private int compassDx;
    private int compassDy;
    private String lineCompass;
    private String lineLoc;

    public CompassGauge(int radius) {
        this.radius = radius;
        this.lineCompass = "";
        this.lineLoc = "";
    }

    @Override
    public String getId() {
        return "gauge.compass";
    }

    @Override
    public Dimension getPreferredSize() {
        // Radius * 2 for circle, plus some extra for text and padding
        return new Dimension(radius * 2 + 50, radius * 2 + 20);
    }

    public void setStyleContext(int radius, int lineWidth, int HUDFontSize, int HUDFontSizeSmall,
            java.awt.Font fontSmall) {
        this.radius = radius;
        this.lineWidth = lineWidth;
        this.HUDFontSize = HUDFontSize;
        this.HUDFontSizeSmall = HUDFontSizeSmall;
        this.fontSmall = fontSmall;
    }

    @Override
    public void update(Object data) {
        if (data instanceof Object[]) {
            Object[] params = (Object[]) data;
            if (params.length >= 5) {
                this.compassRads = (Float) params[0];
                this.compassDx = (Integer) params[1];
                this.compassDy = (Integer) params[2];
                this.lineCompass = (String) params[3];
                this.lineLoc = (String) params[4];
            }
        }
    }

    @Override
    public void onDataUpdate(ui.overlay.model.HUDData data) {
        if (data == null)
            return;

        // 1. Heading Logic
        this.compassRads = (float) Math.toRadians(data.heading);
        // Render Logic moved from MinimalHUD:
        // compassDx = (int) ((ctx.roundCompass * 1.3f) * Math.sin(compassRads));
        // Use 'radius' here which is same as 'roundCompass'
        this.compassDx = (int) ((this.radius * 1.3f) * Math.sin(compassRads));
        this.compassDy = (int) ((this.radius * 1.3f) * Math.cos(compassRads));

        this.lineCompass = String.format("%3.0f", data.heading);

        // 2. Location
        this.lineLoc = data.mapGrid;
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {

        // Cache strokes based on lineWidth
        if (lineWidth != cachedStrokeWidth) {
            outBs = new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            inBs = new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            cachedStrokeWidth = lineWidth;
        }

        int r = radius;

        // Draw Shadow/Outline
        g2d.setStroke(outBs);
        g2d.setColor(Application.colorShadeShape);

        int tipX = x + r + (int) (0.618 * r * Math.sin(compassRads));
        int tipY = y + r - (int) (0.618 * r * Math.cos(compassRads));
        int endX = x + r + compassDx;
        int endY = y + r - compassDy;

        g2d.drawLine(tipX, tipY, endX, endY);
        g2d.drawOval(x, y, r + r, r + r);

        // Draw Foreground
        g2d.setStroke(inBs);
        g2d.setColor(Application.colorNum);

        g2d.drawLine(tipX, tipY, endX, endY);
        g2d.drawOval(x, y, r + r, r + r);

        if (fontSmall != null) {
            UIBaseElements.drawStringShade(g2d, x + lineWidth + 3, y + HUDFontSize - (r - HUDFontSize) / 2, 1,
                    lineCompass, fontSmall);
            UIBaseElements.drawStringShade(g2d, x + lineWidth + 3, y + r + HUDFontSizeSmall / 2 + HUDFontSize, 1,
                    lineLoc,
                    fontSmall);
        }
    }
}
