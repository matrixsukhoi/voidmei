package ui.layout;

import java.awt.Dimension;
import java.awt.Graphics;
import java.awt.Graphics2D;
// import java.awt.Dimension; // Unused
// import java.awt.Graphics; // Unused
// import java.awt.Graphics2D; // Unused

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent;

import prog.Application;
import ui.MainForm;
import prog.i18n.Lang;
import com.alee.laf.button.WebButton;
import com.alee.extended.button.WebSwitch;

public class DynamicDataPage extends BasePage {

    private ZoomPanel scaler;
    private prog.config.ConfigLoader.GroupConfig groupConfig;
    private boolean overlayVisible = false;

    public DynamicDataPage(MainForm parent, prog.config.ConfigLoader.GroupConfig groupConfig) {
        super(parent);
        this.groupConfig = groupConfig;

        if (groupConfig != null) {
            this.overlayVisible = groupConfig.visible;
            // Since BasePage constructor calls createTopToolbar() BEFORE we set
            // overlayVisible,
            // we must refresh the toolbar to sync the switch State.
            refreshToolbar();
        }

        rebuild();

        // Restore overlay State
        // Always call this to ensure State is synchronized
        setOverlayVisible(overlayVisible);
    }

    public DynamicDataPage(MainForm parent) {
        super(parent);
    }

    public void setGroupConfig(prog.config.ConfigLoader.GroupConfig groupConfig) {
        this.groupConfig = groupConfig;
        if (groupConfig != null) {
            this.overlayVisible = groupConfig.visible;
            refreshToolbar();
        }
        rebuild();
    }

    public prog.config.ConfigLoader.GroupConfig getGroupConfig() {
        return groupConfig;
    }

    @Override
    protected void initContent(WebPanel content) {
        scaler = new ZoomPanel();
        scaler.setOpaque(false);
        scaler.setLayout(new com.alee.extended.layout.VerticalFlowLayout());
    }

    private void rebuild() {
        scaler.removeAll();
        scaler.setFocusable(true);

        if (groupConfig == null)
            return;

        rebuildSimple();

        scaler.revalidate();
        scaler.repaint();
        if (parent != null) {
            parent.updateDynamicSize();
        }
    }

    private void rebuildSimple() {
        // Use vertical layout to stack groups with minimal spacing
        scaler.setLayout(new com.alee.extended.layout.VerticalFlowLayout(
                com.alee.extended.layout.VerticalFlowLayout.TOP, 0, 3, true, false));

        WebPanel currentGroup = null;
        WebLabel currentHeader = null;

        for (prog.config.ConfigLoader.RowConfig row : groupConfig.rows) {
            boolean isHeader = row.formula != null && row.formula.equalsIgnoreCase("HEADER");

            if (isHeader) {
                // Finalize previous group if exists
                if (currentGroup != null && currentHeader != null) {
                    addGroupToScaler(currentHeader, currentGroup);
                }

                // Create new header
                currentHeader = new WebLabel(row.label);
                currentHeader.setFont(prog.Application.defaultFontBig);
                currentHeader.setForeground(new java.awt.Color(180, 30, 0));
                currentHeader.setBorder(
                        javax.swing.BorderFactory.createEmptyBorder(3, 5, 2, 5));

                // Create new group container with 4-column responsive grid
                currentGroup = new WebPanel();
                currentGroup.setLayout(new ResponsiveGridLayout(2, 15, 3));
                currentGroup.setOpaque(false);
                currentGroup.setBorder(javax.swing.BorderFactory.createEmptyBorder(2, 10, 5, 10));
            } else {
                // Add switch to current group
                if (currentGroup == null) {
                    // If no header yet, create a default group
                    currentHeader = new WebLabel("配置项");
                    currentHeader.setFont(prog.Application.defaultFontBig);
                    currentHeader.setForeground(new java.awt.Color(180, 30, 0));
                    currentHeader.setBorder(
                            javax.swing.BorderFactory.createEmptyBorder(3, 5, 2, 5));

                    currentGroup = new WebPanel();
                    currentGroup.setLayout(new ResponsiveGridLayout(2, 15, 3));
                    currentGroup.setOpaque(false);
                    currentGroup.setBorder(javax.swing.BorderFactory.createEmptyBorder(2, 10, 5, 10));
                }

                WebSwitch sw = UIBuilder.addSwitch(currentGroup, row.label, row.visible);
                sw.addActionListener(e -> {
                    row.visible = sw.isSelected();
                    save();
                });
            }
        }

        // Add the last group
        if (currentGroup != null && currentHeader != null) {
            addGroupToScaler(currentHeader, currentGroup);
        }
    }

    private void addGroupToScaler(WebLabel header, WebPanel group) {
        // Create a container for the header + group
        WebPanel section = new WebPanel();
        section.setLayout(new java.awt.BorderLayout(0, 2));
        section.setOpaque(false);

        // Minimal padding since red header provides clear visual separation
        section.setBorder(javax.swing.BorderFactory.createEmptyBorder(3, 5, 3, 5));

        section.add(header, java.awt.BorderLayout.NORTH);
        section.add(group, java.awt.BorderLayout.CENTER);

        scaler.add(section);
    }

    @Override
    protected WebPanel createTopToolbar() {
        // Get the styled toolbar container from BasePage
        WebPanel toolbar = super.createTopToolbar();
        // Use our custom ResponsiveGridLayout for elegant multi-column layout
        // 3 columns, 10px horizontal gap, 10px vertical gap
        toolbar.setLayout(new ResponsiveGridLayout(3, 10, 10));

        // --- Item 1: Overlay Visibility ---
        WebPanel visibilityPanel = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 5, 0));
        visibilityPanel.setOpaque(false);
        WebLabel lblOverlay = new WebLabel(Lang.mDisplayOverlay);
        lblOverlay.setFont(prog.Application.defaultFont);
        visibilityPanel.add(lblOverlay);
        WebSwitch swOverlay = new WebSwitch();
        swOverlay.setSelected(overlayVisible);
        UIBuilder.getStyle().decorateSwitch(swOverlay);
        swOverlay.addActionListener(e -> {
            overlayVisible = swOverlay.isSelected();
            setOverlayVisible(overlayVisible);
            if (groupConfig != null) {
                groupConfig.visible = overlayVisible;
                parent.saveDynamicConfig();
            }
        });
        visibilityPanel.add(swOverlay);
        toolbar.add(visibilityPanel);

        // --- Item 2: Overlay Style ---
        WebPanel stylePanel = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 5, 0));
        stylePanel.setOpaque(false);
        WebLabel lblStyle = new WebLabel(Lang.mDisplayStyle);
        lblStyle.setFont(prog.Application.defaultFont);
        stylePanel.add(lblStyle);
        String[] styles = { Lang.mStyleZebra, Lang.mStyleSolid };
        com.alee.laf.combobox.WebComboBox cbStyle = new com.alee.laf.combobox.WebComboBox(styles);
        cbStyle.setFont(prog.Application.defaultFont);
        cbStyle.setWebColoredBackground(false);
        cbStyle.setShadeWidth(1);
        cbStyle.setDrawFocus(false);
        stylePanel.add(cbStyle);
        toolbar.add(stylePanel);

        // --- Item 4: Hotkey Configuration ---
        WebPanel hotkeyPanel = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 5, 0));
        hotkeyPanel.setOpaque(false);
        if (groupConfig != null && groupConfig.hotkey != 0) {
            WebLabel lblHotkey = new WebLabel(Lang.mHotkeyToggle);
            lblHotkey.setFont(prog.Application.defaultFont);
            hotkeyPanel.add(lblHotkey);

            String keyText = com.github.kwhat.jnativehook.keyboard.NativeKeyEvent.getKeyText(groupConfig.hotkey);
            WebButton btnHotkey = new WebButton(keyText);
            btnHotkey.setFont(prog.Application.defaultFont);
            btnHotkey.setFocusable(false);
            btnHotkey.addActionListener(e -> {
                try {
                    Application.silenceNativeHookLogger();
                    if (!com.github.kwhat.jnativehook.GlobalScreen.isNativeHookRegistered()) {
                        com.github.kwhat.jnativehook.GlobalScreen.registerNativeHook();
                    }
                } catch (com.github.kwhat.jnativehook.NativeHookException ex) {
                    ex.printStackTrace();
                }

                btnHotkey.setText(Lang.mWaitHotkey);
                com.github.kwhat.jnativehook.GlobalScreen.addNativeKeyListener(
                        new com.github.kwhat.jnativehook.keyboard.NativeKeyListener() {
                            @Override
                            public void nativeKeyPressed(com.github.kwhat.jnativehook.keyboard.NativeKeyEvent e2) {
                                int code = e2.getKeyCode();
                                if (code == NativeKeyEvent.VC_NUM_LOCK || code == NativeKeyEvent.VC_CAPS_LOCK
                                        || code == NativeKeyEvent.VC_SCROLL_LOCK) {
                                    return;
                                }
                                Application.debugPrint("[DynamicDataPage] configure key: key pressed: " + code);
                                javax.swing.SwingUtilities.invokeLater(() -> {
                                    if (code == com.github.kwhat.jnativehook.keyboard.NativeKeyEvent.VC_ESCAPE) {
                                        btnHotkey.setText(com.github.kwhat.jnativehook.keyboard.NativeKeyEvent
                                                .getKeyText(groupConfig.hotkey));
                                    } else {
                                        groupConfig.hotkey = code;
                                        btnHotkey.setText(com.github.kwhat.jnativehook.keyboard.NativeKeyEvent
                                                .getKeyText(code));
                                        save();
                                    }
                                });
                                com.github.kwhat.jnativehook.GlobalScreen.removeNativeKeyListener(this);
                            }
                        });
            });
            hotkeyPanel.add(btnHotkey);
            toolbar.add(hotkeyPanel);
        }

        return toolbar;
    }

    private void save() {
        // Trigger global save of ui_layout.cfg
        parent.saveDynamicConfig();

        // Also notify our overlay in Controller to rebuild its bindings
        if ("Engine Info".equals(groupConfig.title)) {
            // Special handling for legacy EngineInfo which is not a DynamicOverlay
            if (parent.tc.overlayManager != null) {
                ui.overlay.EngineInfo eI = parent.tc.overlayManager.get("engineInfoSwitch");
                if (eI != null) {
                    eI.reinitConfig();
                }
            }
        } else {
            // Only loop if list is not null (currently commented out/removed list)
            // if (parent.tc.dynamicOverlays != null) { ... }
        }
    }

    public void update() {
        // Now handled by Controller/UIThread loop
    }

    /**
     * Calculates the height needed to display all rows without scaling.
     * Includes padding for titles and bottom panel.
     */
    public int getRequiredHeight() {
        // getPreferredSize on the page itself will account for Center and South
        // components
        Dimension d = getPreferredSize();
        // Add some buffer for WebFrame's title bar and borders (approx 40-60px)
        return d.height + 60;
    }

    public void setOverlayVisible(boolean visible) {
        if ("Engine Info".equals(groupConfig.title)) {
            // Special handling for legacy EngineInfo
            if (parent.tc.overlayManager != null) {
                ui.overlay.EngineInfo eI = parent.tc.overlayManager.get("engineInfoSwitch");
                if (eI != null) {
                    eI.setVisible(visible);
                }
            }
        } else {
            // Temporarily disabled - DynamicOverlay removed
            // if (parent.tc.dynamicOverlays != null) { ... }
        }
        overlayVisible = visible;
    }

    public void toggleOverlay() {
        setOverlayVisible(!overlayVisible);
    }

    public void dispose() {
        // No longer owns the overlay, nothing to dispose here.
        // Overlays are managed by Controller.
    }

    @Override
    protected java.awt.Component createCenterComponent(WebPanel content) {
        return scaler;
    }

    class ZoomPanel extends WebPanel {
        @Override
        public void paint(Graphics g) {
            Graphics2D g2 = (Graphics2D) g;
            Dimension pref = this.getLayout().preferredLayoutSize(this);
            int h = getHeight();
            double scale = 1.0;
            if (pref.height > h && pref.height > 0) {
                scale = (double) h / (double) pref.height;
            }
            if (scale < 0.1)
                scale = 0.1;
            if (scale < 1.0)
                g2.scale(scale, scale);
            super.paint(g2);
        }
    }
}
