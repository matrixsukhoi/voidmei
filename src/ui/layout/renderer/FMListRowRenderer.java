package ui.layout.renderer;

import java.awt.BorderLayout;
import java.awt.Font;
import java.awt.Window;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;
import java.io.File;

import javax.swing.SwingUtilities;

import com.alee.extended.layout.VerticalFlowLayout;
import com.alee.extended.window.WebPopOver;
import com.alee.laf.button.WebButton;
import com.alee.laf.combobox.WebComboBox;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.text.WebTextArea;

import parser.Blkx;
import prog.Application;
import prog.config.ConfigLoader.GroupConfig;
import prog.config.ConfigLoader.RowConfig;
import prog.i18n.Lang;
import prog.util.FileUtils;
import ui.replica.ReplicaBuilder;

public class FMListRowRenderer implements RowRenderer {

    // Drag state for popover
    private int isDragging;
    private int xx;
    private int yy;

    @Override
    public WebPanel render(RowConfig row, GroupConfig groupConfig, RenderContext context) {
        // Parse directory path (standard FM path or from config)
        String dirPath = "data/aces/gamedata/flightmodels/fm";
        // If format is provided, use it? original code hardcodes it, but let's be
        // flexible if cfg provides it
        if (row.format != null && !row.format.isEmpty()) {
            dirPath = row.format;
        }

        // Get file list
        File dir = new File(dirPath);
        String[] files = dir.list();
        if (files == null)
            files = new String[0];
        files = FileUtils.getFilelistNameNoEx(files);

        // Get current value
        String currentVal = context.getStringFromConfigService(row.property,
                row.property.contains("0") ? files.length > 0 ? files[0] : "" : files.length > 0 ? files[0] : "");

        WebPanel panel = new WebPanel(new BorderLayout(5, 0));
        ReplicaBuilder.getStyle().decorateControlPanel(panel);
        panel.setBorder(javax.swing.BorderFactory.createEmptyBorder(4, 5, 4, 5));

        WebLabel label = new WebLabel(row.label);
        if ((row.desc != null && !row.desc.isEmpty()) || (row.descImg != null && !row.descImg.isEmpty())) {
            ReplicaBuilder.applyStylizedTooltip(label, row.desc, row.descImg);
        }
        ReplicaBuilder.getStyle().decorateLabel(label);
        panel.add(label, BorderLayout.WEST);

        WebComboBox combo = new WebComboBox(files);
        combo.setEditable(false);
        // combo.setPreferredSize(new Dimension(150, 26));
        // Styling
        combo.setWebColoredBackground(false);
        combo.setShadeWidth(1);
        combo.setDrawFocus(false);
        combo.setFont(Application.defaultFont);
        combo.setExpandedBgColor(new java.awt.Color(0, 0, 0, 0));
        combo.setBackground(new java.awt.Color(0, 0, 0, 0));

        if (currentVal != null && !currentVal.isEmpty()) {
            combo.setSelectedItem(currentVal);
        }

        combo.addActionListener(e -> {
            if (context.isUpdating())
                return;
            Object selected = combo.getSelectedItem();
            if (selected != null) {
                context.syncStringToConfigService(row.property, selected.toString());
                context.onSave();
            }
        });

        panel.add(combo, BorderLayout.CENTER);

        // Add "View" Button
        WebButton viewBtn = new WebButton("View"); // Lang.mView? Using hardcoded for now or Lang if available
        // LoggingPanel used native interaction, here we add a button
        viewBtn.setMargin(0, 5, 0, 5);
        viewBtn.addActionListener(e -> {
            // Launch new Comparison UI
            Window parentWindow = SwingUtilities.getWindowAncestor(panel);
            Object selected = combo.getSelectedItem();
            String fmName = (selected != null) ? selected.toString() : "a_4h";

            // Use Application.ctr if available, or just null if CompactComparisonWindow
            // doesn't strictly need it for basic view
            // CompactComparisonWindow constructor matches: (Window, Controller, String,
            // String)
            ui.window.comparison.CompactComparisonWindow cf = new ui.window.comparison.CompactComparisonWindow(
                    parentWindow,
                    Application.ctr,
                    fmName,
                    null // Single view mode
            );
            cf.setVisible(true);
        });
        panel.add(viewBtn, BorderLayout.EAST);

        // Critical: Enable ResponsiveGrid alignment
        panel.putClientProperty("alignLabel", label);

        return panel;
    }

    private void displayFM(WebPanel source, String planeName) {
        String path = "data/aces/gamedata/flightmodels/fm/" + planeName + ".Blkx";
        Blkx fmblk = new Blkx(path, planeName);

        Window parentWindow = SwingUtilities.getWindowAncestor(source);

        WebPopOver popOver = new WebPopOver(parentWindow);
        popOver.setMargin(5);
        popOver.setLayout(new VerticalFlowLayout());

        WebButton closeButton = new WebButton(Lang.mCancel, e -> popOver.dispose());
        closeButton.setUndecorated(true);
        closeButton.setFont(Application.defaultFont);
        closeButton.setFontSize((int) (Application.defaultFontsize * 1.5f));
        closeButton.setFontStyle(Font.BOLD);

        WebTextArea textArea = new WebTextArea(fmblk.fmdata);
        popOver.add(textArea);
        popOver.setFont(Application.defaultFont);
        textArea.setFont(Application.defaultFont);
        textArea.setFontSize((int) (Application.defaultFontsize * 1.2f));
        popOver.add(closeButton);

        popOver.show(source); // Show relative to source panel

        // Drag logic
        textArea.addMouseListener(new MouseAdapter() {
            public void mousePressed(MouseEvent e) {
                isDragging = 1;
                xx = e.getX();
                yy = e.getY();
            }

            public void mouseReleased(MouseEvent e) {
                if (isDragging == 1)
                    isDragging = 0;
            }
        });
        textArea.addMouseMotionListener(new MouseMotionAdapter() {
            public void mouseDragged(MouseEvent e) {
                int left = popOver.getLocation().x;
                int top = popOver.getLocation().y;
                popOver.setLocation(left + e.getX() - xx, top + e.getY() - yy);
            }
        });
    }

}
