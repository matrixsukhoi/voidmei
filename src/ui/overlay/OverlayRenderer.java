package ui.overlay;

import java.util.List;
import com.alee.laf.panel.WebPanel;

/**
 * Strategy interface for overlay rendering.
 * Implementations define how data rows are displayed (zebra, solid, grid, etc.)
 */
public interface OverlayRenderer {

    /**
     * Render the given data lines into the panel.
     * 
     * @param data  List of strings to display
     * @param panel The panel to render into
     * @param font  The font to use for rendering
     * @param alpha Transparency level (0-255)
     */
    void render(List<String> data, WebPanel panel, java.awt.Font font, int alpha);

    /**
     * Check if a line should be styled as a header.
     * 
     * @param line The line content
     * @return true if this line is a header
     */
    boolean isHeader(String line);

    /**
     * Set a custom header matcher for this renderer.
     * 
     * @param matcher Predicate to determine if a line is a header
     */
    void setHeaderMatcher(java.util.function.Predicate<String> matcher);
}
