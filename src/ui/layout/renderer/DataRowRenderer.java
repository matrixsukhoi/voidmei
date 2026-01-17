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
        WebPanel itemPanel = ReplicaBuilder.createSwitchItem(row.label, row.visible, false);
        WebSwitch sw = ReplicaBuilder.getSwitch(itemPanel);

        if (sw != null) {
            sw.addActionListener(e -> {
                row.visible = sw.isSelected();
                context.onSave();
            });
        }

        return itemPanel;
    }
}
