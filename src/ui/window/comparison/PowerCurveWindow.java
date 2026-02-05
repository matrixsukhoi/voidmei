package ui.window.comparison;

import java.awt.*;
import java.util.ArrayList;
import java.util.List;
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
    private static final int CHART_WIDTH = 1000;
    private static final int CHART_HEIGHT = 650;
    private static final int MARGIN = 80;
    private static final int MAX_DISPLAY_ALT = 10000;  // Maximum altitude for chart display (m)
    private static final int ALT_STEP = 25;  // Altitude step for curve generation (m)

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
    private double minPower;
    private double displayMaxPower;  // maxPower rounded up to nearest 100hp for clean grid
    private double displayMinPower;  // minPower rounded down to nearest 100hp for clean grid
    private int peakAltitude;
    private boolean isPiston = false;
    private String errorMessage = null;
    private List<InflectionPoint> inflectionPoints = new ArrayList<>();

    /** Stores metadata for a key inflection point on the power curve. */
    private static class InflectionPoint {
        final String label;
        final int altitudeM;
        final double power;
        final Color color;

        InflectionPoint(String label, int altitudeM, double power, Color color) {
            this.label = label;
            this.altitudeM = altitudeM;
            this.power = power;
            this.color = color;
        }
    }

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

        // Try to load Central file for fuel modifications
        // Central file is at flightmodels/<name>.blkx (parent of fm/ directory)
        Blkx.FuelModification fuelMod = loadFuelModification(fmName);

        // Extract compressor parameters (with fuel modification if available)
        CompressorStageParams[] stages = FMPowerExtractor.extractStages(blkx, fuelMod);
        if (stages == null || stages.length == 0) {
            errorMessage = "无法提取发动机参数";
            return;
        }

        isPiston = true;

        // Generate power curve (0m to 10000m, 50m steps)
        powerCurve = PistonPowerModel.generatePowerCurveAdvanced(
            stages, wepMode, speedKmh, true, 15.0, ALT_STEP);

        // Find maximum/minimum power and peak altitude
        // Array index 0 = 0m altitude (range starts at 0m now)
        int seaLevelIdx = 0;
        int maxAltIdx = MAX_DISPLAY_ALT / ALT_STEP;  // Up to 10000m

        maxPower = 0;
        minPower = Double.MAX_VALUE;
        peakAltitude = 0;
        for (int i = seaLevelIdx; i <= maxAltIdx && i < powerCurve.length; i++) {
            if (powerCurve[i] > maxPower) {
                maxPower = powerCurve[i];
                peakAltitude = i * ALT_STEP;
            }
            if (powerCurve[i] < minPower) {
                minPower = powerCurve[i];
            }
        }
        // Round to nearest 100hp for clean grid lines
        displayMaxPower = Math.ceil(maxPower / 100.0) * 100;
        displayMinPower = Math.floor(minPower / 100.0) * 100;

        // Identify geometric turning points from the power curve shape
        identifyInflectionPoints();
    }

    /**
     * Attempts to load fuel modification data from the Central file.
     *
     * <p>The Central file is located in the parent directory of the FM file:
     * {@code data/aces/gamedata/flightmodels/<name>.blkx} (or .blk)
     * while the FM file is at:
     * {@code data/aces/gamedata/flightmodels/fm/<name>.blkx}
     *
     * @param fmName aircraft FM name
     * @return FuelModification data, or null if Central file not found
     */
    private Blkx.FuelModification loadFuelModification(String fmName) {
        // Try common extensions for Central file
        String[] extensions = {".blkx", ".Blkx", ".blk"};
        for (String ext : extensions) {
            String centralPath = "data/aces/gamedata/flightmodels/" + fmName + ext;
            java.io.File cf = new java.io.File(centralPath);
            if (cf.exists()) {
                try {
                    String data = new String(
                        java.nio.file.Files.readAllBytes(cf.toPath()), "UTF-8");
                    Blkx.FuelModification mod = Blkx.extractFuelModifications(data);
                    if (mod.type != Blkx.FuelModification.FuelType.NONE) {
                        prog.util.Logger.info("PowerCurveWindow",
                            "Fuel modification: " + mod.type);
                    }
                    return mod;
                } catch (Exception e) {
                    // Central file exists but failed to parse — continue without fuel mod
                    prog.util.Logger.debug("PowerCurveWindow",
                        "Failed to parse Central file: " + e.getMessage());
                }
            }
        }
        return null;
    }

    /**
     * Detects geometric turning points directly from the power curve shape.
     *
     * <p>Detected features:
     * <ul>
     *   <li>Local maxima (peaks) — curve rises then falls</li>
     *   <li>Local minima (valleys) — curve falls then rises</li>
     *   <li>Inflection points — concavity changes (second derivative sign flip)</li>
     * </ul>
     */
    private void identifyInflectionPoints() {
        inflectionPoints.clear();

        int maxIdx = Math.min(MAX_DISPLAY_ALT / ALT_STEP, powerCurve.length - 1);
        if (maxIdx < 6) return;

        int minSepM = 300;  // minimum separation between markers
        double noiseThreshold = maxPower * 0.005;  // 0.5% of max power

        // 1. Detect local peaks, valleys, and shoulder points (±100m window)
        int hw = 4;
        for (int i = hw; i <= maxIdx - hw; i++) {
            double left = powerCurve[i - hw];
            double center = powerCurve[i];
            double right = powerCurve[i + hw];

            boolean isPeak = center > left && center > right
                && (center - Math.min(left, right)) > noiseThreshold;
            boolean isValley = center < left && center < right
                && (Math.max(left, right) - center) > noiseThreshold;
            // Shoulder: plateau → descent transition
            // Catches flat-topped curves that peak detection misses (center ≈ left)
            boolean isShoulder = !isPeak
                && Math.abs(center - left) < noiseThreshold * 2
                && (center - right) > noiseThreshold;

            if (isPeak && !tooCloseToExisting(i * ALT_STEP, minSepM)) {
                // Refine: find the exact maximum within the window
                int bestIdx = i;
                for (int j = i - hw; j <= i + hw; j++) {
                    if (powerCurve[j] > powerCurve[bestIdx]) bestIdx = j;
                }
                inflectionPoints.add(new InflectionPoint(
                    "Peak", bestIdx * ALT_STEP, powerCurve[bestIdx], PEAK_COLOR));
            } else if (isValley && !tooCloseToExisting(i * ALT_STEP, minSepM)) {
                int bestIdx = i;
                for (int j = i - hw; j <= i + hw; j++) {
                    if (powerCurve[j] < powerCurve[bestIdx]) bestIdx = j;
                }
                inflectionPoints.add(new InflectionPoint(
                    "Valley", bestIdx * ALT_STEP, powerCurve[bestIdx],
                    new Color(100, 200, 255)));
            } else if (isShoulder && !tooCloseToExisting(i * ALT_STEP, minSepM)) {
                inflectionPoints.add(new InflectionPoint(
                    "Peak", i * ALT_STEP, powerCurve[i], PEAK_COLOR));
            }
        }

        // 2. Detect inflection points (concavity changes via second derivative)
        // d2[i] ≈ curve[i+h] - 2*curve[i] + curve[i-h], using h=±200m for smoothing
        int d2h = 8;
        double prevD2 = powerCurve[d2h + d2h] - 2 * powerCurve[d2h] + powerCurve[0];
        // Require meaningful curvature on at least one side of the sign flip
        // to filter noise-driven flips (e.g., flat sections where d2 ≈ 0)
        double d2MinMag = noiseThreshold;
        for (int i = d2h + 1; i <= maxIdx - d2h; i++) {
            double d2 = powerCurve[i + d2h] - 2 * powerCurve[i] + powerCurve[i - d2h];

            if (prevD2 * d2 < 0
                && (Math.abs(prevD2) > d2MinMag || Math.abs(d2) > d2MinMag)
                && !tooCloseToExisting(i * ALT_STEP, minSepM)) {
                inflectionPoints.add(new InflectionPoint(
                    "Inflect", i * ALT_STEP, powerCurve[i],
                    new Color(180, 130, 255)));
            }
            prevD2 = d2;
        }
    }

    /** Returns true if any existing marker is within {@code minSepM} meters of {@code altM}. */
    private boolean tooCloseToExisting(int altM, int minSepM) {
        for (InflectionPoint p : inflectionPoints) {
            if (Math.abs(p.altitudeM - altM) < minSepM) return true;
        }
        return false;
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
        titleLabel.setFont(Application.defaultFont.deriveFont(16f));
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
            peakLabel.setFont(Application.defaultFont.deriveFont(14f));
            statsPanel.add(peakLabel);

            // Sea level power (index 0 = 0m altitude)
            double seaLevelPower = powerCurve[0];
            WebLabel slLabel = new WebLabel(String.format(
                "<html>海平面功率: <b>%.0f hp</b></html>", seaLevelPower));
            slLabel.setForeground(Color.WHITE);
            slLabel.setFont(Application.defaultFont.deriveFont(14f));
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

            // Draw in order: grid, axes, curve, inflection points
            drawGrid(g2d, chartW, chartH);
            drawAxes(g2d, chartW, chartH);
            drawPowerCurve(g2d, chartW, chartH);
            drawInflectionPoints(g2d, chartW, chartH);
        }

        private void drawGrid(Graphics2D g2d, int chartW, int chartH) {
            g2d.setColor(GRID_COLOR);
            g2d.setStroke(new BasicStroke(1));

            double powerRange = displayMaxPower - displayMinPower;

            // Horizontal grid lines: every 1000 m (Y-axis is altitude)
            int altSteps = MAX_DISPLAY_ALT / 1000;
            for (int i = 0; i <= altSteps; i++) {
                int y = MARGIN + chartH - (int) (i * 1000.0 / MAX_DISPLAY_ALT * chartH);
                g2d.drawLine(MARGIN, y, MARGIN + chartW, y);
            }

            // Vertical grid lines: every 100 hp from displayMinPower to displayMaxPower
            int minHpStep = (int) (displayMinPower / 100);
            int maxHpStep = (int) (displayMaxPower / 100);
            for (int i = minHpStep; i <= maxHpStep; i++) {
                double hp = i * 100.0;
                int x = MARGIN + (int) ((hp - displayMinPower) / powerRange * chartW);
                g2d.drawLine(x, MARGIN, x, MARGIN + chartH);
            }
        }

        private void drawAxes(Graphics2D g2d, int chartW, int chartH) {
            g2d.setColor(AXIS_COLOR);
            g2d.setFont(Application.defaultFont.deriveFont(12f));

            FontMetrics fm = g2d.getFontMetrics();
            double powerRange = displayMaxPower - displayMinPower;

            // Y-axis labels: every 1000 m (altitude on Y-axis)
            int altSteps = MAX_DISPLAY_ALT / 1000;
            for (int i = 0; i <= altSteps; i++) {
                int y = MARGIN + chartH - (int) (i * 1000.0 / MAX_DISPLAY_ALT * chartH);
                String label = (i * 1000) + "m";
                int labelWidth = fm.stringWidth(label);
                g2d.drawString(label, MARGIN - labelWidth - 5, y + 4);
            }

            // X-axis labels: every 100 hp from displayMinPower to displayMaxPower
            int minHpStep = (int) (displayMinPower / 100);
            int maxHpStep = (int) (displayMaxPower / 100);
            for (int i = minHpStep; i <= maxHpStep; i++) {
                double hp = i * 100.0;
                int x = MARGIN + (int) ((hp - displayMinPower) / powerRange * chartW);
                String label = String.valueOf(i * 100);
                int labelWidth = fm.stringWidth(label);
                g2d.drawString(label, x - labelWidth / 2, MARGIN + chartH + 18);
            }
            // X-axis title
            g2d.drawString("hp", MARGIN + chartW + 5, MARGIN + chartH + 4);
        }

        private void drawPowerCurve(Graphics2D g2d, int chartW, int chartH) {
            g2d.setColor(CURVE_COLOR);
            g2d.setStroke(new BasicStroke(2.5f, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));

            double powerRange = displayMaxPower - displayMinPower;
            int seaLevelIdx = 0;  // index 0 = 0m altitude
            int maxAltIdx = MAX_DISPLAY_ALT / ALT_STEP;  // 10000m

            int prevX = -1, prevY = -1;
            for (int i = seaLevelIdx; i <= maxAltIdx && i < powerCurve.length; i++) {
                int alt = i * ALT_STEP;  // Altitude in meters
                double power = powerCurve[i];

                // X-axis = power (starting from displayMinPower), Y-axis = altitude
                int x = MARGIN + (int) ((power - displayMinPower) / powerRange * chartW);
                int y = MARGIN + chartH - (int) ((double) alt / MAX_DISPLAY_ALT * chartH);

                if (prevX >= 0) {
                    g2d.drawLine(prevX, prevY, x, y);
                }
                prevX = x;
                prevY = y;
            }
        }

        private void drawInflectionPoints(Graphics2D g2d, int chartW, int chartH) {
            g2d.setFont(Application.defaultFont.deriveFont(Font.BOLD, 11f));
            FontMetrics fm = g2d.getFontMetrics();
            Stroke defaultStroke = g2d.getStroke();
            int labelH = fm.getHeight();
            int panelW = getWidth();
            int panelH = getHeight();
            double powerRange = displayMaxPower - displayMinPower;

            for (InflectionPoint ip : inflectionPoints) {
                if (ip.altitudeM > MAX_DISPLAY_ALT) continue;

                // Map data coordinates to pixel coordinates (X = power starting from displayMinPower, Y = altitude)
                int x = MARGIN + (int) ((ip.power - displayMinPower) / powerRange * chartW);
                int y = MARGIN + chartH - (int) ((double) ip.altitudeM / MAX_DISPLAY_ALT * chartH);

                // Determine curve direction around this point
                int altIdx = (int) (ip.altitudeM / ALT_STEP);
                int lookback = Math.max(altIdx - 2, 0);
                int lookahead = Math.min(altIdx + 2, powerCurve.length - 1);
                boolean fallingBefore = powerCurve[altIdx] < powerCurve[lookback];
                boolean risingAfter = powerCurve[lookahead] > powerCurve[altIdx];
                // Valleys now appear on the left side of the curve (lower power)
                boolean isValley = fallingBefore && risingAfter;

                // Glow halo
                g2d.setColor(new Color(ip.color.getRed(), ip.color.getGreen(), ip.color.getBlue(), 80));
                g2d.fillOval(x - 10, y - 10, 20, 20);

                // Solid marker dot
                g2d.setColor(ip.color);
                g2d.fillOval(x - 6, y - 6, 12, 12);

                // Coordinate label (e.g. "Peak: 1650hp / 4500m")
                String coordLabel = String.format("%s: %.0fhp / %dm", ip.label, ip.power, ip.altitudeM);
                int labelWidth = fm.stringWidth(coordLabel);

                // Position: right side by default, left side for valleys (which are now on the left)
                int labelX;
                int labelY = y - 10;  // slightly above the point
                if (isValley) {
                    labelX = x - labelWidth - 14;  // left side for valleys
                } else {
                    labelX = x + 14;               // right side for peaks
                }

                // Boundary clamping: labels may exceed chart area but not the window
                if (labelX + labelWidth > panelW - 4) {
                    labelX = x - labelWidth - 14;  // Flip to left side
                }
                if (labelX < 4) labelX = 4;
                if (labelY - labelH + 3 < 4) labelY = labelH + 1;
                if (labelY + 2 > panelH - 4) labelY = panelH - 6;

                // Label background
                g2d.setColor(new Color(30, 30, 35, 200));
                g2d.fillRoundRect(labelX - 4, labelY - labelH + 3, labelWidth + 8, labelH + 2, 4, 4);

                // Label text
                g2d.setColor(ip.color);
                g2d.drawString(coordLabel, labelX, labelY);

                // Dashed horizontal line: marker dot → Y-axis (since Y is now altitude)
                g2d.setStroke(new BasicStroke(1f, BasicStroke.CAP_BUTT, BasicStroke.JOIN_MITER,
                    10f, new float[]{4f, 4f}, 0f));
                g2d.setColor(new Color(ip.color.getRed(), ip.color.getGreen(), ip.color.getBlue(), 100));
                g2d.drawLine(MARGIN, y, x - 6, y);
                g2d.setStroke(defaultStroke);
            }
        }
    }
}
