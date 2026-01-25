package ui.layout.renderer;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.button.WebButton;
import com.alee.laf.combobox.WebComboBox;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import prog.audio.VoiceResourceManager;
import prog.config.ConfigLoader.GroupConfig;
import prog.config.ConfigLoader.RowConfig;
import ui.replica.ReplicaBuilder;

import java.awt.BorderLayout;
import java.awt.Dimension;
import javax.sound.sampled.Clip;
import javax.swing.BorderFactory;

/**
 * Renders VOICE type rows.
 * [Label] ... [ComboBox (Pack)] [Play Button] [Switch]
 */
public class VoiceRowRenderer implements RowRenderer {

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
        java.util.List<String> packs = VoiceResourceManager.getInstance().getAvailablePacks();
        WebComboBox combo = new WebComboBox(packs.toArray(new String[0]));
        combo.setEditable(false);
        combo.setPreferredSize(new Dimension(100, 26));

        // Switch
        WebSwitch enableSwitch = new WebSwitch();
        enableSwitch.setRound(4);
        ReplicaBuilder.getStyle().decorateSwitch(enableSwitch);

        // Determine current value parsing
        // Format: "packName|enabled" or just "packName" (default enabled)
        String rawVal = null;
        if (row.property != null) {
            rawVal = context.getStringFromConfigService(row.property, row.getStr());
        }

        String currentPack = "default";
        boolean isEnabled = true;

        if (rawVal != null && !rawVal.isEmpty()) {
            if (rawVal.contains("|")) {
                String[] parts = rawVal.split("\\|");
                currentPack = parts[0];
                if (parts.length > 1)
                    isEnabled = Boolean.parseBoolean(parts[1]);
            } else {
                currentPack = rawVal;
            }
        }

        combo.setSelectedItem(currentPack);
        enableSwitch.setSelected(isEnabled);

        // Listeners
        java.awt.event.ActionListener stateSaver = e -> {
            if (context.isUpdating())
                return;
            String newPack = (String) combo.getSelectedItem();
            boolean newEnabled = enableSwitch.isSelected();
            String newVal = newPack + "|" + newEnabled;

            // Sync to config
            if (row.property != null) {
                context.syncStringToConfigService(row.property, newVal);
            }
            context.onSave();
        };

        combo.addActionListener(stateSaver);
        enableSwitch.addActionListener(stateSaver);

        controls.add(combo);

        // Play Button
        WebButton btnPlay = new WebButton("▶");
        btnPlay.setFont(prog.Application.defaultFont);
        btnPlay.setPreferredSize(new Dimension(30, 26));
        btnPlay.setFocusable(false);
        btnPlay.addActionListener(e -> {
            String pack = (String) combo.getSelectedItem();
            String key = row.property;
            if (key != null && key.startsWith("voice_")) {
                key = key.substring(6);
            }
            // Load and play (ignoring enable state for preview)
            Clip clip = VoiceResourceManager.getInstance().loadClip(key, pack);
            if (clip != null) {
                clip.setFramePosition(0);
                clip.start();
            }
        });
        controls.add(btnPlay);

        controls.add(enableSwitch);

        panel.add(controls, BorderLayout.CENTER);
        panel.putClientProperty("alignLabel", label);

        // Subscribe to update events
        if (row.property != null) {
            java.util.function.Consumer<Object> updateHandler = key -> {
                if (key instanceof String && ((String) key).equals(row.property)) {
                    // Update UI from config
                    String val = context.getStringFromConfigService(row.property, row.getStr());
                    String pack = "default";
                    boolean en = true;
                    if (val != null && !val.isEmpty()) {
                        if (val.contains("|")) {
                            String[] parts = val.split("\\|");
                            pack = parts[0];
                            if (parts.length > 1)
                                en = Boolean.parseBoolean(parts[1]);
                        } else {
                            pack = val;
                        }
                    }

                    // Update UI components on EDT
                    String finalPack = pack;
                    boolean finalEn = en;
                    javax.swing.SwingUtilities.invokeLater(() -> {
                        // Temporary disable listener to avoid loop
                        combo.removeActionListener(stateSaver);
                        enableSwitch.removeActionListener(stateSaver);

                        combo.setSelectedItem(finalPack);
                        enableSwitch.setSelected(finalEn);

                        combo.addActionListener(stateSaver);
                        enableSwitch.addActionListener(stateSaver);
                    });
                }
            };

            prog.event.UIStateBus.getInstance().subscribe(prog.event.UIStateEvents.CONFIG_CHANGED, updateHandler);

            // Clean up listener when panel is removed?
            // Swing doesn't have a great destroy hook for lightweight components.
            // But we can use HierarchyListener.
            panel.addHierarchyListener(e -> {
                if ((e.getChangeFlags() & java.awt.event.HierarchyEvent.DISPLAYABILITY_CHANGED) != 0) {
                    if (!panel.isDisplayable()) {
                        prog.event.UIStateBus.getInstance().unsubscribe(prog.event.UIStateEvents.CONFIG_CHANGED,
                                updateHandler);
                    }
                }
            });
        }

        return panel;
    }
}
