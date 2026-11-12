package ui.overlay;

import java.awt.BasicStroke;
import java.awt.Font;

import prog.Application;
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
    public final int miniHUDScale;
    public final int roundCompass;
    public final int compassDiameter;
    public final int compassRadius;
    public final int compassInnerMarkRadius;

    // --- Styling ---
    public final int lineWidth;
    public final int barWidth;
    public final int halfLine;
    public final Font drawFont;
    public final Font drawFontSmall;
    public final Font drawFontSSmall;
    public final BasicStroke strokeThick;
    public final BasicStroke strokeThin;

    private MinimalHUDContext(Builder builder) {
        this.width = builder.width;
        this.height = builder.height;
        this.hudFontSize = builder.hudFontSize;
        this.hudFontSizeSmall = builder.hudFontSizeSmall;
        this.windowX = builder.windowX;
        this.windowY = builder.windowY;

        this.miniHUDScale = builder.miniHUDScale;
        this.roundCompass = builder.roundCompass;
        this.compassDiameter = builder.compassDiameter;
        this.compassRadius = builder.compassRadius;
        this.compassInnerMarkRadius = builder.compassInnerMarkRadius;

        this.lineWidth = builder.lineWidth;
        this.barWidth = builder.barWidth;
        this.halfLine = builder.halfLine;
        this.drawFont = builder.drawFont;
        this.drawFontSmall = builder.drawFontSmall;
        this.drawFontSSmall = builder.drawFontSSmall;
        this.strokeThick = builder.strokeThick;
        this.strokeThin = builder.strokeThin;
    }

    /**
     * Factory method to create a context from settings.
     * Contains all the detailed calculation logic previously in
     * MinimalHUD.reinitConfig.
     *
     * DPI Scaling: All pixel dimensions are multiplied by Application.dpiScale
     * to ensure the HUD renders crisply on high-DPI displays.
     */
    public static MinimalHUDContext create(HUDSettings settings) {
        Builder b = new Builder();

        // Get DPI scale factor (1.0 = 100%, 2.0 = 200%)
        double dpiScale = Application.dpiScale;

        // 1. Basic Metrics - apply DPI scaling
        int baseScale = settings.getMiniHUDScale();
        b.miniHUDScale = (int) Math.round(baseScale * dpiScale);

        int fAdd = settings.getFontSizeAdd();
        // Font size is derived from miniHUDScale (already scaled)
        b.hudFontSize = (b.miniHUDScale / 4) + (int) Math.round(fAdd * dpiScale);
        // Ensure minimum size to prevent crash
        if (b.hudFontSize < 8)
            b.hudFontSize = 8;

        b.barWidth = b.hudFontSize / 4;
        b.lineWidth = (b.hudFontSize / 10 == 0) ? 1 : b.hudFontSize / 10;

        // 2. Window Dimensions - derived from scaled miniHUDScale
        b.width = (int) (b.miniHUDScale * 2.25) - b.hudFontSize;
        b.height = (int) (b.miniHUDScale * 1.5) + (int) (b.hudFontSize * 3.5);

        b.windowX = settings.getWindowX(b.width);
        b.windowY = settings.getWindowY(b.height);

        // 3. Component Details - derived from scaled hudFontSize
        if (b.lineWidth == 0)
            b.lineWidth = 1;

        b.roundCompass = (int) (Math.round(b.hudFontSize * 0.8f));

        b.compassDiameter = (int) Math.round(2 * b.hudFontSize * 0.618);
        b.compassRadius = (int) Math.round(b.compassDiameter / 2.0);
        b.compassInnerMarkRadius = (int) Math.round(0.618 * b.compassDiameter);

        // 4. Strokes & Fonts - scaled stroke widths for crisp lines
        b.halfLine = (b.lineWidth / 2 == 0) ? 1 : (int) Math.round(b.lineWidth / 2.0f);
        b.strokeThick = new BasicStroke(b.halfLine + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
        b.strokeThin = new BasicStroke(b.halfLine, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);

        String nFont = settings.getNumFont();
        b.drawFont = new Font(nFont, Font.BOLD, b.hudFontSize);
        b.hudFontSizeSmall = (int) (b.hudFontSize * 0.75f);
        b.drawFontSmall = new Font(nFont, Font.BOLD, b.hudFontSizeSmall);
        b.drawFontSSmall = new Font(nFont, Font.BOLD, b.hudFontSize / 2);

        Logger.info("MinimalHUD",
                "MinimalHUD Config: Width=" + b.width + ", Height=" + b.height + ", Scale=" + b.miniHUDScale);

        return new MinimalHUDContext(b);
    }

    /**
     * 派生一个按条目缩放后的上下文（字体/线宽/仪表尺寸同步缩放，窗口基准尺寸不变）。
     * scale = 1.0 时返回 this，避免重复创建。
     */
    public MinimalHUDContext withScale(double factor) {
        if (Math.abs(factor - 1.0) < 1e-6)
            return this;
        Builder b = new Builder();
        b.width = this.width;
        b.height = this.height;
        b.windowX = this.windowX;
        b.windowY = this.windowY;
        b.miniHUDScale = this.miniHUDScale;

        b.hudFontSize = Math.max(8, (int) Math.round(this.hudFontSize * factor));
        b.hudFontSizeSmall = Math.max(6, (int) Math.round(this.hudFontSizeSmall * factor));
        b.barWidth = Math.max(1, (int) Math.round(this.barWidth * factor));
        b.lineWidth = Math.max(1, (int) Math.round(this.lineWidth * factor));
        b.roundCompass = Math.max(2, (int) Math.round(this.roundCompass * factor));
        b.compassDiameter = Math.max(2, (int) Math.round(this.compassDiameter * factor));
        b.compassRadius = Math.max(1, (int) Math.round(this.compassRadius * factor));
        b.compassInnerMarkRadius = Math.max(1, (int) Math.round(this.compassInnerMarkRadius * factor));

        b.halfLine = Math.max(1, (int) Math.round(this.halfLine * factor));
        b.strokeThick = new BasicStroke(b.halfLine + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
        b.strokeThin = new BasicStroke(b.halfLine, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);

        b.drawFont = this.drawFont.deriveFont((float) b.hudFontSize);
        b.drawFontSmall = this.drawFontSmall.deriveFont((float) b.hudFontSizeSmall);
        b.drawFontSSmall = this.drawFontSSmall.deriveFont((float) Math.max(4,
                Math.round(this.drawFontSSmall.getSize() * factor)));
        return new MinimalHUDContext(b);
    }

    // Builder pattern for internal convenience
    private static class Builder {
        int width, height, hudFontSize, hudFontSizeSmall, windowX, windowY;
        int miniHUDScale, roundCompass;
        int compassDiameter, compassRadius, compassInnerMarkRadius;
        int lineWidth, barWidth, halfLine;
        Font drawFont, drawFontSmall, drawFontSSmall;
        BasicStroke strokeThick, strokeThin;
    }
}
