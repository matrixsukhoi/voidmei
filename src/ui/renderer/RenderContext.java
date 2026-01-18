package ui.renderer;

import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Component;

import prog.Application;
import prog.config.ConfigProvider;

/**
 * Context object containing rendering configuration.
 * Passed to renderers to provide font, sizing, and layout information.
 */
public class RenderContext {
    public Font numFont;
    public Font labelFont;
    public Font unitFont;
    public int fontSize;
    public int columnNum;
    public int numHeight;

    public RenderContext(Font numFont, Font labelFont, Font unitFont,
            int fontSize, int columnNum, int numHeight) {
        this.numFont = numFont;
        this.labelFont = labelFont;
        this.unitFont = unitFont;
        this.fontSize = fontSize;
        this.columnNum = columnNum;
        this.numHeight = numHeight;
    }

    /**
     * Create RenderContext from configuration.
     * 
     * @param config       Configuration provider
     * @param component    Component for font metrics calculation
     * @param numFontKey   Config key for number font
     * @param labelFontKey Config key for label font
     * @param fontAddKey   Config key for font size adjustment
     * @param columnKey    Config key for column count
     */
    public static RenderContext fromConfig(ConfigProvider config, Component component,
            String numFontKey, String labelFontKey, String fontAddKey, String columnKey) {

        // Load font names
        String numFontName = getConfigOrDefault(config, numFontKey, Application.defaultNumfontName);
        String labelFontName = getConfigOrDefault(config, labelFontKey, Application.defaultFont.getFontName());
        int fontAdd = getConfigIntOrDefault(config, fontAddKey, 0);
        int columnNum = getConfigIntOrDefault(config, columnKey, 3);

        return create(component, numFontName, labelFontName, fontAdd, columnNum);
    }

    /**
     * Create RenderContext from OverlaySettings.
     */
    public static RenderContext fromSettings(prog.config.OverlaySettings settings, Component component,
            String numFontKey, String columnKey, ConfigProvider legacyConfig) {

        String numFontName = getConfigOrDefault(legacyConfig, numFontKey, Application.defaultNumfontName);
        String labelFontName = (settings != null) ? settings.getFontName() : Application.defaultFont.getFontName();
        int fontAdd = (settings != null) ? settings.getFontSizeAdd() : 0;
        int columnNum = getConfigIntOrDefault(legacyConfig, columnKey, 3);

        return create(component, numFontName, labelFontName, fontAdd, columnNum);
    }

    private static RenderContext create(Component component, String numFontName, String labelFontName, int fontAdd,
            int columnNum) {
        // Calculate font sizes
        int fontSize = 24 + fontAdd;
        Font numFont = new Font(numFontName, Font.BOLD, fontSize);
        Font labelFont = new Font(labelFontName, Font.BOLD, Math.round(fontSize / 2.0f));
        Font unitFont = new Font(numFontName, Font.PLAIN, Math.round(fontSize / 2.0f));

        int numHeight = component.getFontMetrics(numFont).getHeight();

        return new RenderContext(numFont, labelFont, unitFont, fontSize, columnNum, numHeight);
    }

    private static String getConfigOrDefault(ConfigProvider config, String key, String defaultValue) {
        if (config == null || key == null)
            return defaultValue;
        String value = config.getConfig(key);
        return (value != null && !value.isEmpty()) ? value : defaultValue;
    }

    private static int getConfigIntOrDefault(ConfigProvider config, String key, int defaultValue) {
        String value = getConfigOrDefault(config, key, null);
        if (value == null)
            return defaultValue;
        try {
            return Integer.parseInt(value);
        } catch (NumberFormatException e) {
            return defaultValue;
        }
    }

    /**
     * Calculate field width based on font size.
     */
    public int getFieldWidth() {
        return 3 * fontSize;
    }

    /**
     * Calculate total width for the given number of columns.
     */
    public int getTotalWidth() {
        return (fontSize >> 1) + (int) ((columnNum + 0.5) * 5f * fontSize);
    }

    /**
     * Calculate total height for the given number of visible fields.
     */
    public int getTotalHeight(int visibleFieldCount) {
        int addnum = (visibleFieldCount % columnNum == 0) ? 0 : 1;
        return (int) (numHeight + (visibleFieldCount / columnNum + addnum + 1) * 1.0f * numHeight);
    }
}
