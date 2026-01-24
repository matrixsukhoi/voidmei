package ui.layout;

import java.awt.Dimension;
import java.awt.Graphics;
import java.awt.Graphics2D;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent;

import prog.Application;
import ui.MainForm;
import prog.i18n.Lang;
import com.alee.laf.button.WebButton;
import com.alee.extended.button.WebSwitch;
import prog.event.UIStateBus;
import prog.event.UIStateEvents;
import ui.replica.ReplicaBuilder;
import ui.layout.renderer.RowRenderer;
import ui.layout.renderer.RowRendererRegistry;

public class DynamicDataPage extends BasePage {

    private ZoomPanel scaler;
    private prog.config.ConfigLoader.GroupConfig groupConfig;
    private boolean overlayVisible = false;
    private boolean isUpdatingControls = false; // Prevent feedback loop during rebuild

    private java.util.function.Consumer<Object> configHandler;

    public DynamicDataPage(MainForm parent, prog.config.ConfigLoader.GroupConfig groupConfig) {
        super(parent);
        this.groupConfig = groupConfig;

        // Force Pink Style (Modern) for this page as per design requirement
        UIBuilder.setStyle(ReplicaBuilder.getStyle());

        if (groupConfig != null) {
            this.overlayVisible = groupConfig.visible;
            // Since BasePage constructor calls createTopToolbar() BEFORE we set
            // overlayVisible,
            // we must refresh the toolbar to sync the switch State.
            refreshToolbar();
        }

        // Subscribe to global config changes (specifically for RESET_ALL)
        configHandler = key -> {
            if (UIStateEvents.ACTION_RESET_COMPLETED.equals(key)) {
                // Ensure we are on EDT, though event bus usually dispatches there or logic
                // handles it
                javax.swing.SwingUtilities.invokeLater(() -> rebuild());
            }
        };
        UIStateBus.getInstance().subscribe(UIStateEvents.CONFIG_CHANGED, configHandler);

        rebuild();

        // Restore overlay State
        // Always call this to ensure State is synchronized
        setOverlayVisible(overlayVisible);
    }

    public DynamicDataPage(MainForm parent) {
        super(parent);
        // Also subscribe here in case this constructor is used
        configHandler = key -> {
            if (UIStateEvents.ACTION_RESET_COMPLETED.equals(key)) {
                javax.swing.SwingUtilities.invokeLater(() -> rebuild());
            }
        };
        UIStateBus.getInstance().subscribe(UIStateEvents.CONFIG_CHANGED, configHandler);
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

        isUpdatingControls = true;
        rebuildSimple();
        isUpdatingControls = false;

        scaler.revalidate();
        scaler.repaint();
        if (parent != null) {
            parent.updateDynamicSize();
        }
    }

    private void rebuildSimple() {
        if (groupConfig == null)
            return;

        scaler.removeAll();
        scaler.setLayout(new com.alee.extended.layout.VerticalFlowLayout());

        int pCols = groupConfig.panelColumns > 0 ? groupConfig.panelColumns : 2;

        WebPanel currentCard = null;
        WebPanel currentGrid = null;

        // Create render context with callbacks
        final RowRenderer.RenderContext ctx = new RowRenderer.RenderContext() {
            @Override
            public void onSave() {
                save();
            }

            @Override
            public void onRebuild() {
                rebuild();
            }

            @Override
            public boolean isUpdating() {
                return isUpdatingControls;
            }

            @Override
            public void syncToConfigService(String key, boolean value) {
                // Bridge to ConfigurationService for overlay visibility control
                parent.tc.configService.setConfig(key, Boolean.toString(value));

                // Special event handling for FM Print
                if ("enableFMPrint".equals(key)) {
                    UIStateBus.getInstance().publish(UIStateEvents.FM_PRINT_SWITCH_CHANGED,
                            "DynamicDataPage(RenderContext)", value);
                }
            }

            @Override
            public boolean getFromConfigService(String key, boolean defaultVal) {
                // Read from ConfigurationService for initial state
                String val = parent.tc.configService.getConfig(key);
                if (val == null || val.isEmpty())
                    return defaultVal;
                return Boolean.parseBoolean(val);
            }

            @Override
            public void syncStringToConfigService(String key, String value) {
                parent.tc.configService.setConfig(key, value);
            }

            @Override
            public String getStringFromConfigService(String key, String defaultVal) {
                String val = parent.tc.configService.getConfig(key);
                if (val == null || val.isEmpty())
                    return defaultVal;
                return val;
            }
        };

        for (prog.config.ConfigLoader.RowConfig row : groupConfig.rows) {
            String rowType = row.type != null ? row.type : "DATA";

            if (rowType.equals("HEADER")) {
                // Header row triggers new Card
                currentCard = ReplicaBuilder.getStyle().createContainer(row.label);
                currentGrid = new WebPanel();
                currentGrid.setOpaque(false);
                int cols = row.groupColumns > 0 ? row.groupColumns : pCols;
                currentGrid.setLayout(new ResponsiveGridLayout(cols, 10, 5));
                currentCard.add(currentGrid);
                scaler.add(currentCard);
            } else {
                // Ensure we have a grid to add to
                if (currentGrid == null) {
                    currentCard = ReplicaBuilder.getStyle().createContainer("General");
                    currentGrid = new WebPanel();
                    currentGrid.setOpaque(false);
                    currentGrid.setLayout(new ResponsiveGridLayout(pCols, 10, 5));
                    currentCard.add(currentGrid);
                    scaler.add(currentCard);
                }

                // Use strategy pattern to render the row
                RowRenderer renderer = RowRendererRegistry.get(rowType);
                WebPanel itemPanel = renderer.render(row, groupConfig, ctx);

                if (itemPanel != null) {
                    currentGrid.add(itemPanel);
                }
            }
        }

        scaler.revalidate();
        scaler.repaint();
    }

    @Override
    protected WebPanel createTopToolbar() {
        // Get the styled toolbar container from BasePage
        WebPanel toolbar = super.createTopToolbar();
        // Use our custom ResponsiveGridLayout for elegant multi-column layout
        toolbar.setLayout(new ResponsiveGridLayout(3, 10, 10));

        // --- Item 1: Overlay Visibility ---
        WebPanel visibilityPanel = ReplicaBuilder.createSwitchItem(Lang.mDisplayOverlay, overlayVisible, false);
        WebSwitch swOverlay = ReplicaBuilder.getSwitch(visibilityPanel);
        if (swOverlay != null) {
            swOverlay.addActionListener(e -> {
                overlayVisible = swOverlay.isSelected();
                setOverlayVisible(overlayVisible);
                if (groupConfig != null) {
                    groupConfig.visible = overlayVisible;
                    parent.saveDynamicConfig();
                }
            });
        }
        toolbar.add(visibilityPanel);

        // --- Item 2: Overlay Style ---
        String[] styles = { Lang.mStyleZebra, Lang.mStyleSolid };
        WebPanel stylePanel = ReplicaBuilder.createDropdownItem(Lang.mDisplayStyle, styles);
        toolbar.add(stylePanel);

        // --- Item 3: Hotkey Configuration ---
        if (groupConfig != null && groupConfig.hotkey != 0) {
            WebPanel hotkeyPanel = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 5, 0));
            hotkeyPanel.setOpaque(false);
            WebLabel lblHotkey = new WebLabel(Lang.mHotkeyToggle);
            ReplicaBuilder.getStyle().decorateLabel(lblHotkey);
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

        // Trigger global refresh for ui_layout.cfg changes (visibility, fonts, columns)
        // Trigger global refresh for ui_layout.cfg changes (visibility, fonts, columns)
        prog.event.UIStateBus.getInstance().publish(prog.event.UIStateEvents.CONFIG_CHANGED,
                this.getClass().getSimpleName(), "ui_layout.cfg");

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
        if (groupConfig.switchKey != null && !groupConfig.switchKey.isEmpty()) {
            parent.tc.configService.setConfig(groupConfig.switchKey, Boolean.toString(visible));
        }
        overlayVisible = visible;
    }

    public void toggleOverlay() {
        setOverlayVisible(!overlayVisible);
    }

    public void dispose() {
        if (configHandler != null) {
            UIStateBus.getInstance().unsubscribe(UIStateEvents.CONFIG_CHANGED, configHandler);
        }
        // No longer owns the overlay, nothing to dispose here.
        // Overlays are managed by Controller.
    }

    protected java.awt.Component createCenterComponent(WebPanel content) {
        // Scaler contains all cards (config-driven)
        WebPanel main = new WebPanel(new java.awt.BorderLayout());
        ReplicaBuilder.getStyle().decorateMainPanel(main);
        main.add(scaler, java.awt.BorderLayout.CENTER);
        return main;
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
