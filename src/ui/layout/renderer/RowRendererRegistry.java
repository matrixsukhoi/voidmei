package ui.layout.renderer;

import java.util.HashMap;
import java.util.Map;

/**
 * Registry for row renderers. Maps row type strings to renderer instances.
 */
public class RowRendererRegistry {

    private static final Map<String, RowRenderer> renderers = new HashMap<>();
    private static final RowRenderer defaultRenderer = new DataRowRenderer();

    static {
        renderers.put("SLIDER", new SliderRowRenderer());
        renderers.put("COMBO", new ComboRowRenderer());
        renderers.put("SWITCH", new SwitchRowRenderer());
        renderers.put("SWITCH_INV", new SwitchInvRowRenderer());
        renderers.put("DATA", new DataRowRenderer());
        // Note: HEADER is handled specially in the layout loop, not as a renderer
    }

    /**
     * Gets the renderer for a given row type.
     * 
     * @param rowType The row type string (SLIDER, COMBO, SWITCH, DATA)
     * @return The appropriate renderer, or default DataRowRenderer if not found
     */
    public static RowRenderer get(String rowType) {
        return renderers.getOrDefault(rowType, defaultRenderer);
    }

    /**
     * Registers a custom renderer for a row type.
     */
    public static void register(String rowType, RowRenderer renderer) {
        renderers.put(rowType, renderer);
    }
}
