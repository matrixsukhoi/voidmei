package ui.component.gauge;

import java.awt.Color;

/**
 * Immutable marker specification for MarkedGauge component.
 *
 * Markers are visual indicators placed at specific positions on a gauge,
 * such as limit lines, warning zones, or labeled ticks.
 *
 * This class is immutable; use the Builder to create instances and
 * withRatio() for copy-on-write updates (zero-allocation friendly when
 * updating only the ratio).
 *
 * Example usage:
 * <pre>
 * GaugeMarker optimalStage = GaugeMarker.builder()
 *     .id("optimal")
 *     .type(MarkerType.LINE_FULL)
 *     .ratio(0.5)
 *     .color(Application.colorWarning)
 *     .build();
 * </pre>
 */
public final class GaugeMarker {

    /** Unique identifier for this marker (used for dynamic updates). */
    public final String id;

    /** The visual type of this marker. */
    public final MarkerType type;

    /** Position ratio on the gauge (0.0 = minimum, 1.0 = maximum).
     *  Values outside [0, 1] will hide the marker. */
    public final double ratio;

    /** Color used to render this marker. */
    public final Color color;

    /** Text label for TICK_LABELED type markers. */
    public final String label;

    /** Width ratio for ZONE and LINE_PARTIAL types (0.0 to 1.0, relative to gauge width). */
    public final float widthRatio;

    /** Side positioning: -1 = left, 0 = center, 1 = right. */
    public final int side;

    private GaugeMarker(Builder builder) {
        this.id = builder.id;
        this.type = builder.type;
        this.ratio = builder.ratio;
        this.color = builder.color;
        this.label = builder.label;
        this.widthRatio = builder.widthRatio;
        this.side = builder.side;
    }

    /**
     * Creates a new marker with the same properties but a different ratio.
     * This is the preferred method for updating marker positions in the render loop
     * as it avoids allocation when the marker is already at the target ratio.
     *
     * @param newRatio The new position ratio
     * @return A new GaugeMarker with updated ratio, or this if ratio unchanged
     */
    public GaugeMarker withRatio(double newRatio) {
        if (Math.abs(newRatio - this.ratio) < 0.0001) {
            return this;  // No change, return self (zero allocation)
        }
        return new Builder()
            .id(this.id)
            .type(this.type)
            .ratio(newRatio)
            .color(this.color)
            .label(this.label)
            .widthRatio(this.widthRatio)
            .side(this.side)
            .build();
    }

    /**
     * Checks if this marker should be visible (ratio in valid range).
     *
     * @return true if ratio is in [0, 1] and marker should be drawn
     */
    public boolean isVisible() {
        return ratio >= 0.0 && ratio <= 1.0;
    }

    /**
     * Creates a new Builder for constructing GaugeMarker instances.
     *
     * @return A new Builder instance
     */
    public static Builder builder() {
        return new Builder();
    }

    /**
     * Builder for constructing GaugeMarker instances.
     */
    public static final class Builder {
        private String id = "";
        private MarkerType type = MarkerType.LINE_FULL;
        private double ratio = -1;  // Hidden by default
        private Color color = Color.RED;
        private String label = "";
        private float widthRatio = 0.5f;
        private int side = 0;

        public Builder id(String id) {
            this.id = id;
            return this;
        }

        public Builder type(MarkerType type) {
            this.type = type;
            return this;
        }

        public Builder ratio(double ratio) {
            this.ratio = ratio;
            return this;
        }

        public Builder color(Color color) {
            this.color = color;
            return this;
        }

        public Builder label(String label) {
            this.label = label;
            return this;
        }

        public Builder widthRatio(float widthRatio) {
            this.widthRatio = widthRatio;
            return this;
        }

        public Builder side(int side) {
            this.side = side;
            return this;
        }

        public GaugeMarker build() {
            return new GaugeMarker(this);
        }
    }
}
