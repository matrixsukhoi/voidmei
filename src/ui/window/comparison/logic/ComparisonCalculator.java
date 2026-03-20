package ui.window.comparison.logic;

import java.awt.Color;
import ui.window.comparison.model.ComparisonData;

public class ComparisonCalculator {

    public enum WinState {
        WIN, LOSS, DRAW, UNKNOWN
    }

    public static class DiffResult {
        public double diff;
        public double percent;
        public WinState win;

        public DiffResult(double diff, double percent, WinState win) {
            this.diff = diff;
            this.percent = percent;
            this.win = win;
        }
    }

    public static DiffResult compare(double val0, double val1, boolean higherIsBetter) {
        if (val0 == 0 || val1 == 0) {
            return new DiffResult(0, 0, WinState.UNKNOWN);
        }

        double diff = val1 - val0;
        double percent = (diff / val0) * 100.0;

        // Epsilon check
        if (Math.abs(diff) < 0.001) {
            return new DiffResult(0, 0, WinState.DRAW);
        }

        WinState win;
        if (higherIsBetter) {
            win = (diff > 0) ? WinState.WIN : WinState.LOSS;
        } else {
            win = (diff < 0) ? WinState.WIN : WinState.LOSS;
        }

        return new DiffResult(diff, percent, win);
    }
}
