package ui.layout.renderer;

import com.alee.laf.button.WebButton;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.filechooser.WebFileChooser;
import com.alee.extended.layout.VerticalFlowLayout;

import java.awt.event.ActionListener;
import java.awt.event.ActionEvent;
import java.io.File;

import javax.swing.JFileChooser;
import javax.swing.filechooser.FileNameExtensionFilter;

import prog.Application;
import prog.config.ConfigLoader;
import prog.config.ConfigLoader.RowConfig;
import prog.i18n.Lang;
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
                    // Dismiss any open tooltips before showing dialog
                    ReplicaBuilder.disposeAllPopovers();

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
                // Dismiss any open tooltips before showing window
                ReplicaBuilder.disposeAllPopovers();

                String fm0 = ctx.getStringFromConfigService("selectedFM0", "a_4h");
                String fm1 = ctx.getStringFromConfigService("selectedFM1", "a6m5_zero");

                java.awt.Window parent = javax.swing.SwingUtilities.getWindowAncestor(p);
                ui.window.comparison.CompactComparisonWindow win = new ui.window.comparison.CompactComparisonWindow(
                        parent, Application.ctr, fm0, fm1);
                win.setVisible(true);
            });
        }

        if ("openPowerCurve".equals(row.property)) {
            btn.addActionListener(e -> {
                // Dismiss any open tooltips before showing window
                ReplicaBuilder.disposeAllPopovers();

                // Read configuration for both FMs
                String fm0Name = ctx.getStringFromConfigService("selectedFM0", "bf-109f-4");
                String fm1Name = ctx.getStringFromConfigService("selectedFM1", "");

                // Parse speed from string config
                int speed = 0;
                try {
                    String speedStr = ctx.getStringFromConfigService("powerCurveSpeed", "0");
                    speed = Integer.parseInt(speedStr);
                } catch (Exception ex) {
                    speed = 0;
                }

                // Parse WEP mode from boolean config
                boolean wep = ctx.getFromConfigService("powerCurveWep", false);

                // Open power curve window with both FMs
                java.awt.Window parent = javax.swing.SwingUtilities.getWindowAncestor(p);
                ui.window.comparison.PowerCurveWindow win =
                    new ui.window.comparison.PowerCurveWindow(parent, fm0Name, fm1Name, speed, wep);
                win.setVisible(true);
            });
        }

        // Handle import config button
        if ("importConfig".equals(row.property)) {
            btn.addActionListener(e -> {
                // Dismiss any open tooltips before showing file chooser
                ReplicaBuilder.disposeAllPopovers();

                // Show file chooser
                WebFileChooser fileChooser = new WebFileChooser();
                fileChooser.setDialogTitle(Lang.mImportConfigTitle);
                fileChooser.setFileFilter(new FileNameExtensionFilter("Config files (*.cfg)", "cfg"));
                fileChooser.setMultiSelectionEnabled(false);

                int result = fileChooser.showOpenDialog(javax.swing.SwingUtilities.getWindowAncestor(p));
                if (result == JFileChooser.APPROVE_OPTION) {
                    File selectedFile = fileChooser.getSelectedFile();

                    // Show confirmation dialog
                    int confirmResult = com.alee.laf.optionpane.WebOptionPane.showConfirmDialog(
                            p,
                            Lang.mImportConfirmContent,
                            Lang.mImportConfirmTitle,
                            com.alee.laf.optionpane.WebOptionPane.YES_NO_OPTION,
                            com.alee.laf.optionpane.WebOptionPane.WARNING_MESSAGE);

                    if (confirmResult == com.alee.laf.optionpane.WebOptionPane.YES_OPTION) {
                        // Perform import - hot reload happens via CONFIG_CHANGED event
                        boolean success = Application.ctr.getConfigService().importConfig(selectedFile.getAbsolutePath());
                        if (!success) {
                            com.alee.laf.optionpane.WebOptionPane.showMessageDialog(
                                    p,
                                    Lang.mImportFailContent,
                                    Lang.mImportFailTitle,
                                    com.alee.laf.optionpane.WebOptionPane.ERROR_MESSAGE);
                        }
                    }
                }
            });
        }

        // Handle factory reset button
        if ("factoryReset".equals(row.property)) {
            btn.addActionListener(e -> {
                // Dismiss any open tooltips before showing dialog
                ReplicaBuilder.disposeAllPopovers();

                // Show confirmation dialog
                int result = com.alee.laf.optionpane.WebOptionPane.showConfirmDialog(
                        p,
                        Lang.mFactoryResetConfirmContent,
                        Lang.mFactoryResetConfirmTitle,
                        com.alee.laf.optionpane.WebOptionPane.YES_NO_OPTION,
                        com.alee.laf.optionpane.WebOptionPane.WARNING_MESSAGE);

                if (result == com.alee.laf.optionpane.WebOptionPane.YES_OPTION) {
                    // Perform factory reset - hot reload happens via CONFIG_CHANGED event
                    boolean success = Application.ctr.getConfigService().resetToFactory();
                    if (!success) {
                        com.alee.laf.optionpane.WebOptionPane.showMessageDialog(
                                p,
                                Lang.mFactoryResetFailContent,
                                Lang.mFactoryResetFailTitle,
                                com.alee.laf.optionpane.WebOptionPane.ERROR_MESSAGE);
                    }
                }
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
