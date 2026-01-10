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
        boolean visible = true;

        OverlayBinding(String label, String formula, String format, boolean isHeader, boolean visible) {
            this.label = label;
            this.formula = formula;
            this.format = format;
            this.isHeader = isHeader;
            this.visible = visible;
        }
    }

    public DynamicOverlay(mainform parent, ConfigLoader.GroupConfig config) {
        this.parent = parent;
        this.config = config;
        rebuildBindings();

        // Setup Window
        setAlwaysOnTop(true);
        setBackground(new Color(0, 0, 0, 0)); // Transparent
        setSize(320, 400);
        setLocation(config.x, config.y);

        // Mouse Listeners for Dragging
        addMouseListener(new MouseAdapter() {
            public void mousePressed(MouseEvent e) {
                if (parent != null && !parent.moveCheckFlag)
                    return;
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

    public void rebuildBindings() {
        bindings.clear();
        for (ConfigLoader.RowConfig row : config.rows) {
            boolean isHeader = row.formula != null && row.formula.trim().equalsIgnoreCase("HEADER");
            bindings.add(new OverlayBinding(row.label, row.formula, row.format, isHeader, row.visible));
        }
        repaint();
    }

    public void updateAndRepaint() {
        if (!isVisible())
            return;
        if (parent.tc.blkx == null)
            return;

        java.util.Map<String, Object> vars = parent.tc.blkx.getVariableMap();

        for (OverlayBinding b : bindings) {
            if (!b.visible || b.isHeader)
                continue;
            try {
                Object result = FormulaEvaluator.evaluate(b.formula, vars);
                if (result instanceof Number) {
                    b.currentValue = String.format(b.format, ((Number) result).doubleValue());
                } else if (result instanceof java.util.Map) {
                    // Handle Nashorn ScriptObjectMirror (JS Arrays implement Map)
                    java.util.Map<?, ?> map = (java.util.Map<?, ?>) result;
                    // For arrays, values() returns the elements in order
                    b.currentValue = String.format(b.format, map.values().toArray());
                } else if (result instanceof List) {
                    // Handle List results (e.g. from [a, b, c] syntax in FormulaEvaluator)
                    List<?> list = (List<?>) result;
                    b.currentValue = String.format(b.format, list.toArray());
                } else if (result != null && result.getClass().isArray()) {
                    // Handle raw arrays
                    if (result instanceof Object[]) {
                        b.currentValue = String.format(b.format, (Object[]) result);
                    } else {
                        // Primitive arrays not handled here efficiently, simplistic fallback
                        b.currentValue = String.valueOf(result);
                    }
                } else {
                    b.currentValue = String.valueOf(result);
                }
            } catch (Exception e) {
                b.currentValue = "Err";
            }
        }
        repaint();
    }

    // Cache
    private Font cachedFont;
    private int cachedScreenHeight = -1;
    private float cachedScaleFactor = 1.0f;

    private void checkCache() {
        if (cachedScreenHeight == -1) {
            cachedScreenHeight = java.awt.Toolkit.getDefaultToolkit().getScreenSize().height;
            cachedScaleFactor = (float) cachedScreenHeight / 1440.0f;
        }
        if (cachedFont == null || !cachedFont.getFamily().equals(config.fontName)) {
            int fontSize = Math.round(16 * cachedScaleFactor);
            cachedFont = new Font(config.fontName, Font.PLAIN, fontSize);
        }
    }

    @Override
    public void paint(java.awt.Graphics g) {
        checkCache();

        java.awt.Graphics2D g2 = (java.awt.Graphics2D) g;
        g2.setRenderingHint(java.awt.RenderingHints.KEY_TEXT_ANTIALIASING,
                java.awt.RenderingHints.VALUE_TEXT_ANTIALIAS_ON);

        int w = getWidth();
        int h = getHeight();

        // Clear with transparency (only if configured, or relying on system composite)
        // Note: On JWindow without AWTUtilities, CLEAR might show black.
        // If user says it works, we keep it, but it is expensive if not accelerated.
        g2.setComposite(java.awt.AlphaComposite.getInstance(java.awt.AlphaComposite.CLEAR));
        g2.fillRect(0, 0, w, h);
        g2.setComposite(java.awt.AlphaComposite.getInstance(java.awt.AlphaComposite.SRC_OVER));

        g2.setFont(cachedFont);

        int y = 0;
        int fontSize = cachedFont.getSize();
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
            if (!b.visible)
                continue;
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
