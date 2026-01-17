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

    @Override
    public WebPanel createContainer(String title) {
        WebPanel card = new WebPanel();
        card.setLayout(new FlexGridLayout(10, 10));

        TitledBorder border = BorderFactory.createTitledBorder(
                BorderFactory.createLineBorder(new Color(200, 200, 200, 50)), title);
        border.setTitleColor(Color.WHITE);
        border.setTitleFont(Application.defaultFont);
        card.setBorder(border);

        card.setOpaque(false);
        card.setBackground(new Color(0, 0, 0, 0));
        return card;
    }

    @Override
    public void decorateControlPanel(WebPanel panel) {
        panel.setOpaque(false);
        // Modern style uses default flow layout spacing from UIBuilder
    }

    @Override
    public void decorateLabel(WebLabel label) {
        label.setFont(Application.defaultFont);
        label.setForeground(Color.WHITE);
    }

    @Override
    public void decorateSwitch(WebSwitch webSwitch) {
        // Default WebLaf switch style
        // Maybe minimal customization if needed
    }

    @Override
    public void decorateSlider(WebSlider slider) {
        slider.setPaintTicks(false);
        slider.setPaintLabels(false);
    }
}
