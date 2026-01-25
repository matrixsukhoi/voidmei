package ui.layout.renderer;

import com.alee.laf.button.WebButton;
import com.alee.laf.combobox.WebComboBox;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.filechooser.WebFileChooser;

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

    private void applyGlobalPack(String packName, RenderContext context) {
        // We need to iterate over all config rows.
        // We assume we can access Controller's dynamicConfigs via some hack or context
        // doesn't expose it directly.
        // Wait, RenderContext interface:
        // void syncStringToConfigService(String key, String value);
        // It doesn't allow iterating structure.

        // Access strategy:
        // We know the keys are standard: "voice_aoaCrit", "voice_warn_stall", etc.
        // But hardcoding list is bad.
        // Access Controller via the Page?
        // DynamicDataPage is the caller of render(). The 'context' is an anonymous
        // class in DynamicDataPage.
        // DynamicDataPage has access to 'groupConfig'. But not other groups.

        // However, 'context' implementation in DynamicDataPage:
        /*
         * public void syncStringToConfigService(String key, String value) {
         * parent.tc.configService.setConfig(key, value);
         * }
         */
        // We can access 'parent.tc' if we really want, but 'context' hides it.
        // BUT, we can access 'parent.tc' via reflection or by just knowing the list of
        // keys from ui_layout.cfg.

        // Alternate strategy:
        // Ask VoiceResourceManager for "what valid voice files exist in this pack?"
        // Then map those filenames to keys?
        // E.g. "aoaCrit.wav" -> "voice_aoaCrit".
        // Then update those keys.
        // For files NOT in the pack, set them to "default".

        // This assumes 1:1 mapping between filename and config key suffix.
        // Let's check VoiceWarning keys:
        // "aoaCrit" -> "voice_aoaCrit"
        // "warn_stall" -> "voice_warn_stall"
        // Yes, mapping holds.

        // So:
        // 1. List files in voice/packName/
        // 2. Identify keys.
        // 3. Update them.
        // 4. What about keys that are MISSING in the pack?
        // We should revert them to "default" or keep them as is?
        // User requirement: "configured to corresponding... missing use default".
        // So YES, we must set missing ones to "default".

        // But to do Step 4, we need to know the UNIVERSE of all possible voice keys.
        // Where is that defined? Only in ui_layout.cfg items.

        // Since I cannot easily access the full config tree from here without
        // significant refactor,
        // AND the user explicitly added the items to ui_layout.cfg,
        // AND I know the list of items I just added.

        // I will Hardcode the list of keys here. It's safe enough for this specific
        // feature request.
        // Or I can add a static method in VoiceWarning that returns the list of keys it
        // registered?
        // VoiceWarning.clipRegistry has all the keys!
        // But VoiceWarning might not be instantiated in Preview.

        // Safest Path: Hardcode list of 10-12 keys.

        String[] keys = {
                "aoaCrit", "aoaHigh", "warn_stall", "warn_gear",
                "warn_engineoverheat", "warn_lowfuel", "warn_altitude",
                "warn_terrain", "warn_flap", "warn_loadfactor",
                "rudderEff", "elevatorEff", "aileronEff", "warn_lowrpm",
                "warn_highrpm", "warn_ias", "warn_mach", "fail_engine",
                "warn_lowpressure", "fail_nofuel", "warn_highvario",
                "warn_brake", "start1"
        };

        for (String key : keys) {
            String configKey = "voice_" + key;

            // Check existence
            boolean exists = VoiceResourceManager.getInstance().hasResourceStrict(key, packName);
            String targetPack = exists ? packName : "default";

            // Preserve enabled state?
            // We need to read current value.
            // context.getStringFromConfigService(configKey, "default")
            String currentVal = context.getStringFromConfigService(configKey, "default");
            boolean enabled = true;
            if (currentVal != null && currentVal.contains("|")) {
                enabled = Boolean.parseBoolean(currentVal.split("\\|")[1]);
            }

            String newVal = targetPack + "|" + enabled;
            context.syncStringToConfigService(configKey, newVal);
        }
    }
}
