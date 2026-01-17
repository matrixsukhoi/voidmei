package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;
import com.alee.laf.combobox.WebComboBox;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import prog.util.PropertyBinder;
import ui.replica.ReplicaBuilder;

/**
 * Renders COMBO (dropdown) type rows.
 */
public class ComboRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        String[] options = getComboOptions(row.format);
        WebPanel itemPanel = ReplicaBuilder.createDropdownItem(row.label, options);
        WebComboBox combo = ReplicaBuilder.getComboBox(itemPanel);

        if (combo != null) {
            // Set current value
            String currentVal = PropertyBinder.getString(groupConfig, row.property, row.defaultValue);
            if (currentVal != null) {
                combo.setSelectedItem(currentVal);
            }

            final String prop = row.property;
            combo.addActionListener(e -> {
                if (context.isUpdating())
                    return;
                PropertyBinder.setString(groupConfig, prop, (String) combo.getSelectedItem());
                context.onSave();
            });
        }

        return itemPanel;
    }

    private String[] getComboOptions(String optionSource) {
        if (optionSource == null)
            return new String[0];
        if (optionSource.equals("_FONTS_")) {
            return java.awt.GraphicsEnvironment.getLocalGraphicsEnvironment().getAvailableFontFamilyNames();
        }
        return optionSource.split(",");
    }
}
