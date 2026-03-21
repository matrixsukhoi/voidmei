package ui.component.gauge;

/**
 * Enumeration of marker types for MarkedGauge component.
 *
 * Each type represents a different visual style for gauge markers:
 * - LINE_FULL: A line spanning the full width of the gauge (like Mach red line in SpeedRatioBar)
 * - LINE_PARTIAL: A partial-width tick mark (like aileron/rudder lock lines)
 * - ZONE: A filled region marking a range (like stall warning zone)
 * - TICK_LABELED: A tick mark with an associated text label
 */
public enum MarkerType {
    /**
     * Full-width line spanning the entire gauge width.
     * Example: Mach limit red line, optimal compressor stage indicator.
     */
    LINE_FULL,

    /**
     * Partial-width tick mark, typically on one side of the gauge.
     * Use GaugeMarker.side to control position (-1=left, 0=center, 1=right).
     * Use GaugeMarker.widthRatio to control tick length as a ratio of gauge width.
     */
    LINE_PARTIAL,

    /**
     * Filled rectangular zone marking a range.
     * Use GaugeMarker.widthRatio to control zone width as a ratio of gauge width.
     * Use GaugeMarker.side to position the zone.
     */
    ZONE,

    /**
     * Tick mark with an attached text label.
     * Use GaugeMarker.label for the text content.
     */
    TICK_LABELED
}
