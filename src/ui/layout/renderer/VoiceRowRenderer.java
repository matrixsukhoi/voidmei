package ui.layout.renderer;

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
 * [Label] ... [ComboBox (Pack)] [Play Button]
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
        combo.setPreferredSize(new Dimension(100, 26)); // Slightly wider for pack names

        // Determine current value
        String currentVal = null;
        if (row.property != null) { // property maps to "voice_xxx"
            currentVal = context.getStringFromConfigService(row.property, row.getStr());
        }
        if (currentVal == null || currentVal.isEmpty()) {
            currentVal = "default";
        }
        combo.setSelectedItem(currentVal);

        // Listener
        combo.addActionListener(e -> {
            if (context.isUpdating())
                return;
            String newVal = (String) combo.getSelectedItem();

            // Sync to config
            if (row.property != null) {
                context.syncStringToConfigService(row.property, newVal);
            }
            context.onSave();
        });
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
            // Load and play
            Clip clip = VoiceResourceManager.getInstance().loadClip(key, pack);
            if (clip != null) {
                clip.setFramePosition(0);
                clip.start();
            }
        });
        controls.add(btnPlay);

        panel.add(controls, BorderLayout.CENTER);

        // Critical: Enable ResponsiveGrid alignment
        panel.putClientProperty("alignLabel", label);

        return panel;
    }
}
