package ui.component;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics2D;
import java.awt.geom.AffineTransform;

import prog.Application;
import ui.UIBaseElements;

/**
 * Reusable Attitude Indicator component.
 */
public class AttitudeIndicatorGauge {

    // Cached strokes
    private BasicStroke strokeThick;
    private BasicStroke strokeThin;
    private int cachedLineWidth = -1;

    // State
    private double pitch; // In pixels (already scaled)
    private double rollDeg;
    private int aosX;
    private String sAttitude;
    private int roundHorizon;

    /**
     * Update component state.
     * 
     * @param pitch        Pitch value (in pixels, scaled)
     * @param rollDeg      Roll in degrees
     * @param aosX         Angle of Sideslip X offset
     * @param sAttitude    Text to display
     * @param roundHorizon Horizon value for color logic
     */
    public void update(double pitch, double rollDeg, int aosX, String sAttitude, int roundHorizon) {
        this.pitch = pitch;
        this.rollDeg = rollDeg;
        this.aosX = aosX;
        this.sAttitude = sAttitude;
        this.roundHorizon = roundHorizon;
    }

    /**
     * Draw the attitude indicator.
     * 
     * @param g2d                    Graphics context
     * @param centerX                Center X coordinate
     * @param centerY                Center Y coordinate
     * @param compassDiameter        Diameter of the arc
     * @param compassRadius          Radius of the arc
     * @param compassInnerMarkRadius Inner radius for marks
     * @param lineWidth              Line width for strokes
     * @param halfLine               Half line width offset
     * @param font                   Font for text
     */
    public void draw(Graphics2D g2d, int centerX, int centerY, int compassDiameter, int compassRadius,
            int compassInnerMarkRadius, int lineWidth, int halfLine, Font font) {

        Application.debugPrint("Component: AttitudeIndicator, x=" + centerX + ", y=" + centerY);

        updateStrokes(lineWidth);
        double rollDegRad = Math.toRadians(rollDeg);

        // 绘制地面和牵引线 (Draw ground/traction line)
        g2d.setStroke(strokeThick);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawLine(centerX + aosX, centerY + (int) pitch, centerX, centerY);

        g2d.setStroke(strokeThin);
        g2d.setColor(Application.colorLabel);
        g2d.drawLine(centerX + aosX, centerY + (int) pitch, centerX, centerY);

        // 旋转整个组合图形表示横滚角 (Rotate for roll)
        AffineTransform oldTransform = g2d.getTransform();
        AffineTransform transform = AffineTransform.getRotateInstance(-rollDegRad, centerX, centerY);
        g2d.setTransform(transform);

        // Draw Shade (Thick)
        g2d.setStroke(new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
        g2d.setColor(Application.colorShadeShape);
        drawMarks(g2d, centerX, centerY, compassDiameter, compassRadius, compassInnerMarkRadius, halfLine);

        // Draw Main (Thin)
        g2d.setStroke(new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
        g2d.setColor(Application.colorNum);
        drawMarks(g2d, centerX, centerY, compassDiameter, compassRadius, compassInnerMarkRadius, halfLine);

        g2d.setTransform(oldTransform);

        // 画文字 (Draw Text)
        Color textColor = (roundHorizon >= 0) ? Application.colorNum : Application.colorUnit;
        UIBaseElements.__drawStringShade(g2d, centerX, centerY - 1, 1, sAttitude, font, textColor);
    }

    private void drawMarks(Graphics2D g2d, int centerX, int centerY, int compassDiameter, int compassRadius,
            int compassInnerMarkRadius, int halfLine) {
        int hbs = halfLine;
        g2d.drawArc(centerX - compassRadius + hbs, centerY - compassRadius + hbs, compassDiameter, compassDiameter,
                -180, 180);
        g2d.drawLine(centerX + hbs, centerY - compassRadius / 2 + hbs, centerX + hbs,
                centerY - compassInnerMarkRadius + hbs);
        g2d.drawLine(centerX + compassRadius + hbs, centerY + hbs, centerX + compassInnerMarkRadius + hbs,
                centerY + hbs);
        g2d.drawLine(centerX - compassRadius + hbs, centerY + hbs, centerX - compassInnerMarkRadius + hbs,
                centerY + hbs);
    }

    private void updateStrokes(int lineWidth) {
        if (cachedLineWidth != lineWidth) {
            // Using similar logic to what was likely in MinimalHUD or creating reasonable
            // defaults
            strokeThick = new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            strokeThin = new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            cachedLineWidth = lineWidth;
        }
    }
}
