package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;
import com.alee.laf.combobox.WebComboBox;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import prog.util.PropertyBinder;
import prog.util.FileUtils;
import ui.replica.ReplicaBuilder;
import java.io.File;

/**
 * Renders COMBO (dropdown) type rows.
 */
public class ComboRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        String[] options = getComboOptions(row.format);
        WebPanel itemPanel = ReplicaBuilder.createDropdownItem(row.label, options, row.desc);
        WebComboBox combo = ReplicaBuilder.getComboBox(itemPanel);

        if (combo != null) {
            // Priority for initial value:
            // 1. If property exists in GroupConfig, use PropertyBinder
            // 2. Otherwise try ConfigurationService
            // 3. Fallback to row.value (from config file)
            String currentVal;
            if (row.property != null && PropertyBinder.hasField(groupConfig, row.property)) {
                currentVal = PropertyBinder.getString(groupConfig, row.property, row.getStr());
            } else if (row.property != null) {
                currentVal = context.getStringFromConfigService(row.property, row.getStr());
            } else {
                currentVal = row.getStr();
            }

            if (currentVal != null && !currentVal.isEmpty()) {
                combo.setSelectedItem(currentVal);
            }

            final String prop = row.property;
            combo.addActionListener(e -> {
                if (context.isUpdating())
                    return;
                String newVal = (String) combo.getSelectedItem();
                // Update memory model so it saves to ui_layout.cfg
                row.value = newVal;

                // Try property binding first
                if (!PropertyBinder.setString(groupConfig, prop, newVal)) {
                    // Fallback or ignore
                }
                // Always sync to ConfigurationService
                if (prop != null) {
                    context.syncStringToConfigService(prop, newVal);
                }
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
        if (optionSource.equals("_CROSSHAIRS_")) {
            File dir = new File("image/gunsight");
            String[] files = dir.list();
            if (files == null)
                files = new String[0];
            files = FileUtils.getFilelistNameNoEx(files);

            String[] combined = new String[files.length + 1];
            combined[0] = "软件渲染准星";
            System.arraycopy(files, 0, combined, 1, files.length);
            return combined;
        }
        return optionSource.split(",");
    }
}
