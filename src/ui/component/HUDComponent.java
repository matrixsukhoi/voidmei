package ui.component;

import java.awt.Dimension;
import java.awt.Graphics2D;

/**
 * Common interface for all HUD components that can be managed by the Layout
 * Engine.
 */
public interface HUDComponent {

    /**
     * Unique identifier for the component (used for layout mapping).
     */
    String getId();

    /**
     * Returns the preferred size of the component.
     */
    Dimension getPreferredSize();

    /**
     * Draws the component at the specified absolute coordinates.
     */
    void draw(Graphics2D g, int x, int y);

    /**
     * Updates the component with fresh data.
     * The type of data depends on the implementation.
     */
    void update(Object data);
}
