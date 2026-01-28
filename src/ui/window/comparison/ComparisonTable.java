package ui.window.comparison;

import java.awt.Color;
import java.awt.Component;
import java.awt.Font;
import javax.swing.JTable;
import javax.swing.table.AbstractTableModel;
import javax.swing.table.DefaultTableCellRenderer;
import javax.swing.table.TableCellRenderer;

import prog.Application;
import ui.window.comparison.logic.ComparisonCalculator;
import ui.window.comparison.logic.ComparisonCalculator.DiffResult;
import ui.window.comparison.logic.ComparisonCalculator.WinState;
import ui.window.comparison.model.ComparisonData;

/**
 * @deprecated Replaced by lightweight parsing in
 *             {@link CompactComparisonWindow}.
 */
@Deprecated
public class ComparisonTable extends JTable {

    public ComparisonTable() {
        super();
        this.setOpaque(false);
        this.setBackground(new Color(0, 0, 0, 0));
        this.setShowGrid(false);
        this.setIntercellSpacing(new java.awt.Dimension(0, 0));
        this.setRowHeight(30); // Compact but readable
        this.setDefaultRenderer(Object.class, new ComparisonCellRenderer());
    }

    public void setData(ComparisonData d0, ComparisonData d1) {
        this.setModel(new ComparisonTableModel(d0, d1));
        // Column sizing
        this.getColumnModel().getColumn(0).setPreferredWidth(100); // Val 0
        this.getColumnModel().getColumn(1).setPreferredWidth(120); // Title
        this.getColumnModel().getColumn(2).setPreferredWidth(100); // Val 1
    }

    private static class ComparisonTableModel extends AbstractTableModel {
        private final ComparisonData d0;
        private final ComparisonData d1;
        private final String[] rows = {
                "Empty Weight", "Max T/O Weight",
                "Max Speed (SL)", "Max Speed (Best)", "Vne",
                "Max Climb (SL)", "Turn Time", "Roll Rate",
                "Wing Loading", "Thrust/Weight", "Max G"
        };
        // Define "Lower is Better" for specific rows
        private final boolean[] lowerIsBetter = {
                true, false,
                false, false, false,
                false, true, false,
                true, false, false
        };

        public ComparisonTableModel(ComparisonData d0, ComparisonData d1) {
            this.d0 = d0;
            this.d1 = d1;
        }

        @Override
        public int getRowCount() {
            return rows.length;
        }

        @Override
        public int getColumnCount() {
            return 3; // Val0, Title, Val1
        }

        @Override
        public Object getValueAt(int rowIndex, int columnIndex) {
            if (d0 == null || d1 == null)
                return "-";

            // Map row to data value
            double v0 = getVal(d0, rowIndex);
            double v1 = getVal(d1, rowIndex);

            if (columnIndex == 1)
                return rows[rowIndex]; // Title

            // Return a wrapper object containing value + comparison result
            boolean lib = lowerIsBetter[rowIndex];
            DiffResult res = ComparisonCalculator.compare(v0, v1, !lib);

            if (columnIndex == 0)
                return new CellData(v0, res.win == WinState.LOSS ? WinState.WIN : WinState.LOSS); // Invert for left
                                                                                                  // side? No, simpler:
                                                                                                  // Val0 vs Val1. If
                                                                                                  // Val1 wins, Val0
                                                                                                  // loses.
            // Wait, ComparisonCalculator returns result relative to Val0 -> Val1
            // transition?
            // compare(v0, v1) -> diff = v1 - v0.
            // if diff > 0 and higherBetter -> WIN (Val1 is better).
            // So if res.win == WIN, Val1 is winner.

            if (columnIndex == 0) {
                // If Val1 is WINner, Val0 is LOSER.
                // If Val1 is LOSER, Val0 is WINNER.
                WinState s = WinState.DRAW;
                if (res.win == WinState.WIN)
                    s = WinState.LOSS;
                if (res.win == WinState.LOSS)
                    s = WinState.WIN;
                return new CellData(v0, s);
            }

            if (columnIndex == 2)
                return new CellData(v1, res.win);

            return "";
        }

        private double getVal(ComparisonData d, int r) {
            switch (r) {
                case 0:
                    return d.emptyWeight;
                case 1:
                    return d.maxTakeoffWeight;
                case 2:
                    return d.maxSpeedSeaLevel;
                case 3:
                    return d.maxSpeedBest;
                case 4:
                    return d.vne;
                case 5:
                    return d.maxClimbSeaLevel;
                case 6:
                    return d.turnTime;
                case 7:
                    return d.maxRollRate;
                case 8:
                    return d.wingLoading;
                case 9:
                    return d.thrustToWeight;
                case 10:
                    return d.maxG;
                default:
                    return 0;
            }
        }
    }

    public static class CellData {
        public double val;
        public WinState state;

        public CellData(double v, WinState s) {
            this.val = v;
            this.state = s;
        }

        public String toString() {
            return String.format("%.1f", val);
        }
    }

    private static class ComparisonCellRenderer extends DefaultTableCellRenderer {
        private static final Color BG_ODD = new Color(30, 30, 30, 255);
        private static final Color BG_EVEN = new Color(18, 18, 18, 255);
        private static final Color COL_WIN = new Color(0, 255, 65);
        private static final Color COL_LOSS = new Color(176, 176, 176); // Dimmed

        @Override
        public Component getTableCellRendererComponent(JTable table, Object value, boolean isSelected, boolean hasFocus,
                int row, int column) {
            Component c = super.getTableCellRendererComponent(table, value, isSelected, hasFocus, row, column);

            // Zebra Striping
            if (row % 2 == 0) {
                c.setBackground(BG_EVEN);
            } else {
                c.setBackground(BG_ODD);
            }

            // Text Styling
            this.setHorizontalAlignment(column == 1 ? javax.swing.SwingConstants.CENTER
                    : (column == 0 ? javax.swing.SwingConstants.RIGHT : javax.swing.SwingConstants.LEFT));
            this.setFont(Application.defaultFont.deriveFont(14f));

            if (value instanceof CellData) {
                CellData cd = (CellData) value;
                this.setText(String.format("%.1f", cd.val));

                if (cd.state == WinState.WIN) {
                    this.setForeground(COL_WIN);
                    this.setFont(this.getFont().deriveFont(Font.BOLD));
                } else if (cd.state == WinState.LOSS) {
                    this.setForeground(COL_LOSS);
                } else {
                    this.setForeground(Color.WHITE);
                }
            } else {
                this.setForeground(Color.WHITE);
            }

            return c;
        }
    }
}
