package ui.replica;

import java.awt.BorderLayout;
import java.awt.Dimension;
import javax.swing.BorderFactory;
import javax.swing.JLabel;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.text.WebTextField;
import com.alee.laf.spinner.WebSpinner;
import com.alee.laf.combobox.WebComboBox;
import com.alee.extended.window.WebPopOver;

import java.awt.Color;
import java.awt.Font;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;

import prog.Application;

/**
 * Factory class to create specific UI components for the Replica layout.
 * Enforces the structure: [Label] ...... [Switch] [Gear]
 */
public class ReplicaBuilder {

    private static final PinkStyle style = new PinkStyle();

    public static WebPanel createSwitchItem(String labelText, boolean isSelected, boolean showGear) {
        return createSwitchItem(labelText, isSelected, showGear, null);
    }

    /**
     * Creates a standard row item: Label on Left, Switch + Optional Gear on Right.
     */
    public static WebPanel createSwitchItem(String labelText, boolean isSelected, boolean showGear, String tooltip) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(2, 5, 2, 5));

        // Label
        WebLabel label = new WebLabel(labelText);
        if (tooltip != null && !tooltip.isEmpty()) {
            applyStylizedTooltip(label, tooltip);
        }
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

    public static WebPanel createSpinnerItem(String labelText, double value, double min, double max, double step) {
        return createSpinnerItem(labelText, value, min, max, step, null);
    }

    /**
     * Creates a spinner row: [Label] ... [Spinner]
     */
    public static WebPanel createSpinnerItem(String labelText, double value, double min, double max, double step,
            String tooltip) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        if (tooltip != null && !tooltip.isEmpty()) {
            label.setToolTipText(tooltip);
        }
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

    public static WebPanel createDropdownItem(String labelText, String[] items) {
        return createDropdownItem(labelText, items, null);
    }

    /**
     * Creates a ComboBox Row: [Label] ... [ComboBox]
     */
    public static WebPanel createDropdownItem(String labelText, String[] items, String tooltip) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        if (tooltip != null && !tooltip.isEmpty()) {
            applyStylizedTooltip(label, tooltip);
        }
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

    public static WebPanel createSliderItem(String labelText, int min, int max, int value, int width) {
        return createSliderItem(labelText, min, max, value, width, null);
    }

    /**
     * Creates a Slider row: [Label] ... [Slider]
     */
    public static WebPanel createSliderItem(String labelText, int min, int max, int value, int width, String tooltip) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        if (tooltip != null && !tooltip.isEmpty()) {
            applyStylizedTooltip(label, tooltip);
        }
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

    public static WebPanel createColorField(String labelText, String colorText, Color initialColor) {
        return createColorField(labelText, colorText, initialColor, null);
    }

    /**
     * Creates a Color field row: [Label] ... [Color Icon] [RGBA Text]
     */
    public static WebPanel createColorField(String labelText, String colorText, Color initialColor, String tooltip) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        if (tooltip != null && !tooltip.isEmpty()) {
            applyStylizedTooltip(label, tooltip);
        }
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

    private static void applyStylizedTooltip(WebLabel label, String text) {
        // Remove standard tooltip if any
        label.setToolTipText(null);

        label.addMouseListener(new MouseAdapter() {
            private WebPopOver popover;

            @Override
            public void mouseEntered(MouseEvent e) {
                if (popover == null || !popover.isVisible()) {
                    popover = new WebPopOver(label);
                    // Use a safe, stable margin
                    popover.setMargin(0);
                    popover.setMovable(false);
                    popover.setFocusableWindowState(false);
                    popover.setFocusable(false);
                    popover.setCloseOnFocusLoss(true);

                    // Kill shadow and artifacts
                    popover.setShadeWidth(0);
                    popover.setWindowOpaque(false);
                    popover.setBackground(new Color(0, 0, 0, 0));

                    // Create our own "Bubble" with perfect control
                    WebPanel bubble = new WebPanel(new BorderLayout());
                    bubble.setOpaque(true);
                    bubble.setBackground(Color.WHITE);
                    bubble.setBorder(javax.swing.BorderFactory.createLineBorder(PinkStyle.COLOR_PRIMARY, 1));

                    WebLabel content = new WebLabel(text);
                    content.setFont(PinkStyle.FONT_NORMAL);
                    content.setForeground(PinkStyle.COLOR_TEXT);
                    content.setBorder(javax.swing.BorderFactory.createEmptyBorder(2, 8, 2, 8));

                    bubble.add(content, BorderLayout.CENTER);
                    popover.add(bubble);

                    // Position: Place it tightly below the label
                    popover.show(label, label.getWidth() / 2 - 20, label.getHeight() - 10);

                    // Reduce arrow size to ~1/3 of default (10 -> 4)
                    popover.setCornerWidth(4);

                    // Force the window to the very front to prevent label-clipping
                    java.awt.Window win = javax.swing.SwingUtilities.getWindowAncestor(popover);
                    if (win != null) {
                        win.toFront();
                    }
                }
            }

            @Override
            public void mouseExited(MouseEvent e) {
                if (popover != null) {
                    popover.dispose();
                    popover = null;
                }
            }
        });
    }
}
