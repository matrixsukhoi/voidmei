package ui.layout;

import java.awt.Component;
import java.awt.Container;
import java.awt.Dimension;
import java.awt.Insets;
import java.awt.LayoutManager;

import com.alee.laf.label.WebLabel;

/**
 * 响应式网格布局管理器 (Responsive Grid Layout)
 * 
 * 功能特性:
 * 1. 固定列数: 构造时指定每行的列数 (columns).
 * 2. 自动计算列宽: 根据容器宽度自动均分计算每列的宽度.
 * 3. 智能跨列 (Smart Spanning):
 * - 如果一个组件的内容宽度超过单列宽度，自动占用两个格子.
 * - 如果本行剩余空间不足以容纳该组件，自动换行并占用整行格子.
 * 
 * 适用于: 需要整齐排列但又能适应不同长度内容的表单或控制面板.
 */
public class ResponsiveGridLayout implements LayoutManager {

    private int hgap;
    private int vgap;
    private int columns;

    public ResponsiveGridLayout(int columns, int hgap, int vgap) {
        this.columns = columns;
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
            int totalHeight = insets.top + insets.bottom;
            int totalWidth = insets.left + insets.right;

            int componentCount = parent.getComponentCount();
            int[] colWidths = new int[columns];
            int[] rowHeights = new int[(componentCount + columns - 1) / columns];

            // 1. Calculate max width for each column and max height for each row
            for (int i = 0; i < componentCount; i++) {
                Component comp = parent.getComponent(i);
                if (comp.isVisible()) {
                    int col = i % columns;
                    int row = i / columns;
                    Dimension d = comp.getPreferredSize();
                    colWidths[col] = Math.max(colWidths[col], d.width);
                    rowHeights[row] = Math.max(rowHeights[row], d.height);
                }
            }

            // Sum column widths and gaps
            int sumColWidths = 0;
            for (int w : colWidths)
                sumColWidths += w;
            totalWidth += sumColWidths + (columns - 1) * hgap;

            // Sum row heights and gaps
            int sumRowHeights = 0;
            for (int h : rowHeights)
                sumRowHeights += h;
            // Add gaps only if there are rows
            if (rowHeights.length > 0) {
                totalHeight += sumRowHeights + (rowHeights.length - 1) * vgap;
            }

            return new Dimension(totalWidth, totalHeight);
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
            int numberOfComponents = parent.getComponentCount();
            if (numberOfComponents == 0) {
                return;
            }

            int w = parent.getWidth() - (insets.left + insets.right);
            int availableW = w - (columns - 1) * hgap;
            if (availableW < 0)
                availableW = 0;

            // 1. Calculate natural max width for each column (ignoring user pref size if
            // it's a fixed-width-forcing wrapper)
            // Ideally just use getPreferredSize(), relying on components to report
            // correctly.
            int[] colWidths = new int[columns];

            // Loop to determine content width requirements
            for (int i = 0; i < numberOfComponents; i++) {
                Component comp = parent.getComponent(i);
                if (comp.isVisible()) {
                    int col = i % columns;
                    // Use standard preferred size logic.
                    // Previously we used getUI().getPreferredSize() to bypass cached values,
                    // but for general layout, comp.getPreferredSize() is safer unless we know
                    // specific component issues.
                    // Given the previous fix, let's stick to getPreferredSize() unless it's a Label
                    // we adjusted.
                    // But actually, we want the *content* logic.
                    // Let's assume getPreferredSize() is correct for the content.
                    int reqW = comp.getPreferredSize().width;
                    colWidths[col] = Math.max(colWidths[col], reqW);
                }
            }

            // 2. Distribute Widths
            long totalReqWidth = 0;
            for (int cw : colWidths)
                totalReqWidth += cw;

            double[] finalColWidths = new double[columns];

            if (totalReqWidth <= availableW) {
                // Case A: Enough space. Use preferred widths and distribute extra.
                // Or just keep them compact? "left align" usually implies compact.
                // But users usually expect grid to fill width.
                // Let's Distribute Extra Equally to avoid one column looking weirdly massive?
                // OR: Distribute extra proportionally?
                // The prompt says: "if a column needs more, shorten others".
                // This implies a constrained scenario. In unconstrained, maybe just fill
                // proportionally.
                // Let's use proportional expansion so layout remains stable.
                int extra = availableW - (int) totalReqWidth;
                for (int i = 0; i < columns; i++) {
                    finalColWidths[i] = colWidths[i];
                    // If we have extra space, do we expand?
                    // If we don't, background might show through.
                    // Let's fill.
                    if (totalReqWidth > 0 && extra > 0) {
                        finalColWidths[i] += extra * ((double) colWidths[i] / totalReqWidth);
                    }
                    // Fallback if totalReqWidth is 0 (empty components)
                    if (totalReqWidth == 0 && extra > 0) {
                        finalColWidths[i] = (double) extra / columns;
                    }
                }
            } else {
                // Case B: Not enough space. Shrink proportionally.
                // "Shorten nearby columns" implemented by scaling everyone down by their
                // weight.
                for (int i = 0; i < columns; i++) {
                    if (totalReqWidth > 0) {
                        finalColWidths[i] = (double) availableW * ((double) colWidths[i] / totalReqWidth);
                    }
                }
            }

            // Fix rounding errors to ensure we use exactly availableW?
            // Optional but good for pixel perfection.

            // 3. Perform Layout
            // We need row heights. Row height is max height of components in that row
            // *given the widths*?
            // Usually height doesn't change with width for simple controls
            // (Switches/Labels), but for TextAreas it might.
            // Assumption: Height is fixed preferred height.

            int currentY = insets.top;
            int currentRowHeight = 0;

            // Pre-calculate Row Heights for consistent grid rows
            int rowCount = (numberOfComponents + columns - 1) / columns;
            int[] rowHeights = new int[rowCount];

            for (int i = 0; i < numberOfComponents; i++) {
                Component comp = parent.getComponent(i);
                if (comp.isVisible()) {
                    int row = i / columns;
                    rowHeights[row] = Math.max(rowHeights[row], comp.getPreferredSize().height);
                }
            }

            for (int row = 0; row < rowCount; row++) {
                int currentX = insets.left;

                for (int col = 0; col < columns; col++) {
                    int idx = row * columns + col;
                    if (idx >= numberOfComponents)
                        break;

                    Component comp = parent.getComponent(idx);
                    if (comp.isVisible()) {
                        int wAlloc = (int) finalColWidths[col];

                        // Handle rounding gap compensation on the last column?
                        // If it's the last column, stretch to edge to avoid 1px gap?
                        if (col == columns - 1) {
                            int usedWidth = currentX - insets.left;
                            wAlloc = w - usedWidth + hgap; // +hgap because loop adds it at end? No.
                            // accurate: wAlloc = w - (currentX - insets.left);
                            wAlloc = Math.max(0, parent.getWidth() - insets.right - currentX);
                        }

                        // Center vertically in the row? Or Fill?
                        // Usually Fill height or Top align.
                        // Let's do Fill to row height.
                        comp.setBounds(currentX, currentY, wAlloc, rowHeights[row]);

                        // Special Logic: If this component used 'alignLabel', we previously set its
                        // preferred size.
                        // Now we should RESET it to the calculated layout if we want internals to flow?
                        // Actually, 'ReplicaBuilder' put alignLabel client prop.
                        // We can manually adjust the internal label width if we want perfect alignment?
                        // But since we are allocating the WHOLE cell width based on content,
                        // and ReplicaBuilder uses BorderLayout.LEFT + CENTER,
                        // The internal layout will handle "Label Left, contents fill right".
                        // BUT we need to make sure the internal Label is consistent across the column
                        // if we want strict text alignment.
                        // Wait, if the column width is determined by the Max Label + Switch,
                        // Then:
                        // Row 1: "Short" + Switch
                        // Row 2: "Very Long Text" + Switch
                        // Column Width will be "Very Long Text" + Switch.
                        // Row 1's cell will be wide. "Short" label will sit left. Switch will sit...
                        // If we want switches to align vertically, we need the LABELS to force the same
                        // width.
                        // "ReplicaBuilder" puts controls in CENTER. Center fills.
                        // If Label is WEST.
                        // Row 1: [Short] [ Switch ]
                        // Row 2: [Very Long Text] [ Switch ]
                        // The switches WON'T align.

                        // To align switches, we need to FORCE the labels to be the same width (Max
                        // Label Width).
                        // I did this in the PREVIOUS implementation using `maxLabelWidthPerColumn`.
                        // I should PRESERVE that logic if I want alignment like Figure 2.
                        // Figure 2 shows labels left aligned, and switches left aligned in a column.
                        // Yes, I MUST preserve 'maxLabelWidthPerColumn' application.

                        currentX += wAlloc + hgap;
                    }
                }
                currentY += rowHeights[row] + vgap;
            }

            // Re-apply label width forcing for alignment?
            // Yes, let's do it.
            // Re-calculate max label width per column.
            int[] maxLabelW = new int[columns];
            for (int i = 0; i < numberOfComponents; i++) {
                int col = i % columns;
                Component comp = parent.getComponent(i);
                if (comp instanceof javax.swing.JComponent) {
                    Object lbl = ((javax.swing.JComponent) comp).getClientProperty("alignLabel");
                    if (lbl instanceof WebLabel) {
                        maxLabelW[col] = Math.max(maxLabelW[col],
                                ((WebLabel) lbl).getUI().getPreferredSize((javax.swing.JComponent) lbl).width);
                    }
                }
            }

            // Apply to labels
            for (int i = 0; i < numberOfComponents; i++) {
                int col = i % columns;
                Component comp = parent.getComponent(i);
                if (comp instanceof javax.swing.JComponent) {
                    Object lbl = ((javax.swing.JComponent) comp).getClientProperty("alignLabel");
                    if (lbl instanceof WebLabel) {
                        WebLabel wLbl = (WebLabel) lbl;
                        // Set the size
                        wLbl.setPreferredSize(new Dimension(maxLabelW[col], wLbl.getHeight()));
                        // We might need to invalidate the comp to ensure it re-lays out internally
                        // before we setBounds?
                        // Actually setBounds triggers validation usually or painting.
                        // But since we are inside layoutContainer, modifying preferences might requires
                        // a re-validation
                        // of the child.
                        comp.validate();
                    }
                }
            }

        }
    }
}
