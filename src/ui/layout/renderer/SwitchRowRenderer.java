package ui.layout.renderer;

import com.alee.laf.panel.WebPanel;
import com.alee.extended.button.WebSwitch;
import prog.config.ConfigLoader.RowConfig;
import prog.config.ConfigLoader.GroupConfig;
import prog.util.GPUCompatibilityHelper;
import ui.replica.ReplicaBuilder;
import ui.util.DialogService;
import com.alee.laf.optionpane.WebOptionPane;

/**
 * Renders SWITCH type rows (bound to a GroupConfig property or row.value).
 * If property binding fails, falls back to storing state in row.value.
 * Also syncs to ConfigurationService for overlay control keys.
 */
public class SwitchRowRenderer implements RowRenderer {

    private static final String GPU_COMPAT_KEY = "gpuCompatibilityMode";

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        boolean defaultVal = row.getBool();
        boolean currentVal;

        // Special case: GPU compatibility mode reads from external properties file
        // because the setting must be applied before AWT loads (by Launcher.java)
        if (GPU_COMPAT_KEY.equals(row.property)) {
            currentVal = GPUCompatibilityHelper.isEnabled();
        } else {
            // 使用统一的配置读取助手
            // 优先级: PropertyBinder → ConfigurationService → 默认值
            currentVal = RendererConfigHelper.readBool(context, groupConfig, row, defaultVal);
        }

        WebPanel itemPanel = ReplicaBuilder.createSwitchItem(row.label, currentVal, false, row.desc, row.descImg);
        WebSwitch sw = ReplicaBuilder.getSwitch(itemPanel);

        if (sw != null) {
            final String prop = row.property;
            sw.addActionListener(e -> {
                if (context.isUpdating())
                    return;
                boolean newVal = sw.isSelected();

                // Special case: GPU compatibility mode saves to external file
                // and requires application restart to take effect
                if (GPU_COMPAT_KEY.equals(prop)) {
                    GPUCompatibilityHelper.saveSettings(newVal);
                    // Show dialog to remind user to restart (more prominent than toast)
                    DialogService.showMessageDialog(
                            itemPanel,
                            "GPU兼容模式设置已更改，请重启VoidMei使设置生效。",
                            "需要重启",
                            WebOptionPane.INFORMATION_MESSAGE);
                    // Still sync to ConfigurationService for consistency
                    context.syncToConfigService(prop, newVal);
                    context.onSave();
                    return;
                }

                // 使用统一的配置写入助手
                // 如果 PropertyBinder 写入失败，存储到 row.value
                if (!RendererConfigHelper.writeBool(context, groupConfig, prop, newVal)) {
                    row.value = newVal;
                }
                context.onSave();
            });
        }

        return itemPanel;
    }
}
