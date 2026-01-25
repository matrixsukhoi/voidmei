package ui.overlay;

import java.awt.BasicStroke;
import java.awt.Font;
import java.awt.Image;
import java.io.File;
import javax.imageio.ImageIO;

import prog.config.HUDSettings;
import prog.util.Logger;

/**
 * Immutable context object holding pre-calculated metrics and resources for
 * MinimalHUD.
 * Generated from HUDSettings.
 */
public class MinimalHUDContext {

    // --- Layout Dimensions ---
    public final int width;
    public final int height;
    public final int hudFontSize;
    public final int hudFontSizeSmall;
    public final int windowX;
    public final int windowY;

    // --- Component Metrics ---
    public final int crossScale;
    public final int crossX;
    public final int crossY;
    public final int roundCompass;
    public final int rightDraw;
    public final int compassDiameter;
    public final int compassRadius;
    public final int compassInnerMarkRadius;
    public final double aoaLength;

    // --- Styling ---
    public final int lineWidth;
    public final int barWidth;
    public final int halfLine;
    public final Font drawFont;
    public final Font drawFontSmall;
    public final Font drawFontSSmall;
    public final BasicStroke strokeThick;
    public final BasicStroke strokeThin;

    // --- Resources ---
    public final Image crosshairImageScaled;

    private MinimalHUDContext(Builder builder) {
        this.width = builder.width;
        this.height = builder.height;
        this.hudFontSize = builder.hudFontSize;
        this.hudFontSizeSmall = builder.hudFontSizeSmall;
        this.windowX = builder.windowX;
        this.windowY = builder.windowY;

        this.crossScale = builder.crossScale;
        this.crossX = builder.crossX;
        this.crossY = builder.crossY;
        this.roundCompass = builder.roundCompass;
        this.rightDraw = builder.rightDraw;
        this.compassDiameter = builder.compassDiameter;
        this.compassRadius = builder.compassRadius;
        this.compassInnerMarkRadius = builder.compassInnerMarkRadius;
        this.aoaLength = builder.aoaLength;

        this.lineWidth = builder.lineWidth;
        this.barWidth = builder.barWidth;
        this.halfLine = builder.halfLine;
        this.drawFont = builder.drawFont;
        this.drawFontSmall = builder.drawFontSmall;
        this.drawFontSSmall = builder.drawFontSSmall;
        this.strokeThick = builder.strokeThick;
        this.strokeThin = builder.strokeThin;

        this.crosshairImageScaled = builder.crosshairImageScaled;
    }

    /**
     * Factory method to create a context from settings.
     * Contains all the detailed calculation logic previously in
     * MinimalHUD.reinitConfig.
     */
    public static MinimalHUDContext create(HUDSettings settings) {
        Builder b = new Builder();

        // 1. Basic Metrics
        b.crossScale = settings.getCrosshairScale();
        int fAdd = settings.getFontSizeAdd();
        b.hudFontSize = (b.crossScale / 4) + fAdd;
        // Ensure minimum size to prevent crash
        if (b.hudFontSize < 8)
            b.hudFontSize = 8;

        b.barWidth = b.hudFontSize / 4;
        b.lineWidth = (b.hudFontSize / 10 == 0) ? 1 : b.hudFontSize / 10;

        // 2. Window Dimensions
        if (!settings.isDisplayCrosshair()) {
            b.width = (int) (b.crossScale * 2.25) - b.hudFontSize;
        } else {
            b.width = (int) (b.crossScale * 2.25);
        }
        b.height = (int) (b.crossScale * 1.5) + (int) (b.hudFontSize * 3.5);

        b.windowX = settings.getWindowX(b.width);
        b.windowY = settings.getWindowY(b.height);

        b.crossX = b.width / 2;
        b.crossY = b.height / 2;

        // 3. Component Details
        if (b.lineWidth == 0)
            b.lineWidth = 1;

        b.roundCompass = (int) (Math.round(b.hudFontSize * 0.8f));

        // Dynamic rightDraw calculation (WYSWYG Overlap Fix)
        // Standard value (~5 chars): 3.5f * fontSize
        // Labeled value (~9 chars): 5.5f * fontSize
        float multiplier = 3.5f;
        if (!settings.isSpeedLabelDisabled() || !settings.isAltitudeLabelDisabled() || !settings.isSEPLabelDisabled()) {
            multiplier = 5.5f;
        }
        b.rightDraw = (int) (b.hudFontSize * multiplier);

        b.compassDiameter = (int) Math.round(2 * b.hudFontSize * 0.618);
        b.compassRadius = (int) Math.round(b.compassDiameter / 2.0);
        b.compassInnerMarkRadius = (int) Math.round(0.618 * b.compassDiameter);

        b.aoaLength = b.rightDraw - b.hudFontSize / 1.5; // Adjusted for dynamic rightDraw

        // 4. Strokes & Fonts
        b.halfLine = (b.lineWidth / 2 == 0) ? 1 : (int) Math.round(b.lineWidth / 2.0f);
        b.strokeThick = new BasicStroke(b.halfLine + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
        b.strokeThin = new BasicStroke(b.halfLine, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);

        String nFont = settings.getNumFont();
        b.drawFont = new Font(nFont, Font.BOLD, b.hudFontSize);
        b.hudFontSizeSmall = (int) (b.hudFontSize * 0.75f);
        b.drawFontSmall = new Font(nFont, Font.BOLD, b.hudFontSizeSmall);
        b.drawFontSSmall = new Font(nFont, Font.BOLD, b.hudFontSize / 2);

        // 5. Resource Loading (IO) - Integrating Scheme 3 partially here
        String cName = settings.getCrosshairName();
        String path = "image/gunsight/" + cName + ".png";
        File imgFile = new File(path);

        if (imgFile.exists()) {
            try {
                Image raw = ImageIO.read(imgFile);
                if (raw != null) {
                    int targetSize = b.crossScale * 2;
                    java.awt.image.BufferedImage scaled = new java.awt.image.BufferedImage(targetSize, targetSize,
                            java.awt.image.BufferedImage.TYPE_INT_ARGB);
                    java.awt.Graphics2D g2 = scaled.createGraphics();
                    g2.setRenderingHint(java.awt.RenderingHints.KEY_INTERPOLATION,
                            java.awt.RenderingHints.VALUE_INTERPOLATION_BILINEAR);
                    g2.drawImage(raw, 0, 0, targetSize, targetSize, null);
                    g2.dispose();
                    b.crosshairImageScaled = scaled;
                }
            } catch (Exception e) {
                Logger.info("MinimalHUDContext", "Failed to load crosshair: " + e.getMessage());
            }
        }

        Logger.info("MinimalHUD",
                "MinimalHUD Config: Width=" + b.width + ", Height=" + b.height + ", CrossWidth=" + b.crossScale);

        return new MinimalHUDContext(b);
    }

    // Builder pattern for internal convenience
    private static class Builder {
        int width, height, hudFontSize, hudFontSizeSmall, windowX, windowY;
        int crossScale, crossX, crossY, roundCompass, rightDraw;
        int compassDiameter, compassRadius, compassInnerMarkRadius;
        double aoaLength;
        int lineWidth, barWidth, halfLine;
        Font drawFont, drawFontSmall, drawFontSSmall;
        BasicStroke strokeThick, strokeThin;
        Image crosshairImageScaled;
    }
}
