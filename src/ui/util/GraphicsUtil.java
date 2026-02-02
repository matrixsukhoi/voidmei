package ui.util;

import java.awt.BasicStroke;
import java.awt.Stroke;

/**
 * Utility for Graphics2D rendering operations.
 * Provides optimized stroke factories and drawing helpers.
 */
public class GraphicsUtil {

    /**
     * Creates a stroke with precise endpoints (no cap extension).
     *
     * Use this for lines that must align exactly with component boundaries.
     * The default BasicStroke uses CAP_SQUARE which extends the line by
     * half the stroke width at each endpoint.
     *
     * @param width The stroke width in pixels
     * @return A BasicStroke with CAP_BUTT (no endpoint extension)
     */
    public static Stroke createPreciseStroke(float width) {
        return new BasicStroke(width, BasicStroke.CAP_BUTT, BasicStroke.JOIN_MITER);
    }

    /**
     * Creates a stroke with precise endpoints and custom join style.
     *
     * @param width The stroke width in pixels
     * @param join  The join style (e.g., BasicStroke.JOIN_ROUND)
     * @return A BasicStroke with CAP_BUTT
     */
    public static Stroke createPreciseStroke(float width, int join) {
        return new BasicStroke(width, BasicStroke.CAP_BUTT, join);
    }

    /**
     * Creates a stroke with rounded endpoints.
     *
     * Use this for decorative lines where smooth endpoints are desired.
     *
     * @param width The stroke width in pixels
     * @return A BasicStroke with CAP_ROUND
     */
    public static Stroke createRoundedStroke(float width) {
        return new BasicStroke(width, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
    }
}
