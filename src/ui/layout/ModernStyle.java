package ui.layout;

import java.awt.Color;
import java.awt.Font;
import javax.swing.BorderFactory;
import javax.swing.border.TitledBorder;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

import prog.Application;

public class ModernStyle implements UIStyle {

    // Design System Colors
    private static final Color ACCENT_PINK = new Color(255, 105, 180); // Hot Pink
    private static final Color BG_MAIN = new Color(245, 245, 247); // Off-white/Light Gray
    private static final Color BG_CARD = Color.WHITE;
    private static final Color TEXT_HEADER = new Color(60, 60, 60);
    private static final Color TEXT_LABEL = new Color(80, 80, 80);
    private static final Color BORDER_COLOR = new Color(230, 230, 230);

    @Override
    public WebPanel createContainer(String title) {
        WebPanel card = new WebPanel();
        card.setOpaque(true);
        card.setBackground(BG_CARD);
        // Use BorderLayout to ensure the inner ResponsiveGrid fills the card width
        card.setLayout(new java.awt.BorderLayout());

        // Modern flat border with title
        TitledBorder border = BorderFactory.createTitledBorder(
                BorderFactory.createLineBorder(BORDER_COLOR, 1),
                title != null ? title : "");
        border.setTitleColor(ACCENT_PINK);
        border.setTitleFont(Application.defaultFont.deriveFont(Font.BOLD, 14f));
        card.setBorder(BorderFactory.createCompoundBorder(
                BorderFactory.createEmptyBorder(5, 10, 5, 10), // Outer margin
                BorderFactory.createCompoundBorder(
                        border,
                        BorderFactory.createEmptyBorder(10, 15, 10, 15) // Inner padding
                )));

        return card;
    }

    @Override
    public void decorateControlPanel(WebPanel panel) {
        panel.setOpaque(false);
    }

    @Override
    public void decorateLabel(WebLabel label) {
        label.setFont(Application.defaultFont);
        label.setForeground(TEXT_LABEL);
    }

    @Override
    public void decorateSwitch(WebSwitch webSwitch) {
        // Modern Capsule Switch
        webSwitch.setRound(10); // Fully rounded
        // Standard WebSwitch color setters are limited.
        // We rely on LookAndFeel or basic properties.
        webSwitch.setWebColoredBackground(true); // Attempt to engage colored mode
    }

    @Override
    public void decorateSlider(WebSlider slider) {
        slider.setPaintTicks(false);
        slider.setPaintLabels(false);
        slider.setOpaque(false);
        // WebLaF sliders are hard to style via simple methods,
        // but removing ticks helps the clean look.
    }

    @Override
    public void decorateMainPanel(WebPanel panel) {
        // Modern style uses a light gray background for the main panel
        panel.setOpaque(true);
        panel.setBackground(BG_MAIN);
    }
}
