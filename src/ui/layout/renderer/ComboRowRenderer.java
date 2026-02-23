package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;
import com.alee.laf.combobox.WebComboBox;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import prog.util.FileUtils;
import prog.util.Logger;
import ui.replica.ReplicaBuilder;
import java.io.File;
import javax.swing.event.PopupMenuListener;
import javax.swing.event.PopupMenuEvent;

/**
 * Renders COMBO (dropdown) type rows.
 */
public class ComboRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        String[] options = getComboOptions(row.format);
        WebPanel itemPanel = ReplicaBuilder.createDropdownItem(row.label, options, row.desc, row.descImg);
        WebComboBox combo = ReplicaBuilder.getComboBox(itemPanel);

        if (combo != null) {
            // 使用统一的配置读取助手
            // 优先级: PropertyBinder → ConfigurationService → 默认值
            String currentVal = RendererConfigHelper.readString(context, groupConfig, row, row.getStr());

            if (currentVal != null && !currentVal.isEmpty()) {
                combo.setSelectedItem(currentVal);
            }

            // Debug: Track popup open/close events to diagnose auto-close issue on Windows
            final String comboLabel = row.label;
            combo.addPopupMenuListener(new PopupMenuListener() {
                @Override
                public void popupMenuWillBecomeVisible(PopupMenuEvent e) {
                    Logger.info("ComboDebug", "OPEN: " + comboLabel);
                }
                @Override
                public void popupMenuWillBecomeInvisible(PopupMenuEvent e) {
                    Logger.info("ComboDebug", "CLOSE: " + comboLabel);
                    // Print stack trace to find what triggered the close
                    Thread.dumpStack();
                }
                @Override
                public void popupMenuCanceled(PopupMenuEvent e) {
                    Logger.info("ComboDebug", "CANCEL: " + comboLabel);
                }
            });

            final String prop = row.property;
            combo.addActionListener(e -> {
                if (context.isUpdating())
                    return;
                String newVal = (String) combo.getSelectedItem();
                // Update memory model so it saves to ui_layout.cfg
                row.value = newVal;

                // 使用统一的配置写入助手
                RendererConfigHelper.writeString(context, groupConfig, prop, newVal);
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
