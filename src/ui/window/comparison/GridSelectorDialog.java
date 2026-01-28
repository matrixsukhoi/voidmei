package ui.window.comparison;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Dimension;
import java.awt.FlowLayout;
import java.io.File;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
import java.util.ArrayList;

import javax.swing.JDialog;
import javax.swing.JFrame;
import javax.swing.JScrollPane;
import javax.swing.SwingUtilities;
import javax.swing.event.DocumentEvent;
import javax.swing.event.DocumentListener;
import javax.swing.border.EmptyBorder;
import javax.swing.JLabel;

import com.alee.laf.button.WebButton;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.text.WebTextField;
import com.alee.laf.tabbedpane.WebTabbedPane;

import prog.Application;
import prog.util.FileUtils;

public class GridSelectorDialog extends JDialog {

    private String selectedPlane = null;
    private WebPanel gridPanel;
    private WebPanel recentPanel;
    private List<String> allPlanes;
    private static List<String> recentPlanes = new ArrayList<>(); // Static to persist per session

    // Heuristic filters
    private String currentFilter = "All";

    public GridSelectorDialog(JFrame owner) {
        super(owner, "Select Aircraft", true);
        initUI();
    }

    public GridSelectorDialog(java.awt.Window owner) {
        super(owner, "Select Aircraft", java.awt.Dialog.ModalityType.APPLICATION_MODAL);
        initUI();
    }

    private void initUI() {
        this.setSize(900, 700);
        this.setLocationRelativeTo(getOwner());
        this.setLayout(new BorderLayout());
        this.getContentPane().setBackground(new Color(25, 25, 25)); // Dark bg

        // 1. Top Section (Search + Tabs)
        WebPanel topContainer = new WebPanel(new BorderLayout());
        topContainer.setOpaque(false);
        topContainer.setBorder(new EmptyBorder(10, 10, 5, 10));

        // Search Bar
        WebPanel searchPanel = new WebPanel(new FlowLayout(FlowLayout.CENTER));
        searchPanel.setOpaque(false);
        WebTextField search = new WebTextField(20);
        search.setInputPrompt("Search aircraft...");
        search.getDocument().addDocumentListener(new DocumentListener() {
            public void changedUpdate(DocumentEvent e) {
                filter(search.getText());
            }

            public void removeUpdate(DocumentEvent e) {
                filter(search.getText());
            }

            public void insertUpdate(DocumentEvent e) {
                filter(search.getText());
            }
        });
        searchPanel.add(search);
        topContainer.add(searchPanel, BorderLayout.NORTH);

        // Filter Tabs
        WebPanel tabsPanel = new WebPanel(new FlowLayout(FlowLayout.CENTER, 10, 0));
        tabsPanel.setOpaque(false);
        tabsPanel.add(createFilterBtn("All"));
        tabsPanel.add(createFilterBtn("WWII")); // Heuristic: No "Jet" or "Afterburner"?
        tabsPanel.add(createFilterBtn("Modern")); // Heuristic: "f-16", "su-27" etc?
        tabsPanel.add(createFilterBtn("Red")); // USSR/China
        tabsPanel.add(createFilterBtn("Blue")); // US/NATO
        topContainer.add(tabsPanel, BorderLayout.SOUTH);

        this.add(topContainer, BorderLayout.NORTH);

        // 2. Main Content (Recent + Grid)
        WebPanel mainContent = new WebPanel(new BorderLayout());
        mainContent.setOpaque(false);
        mainContent.setBorder(new EmptyBorder(5, 10, 10, 10));

        // Recent Section
        if (!recentPlanes.isEmpty()) {
            recentPanel = new WebPanel(new FlowLayout(FlowLayout.LEFT, 10, 10));
            recentPanel.setOpaque(false);
            recentPanel.setBorder(javax.swing.BorderFactory.createTitledBorder(
                    javax.swing.BorderFactory.createMatteBorder(1, 0, 0, 0, Color.GRAY),
                    "Recent",
                    javax.swing.border.TitledBorder.LEFT,
                    javax.swing.border.TitledBorder.TOP,
                    Application.defaultFont,
                    Color.LIGHT_GRAY));

            for (String p : recentPlanes) {
                recentPanel.add(createPlaneButton(p));
            }
            mainContent.add(recentPanel, BorderLayout.NORTH);
        }

        // Grid Area
        // gridPanel = new WebPanel(new WrapFlowLayout(FlowLayout.LEFT, 10, 10));
        gridPanel = new WebPanel(new FlowLayout(FlowLayout.LEFT, 10, 10)); // Fallback
        gridPanel.setOpaque(false);

        JScrollPane scroll = new JScrollPane(gridPanel);
        scroll.setOpaque(false);
        scroll.getViewport().setOpaque(false);
        scroll.setBorder(null);
        scroll.getVerticalScrollBar().setUnitIncrement(16);

        // Wrap scroll in a panel to hold Recent + Grid
        // Actually JScrollPane needs to be the center of mainContent?
        // If we want Recent to scroll WITH grid, we need one big panel.
        // If Recent is sticky, it stays in North.
        // Let's make Recent sticky for now.
        mainContent.add(scroll, BorderLayout.CENTER);

        this.add(mainContent, BorderLayout.CENTER);

        // Load Data
        loadPlanes();
        filter("");
    }

    private WebButton createFilterBtn(String name) {
        WebButton b = new WebButton(name);
        b.setRolloverDecoratedOnly(true);
        b.setDrawFocus(false);
        b.addActionListener(e -> {
            currentFilter = name;
            filter("");
        });
        return b;
    }

    private void loadPlanes() {
        File dir = new File("data/aces/gamedata/flightmodels/fm");
        String[] files = dir.list();
        if (files == null)
            files = new String[0];
        // Strip extensions
        allPlanes = Arrays.stream(files)
                .map(f -> f.replace(".blk", "").replace(".Blkx", ""))
                .sorted()
                .collect(Collectors.toList());
    }

    private void filter(String query) {
        gridPanel.removeAll();
        String q = query.toLowerCase();

        for (String plane : allPlanes) {
            boolean matches = plane.toLowerCase().contains(q);

            // Basic Heuristic Filtering (Mock implementation)
            if (matches) {
                if ("WWII".equals(currentFilter) && (plane.contains("f-16") || plane.contains("su-27")))
                    matches = false;
                if ("Modern".equals(currentFilter)
                        && !(plane.contains("f-16") || plane.contains("su-27") || plane.contains("mig-29")))
                    matches = false;
                // Real filtering would require a database or parsing the FM
            }

            if (matches) {
                gridPanel.add(createPlaneButton(plane));
            }
        }
        gridPanel.revalidate();
        gridPanel.repaint();
    }

    private WebButton createPlaneButton(String plane) {
        WebButton btn = new WebButton(plane);
        btn.setPreferredSize(new Dimension(140, 40));
        btn.addActionListener(e -> {
            selectedPlane = plane;
            // Update Recent
            recentPlanes.remove(plane);
            recentPlanes.add(0, plane);
            if (recentPlanes.size() > 5)
                recentPlanes.remove(5);

            dispose();
        });
        return btn;
    }

    public String getSelectedPlane() {
        return selectedPlane;
    }
}
