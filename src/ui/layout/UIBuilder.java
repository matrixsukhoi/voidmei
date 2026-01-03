package ui.layout;

import java.awt.Container;
import java.awt.Dimension;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

public class UIBuilder {

    private static UIStyle activeStyle = new ClassicStyle();

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
        // Use FlowLayout to prevent vertical stretching of the switch
        WebPanel p = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 5, 0));
        p.setOpaque(false);

        WebLabel label = new WebLabel(labelText);
        activeStyle.decorateLabel(label);

        WebSwitch ws = new WebSwitch();
        ws.setSelected(initialValue);
        activeStyle.decorateSwitch(ws);

        p.add(label);
        p.add(ws);

        // Store label reference for ResponsiveGridLayout to enable dynamic column
        // alignment
        p.putClientProperty("alignLabel", label);

        parent.add(p);
        return ws;
    }

    public static WebSlider addSlider(Container parent, String labelText, int min, int max, int val) {
        WebPanel p = new WebPanel(new java.awt.BorderLayout(5, 0));

        WebLabel label = new WebLabel(labelText);
        activeStyle.decorateLabel(label);

        // --- Alignment Logic ---
        Dimension d2 = label.getPreferredSize();
        if (d2.width < 110) {
            label.setPreferredSize(new Dimension(110, d2.height));
        }

        WebSlider slider = new WebSlider(WebSlider.HORIZONTAL, min, max, val);
        activeStyle.decorateSlider(slider);

        p.setPreferredSize(new Dimension(300, 30));
        p.add(label, java.awt.BorderLayout.WEST);
        p.add(slider, java.awt.BorderLayout.CENTER);

        activeStyle.decorateControlPanel(p);

        // Store label reference for ResponsiveGridLayout
        p.putClientProperty("alignLabel", label);

        parent.add(p);
        return slider;
    }

    public static com.alee.laf.combobox.WebComboBox addComboBox(Container parent, String labelText, String[] items) {
        WebPanel p = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 5, 0));
        p.setOpaque(false);

        WebLabel label = new WebLabel(labelText);
        activeStyle.decorateLabel(label);

        com.alee.laf.combobox.WebComboBox comboBox = new com.alee.laf.combobox.WebComboBox(items);
        comboBox.setWebColoredBackground(false);
        comboBox.setShadeWidth(1);
        comboBox.setDrawFocus(false);
        comboBox.setFont(prog.app.defaultFont);
        comboBox.setExpandedBgColor(new java.awt.Color(0, 0, 0, 0));
        comboBox.setBackground(new java.awt.Color(0, 0, 0, 0));

        p.add(label);
        p.add(comboBox);

        // Store label reference for ResponsiveGridLayout
        p.putClientProperty("alignLabel", label);

        parent.add(p);
        return comboBox;
    }
}
