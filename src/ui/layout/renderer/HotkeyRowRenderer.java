package ui.layout.renderer;

import java.awt.BorderLayout;
import java.awt.Dimension;

import com.alee.laf.button.WebButton;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.github.kwhat.jnativehook.GlobalScreen;
import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent;
import com.github.kwhat.jnativehook.keyboard.NativeKeyListener;

import prog.config.ConfigLoader.GroupConfig;
import prog.config.ConfigLoader.RowConfig;
import ui.replica.ReplicaBuilder;

public class HotkeyRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        ReplicaBuilder.getStyle().decorateControlPanel(panel);
        panel.setBorder(javax.swing.BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(row.label);
        if ((row.desc != null && !row.desc.isEmpty()) || (row.descImg != null && !row.descImg.isEmpty())) {
            ReplicaBuilder.applyStylizedTooltip(label, row.desc, row.descImg);
        }
        ReplicaBuilder.getStyle().decorateLabel(label);
        panel.add(label, BorderLayout.WEST);

        // Get current key code from config
        String val = context.getStringFromConfigService(row.property, "0");
        int keyCode = 0;
        try {
            keyCode = Integer.parseInt(val);
        } catch (NumberFormatException e) {
            keyCode = 0;
        }

        String keyText = (keyCode == 0) ? "None" : NativeKeyEvent.getKeyText(keyCode);
        WebButton btn = new WebButton(keyText);
        btn.setPreferredSize(new Dimension(100, 30));
        btn.setFocusable(false); // specialized button style?

        // Use a final wrapper for the key code to allow updating
        final int[] currentKey = { keyCode };

        btn.addActionListener(e -> {
            btn.setText("Press Key...");

            // Add Native Hook Listener
            GlobalScreen.addNativeKeyListener(new NativeKeyListener() {
                @Override
                public void nativeKeyPressed(NativeKeyEvent ev) {
                    int code = ev.getKeyCode();
                    // Ignore modifier keys if desired, or allow them.
                    // Original code ignored Lock keys
                    if (code == NativeKeyEvent.VC_NUM_LOCK || code == NativeKeyEvent.VC_CAPS_LOCK
                            || code == NativeKeyEvent.VC_SCROLL_LOCK) {
                        return;
                    }

                    // Update State
                    currentKey[0] = code;
                    String newText = NativeKeyEvent.getKeyText(code);

                    // Update UI (must be on EDT)
                    javax.swing.SwingUtilities.invokeLater(() -> {
                        btn.setText(newText);
                        context.syncStringToConfigService(row.property, Integer.toString(code));
                        row.value = Integer.toString(code);
                        context.onSave();
                    });

                    // Remove listener
                    GlobalScreen.removeNativeKeyListener(this);
                }
            });
        });

        panel.add(btn, BorderLayout.EAST);

        // Critical: Enable ResponsiveGrid alignment
        panel.putClientProperty("alignLabel", label);

        return panel;
    }

}
