package ui.renderer;

import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Component;

import prog.Application;
import prog.config.ConfigLoader;
import prog.config.ConfigProvider;
import prog.util.PropertyBinder;

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
     *
     * @param numFontKey   Config key for number font (e.g. "flightInfoFontC")
     * @param labelFontKey Config key for label font (e.g. "flightInfoFontC")
     * @param columnKey    Config key for column count
     */
    public static RenderContext fromSettings(prog.config.OverlaySettings settings, Component component,
            String numFontKey, String labelFontKey, String columnKey, ConfigProvider legacyConfig) {

        // 修复 issue #60: 字体必须按显式配置键解析(分组内优先, 全局兜底),
        // 键缺失/值为空时才回退到 OverlaySettings 字体链
        // (数字: GlobalNumFont; 标签: Panel :font → GlobalTextFont)。
        String numFontName = resolveFontName(settings, numFontKey,
                (settings != null) ? settings.getNumFontName() : Application.defaultNumfontName);
        String labelFontName = resolveFontName(settings, labelFontKey,
                (settings != null) ? settings.getFontName() : Application.defaultFont.getFontName());
        int fontAdd = (settings != null) ? settings.getFontSizeAdd() : 0;
        int columnNum = getConfigIntOrDefault(legacyConfig, columnKey, 3);

        return create(component, numFontName, labelFontName, fontAdd, columnNum);
    }

    /**
     * 按配置键解析字体名。
     * 优先级: 1) GroupConfig 同名字段 —— 设置 UI 经 PropertyBinder 组作用域写入的目标,
     * 与写入同源防止 setConfig 全局更新同名 row 造成的跨组串扰;
     * 2) 分组内 row —— 非字段键 (如 flightInfoFontC), getString 分组作用域优先;
     * 3) fallback 字体链。
     */
    private static String resolveFontName(prog.config.OverlaySettings settings, String key, String fallback) {
        if (settings == null || key == null)
            return fallback;
        // fontSize/fontName 等恰为 GroupConfig 字段名的键, 用户改动落在字段而非 row
        String value = (settings.getGroupConfig() != null)
                ? PropertyBinder.getString(settings.getGroupConfig(), key, null)
                : null;
        if (value == null || value.isEmpty())
            value = settings.getString(key, null);
        return (value != null && !value.isEmpty()) ? value : fallback;
    }

    private static RenderContext create(Component component, String numFontName, String labelFontName, int fontAdd,
            int columnNum) {
        // Calculate font sizes
        int fontSize = 24 + fontAdd;
        Font numFont = new Font(numFontName, Font.BOLD, fontSize);
        Font labelFont = new Font(labelFontName, Font.BOLD, Math.round(fontSize / 2.0f));
        Font unitFont = new Font(numFontName, Font.PLAIN, Math.round(fontSize / 2.0f));

        // Use Toolkit for font metrics to avoid issues with unrealized components.
        // Component.getFontMetrics() can return inaccurate values before setVisible(true).
        int numHeight = java.awt.Toolkit.getDefaultToolkit().getFontMetrics(numFont).getHeight();

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
