package ui.renderer;

import java.awt.Dimension;
import java.awt.Graphics2D;
import java.util.List;

import ui.model.FlightField;

/**
 * Interface for rendering flight info fields.
 * Implementations can provide different visual styles.
 */
public interface FlightInfoRenderer {

    /**
     * Render all visible fields.
     * 
     * @param g2d    Graphics context
     * @param fields List of fields to render
     * @param ctx    Render context with fonts and sizing
     * @param offset Starting offset [x, y], will be modified during rendering
     */
    void render(Graphics2D g2d, List<FlightField> fields, RenderContext ctx, int[] offset);

    /**
     * Calculate the preferred size for rendering the given fields.
     */
    Dimension calculatePreferredSize(List<FlightField> fields, RenderContext ctx);
}
