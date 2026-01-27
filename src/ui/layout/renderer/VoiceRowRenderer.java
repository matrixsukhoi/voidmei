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
        // ComboBox (Packs)
        // Filter logic: Only show packs that have this specific voice key
        String voiceKey = row.property;
        if (voiceKey != null && voiceKey.startsWith("voice_")) {
            voiceKey = voiceKey.substring(6);
        }

        java.util.List<String> allPacks = VoiceResourceManager.getInstance().getAvailablePacks();
        java.util.List<String> validPacks = new java.util.ArrayList<>();

        // Always include default (or check if default has it? Usually default should
        // have it, or at least be an option)
        // Let's always add default first.
        validPacks.add("default"); // Assuming 'default' is always valid fallback/base

        for (String p : allPacks) {
            if ("default".equals(p))
                continue;
            if (VoiceResourceManager.getInstance().hasResourceStrict(voiceKey, p)) {
                validPacks.add(p);
            }
        }

        WebComboBox combo = new WebComboBox(validPacks.toArray(new String[0]));
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
            String pKey = row.property;
            if (pKey != null && pKey.startsWith("voice_")) {
                pKey = pKey.substring(6);
            }
            // Load and play (ignoring enable state for preview)
            Clip clip = VoiceResourceManager.getInstance().loadClip(pKey, pack);
            if (clip != null) {
                clip.setFramePosition(0);
                clip.start();
            }
        });
        controls.add(btnPlay);

        // Status Label (for warning)
        WebLabel statusLabel = new WebLabel();
        statusLabel.setMargin(0, 5, 0, 0); // Spacing
        controls.add(statusLabel);

        // Validation logic
        Runnable validate = () -> {
            String validationPack = (String) combo.getSelectedItem();
            if (validationPack == null)
                validationPack = "default";

            String vKey = row.property;
            if (vKey != null && vKey.startsWith("voice_")) {
                vKey = vKey.substring(6);
            }

            // Should prompt warning if:
            // 1. Pack is default, and default missing file.
            // 2. Pack is NOT default, and pack missing file (even if default exists? user
            // might want to know pack is incomplete)
            // But wait, if pack missing, system plays default.
            // User requirement: "If a voice has no corresponding file... hint special way".
            // Since VoiceResourceManager.hasResourceStrict checks specific pack.

            boolean missing = !VoiceResourceManager.getInstance().hasResourceStrict(vKey, validationPack);

            // If checking strict on custom pack, and it's missing, it will fallback to
            // default.
            // So technically it's not "broken" if default exists.
            // But user might want to know "Jarvis doesn't have this file".
            // Let's check effective playability:
            // If custom pack missing, check default.

            boolean effectivelyMissing = false;
            String tooltip = null;

            if (missing) {
                if ("default".equals(validationPack)) {
                    effectivelyMissing = true;
                    tooltip = "Missing audio file in default pack!";
                } else {
                    // Check fallback
                    boolean fallbackExists = VoiceResourceManager.getInstance().hasResourceStrict(vKey, "default");
                    if (!fallbackExists) {
                        effectivelyMissing = true;
                        tooltip = "Missing audio file in '" + validationPack + "' and default pack!";
                    } else {
                        // It's missing in pack but exists in default.
                        // Should we warn? "Using default fallback".
                        // User screenshot suggests "default" was shown in combo.
                        // If user selects "Jarvis", and it's missing, GlobalRenderer logic sets value
                        // to "default".
                        // So validationPack IS "default".
                        // So logic branch 1 is taken.
                    }
                }
            }

            if (effectivelyMissing) {
                statusLabel.setText("⚠️"); // Or use icon
                statusLabel.setForeground(java.awt.Color.RED);

                // Enhanced WebPopup tooltip
                String fileName = vKey + ".wav";
                String html = "<html><div style='width:200px;'>" +
                        "<b>Missing File Warning</b><br>" +
                        "Expected File: <span style='color:blue;'>" + fileName + "</span><br><br>" +
                        tooltip +
                        "</div></html>";

                // Remove existing listeners
                for (java.awt.event.MouseListener ml : statusLabel.getMouseListeners()) {
                    statusLabel.removeMouseListener(ml);
                }

                ReplicaBuilder.applyHtmlTooltip(statusLabel, html);

            } else {
                statusLabel.setText("");
                statusLabel.setToolTipText(null);
                // Remove listeners
                for (java.awt.event.MouseListener ml : statusLabel.getMouseListeners()) {
                    statusLabel.removeMouseListener(ml);
                }
            }
        };

        // Initial check
        validate.run();

        // Add validation to listeners
        java.awt.event.ActionListener validatingListener = e -> validate.run();
        combo.addActionListener(validatingListener);

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
                        combo.removeActionListener(validatingListener);
                        enableSwitch.removeActionListener(stateSaver);

                        combo.setSelectedItem(finalPack);
                        enableSwitch.setSelected(finalEn);

                        combo.addActionListener(stateSaver);
                        combo.addActionListener(validatingListener);
                        enableSwitch.addActionListener(stateSaver);

                        // Re-run validation
                        validate.run();
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

        // Subscribe to Pack Refresh events to update combo box items
        java.util.function.Consumer<Object> refreshHandler = ignore -> {
            java.util.List<String> freshPacks = VoiceResourceManager.getInstance().getAvailablePacks();
            // Filter again
            java.util.List<String> refreshedValidPacks = new java.util.ArrayList<>();
            refreshedValidPacks.add("default");

            String rKey = row.property;
            if (rKey != null && rKey.startsWith("voice_")) {
                rKey = rKey.substring(6);
            }

            for (String p : freshPacks) {
                if ("default".equals(p))
                    continue;
                if (VoiceResourceManager.getInstance().hasResourceStrict(rKey, p)) {
                    refreshedValidPacks.add(p);
                }
            }

            javax.swing.SwingUtilities.invokeLater(() -> {
                // Mute listeners to prevent infinite loop
                combo.removeActionListener(stateSaver);
                combo.removeActionListener(validatingListener);

                // Preserve selection if possible
                String current = (String) combo.getSelectedItem();
                combo.removeAllItems();
                for (String p : refreshedValidPacks)
                    combo.addItem(p);

                // Restore selection
                boolean found = false;
                for (String p : freshPacks) {
                    if (p.equals(current)) {
                        combo.setSelectedItem(p);
                        found = true;
                        break;
                    }
                }
                if (!found)
                    combo.setSelectedItem("default");

                // Restore listeners
                combo.addActionListener(stateSaver);
                combo.addActionListener(validatingListener);

                // Re-validate
                validate.run();
            });
        };
        prog.event.UIStateBus.getInstance().subscribe(prog.event.UIStateEvents.VOICE_PACKS_REFRESH, refreshHandler);

        // Determine cleanup for refresh handler (reusing hierarchy listener if
        // possible)
        panel.addHierarchyListener(e -> {
            if ((e.getChangeFlags() & java.awt.event.HierarchyEvent.DISPLAYABILITY_CHANGED) != 0) {
                if (!panel.isDisplayable()) {
                    prog.event.UIStateBus.getInstance().unsubscribe(prog.event.UIStateEvents.VOICE_PACKS_REFRESH,
                            refreshHandler);
                }
            }
        });

        return panel;
    }
}
