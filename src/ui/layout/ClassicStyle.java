package ui.layout;

import java.awt.Color;
import java.awt.Font;

import javax.swing.SwingConstants;

import com.alee.extended.button.WebSwitch;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

import prog.Application;

public class ClassicStyle implements UIStyle {

    @Override
    public WebPanel createContainer(String title) {
        WebPanel container = new WebPanel();
        // Use FlexGridLayout to keep the "Auto Layout" feature the user likes
        container.setLayout(new FlexGridLayout(15, 10));

        // Classic style has NO visible border/card background for groups
        // But we might want to show the title if provided?
        // For strict classic compatibility, maybe just a label header?
        // Let's stick to borderless container for now.
        container.setOpaque(false);
        container.setBackground(new Color(0, 0, 0, 0));

        if (title != null && !title.isEmpty()) {
            // In classic style, titles are usually just WebLabels in the flow
            // But strict container return means we assume the caller handles title
            // hierarchy
            // Or we could add a TitledBorder that matches the classic theme (none usually)
        }
        return container;
    }

    @Override
    public void decorateControlPanel(WebPanel panel) {
        panel.setOpaque(false);
        // Tweak size or spacing if necessary to match classic density
    }

    @Override
    public void decorateLabel(WebLabel label) {
        label.setHorizontalAlignment(SwingConstants.LEFT); // Left align as requested
        label.setForeground(new Color(0, 0, 0, 230));
        label.setShadeColor(Color.WHITE);
        label.setFont(Application.defaultFont);
    }

    @Override
    public void decorateSwitch(WebSwitch ws) {
        // Replicating logic from MainForm.createLCGroup
        ws.getWebUI().setShadeWidth(0);
        ws.setWebColoredBackground(false);
        ws.setBackground(new Color(255, 255, 255, 255));
        ws.getWebUI().setPaintSides(true, false, true, false);
        ws.setRound(5);
        ws.setShadeWidth(1);

        Font numFont = new Font(Application.defaultNumfontName, Font.PLAIN, 14);
        ws.getLeftComponent().setFont(numFont);
        ws.getRightComponent().setFont(numFont);

        ws.getLeftComponent().setDrawShade(false);
        ws.getRightComponent().setDrawShade(false);

        ws.getLeftComponent().setText("On");
        ws.getRightComponent().setText("Off");
    }

    @Override
    public void decorateSlider(WebSlider slider) {
        // Classic sliders usually look standard, just transparency
        slider.setOpaque(false);
    }
}
