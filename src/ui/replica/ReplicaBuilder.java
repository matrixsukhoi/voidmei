package ui.replica;

import java.awt.BorderLayout;
import java.awt.Dimension;
import java.awt.event.ActionListener;
import java.util.function.Consumer;

import javax.swing.JButton;
import javax.swing.BorderFactory;
import javax.swing.JLabel;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.text.WebTextField;
import com.alee.laf.spinner.WebSpinner;
import com.alee.laf.combobox.WebComboBox;

import java.awt.Color;
import java.awt.Font;

import prog.Application;

/**
 * Factory class to create specific UI components for the Replica layout.
 * Enforces the structure: [Label] ...... [Switch] [Gear]
 */
public class ReplicaBuilder {

    private static final PinkStyle style = new PinkStyle();

    /**
     * Creates a standard row item: Label on Left, Switch + Optional Gear on Right.
     */
    public static WebPanel createSwitchItem(String labelText, boolean isSelected, boolean showGear) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(2, 5, 2, 5));

        // Label
        WebLabel label = new WebLabel(labelText);
        style.decorateLabel(label);
        panel.add(label, BorderLayout.CENTER); // Center takes available space? No, West/East split usually better for
                                               // justified.
        // Actually, Borderlayout CENTER takes space. If we want label left and controls
        // right:
        // formatting: [Label (Center or West)] [Controls (East)]

        // Lets align Label to West if we want it to stick to left.
        // But if grids are consistent, standard flow is fine.
        // The screenshot shows columns. In a column, label is left, switch is right.

        panel.add(label, BorderLayout.WEST);

        // Controls container (Switch + Gear)
        // Use LEFT alignment so controls sit immediately next to the label (which is
        // forced to max width)
        WebPanel controls = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 5, 0));
        controls.setOpaque(false);

        // Switch
        WebSwitch sw = new WebSwitch();
        sw.setSelected(isSelected);
        style.decorateSwitch(sw);
        controls.add(sw);

        // Gear Icon (Simulator)
        if (showGear) {
            JLabel gear = new JLabel("⚙"); // Unicode gear as placeholder, or load icon
            gear.setForeground(PinkStyle.COLOR_PRIMARY);
            gear.setFont(layerFont(14));
            controls.add(gear);
        }

        // Use CENTER so it takes up remaining space, but FlowLayout.LEFT ensures it
        // starts near the label
        panel.add(controls, BorderLayout.CENTER);

        // Critical: Enable ResponsiveGrid alignment
        panel.putClientProperty("alignLabel", label);

        return panel;
    }

    /**
     * Creates a spinner row: [Label] ... [Spinner]
     */
    public static WebPanel createSpinnerItem(String labelText, double value, double min, double max, double step) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        style.decorateLabel(label);
        panel.add(label, BorderLayout.WEST);

        // Spinner
        // Note: javax.swing.SpinnerNumberModel required for doubles
        javax.swing.SpinnerNumberModel model = new javax.swing.SpinnerNumberModel(value, min, max, step);
        WebSpinner spinner = new WebSpinner(model);
        spinner.setPreferredSize(new Dimension(80, 26));
        spinner.setDrawFocus(false);

        panel.add(spinner, BorderLayout.EAST);

        // Critical: Enable ResponsiveGrid alignment
        panel.putClientProperty("alignLabel", label);

        return panel;
    }

    /**
     * Creates a ComboBox Row: [Label] ... [ComboBox]
     */
    public static WebPanel createDropdownItem(String labelText, String[] items) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        style.decorateLabel(label);

        // Vertical stacked label? Screenshot shows "Select Voice" (Label) ...
        // [Dropdown]

        // Actually screenshot shows "Select Voice" title above the dropdown or inline?
        // "选择声音" (Label) ... [jp 日语女声 V]
        // It looks like a standard row.

        panel.add(label, BorderLayout.WEST);

        WebComboBox combo = new WebComboBox(items);
        combo.setEditable(false);
        // combo.setPreferredSize(new Dimension(200, 26)); // Let it fill or fixed?

        // If we want it to fill the remaining space?
        // In BorderLayout, CENTER fills.
        panel.add(combo, BorderLayout.CENTER);

        // Critical: Enable ResponsiveGrid alignment
        panel.putClientProperty("alignLabel", label);

        return panel;
    }

    /**
     * Creates a Slider row: [Label] ... [Slider]
     */
    public static WebPanel createSliderItem(String labelText, int min, int max, int value, int width) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        style.decorateLabel(label);
        panel.add(label, BorderLayout.WEST);

        com.alee.laf.slider.WebSlider slider = new com.alee.laf.slider.WebSlider(
                com.alee.laf.slider.WebSlider.HORIZONTAL, min, max, value);
        slider.setPreferredSize(new Dimension(width, 30));
        slider.setOpaque(false);
        style.decorateSlider(slider);

        panel.add(slider, BorderLayout.CENTER);

        // Critical: Enable ResponsiveGrid alignment
        panel.putClientProperty("alignLabel", label);
        // Store slider reference for retrieval
        panel.putClientProperty("slider", slider);

        return panel;
    }

    /**
     * Extracts the WebSwitch from a panel created by createSwitchItem.
     */
    public static WebSwitch getSwitch(WebPanel itemPanel) {
        for (java.awt.Component c : itemPanel.getComponents()) {
            if (c instanceof WebPanel) {
                for (java.awt.Component inner : ((WebPanel) c).getComponents()) {
                    if (inner instanceof WebSwitch) {
                        return (WebSwitch) inner;
                    }
                }
            }
            if (c instanceof WebSwitch) {
                return (WebSwitch) c;
            }
        }
        return null;
    }

    /**
     * Extracts the WebSlider from a panel created by createSliderItem.
     */
    public static com.alee.laf.slider.WebSlider getSlider(WebPanel itemPanel) {
        Object slider = itemPanel.getClientProperty("slider");
        if (slider instanceof com.alee.laf.slider.WebSlider) {
            return (com.alee.laf.slider.WebSlider) slider;
        }
        return null;
    }

    /**
     * Creates a Color field row: [Label] ... [Color Icon] [RGBA Text]
     */
    public static WebPanel createColorField(String labelText, String colorText, Color initialColor) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        style.decorateLabel(label);
        panel.add(label, BorderLayout.WEST);

        WebTextField trailing = new WebTextField(colorText, 15);
        trailing.setMargin(0, 0, 0, 2);
        trailing.setLeadingComponent(
                new com.alee.extended.image.WebImage(com.alee.utils.ImageUtils.createColorIcon(initialColor)));
        trailing.setShadeWidth(2);

        panel.add(trailing, BorderLayout.CENTER);

        // Critical: Enable ResponsiveGrid alignment
        panel.putClientProperty("alignLabel", label);

        return panel;
    }

    /**
     * Extracts the WebComboBox from a panel created by createDropdownItem.
     */
    public static WebComboBox getComboBox(WebPanel itemPanel) {
        for (java.awt.Component c : itemPanel.getComponents()) {
            if (c instanceof WebComboBox) {
                return (WebComboBox) c;
            }
        }
        return null;
    }

    /**
     * Extracts the WebTextField from a panel created by createColorField.
     */
    public static WebTextField getColorField(WebPanel itemPanel) {
        for (java.awt.Component c : itemPanel.getComponents()) {
            if (c instanceof WebTextField) {
                return (WebTextField) c;
            }
        }
        return null;
    }

    // Helper to get font quickly
    private static Font layerFont(float size) {
        return Application.defaultFont.deriveFont(size);
    }

    public static PinkStyle getStyle() {
        return style;
    }
}
