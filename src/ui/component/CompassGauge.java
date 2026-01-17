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

    public void draw(Graphics2D g2d, int x, int y, int lineWidth, java.awt.Font fontSmall, java.awt.Font fontBig) {
        // Cache strokes based on lineWidth
        if (lineWidth != cachedStrokeWidth) {
            outBs = new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            inBs = new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            cachedStrokeWidth = lineWidth;
        }

        int r = radius;
        int d = r * 2;

        // Draw Shadow/Outline
        g2d.setStroke(outBs);
        g2d.setColor(Application.colorShadeShape);

        // Line vector
        int x2 = x + r + compassDx;
        int y2 = y + r - compassDy;

        // Needle tip position logic from MinimalHUD:
        // x1 = kx + r + (int) (0.618 * r * Math.sin(compassRads))
        // y1 = n + yOffset + r - (int) (0.618 * r * Math.cos(compassRads))
        // target = kx + r + compassDx, n + yOffset + r - compassDy

        int tipX = x + r + (int) (0.618 * r * Math.sin(compassRads));
        int tipY = y + r - (int) (0.618 * r * Math.cos(compassRads));

        g2d.drawLine(tipX, tipY, x2, y2);
        g2d.drawOval(x, y, d, d);

        // Draw Foreground
        g2d.setStroke(inBs);
        g2d.setColor(Application.colorNum);

        g2d.drawLine(tipX, tipY, x2, y2);
        g2d.drawOval(x, y, d, d);

        // Draw Text
        // UIBaseElements.drawStringShade(g, kx + lineWidth + 3, n + y - (r -
        // HUDFontsize) / 2, 1, lineCompass, drawFontSmall);
        // UIBaseElements.drawStringShade(g, kx + lineWidth + 3, n + y + r +
        // HUDFontSizeSmall / 2, 1, lineLoc, drawFontSmall);

        // Use local simplistic drawStringShade or delegation?
        // Let's use delegation for text consistency or extract that too.
        // For performance, we can stick to UIBaseElements or simpler logic.
        // Given we are inside a component, let's keep it self-contained if possible,
        // but UIBaseElements.drawStringShade is effectively stateless (except
        // FontRenderContext creation).

        // Adjusted coordinates based on MinimalHUD logic (approximate relative to 'x,
        // y')
        // MinimalHUD: kx = x, n + yOffset = y
        // Text 1: kx + lineWidth + 3, n + y - (r - HUDFontsize) / 2
        // -> x + lineWidth + 3, y - (r - fontBig.getSize())/2 ?? logic seems slightly
        // off in original,
        // but let's replicate logic relative to center/top.

        int fontSize = fontBig.getSize();
        int fontSmallSize = fontSmall.getSize();

        // Compass Value (Top Center-ish or Top Left?)
        // MinimalHUD: n + y - (r - HUDFontsize)/2 => y - (r - fontSize)/2
        // Actually, let's just place it nicely.

        drawTextShaded(g2d, x + lineWidth + 3, y + r - (r / 2), lineCompass, fontSmall);
        drawTextShaded(g2d, x + lineWidth + 3, y + r + r / 2 + fontSmallSize, lineLoc, fontSmall);
    }

    private void drawTextShaded(Graphics2D g2d, int x, int y, String s, java.awt.Font f) {
        g2d.setFont(f);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawString(s, x + 1, y + 1);
        g2d.setColor(Application.colorNum); // Default color
        g2d.drawString(s, x, y);
    }
}
