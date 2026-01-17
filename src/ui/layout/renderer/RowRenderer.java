package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;

/**
 * Strategy interface for rendering different types of config rows.
 * Each row type (SLIDER, COMBO, SWITCH, DATA, HEADER) has its own renderer.
 */
public interface RowRenderer {

    /**
     * Renders a config row into a UI component.
     * 
     * @param row         The row configuration
     * @param groupConfig The parent group configuration (for property binding)
     * @param context     Rendering context with callbacks
     * @return The rendered panel, or null if this row should not produce a
     *         component
     */
    WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context);

    /**
     * Context object providing callbacks and state for rendering.
     */
    interface RenderContext {
        /** Called when user changes a value and config should be saved */
        void onSave();

        /** Called when layout needs to be rebuilt (e.g., panelColumns changed) */
        void onRebuild();

        /** Returns true if we're in the middle of programmatic updates */
        boolean isUpdating();

        /** Syncs a boolean value to ConfigurationService (for overlay control) */
        void syncToConfigService(String key, boolean value);

        /** Gets a boolean value from ConfigurationService (for initial state) */
        boolean getFromConfigService(String key, boolean defaultVal);

        /** Syncs a string value to ConfigurationService */
        void syncStringToConfigService(String key, String value);

        /** Gets a string value from ConfigurationService (for initial state) */
        String getStringFromConfigService(String key, String defaultVal);
    }
}
