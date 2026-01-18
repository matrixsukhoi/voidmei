package ui.component;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Dimension;
import java.awt.Graphics2D;
import java.awt.Image;

import prog.Application;

/**
 * High-performance Crosshair/Gunsight component.
 * Extracted from MinimalHUD for reusability.
 */
public class CrosshairGauge implements HUDComponent {

    // Cached strokes for performance
    private BasicStroke outerStroke;
    private BasicStroke innerStroke;
    private int cachedStrokeWidth = -1;

    // Styling Context
    private int width;
    private boolean useTexture;
    private Image texture;
    private int crossWidthVario;

    // Colors
    private Color shadowColor = new Color(0, 0, 0, 75);
    private Color foregroundColor = new Color(255, 215, 8, 255);

    public CrosshairGauge() {
    }

    @Override
    public String getId() {
        return "gauge.crosshair";
    }

    @Override
    public Dimension getPreferredSize() {
        if (useTexture) {
            return new Dimension(crossWidthVario * 2, crossWidthVario * 2);
        }
        return new Dimension(width, width);
    }

    public void setStyleContext(int width) {
        this.width = width;
        this.useTexture = false;
    }

    public void setTextureStyle(boolean useTexture, Image texture, int crossWidthVario) {
        this.useTexture = useTexture;
        this.texture = texture;
        this.crossWidthVario = crossWidthVario;
    }

    /**
     * Set custom colors.
     */
    public void setColors(Color shadow, Color foreground) {
        this.shadowColor = shadow;
        this.foregroundColor = foreground;
    }

    @Override
    public void update(Object data) {
    }

    @Override
    public void draw(Graphics2D g2d, int centerX, int centerY) {
        if (useTexture && texture != null) {
            int x = centerX - crossWidthVario;
            int y = centerY - crossWidthVario;
            Application.debugPrint("Component: Crosshair (Texture), x=" + centerX + ", y=" + centerY);
            g2d.drawImage(texture, x, y, crossWidthVario * 2, crossWidthVario * 2, null);
            return;
        }

        Application.debugPrint("Component: Crosshair (Vector), x=" + centerX + ", y=" + centerY);

        // Length multiplier for lines
        int l = 4;

        // Cache strokes
        int strokeWidth = Math.max(width / 30, 2);
        if (strokeWidth != cachedStrokeWidth) {
            outerStroke = new BasicStroke(strokeWidth + 1, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            innerStroke = new BasicStroke(strokeWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            cachedStrokeWidth = strokeWidth;
        }

        int halfWidth = width / 2;
        int quarterWidth = width / 4;
        int lineLength = width * l / 4;

        // Draw shadow layer
        g2d.setStroke(outerStroke);
        g2d.setColor(shadowColor);

        drawCrosshairShape(g2d, centerX, centerY, halfWidth, quarterWidth, lineLength);

        // Draw foreground layer
        g2d.setStroke(innerStroke);
        g2d.setColor(foregroundColor);

        drawCrosshairShape(g2d, centerX, centerY, halfWidth, quarterWidth, lineLength);
    }

    private void drawCrosshairShape(Graphics2D g2d, int cx, int cy, int halfW, int quarterW, int lineLen) {
        // Circle
        g2d.drawOval(cx - halfW, cy - halfW, halfW * 2, halfW * 2);

        // Horizontal lines (left and right of center)
        g2d.drawLine(cx - lineLen, cy, cx - quarterW, cy);
        g2d.drawLine(cx + quarterW, cy, cx + lineLen, cy);

        // Vertical lines (top and bottom of center)
        g2d.drawLine(cx, cy - lineLen, cx, cy - quarterW);
        g2d.drawLine(cx, cy + quarterW, cx, cy + lineLen);
    }
}
