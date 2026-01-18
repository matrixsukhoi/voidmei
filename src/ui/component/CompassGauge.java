package ui.component;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Graphics2D;

import prog.Application;
import ui.UIBaseElements;

/**
 * High-performance Compass Gauge component.
 * Extracted from MinimalHUD.
 */
public class CompassGauge {

    private int radius;
    private BasicStroke outBs;
    private BasicStroke inBs;
    private int cachedStrokeWidth = -1;

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

    public void update(float compassRads, int compassDx, int compassDy, String compassStr, String locStr) {
        this.compassRads = compassRads;
        this.compassDx = compassDx;
        this.compassDy = compassDy;
        this.lineCompass = compassStr;
        this.lineLoc = locStr;
    }

    public void draw(Graphics2D g2d, int x, int y, int lineWidth, int HUDFontSize, int HUDFontSizeSmall,
            java.awt.Font fontSmall) {
        Application.debugPrint("Component: Compass, x=" + x + ", y=" + y);

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

        // Draw Text
        // UIBaseElements.drawStringShade(g, kx + lineWidth + 3, verticalTextOffset + y
        // - (r - HUDFontsize) / 2, 1, lineCompass, drawFontSmall);
        // Note: 'y' passed here corresponds to 'verticalTextOffset + yOffset' in
        // MinimalHUD.
        // But MinimalHUD text Y logic was: 'verticalTextOffset + y - ...'
        // Wait. In MinimalHUD:
        // draw(..., kx, verticalTextOffset + yOffset)
        // passed 'y' is 'verticalTextOffset + yOffset'.
        // text Y logic: 'verticalTextOffset + y - (r - HUDFontsize)/2'.
        // There is a discrepancy between yOffset and y.
        // MinimalHUD.drawTextseries(x, y): y is 'y'. yOffset = y - HUDFontSize.
        // So 'verticalTextOffset + yOffset' is 'verticalTextOffset + y - HUDFontSize'.
        // And text Y is 'verticalTextOffset + y - (r - HUDFontSize)/2'.
        // So Text Y = (Compass Y + HUDFontSize) - (r - HUDFontSize)/2.
        // Let's assume passed 'y' is the top of the compass circle.
        // We will just try to replicate relative positioning.

        // Let's rely on Relative Positioning used in component if simpler.
        // Or passed explicit font sizes.

        // Replicating exact MinimalHUD text placement:
        // Text 1 Y: y + HUDFontSize - (r - HUDFontSize)/2 (Assuming y passed is loop
        // VerticalOffset + yOffset)
        // Let's verify layout.

        UIBaseElements.drawStringShade(g2d, x + lineWidth + 3, y + HUDFontSize - (r - HUDFontSize) / 2, 1, lineCompass,
                fontSmall);
        UIBaseElements.drawStringShade(g2d, x + lineWidth + 3, y + r + HUDFontSizeSmall / 2 + HUDFontSize, 1, lineLoc,
                fontSmall);
    }
}
