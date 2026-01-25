package ui.replica;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Dimension;
import java.awt.Font;
import java.util.function.Consumer;
import java.beans.PropertyChangeEvent;
import java.beans.PropertyChangeListener;

import javax.swing.BorderFactory;
import javax.swing.border.TitledBorder;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

import ui.layout.UIStyle;
import prog.Application;

public class PinkStyle implements UIStyle {

    // Visual Constants derived from screenshot
    public static final Color COLOR_PRIMARY = new Color(255, 105, 180); // Hot Pink
    public static final Color COLOR_BG_PANEL = new Color(255, 255, 255); // White content
    public static final Color COLOR_BG_MAIN = new Color(245, 245, 245); // Light Grey Sidebar/Bg
    public static final Color COLOR_BORDER = new Color(230, 230, 230); // Light Grey Border
    public static final Color COLOR_TEXT = new Color(51, 51, 51); // Dark Grey Text
    public static final Color COLOR_SWITCH_OFF = new Color(150, 150, 150); // Grey for Off

    public static final Font FONT_HEADER = Application.defaultFont.deriveFont(Font.BOLD, 14f);
    public static final Font FONT_NORMAL = Application.defaultFont.deriveFont(12f);

    @Override
    public WebPanel createContainer(String title) {
        WebPanel card = new WebPanel();
        card.setOpaque(true);
        card.setBackground(COLOR_BG_PANEL);
        card.setLayout(new BorderLayout()); // Will hold the grid

        // Titled Border with custom coloring
        TitledBorder border = BorderFactory.createTitledBorder(
                BorderFactory.createLineBorder(COLOR_BORDER, 1),
                title);
        border.setTitleFont(FONT_HEADER);
        border.setTitleColor(COLOR_TEXT);

        card.setBorder(BorderFactory.createCompoundBorder(
                BorderFactory.createEmptyBorder(5, 5, 10, 5), // Outer margin
                BorderFactory.createCompoundBorder(
                        border,
                        BorderFactory.createEmptyBorder(10, 10, 10, 10) // Inner padding
                )));

        return card;
    }

    @Override
    public void decorateControlPanel(WebPanel panel) {
        panel.setOpaque(false);
    }

    @Override
    public void decorateLabel(WebLabel label) {
        label.setFont(FONT_NORMAL);
        label.setForeground(COLOR_TEXT);
    }

    @Override
    public void decorateSwitch(WebSwitch webSwitch) {
        // Capsule Styling
        webSwitch.setRound(12); // More rounded like capsule - User preference
        webSwitch.setPreferredSize(new Dimension(50, 24));

        // Explicitly set colors
        // Disable "WebColored" mode because it forces default look
        webSwitch.setWebColoredBackground(false);

        // Define colors
        Color colorOnBg = COLOR_PRIMARY;
        Color colorOnKnob = Color.WHITE;

        Color colorOffBg = Color.WHITE;
        Color colorOffKnob = new Color(80, 80, 80); // Dark Grey
        Color borderColor = new Color(200, 200, 200); // Light Grey Border

        // Helper to update colors
        Consumer<Boolean> updateColors = (selected) -> {
            if (selected) {
                webSwitch.setBackground(colorOnBg);
                webSwitch.setForeground(colorOnKnob);
                // Remove border for ON state
                webSwitch.setShadeWidth(0);
                // Force border color to match background heavily to hide any artifacts
                webSwitch.setBorderColor(colorOnBg);
            } else {
                webSwitch.setBackground(colorOffBg);
                webSwitch.setForeground(colorOffKnob);
                // Enable border (shade) for OFF state
                webSwitch.setShadeWidth(1);
                webSwitch.setBorderColor(borderColor);
            }
            webSwitch.repaint();
        };

        // Initial State via invokeLater
        javax.swing.SwingUtilities.invokeLater(() -> {
            updateColors.accept(webSwitch.isSelected());
        });

        // Add Listener to toggle color (User Clicks)
        webSwitch.addActionListener(e -> {
            updateColors.accept(webSwitch.isSelected());
        });

        // Add PropertyChangeListener to catch programmatic changes (Data Binding)
        // WebSwitch likely uses "selected" property
        webSwitch.addPropertyChangeListener("selected", new PropertyChangeListener() {
            @Override
            public void propertyChange(PropertyChangeEvent evt) {
                updateColors.accept(webSwitch.isSelected());
            }
        });

        // Reset text
        webSwitch.getLeftComponent().setText("");
        webSwitch.getRightComponent().setText("");
    }

    @Override
    public void decorateSlider(WebSlider slider) {
        slider.setOpaque(false);
        slider.setPaintTicks(true);
        slider.setPaintLabels(true);
        slider.setFont(FONT_NORMAL.deriveFont(9f)); // Even smaller font for ticks
        slider.setForeground(COLOR_TEXT);

        // Customizing the appearance if possible through WebLaF properties
        // We'll stick to standard JSlider/WebSlider properties for compatibility
    }

    @Override
    public void decorateMainPanel(WebPanel panel) {
        panel.setOpaque(true);
        panel.setBackground(COLOR_BG_MAIN);
    }
}
