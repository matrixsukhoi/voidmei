package ui.layout;

import java.awt.BorderLayout;
import java.awt.Dimension;
import java.awt.Graphics;
import java.awt.Graphics2D;
// import java.awt.Dimension; // Unused
// import java.awt.Graphics; // Unused
// import java.awt.Graphics2D; // Unused

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent;

import prog.app;
import ui.mainform;
import prog.lang;
import com.alee.laf.button.WebButton;
import com.alee.extended.button.WebSwitch;
import java.awt.GridBagLayout;
import java.awt.GridBagConstraints;
import java.awt.Insets;

public class DynamicDataPage extends BasePage {

    private ZoomPanel scaler;
    private ui.util.ConfigLoader.GroupConfig groupConfig;
    private boolean overlayVisible = false;

    public DynamicDataPage(mainform parent, ui.util.ConfigLoader.GroupConfig groupConfig) {
        super(parent);
        this.groupConfig = groupConfig;

        if (groupConfig != null) {
            this.overlayVisible = groupConfig.visible;
            // Since BasePage constructor calls createTopToolbar() BEFORE we set
            // overlayVisible,
            // we must refresh the toolbar to sync the switch state.
            refreshToolbar();
        }

        rebuild();

        // Restore overlay state
        // Always call this to ensure state is synchronized
        setOverlayVisible(overlayVisible);
    }

    public DynamicDataPage(mainform parent) {
        super(parent);
    }

    public void setGroupConfig(ui.util.ConfigLoader.GroupConfig groupConfig) {
        this.groupConfig = groupConfig;
        if (groupConfig != null) {
            this.overlayVisible = groupConfig.visible;
            refreshToolbar();
        }
        rebuild();
    }

    public ui.util.ConfigLoader.GroupConfig getGroupConfig() {
        return groupConfig;
    }

    @Override
    protected void initContent(WebPanel content) {
        scaler = new ZoomPanel();
        scaler.setOpaque(false);
        scaler.setLayout(new GridBagLayout());
    }

    private class ColumnDef {
        String id;
        double weight;
        int minW;
        int maxW;
        int measuredMax = 0;
        int allocatedW = 0;

        ColumnDef(String id, double weight, int minW, int maxW) {
            this.id = id;
            this.weight = weight;
            this.minW = minW;
            this.maxW = maxW;
        }
    }

    private void rebuild() {
        scaler.removeAll();
        scaler.setFocusable(true);

        if (groupConfig == null)
            return;

        if (isDetailedMode) {
            rebuildDetailed();
        } else {
            rebuildSimple();
        }

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

        for (ui.util.ConfigLoader.RowConfig row : groupConfig.rows) {
            boolean isHeader = row.formula != null && row.formula.equalsIgnoreCase("HEADER");

            if (isHeader) {
                // Finalize previous group if exists
                if (currentGroup != null && currentHeader != null) {
                    addGroupToScaler(currentHeader, currentGroup);
                }

                // Create new header
                currentHeader = new WebLabel(row.label);
                currentHeader.setFont(prog.app.defaultFontBig);
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
                    currentHeader.setFont(prog.app.defaultFontBig);
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
        WebLabel lblOverlay = new WebLabel(lang.mDisplayOverlay);
        lblOverlay.setFont(prog.app.defaultFont);
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
        WebLabel lblStyle = new WebLabel(lang.mDisplayStyle);
        lblStyle.setFont(prog.app.defaultFont);
        stylePanel.add(lblStyle);
        String[] styles = { lang.mStyleZebra, lang.mStyleSolid };
        com.alee.laf.combobox.WebComboBox cbStyle = new com.alee.laf.combobox.WebComboBox(styles);
        cbStyle.setFont(prog.app.defaultFont);
        cbStyle.setWebColoredBackground(false);
        cbStyle.setShadeWidth(1);
        cbStyle.setDrawFocus(false);
        stylePanel.add(cbStyle);
        toolbar.add(stylePanel);

        // --- Item 3: Detailed Mode ---
        WebPanel modePanel = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 5, 0));
        modePanel.setOpaque(false);
        WebLabel lblMode = new WebLabel(lang.mDetailedMode);
        lblMode.setFont(prog.app.defaultFont);
        modePanel.add(lblMode);
        WebSwitch swMode = new WebSwitch();
        swMode.setSelected(isDetailedMode);
        UIBuilder.getStyle().decorateSwitch(swMode);
        swMode.addActionListener(e -> {
            isDetailedMode = swMode.isSelected();
            rebuild();
        });
        modePanel.add(swMode);
        toolbar.add(modePanel);

        // --- Item 4: Hotkey Configuration ---
        WebPanel hotkeyPanel = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 5, 0));
        hotkeyPanel.setOpaque(false);
        if (groupConfig != null && groupConfig.hotkey != 0) {
            WebLabel lblHotkey = new WebLabel(lang.mHotkeyToggle);
            lblHotkey.setFont(prog.app.defaultFont);
            hotkeyPanel.add(lblHotkey);

            String keyText = com.github.kwhat.jnativehook.keyboard.NativeKeyEvent.getKeyText(groupConfig.hotkey);
            WebButton btnHotkey = new WebButton(keyText);
            btnHotkey.setFont(prog.app.defaultFont);
            btnHotkey.setFocusable(false);
            btnHotkey.addActionListener(e -> {
                try {
                    app.silenceNativeHookLogger();
                    if (!com.github.kwhat.jnativehook.GlobalScreen.isNativeHookRegistered()) {
                        com.github.kwhat.jnativehook.GlobalScreen.registerNativeHook();
                    }
                } catch (com.github.kwhat.jnativehook.NativeHookException ex) {
                    ex.printStackTrace();
                }

                btnHotkey.setText(lang.mWaitHotkey);
                com.github.kwhat.jnativehook.GlobalScreen.addNativeKeyListener(
                        new com.github.kwhat.jnativehook.keyboard.NativeKeyListener() {
                            @Override
                            public void nativeKeyPressed(com.github.kwhat.jnativehook.keyboard.NativeKeyEvent e2) {
                                int code = e2.getKeyCode();
                                if (code == NativeKeyEvent.VC_NUM_LOCK || code == NativeKeyEvent.VC_CAPS_LOCK
                                        || code == NativeKeyEvent.VC_SCROLL_LOCK) {
                                    return;
                                }
                                app.debugPrint("[DynamicDataPage] configure key: key pressed: " + code);
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

    private void rebuildDetailed() {
        // Reset layout to GridBagLayout for detailed mode
        scaler.setLayout(new GridBagLayout());
        scaler.setFocusable(true);
        scaler.addMouseListener(new java.awt.event.MouseAdapter() {
            @Override
            public void mousePressed(java.awt.event.MouseEvent e) {
                scaler.requestFocus();
            }
        });

        if (groupConfig == null)
            return;

        // --- Step 1: Define Columns ---
        java.util.List<ColumnDef> cols = new java.util.ArrayList<>();
        cols.add(new ColumnDef("label", 0.25, 80, 200));
        cols.add(new ColumnDef("formula", 0.30, 120, 350));
        cols.add(new ColumnDef("format", 0.30, 120, 200));
        cols.add(new ColumnDef("actions", 0.15, 130, 160));

        // --- Step 2: Pass 1 - Measurement ---
        java.util.List<RowComponents> rowComps = new java.util.ArrayList<>();
        java.awt.Font font = prog.app.defaultFont;
        for (ui.util.ConfigLoader.RowConfig row : groupConfig.rows) {
            RowComponents rc = new RowComponents(row);
            rowComps.add(rc);

            // Measure actual text width instead of relying on unrendered component
            // preferred size
            cols.get(0).measuredMax = Math.max(cols.get(0).measuredMax, measureString(row.label, font));
            if (!rc.isHeader) {
                cols.get(1).measuredMax = Math.max(cols.get(1).measuredMax, measureString(row.formula, font));
                cols.get(2).measuredMax = Math.max(cols.get(2).measuredMax, measureString(row.format, font));
            }
            cols.get(3).measuredMax = 130; // Actions are fixed width
        }

        // --- Step 3: Pass 2 - Width Allocation ---
        // Width estimation: window width (800) - shade (31) - tab area (approx 150)
        // - BasePage padding (40) - GridBagLayout insets (40) - safety (9) = 530.
        int targetTotal = 530;
        int sumRequired = 0;
        for (ColumnDef col : cols) {
            col.allocatedW = Math.max(col.minW, Math.min(col.maxW, col.measuredMax + 20));
            sumRequired += col.allocatedW;
        }

        // Proportional scale down if we exceed targetTotal to stay within corrected
        // frame
        if (sumRequired > targetTotal) {
            double scale = (double) targetTotal / sumRequired;
            for (ColumnDef col : cols) {
                col.allocatedW = Math.max(col.minW, (int) (col.allocatedW * scale));
            }
        }

        // --- Step 4: Final Construction ---
        int rowIdx = 0;
        for (RowComponents rc : rowComps) {
            addRowToGrid(rc, rowIdx++, cols);
        }

        // Push everything to top
        GridBagConstraints gbc = new GridBagConstraints();
        gbc.gridx = 0;
        gbc.gridy = rowIdx;
        gbc.gridwidth = cols.size();
        gbc.weighty = 1.0;
        gbc.fill = GridBagConstraints.VERTICAL;
        WebPanel filler = new WebPanel();
        filler.setOpaque(false);
        scaler.add(filler, gbc);
    }

    private class RowComponents {
        EditableField fLabel;
        EditableField fFormula;
        EditableField fFormat;
        WebLabel headerIndicator;
        WebPanel actions;
        boolean isHeader;

        RowComponents(ui.util.ConfigLoader.RowConfig row) {
            this.isHeader = row.formula != null && row.formula.equalsIgnoreCase("HEADER");

            fLabel = new EditableField(row.label, (val) -> {
                row.label = val;
                save();
            });
            fLabel.setTextColor(isHeader ? new java.awt.Color(180, 30, 0) : new java.awt.Color(20, 20, 20));
            if (isHeader)
                fLabel.label.setFont(prog.app.defaultFontBig);

            if (!isHeader) {
                fFormula = new EditableField(row.formula, (val) -> {
                    if (prog.util.FormulaEvaluator.check(val)) {
                        row.formula = val;
                        save();
                        return true;
                    }
                    return false;
                });
                fFormula.setTextColor(new java.awt.Color(0, 100, 215));

                fFormat = new EditableField(row.format, (val) -> {
                    row.format = val;
                    save();
                });
                fFormat.setTextColor(new java.awt.Color(0, 100, 215));
            } else {
                headerIndicator = new WebLabel("HEADER");
                headerIndicator.setFont(prog.app.defaultFont);
                headerIndicator.setForeground(new java.awt.Color(0, 0, 0, 60));
            }

            actions = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.RIGHT, 5, 0));
            actions.setOpaque(false);

            WebButton btnAdd = new WebButton("+");
            btnAdd.addActionListener(e -> {
                int idx = groupConfig.rows.indexOf(row);
                if (idx != -1) {
                    groupConfig.rows.add(idx + 1, new ui.util.ConfigLoader.RowConfig("New Row", "0", "%s"));
                    rebuild();
                    save();
                }
            });

            WebSwitch btnVis = new WebSwitch();
            btnVis.setSelected(row.visible);
            UIBuilder.getStyle().decorateSwitch(btnVis);
            btnVis.addActionListener(e -> {
                row.visible = btnVis.isSelected();
                save();
            });

            WebButton btnDel = new WebButton("X");
            btnDel.addActionListener(e -> {
                if (groupConfig.rows.remove(row)) {
                    rebuild();
                    save();
                }
            });

            actions.add(btnVis);
            actions.add(btnAdd);
            actions.add(btnDel);
        }
    }

    private void addRowToGrid(RowComponents rc, int rowIdx, java.util.List<ColumnDef> cols) {
        GridBagConstraints gbc = new GridBagConstraints();
        gbc.insets = new Insets(2, 5, 2, 5);
        gbc.fill = GridBagConstraints.HORIZONTAL;
        gbc.gridy = rowIdx;

        // Label
        gbc.gridx = 0;
        gbc.weightx = cols.get(0).weight;
        Dimension size0 = new Dimension(cols.get(0).allocatedW, 24);
        rc.fLabel.setPreferredSize(size0);
        rc.fLabel.setMinimumSize(size0); // CRITICAL: Stop GridBagLayout from shrinking
        scaler.add(rc.fLabel, gbc);

        if (!rc.isHeader) {
            // Formula
            gbc.gridx = 1;
            gbc.weightx = cols.get(1).weight;
            Dimension size1 = new Dimension(cols.get(1).allocatedW, 24);
            rc.fFormula.setPreferredSize(size1);
            rc.fFormula.setMinimumSize(size1);
            scaler.add(rc.fFormula, gbc);

            // Format
            gbc.gridx = 2;
            gbc.weightx = cols.get(2).weight;
            Dimension size2 = new Dimension(cols.get(2).allocatedW, 24);
            rc.fFormat.setPreferredSize(size2);
            rc.fFormat.setMinimumSize(size2);
            scaler.add(rc.fFormat, gbc);
        } else {
            gbc.gridx = 1;
            gbc.weightx = cols.get(1).weight;
            scaler.add(rc.headerIndicator, gbc);
            gbc.gridx = 2;
            gbc.weightx = cols.get(2).weight;
            scaler.add(new WebLabel(""), gbc);
        }

        // Actions
        gbc.gridx = 3;
        gbc.weightx = cols.get(3).weight;
        Dimension size3 = new Dimension(cols.get(3).allocatedW, 24);
        rc.actions.setPreferredSize(size3);
        rc.actions.setMinimumSize(size3);
        scaler.add(rc.actions, gbc);
    }

    private int measureString(String s, java.awt.Font font) {
        if (s == null || s.isEmpty())
            return 20;
        java.awt.image.BufferedImage img = new java.awt.image.BufferedImage(1, 1,
                java.awt.image.BufferedImage.TYPE_INT_ARGB);
        java.awt.FontMetrics fm = img.getGraphics().getFontMetrics(font);
        return fm.stringWidth(s);
    }

    /**
     * A field that switches from Label to TextField on click.
     */
    private class EditableField extends WebPanel {
        WebLabel label;
        com.alee.laf.text.WebTextField editor;
        String value;
        java.util.function.Function<String, Object> onSave; // Function returns boolean or null

        EditableField(String val, java.util.function.Consumer<String> onSaveRaw) {
            this(val, (v) -> {
                onSaveRaw.accept(v);
                return true;
            });
        }

        EditableField(String val, java.util.function.Function<String, Object> onSave) {
            this.value = val;
            this.onSave = onSave;
            setOpaque(false);
            setLayout(new BorderLayout());

            label = new WebLabel(val == null || val.isEmpty() ? "---" : val);
            label.setFont(prog.app.defaultFont);
            add(label, BorderLayout.CENTER);

            editor = new com.alee.laf.text.WebTextField(val);
            editor.setFont(prog.app.defaultFont);

            label.addMouseListener(new java.awt.event.MouseAdapter() {
                public void mousePressed(java.awt.event.MouseEvent e) {
                    label.requestFocusInWindow();
                }

                public void mouseClicked(java.awt.event.MouseEvent e) {
                    if (e.getClickCount() == 2) {
                        switchToEditor();
                    }
                }
            });

            editor.addActionListener(e -> commit());
            editor.addKeyListener(new java.awt.event.KeyAdapter() {
                @Override
                public void keyPressed(java.awt.event.KeyEvent e) {
                    if (e.getKeyCode() == java.awt.event.KeyEvent.VK_ESCAPE) {
                        switchToLabel();
                    }
                }
            });
            editor.addFocusListener(new java.awt.event.FocusAdapter() {
                public void focusLost(java.awt.event.FocusEvent e) {
                    commit();
                }
            });
        }

        public void setTextColor(java.awt.Color color) {
            label.setForeground(color);
        }

        private void switchToEditor() {
            removeAll();
            editor.setText(value);
            add(editor, BorderLayout.CENTER);
            revalidate();
            repaint();
            editor.requestFocus();
            editor.selectAll();
        }

        private void switchToLabel() {
            removeAll();
            label.setText(value == null || value.isEmpty() ? "---" : value);
            add(label, BorderLayout.CENTER);
            revalidate();
            repaint();
        }

        private void commit() {
            String newVal = editor.getText();
            Object success = onSave.apply(newVal);
            if (Boolean.TRUE.equals(success)) {
                this.value = newVal;
            }
            switchToLabel();
        }

    }

    private void save() {
        // Trigger global save of ui_layout.cfg
        parent.saveDynamicConfig();

        // Also notify our overlay in controller to rebuild its bindings
        if (parent.tc.dynamicOverlays != null) {
            for (ui.overlay.DynamicOverlay overlay : parent.tc.dynamicOverlays) {
                if (overlay.getGroupConfig().title.equals(groupConfig.title)) {
                    overlay.rebuildBindings();
                    break;
                }
            }
        }
    }

    public void update() {
        // Now handled by controller/uiThread loop
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
        if (parent.tc.dynamicOverlays != null) {
            for (ui.overlay.DynamicOverlay overlay : parent.tc.dynamicOverlays) {
                if (overlay.getGroupConfig().title.equals(groupConfig.title)) {
                    overlay.setVisible(visible);
                    break;
                }
            }
        }
        overlayVisible = visible;
    }

    public void toggleOverlay() {
        setOverlayVisible(!overlayVisible);
    }

    public void dispose() {
        // No longer owns the overlay, nothing to dispose here.
        // Overlays are managed by controller.
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
