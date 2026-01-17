package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;
import com.alee.extended.button.WebSwitch;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import prog.util.PropertyBinder;
import ui.replica.ReplicaBuilder;

/**
 * Renders SWITCH type rows (bound to a GroupConfig property or row.visible).
 * If property binding fails, falls back to storing state in row.visible.
 * Also syncs to ConfigurationService for overlay control keys.
 */
public class SwitchRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        // Priority for initial value:
        // 1. If property exists in GroupConfig, use PropertyBinder
        // 2. Otherwise try ConfigurationService (for overlay control keys like
        // enableAxis)
        // 3. Fallback to row.visible (from config file)
        boolean currentVal;
        if (row.property != null && PropertyBinder.hasField(groupConfig, row.property)) {
            // Property exists in GroupConfig (e.g., fontSize, columns)
            currentVal = PropertyBinder.getBool(groupConfig, row.property, row.visible);
        } else if (row.property != null) {
            // Property not in GroupConfig, try ConfigurationService (overlay control keys)
            currentVal = context.getFromConfigService(row.property, row.visible);
        } else {
            currentVal = row.visible;
        }

        WebPanel itemPanel = ReplicaBuilder.createSwitchItem(row.label, currentVal, false);
        WebSwitch sw = ReplicaBuilder.getSwitch(itemPanel);

        if (sw != null) {
            final String prop = row.property;
            sw.addActionListener(e -> {
                if (context.isUpdating())
                    return;
                boolean newVal = sw.isSelected();
                // Try property binding first, if fails, store in row.visible
                if (!PropertyBinder.setBool(groupConfig, prop, newVal)) {
                    row.visible = newVal;
                }
                // Always sync to ConfigurationService for overlay control
                // This bridges ui_layout.cfg to the overlay system
                if (prop != null) {
                    context.syncToConfigService(prop, newVal);
                }
                context.onSave();
            });
        }

        return itemPanel;
    }
}
