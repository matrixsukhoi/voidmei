package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;
import com.alee.extended.button.WebSwitch;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import prog.util.PropertyBinder;
import ui.replica.ReplicaBuilder;

/**
 * Renders SWITCH type rows (bound to a GroupConfig property).
 */
public class SwitchRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        boolean currentVal = PropertyBinder.getBool(groupConfig, row.property, row.visible);
        WebPanel itemPanel = ReplicaBuilder.createSwitchItem(row.label, currentVal, false);
        WebSwitch sw = ReplicaBuilder.getSwitch(itemPanel);

        if (sw != null) {
            final String prop = row.property;
            sw.addActionListener(e -> {
                if (context.isUpdating())
                    return;
                PropertyBinder.setBool(groupConfig, prop, sw.isSelected());
                context.onSave();
            });
        }

        return itemPanel;
    }
}
