package ui.component.row;

import java.awt.Dimension;
import ui.component.HUDComponent;

/**
 * Interface for a single row in the HUD text series, now as a HUDComponent.
 */
public interface HUDRow extends HUDComponent {
    /**
     * Get the height of this row.
     */
    int getHeight();

    @Override
    default Dimension getPreferredSize() {
        // Most rows have a fixed height and variable width (managed by LayoutEngine)
        // We'll use a placeholder width or the actual text width if needed.
        return new Dimension(200, getHeight());
    }
}
