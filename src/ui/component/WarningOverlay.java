package ui.component;

import java.awt.BasicStroke;
import java.awt.Graphics2D;

import prog.Application;

/**
 * High-performance Warning Overlay component.
 * Draws a blinking X overlay for critical warnings.
 * 
 * Performance notes:
 * - Strokes are cached
 * - Blink state managed externally for frame timing control
 */
public class WarningOverlay {

    // Cached strokes
    private BasicStroke outerStroke;
    private BasicStroke innerStroke;
    private int cachedWidth = -1;

    public WarningOverlay() {
    }

    /**
     * Draw the warning X overlay.
     * 
     * @param g2d        Graphics context
     * @param x          Left position
     * @param y          Top position
     * @param width      Overlay width
     * @param height     Overlay height
     * @param isBlinkOff If true, skip drawing (for blink effect)
     */
    public void draw(Graphics2D g2d, int x, int y, int width, int height, boolean isBlinkOff) {
        if (isBlinkOff) {
            return; // Blink is in "off" phase
        }

        // Cache strokes based on width
        if (width != cachedWidth) {
            outerStroke = new BasicStroke(5, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            innerStroke = new BasicStroke(3, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            cachedWidth = width;
        }

        // Draw shadow X
        g2d.setStroke(outerStroke);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawLine(x + 2, y + 2, x + width - 2, y + height - 2);
        g2d.drawLine(x + width - 2, y + 2, x + 2, y + height - 2);

        // Draw foreground X
        g2d.setStroke(innerStroke);
        g2d.setColor(Application.colorNum);
        g2d.drawLine(x + 1, y + 1, x + width - 1, y + height - 1);
        g2d.drawLine(x + width - 1, y + 1, x + 1, y + height - 1);
    }
}
