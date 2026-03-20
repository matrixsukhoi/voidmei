package ui.component.gauge;

import java.awt.Color;
import java.awt.Stroke;

import ui.util.GraphicsUtil;

/**
 * Immutable configuration for the bar portion of a MarkedGauge.
 *
 * This class pre-caches Stroke objects to avoid allocation during rendering.
 * Create once via Builder and reuse across frames.
 *
 * Example usage:
 * <pre>
 * GaugeBarStyle style = GaugeBarStyle.builder()
 *     .fillColor(Application.colorNum)
 *     .backgroundColor(Application.colorShadeShape)
 *     .vertical(true)
 *     .strokeWidth(2)
 *     .build();
 * </pre>
 */
public final class GaugeBarStyle {

    /** Color for the filled portion of the bar (current value). */
    public final Color fillColor;

    /** Color for the background/empty portion of the bar. */
    public final Color backgroundColor;

    /** Color for the bar border (if showBorder is true). */
    public final Color borderColor;

    /** Whether to draw a border around the bar. */
    public final boolean showBorder;

    /** Whether this is a vertical gauge (bottom-to-top fill). */
    public final boolean vertical;

    /** Width in pixels for strokes (border, ticks). */
    public final int strokeWidth;

    /** Pre-cached stroke for border drawing (zero-allocation in draw). */
    public final Stroke borderStroke;

    /** Pre-cached stroke for tick/marker drawing (zero-allocation in draw). */
    public final Stroke tickStroke;

    private GaugeBarStyle(Builder builder) {
        this.fillColor = builder.fillColor;
        this.backgroundColor = builder.backgroundColor;
        this.borderColor = builder.borderColor;
        this.showBorder = builder.showBorder;
        this.vertical = builder.vertical;
        this.strokeWidth = builder.strokeWidth;

        // Pre-cache strokes at construction time
        this.borderStroke = GraphicsUtil.createPreciseStroke(1);
        this.tickStroke = GraphicsUtil.createPreciseStroke(builder.strokeWidth);
    }

    /**
     * Creates a new Builder for constructing GaugeBarStyle instances.
     *
     * @return A new Builder instance
     */
    public static Builder builder() {
        return new Builder();
    }

    /**
     * Builder for constructing GaugeBarStyle instances.
     */
    public static final class Builder {
        private Color fillColor = Color.CYAN;
        private Color backgroundColor = Color.DARK_GRAY;
        private Color borderColor = Color.GRAY;
        private boolean showBorder = false;
        private boolean vertical = true;
        private int strokeWidth = 2;

        public Builder fillColor(Color fillColor) {
            this.fillColor = fillColor;
            return this;
        }

        public Builder backgroundColor(Color backgroundColor) {
            this.backgroundColor = backgroundColor;
            return this;
        }

        public Builder borderColor(Color borderColor) {
            this.borderColor = borderColor;
            return this;
        }

        public Builder showBorder(boolean showBorder) {
            this.showBorder = showBorder;
            return this;
        }

        public Builder vertical(boolean vertical) {
            this.vertical = vertical;
            return this;
        }

        public Builder strokeWidth(int strokeWidth) {
            this.strokeWidth = strokeWidth;
            return this;
        }

        public GaugeBarStyle build() {
            return new GaugeBarStyle(this);
        }
    }
}
