package ui.overlay;

import java.awt.Toolkit;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

import ui.util.ConfigLoader;
import prog.util.FormulaEvaluator;
import prog.controller;

/**
 * Dynamic overlay that displays formula-evaluated data using BaseOverlay's
 * pluggable rendering.
 */
public class DynamicOverlay extends BaseOverlay implements BaseOverlay.ConfigBridge {

    private ConfigLoader.GroupConfig config;
    private controller tc;
    private List<OverlayBinding> bindings = new ArrayList<>();

    private static class OverlayBinding {
        String label;
        String formula;
        String format;
        String currentValue = "";
        boolean isHeader = false;
        boolean visible = true;

        OverlayBinding(String label, String formula, String format, boolean isHeader, boolean visible) {
            this.label = label;
            this.formula = formula;
            this.format = format;
            this.isHeader = isHeader;
            this.visible = visible;
        }
    }

    public DynamicOverlay(controller tc, ConfigLoader.GroupConfig config) {
        super();
        this.tc = tc;
        this.config = config;
        rebuildBindings();

        // Calculate position from relative config
        java.awt.Dimension screen = Toolkit.getDefaultToolkit().getScreenSize();
        int initialX = (int) (config.x * screen.width);
        int initialY = (int) (config.y * screen.height);

        // Initialize using BaseOverlay's init
        super.init(this,
                "dynamicOverlay_" + config.title + "_X",
                "dynamicOverlay_" + config.title + "_Y",
                this::generateDisplayLines);

        // Configure custom header matcher for this overlay
        setHeaderMatcher(line -> {
            if (line.equals(config.title))
                return true;
            for (OverlayBinding b : bindings) {
                if (b.isHeader && line.equals(b.label))
                    return true;
            }
            return false;
        });

        // Set alpha from config
        setAlpha(config.alpha);

        // Override position from config (BaseOverlay.init sets default bottom-right)
        this.setLocation(initialX, initialY);

        // Set font from config
        if (config.fontName != null && !config.fontName.isEmpty()) {
            this.fontName = config.fontName;
            setupFont();
        }

        // Initial visibility from config
        setVisible(config.visible);
    }

    /**
     * Initialize for preview mode (with drag support)
     */
    public void initForPreview() {
        java.awt.Dimension screen = Toolkit.getDefaultToolkit().getScreenSize();
        int initialX = (int) (config.x * screen.width);
        int initialY = (int) (config.y * screen.height);

        super.initPreview(this,
                "dynamicOverlay_" + config.title + "_X",
                "dynamicOverlay_" + config.title + "_Y",
                this::generateDisplayLines);

        this.setLocation(initialX, initialY);

        if (config.fontName != null && !config.fontName.isEmpty()) {
            this.fontName = config.fontName;
            setupFont();
        }
    }

    /**
     * Generates display lines from bindings for ListOverlay to render.
     */
    private List<String> generateDisplayLines() {
        updateBindingValues();

        List<String> lines = new ArrayList<>();
        // Title row
        lines.add(config.title);

        for (OverlayBinding b : bindings) {
            if (!b.visible)
                continue;
            if (b.isHeader) {
                lines.add(b.label);
            } else {
                // Format: "Label: Value" or use padding for alignment
                lines.add(b.label + ": " + b.currentValue);
            }
        }
        return lines;
    }

    /**
     * Update binding values from formula evaluation.
     */
    private void updateBindingValues() {
        if (tc.blkx == null)
            return;

        Map<String, Object> vars = tc.blkx.getVariableMap();

        for (OverlayBinding b : bindings) {
            if (!b.visible || b.isHeader)
                continue;
            try {
                Object result = FormulaEvaluator.evaluate(b.formula, vars);
                if (result instanceof Number) {
                    b.currentValue = String.format(b.format, ((Number) result).doubleValue());
                } else if (result instanceof Map) {
                    Map<?, ?> map = (Map<?, ?>) result;
                    b.currentValue = String.format(b.format, map.values().toArray());
                } else if (result instanceof List) {
                    List<?> list = (List<?>) result;
                    b.currentValue = String.format(b.format, list.toArray());
                } else if (result != null && result.getClass().isArray()) {
                    if (result instanceof Object[]) {
                        b.currentValue = String.format(b.format, (Object[]) result);
                    } else {
                        b.currentValue = String.valueOf(result);
                    }
                } else {
                    b.currentValue = String.valueOf(result);
                }
            } catch (Exception e) {
                b.currentValue = "Err";
            }
        }
    }

    public void rebuildBindings() {
        bindings.clear();
        for (ConfigLoader.RowConfig row : config.rows) {
            boolean isHeader = row.formula != null && row.formula.trim().equalsIgnoreCase("HEADER");
            bindings.add(new OverlayBinding(row.label, row.formula, row.format, isHeader, row.visible));
        }
        lastData = null; // Force refresh
    }

    public void toggleVisibility() {
        setVisible(!isVisible());
    }

    public ConfigLoader.GroupConfig getGroupConfig() {
        return config;
    }

    // --- ListOverlay.ConfigBridge Implementation ---

    @Override
    public String getConfig(String key) {
        return tc.getconfig(key);
    }

    @Override
    public void setConfig(String key, String value) {
        tc.setconfig(key, value);
    }

    // --- BaseOverlay Overrides ---

    @Override
    protected boolean isVisibleNow() {
        return config.visible;
    }

    @Override
    public void saveCurrentPosition() {
        super.saveCurrentPosition();
        // Also update config for persistence
        java.awt.Dimension screen = Toolkit.getDefaultToolkit().getScreenSize();
        config.x = (double) getX() / screen.width;
        config.y = (double) getY() / screen.height;
    }
}
