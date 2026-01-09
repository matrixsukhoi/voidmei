package ui.overlay;

import java.awt.Color;
import java.awt.Font;
import java.awt.Point;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.util.ArrayList;
import java.util.List;

import javax.swing.JWindow;
import javax.swing.SwingUtilities;

import ui.util.ConfigLoader;
import prog.util.FormulaEvaluator;
import parser.blkx;
import ui.mainform;

public class DynamicOverlay extends JWindow {

    private ConfigLoader.GroupConfig config;
    private mainform parent;
    private List<OverlayBinding> bindings = new ArrayList<>();

    // Drag support
    private Point initialClick;

    private class OverlayBinding {
        String label;
        String formula;
        String format;
        String currentValue = ""; // Cache for repaint
        boolean isHeader = false;

        OverlayBinding(String label, String formula, String format, boolean isHeader) {
            this.label = label;
            this.formula = formula;
            this.format = format;
            this.isHeader = isHeader;
        }
    }

    public DynamicOverlay(mainform parent, ConfigLoader.GroupConfig config) {
        this.parent = parent;
        this.config = config;

        // Setup Window
        setAlwaysOnTop(true);
        setBackground(new Color(0, 0, 0, 0)); // Transparent
        setSize(320, 400);
        setLocation(config.x, config.y);

        // Build Bindings
        for (ConfigLoader.RowConfig row : config.rows) {
            boolean isHeader = row.formula != null && row.formula.trim().equalsIgnoreCase("HEADER");
            bindings.add(new OverlayBinding(row.label, row.formula, row.format, isHeader));
        }

        // Mouse Listeners for Dragging
        addMouseListener(new MouseAdapter() {
            public void mousePressed(MouseEvent e) {
                initialClick = e.getPoint();
            }
        });
        addMouseMotionListener(new MouseAdapter() {
            public void mouseDragged(MouseEvent e) {
                int thisX = getLocation().x;
                int thisY = getLocation().y;
                int xMoved = e.getX() - initialClick.x;
                int yMoved = e.getY() - initialClick.y;
                setLocation(thisX + xMoved, thisY + yMoved);
                // Update config for potential save (not implemented yet)
                config.x = getX();
                config.y = getY();
            }
        });
    }

    public void updateAndRepaint() {
        if (!isVisible())
            return;
        if (parent.tc.blkx == null)
            return;

        java.util.Map<String, Object> vars = parent.tc.blkx.getVariableMap();

        for (OverlayBinding b : bindings) {
            if (b.isHeader)
                continue;
            try {
                Object result = FormulaEvaluator.evaluate(b.formula, vars);
                if (result instanceof Number) {
                    b.currentValue = String.format(b.format, ((Number) result).doubleValue());
                } else {
                    b.currentValue = String.valueOf(result);
                }
            } catch (Exception e) {
                b.currentValue = "Err";
            }
        }
        repaint();
    }

    @Override
    public void paint(java.awt.Graphics g) {
        java.awt.Graphics2D g2 = (java.awt.Graphics2D) g;
        g2.setRenderingHint(java.awt.RenderingHints.KEY_TEXT_ANTIALIASING,
                java.awt.RenderingHints.VALUE_TEXT_ANTIALIAS_ON);

        int w = getWidth();
        int h = getHeight();

        // Clear with transparency
        g2.setComposite(java.awt.AlphaComposite.getInstance(java.awt.AlphaComposite.CLEAR));
        g2.fillRect(0, 0, w, h);
        g2.setComposite(java.awt.AlphaComposite.getInstance(java.awt.AlphaComposite.SRC_OVER));

        // Use the configured font
        int screenHeight = java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;
        float scaleFactor = (float) screenHeight / 1440.0f;
        int fontSize = Math.round(16 * scaleFactor);
        Font font = new Font(config.fontName, Font.PLAIN, fontSize);
        g2.setFont(font);

        int y = 0;
        int rowH = fontSize + 8; // Margin top/bottom
        int textYOffset = fontSize + 4;

        // Title Row (if needed, but usually we just draw the items)
        // Match someUsefulData header style: Deep Amber
        g2.setColor(new Color(80, 60, 0, config.alpha));
        g2.fillRect(0, y, w, rowH);
        g2.setColor(Color.WHITE);
        g2.drawString(config.title, 6, y + textYOffset);
        y += rowH;

        int rowIndex = 0;
        for (OverlayBinding b : bindings) {
            if (b.isHeader) {
                g2.setColor(new Color(80, 60, 0, config.alpha));
                g2.fillRect(0, y, w, rowH);
                g2.setColor(new Color(255, 255, 255)); // White for header text
                g2.drawString(b.label, 6, y + textYOffset);
                y += rowH;
            } else {
                // Zebra stripe matching someUsefulData colors
                if (rowIndex % 2 == 0) {
                    g2.setColor(new Color(25, 25, 25, config.alpha));
                } else {
                    g2.setColor(new Color(40, 40, 40, config.alpha));
                }
                g2.fillRect(0, y, w, rowH);

                // Label (Left)
                g2.setColor(Color.WHITE);
                g2.drawString(b.label, 6, y + textYOffset);

                // Value (Right)
                String val = b.currentValue;
                int valWidth = g2.getFontMetrics().stringWidth(val);
                g2.drawString(val, w - valWidth - 6, y + textYOffset);

                y += rowH;
                rowIndex++;
            }
        }

        // Auto-resize height
        if (Math.abs(h - y) > 5) {
            final int newH = y;
            SwingUtilities.invokeLater(() -> setSize(getWidth(), newH));
        }
    }
}
