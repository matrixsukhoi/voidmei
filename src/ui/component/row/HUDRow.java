package ui.component.row;

import java.awt.Graphics2D;

/**
 * Interface for a single row in the HUD text series.
 */
public interface HUDRow {
    /**
     * Draw the row at the specified coordinates.
     * 
     * @param g2d Graphics context
     * @param x   Left X coordinate
     * @param y   Top Y coordinate (or baseline, depending on impl)
     */
    void draw(Graphics2D g2d, int x, int y);

    /**
     * Get the height of this row to calculate offset for the next row.
     * 
     * @return Height in pixels
     */
    int getHeight();
}
