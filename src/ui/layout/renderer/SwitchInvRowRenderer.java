package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;
import com.alee.extended.button.WebSwitch;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import ui.replica.ReplicaBuilder;

/**
 * Renders SWITCH_INV type rows (inverted boolean switch).
 * Switch ON = writes false to ConfigurationService
 * Switch OFF = writes true to ConfigurationService
 * This is for "disable*" config keys where the display logic is inverted.
 */
public class SwitchInvRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        // For inverted switch, we read the value and negate it for display
        // ConfigurationService has "disableX=true" means switch should show OFF
        // ConfigurationService has "disableX=false" means switch should show ON
        // unused variable removed

        WebPanel itemPanel = ReplicaBuilder.createSwitchItem(row.label,
                !context.getFromConfigService(row.property, !row.getBool()), false, row.desc);
        WebSwitch sw = ReplicaBuilder.getSwitch(itemPanel);

        if (sw != null) {
            final String prop = row.property;
            sw.addActionListener(e -> {
                if (context.isUpdating())
                    return;
                boolean newDisplayVal = sw.isSelected();
                // Invert for storage: display ON -> store false, display OFF -> store true
                boolean newConfigVal = !newDisplayVal;
                context.syncToConfigService(prop, newConfigVal);
                row.value = newDisplayVal; // Keep display value in row.value for persistence
                context.onSave();
            });
        }

        return itemPanel;
    }
}
