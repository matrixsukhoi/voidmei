package ui.layout;

import java.awt.Dimension;
import java.awt.Rectangle;

import javax.swing.Scrollable;

import com.alee.laf.panel.WebPanel;
import com.alee.laf.scroll.WebScrollPane;

import ui.MainForm;
import prog.event.UIStateBus;
import prog.event.UIStateEvents;
import prog.util.Logger;
import ui.replica.ReplicaBuilder;
import ui.layout.renderer.RowRenderer;
import ui.layout.renderer.RowRendererRegistry;

public class DynamicDataPage extends BasePage {

    private ScrollablePanel scaler;
    private prog.config.ConfigLoader.GroupConfig groupConfig;
    private boolean isUpdatingControls = false; // Prevent feedback loop during rebuild

    private java.util.function.Consumer<Object> configHandler;

    public DynamicDataPage(MainForm parent, prog.config.ConfigLoader.GroupConfig groupConfig) {
        super(parent);
        this.groupConfig = groupConfig;

        // Force Pink Style (Modern) for this page as per design requirement
        UIBuilder.setStyle(ReplicaBuilder.getStyle());

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
        rebuild();
    }

    public prog.config.ConfigLoader.GroupConfig getGroupConfig() {
        return groupConfig;
    }

    @Override
    protected void initContent(WebPanel content) {
        scaler = new ScrollablePanel();
        scaler.setOpaque(false);
        scaler.setLayout(new com.alee.extended.layout.VerticalFlowLayout());
        scaler.setFocusable(true);

        // 点击空白处时请求焦点，触发文本框的 focusLost
        scaler.addMouseListener(new java.awt.event.MouseAdapter() {
            @Override
            public void mousePressed(java.awt.event.MouseEvent e) {
                scaler.requestFocusInWindow();
            }
        });
    }

    private void rebuild() {
        Logger.info("ComboDebug", "rebuild() called");
        // Dispose all active popovers before removing components to prevent
        // IllegalComponentStateException when window moves
        ReplicaBuilder.disposeAllPopovers();

        scaler.removeAll();

        if (groupConfig == null)
            return;

        // 从 ConfigurationService 获取最新的 GroupConfig
        // 导入配置后 layoutConfigs 会重新加载为全新的对象树，但 this.groupConfig 仍指向旧引用
        // 这会导致 UI 显示旧的配置值（如热键），因此需要根据 title 获取最新的 GroupConfig
        prog.config.ConfigLoader.GroupConfig freshConfig = parent.tc.configService.findGroupByTitle(groupConfig.title);
        if (freshConfig != null) {
            this.groupConfig = freshConfig;
        }

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

        // Dispose popovers before removing (also called from rebuild, but safe to call twice)
        ReplicaBuilder.disposeAllPopovers();

        scaler.removeAll();
        scaler.setLayout(new com.alee.extended.layout.VerticalFlowLayout());

        int pCols = groupConfig.panelColumns > 0 ? groupConfig.panelColumns : 2;

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

        // Start recursive build
        buildContainer(groupConfig.rows, scaler, pCols, ctx);

        scaler.revalidate();
        scaler.repaint();
    }

    private void buildContainer(java.util.List<prog.config.ConfigLoader.RowConfig> rows, WebPanel parentContainer,
            int defaultCols, RowRenderer.RenderContext ctx) {
        WebPanel currentCard = null;
        WebPanel currentGrid = null;

        for (prog.config.ConfigLoader.RowConfig row : rows) {
            String rowType = row.type != null ? row.type : "DATA";

            if (rowType.equals("HEADER")) {
                // Header row triggers new Card (Group)
                currentCard = ReplicaBuilder.getStyle().createContainer(row.label);

                // Nested container usually has a vertical layout to stack its own grid + nested
                // groups
                WebPanel nestedContent = new WebPanel();
                nestedContent.setOpaque(false);
                nestedContent.setLayout(new com.alee.extended.layout.VerticalFlowLayout());

                // Add a Grid for the IMMEDIATE children items of this group
                // But wait, the children hierarchy is now separate.
                // In recursive model, 'row' has 'children'.
                // So we recursively build the children into 'nestedContent'.

                int cols = row.groupColumns > 0 ? row.groupColumns : defaultCols;

                // Recursive call
                // Note: The Grid creation logic needs to happen INSIDE buildContainer
                // if we want to mix items and groups.
                // But our loop here iterates linearly if flattened, or hierarchical if not.
                // With the new structure, 'row' has a list of 'children'.

                buildContainer(row.children, nestedContent, cols, ctx);

                currentCard.add(nestedContent);
                parentContainer.add(currentCard);

                // Reset currentGrid so next items at THIS level get a new grid if needed
                // (Though with strict hierarchy, items should be inside the group/HEADER logic)
                currentGrid = null;
            } else {
                // Regular Item
                // Ensure we have a grid to add to in the current container
                if (currentGrid == null) {
                    // If we are at root or inside a group, we might need a grid for loose items
                    currentGrid = new WebPanel();
                    currentGrid.setOpaque(false);
                    currentGrid.setLayout(new ResponsiveGridLayout(defaultCols, 10, 5));
                    parentContainer.add(currentGrid);
                }

                // Use strategy pattern to render the row
                RowRenderer renderer = RowRendererRegistry.get(rowType);
                WebPanel itemPanel = renderer.render(row, groupConfig, ctx);

                if (itemPanel != null) {
                    currentGrid.add(itemPanel);
                }
            }
        }
    }

    private void save() {
        Logger.info("ComboDebug", "save() called");
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

        // Use scroll pane instead of scaling for content that exceeds available height
        WebScrollPane scrollPane = new WebScrollPane(scaler);
        scrollPane.setOpaque(false);
        scrollPane.getViewport().setOpaque(false);
        scrollPane.setBorder(null);
        scrollPane.setViewportBorder(null);
        scrollPane.setDrawBorder(false);
        // Disable horizontal scrolling - content should fit width
        scrollPane.setHorizontalScrollBarPolicy(WebScrollPane.HORIZONTAL_SCROLLBAR_NEVER);
        // Optimize scroll speed for smooth mouse wheel navigation
        scrollPane.getVerticalScrollBar().setUnitIncrement(16);

        main.add(scrollPane, java.awt.BorderLayout.CENTER);
        return main;
    }

    /**
     * A WebPanel that implements Scrollable to track viewport width.
     * This ensures content width matches the scroll pane's viewport,
     * preventing horizontal overflow.
     */
    private static class ScrollablePanel extends WebPanel implements Scrollable {
        @Override
        public Dimension getPreferredScrollableViewportSize() {
            return getPreferredSize();
        }

        @Override
        public int getScrollableUnitIncrement(Rectangle visibleRect, int orientation, int direction) {
            return 16;
        }

        @Override
        public int getScrollableBlockIncrement(Rectangle visibleRect, int orientation, int direction) {
            return visibleRect.height;
        }

        @Override
        public boolean getScrollableTracksViewportWidth() {
            // Force width to match viewport - prevents horizontal scrolling
            return true;
        }

        @Override
        public boolean getScrollableTracksViewportHeight() {
            // Allow vertical scrolling - don't track viewport height
            return false;
        }
    }
}
