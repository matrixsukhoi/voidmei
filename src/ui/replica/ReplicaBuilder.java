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
        return createSwitchItem(labelText, isSelected, showGear, null, null);
    }

    public static WebPanel createSwitchItem(String labelText, boolean isSelected, boolean showGear, String tooltip) {
        return createSwitchItem(labelText, isSelected, showGear, tooltip, null);
    }

    /**
     * Creates a standard row item: Label on Left, Switch + Optional Gear on Right.
     */
    public static WebPanel createSwitchItem(String labelText, boolean isSelected, boolean showGear, String tooltip,
            String tooltipImg) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(2, 5, 2, 5));

        // Label
        WebLabel label = new WebLabel(labelText);
        if ((tooltip != null && !tooltip.isEmpty()) || (tooltipImg != null && !tooltipImg.isEmpty())) {
            applyStylizedTooltip(label, tooltip, tooltipImg);
        }
        style.decorateLabel(label);
        panel.add(label, BorderLayout.WEST);

        // Controls container (Switch + Gear)
        WebPanel controls = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 5, 0));
        controls.setOpaque(false);

        // Switch
        WebSwitch sw = new WebSwitch();
        sw.setSelected(isSelected);
        style.decorateSwitch(sw);
        controls.add(sw);

        // Gear Icon (Simulator)
        if (showGear) {
            JLabel gear = new JLabel("⚙");
            gear.setForeground(PinkStyle.COLOR_PRIMARY);
            gear.setFont(layerFont(14));
            controls.add(gear);
        }

        panel.add(controls, BorderLayout.CENTER);

        // Critical: Enable ResponsiveGrid alignment
        panel.putClientProperty("alignLabel", label);

        return panel;
    }

    public static WebPanel createSpinnerItem(String labelText, double value, double min, double max, double step) {
        return createSpinnerItem(labelText, value, min, max, step, null, null);
    }

    public static WebPanel createSpinnerItem(String labelText, double value, double min, double max, double step,
            String tooltip) {
        return createSpinnerItem(labelText, value, min, max, step, tooltip, null);
    }

    /**
     * Creates a spinner row: [Label] ... [Spinner]
     */
    public static WebPanel createSpinnerItem(String labelText, double value, double min, double max, double step,
            String tooltip, String tooltipImg) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        if ((tooltip != null && !tooltip.isEmpty()) || (tooltipImg != null && !tooltipImg.isEmpty())) {
            applyStylizedTooltip(label, tooltip, tooltipImg);
        }
        style.decorateLabel(label);
        panel.add(label, BorderLayout.WEST);

        // Spinner
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
        return createDropdownItem(labelText, items, null, null);
    }

    public static WebPanel createDropdownItem(String labelText, String[] items, String tooltip) {
        return createDropdownItem(labelText, items, tooltip, null);
    }

    /**
     * Creates a ComboBox Row: [Label] ... [ComboBox]
     */
    public static WebPanel createDropdownItem(String labelText, String[] items, String tooltip, String tooltipImg) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        if ((tooltip != null && !tooltip.isEmpty()) || (tooltipImg != null && !tooltipImg.isEmpty())) {
            applyStylizedTooltip(label, tooltip, tooltipImg);
        }
        style.decorateLabel(label);
        panel.add(label, BorderLayout.WEST);

        WebComboBox combo = new WebComboBox(items);
        combo.setEditable(false);

        panel.add(combo, BorderLayout.CENTER);

        // Critical: Enable ResponsiveGrid alignment
        panel.putClientProperty("alignLabel", label);

        return panel;
    }

    /**
     * Creates a text input row: [Label] ... [TextField]
     */
    public static WebPanel createTextItem(String labelText, String initialValue, int columns, String tooltip,
            String tooltipImg) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        if ((tooltip != null && !tooltip.isEmpty()) || (tooltipImg != null && !tooltipImg.isEmpty())) {
            applyStylizedTooltip(label, tooltip, tooltipImg);
        }
        style.decorateLabel(label);
        panel.add(label, BorderLayout.WEST);

        WebTextField textField = new WebTextField(initialValue, columns);
        textField.setPreferredSize(new Dimension(80, 26));
        textField.setDrawFocus(false);

        panel.add(textField, BorderLayout.EAST);

        // Critical: Enable ResponsiveGrid alignment
        panel.putClientProperty("alignLabel", label);

        return panel;
    }

    public static WebPanel createSliderItem(String labelText, int min, int max, int value, int width) {
        return createSliderItem(labelText, min, max, value, width, null, null);
    }

    public static WebPanel createSliderItem(String labelText, int min, int max, int value, int width, String tooltip) {
        return createSliderItem(labelText, min, max, value, width, tooltip, null);
    }

    /**
     * Creates a Slider row: [Label] ... [Slider]
     */
    public static WebPanel createSliderItem(String labelText, int min, int max, int value, int width, String tooltip,
            String tooltipImg) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        if ((tooltip != null && !tooltip.isEmpty()) || (tooltipImg != null && !tooltipImg.isEmpty())) {
            applyStylizedTooltip(label, tooltip, tooltipImg);
        }
        style.decorateLabel(label);
        panel.add(label, BorderLayout.WEST);

        com.alee.laf.slider.WebSlider slider = new com.alee.laf.slider.WebSlider(
                com.alee.laf.slider.WebSlider.HORIZONTAL, min, max, value);
        slider.setPreferredSize(new Dimension(width, 38)); // More compact height for labels
        slider.setOpaque(false);

        // --- Tick Setup ---
        int range = Math.abs(max - min);
        if (range > 0) {
            int major;
            if (range <= 10)
                major = 2;
            else if (range <= 20)
                major = 5;
            else if (range <= 50)
                major = 10;
            else if (range <= 100)
                major = 20;
            else if (range <= 500)
                major = 100;
            else
                major = range / 5;

            slider.setMajorTickSpacing(major);
            slider.setMinorTickSpacing(major / 5 > 0 ? major / 5 : 1);
        }

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
        return createColorField(labelText, colorText, initialColor, null, null);
    }

    public static WebPanel createColorField(String labelText, String colorText, Color initialColor, String tooltip) {
        return createColorField(labelText, colorText, initialColor, tooltip, null);
    }

    /**
     * Creates a Color field row: [Label] ... [Color Icon] [RGBA Text]
     */
    public static WebPanel createColorField(String labelText, String colorText, Color initialColor, String tooltip,
            String tooltipImg) {
        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        style.decorateControlPanel(panel);
        panel.setBorder(BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(labelText);
        if ((tooltip != null && !tooltip.isEmpty()) || (tooltipImg != null && !tooltipImg.isEmpty())) {
            applyStylizedTooltip(label, tooltip, tooltipImg);
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

    /**
     * Extracts the WebTextField from a panel created by createTextItem.
     */
    public static WebTextField getTextField(WebPanel itemPanel) {
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

    public static void applyStylizedTooltip(javax.swing.JComponent component, String text, String img) {
        // Remove standard tooltip if any
        component.setToolTipText(null);

        component.addMouseListener(new MouseAdapter() {
            private WebPopOver popover;

            @Override
            public void mouseEntered(MouseEvent e) {
                if (popover == null || !popover.isVisible()) {
                    popover = new WebPopOver(component);
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

                    // --- Construct HTML Content ---
                    StringBuilder html = new StringBuilder("<html><div style='padding:2px 8px;'>");
                    if (text != null && !text.isEmpty()) {
                        html.append("<span>").append(text).append("</span>");
                    }
                    if (img != null && !img.isEmpty()) {
                        if (text != null && !text.isEmpty()) {
                            html.append("<br>");
                        }
                        // Handle both "image.png" and "image/image.png"
                        String cleanImg = img;
                        if (cleanImg.startsWith("image/") || cleanImg.startsWith("image\\")) {
                            cleanImg = cleanImg.substring(6);
                        }
                        java.io.File imageFile = new java.io.File("image", cleanImg);
                        if (imageFile.exists()) {
                            String path = "file:///" + imageFile.getAbsolutePath();
                            html.append("<img src='").append(path).append("' width='200' height='150'>");
                        } else {
                            System.err.println(
                                    "[ReplicaBuilder] Tooltip image not found: " + imageFile.getAbsolutePath());
                            html.append("<div style='color:red; border:1px solid red; padding:5px;'>[Image Not Found: ")
                                    .append(cleanImg).append("]</div>");
                        }
                    }
                    html.append("</div></html>");

                    WebLabel content = new WebLabel(html.toString());
                    content.setFont(PinkStyle.FONT_NORMAL);
                    content.setForeground(PinkStyle.COLOR_TEXT);

                    bubble.add(content, BorderLayout.CENTER);
                    popover.add(bubble);

                    // Position: Place it tightly below the label
                    popover.show(component, component.getWidth() / 2 - 20, component.getHeight() - 10);

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
