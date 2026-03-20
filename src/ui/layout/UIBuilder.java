package ui.layout;

import prog.Application;

import java.awt.Container;
import java.awt.Dimension;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;

public class UIBuilder {

    private static UIStyle activeStyle = new ClassicStyle();

    private static final java.awt.Color COL_TRANSPARENT = new java.awt.Color(0, 0, 0, 0);
    private static final java.awt.Color COL_BORDER = new java.awt.Color(0, 0, 0, 100);
    private static final int GRID_GAP_H = 15;
    private static final int GRID_GAP_V = 5;

    private static void decoratePanelBase(WebPanel panel, int shadeWidth) {
        panel.setWebColoredBackground(false);
        panel.setBackground(COL_TRANSPARENT);
        panel.setOpaque(false);
        panel.setShadeWidth(shadeWidth);
        panel.setRound(com.alee.global.StyleConstants.largeRound);
        panel.setBorderColor(COL_BORDER);
    }

    public static void decorateStandardPanel(WebPanel panel) {
        decoratePanelBase(panel, 2);
        panel.setUndecorated(false);
        panel.setPaintBottom(false);
        panel.setPaintTop(false);
        panel.setPaintRight(false);
    }

    public static void decorateInsidePanel(WebPanel panel) {
        decoratePanelBase(panel, 2);
        panel.setShadeTransparency(0.1f);
    }

    public static void setStyle(UIStyle style) {
        activeStyle = style;
    }

    public static UIStyle getStyle() {
        return activeStyle;
    }

    public static WebPanel createGridContainer(int columns) {
        WebPanel container = new WebPanel();
        container.setLayout(new ResponsiveGridLayout(columns, GRID_GAP_H, GRID_GAP_V));
        // activeStyle.decorateContainer(container); // If we add this method to
        // interface later
        container.setOpaque(false);
        container.setBackground(COL_TRANSPARENT);
        container.setBorder(null); // Ensure no default border
        return container;
    }

    public static void addPlaceholder(Container parent) {
        WebPanel placeholder = new WebPanel();
        placeholder.setOpaque(false);
        // Ensure it has the same height as a standard switch panel to maintain grid
        // alignment
        placeholder.setPreferredSize(new Dimension(200, 30));
        parent.add(placeholder);
    }

    public static WebPanel createCard(String title) {
        return activeStyle.createContainer(title);
    }

    public static WebPanel createToggleContainer(String labelText, boolean initialValue) {
        WebPanel container = new WebPanel();
        container.setLayout(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT));

        WebLabel label = new WebLabel(labelText);
        activeStyle.decorateLabel(label);

        WebSwitch webSwitch = new WebSwitch();
        webSwitch.setSelected(initialValue);
        activeStyle.decorateSwitch(webSwitch);

        container.add(label);
        container.add(webSwitch);

        activeStyle.decorateControlPanel(container);
        return container;
    }

    // Quick helper to add a standardized switch to a container

    /**
     * Adds a tab with a custom right-aligned label component.
     * This ensures the tab title is right-aligned even for short English text.
     */
    public static void addRightAlignedTab(com.alee.laf.tabbedpane.WebTabbedPane tabbedPane, String title,
            java.awt.Component content, java.awt.Font font) {
        tabbedPane.addTab(title, content);

        com.alee.laf.label.WebLabel label = new com.alee.laf.label.WebLabel(title);
        label.setHorizontalAlignment(javax.swing.SwingConstants.RIGHT);
        label.setFont(font);
        // Force a specific dimension to ensure there is space for right alignment
        label.setPreferredSize(new Dimension(70, 30));
        // Add padding to satisfy "right align with padding" requirement
        // label.setBorder(javax.swing.BorderFactory.createEmptyBorder(0, 0, 0, 0));

        tabbedPane.setTabComponentAt(tabbedPane.getTabCount() - 1, label);
    }

    /**
     * Creates a standardized footer button with consistent size (120x30) and font
     * (defaultFontBig).
     */
    public static com.alee.laf.button.WebButton createFooterButton(String text) {
        com.alee.laf.button.WebButton btn = new com.alee.laf.button.WebButton(text);
        btn.setFont(prog.Application.defaultFontBig);
        btn.setPreferredSize(new Dimension(120, 80));

        // Basic styling consistent with MainForm.createButton
        btn.setShadeWidth(1);
        btn.setDrawShade(true);
        btn.setTopBgColor(new java.awt.Color(0, 0, 0, 0));
        btn.setBottomBgColor(new java.awt.Color(0, 0, 0, 0));
        btn.setBorderPainted(false);

        return btn;
    }

    /**
     * Adds a standardized item (Label + Switch) to a card/grid container.
     */
    public static WebSwitch addCardItem(Container parent, String labelText, boolean isSelected) {
        // Create a wrapper panel for the item to ensure proper alignment if needed,
        // or add directly if parent is a grid that handles pairs.
        // Assuming parent is a ResponsiveGridLayout with 2*Columns.

        WebLabel label = new WebLabel(labelText);
        activeStyle.decorateLabel(label);

        WebSwitch webSwitch = new WebSwitch();
        webSwitch.setSelected(isSelected);
        activeStyle.decorateSwitch(webSwitch);

        parent.add(label);
        parent.add(webSwitch);

        return webSwitch;
    }
}
