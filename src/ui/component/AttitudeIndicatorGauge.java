package ui.component;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Dimension;
import java.awt.Font;
import java.awt.Graphics2D;
import java.awt.geom.AffineTransform;

import prog.Application;
import ui.UIBaseElements;

/**
 * Reusable Attitude Indicator component.
 */
public class AttitudeIndicatorGauge extends AbstractHUDComponent {

    // Cached strokes
    private BasicStroke strokeThick;
    private BasicStroke strokeThin;
    private int cachedLineWidth = -1;

    // Styling Context
    private int compassDiameter;
    private int compassRadius;
    private int compassInnerMarkRadius;
    private int lineWidth;
    private int halfLine;
    private Font font;

    // State
    private double pitch; // In pixels (already scaled)
    private double rollDeg;
    private int aosX;
    private String sAttitude;
    private int roundHorizon;
    private String sSideslip;       // Sideslip angle display text
    private int roundSlip;          // Rounded sideslip angle (for color logic)
    private boolean pitchValid;     // Whether pitch data is valid from API

    // Coordinate system mode: false = body-fixed (pilot view), true = earth-fixed (external view)
    private boolean inertialMode;

    public AttitudeIndicatorGauge() {
        this.sAttitude = "";
        this.sSideslip = "";
        this.inertialMode = false;
    }

    /**
     * Sets the coordinate system mode for the attitude indicator.
     * @param inertial true for earth-fixed (inertial) mode, false for body-fixed mode
     */
    public void setInertialMode(boolean inertial) {
        this.inertialMode = inertial;
    }

    @Override
    public String getId() {
        return "gauge.attitude";
    }

    @Override
    public Dimension getPreferredSize() {
        return new Dimension(compassDiameter, compassDiameter);
    }

    public void setStyleContext(int compassDiameter, int compassRadius, int compassInnerMarkRadius, int lineWidth,
            int halfLine, Font font) {
        this.compassDiameter = compassDiameter;
        this.compassRadius = compassRadius;
        this.compassInnerMarkRadius = compassInnerMarkRadius;
        this.lineWidth = lineWidth;
        this.halfLine = halfLine;
        this.font = font;
    }

    @Override
    public void update(Object data) {
        if (data instanceof Object[]) {
            Object[] params = (Object[]) data;
            if (params.length >= 5) {
                this.pitch = (Double) params[0];
                this.rollDeg = (Double) params[1];
                this.aosX = (Integer) params[2];
                this.sAttitude = (String) params[3];
                this.roundHorizon = (Integer) params[4];
            }
        }
    }

    public void update(double pitch, double rollDeg, int aosX, String sAttitude, int roundHorizon) {
        this.pitch = pitch;
        this.rollDeg = rollDeg;
        this.aosX = aosX;
        this.sAttitude = sAttitude;
        this.roundHorizon = roundHorizon;
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {

        updateStrokes(lineWidth);

        // Convert Top-Left input (x, y) to Center (centerX, centerY)
        // PreferredSize is compassDiameter x compassDiameter
        // So radius = compassDiameter / 2
        int radius = compassDiameter / 2;
        int centerX = x + radius;
        int centerY = y + radius;

        double rollDegRad = Math.toRadians(rollDeg);

        // Coordinate system mode determines sign conventions:
        // Body-Fixed (inertialMode=false): Pilot's view from cockpit
        //   - Pitch up → horizon moves DOWN (signPitch = -1)
        //   - Roll right → horizon rotates CCW (rollSign = +1)
        //   - Slip right → horizon moves LEFT (signSlip = +1)
        // Earth-Fixed (inertialMode=true): External observer view
        //   - Pitch up → horizon moves UP (signPitch = +1)
        //   - Roll right → horizon rotates CW (rollSign = -1)
        //   - Slip right → horizon moves RIGHT (signSlip = -1)
        int signPitch, signSlip;
        double rollSign;

        if (inertialMode) {
            // Earth-fixed (inertial) coordinate system
            signPitch = +1;   // Pitch up → horizon UP
            signSlip = +1;    // Slip right → horizon LEFT
            rollSign = -1.0;  // Roll right → CW rotation
        } else {
            // Body-fixed coordinate system (default)
            signPitch = -1;   // Pitch up → horizon DOWN
            signSlip = -1;    // Slip right → horizon RIGHT
            rollSign = +1.0;  // Roll right → CCW rotation
        }

        int targetX = centerX + signSlip * aosX * 3 / 2;
        int targetY = centerY + signPitch * (int)(pitch / 2);

        // 绘制地面和牵引线 (Draw ground/traction line)
        g2d.setStroke(strokeThick);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawLine(centerX, centerY, targetX, targetY);

        g2d.setStroke(strokeThin);
        g2d.setColor(Application.colorLabel);
        g2d.drawLine(centerX, centerY, targetX, targetY);

        // 旋转整个组合图形表示横滚角 (Rotate for roll around target)
        AffineTransform oldTransform = g2d.getTransform();
        AffineTransform transform = AffineTransform.getRotateInstance(rollSign * rollDegRad, targetX, targetY);
        g2d.setTransform(transform);

        // Draw Shade (Thick)
        g2d.setStroke(new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
        g2d.setColor(Application.colorShadeShape);
        drawMarks(g2d, targetX, targetY, compassDiameter, compassRadius, compassInnerMarkRadius, halfLine);

        // Draw Main (Thin)
        g2d.setStroke(new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));
        g2d.setColor(Application.colorNum);
        drawMarks(g2d, targetX, targetY, compassDiameter, compassRadius, compassInnerMarkRadius, halfLine);

        g2d.setTransform(oldTransform);

        // 画文字 (Draw Text at target position)
        if (font != null) {
            int gap = font.getSize() / 4;  // Center gap proportional to font size

            // Pitch angle - right side
            Color pitchColor = (roundHorizon >= 0) ? Application.colorNum : Application.colorUnit;
            UIBaseElements.__drawStringShade(g2d, targetX + gap, targetY - 1, 1, sAttitude, font, pitchColor);

            // Sideslip angle - left side (left-aligned with template width)
            if (sSideslip != null && !sSideslip.isEmpty()) {
                int templateWidth = g2d.getFontMetrics(font).stringWidth("888"); // Fixed width for consistent left edge
                Color slipColor = (roundSlip >= 0) ? Application.colorNum : Application.colorUnit;
                UIBaseElements.__drawStringShade(g2d, targetX - gap - templateWidth, targetY - 1, 1, sSideslip, font, slipColor);
            }
        }
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
            strokeThick = new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            strokeThin = new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            cachedLineWidth = lineWidth;
        }
    }

    @Override
    public void onDataUpdate(ui.overlay.model.HUDData data) {
        if (data == null)
            return;

        // Pitch & Roll
        this.pitch = data.pitch;
        this.rollDeg = data.roll;

        // AoS (Slip) Calculation
        // Logic: aosX = (int) (-AoS * slideLimit / 30.0f);
        // slideLimit = 4 * hudFontSize.
        if (font != null) {
            int slideLimit = 4 * font.getSize();
            this.aosX = (int) (-data.slip * slideLimit / 30.0f);
        } else {
            this.aosX = 0;
        }

        // Attitude Text - 仅在数据有效时显示
        this.pitchValid = data.pitchValid;
        if (this.pitchValid) {
            this.roundHorizon = (int) Math.round(data.pitch);
            this.sAttitude = String.format("%3d", Math.abs(this.roundHorizon));
        } else {
            this.roundHorizon = 0;
            this.sAttitude = "";
        }

        // Sideslip Text - 始终显示，保留一位小数
        double slipValue = Math.round(data.slip * 10) / 10.0;
        this.roundSlip = (slipValue >= 0) ? 1 : -1;  // 用于颜色判断，保留符号信息
        this.sSideslip = String.format("%-4.1f", Math.abs(slipValue));
    }
}
