package ui.layout.renderer;

import java.io.File;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.combobox.WebComboBox;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import prog.util.FileUtils;
import ui.replica.ReplicaBuilder;

/**
 * Renders FILELIST type rows (file selection dropdown).
 * Format: FILELIST:configKey:directoryPath
 * Lists files from the specified directory as dropdown options.
 */
public class FileListRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        // Parse directory path from format field (set during config parsing)
        String dirPath = row.format;
        if (dirPath == null || dirPath.isEmpty()) {
            dirPath = ".";
        }

        // Get file list
        File dir = new File(dirPath);
        String[] files = dir.list();
        if (files == null)
            files = new String[0];
        files = FileUtils.getFilelistNameNoEx(files);

        // Get current value from ConfigurationService
        String currentVal = context.getStringFromConfigService(row.property, row.defaultValue);

        // Create combo box with file list
        WebPanel itemPanel = ReplicaBuilder.createDropdownItem(row.label, files);
        WebComboBox combo = ReplicaBuilder.getComboBox(itemPanel);

        if (combo != null) {
            // Set initial selection
            if (currentVal != null && !currentVal.isEmpty()) {
                combo.setSelectedItem(currentVal);
            }

            final String prop = row.property;
            combo.addActionListener(e -> {
                if (context.isUpdating())
                    return;
                Object selected = combo.getSelectedItem();
                if (selected != null) {
                    context.syncStringToConfigService(prop, selected.toString());
                }
                context.onSave();
            });
        }

        return itemPanel;
    }
}
