package ui.layout;

import java.awt.BorderLayout;
import java.awt.Component;
import java.awt.Dimension;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.Insets;
import java.io.BufferedReader;
import java.io.File;
import java.io.FileInputStream;
import java.io.InputStreamReader;
import java.util.ArrayList;
import java.util.List;

import javax.swing.SwingConstants;

import com.alee.extended.layout.VerticalFlowLayout;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;

import ui.mainform;
import parser.blkx;

public class DynamicDataPage extends BasePage {

    private ZoomPanel scaler;
    private ui.util.ConfigLoader.GroupConfig groupConfig;
    private java.util.List<DataBinding> bindings = new ArrayList<>();

    private class DataBinding {
        WebLabel label;
        String formula;
        String format;

        DataBinding(WebLabel label, String formula, String format) {
            this.label = label;
            this.formula = formula;
            this.format = format;
        }
    }

    // Constructor now accepts a specific GroupConfig
    public DynamicDataPage(mainform parent, ui.util.ConfigLoader.GroupConfig groupConfig) {
        super(parent);
        this.groupConfig = groupConfig;
        rebuild();
    }

    // Default constructor for compatibility if needed, though we should use the one
    // above.
    public DynamicDataPage(mainform parent) {
        super(parent);
    }

    public void setGroupConfig(ui.util.ConfigLoader.GroupConfig groupConfig) {
        this.groupConfig = groupConfig;
        rebuild();
    }

    @Override
    protected void initContent(WebPanel content) {
        scaler = new ZoomPanel();
        scaler.setOpaque(false);
        scaler.setLayout(new VerticalFlowLayout(0, 5));

        // If config is already available (unlikely due to super call order), build.
        // Otherwise wait for setGroupConfig or manual call.
    }

    private void rebuild() {
        scaler.removeAll();
        bindings.clear();

        if (groupConfig == null)
            return;

        // Build UI from GroupConfig
        // Header is the tab title usually, but we can add a title inside too if needed.
        // WebLabel header = new WebLabel(groupConfig.title);
        // header.setFont(prog.app.defaultFontBig);
        // ...

        for (ui.util.ConfigLoader.RowConfig row : groupConfig.rows) {
            // Check if it is a Header
            if (row.formula != null && row.formula.trim().equalsIgnoreCase("HEADER")) {
                WebLabel header = new WebLabel(row.label);
                header.setFont(prog.app.defaultFontBig);
                header.setForeground(new java.awt.Color(200, 200, 100)); // Yellowish
                header.setMargin(new java.awt.Insets(10, 0, 5, 0));
                scaler.add(header);
                continue;
            }

            WebLabel lb = new WebLabel(row.label);
            lb.setFont(prog.app.defaultFont);

            // Value Label (initially empty or waiting...)
            WebLabel val = new WebLabel("...");
            val.setFont(prog.app.defaultFont);
            val.setForeground(java.awt.Color.WHITE);

            // Register binding for updates
            if (row.formula != null && !row.formula.isEmpty()) {
                bindings.add(new DataBinding(val, row.formula, row.format));
            }

            WebPanel flowRow = new WebPanel(new java.awt.FlowLayout(java.awt.FlowLayout.LEFT, 5, 0));
            flowRow.setOpaque(false);
            flowRow.add(lb);
            flowRow.add(val);

            scaler.add(flowRow);
        }
        scaler.revalidate();
        scaler.repaint();
    }

    private ui.overlay.DynamicOverlay overlay;

    public void update() {
        if (overlay == null) {
            overlay = new ui.overlay.DynamicOverlay(parent, groupConfig);
        }

        // Update Tab UI
        if (parent.tc.blkx == null)
            return;
        java.util.Map<String, Object> vars = parent.tc.blkx.getVariableMap();

        for (DataBinding b : bindings) {
            try {
                Object result = prog.util.FormulaEvaluator.evaluate(b.formula, vars);
                if (result instanceof Number) {
                    b.label.setText(String.format(b.format, ((Number) result).doubleValue()));
                } else {
                    b.label.setText(String.valueOf(result));
                }
            } catch (Exception e) {
                b.label.setText("Err");
            }
        }

        // Update Overlay
        if (overlay.isVisible()) {
            overlay.updateAndRepaint();
        }
    }

    // Toggle overlay visibility based on preview state
    public void setOverlayVisible(boolean visible) {
        if (overlay == null) {
            overlay = new ui.overlay.DynamicOverlay(parent, groupConfig);
        }
        overlay.setVisible(visible);
    }

    @Override
    protected java.awt.Component createCenterComponent(WebPanel content) {
        return scaler;
    }

    /**
     * A panel that scales its content down to fit its bounds.
     */
    class ZoomPanel extends WebPanel {
        @Override
        public void paint(Graphics g) {
            Graphics2D g2 = (Graphics2D) g;
            Dimension pref = this.getLayout().preferredLayoutSize(this);
            int h = getHeight();

            double scale = 1.0;
            if (pref.height > h && pref.height > 0) {
                scale = (double) h / (double) pref.height;
            }
            if (scale < 0.1)
                scale = 0.1;

            if (scale < 1.0) {
                g2.scale(scale, scale);
            }
            super.paint(g2);
        }
    }
}
