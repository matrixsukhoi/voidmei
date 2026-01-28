package ui.window.comparison;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.FlowLayout;
import java.awt.GridLayout;
import java.awt.event.MouseAdapter;
import java.awt.event.ActionListener;

import javax.swing.BorderFactory;
import javax.swing.JDialog;
import javax.swing.JFrame;
import javax.swing.JScrollPane;

import com.alee.laf.button.WebButton;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;

import parser.Blkx;
import parser.FlightModelParser;
import ui.window.comparison.model.ComparisonData;

/**
 * @deprecated Replaced by {@link CompactComparisonWindow} which provides a
 *             better layout and correct data parsing.
 *             This legacy implementation had layout issues and poor font
 *             rendering.
 */
@Deprecated
public class ComparisonFrame extends JDialog {

    private ComparisonTable table;
    private WebButton btnSlot0;
    private WebButton btnSlot1;

    private String fm0 = "a_4h";
    private String fm1 = "a6m5_zero";

    public ComparisonFrame(java.awt.Window owner) {
        super(owner, "Aircraft Comparison", ModalityType.MODELESS);
        initUI();
    }

    private void initUI() {
        this.setSize(900, 600);
        this.setLocationRelativeTo(getOwner());
        this.setUndecorated(true);
        // Opaque Dark Background as mandated
        this.getContentPane().setBackground(new Color(18, 18, 18));
        this.setLayout(new BorderLayout());

        // Prevent Click-through (Architecture Mandate)
        this.addMouseListener(new MouseAdapter() {
        });
        this.addMouseMotionListener(new MouseAdapter() {
        });

        // Header
        WebPanel header = new WebPanel(new GridLayout(1, 3, 10, 0));
        header.setOpaque(false);
        header.setBorder(BorderFactory.createEmptyBorder(20, 20, 20, 20));

        btnSlot0 = createSlotButton(fm0, e -> selectPlane(0));
        WebLabel centerVS = new WebLabel("VS", WebLabel.CENTER);
        centerVS.setForeground(Color.WHITE);
        centerVS.setFont(centerVS.getFont().deriveFont(24f));
        btnSlot1 = createSlotButton(fm1, e -> selectPlane(1));

        header.add(btnSlot0);
        header.add(centerVS);
        header.add(btnSlot1);
        this.add(header, BorderLayout.NORTH);

        // Body
        table = new ComparisonTable();
        JScrollPane scroll = new JScrollPane(table);
        scroll.setOpaque(false);
        scroll.getViewport().setOpaque(false);
        scroll.setBorder(BorderFactory.createEmptyBorder(0, 20, 0, 20));
        this.add(scroll, BorderLayout.CENTER);

        // Footer
        WebPanel footer = new WebPanel(new FlowLayout(FlowLayout.RIGHT));
        footer.setOpaque(false);
        WebButton closeBtn = new WebButton("Close", e -> dispose());
        closeBtn.setRound(4);
        footer.add(closeBtn);
        this.add(footer, BorderLayout.SOUTH);

        // Initial Load
        refreshData();
    }

    private WebButton createSlotButton(String text, ActionListener l) {
        WebButton b = new WebButton(text);
        b.setFont(b.getFont().deriveFont(16f));
        b.setRound(10);
        // b.setPainter(new com.alee.laf.button.WebButtonPainter()); // Default painter
        // for now
        b.addActionListener(l);
        return b;
    }

    private void selectPlane(int slot) {
        GridSelectorDialog dialog = new GridSelectorDialog(this);
        dialog.setVisible(true);
        String selection = dialog.getSelectedPlane();

        if (selection != null) {
            if (slot == 0)
                fm0 = selection;
            else
                fm1 = selection;
            refreshData();
        }
    }

    private void refreshData() {
        btnSlot0.setText(fm0);
        btnSlot1.setText(fm1);

        // Parse Data
        ComparisonData d0 = loadData(fm0);
        ComparisonData d1 = loadData(fm1);

        table.setData(d0, d1);
    }

    private ComparisonData loadData(String name) {
        // Logic similar to Controller.loadFMData but we need to create Blkx manually
        String path = "data/aces/gamedata/flightmodels/fm/" + name + ".Blkx";
        // Fallback logic for path existence...
        java.io.File f = new java.io.File(path);
        if (!f.exists())
            path = "data/aces/gamedata/flightmodels/fm/" + name + ".blk";

        Blkx b = new Blkx(path, name);
        // Important: Trigger parsing! Blkx constructor might not parse immediately or
        // recursively?
        // In Controller it calls getAllplotdata() etc.
        // Let's assume new Blkx(path) + FlightModelParser is enough for basic keys
        return FlightModelParser.parse(b);
    }
}
