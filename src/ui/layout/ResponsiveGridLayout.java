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
        // 使用模拟布局来精确计算所需高度
        synchronized (parent.getTreeLock()) {
            Insets insets = parent.getInsets();
            int w = parent.getWidth();

            // 修复: 初始宽度为0时，不要返回过大的宽度(如1000)，否则会导致水平滚动条出现。
            // 返回一个较小的值允许父容器正常重新布局。
            if (w <= 0)
                w = 10;

            int availW = w - (insets.left + insets.right);
            // 避免除以0或负宽
            if (availW < 1)
                availW = 1;

            int compWidth = (availW - (columns - 1) * hgap) / columns;
            if (compWidth < 1)
                compWidth = 1;

            int currentRowHeight = 0;
            int colIndex = 0;
            int currentY = insets.top; // 当前绘制坐标 Y

            for (Component comp : parent.getComponents()) {
                if (comp.isVisible()) {
                    Dimension d = comp.getPreferredSize();

                    int span = 1;
                    if (d.width > compWidth)
                        span = 2;

                    if (span > (columns - colIndex)) {
                        // 换行
                        currentY += currentRowHeight + vgap;
                        currentRowHeight = 0;
                        colIndex = 0;
                        span = columns;
                    }

                    currentRowHeight = Math.max(currentRowHeight, d.height);
                    colIndex += span;

                    if (colIndex >= columns) {
                        colIndex = 0;
                        currentY += currentRowHeight + vgap;
                        currentRowHeight = 0;
                    }
                }
            }

            int totalHeight = currentY + currentRowHeight + insets.bottom;
            return new Dimension(w, totalHeight);
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
            int compWidth = (w - (columns - 1) * hgap) / columns;
            if (compWidth < 1)
                compWidth = 1;

            // 第一遍: 计算每列的最大标签宽度 (First pass: calculate max label width per column)
            int[] maxLabelWidthPerColumn = new int[columns];
            int colIndex = 0;

            for (int i = 0; i < numberOfComponents; i++) {
                Component comp = parent.getComponent(i);
                if (comp.isVisible()) {
                    Dimension d = comp.getPreferredSize();

                    int span = 1;
                    if (d.width > compWidth)
                        span = 2;

                    if (span > (columns - colIndex)) {
                        colIndex = 0;
                        span = columns;
                    }

                    // 只对单列组件计算标签宽度 (Only calculate label width for single-column items)
                    if (span == 1) {
                        Object labelObj = comp instanceof javax.swing.JComponent
                                ? ((javax.swing.JComponent) comp).getClientProperty("alignLabel")
                                : null;
                        if (labelObj instanceof WebLabel) {
                            WebLabel label = (WebLabel) labelObj;
                            int labelWidth = label.getPreferredSize().width;
                            maxLabelWidthPerColumn[colIndex] = Math.max(
                                    maxLabelWidthPerColumn[colIndex],
                                    labelWidth);
                        }
                    }

                    colIndex += span;
                    if (colIndex >= columns) {
                        colIndex = 0;
                    }
                }
            }

            // 第二遍: 应用对齐并执行实际布局 (Second pass: apply alignment and do actual layout)
            int x = insets.left;
            int y = insets.top;
            int currentRowHeight = 0;
            colIndex = 0;

            for (int i = 0; i < numberOfComponents; i++) {
                Component comp = parent.getComponent(i);
                if (comp.isVisible()) {
                    Dimension d = comp.getPreferredSize();

                    int span = 1;
                    if (d.width > compWidth)
                        span = 2;

                    if (span > (columns - colIndex)) {
                        colIndex = 0;
                        x = insets.left;
                        y += currentRowHeight + vgap;
                        currentRowHeight = 0;
                        span = columns;
                    }

                    // 应用该列的最大标签宽度 (Apply this column's max label width)
                    if (span == 1 && maxLabelWidthPerColumn[colIndex] > 0) {
                        Object labelObj = comp instanceof javax.swing.JComponent
                                ? ((javax.swing.JComponent) comp).getClientProperty("alignLabel")
                                : null;
                        if (labelObj instanceof WebLabel) {
                            WebLabel label = (WebLabel) labelObj;
                            Dimension labelSize = label.getPreferredSize();
                            label.setPreferredSize(new Dimension(
                                    maxLabelWidthPerColumn[colIndex],
                                    labelSize.height));
                            // 强制重新布局以应用新宽度 (Force relayout to apply new width)
                            if (comp instanceof Container) {
                                ((Container) comp).invalidate();
                                ((Container) comp).validate();
                            }
                        }
                    }

                    int actualWidth = span * compWidth + (span - 1) * hgap;
                    comp.setBounds(x, y, actualWidth, d.height);

                    currentRowHeight = Math.max(currentRowHeight, d.height);

                    x += actualWidth + hgap;
                    colIndex += span;

                    if (colIndex >= columns) {
                        colIndex = 0;
                        x = insets.left;
                        y += currentRowHeight + vgap;
                        currentRowHeight = 0;
                    }
                }
            }
        }
    }
}
