package ui.component.gauge;

import java.awt.Color;
import java.awt.Dimension;
import java.awt.Font;
import java.awt.Graphics2D;
import java.util.ArrayList;
import java.util.List;

import prog.Application;
import ui.component.AbstractHUDComponent;

/**
 * A gauge component with support for configurable markers (lines, zones, ticks).
 *
 * This component combines:
 * - A standard vertical or horizontal bar gauge
 * - A pluggable marker system for visualizing limits, optimal values, zones, etc.
 *
 * Design follows the composition pattern - it does NOT extend LinearGauge but
 * provides similar functionality with added marker support.
 *
 * Example usage:
 * <pre>
 * MarkedGauge gauge = new MarkedGauge();
 * gauge.setStyleContext(width, height, font, GaugeBarStyle.builder()
 *     .fillColor(Application.colorNum)
 *     .backgroundColor(Application.colorShadeShape)
 *     .vertical(true)
 *     .build());
 *
 * gauge.addMarker(GaugeMarker.builder()
 *     .id("optimal")
 *     .type(MarkerType.LINE_FULL)
 *     .color(Application.colorWarning)
 *     .ratio(-1)  // Initially hidden
 *     .build());
 *
 * // In update loop:
 * gauge.setValue(currentValue, maxValue);
 * gauge.updateMarkerRatio("optimal", optimalRatio);
 * </pre>
 *
 * Zero-allocation considerations:
 * - Markers are stored in ArrayList (no allocation during updates)
 * - Stroke objects are pre-cached in GaugeBarStyle
 * - valueBuffer provides zero-GC number display
 */
public class MarkedGauge extends AbstractHUDComponent {

    // --- Style Configuration ---
    private int width = 10;
    private int height = 100;
    private Font tickFont;
    private GaugeBarStyle barStyle;

    // --- Markers ---
    private final List<GaugeMarker> markers = new ArrayList<>();

    // --- Current State ---
    private double currentValue = 0;
    private double maxValue = 1.0;

    // --- Value Display (Zero-GC) ---
    public String label = "";
    public final char[] valueBuffer = new char[32];
    public int valueLen = 0;
    public String displayValue = "";

    public MarkedGauge() {
        // Default style
        this.barStyle = GaugeBarStyle.builder()
            .fillColor(Application.colorNum)
            .backgroundColor(Application.colorShadeShape)
            .vertical(true)
            .build();
    }

    // --- Configuration API ---

    /**
     * Configures the gauge dimensions, font, and bar style.
     * Call this once during initialization or when config changes.
     *
     * @param width    Gauge width in pixels
     * @param height   Gauge height in pixels
     * @param tickFont Font for value/label text
     * @param style    Bar styling configuration
     */
    public void setStyleContext(int width, int height, Font tickFont, GaugeBarStyle style) {
        this.width = width;
        this.height = height;
        this.tickFont = tickFont;
        this.barStyle = style;
    }

    /**
     * Sets just the bar style (for reconfigurations without dimension changes).
     *
     * @param style Bar styling configuration
     */
    public void setBarStyle(GaugeBarStyle style) {
        this.barStyle = style;
    }

    /**
     * Sets the max value for the gauge scale.
     *
     * @param maxValue Maximum value (gauge shows 0 to maxValue)
     */
    public void setMaxValue(double maxValue) {
        this.maxValue = maxValue;
    }

    // --- Marker API ---

    /**
     * Adds a marker to the gauge.
     * Markers are drawn in the order they were added.
     *
     * @param marker The marker specification
     */
    public void addMarker(GaugeMarker marker) {
        markers.add(marker);
    }

    /**
     * Clears all markers from the gauge.
     */
    public void clearMarkers() {
        markers.clear();
    }

    /**
     * Updates the position ratio of an existing marker by ID.
     * If the marker is not found, this method does nothing.
     *
     * This method uses copy-on-write semantics for thread safety
     * and avoids allocation when the ratio is unchanged.
     *
     * @param markerId The marker's unique ID
     * @param newRatio The new position ratio (0.0 to 1.0, or negative to hide)
     */
    public void updateMarkerRatio(String markerId, double newRatio) {
        for (int i = 0; i < markers.size(); i++) {
            GaugeMarker m = markers.get(i);
            if (m.id.equals(markerId)) {
                GaugeMarker updated = m.withRatio(newRatio);
                if (updated != m) {  // Only replace if changed
                    markers.set(i, updated);
                }
                return;
            }
        }
    }

    // --- Value API ---

    /**
     * Updates the current value using pre-formatted char buffer (zero-GC).
     *
     * @param value The current value
     * @param buf   Character buffer containing formatted value
     * @param len   Number of characters in buffer
     */
    public void update(int value, char[] buf, int len) {
        this.currentValue = value;
        if (len > 32) len = 32;
        System.arraycopy(buf, 0, this.valueBuffer, 0, len);
        this.valueLen = len;
    }

    /**
     * Updates the current value with a display string.
     *
     * @param value        The current value
     * @param displayValue The formatted display string
     */
    public void update(int value, String displayValue) {
        this.currentValue = value;
        this.displayValue = displayValue;
        this.valueLen = 0;  // Use displayValue instead of buffer
    }

    /**
     * Sets the current value and max value (for ratio-based gauges).
     *
     * @param value    Current value
     * @param maxValue Maximum value
     */
    public void setValue(double value, double maxValue) {
        this.currentValue = value;
        this.maxValue = maxValue;
    }

    // --- HUDComponent Interface ---

    @Override
    public String getId() {
        return "gauge.marked." + label;
    }

    @Override
    public Dimension getPreferredSize() {
        int textMetric = 30;
        if (tickFont != null) {
            textMetric = (int) (tickFont.getSize() * 2.0);
        }

        if (barStyle.vertical) {
            int estimatedWidth = textMetric + 5 + width;
            return new Dimension(estimatedWidth, height);
        } else {
            int estimatedHeight = height + textMetric;
            return new Dimension(width, estimatedHeight);
        }
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        draw(g2d, x, y, height, width, tickFont, tickFont);
    }

    /**
     * Draws the gauge with explicit dimensions and fonts.
     * This signature matches LinearGauge for compatibility.
     *
     * @param g2d       Graphics context
     * @param x         X position
     * @param y         Y position
     * @param length    Length of the gauge bar (height for vertical, width for horizontal)
     * @param thickness Thickness of the gauge bar
     * @param fontLabel Font for labels
     * @param fontValue Font for values
     */
    public void draw(Graphics2D g2d, int x, int y, int length, int thickness,
                     Font fontLabel, Font fontValue) {
        if (barStyle.vertical) {
            drawVertical(g2d, x, y, length, thickness, fontLabel, fontValue);
        } else {
            drawHorizontal(g2d, x, y, length, thickness, fontLabel, fontValue);
        }
    }

    // --- Private Drawing Methods ---

    private void drawVertical(Graphics2D g2d, int x, int y, int length, int thickness,
                              Font fontLabel, Font fontValue) {
        // Calculate filled portion
        int pixVal = 0;
        if (maxValue > 0) {
            pixVal = Math.round((float) (currentValue * length / maxValue));
            if (pixVal > length) pixVal = length;
            if (pixVal < 0) pixVal = 0;
        }

        // Calculate text width for layout
        int textWidth = getValueWidth(g2d, fontValue);
        int labelSpacing = 2;

        // Bar X position (text on left, bar on right)
        int barX = x + textWidth + labelSpacing;

        // 1. Draw markers BEHIND the bar (partial lines that get covered)
        for (GaugeMarker m : markers) {
            if (m.isVisible() && m.type == MarkerType.LINE_PARTIAL) {
                drawMarkerVertical(g2d, barX, y, length, thickness, m);
            }
        }

        // 2. Draw background bar
        g2d.setColor(barStyle.backgroundColor);
        g2d.fillRect(barX, y, thickness, length);

        // 3. Draw filled portion (from bottom up)
        if (pixVal > 0) {
            g2d.setColor(barStyle.fillColor);
            g2d.fillRect(barX, y + length - pixVal, thickness, pixVal);
        }

        // 4. Draw ZONE markers (overlay on bar)
        for (GaugeMarker m : markers) {
            if (m.isVisible() && m.type == MarkerType.ZONE) {
                drawMarkerVertical(g2d, barX, y, length, thickness, m);
            }
        }

        // 5. Draw LINE_FULL markers (on top of bar)
        for (GaugeMarker m : markers) {
            if (m.isVisible() && m.type == MarkerType.LINE_FULL) {
                drawMarkerVertical(g2d, barX, y, length, thickness, m);
            }
        }

        // 6. Draw separator line and value text (moving with value)
        int sepY = y + length - 1 - pixVal;
        Color textColor = barStyle.fillColor;

        // Separator line
        g2d.setStroke(barStyle.borderStroke);
        int totalWidth = textWidth + labelSpacing + thickness;
        drawSeparator(g2d, x, sepY, totalWidth, textColor);

        // Value text
        drawValueText(g2d, x, sepY - 1, fontValue, textColor);

        // 7. Draw border (if enabled)
        if (barStyle.showBorder) {
            g2d.setColor(barStyle.borderColor);
            g2d.setStroke(barStyle.borderStroke);
            g2d.drawRect(barX, y, thickness - 1, length - 1);
        }
    }

    private void drawHorizontal(Graphics2D g2d, int x, int y, int length, int thickness,
                                Font fontLabel, Font fontValue) {
        // Calculate filled portion
        int pixVal = 0;
        if (maxValue > 0) {
            pixVal = Math.round((float) (currentValue * length / maxValue));
            if (pixVal > length) pixVal = length;
            if (pixVal < 0) pixVal = 0;
        }

        // 1. Draw markers BEHIND the bar
        for (GaugeMarker m : markers) {
            if (m.isVisible() && m.type == MarkerType.LINE_PARTIAL) {
                drawMarkerHorizontal(g2d, x, y, length, thickness, m);
            }
        }

        // 2. Draw background bar
        g2d.setColor(barStyle.backgroundColor);
        g2d.fillRect(x, y, length, thickness);

        // 3. Draw filled portion (left to right)
        if (pixVal > 0) {
            g2d.setColor(barStyle.fillColor);
            g2d.fillRect(x, y, pixVal, thickness);
        }

        // 4. Draw ZONE markers
        for (GaugeMarker m : markers) {
            if (m.isVisible() && m.type == MarkerType.ZONE) {
                drawMarkerHorizontal(g2d, x, y, length, thickness, m);
            }
        }

        // 5. Draw LINE_FULL markers
        for (GaugeMarker m : markers) {
            if (m.isVisible() && m.type == MarkerType.LINE_FULL) {
                drawMarkerHorizontal(g2d, x, y, length, thickness, m);
            }
        }

        // 6. Draw separator line
        int sepHeight = thickness + fontValue.getSize() + 2;
        Color textColor = barStyle.fillColor;

        g2d.setColor(Application.colorShadeShape);
        g2d.drawLine(x + pixVal + 1, y, x + pixVal + 1, y + sepHeight);
        g2d.setColor(textColor);
        g2d.drawLine(x + pixVal, y, x + pixVal, y + sepHeight);

        // 7. Draw value text
        drawValueText(g2d, x + pixVal, y + thickness + fontValue.getSize(), fontValue, textColor);

        // 8. Draw border
        if (barStyle.showBorder) {
            g2d.setColor(barStyle.borderColor);
            g2d.setStroke(barStyle.borderStroke);
            g2d.drawRect(x, y, length - 1, thickness - 1);
        }
    }

    private void drawMarkerVertical(Graphics2D g2d, int barX, int barY,
                                    int length, int thickness, GaugeMarker m) {
        // Y position: bottom is 0, top is 1
        int markerY = barY + length - (int) (length * clamp(m.ratio));

        g2d.setStroke(barStyle.tickStroke);
        g2d.setColor(m.color);

        switch (m.type) {
            case LINE_FULL:
                // Full width line across the bar
                g2d.drawLine(barX, markerY, barX + thickness, markerY);
                break;

            case LINE_PARTIAL:
                // Partial line based on side and widthRatio
                int lineWidth = (int) (thickness * m.widthRatio);
                if (m.side < 0) {
                    // Left side, extends into bar
                    g2d.drawLine(barX - 4, markerY, barX + lineWidth, markerY);
                } else if (m.side > 0) {
                    // Right side, extends out of bar
                    g2d.drawLine(barX + thickness - lineWidth, markerY,
                                 barX + thickness + 4, markerY);
                } else {
                    // Center
                    int start = barX + (thickness - lineWidth) / 2;
                    g2d.drawLine(start, markerY, start + lineWidth, markerY);
                }
                break;

            case ZONE:
                // Filled zone from bottom to marker position
                int zoneWidth = (int) (thickness * m.widthRatio);
                int zoneHeight = (int) (length * clamp(m.ratio));
                int zoneX = barX;
                if (m.side > 0) {
                    zoneX = barX + thickness - zoneWidth;
                } else if (m.side == 0) {
                    zoneX = barX + (thickness - zoneWidth) / 2;
                }
                g2d.fillRect(zoneX, barY + length - zoneHeight, zoneWidth, zoneHeight);
                break;

            case TICK_LABELED:
                // Line with label
                g2d.drawLine(barX, markerY, barX + thickness, markerY);
                if (m.label != null && tickFont != null) {
                    g2d.setFont(tickFont);
                    // Draw label to the right of the bar
                    drawTextShaded(g2d, barX + thickness + 4, markerY + 4,
                                   m.label, tickFont, m.color);
                }
                break;
        }
    }

    private void drawMarkerHorizontal(Graphics2D g2d, int barX, int barY,
                                      int length, int thickness, GaugeMarker m) {
        // X position: left is 0, right is 1
        int markerX = barX + (int) (length * clamp(m.ratio));

        g2d.setStroke(barStyle.tickStroke);
        g2d.setColor(m.color);

        switch (m.type) {
            case LINE_FULL:
                g2d.drawLine(markerX, barY, markerX, barY + thickness);
                break;

            case LINE_PARTIAL:
                int lineHeight = (int) (thickness * m.widthRatio);
                if (m.side < 0) {
                    // Top side
                    g2d.drawLine(markerX, barY - 4, markerX, barY + lineHeight);
                } else if (m.side > 0) {
                    // Bottom side
                    g2d.drawLine(markerX, barY + thickness - lineHeight,
                                 markerX, barY + thickness + 4);
                } else {
                    int start = barY + (thickness - lineHeight) / 2;
                    g2d.drawLine(markerX, start, markerX, start + lineHeight);
                }
                break;

            case ZONE:
                int zoneHeight = (int) (thickness * m.widthRatio);
                int zoneWidth = (int) (length * clamp(m.ratio));
                int zoneY = barY;
                if (m.side > 0) {
                    zoneY = barY + thickness - zoneHeight;
                } else if (m.side == 0) {
                    zoneY = barY + (thickness - zoneHeight) / 2;
                }
                g2d.fillRect(barX, zoneY, zoneWidth, zoneHeight);
                break;

            case TICK_LABELED:
                g2d.drawLine(markerX, barY, markerX, barY + thickness);
                if (m.label != null && tickFont != null) {
                    g2d.setFont(tickFont);
                    drawTextShaded(g2d, markerX + 4, barY + thickness + tickFont.getSize(),
                                   m.label, tickFont, m.color);
                }
                break;
        }
    }

    private void drawSeparator(Graphics2D g2d, int x, int y, int width, Color c) {
        // Shadow
        g2d.setColor(Application.colorShadeShape);
        g2d.drawRect(x, y, width - 1, 3 - 1);
        // Fill
        g2d.setColor(c);
        g2d.fillRect(x + 1, y + 1, width - 2, 3 - 2);
    }

    private int getValueWidth(Graphics2D g2d, Font f) {
        if (f == null) return 30;
        int labelW = (label != null && !label.isEmpty())
            ? g2d.getFontMetrics(f).stringWidth(label) : 0;
        int valueW;
        if (valueLen > 0) {
            valueW = g2d.getFontMetrics(f).charsWidth(valueBuffer, 0, valueLen);
        } else {
            valueW = g2d.getFontMetrics(f).stringWidth(displayValue != null ? displayValue : "");
        }
        return labelW + valueW;
    }

    private void drawValueText(Graphics2D g2d, int x, int y, Font f, Color c) {
        if (f == null) return;

        int labelW = 0;
        if (label != null && !label.isEmpty()) {
            drawTextShaded(g2d, x, y, label, f, c);
            labelW = g2d.getFontMetrics(f).stringWidth(label);
        }

        if (valueLen > 0) {
            drawTextShaded(g2d, x + labelW, y, valueBuffer, valueLen, f, c);
        } else if (displayValue != null && !displayValue.isEmpty()) {
            drawTextShaded(g2d, x + labelW, y, displayValue, f, c);
        }
    }

    private void drawTextShaded(Graphics2D g2d, int x, int y, String s, Font f, Color c) {
        g2d.setFont(f);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawString(s, x + 1, y + 1);
        g2d.setColor(c);
        g2d.drawString(s, x, y);
    }

    private void drawTextShaded(Graphics2D g2d, int x, int y, char[] buf, int len, Font f, Color c) {
        g2d.setFont(f);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawChars(buf, 0, len, x + 1, y + 1);
        g2d.setColor(c);
        g2d.drawChars(buf, 0, len, x, y);
    }

    private double clamp(double v) {
        if (v < 0) return 0;
        if (v > 1) return 1;
        return v;
    }
}
