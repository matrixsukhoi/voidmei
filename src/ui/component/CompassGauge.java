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
        // Radius * 2 for circle. Removed excess padding to fit layout tightly.
        return new Dimension(radius * 2, radius * 2);
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

        // Convert Top-Left (x, y) to Center implied by drawing logic
        // The original logic seemed to treat (x,y) as Top-Left of the circle's bounding
        // box for drawOval?
        // Original: g2d.drawOval(x, y, r + r, r + r); -> This draws circle at (x,y)
        // width 2r.
        // So (x,y) IS Top-Left of the circle.
        // But the Lines were drawn relative to center?
        // Original: int tipX = x + r + ...
        // x+r IS the center X.
        // So actually CompassGauge WAS respecting Top-Left!
        // Wait, why did the user say "CompassGauge box is too big, lots of empty
        // space"?
        // Maybe PreferredSize is too big?
        // PreferredSize: radius*2 + 50
        // Draw: Oval at (x,y) size 2r.
        // Text is drawn at x + lineWidth + 3.

        // Let's re-read the user complaint: "compassGauge的框特别大, 有很多留空" (Compass box is
        // huge, lots of empty space).
        // This implies getPreferredSize() returns much larger w/h than the actual drawn
        // circle.
        // getPreferredSize returns (2r + 50, 2r + 20).
        // Bounding box (red line) is drawn based on PreferredSize.
        // Circle is drawn at (x,y).
        // So Circle is at Top-Left of the Red Box.
        // The Red Box extends 50px right and 20px down beyond the circle.

        // Fix: Center the circle within the PreferredSize box, or reduce PreferredSize.
        // If we reduce PreferredSize, text might be cut off?
        // Text is drawn INSIDE the circle?
        // y + HUDFontSize - (r - HUDFontSize) / 2 -> This is vertically centered?
        // No, let's look at text drawing.

        // Let's just center the content within the (x,y) box provided by layout.
        // Actually Layout provides (x,y) based on PreferredSize.
        // If we want to fix "Left-Top alignment" visual glitch, we should align content
        // to center of the box if the box is bigger.
        // BUT, better to make the box match the content tightly.
        // Why +50?
        // Let's assume the user just wants standard Top-Left alignment to mean
        // "Top-Left of Content".

        // If I change nothing, x,y is Top-Left of circle. Red box is bigger.
        // User sees Red Box bigger than circle.
        // If I want Red Box to hug circle, I should change getPreferredSize.

        // BUT, looking at MinimalHUD logic, Compass is child of Attitude.
        // If Compass draws at (x,y), and box is big, maybe it pushes neighbors away?
        // No, neighbors are relative.

        // Let's Recalculate PreferredSize in a separate edit if needed.
        // For now, ensure consistency.
        // Since (x,y) is passed, and circle is drawn at (x,y), it IS Top-Left aligned.
        // Problem: Padding.

        // Let's focus on LinearGauge first which has severe offset (throttle).
        // I will assume Compass is "Correctly Top-Left" but "Has Padding".
        // I will address LinearGauge first in this turn.

        int tipX = x + r + (int) (0.618 * r * Math.sin(compassRads));
        int tipY = y + r - (int) (0.618 * r * Math.cos(compassRads));
        int endX = x + r + compassDx;
        int endY = y + r - compassDy;

        g2d.setStroke(outBs);
        g2d.setColor(Application.colorShadeShape);
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
