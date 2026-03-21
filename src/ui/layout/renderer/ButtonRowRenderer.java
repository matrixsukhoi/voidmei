package ui.layout.renderer;

import com.alee.laf.button.WebButton;
import com.alee.laf.panel.WebPanel;
import com.alee.extended.layout.VerticalFlowLayout;

import java.awt.event.ActionListener;
import java.awt.event.ActionEvent;
import java.io.File;

import prog.Application;
import prog.config.ConfigLoader;
import prog.config.ConfigLoader.RowConfig;
import prog.i18n.Lang;
import ui.replica.ReplicaBuilder;
import ui.util.DialogService;

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

                    // Show confirmation dialog (using DialogService to avoid overlay blocking)
                    int result = DialogService.showConfirmDialog(
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

        // Handle import config button - 使用现代化拖放导入对话框
        if ("importConfig".equals(row.property)) {
            btn.addActionListener(e -> {
                // 关闭所有打开的 tooltip/popover，防止被对话框覆盖
                ReplicaBuilder.disposeAllPopovers();

                // 使用静态方法入口，内置 CAS 防重复点击保护
                ConfigImportDialog.showDialog(btn, file -> {
                    handleConfigImport(p, file);
                });
            });
        }

        // Handle factory reset button
        if ("factoryReset".equals(row.property)) {
            btn.addActionListener(e -> {
                // Dismiss any open tooltips before showing dialog
                ReplicaBuilder.disposeAllPopovers();

                // Show confirmation dialog (using DialogService to avoid overlay blocking)
                int result = DialogService.showConfirmDialog(
                        p,
                        Lang.mFactoryResetConfirmContent,
                        Lang.mFactoryResetConfirmTitle,
                        com.alee.laf.optionpane.WebOptionPane.YES_NO_OPTION,
                        com.alee.laf.optionpane.WebOptionPane.WARNING_MESSAGE);

                if (result == com.alee.laf.optionpane.WebOptionPane.YES_OPTION) {
                    // Perform factory reset - hot reload happens via CONFIG_CHANGED event
                    boolean success = Application.ctr.getConfigService().resetToFactory();
                    if (!success) {
                        DialogService.showMessageDialog(
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

    /**
     * 处理配置文件导入逻辑
     *
     * @param parentPanel 父面板，用于显示对话框
     * @param file        要导入的配置文件
     */
    private void handleConfigImport(WebPanel parentPanel, File file) {
        // 显示确认对话框 (使用 DialogService 避免 overlay 遮挡问题)
        int confirmResult = DialogService.showConfirmDialog(
                parentPanel,
                Lang.mImportConfirmContent,
                Lang.mImportConfirmTitle,
                com.alee.laf.optionpane.WebOptionPane.YES_NO_OPTION,
                com.alee.laf.optionpane.WebOptionPane.WARNING_MESSAGE);

        if (confirmResult == com.alee.laf.optionpane.WebOptionPane.YES_OPTION) {
            // 执行导入 - 热重载通过 CONFIG_CHANGED 事件触发
            boolean success = Application.ctr.getConfigService().importConfig(file.getAbsolutePath());
            if (!success) {
                DialogService.showMessageDialog(
                        parentPanel,
                        Lang.mImportFailContent,
                        Lang.mImportFailTitle,
                        com.alee.laf.optionpane.WebOptionPane.ERROR_MESSAGE);
            }
        }
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
