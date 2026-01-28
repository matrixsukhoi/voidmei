package ui.layout.renderer;

import com.alee.laf.button.WebButton;
import com.alee.laf.panel.WebPanel;
import com.alee.extended.layout.VerticalFlowLayout;

import java.awt.event.ActionListener;
import java.awt.event.ActionEvent;

import prog.Application;
import prog.config.ConfigLoader;
import prog.config.ConfigLoader.RowConfig;
import ui.replica.ReplicaBuilder;

public class ButtonRowRenderer implements RowRenderer {

    @Override
    public WebPanel render(RowConfig row, ConfigLoader.GroupConfig group, RenderContext ctx) {
        WebPanel p = new WebPanel(new VerticalFlowLayout(0, 5));
        p.setOpaque(false);

        WebButton btn = new WebButton(row.label);
        btn.setFont(Application.defaultFont);
        btn.setFocusable(false);
        if ((row.desc != null && !row.desc.isEmpty()) || (row.descImg != null && !row.descImg.isEmpty())) {
            // Re-using applyStylizedTooltip, casting button to WebLabel as it likely works
            // but WebButton might need its own if it doesn't extend properly.
            // Actually WebButton inherits from WebLabel in some LAF versions, but let's
            // check.
            // If it doesn't, we might need a custom one. For now let's try standard.
            ReplicaBuilder.applyStylizedTooltip(btn, row.desc, row.descImg);
        }

        // Handle Action
        if ("resetConfig".equals(row.property)) {
            btn.addActionListener(new ActionListener() {
                @Override
                public void actionPerformed(ActionEvent e) {
                    // Show confirmation dialog
                    int result = com.alee.laf.optionpane.WebOptionPane.showConfirmDialog(
                            p,
                            prog.i18n.Lang.mResetConfirmContent,
                            prog.i18n.Lang.mResetConfirmTitle,
                            com.alee.laf.optionpane.WebOptionPane.YES_NO_OPTION,
                            com.alee.laf.optionpane.WebOptionPane.WARNING_MESSAGE);

                    if (result == com.alee.laf.optionpane.WebOptionPane.YES_OPTION) {
                        // Publish request to reset configs (EDA implementation)
                        prog.event.UIStateBus.getInstance().publish(
                                prog.event.UIStateEvents.CONFIG_CHANGED,
                                "ButtonRowRenderer",
                                prog.event.UIStateEvents.ACTION_RESET_REQUEST);
                    }
                }
            });
        }

        if ("openComparison".equals(row.property)) {
            btn.addActionListener(e -> {
                String fm0 = ctx.getStringFromConfigService("selectedFM0", "a_4h");
                String fm1 = ctx.getStringFromConfigService("selectedFM1", "a6m5_zero");

                java.awt.Window parent = javax.swing.SwingUtilities.getWindowAncestor(p);
                ui.window.comparison.CompactComparisonWindow win = new ui.window.comparison.CompactComparisonWindow(
                        parent, Application.ctr, fm0, fm1);
                win.setVisible(true);
            });
        }

        if (row.fgColor != null) {
            java.awt.Color color = parseColor(row.fgColor);
            if (color != null) {
                btn.setForeground(color);
            }
        }

        p.add(btn);
        return p;
    }

    private java.awt.Color parseColor(String s) {
        if (s == null)
            return null;
        try {
            String[] p = s.split(",");
            if (p.length >= 3) {
                return new java.awt.Color(
                        Integer.parseInt(p[0].trim()),
                        Integer.parseInt(p[1].trim()),
                        Integer.parseInt(p[2].trim()));
            }
        } catch (Exception e) {
        }
        return null;
    }
}
