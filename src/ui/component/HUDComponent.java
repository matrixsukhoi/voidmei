package ui.component;

import java.awt.Graphics2D;
import java.awt.Dimension;
import ui.overlay.model.HUDData;

/**
 * Interface for all HUD Components using the new Reactive Architecture.
 */
public interface HUDComponent {

    /**
     * Unique ID for the component (used for layout/config).
     */
    String getId();

    /**
     * Preferred size of the component.
     */
    Dimension getPreferredSize();

    /**
     * Draw the component.
     */
    void draw(Graphics2D g2d, int x, int y);

    /**
     * Legacy generic update method.
     * 
     * @deprecated Use onDataUpdate(HUDData) instead.
     */
    @Deprecated
    default void update(Object data) {
    }

    /**
     * Update component state from the centralized HUD Data snapshot.
     * Components should extract only the fields they care about.
     * 
     * @param data The current frame's full data snapshot.
     */
    /**
     * Check if component is visible.
     */
    default boolean isVisible() {
        return true;
    }

    /**
     * Set visibility.
     */
    default void setVisible(boolean visible) {
    }

    /**
     * Update component state from the centralized HUD Data snapshot.
     * Components should extract only the fields they care about.
     * 
     * @param data The current frame's full data snapshot.
     */
    default void onDataUpdate(HUDData data) {
    }
}
