package prog.config;

/**
 * Generalized interface for overlay settings.
 * Provides relative-to-absolute coordinate mapping and supports write-back for
 * dragging.
 */
public interface OverlaySettings {
    /**
     * Get absolute X coordinate in pixels.
     * 
     * @param width Window width for centering fallback (if applicable)
     */
    int getWindowX(int width);

    /**
     * Get absolute Y coordinate in pixels.
     * 
     * @param height Window height for centering fallback (if applicable)
     */
    int getWindowY(int height);

    /**
     * Save absolute pixel coordinates back to the relative coordinate system.
     */
    void saveWindowPosition(double x, double y);

    /**
     * Get the font name for this overlay.
     */
    String getFontName();

    /**
     * Get the font size adjustment for this overlay.
     */
    int getFontSizeAdd();

    /**
     * Generic property getters
     */
    boolean getBool(String key, boolean def);

    int getInt(String key, int def);

    String getString(String key, String def);
}
