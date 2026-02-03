package ui.window.comparison;

import java.awt.*;
import javax.swing.*;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;

import parser.Blkx;
import prog.Application;
import prog.util.FMPowerExtractor;
import prog.util.PistonPowerModel;
import prog.util.PistonPowerModel.CompressorStageParams;

/**
 * Displays a power-altitude curve chart for piston engine aircraft.
 *
 * <p>The chart shows how engine power varies with altitude, accounting for:
 * <ul>
 *   <li>Supercharger critical altitudes (where power peaks)</li>
 *   <li>Multi-stage supercharger transitions</li>
 *   <li>WEP (War Emergency Power) effects</li>
 *   <li>RAM air effect from forward motion</li>
 * </ul>
 *
 * <h3>Usage</h3>
 * <pre>
 * PowerCurveWindow win = new PowerCurveWindow(parent, "bf-109f-4", 400, true);
 * win.setVisible(true);
 * </pre>
 */
public class PowerCurveWindow extends JDialog {

    // Chart dimensions
    private static final int CHART_WIDTH = 600;
    private static final int CHART_HEIGHT = 400;
    private static final int MARGIN = 60;

    // Color palette (Material Dark theme)
    private static final Color BG_COLOR = new Color(30, 30, 35);
    private static final Color CHART_BG = new Color(40, 40, 50);
    private static final Color GRID_COLOR = new Color(60, 60, 70);
    private static final Color AXIS_COLOR = new Color(180, 180, 180);
    private static final Color CURVE_COLOR = new Color(46, 255, 113);  // Neon green
    private static final Color PEAK_COLOR = new Color(255, 215, 0);    // Gold
    private static final Color ERROR_COLOR = new Color(255, 160, 0);   // Orange

    private final String fmName;
    private final int speedKmh;
    private final boolean wepMode;

    // Calculated data
    private double[] powerCurve;
    private double maxPower;
    private int peakAltitude;
    private boolean isPiston = false;
    private String errorMessage = null;

    /**
     * Creates a new power curve window.
     *
     * @param owner     parent window
     * @param fmName    FM file name (without extension)
     * @param speedKmh  aircraft speed for RAM effect (IAS, km/h), 0 for static
     * @param wepMode   true to show WEP power, false for military power
     */
    public PowerCurveWindow(Window owner, String fmName, int speedKmh, boolean wepMode) {
        super(owner, "功率曲线: " + fmName, ModalityType.MODELESS);
        this.fmName = fmName;
        this.speedKmh = speedKmh;
        this.wepMode = wepMode;

        loadPowerCurve();
        initUI();
    }

    /**
     * Loads FM data and generates the power curve.
     */
    private void loadPowerCurve() {
        // Try both .Blkx and .blk extensions
        String path = "data/aces/gamedata/flightmodels/fm/" + fmName + ".Blkx";
        java.io.File f = new java.io.File(path);
        if (!f.exists()) {
            path = "data/aces/gamedata/flightmodels/fm/" + fmName + ".blk";
            f = new java.io.File(path);
        }

        if (!f.exists()) {
            errorMessage = "找不到FM文件: " + fmName;
            return;
        }

        // Parse FM file
        Blkx blkx = new Blkx(path, fmName);
        blkx.getAllplotdata();

        // Check if piston engine
        if (!FMPowerExtractor.isPistonEngine(blkx)) {
            isPiston = false;
            errorMessage = "该飞机不是活塞引擎，无法生成功率曲线";
            return;
        }

        // Extract compressor parameters
        CompressorStageParams[] stages = FMPowerExtractor.extractStages(blkx);
        if (stages == null || stages.length == 0) {
            errorMessage = "无法提取发动机参数";
            return;
        }

        isPiston = true;

        // Generate power curve (0m to 15000m, 100m steps)
        // generatePowerCurveAdvanced returns data from -4000m, so we need to offset
        powerCurve = PistonPowerModel.generatePowerCurveAdvanced(
            stages, wepMode, speedKmh, true, 15.0, 100);

        // Find maximum power and peak altitude
        // Array index 40 = 0m altitude (-4000 + 40*100 = 0)
        int seaLevelIdx = 40;
        int maxAltIdx = seaLevelIdx + 150;  // Up to 15000m

        maxPower = 0;
        peakAltitude = 0;
        for (int i = seaLevelIdx; i < maxAltIdx && i < powerCurve.length; i++) {
            if (powerCurve[i] > maxPower) {
                maxPower = powerCurve[i];
                peakAltitude = (i - seaLevelIdx) * 100;
            }
        }
    }

    /**
     * Initializes the UI components.
     */
    private void initUI() {
        setUndecorated(true);
        setSize(CHART_WIDTH + 80, CHART_HEIGHT + 150);
        setLocationRelativeTo(getOwner());
        getContentPane().setBackground(BG_COLOR);

        WebPanel mainPanel = new WebPanel(new BorderLayout(10, 10));
        mainPanel.setOpaque(true);
        mainPanel.setBackground(BG_COLOR);
        mainPanel.setBorder(BorderFactory.createEmptyBorder(15, 15, 15, 15));

        // Title with aircraft info
        String modeText = wepMode ? "WEP" : "军用";
        String speedText = speedKmh > 0 ? speedKmh + " km/h (IAS)" : "静态";
        WebLabel titleLabel = new WebLabel(String.format(
            "<html><center><b style='font-size:14pt'>%s</b><br>" +
            "<span style='font-size:10pt'>速度: %s | 模式: %s</span></center></html>",
            fmName, speedText, modeText), WebLabel.CENTER);
        titleLabel.setForeground(Color.WHITE);
        titleLabel.setFont(Application.defaultFont.deriveFont(14f));
        mainPanel.add(titleLabel, BorderLayout.NORTH);

        // Main content
        if (errorMessage != null) {
            // Show error message
            WebLabel errorLabel = new WebLabel(errorMessage, WebLabel.CENTER);
            errorLabel.setForeground(ERROR_COLOR);
            errorLabel.setFont(Application.defaultFont.deriveFont(14f));
            mainPanel.add(errorLabel, BorderLayout.CENTER);
        } else if (!isPiston) {
            WebLabel errorLabel = new WebLabel("该飞机不是活塞引擎，无法生成功率曲线", WebLabel.CENTER);
            errorLabel.setForeground(ERROR_COLOR);
            errorLabel.setFont(Application.defaultFont.deriveFont(14f));
            mainPanel.add(errorLabel, BorderLayout.CENTER);
        } else {
            // Chart panel
            ChartPanel chartPanel = new ChartPanel();
            mainPanel.add(chartPanel, BorderLayout.CENTER);

            // Statistics panel
            WebPanel statsPanel = new WebPanel(new FlowLayout(FlowLayout.CENTER, 20, 5));
            statsPanel.setOpaque(false);

            WebLabel peakLabel = new WebLabel(String.format(
                "<html>峰值功率: <b style='color:#2EFF71'>%.0f hp</b> @ <b>%d m</b></html>",
                maxPower, peakAltitude));
            peakLabel.setForeground(Color.WHITE);
            peakLabel.setFont(Application.defaultFont.deriveFont(12f));
            statsPanel.add(peakLabel);

            // Sea level power
            int seaLevelIdx = 40;
            double seaLevelPower = powerCurve[seaLevelIdx];
            WebLabel slLabel = new WebLabel(String.format(
                "<html>海平面功率: <b>%.0f hp</b></html>", seaLevelPower));
            slLabel.setForeground(Color.WHITE);
            slLabel.setFont(Application.defaultFont.deriveFont(12f));
            statsPanel.add(slLabel);

            mainPanel.add(statsPanel, BorderLayout.SOUTH);
        }

        setContentPane(mainPanel);

        // Close button
        addCloseButton(mainPanel);

        // ESC to close
        KeyStroke esc = KeyStroke.getKeyStroke(java.awt.event.KeyEvent.VK_ESCAPE, 0);
        getRootPane().registerKeyboardAction(e -> dispose(), esc, JComponent.WHEN_IN_FOCUSED_WINDOW);
    }

    private void addCloseButton(WebPanel mainPanel) {
        WebLabel close = new WebLabel("关闭", WebLabel.CENTER);
        close.setOpaque(true);
        close.setBackground(new Color(183, 28, 28));
        close.setForeground(Color.WHITE);
        close.setFont(Application.defaultFont.deriveFont(Font.BOLD, 12f));
        close.setBorder(BorderFactory.createEmptyBorder(8, 0, 8, 0));
        close.addMouseListener(new java.awt.event.MouseAdapter() {
            public void mouseClicked(java.awt.event.MouseEvent e) {
                dispose();
            }
            public void mouseEntered(java.awt.event.MouseEvent e) {
                close.setBackground(new Color(211, 47, 47));
            }
            public void mouseExited(java.awt.event.MouseEvent e) {
                close.setBackground(new Color(183, 28, 28));
            }
        });

        // Replace SOUTH with a compound panel
        JPanel southPanel = new JPanel(new BorderLayout());
        southPanel.setOpaque(false);

        // Get existing south component if any
        Component existing = ((BorderLayout)mainPanel.getLayout()).getLayoutComponent(BorderLayout.SOUTH);
        if (existing != null) {
            mainPanel.remove(existing);
            southPanel.add(existing, BorderLayout.CENTER);
        }
        southPanel.add(close, BorderLayout.SOUTH);
        mainPanel.add(southPanel, BorderLayout.SOUTH);
    }

    /**
     * Inner class for drawing the power curve chart.
     */
    private class ChartPanel extends JPanel {

        ChartPanel() {
            setPreferredSize(new Dimension(CHART_WIDTH, CHART_HEIGHT));
            setBackground(CHART_BG);
        }

        @Override
        protected void paintComponent(Graphics g) {
            super.paintComponent(g);
            if (powerCurve == null || maxPower <= 0) return;

            Graphics2D g2d = (Graphics2D) g;
            g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING,
                                 RenderingHints.VALUE_ANTIALIAS_ON);
            g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING,
                                 RenderingHints.VALUE_TEXT_ANTIALIAS_ON);

            int w = getWidth();
            int h = getHeight();
            int chartW = w - 2 * MARGIN;
            int chartH = h - 2 * MARGIN;

            // Draw in order: grid, axes, curve, peak marker
            drawGrid(g2d, chartW, chartH);
            drawAxes(g2d, chartW, chartH);
            drawPowerCurve(g2d, chartW, chartH);
            drawPeakMarker(g2d, chartW, chartH);
        }

        private void drawGrid(Graphics2D g2d, int chartW, int chartH) {
            g2d.setColor(GRID_COLOR);
            g2d.setStroke(new BasicStroke(1));

            // Horizontal grid lines (power levels)
            for (int i = 0; i <= 5; i++) {
                int y = MARGIN + chartH - (i * chartH / 5);
                g2d.drawLine(MARGIN, y, MARGIN + chartW, y);
            }

            // Vertical grid lines (altitude levels)
            for (int i = 0; i <= 5; i++) {
                int x = MARGIN + (i * chartW / 5);
                g2d.drawLine(x, MARGIN, x, MARGIN + chartH);
            }
        }

        private void drawAxes(Graphics2D g2d, int chartW, int chartH) {
            g2d.setColor(AXIS_COLOR);
            g2d.setFont(Application.defaultFont.deriveFont(10f));

            FontMetrics fm = g2d.getFontMetrics();

            // Y-axis labels (power)
            for (int i = 0; i <= 5; i++) {
                int y = MARGIN + chartH - (i * chartH / 5);
                int power = (int) (maxPower * i / 5);
                String label = String.valueOf(power);
                int labelWidth = fm.stringWidth(label);
                g2d.drawString(label, MARGIN - labelWidth - 5, y + 4);
            }
            // Y-axis title
            g2d.drawString("hp", 5, MARGIN - 10);

            // X-axis labels (altitude)
            for (int i = 0; i <= 5; i++) {
                int x = MARGIN + (i * chartW / 5);
                int alt = i * 3000;  // 0-15000m range
                String label = alt + "m";
                int labelWidth = fm.stringWidth(label);
                g2d.drawString(label, x - labelWidth / 2, MARGIN + chartH + 18);
            }
        }

        private void drawPowerCurve(Graphics2D g2d, int chartW, int chartH) {
            g2d.setColor(CURVE_COLOR);
            g2d.setStroke(new BasicStroke(2.5f, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));

            int seaLevelIdx = 40;  // -4000 + 40*100 = 0m
            int maxAltIdx = seaLevelIdx + 150;  // 15000m

            int prevX = -1, prevY = -1;
            for (int i = seaLevelIdx; i < maxAltIdx && i < powerCurve.length; i++) {
                int alt = (i - seaLevelIdx) * 100;  // Altitude in meters
                double power = powerCurve[i];

                int x = MARGIN + (int) ((double) alt / 15000 * chartW);
                int y = MARGIN + chartH - (int) (power / maxPower * chartH);

                if (prevX >= 0) {
                    g2d.drawLine(prevX, prevY, x, y);
                }
                prevX = x;
                prevY = y;
            }
        }

        private void drawPeakMarker(Graphics2D g2d, int chartW, int chartH) {
            // Draw peak point marker
            int peakX = MARGIN + (int) ((double) peakAltitude / 15000 * chartW);
            int peakY = MARGIN + chartH - chartH;  // maxPower/maxPower = 1.0

            // Glow effect
            g2d.setColor(new Color(255, 215, 0, 50));
            g2d.fillOval(peakX - 8, peakY - 8, 16, 16);

            // Solid marker
            g2d.setColor(PEAK_COLOR);
            g2d.fillOval(peakX - 5, peakY - 5, 10, 10);

            // Label
            g2d.setFont(Application.defaultFont.deriveFont(Font.BOLD, 10f));
            String peakLabel = String.format("%.0f hp", maxPower);
            FontMetrics fm = g2d.getFontMetrics();
            int labelWidth = fm.stringWidth(peakLabel);

            // Position label to avoid edge clipping
            int labelX = peakX - labelWidth / 2;
            int labelY = peakY - 12;
            if (labelX < MARGIN) labelX = MARGIN;
            if (labelX + labelWidth > MARGIN + chartW) labelX = MARGIN + chartW - labelWidth;

            g2d.drawString(peakLabel, labelX, labelY);
        }
    }
}
