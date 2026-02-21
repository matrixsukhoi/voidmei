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
public class CompassGauge extends AbstractHUDComponent {

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

    // Coordinate system mode (logic inverted to match AttitudeIndicatorGauge):
    //   false (config OFF, default) = 离体/Inertial: pointer rotates, North fixed at top
    //   true  (config ON)           = 随体/Body-fixed: aircraft fixed at top, North rotates
    private boolean inertialMode = false;

    // Pre-allocated arrays for triangle drawing (zero-GC)
    private int[] triangleXPoints = new int[3];
    private int[] triangleYPoints = new int[3];

    // Thin stroke for triangle border (1 pixel)
    private static final BasicStroke THIN_STROKE = new BasicStroke(1);

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

    /**
     * Sets the coordinate system mode for the compass.
     * Note: Logic is inverted to match AttitudeIndicatorGauge behavior.
     * @param inertial false (default) = 离体/Inertial mode: pointer rotates to show heading, North fixed at top;
     *                 true = 随体/Body-fixed mode: aircraft symbol fixed at top, North triangle rotates
     */
    public void setInertialMode(boolean inertial) {
        this.inertialMode = inertial;
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
        int centerX = x + r;
        int centerY = y + r;

        // Layer 1 (bottom): Draw North triangle first
        // Logic inverted to match AttitudeIndicatorGauge behavior
        if (inertialMode) {
            // Config ON: Body-fixed mode - rotating North triangle
            drawNorthTriangle(g2d, centerX, centerY, r, -compassRads);
        } else {
            // Config OFF: Inertial mode - fixed North triangle at 12 o'clock
            drawNorthTriangle(g2d, centerX, centerY, r, 0);
        }

        // Layer 2: Draw the compass circle
        g2d.setStroke(outBs);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawOval(x, y, r + r, r + r);

        g2d.setStroke(inBs);
        g2d.setColor(Application.colorNum);
        g2d.drawOval(x, y, r + r, r + r);

        // Layer 3 (top): Draw heading pointer or aircraft line
        // Logic inverted to match AttitudeIndicatorGauge behavior
        if (inertialMode) {
            // Config ON: Body-fixed mode - aircraft symbol fixed at 12 o'clock
            int fixedTipY = centerY - (int) (0.618 * r);
            int fixedEndY = centerY - (int) (1.3 * r);

            g2d.setStroke(outBs);
            g2d.setColor(Application.colorShadeShape);
            g2d.drawLine(centerX, fixedTipY, centerX, fixedEndY);

            g2d.setStroke(inBs);
            g2d.setColor(Application.colorNum);
            g2d.drawLine(centerX, fixedTipY, centerX, fixedEndY);

        } else {
            // Config OFF: Inertial mode - pointer rotates to show heading
            int tipX = centerX + (int) (0.618 * r * Math.sin(compassRads));
            int tipY = centerY - (int) (0.618 * r * Math.cos(compassRads));
            int endX = centerX + compassDx;
            int endY = centerY - compassDy;

            g2d.setStroke(outBs);
            g2d.setColor(Application.colorShadeShape);
            g2d.drawLine(tipX, tipY, endX, endY);

            g2d.setStroke(inBs);
            g2d.setColor(Application.colorNum);
            g2d.drawLine(tipX, tipY, endX, endY);
        }

        // Draw text labels (same for both modes)
        if (fontSmall != null) {
            UIBaseElements.drawStringShade(g2d, x + lineWidth + 3, y + HUDFontSize - (r - HUDFontSize) / 2, 1,
                    lineCompass, fontSmall);
            UIBaseElements.drawStringShade(g2d, x + lineWidth + 3, y + r + HUDFontSizeSmall / 2 + HUDFontSize, 1,
                    lineLoc,
                    fontSmall);
        }
    }

    /**
     * Draws a north indicator triangle at the specified angle from center.
     * The triangle points outward away from the center.
     *
     * @param g2d     Graphics context
     * @param centerX Center X of the compass
     * @param centerY Center Y of the compass
     * @param radius  Radius of the compass
     * @param angleRads Angle in radians (0 = 12 o'clock / North)
     */
    private void drawNorthTriangle(Graphics2D g2d, int centerX, int centerY, int radius, double angleRads) {
        // Triangle size: height is about half the aircraft line length
        int triangleHeight = (int) (radius * 0.35);
        int triangleHalfBase = (int) (radius * 0.30);

        // Triangle tip points outward, base sits on the circle edge
        // Tip is outside the circle
        double tipDist = radius + triangleHeight;
        int tipX = centerX + (int) (tipDist * Math.sin(angleRads));
        int tipY = centerY - (int) (tipDist * Math.cos(angleRads));

        // Base center sits on the circle edge
        int baseX = centerX + (int) (radius * Math.sin(angleRads));
        int baseY = centerY - (int) (radius * Math.cos(angleRads));

        // Perpendicular direction (tangent to circle): use (cos(θ), sin(θ))
        int corner1X = baseX + (int) (triangleHalfBase * Math.cos(angleRads));
        int corner1Y = baseY + (int) (triangleHalfBase * Math.sin(angleRads));
        int corner2X = baseX - (int) (triangleHalfBase * Math.cos(angleRads));
        int corner2Y = baseY - (int) (triangleHalfBase * Math.sin(angleRads));

        // Fill triangle arrays (reuse pre-allocated arrays for zero-GC)
        triangleXPoints[0] = tipX;
        triangleXPoints[1] = corner1X;
        triangleXPoints[2] = corner2X;

        triangleYPoints[0] = tipY;
        triangleYPoints[1] = corner1Y;
        triangleYPoints[2] = corner2Y;

        // Draw filled triangle using unit color
        g2d.setColor(Application.colorUnit);
        g2d.fillPolygon(triangleXPoints, triangleYPoints, 3);

        // Draw thin border on top using shade color
        g2d.setColor(Application.colorShadeShape);
        g2d.setStroke(THIN_STROKE);
        g2d.drawPolygon(triangleXPoints, triangleYPoints, 3);
    }
}
