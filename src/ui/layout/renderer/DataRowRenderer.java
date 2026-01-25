package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;
import com.alee.extended.button.WebSwitch;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import ui.replica.ReplicaBuilder;

/**
 * Renders DATA type rows (standard data toggles that control row.visible).
 */
public class DataRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        // Fallback checks
        // Always render DATA rows (as they are toggles)
        // boolean isVisible = row.getBool(); // Removed unused check

        String displayVal = context.getStringFromConfigService(row.property != null ? row.property : row.label, "");
        WebPanel itemPanel = ReplicaBuilder.createSwitchItem(row.label, Boolean.parseBoolean(displayVal), false,
                row.desc, row.descImg);
        WebSwitch sw = ReplicaBuilder.getSwitch(itemPanel);

        if (sw != null) {
            sw.addActionListener(e -> {
                context.syncStringToConfigService(row.property != null ? row.property : row.label,
                        String.valueOf(sw.isSelected()));
                context.onSave();
            });
        }

        return itemPanel;
    }
}
