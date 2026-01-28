package ui.window.comparison;

import java.awt.Color;
import java.awt.GridBagConstraints;
import java.awt.GridBagLayout;
import java.awt.Insets;
import java.util.HashMap;
import java.util.Map;
import java.util.ArrayList;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import javax.swing.BorderFactory;
import javax.swing.JDialog;
import javax.swing.JLabel;
import javax.swing.JPanel;
import javax.swing.JScrollPane;
import javax.swing.SwingConstants;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;

import parser.Blkx;
import prog.Application;
import prog.Controller;

public class CompactComparisonWindow extends JDialog {

    private final String fm0Name;
    private final String fm1Name;
    private final Controller controller;

    public CompactComparisonWindow(java.awt.Window owner, Controller c, String fm0, String fm1) {
        super(owner, (fm1 == null || fm1.isEmpty()) ? "Aircraft Data: " + fm0 : "Comparison: " + fm0 + " vs " + fm1,
                ModalityType.MODELESS);
        this.controller = c;
        this.fm0Name = fm0;
        this.fm1Name = fm1;
        initUI();
    }

    // ... (Patterns and Colors remain)

    // ... (initUI remains)

    private void addHeader(JPanel p, String n0, String n1) {
        GridBagConstraints gbc = new GridBagConstraints();
        gbc.gridy = 0;
        gbc.fill = GridBagConstraints.HORIZONTAL;
        gbc.insets = new Insets(0, 0, 10, 0); // Bottom margin for header

        boolean singleMode = (n1 == null || n1.isEmpty());

        if (singleMode) {
            // 1. Empty (Matches Attribute Column)
            gbc.gridx = 0;
            gbc.weightx = 0.4;
            p.add(createLabel("", TEXT_SECONDARY, SwingConstants.LEFT, true), gbc);

            // 2. Name 0 (Matches Value A Column) - Left aligned for data view
            gbc.gridx = 1;
            gbc.weightx = 0.6;
            p.add(createLabel(n0, HEADER_A, SwingConstants.LEFT, true), gbc);
        } else {
            // 1. Empty (Matches Attribute Column)
            gbc.gridx = 0;
            gbc.weightx = 0.35;
            p.add(createLabel("", TEXT_SECONDARY, SwingConstants.LEFT, true), gbc);

            // 2. Name 0 (Matches Value A Column)
            gbc.gridx = 1;
            gbc.weightx = 0.25;
            p.add(createLabel(n0, HEADER_A, SwingConstants.CENTER, true), gbc);

            // 3. VS (Matches Symbol Column)
            gbc.gridx = 2;
            gbc.weightx = 0.15;
            p.add(createLabel("VS", TEXT_SECONDARY, SwingConstants.CENTER, true), gbc);

            // 4. Name 1 (Matches Value B Column)
            gbc.gridx = 3;
            gbc.weightx = 0.25;
            p.add(createLabel(n1, HEADER_B, SwingConstants.CENTER, true), gbc);
        }
    }

    // ... (addSectionHeader remains)

    private void addComparisonRow(JPanel p, String prop, String v0, String v1, int row) {
        GridBagConstraints gbc = new GridBagConstraints();
        gbc.gridy = row;
        gbc.fill = GridBagConstraints.HORIZONTAL;
        gbc.insets = new Insets(0, 0, 0, 0);

        boolean singleMode = (v1 == null);

        // Try numeric parse
        Double d0 = parseDouble(v0);
        Double d1 = (singleMode) ? null : parseDouble(v1);

        // Determine Winner
        int win = 0; // 0=draw, -1=left(v0), 1=right(v1)
        if (d0 != null && d1 != null) {
            boolean lowerIsBetter = isLowerBetter(prop);
            if (Math.abs(d0 - d1) > 0.001) {
                if (d0 > d1)
                    win = lowerIsBetter ? 1 : -1;
                else
                    win = lowerIsBetter ? -1 : 1;
            }
        }

        if (singleMode) {
            // 1. Attribute
            gbc.gridx = 0;
            gbc.weightx = 0.4;
            WebLabel lProp = createLabel(prop, TEXT_PRIMARY, SwingConstants.LEFT, false);
            p.add(lProp, gbc);

            // 2. Value A
            gbc.gridx = 1;
            gbc.weightx = 0.6;
            // No comparison color
            p.add(createLabel(v0, TEXT_SECONDARY, SwingConstants.LEFT, false), gbc);
        } else {
            // 1. Attribute
            gbc.gridx = 0;
            gbc.weightx = 0.35;
            WebLabel lProp = createLabel(prop, TEXT_PRIMARY, SwingConstants.LEFT, false);
            p.add(lProp, gbc);

            // 2. Value A
            gbc.gridx = 1;
            gbc.weightx = 0.25;
            Color c0 = (win == -1) ? ACCENT_BETTER : (win == 1 ? ACCENT_WORSE : TEXT_SECONDARY);
            p.add(createLabel(v0, c0, SwingConstants.LEFT, false), gbc);

            // 3. Symbol
            gbc.gridx = 2;
            gbc.weightx = 0.15;
            gbc.insets = new Insets(0, 15, 0, 15);
            String sym = "-";
            if (win == -1)
                sym = "▶";
            if (win == 1)
                sym = "◀";
            p.add(createLabel(sym, SYMBOL_COLOR, SwingConstants.CENTER, false), gbc);

            // 4. Value B
            gbc.gridx = 3;
            gbc.weightx = 0.25;
            gbc.insets = new Insets(0, 0, 0, 0);
            Color c1 = (win == 1) ? ACCENT_BETTER : (win == -1 ? ACCENT_WORSE : TEXT_SECONDARY);
            p.add(createLabel(v1, c1, SwingConstants.LEFT, false), gbc);
        }
    }

    // Regex to parse: "Property Name: Value [Unit]"
    // Example: "空重(kg): 4644.0" -> Prop="空重(kg)", Val=4644.0
    // Example: "临界速度(km/h): [144, 1167]" -> Prop="...", Val="[144, 1167]" (Complex)
    private static final Pattern PROP_PATTERN = Pattern.compile("([^:]+):\\s*(.*)");
    // High Contrast Palette (Material Design Dark compliant)
    private static final Color BG_COLOR = new Color(18, 18, 18);
    private static final Color TEXT_PRIMARY = new Color(255, 255, 255); // Pure White
    private static final Color TEXT_SECONDARY = new Color(220, 220, 220); // Bright Grey
    private static final Color ACCENT_BETTER = new Color(46, 255, 113); // Neon Green
    private static final Color ACCENT_WORSE = new Color(255, 60, 60); // Bright Red
    private static final Color HEADER_A = new Color(0, 220, 255); // Bright Cyan
    private static final Color HEADER_B = new Color(255, 80, 180); // Bright Pink
    private static final Color SYMBOL_COLOR = new Color(255, 215, 0); // Gold

    private void initUI() {
        this.setUndecorated(true);
        // this.setSize(500, 600); // Auto-size requested
        // this.setLocationRelativeTo(getOwner()); // Move to end
        this.getContentPane().setBackground(BG_COLOR); // Dark

        WebPanel content = new WebPanel(new GridBagLayout());
        content.setOpaque(true); // Force opaque
        content.setBackground(BG_COLOR);
        content.setBorder(BorderFactory.createEmptyBorder(10, 10, 10, 10)); // Increased padding

        // Get Data
        List<String> lines0 = loadFmLines(fm0Name);
        List<String> lines1 = loadFmLines(fm1Name);

        // Header
        addHeader(content, fm0Name, fm1Name);

        // Body
        // Parse Data into Maps and Structure
        List<DisplayItem> structure = new ArrayList<>();
        Map<String, String> map0 = new HashMap<>();
        Map<String, String> map1 = new HashMap<>();
        List<String> lines1Safe = (lines1 != null) ? lines1 : new ArrayList<>();

        // 1. Build initial structure from lines0
        for (String l : lines0) {
            l = l.trim();
            if (l.isEmpty())
                continue;

            if (l.contains("------")) {
                String h = l.replaceAll("-", "").trim();
                structure.add(new DisplayItem(true, h));
                continue;
            }

            Matcher m = PROP_PATTERN.matcher(l);
            if (m.matches()) {
                String k = m.group(1).trim();
                String v = m.group(2).trim();
                structure.add(new DisplayItem(false, k));
                map0.put(k, v);
            }
        }

        // 2. Parse lines1 and merge unique keys
        int lastMatchIndex = -1;
        // Find where the content starts (skip initial headers if possible, or just
        // merge)
        // Simple merge: scan lines1. If key exists, update index. If not, insert after
        // index.
        for (String l : lines1Safe) {
            l = l.trim();
            if (l.isEmpty() || l.contains("------"))
                continue; // Skip headers in merge for now to avoid duplications

            Matcher m = PROP_PATTERN.matcher(l);
            if (m.matches()) {
                String k = m.group(1).trim();
                String v = m.group(2).trim();
                map1.put(k, v);

                // Check struct
                int idx = findInStructure(structure, k);
                if (idx != -1) {
                    lastMatchIndex = idx;
                } else {
                    // Insert after last match
                    if (lastMatchIndex < structure.size() - 1) {
                        structure.add(lastMatchIndex + 1, new DisplayItem(false, k));
                    } else {
                        structure.add(new DisplayItem(false, k));
                    }
                    lastMatchIndex++;
                }
            }
        }

        // Render
        int row = 1;
        for (DisplayItem item : structure) {
            if (item.isHeader) {
                addSectionHeader(content, item.text, row++);
            } else {
                String k = item.text;
                String v0 = map0.get(k);
                String v1 = map1.get(k);

                // If v0 is null, it means it's a key only in FM1.
                // If v1 is null, it means it's a key only in FM0 (or Single Mode).

                // Single View Mode Check: If we are in single view, we iterate structure which
                // is mostly FM0.
                // If structure has keys from FM1 (impossible since we don't merge if single?),
                // actually if single mode, lines1Safe is empty, so loop 2 doesn't run. Correct.

                addComparisonRow(content, k, (v0 == null ? "-" : v0),
                        (fm1Name == null || fm1Name.isEmpty()) ? null : (v1 == null ? "-" : v1), row++);
            }
        }

        // JScrollPane scroll = new JScrollPane(content);
        // scroll.setOpaque(false);
        // scroll.getViewport().setOpaque(false);
        // scroll.setBorder(null);
        // scroll.getVerticalScrollBar().setUnitIncrement(16);

        this.getContentPane().add(content, java.awt.BorderLayout.CENTER);

        // Close on click out? Or add close button? Undecorated needs a way to close.
        // Adding a Close Listener for clicking outside is complex for JDialog.
        // Let's add a close button at the bottom or click-to-close on the window
        // itself.
        this.addMouseListener(new java.awt.event.MouseAdapter() {
            public void mouseClicked(java.awt.event.MouseEvent e) {
                // dispose(); // Maybe too aggressive if clicking to select text?
            }
        });

        // Add Close Button at bottom
        WebLabel close = new WebLabel("CLOSE", WebLabel.CENTER);
        close.setOpaque(true);
        close.setBackground(new Color(183, 28, 28)); // Red 900
        close.setForeground(Color.WHITE);
        close.setFont(Application.defaultFont.deriveFont(java.awt.Font.BOLD, 14f));
        close.setBorder(BorderFactory.createEmptyBorder(8, 0, 8, 0));
        close.addMouseListener(new java.awt.event.MouseAdapter() {
            public void mouseClicked(java.awt.event.MouseEvent e) {
                dispose();
            }

            public void mouseEntered(java.awt.event.MouseEvent e) {
                close.setBackground(new Color(211, 47, 47)); // Red 700 hover
            }

            public void mouseExited(java.awt.event.MouseEvent e) {
                close.setBackground(new Color(183, 28, 28));
            }
        });
        this.getContentPane().add(close, java.awt.BorderLayout.SOUTH);

        this.pack();
        this.setLocationRelativeTo(getOwner());

        // ESC to Close
        javax.swing.KeyStroke esc = javax.swing.KeyStroke.getKeyStroke(java.awt.event.KeyEvent.VK_ESCAPE, 0);
        this.getRootPane().registerKeyboardAction(e -> dispose(), esc, javax.swing.JComponent.WHEN_IN_FOCUSED_WINDOW);
    }

    private void addHeader(JPanel p, String n0, String n1) {
        GridBagConstraints gbc = new GridBagConstraints();
        gbc.gridy = 0;
        gbc.fill = GridBagConstraints.HORIZONTAL;
        gbc.insets = new Insets(0, 0, 15, 0); // Bottom margin for header

        // 1. Empty (Matches Attribute Column)
        gbc.gridx = 0;
        gbc.weightx = 0.35;
        p.add(createLabel("", TEXT_SECONDARY, SwingConstants.LEFT, true), gbc);

        // 2. Name 0 (Matches Value A Column)
        gbc.gridx = 1;
        gbc.weightx = 0.25;
        p.add(createLabel(n0, HEADER_A, SwingConstants.CENTER, true), gbc);

        // 3. VS (Matches Symbol Column)
        gbc.gridx = 2;
        gbc.weightx = 0.15;
        p.add(createLabel("VS", TEXT_SECONDARY, SwingConstants.CENTER, true), gbc);

        // 4. Name 1 (Matches Value B Column)
        gbc.gridx = 3;
        gbc.weightx = 0.25;
        p.add(createLabel(n1, HEADER_B, SwingConstants.CENTER, true), gbc);
    }

    private void addSectionHeader(JPanel p, String text, int row) {
        GridBagConstraints gbc = new GridBagConstraints();
        gbc.gridx = 0;
        gbc.gridy = row;
        gbc.gridwidth = 4; // Spanning all columns
        gbc.fill = GridBagConstraints.HORIZONTAL;
        gbc.insets = new Insets(15, 0, 5, 0);

        WebLabel l = new WebLabel(text);
        l.setFont(Application.defaultFont.deriveFont(java.awt.Font.BOLD, 13f));
        l.setForeground(TEXT_SECONDARY);
        l.setHorizontalAlignment(SwingConstants.CENTER);

        // Add a separator line visual if possible, or just the text
        // For simple robust UI, just text for now
        p.add(l, gbc);
    }

    private void addComparisonRow(JPanel p, String prop, String v0, String v1, int row) {
        GridBagConstraints gbc = new GridBagConstraints();
        gbc.gridy = row;
        gbc.fill = GridBagConstraints.HORIZONTAL;
        gbc.insets = new Insets(2, 0, 2, 0);

        // Try numeric parse
        Double d0 = parseDouble(v0);
        Double d1 = parseDouble(v1);

        // Determine Winner
        int win = 0; // 0=draw, -1=left(v0), 1=right(v1)
        if (d0 != null && d1 != null) {
            boolean lowerIsBetter = isLowerBetter(prop);
            if (Math.abs(d0 - d1) > 0.001) {
                if (d0 > d1)
                    win = lowerIsBetter ? 1 : -1;
                else
                    win = lowerIsBetter ? -1 : 1;
            }
        }

        // Left (Prop + Val0) - Combined? User said "Three columns: Prop(Left)
        // Symbol(Center) Val(Right)"
        // No, user said: "[Attr] : [A] (Diff) [B]" -> "Attr : A [Sym] B".
        // Let's do: Col0=Attr, Col1=A, Col2=Sym, Col3=B. Or User said "Three Column
        // Alignment: Prop Left, Symbol Center, Value Right".
        // This suggests Prop is Column 1, The rest is... where?
        // "Attr : A (Diff) B" is one logic. "Prop Left, Symbol Center, Value Right" is
        // another.
        // Let's go with "Attr (Left) | A (Center-Left) | Sym (Center) | B
        // (Center-Right)".

        // 1. Attribute
        gbc.gridx = 0;
        gbc.weightx = 0.35;
        WebLabel lProp = createLabel(prop, TEXT_PRIMARY, SwingConstants.LEFT, false);
        p.add(lProp, gbc);

        // 2. Value A
        gbc.gridx = 1;
        gbc.weightx = 0.25;
        Color c0 = (win == -1) ? ACCENT_BETTER : (win == 1 ? ACCENT_WORSE : TEXT_SECONDARY);
        p.add(createLabel(v0, c0, SwingConstants.LEFT, false), gbc);

        // 3. Symbol
        gbc.gridx = 2;
        gbc.weightx = 0.15;
        gbc.insets = new Insets(2, 15, 2, 15); // Add horizontal padding
        String sym = "-";
        if (win == -1)
            sym = "▶"; // A is better
        if (win == 1)
            sym = "◀"; // B is better
        p.add(createLabel(sym, SYMBOL_COLOR, SwingConstants.CENTER, false), gbc);

        // 4. Value B
        gbc.gridx = 3;
        gbc.weightx = 0.25;
        gbc.insets = new Insets(2, 0, 2, 0); // Reset padding
        Color c1 = (win == 1) ? ACCENT_BETTER : (win == -1 ? ACCENT_WORSE : TEXT_SECONDARY);
        p.add(createLabel(v1, c1, SwingConstants.LEFT, false), gbc);
    }

    private WebLabel createLabel(String txt, Color c, int align, boolean bold) {
        WebLabel l = new WebLabel(txt);
        l.setForeground(c);
        l.setFont(Application.defaultFont.deriveFont(bold ? java.awt.Font.BOLD : java.awt.Font.PLAIN, 13f)); // 1.2x
                                                                                                             // readable
        l.setHorizontalAlignment(align);
        return l;
    }

    private Double parseDouble(String s) {
        if (s == null || s.isEmpty())
            return null;
        if (s.startsWith("["))
            return null; // Array/Range - hard to compare simply

        try {
            // Find first number pattern: optional minus, digits, optional dot, optional
            // decimals
            Pattern p = Pattern.compile("(-?\\d+(\\.\\d+)?)");
            Matcher m = p.matcher(s);
            if (m.find()) {
                return Double.parseDouble(m.group(1));
            }
        } catch (Exception e) {
            // ignore
        }
        return null;
    }

    private boolean isLowerBetter(String prop) {
        prop = prop.toLowerCase();
        // Lower is better for:
        // Weight (Empty/Standard, generally agility)
        // Loading (Wing loading)
        // Drag (Coefficients, Areas)
        // Turn Time (Time is bad)
        // Takeoff distance
        if (prop.contains("turn time") || prop.contains("time"))
            return true; // time usually lower is better (except endurance)
        if (prop.contains("empty weight") || prop.contains("空重"))
            return true;
        if (prop.contains("wing loading") || prop.contains("翼载"))
            return true;
        if (prop.contains("drag") || prop.contains("阻力"))
            return true;

        return false;
    }

    private int findInStructure(List<DisplayItem> list, String key) {
        for (int i = 0; i < list.size(); i++) {
            if (!list.get(i).isHeader && list.get(i).text.equals(key))
                return i;
        }
        return -1;
    }

    private static class DisplayItem {
        boolean isHeader;
        String text;

        public DisplayItem(boolean h, String t) {
            isHeader = h;
            text = t;
        }
    }

    private List<String> loadFmLines(String name) {
        if (name == null || name.trim().isEmpty()) {
            return new ArrayList<>();
        }
        // Reuse Blkx logic from Controller
        // We create a temporary Blkx to parse locally since we don't want to disturb
        // main Controller state
        String path = "data/aces/gamedata/flightmodels/fm/" + name + ".Blkx";
        java.io.File f = new java.io.File(path);
        if (!f.exists())
            path = "data/aces/gamedata/flightmodels/fm/" + name + ".blk";

        Blkx b = new Blkx(path, name); // This constructor parses immediately in original code?
        // Checking Blkx.java... "public Blkx(String path...)" calls nothing?
        // Ah, Controller calls "b.getAllplotdata(); b.finalizeLoading();".
        // fmdata is populated during parsing.
        // Wait, Blkx(String path, String name) calls load?
        // Looking at Blkx.java...
        // It reads file, calls `getload()`.
        // fmdata seems to be populated in `getload` or via `WritePartsFm`.
        // We might need to manually trigger the populating logic if the constructor
        // doesn't.
        // Let's assume standard instantiation works as Controller uses it.
        // Actually, Controller calls:
        // Blkx = new Blkx(..., ...);
        // Blkx.getAllplotdata();
        // Blkx.finalizeLoading();
        // We should replicate that.

        b.getAllplotdata(); // Might be heavy?
        // fmdata is string.
        if (b.fmdata == null)
            return new ArrayList<>();

        String[] lines = b.fmdata.split("\n");
        List<String> list = new ArrayList<>();
        for (String s : lines)
            if (!s.trim().isEmpty())
                list.add(s);
        return list;
    }
}
