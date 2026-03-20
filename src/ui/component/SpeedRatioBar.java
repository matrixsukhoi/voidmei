package ui.component;

import java.awt.Font;
import java.awt.Graphics2D;
import java.awt.Stroke;

import prog.Application;
import ui.overlay.model.HUDData;
import ui.util.GraphicsUtil;

/**
 * A specialized vertical bar for displaying Speed Ratio, Stall Speed, and
 * Limits.
 * 
 * Visuals:
 * - Full Height 0.0 to 1.0 represents 0 to Limit (VNE/MNE).
 * - Green Bar: Current Speed Ratio (Bottom to Value).
 * - Blue Bar: Remaining Range (Value to Top).
 * - Red Zone: Stall Ratio (Bottom to Value, 1/3 Width).
 * - Red Tick: Unit Mach Limit Ratio.
 * - Blue Ticks: Aileron/Rudder Lock Ratios (Left/Right).
 */
public class SpeedRatioBar extends AbstractHUDComponent {

    private int width = 10;
    private int height = 100;

    // Cached stroke for ticks
    private Stroke tickStroke;
    private int cachedTickWidth = -1;

    // Font for tick label
    private Font tickFont;

    // Fixed-width template for value display (prevents layout jitter)
    private static final String VALUE_TEMPLATE = "888";

    // Data State
    private double speedRatio = 0;
    private double stallRatio = 0;
    private double unitMachRatio = 0;
    private double aileronLockRatio = 0;
    private double rudderLockRatio = 0;

    public SpeedRatioBar() {
    }

    @Override
    public String getId() {
        return "bar.speed_ratio";
    }

    @Override
    public java.awt.Dimension getPreferredSize() {
        return new java.awt.Dimension(width, height);
    }

    public void setSize(int width, int height) {
        this.width = width;
        this.height = height;
    }

    public void setStyleContext(int width, int height, Font tickFont) {
        this.width = width;
        this.height = height;
        this.tickFont = tickFont;
    }

    @Override
    public void onDataUpdate(HUDData data) {
        if (data == null)
            return;
        this.speedRatio = data.speedBar_speedRatio;
        this.stallRatio = data.speedBar_stallRatio;
        this.unitMachRatio = data.speedBar_unitMachLimitRatio;
        this.aileronLockRatio = data.speedBar_aileronLockRatio;
        this.rudderLockRatio = data.speedBar_rudderLockRatio;
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        // Draw Vertical Bar
        // y is Top-Left. Height goes down.
        // Value 0 is at Bottom (y + height).
        // Value 1 is at Top (y).

        // Cache tick stroke (2px width to match FlapAngleBar style)
        // Use precise stroke to prevent line extending beyond specified endpoints
        if (cachedTickWidth != 2) {
            tickStroke = GraphicsUtil.createPreciseStroke(2);
            cachedTickWidth = 2;
        }

        // 1. Aileron Lock Tick (Left) - drawn BEFORE bar so it gets partially covered
        if (aileronLockRatio > 0 && aileronLockRatio < 1.0) {
            int lockY = y + height - Math.round((float) (height * aileronLockRatio));
            g2d.setStroke(tickStroke);
            g2d.setColor(Application.colorNum);
            // Line from left side extending into vbar
            g2d.drawLine(x - 4, lockY, x + width / 2, lockY);
        }

        // 2. Rudder Lock Tick (Right) - drawn BEFORE bar so it gets partially covered
        if (rudderLockRatio > 0 && rudderLockRatio < 1.0) {
            int lockY = y + height - Math.round((float) (height * rudderLockRatio));
            g2d.setStroke(tickStroke);
            g2d.setColor(Application.colorNum);
            // Line from vbar center extending to right side
            g2d.drawLine(x + width / 2, lockY, x + width + 4, lockY);
        }

        // 3. Background (Blue - Remaining part from speedRatio to top)
        g2d.setColor(Application.colorNum);
        g2d.fillRect(x, y, width, height);

        // 4. Green Bar (Current Speed) - from bottom to speedRatio
        int greenH = Math.round((float) (height * clamp(speedRatio)));
        if (greenH > 0) {
            g2d.setColor(Application.colorShadeShape);
            g2d.fillRect(x, y + height - greenH, width, greenH);
        }

        // 4.5. Speed Ratio Tick (Left) - shows current speedRatio value
        {
            int tickY = y + height - Math.round((float) (height * clamp(speedRatio)));

            // Calculate fixed template width for consistent layout
            int templateWidth = (tickFont != null) ? g2d.getFontMetrics(tickFont).stringWidth(VALUE_TEMPLATE) : 0;
            int tickExtend = 4;
            int labelSpacing = 2;

            // Tick spans from text area left edge to vbar right edge
            int tickStartX = x - tickExtend - labelSpacing - templateWidth;
            int tickWidth = templateWidth + labelSpacing + tickExtend + width;

            // Draw tick
            g2d.setStroke(tickStroke);
            g2d.setColor(Application.colorNum);
            g2d.drawLine(tickStartX, tickY, tickStartX + tickWidth - 1, tickY);

            // Draw value text (above tick, right-aligned to tick left edge)
            if (tickFont != null) {
                int displayValue = (int) Math.round(speedRatio * 100);
                String valueStr = String.valueOf(displayValue);

                // Calculate actual text width for right-alignment
                int actualTextWidth = g2d.getFontMetrics(tickFont).stringWidth(valueStr);
                int textRightEdge = x - tickExtend - labelSpacing;  // tick left edge with spacing
                int textX = textRightEdge - actualTextWidth;
                int textY = tickY - 3;  // above the tick

                // Draw text with shadow
                g2d.setFont(tickFont);
                g2d.setColor(Application.colorShadeShape);
                g2d.drawString(valueStr, textX + 1, textY + 1);
                g2d.setColor(Application.colorNum);
                g2d.drawString(valueStr, textX, textY);
            }
        }

        // 5. Red Zone (Stall) - right side 1/2 width, overlays bar
        int stallH = Math.round((float) (height * clamp(stallRatio)));
        if (stallH > 0) {
            g2d.setColor(Application.colorWarning);
            int stallW = width / 2;
            if (stallW < 2)
                stallW = 2;
            // Draw at Bottom-Right (right side 1/2)
            g2d.fillRect(x + width - stallW, y + height - stallH, stallW, stallH);
        }

        // 6. Unit Mach Red Line
        if (unitMachRatio > 0 && unitMachRatio < 1.0) {
            int machY = y + height - Math.round((float) (height * unitMachRatio));
            g2d.setStroke(tickStroke);
            g2d.setColor(Application.colorWarning);
            g2d.drawLine(x, machY, x + width, machY);
        }

        // No border - matches FlapAngleBar style
    }

    private double clamp(double v) {
        if (v < 0)
            return 0;
        if (v > 1)
            return 1;
        return v;
    }
}
