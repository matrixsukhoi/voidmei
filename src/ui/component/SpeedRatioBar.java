package ui.component;

import java.awt.BasicStroke;
import java.awt.Color;
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

    // Strokes
    private BasicStroke tickStroke = new BasicStroke(4);

    private Color colorSafe = new Color(0, 255, 0); // Green
    private Color colorRemaining = new Color(0, 100, 255); // Blue
    private Color colorDanger = new Color(255, 0, 0); // Red
    private Color colorLock = new Color(0, 200, 255); // Light Blue for locks

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

        // 0. Ticks (Draw FIRST so they are behind the bar fills)
        // No stroke needed for rects, but reset default
        g2d.setStroke(new BasicStroke(1));

        // Unit Mach (Red Line)
        if (unitMachRatio > 0 && unitMachRatio <= 1.0) {
            int machY = y + height - (int) (height * unitMachRatio);
            g2d.setColor(colorDanger);
            // Height 4px centered
            g2d.fillRect(x, machY - 2, width, 4);
            // Border
            g2d.setColor(Application.colorShadeShape);
            g2d.drawRect(x, machY - 2, width, 4);
        }

        // Lock Ratios (Blue Ticks)
        // Aileron (Left Tick)
        if (aileronLockRatio > 0 && aileronLockRatio <= 1.0) {
            int lockY = y + height - (int) (height * aileronLockRatio);
            // Draw tick on Left Edge: x-6 to x+2 -> Width = 8
            int tX = x - 6;
            int tY = lockY - 2;
            int tW = 8;
            int tH = 4;

            g2d.setColor(colorLock);
            g2d.fillRect(tX, tY, tW, tH);
            g2d.setColor(Application.colorShadeShape);
            g2d.drawRect(tX, tY, tW, tH);
        }

        // Rudder (Right Tick)
        if (rudderLockRatio > 0 && rudderLockRatio <= 1.0) {
            int lockY = y + height - (int) (height * rudderLockRatio);
            // Draw tick on Right Edge: x + width - 2 to x + width + 6
            // Start x + width - 2. Width = 8.
            int tX = x + width - 2;
            int tY = lockY - 2;
            int tW = 8;
            int tH = 4;

            g2d.setColor(colorLock);
            g2d.fillRect(tX, tY, tW, tH);
            g2d.setColor(Application.colorShadeShape);
            g2d.drawRect(tX, tY, tW, tH);
        }

        // 1. Background (Blue - Remaining part)
        // Cleanest: Fill Full Blue first (Background)
        g2d.setColor(colorRemaining);
        g2d.fillRect(x, y, width, height);

        // 2. Green Bar (Current Speed)
        // From Bottom (y+height) up to Ratio.
        int greenH = (int) (height * clamp(speedRatio));
        if (greenH > 0) {
            g2d.setColor(colorSafe);
            // x, y_top, w, h
            // y_top = (y+height) - greenH
            g2d.fillRect(x, y + height - greenH, width, greenH);
        }

        // 3. Red Zone (Stall) -> 1/3 Width, Bottom Left?
        int stallH = (int) (height * clamp(stallRatio));
        if (stallH > 0) {
            g2d.setColor(colorDanger);
            int stallW = width / 3;
            if (stallW < 2)
                stallW = 2; // Min width
            // Draw at Bottom-Left (or Center?)
            // Usually stall strip is on the side. Let's place it Left.
            g2d.fillRect(x, y + height - stallH, stallW, stallH);
        }

        // 4. Frame/Border (Optional, but looks better)
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
