package ui.layout.renderer;

import com.alee.laf.button.WebButton;
import com.alee.laf.combobox.WebComboBox;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.filechooser.WebFileChooser;

import prog.audio.VoiceAlertType;
import prog.audio.VoicePackConfig;
import prog.audio.VoiceResourceManager;
import prog.config.ConfigLoader.GroupConfig;
import prog.config.ConfigLoader.RowConfig;
import ui.replica.ReplicaBuilder;

import java.awt.BorderLayout;
import java.awt.Dimension;
import java.io.File;
import java.util.List;
import javax.swing.BorderFactory;
import javax.swing.JFileChooser;

/**
 * Renders VOICE_GLOBAL type rows.
 * [Label] ... [ComboBox (Global Pack)] [Import Button]
 */
public class VoiceGlobalRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        ReplicaBuilder.getStyle().decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        // Label
        WebLabel label = new WebLabel(row.label);
        if ((row.desc != null && !row.desc.isEmpty()) || (row.descImg != null && !row.descImg.isEmpty())) {
            ReplicaBuilder.applyStylizedTooltip(label, row.desc, row.descImg);
        }
        ReplicaBuilder.getStyle().decorateLabel(label);
        panel.add(label, BorderLayout.WEST);

        // Control Container
        WebPanel controls = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 5, 0));
        controls.setOpaque(false);

        // ComboBox (Packs)
        // Refresh packs every time just in case
        List<String> packs = VoiceResourceManager.getInstance().getAvailablePacks();
        WebComboBox combo = new WebComboBox(packs.toArray(new String[0]));
        combo.setEditable(false);
        combo.setPreferredSize(new Dimension(100, 26));

        // Initial selection: Load from config
        String globalPack = "default";
        if (row.property != null) {
            globalPack = context.getStringFromConfigService(row.property, row.getStr());
            if (globalPack == null || globalPack.isEmpty())
                globalPack = "default";
        }
        combo.setSelectedItem(globalPack);

        // Logic: When changed, iterate all config items and update them
        combo.addActionListener(e -> {
            if (context.isUpdating())
                return;
            String selectedPack = (String) combo.getSelectedItem();

            // Save logic: Sync globalPack property first
            if (row.property != null) {
                context.syncStringToConfigService(row.property, selectedPack);
            }

            applyGlobalPack(selectedPack, context);
            context.onSave(); // Trigger save and event bus
        });

        controls.add(combo);

        // Import Button
        WebButton btnImport = new WebButton("Import Zip");
        btnImport.setFont(prog.Application.defaultFont);
        btnImport.setFocusable(false);
        btnImport.addActionListener(e -> {
            WebFileChooser fileChooser = new WebFileChooser();
            fileChooser.setDialogTitle("Select Voice Pack Zip");
            fileChooser.setFileFilter(new javax.swing.filechooser.FileNameExtensionFilter("Zip Files", "zip"));
            fileChooser.setMultiSelectionEnabled(false);

            if (fileChooser.showOpenDialog(panel) == JFileChooser.APPROVE_OPTION) {
                File file = fileChooser.getSelectedFile();
                if (file != null) {
                    VoiceResourceManager.getInstance().installPack(file);
                    // Refresh Combo
                    List<String> newPacks = VoiceResourceManager.getInstance().getAvailablePacks();
                    combo.removeAllItems();
                    for (String p : newPacks)
                        combo.addItem(p);

                    // Publish Refresh Event for other renderers FIRST
                    prog.event.UIStateBus.getInstance().publish(prog.event.UIStateEvents.VOICE_PACKS_REFRESH, null);

                    // Auto-select the new pack if found (Running later to ensure others have
                    // refreshed)
                    String newName = file.getName();
                    if (newName.endsWith(".zip"))
                        newName = newName.substring(0, newName.length() - 4);

                    String finalNewName = newName;
                    javax.swing.SwingUtilities.invokeLater(() -> {
                        for (String p : newPacks) {
                            if (p.equalsIgnoreCase(finalNewName)) {
                                combo.setSelectedItem(p);
                                // The listener on combo handles the apply and save
                                break;
                            }
                        }
                    });
                }
            }
        });
        controls.add(btnImport);

        panel.add(controls, BorderLayout.CENTER);
        panel.putClientProperty("alignLabel", label);

        return panel;
    }

    /**
     * 应用全局语音包设置
     * 使用 VoiceAlertType 枚举获取所有告警 key，避免硬编码列表
     * 使用 VoicePackConfig 解析和序列化配置值
     */
    private void applyGlobalPack(String packName, RenderContext context) {
        // 使用 VoiceAlertType 获取所有告警 key，避免硬编码
        for (String key : VoiceAlertType.getAllKeys()) {
            String configKey = VoicePackConfig.withVoicePrefix(key);

            // Smart Logic: Only switch if the pack actually contains this voice file.
            boolean existsInTarget = VoiceResourceManager.getInstance().hasResourceStrict(key, packName);

            if (existsInTarget) {
                // 使用 VoicePackConfig 解析当前配置，保持启用状态
                String currentVal = context.getStringFromConfigService(configKey, VoicePackConfig.DEFAULT_PACK);
                VoicePackConfig current = VoicePackConfig.parse(currentVal);

                // 保持启用状态，只改包名
                VoicePackConfig updated = current.withPackName(packName);
                context.syncStringToConfigService(configKey, updated.toConfigString());
            }
            // 如果目标包没有该文件，保持原设置不变
        }
    }
}
