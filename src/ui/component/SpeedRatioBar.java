package ui.component;

import java.awt.BasicStroke;
import java.awt.Graphics2D;

import prog.Application;
import ui.overlay.model.HUDData;

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

        g2d.setStroke(new BasicStroke(1));

        // 1. Aileron Lock Tick (Left) - drawn BEFORE bar so it gets partially covered
        if (aileronLockRatio > 0 && aileronLockRatio < 1.0) {
            int lockY = y + height - Math.round((float) (height * aileronLockRatio));
            int tX = x - 6;
            int tY = lockY - 2;
            int tW = 8;
            int tH = 4;

            g2d.setColor(Application.colorNum);
            g2d.fillRect(tX, tY, tW, tH);
            g2d.setColor(Application.colorShadeShape);
            g2d.drawRect(tX, tY, tW, tH);
        }

        // 2. Rudder Lock Tick (Right) - drawn BEFORE bar so it gets partially covered
        if (rudderLockRatio > 0 && rudderLockRatio < 1.0) {
            int lockY = y + height - Math.round((float) (height * rudderLockRatio));
            int tX = x + width - 2;
            int tY = lockY - 2;
            int tW = 8;
            int tH = 4;

            g2d.setColor(Application.colorNum);
            g2d.fillRect(tX, tY, tW, tH);
            g2d.setColor(Application.colorShadeShape);
            g2d.drawRect(tX, tY, tW, tH);
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

        // 6. Unit Mach Red Line - overlays bar, extends beyond bar edges
        if (unitMachRatio > 0 && unitMachRatio < 1.0) {
            int machY = y + height - Math.round((float) (height * unitMachRatio));
            g2d.setColor(Application.colorWarning);
            // Extends 4px beyond bar on each side
            // g2d.fillRect(x - 4, machY - 2, width + 8, 4);
            g2d.fillRect(x, machY - 2, width, 2);
        }

        // 6. Unit Mach Red Line - overlays bar, extends beyond bar edges
        if (unitMachRatio > 0 && unitMachRatio < 1.0) {
            int machY = y + height - Math.round((float) (height * unitMachRatio));
            g2d.setColor(Application.colorWarning);
            // Extends 4px beyond bar on each side
            g2d.fillRect(x - 4, machY - 2, width + 8, 4);
        }

        // 7. Frame/Border
        g2d.setStroke(new BasicStroke(1));
        g2d.setColor(Application.colorShadeShape);
        g2d.drawRect(x, y, width, height);
    }

    private double clamp(double v) {
        if (v < 0)
            return 0;
        if (v > 1)
            return 1;
        return v;
    }
}
