package ui.layout;

import java.awt.Container;
import java.awt.Dimension;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

public class UIBuilder {

    private static UIStyle activeStyle = new ClassicStyle();

    public static WebSwitch addLCGroup(Container parent, String text) {
        WebLabel lb = new WebLabel(text);
        activeStyle.decorateLabel(lb);

        WebSwitch ws = new WebSwitch();
        activeStyle.decorateSwitch(ws);

        parent.add(lb);
        parent.add(ws);
        return ws;
    }

    public static WebLabel addVoidWebLabel(Container parent, String text) {
        WebLabel lb = new WebLabel(text);
        lb.setHorizontalAlignment(javax.swing.SwingConstants.CENTER);
        lb.setForeground(new java.awt.Color(0, 0, 0, 230));
        lb.setShadeColor(java.awt.Color.WHITE);
        lb.setFont(prog.app.defaultFont);
        parent.add(lb);
        return lb;
    }

    public static void decorateStandardPanel(WebPanel JP) {
        JP.setWebColoredBackground(false);
        JP.setBackground(new java.awt.Color(0, 0, 0, 0));
        JP.setOpaque(false);
        JP.setUndecorated(false);
        JP.setShadeWidth(2);
        JP.setRound(com.alee.global.StyleConstants.largeRound);
        JP.setBorderColor(new java.awt.Color(0, 0, 0, 100));
        JP.setPaintBottom(false);
        JP.setPaintTop(false);
        JP.setPaintRight(false);
    }

    public static void decorateInsidePanel(WebPanel JP) {
        JP.setWebColoredBackground(false);
        JP.setBackground(new java.awt.Color(0, 0, 0, 0));
        JP.setOpaque(false);
        JP.setShadeTransparency(0.1f);
        JP.setShadeWidth(2);
        JP.setRound(com.alee.global.StyleConstants.largeRound);
        JP.setBorderColor(new java.awt.Color(0, 0, 0, 100));
    }

    public static com.alee.laf.combobox.WebComboBox addCrosshairList(Container parent, String text,
            boolean isInitializing, Runnable onSave) {
        WebLabel lb = new WebLabel(text);
        activeStyle.decorateLabel(lb);

        java.io.File file = new java.io.File("image/gunsight");
        String[] filelist = file.list();
        if (filelist == null)
            filelist = new String[0];
        filelist = getFilelistNameNoEx(filelist);

        com.alee.laf.combobox.WebComboBox comboBox = new com.alee.laf.combobox.WebComboBox(filelist);
        comboBox.setWebColoredBackground(false);
        comboBox.setShadeWidth(1);
        comboBox.setDrawFocus(false);
        comboBox.setFont(prog.app.defaultFont);
        comboBox.setExpandedBgColor(new java.awt.Color(0, 0, 0, 0));
        comboBox.addActionListener(e -> {
            if (isInitializing)
                return;
            if (onSave != null)
                onSave.run();
        });

        parent.add(lb);
        parent.add(comboBox);
        return comboBox;
    }

    private static String[] getFilelistNameNoEx(String[] filelist) {
        if (filelist == null)
            return new String[0];
        String[] res = new String[filelist.length];
        for (int i = 0; i < filelist.length; i++) {
            int dot = filelist[i].lastIndexOf('.');
            if ((dot > -1) && (dot < (filelist[i].length()))) {
                res[i] = filelist[i].substring(0, dot);
            }
        }
        return res;
    }

    public static void setStyle(UIStyle style) {
        activeStyle = style;
    }

    public static UIStyle getStyle() {
        return activeStyle;
    }

    public static WebPanel createGridContainer(int columns) {
        WebPanel container = new WebPanel();
        container.setLayout(new ResponsiveGridLayout(columns, 15, 5)); // Tighter gaps matching screenshot
        // activeStyle.decorateContainer(container); // If we add this method to
        // interface later
        container.setOpaque(false);
        container.setBackground(new java.awt.Color(0, 0, 0, 0));
        container.setBorder(null); // Ensure no default border
        return container;
    }

    public static void addPlaceholder(Container parent) {
        WebPanel p = new WebPanel();
        p.setOpaque(false);
        // Ensure it has the same height as a standard switch panel to maintain grid
        // alignment
        p.setPreferredSize(new Dimension(200, 30));
        parent.add(p);
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
    public static WebSwitch addSwitch(Container parent, String labelText, boolean initialValue) {
        // Flat Strategy: Add directly to parent to respect FlowLayout
        WebLabel label = new WebLabel(labelText);
        activeStyle.decorateLabel(label);

        WebSwitch webSwitch = new WebSwitch();
        webSwitch.setSelected(initialValue);
        activeStyle.decorateSwitch(webSwitch);

        parent.add(label);
        parent.add(webSwitch);
        return webSwitch;
    }

    public static WebSlider addSlider(Container parent, String labelText, int min, int max, int val, int width,
            int minorTick,
            int majorTick) {
        // Flat Strategy
        WebLabel label = new WebLabel(labelText);
        activeStyle.decorateLabel(label);

        // --- Alignment Logic (Legacy) ---
        // Preserving the check
        Dimension d2 = label.getPreferredSize();
        if (d2.width < 110) {
            label.setPreferredSize(new Dimension(110, d2.height));
        }

        WebSlider slider = new WebSlider(WebSlider.HORIZONTAL, min, max, val);
        slider.setMinorTickSpacing(minorTick);
        slider.setMajorTickSpacing(majorTick);
        slider.setPaintTicks(true);
        slider.setPaintLabels(true);

        activeStyle.decorateSlider(slider);
        slider.setPreferredSize(new Dimension(width, 50));

        parent.add(label);
        parent.add(slider);
        return slider;
    }

    public static WebSlider addSlider(Container parent, String labelText, int min, int max, int val, int width) {
        return addSlider(parent, labelText, min, max, val, width, 0, 0);
    }

    public static WebSlider addSlider(Container parent, String labelText, int min, int max, int val) {
        return addSlider(parent, labelText, min, max, val, 300, 0, 0);
    }

    public static com.alee.laf.combobox.WebComboBox addFontComboBox(Container parent, String labelText,
            String[] fonts) {
        return addComboBox(parent, labelText, fonts);
    }

    public static com.alee.laf.text.WebTextField addColorField(Container parent, String labelText, String colorText,
            java.awt.Color initialColor) {
        // Flat Strategy
        WebLabel label = new WebLabel(labelText);
        activeStyle.decorateLabel(label);

        com.alee.laf.text.WebTextField trailing = new com.alee.laf.text.WebTextField(colorText, 15);
        trailing.setMargin(0, 0, 0, 2);
        trailing.setLeadingComponent(
                new com.alee.extended.image.WebImage(com.alee.utils.ImageUtils.createColorIcon(initialColor)));
        trailing.setShadeWidth(2);

        parent.add(label);
        parent.add(trailing);
        return trailing;
    }

    public static com.alee.laf.combobox.WebComboBox addComboBox(Container parent, String labelText, String[] items) {
        // Flat Strategy
        WebLabel label = new WebLabel(labelText);
        activeStyle.decorateLabel(label);

        com.alee.laf.combobox.WebComboBox comboBox = new com.alee.laf.combobox.WebComboBox(items);
        comboBox.setWebColoredBackground(false);
        comboBox.setShadeWidth(1);
        comboBox.setDrawFocus(false);
        comboBox.setFont(prog.app.defaultFont);
        comboBox.setExpandedBgColor(new java.awt.Color(0, 0, 0, 0));
        comboBox.setBackground(new java.awt.Color(0, 0, 0, 0));

        parent.add(label);
        parent.add(comboBox);
        return comboBox;
    }

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
        btn.setFont(prog.app.defaultFontBig);
        btn.setPreferredSize(new Dimension(120, 80));

        // Basic styling consistent with mainform.createButton
        btn.setShadeWidth(1);
        btn.setDrawShade(true);
        btn.setTopBgColor(new java.awt.Color(0, 0, 0, 0));
        btn.setBottomBgColor(new java.awt.Color(0, 0, 0, 0));
        btn.setBorderPainted(false);

        return btn;
    }
}
