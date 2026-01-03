package ui.layout;

import java.awt.Component;
import java.awt.Container;
import java.awt.Dimension;
import java.awt.Insets;
import java.awt.LayoutManager;

/**
 * A flexible grid layout that automatically wraps components to the next row
 * when there is no horizontal space left.
 * Similar to CSS Flexbox with flex-wrap: wrap.
 */
public class FlexGridLayout implements LayoutManager {

    private int hgap;
    private int vgap;

    public FlexGridLayout() {
        this(5, 5);
    }

    public FlexGridLayout(int hgap, int vgap) {
        this.hgap = hgap;
        this.vgap = vgap;
    }

    @Override
    public void addLayoutComponent(String name, Component comp) {
    }

    @Override
    public void removeLayoutComponent(Component comp) {
    }

    @Override
    public Dimension preferredLayoutSize(Container parent) {
        synchronized (parent.getTreeLock()) {
            Insets insets = parent.getInsets();
            int maxWidth = parent.getWidth() - (insets.left + insets.right);
            if (maxWidth <= 0) {
                // Fallback if container has no width yet
                maxWidth = Integer.MAX_VALUE;
            }

            int x = 0;
            int y = 0;
            int rowHeight = 0;
            int maxRowWidth = 0;

            for (Component comp : parent.getComponents()) {
                if (comp.isVisible()) {
                    Dimension d = comp.getPreferredSize();
                    if (x + d.width > maxWidth && x > 0) {
                        x = 0;
                        y += rowHeight + vgap;
                        rowHeight = 0;
                    }
                    rowHeight = Math.max(rowHeight, d.height);
                    x += d.width + hgap;
                    maxRowWidth = Math.max(maxRowWidth, x);
                }
            }

            return new Dimension(maxRowWidth + insets.left + insets.right, y + rowHeight + insets.top + insets.bottom);
        }
    }

    @Override
    public Dimension minimumLayoutSize(Container parent) {
        return preferredLayoutSize(parent);
    }

    @Override
    public void layoutContainer(Container parent) {
        synchronized (parent.getTreeLock()) {
            Insets insets = parent.getInsets();
            int maxWidth = parent.getWidth() - (insets.left + insets.right);
            int x = insets.left;
            int y = insets.top;
            int rowHeight = 0;

            for (Component comp : parent.getComponents()) {
                if (comp.isVisible()) {
                    Dimension d = comp.getPreferredSize();
                    // Wrap to next line if it doesn't fit
                    if (x + d.width > maxWidth + insets.left && x > insets.left) {
                        x = insets.left;
                        y += rowHeight + vgap;
                        rowHeight = 0;
                    }
                    comp.setBounds(x, y, d.width, d.height);
                    x += d.width + hgap;
                    rowHeight = Math.max(rowHeight, d.height);
                }
            }
        }
    }
}
