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
    private static final Color ERROR_COLOR = new Color(255, 160, 0);   // Orange

    // FM0 color scheme (green family)
    private static final Color CURVE_COLOR_FM0 = new Color(46, 255, 113);   // Neon green
    private static final Color PEAK_COLOR_FM0 = new Color(255, 215, 0);     // Gold
    private static final Color VALLEY_COLOR_FM0 = new Color(100, 200, 255); // Light blue
    private static final Color KINK_COLOR_FM0 = new Color(180, 130, 255);   // Purple

    // FM1 color scheme (cyan family)
    private static final Color CURVE_COLOR_FM1 = new Color(0, 212, 255);    // Cyan
    private static final Color PEAK_COLOR_FM1 = new Color(255, 128, 180);   // Pink
    private static final Color VALLEY_COLOR_FM1 = new Color(255, 153, 102); // Orange
    private static final Color KINK_COLOR_FM1 = new Color(130, 255, 180);   // Mint green

    private final String fm0Name;
    private final String fm1Name;  // null or empty for single curve mode
    private final int speedKmh;
    private final boolean wepMode;

    // Calculated data for each curve
    private CurveData curveData0;
    private CurveData curveData1;  // null for single curve mode

    // Combined display range (merged from both curves)
    private double displayMaxPower;
    private double displayMinPower;

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

    /** Encapsulates all data for a single power curve. */
    private static class CurveData {
        final String fmName;
        final double[] powerCurve;
        final double maxPower;
        final double minPower;
        final int peakAltitude;
        final List<InflectionPoint> inflectionPoints;
        final String errorMessage;  // null means success
        final Color curveColor;
        final Color peakColor;
        final Color valleyColor;
        final Color kinkColor;

        CurveData(String fmName, double[] powerCurve, double maxPower, double minPower,
                  int peakAltitude, List<InflectionPoint> inflectionPoints, String errorMessage,
                  Color curveColor, Color peakColor, Color valleyColor, Color kinkColor) {
            this.fmName = fmName;
            this.powerCurve = powerCurve;
            this.maxPower = maxPower;
            this.minPower = minPower;
            this.peakAltitude = peakAltitude;
            this.inflectionPoints = inflectionPoints;
            this.errorMessage = errorMessage;
            this.curveColor = curveColor;
            this.peakColor = peakColor;
            this.valleyColor = valleyColor;
            this.kinkColor = kinkColor;
        }

        boolean isValid() {
            return errorMessage == null && powerCurve != null;
        }
    }

    /** Tracks label position for collision detection. */
    private static class LabelPosition {
        int markerX, markerY;      // Marker dot position
        int labelX, labelY;        // Label text position
        int labelWidth, labelHeight;
        String text;
        Color color;
        boolean isLeftSide;        // true if label is on left side of marker
        InflectionPoint ip;
        int curveIndex;            // 0 for FM0, 1 for FM1

        Rectangle getBounds() {
            return new Rectangle(labelX - 4, labelY - labelHeight + 3,
                                 labelWidth + 8, labelHeight + 2);
        }

        void flipSide() {
            if (isLeftSide) {
                // Move to right side
                labelX = markerX + 14;
                isLeftSide = false;
            } else {
                // Move to left side
                labelX = markerX - labelWidth - 14;
                isLeftSide = true;
            }
        }

        void offsetY(int delta) {
            labelY += delta;
        }
    }

    /**
     * Creates a new power curve window with a single FM.
     *
     * @param owner     parent window
     * @param fmName    FM file name (without extension)
     * @param speedKmh  aircraft speed for RAM effect (IAS, km/h), 0 for static
     * @param wepMode   true to show WEP power, false for military power
     */
    public PowerCurveWindow(Window owner, String fmName, int speedKmh, boolean wepMode) {
        this(owner, fmName, null, speedKmh, wepMode);
    }

    /**
     * Creates a new power curve window with two FMs for comparison.
     *
     * @param owner     parent window
     * @param fm0Name   first FM file name (without extension)
     * @param fm1Name   second FM file name (without extension), null or empty for single mode
     * @param speedKmh  aircraft speed for RAM effect (IAS, km/h), 0 for static
     * @param wepMode   true to show WEP power, false for military power
     */
    public PowerCurveWindow(Window owner, String fm0Name, String fm1Name, int speedKmh, boolean wepMode) {
        super(owner, "功率曲线", ModalityType.MODELESS);
        this.fm0Name = fm0Name;
        // Treat fm1Name == fm0Name as single curve mode
        this.fm1Name = (fm1Name != null && !fm1Name.isEmpty() && !fm1Name.equals(fm0Name)) ? fm1Name : null;
        this.speedKmh = speedKmh;
        this.wepMode = wepMode;

        loadPowerCurves();
        initUI();
    }

    /** Returns true if displaying two curves. */
    private boolean isDualMode() {
        return fm1Name != null && curveData1 != null;
    }

    /**
     * Loads FM data and generates power curves for both FMs.
     */
    private void loadPowerCurves() {
        // Load FM0 (primary curve)
        curveData0 = loadSingleCurve(fm0Name,
            CURVE_COLOR_FM0, PEAK_COLOR_FM0, VALLEY_COLOR_FM0, KINK_COLOR_FM0);

        // Load FM1 (secondary curve) if in dual mode
        if (fm1Name != null) {
            curveData1 = loadSingleCurve(fm1Name,
                CURVE_COLOR_FM1, PEAK_COLOR_FM1, VALLEY_COLOR_FM1, KINK_COLOR_FM1);
        }

        // Calculate combined display range
        calculateDisplayRange();
    }

    /**
     * Loads a single FM and generates its power curve.
     *
     * @param fmName      FM file name
     * @param curveColor  color for the curve line
     * @param peakColor   color for peak markers
     * @param valleyColor color for valley markers
     * @param kinkColor   color for kink markers
     * @return CurveData containing all curve information
     */
    private CurveData loadSingleCurve(String fmName,
            Color curveColor, Color peakColor, Color valleyColor, Color kinkColor) {

        // Try both .Blkx and .blk extensions
        String path = "data/aces/gamedata/flightmodels/fm/" + fmName + ".Blkx";
        java.io.File f = new java.io.File(path);
        if (!f.exists()) {
            path = "data/aces/gamedata/flightmodels/fm/" + fmName + ".blk";
            f = new java.io.File(path);
        }

        if (!f.exists()) {
            return new CurveData(fmName, null, 0, 0, 0, new ArrayList<>(),
                "找不到FM文件: " + fmName, curveColor, peakColor, valleyColor, kinkColor);
        }

        // Parse FM file
        Blkx blkx = new Blkx(path, fmName);
        blkx.getAllplotdata();

        // Check if piston engine
        if (!FMPowerExtractor.isPistonEngine(blkx)) {
            return new CurveData(fmName, null, 0, 0, 0, new ArrayList<>(),
                fmName + " 不是活塞引擎", curveColor, peakColor, valleyColor, kinkColor);
        }

        // Try to load Central file for fuel modifications
        Blkx.FuelModification fuelMod = loadFuelModification(fmName);

        // Extract compressor parameters (with fuel modification if available)
        CompressorStageParams[] stages = FMPowerExtractor.extractStages(blkx, fuelMod);
        if (stages == null || stages.length == 0) {
            return new CurveData(fmName, null, 0, 0, 0, new ArrayList<>(),
                "无法提取 " + fmName + " 的发动机参数", curveColor, peakColor, valleyColor, kinkColor);
        }

        // Generate power curve (0m to 10000m)
        double[] powerCurve = PistonPowerModel.generatePowerCurveAdvanced(
            stages, wepMode, speedKmh, true, 15.0, ALT_STEP);

        // Multi-engine aircraft: multiply each point by engine count
        if (blkx.engineNum > 1) {
            for (int i = 0; i < powerCurve.length; i++) {
                powerCurve[i] *= blkx.engineNum;
            }
        }

        // Find maximum/minimum power and peak altitude
        int maxAltIdx = MAX_DISPLAY_ALT / ALT_STEP;
        double maxPower = 0;
        double minPower = Double.MAX_VALUE;
        int peakAltitude = 0;

        for (int i = 0; i <= maxAltIdx && i < powerCurve.length; i++) {
            if (powerCurve[i] > maxPower) {
                maxPower = powerCurve[i];
                peakAltitude = i * ALT_STEP;
            }
            if (powerCurve[i] < minPower) {
                minPower = powerCurve[i];
            }
        }

        // Identify inflection points
        List<InflectionPoint> inflectionPoints = identifyInflectionPointsForCurve(
            powerCurve, maxPower, peakColor, valleyColor, kinkColor);

        return new CurveData(fmName, powerCurve, maxPower, minPower, peakAltitude,
            inflectionPoints, null, curveColor, peakColor, valleyColor, kinkColor);
    }

    /**
     * Calculates the combined display range from both curves.
     */
    private void calculateDisplayRange() {
        double combinedMax = 0;
        double combinedMin = Double.MAX_VALUE;

        if (curveData0 != null && curveData0.isValid()) {
            combinedMax = Math.max(combinedMax, curveData0.maxPower);
            combinedMin = Math.min(combinedMin, curveData0.minPower);
        }
        if (curveData1 != null && curveData1.isValid()) {
            combinedMax = Math.max(combinedMax, curveData1.maxPower);
            combinedMin = Math.min(combinedMin, curveData1.minPower);
        }

        // Handle case where no valid curves
        if (combinedMin == Double.MAX_VALUE) {
            combinedMin = 0;
            combinedMax = 1000;
        }

        // Round to nearest 100hp for clean grid lines
        displayMaxPower = Math.ceil(combinedMax / 100.0) * 100;
        displayMinPower = Math.floor(combinedMin / 100.0) * 100;
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
     *
     * @param powerCurve  the power curve array
     * @param maxPower    maximum power value for noise threshold calculation
     * @param peakColor   color for peak markers
     * @param valleyColor color for valley markers
     * @param kinkColor   color for kink markers
     * @return list of detected inflection points
     */
    private List<InflectionPoint> identifyInflectionPointsForCurve(double[] powerCurve,
            double maxPower, Color peakColor, Color valleyColor, Color kinkColor) {

        List<InflectionPoint> result = new ArrayList<>();

        int maxIdx = Math.min(MAX_DISPLAY_ALT / ALT_STEP, powerCurve.length - 1);
        if (maxIdx < 6) return result;

        int minSepM = 300;  // minimum separation between markers
        double noiseThreshold = maxPower * 0.005;  // 0.5% of max power
        int hw = 4;  // ±100m window (4 × 25m step)

        // ========== Phase 1: Collect peaks and valleys separately ==========
        List<double[]> peakCandidates = new ArrayList<>();
        List<double[]> valleyCandidates = new ArrayList<>();

        for (int i = hw; i <= maxIdx - hw; i++) {
            double left = powerCurve[i - hw];
            double center = powerCurve[i];
            double right = powerCurve[i + hw];

            double leftSlope = center - left;
            double rightSlope = right - center;

            boolean slopeSignChangePeak = leftSlope > 0 && rightSlope < 0;
            boolean slopeSignChangeValley = leftSlope < 0 && rightSlope > 0;

            double peakProminence = center - Math.min(left, right);
            double valleyProminence = Math.max(left, right) - center;

            if (slopeSignChangePeak && peakProminence > noiseThreshold) {
                int bestIdx = i;
                for (int j = i - hw; j <= i + hw; j++) {
                    if (powerCurve[j] > powerCurve[bestIdx]) bestIdx = j;
                }
                int altM = bestIdx * ALT_STEP;
                boolean tooClose = false;
                for (double[] prev : peakCandidates) {
                    if (Math.abs((int) prev[0] - altM) < minSepM) { tooClose = true; break; }
                }
                if (!tooClose) {
                    peakCandidates.add(new double[]{altM, powerCurve[bestIdx]});
                }
            } else if (slopeSignChangeValley && valleyProminence > noiseThreshold) {
                int bestIdx = i;
                for (int j = i - hw; j <= i + hw; j++) {
                    if (powerCurve[j] < powerCurve[bestIdx]) bestIdx = j;
                }
                int altM = bestIdx * ALT_STEP;
                boolean tooClose = false;
                for (double[] prev : valleyCandidates) {
                    if (Math.abs((int) prev[0] - altM) < minSepM) { tooClose = true; break; }
                }
                if (!tooClose) {
                    valleyCandidates.add(new double[]{altM, powerCurve[bestIdx]});
                }
            }
        }

        // Sort by altitude (ascending)
        peakCandidates.sort((a, b) -> Double.compare(a[0], b[0]));
        valleyCandidates.sort((a, b) -> Double.compare(a[0], b[0]));

        // ========== Phase 2: Add valleys (stage transitions) ==========
        for (double[] valley : valleyCandidates) {
            int altM = (int) valley[0];
            double power = valley[1];

            int peaksBelow = 0;
            for (double[] peak : peakCandidates) {
                if (peak[0] < altM) peaksBelow++;
            }

            int fromStage = Math.max(1, peaksBelow);
            int toStage = fromStage + 1;

            String label = fromStage + "→" + toStage + "档";
            result.add(new InflectionPoint(label, altM, power, valleyColor));
        }

        // ========== Phase 3: Add peaks (critical altitudes) ==========
        int stageNum = 1;
        for (double[] peak : peakCandidates) {
            int altM = (int) peak[0];
            double power = peak[1];

            if (tooCloseToList(altM, minSepM, result)) {
                stageNum++;
                continue;
            }

            String label = stageNum + "档";
            result.add(new InflectionPoint(label, altM, power, peakColor));
            stageNum++;
        }

        // ========== Phase 4: Detect slope kinks ==========
        int kinkHalfWindow = 4;
        double avgSlope = Math.abs(powerCurve[maxIdx] - powerCurve[0]) / (maxIdx * ALT_STEP);
        double kinkThreshold = Math.max(avgSlope * 2.5, 0.08);

        for (int i = kinkHalfWindow; i <= maxIdx - kinkHalfWindow; i++) {
            if (tooCloseToList(i * ALT_STEP, minSepM, result)) {
                continue;
            }

            double leftSlope = (powerCurve[i] - powerCurve[i - kinkHalfWindow])
                               / (kinkHalfWindow * ALT_STEP);
            double rightSlope = (powerCurve[i + kinkHalfWindow] - powerCurve[i])
                                / (kinkHalfWindow * ALT_STEP);

            double slopeChange = Math.abs(rightSlope - leftSlope);
            boolean sameSlopeDirection = (leftSlope * rightSlope >= 0);
            boolean isPeakOrValley = !sameSlopeDirection;

            if (!isPeakOrValley && slopeChange > kinkThreshold) {
                int bestIdx = i;
                double bestChange = slopeChange;
                for (int j = i - 2; j <= i + 2 && j >= kinkHalfWindow && j <= maxIdx - kinkHalfWindow; j++) {
                    double lS = (powerCurve[j] - powerCurve[j - kinkHalfWindow])
                                / (kinkHalfWindow * ALT_STEP);
                    double rS = (powerCurve[j + kinkHalfWindow] - powerCurve[j])
                                / (kinkHalfWindow * ALT_STEP);
                    double sc = Math.abs(rS - lS);
                    if (sc > bestChange) {
                        bestChange = sc;
                        bestIdx = j;
                    }
                }

                if (!tooCloseToList(bestIdx * ALT_STEP, minSepM, result)) {
                    result.add(new InflectionPoint("Kink", bestIdx * ALT_STEP,
                        powerCurve[bestIdx], kinkColor));
                }
            }
        }

        return result;
    }

    /** Returns true if any point in the list is within {@code minSepM} meters of {@code altM}. */
    private boolean tooCloseToList(int altM, int minSepM, List<InflectionPoint> list) {
        for (InflectionPoint p : list) {
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
        String titleText;
        if (isDualMode()) {
            titleText = String.format(
                "<html><center><b style='font-size:14pt'>%s vs %s</b><br>" +
                "<span style='font-size:10pt'>速度: %s | 模式: %s</span></center></html>",
                fm0Name, fm1Name, speedText, modeText);
        } else {
            titleText = String.format(
                "<html><center><b style='font-size:14pt'>%s</b><br>" +
                "<span style='font-size:10pt'>速度: %s | 模式: %s</span></center></html>",
                fm0Name, speedText, modeText);
        }
        WebLabel titleLabel = new WebLabel(titleText, WebLabel.CENTER);
        titleLabel.setForeground(Color.WHITE);
        titleLabel.setFont(Application.defaultFont.deriveFont(16f));
        mainPanel.add(titleLabel, BorderLayout.NORTH);

        // Determine if we have any valid curves
        boolean hasFm0 = curveData0 != null && curveData0.isValid();
        boolean hasFm1 = curveData1 != null && curveData1.isValid();
        boolean hasAnyCurve = hasFm0 || hasFm1;

        // Collect error messages
        String errorMsg = buildErrorMessage();

        if (!hasAnyCurve && errorMsg != null) {
            // No valid curves - show error
            WebLabel errorLabel = new WebLabel(errorMsg, WebLabel.CENTER);
            errorLabel.setForeground(ERROR_COLOR);
            errorLabel.setFont(Application.defaultFont.deriveFont(14f));
            mainPanel.add(errorLabel, BorderLayout.CENTER);
        } else {
            // At least one valid curve
            ChartPanel chartPanel = new ChartPanel();
            mainPanel.add(chartPanel, BorderLayout.CENTER);

            // Statistics panel
            WebPanel statsPanel = new WebPanel(new FlowLayout(FlowLayout.CENTER, 20, 5));
            statsPanel.setOpaque(false);

            // Show stats for FM0
            if (hasFm0) {
                String colorHex = isDualMode() ? "#2EFF71" : "#2EFF71";
                WebLabel peakLabel = new WebLabel(String.format(
                    "<html>%s 峰值: <b style='color:%s'>%.0f hp</b> @ <b>%d m</b></html>",
                    isDualMode() ? fm0Name : "峰值功率", colorHex,
                    curveData0.maxPower, curveData0.peakAltitude));
                peakLabel.setForeground(Color.WHITE);
                peakLabel.setFont(Application.defaultFont.deriveFont(14f));
                statsPanel.add(peakLabel);
            }

            // Show stats for FM1
            if (hasFm1) {
                WebLabel peakLabel1 = new WebLabel(String.format(
                    "<html>%s 峰值: <b style='color:#00D4FF'>%.0f hp</b> @ <b>%d m</b></html>",
                    fm1Name, curveData1.maxPower, curveData1.peakAltitude));
                peakLabel1.setForeground(Color.WHITE);
                peakLabel1.setFont(Application.defaultFont.deriveFont(14f));
                statsPanel.add(peakLabel1);
            }

            // Show partial error if one FM failed
            if (errorMsg != null) {
                WebLabel errorLabel = new WebLabel(String.format(
                    "<html><span style='color:#FFA000'>%s</span></html>", errorMsg));
                errorLabel.setFont(Application.defaultFont.deriveFont(12f));
                statsPanel.add(errorLabel);
            }

            mainPanel.add(statsPanel, BorderLayout.SOUTH);
        }

        setContentPane(mainPanel);

        // Close button
        addCloseButton(mainPanel);

        // ESC to close
        KeyStroke esc = KeyStroke.getKeyStroke(java.awt.event.KeyEvent.VK_ESCAPE, 0);
        getRootPane().registerKeyboardAction(e -> dispose(), esc, JComponent.WHEN_IN_FOCUSED_WINDOW);
    }

    /**
     * Builds an error message from failed curves.
     * @return error message, or null if no errors
     */
    private String buildErrorMessage() {
        boolean hasFm0 = curveData0 != null && curveData0.isValid();
        boolean hasFm1 = curveData1 != null && curveData1.isValid();

        if (!hasFm0 && !hasFm1) {
            // Both failed
            StringBuilder sb = new StringBuilder();
            if (curveData0 != null && curveData0.errorMessage != null) {
                sb.append(curveData0.errorMessage);
            }
            if (curveData1 != null && curveData1.errorMessage != null) {
                if (sb.length() > 0) sb.append(" | ");
                sb.append(curveData1.errorMessage);
            }
            return sb.length() > 0 ? sb.toString() : "无法加载功率曲线";
        } else if (!hasFm0 && curveData0 != null && curveData0.errorMessage != null) {
            return curveData0.errorMessage;
        } else if (!hasFm1 && curveData1 != null && curveData1.errorMessage != null) {
            return curveData1.errorMessage;
        }
        return null;
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

            // Check if we have any valid curve
            boolean hasFm0 = curveData0 != null && curveData0.isValid();
            boolean hasFm1 = curveData1 != null && curveData1.isValid();
            if (!hasFm0 && !hasFm1) return;

            Graphics2D g2d = (Graphics2D) g;
            g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING,
                                 RenderingHints.VALUE_ANTIALIAS_ON);
            g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING,
                                 RenderingHints.VALUE_TEXT_ANTIALIAS_ON);

            int w = getWidth();
            int h = getHeight();
            int chartW = w - 2 * MARGIN;
            int chartH = h - 2 * MARGIN;

            // Draw in order: grid, axes, curves, inflection points, legend
            drawGrid(g2d, chartW, chartH);
            drawAxes(g2d, chartW, chartH);

            // Draw both curves
            if (hasFm0) {
                drawPowerCurve(g2d, chartW, chartH, curveData0);
            }
            if (hasFm1) {
                drawPowerCurve(g2d, chartW, chartH, curveData1);
            }

            // Draw inflection points with collision detection
            drawAllInflectionPoints(g2d, chartW, chartH);

            // Draw legend in dual mode
            if (isDualMode()) {
                drawLegend(g2d, chartW, chartH);
            }
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

        private void drawPowerCurve(Graphics2D g2d, int chartW, int chartH, CurveData curve) {
            g2d.setColor(curve.curveColor);
            g2d.setStroke(new BasicStroke(2.5f, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND));

            double powerRange = displayMaxPower - displayMinPower;
            int maxAltIdx = MAX_DISPLAY_ALT / ALT_STEP;

            int prevX = -1, prevY = -1;
            for (int i = 0; i <= maxAltIdx && i < curve.powerCurve.length; i++) {
                int alt = i * ALT_STEP;
                double power = curve.powerCurve[i];

                int x = MARGIN + (int) ((power - displayMinPower) / powerRange * chartW);
                int y = MARGIN + chartH - (int) ((double) alt / MAX_DISPLAY_ALT * chartH);

                if (prevX >= 0) {
                    g2d.drawLine(prevX, prevY, x, y);
                }
                prevX = x;
                prevY = y;
            }
        }

        /**
         * Draws all inflection points from both curves with collision detection.
         */
        private void drawAllInflectionPoints(Graphics2D g2d, int chartW, int chartH) {
            g2d.setFont(Application.defaultFont.deriveFont(Font.BOLD, 11f));
            FontMetrics fm = g2d.getFontMetrics();
            int labelH = fm.getHeight();
            int panelW = getWidth();
            int panelH = getHeight();
            double powerRange = displayMaxPower - displayMinPower;

            // Collect all label positions
            List<LabelPosition> labels = new ArrayList<>();

            // Process FM0 inflection points
            if (curveData0 != null && curveData0.isValid()) {
                for (InflectionPoint ip : curveData0.inflectionPoints) {
                    LabelPosition lp = createLabelPosition(ip, fm, chartW, chartH, panelW, panelH,
                        powerRange, curveData0.powerCurve, 0);
                    if (lp != null) labels.add(lp);
                }
            }

            // Process FM1 inflection points
            if (curveData1 != null && curveData1.isValid()) {
                for (InflectionPoint ip : curveData1.inflectionPoints) {
                    LabelPosition lp = createLabelPosition(ip, fm, chartW, chartH, panelW, panelH,
                        powerRange, curveData1.powerCurve, 1);
                    if (lp != null) labels.add(lp);
                }
            }

            // Resolve collisions
            resolveCollisions(labels, panelW, panelH, fm.getHeight());

            // Draw all markers and labels
            Stroke defaultStroke = g2d.getStroke();
            for (LabelPosition lp : labels) {
                // Glow halo
                g2d.setColor(new Color(lp.color.getRed(), lp.color.getGreen(), lp.color.getBlue(), 80));
                g2d.fillOval(lp.markerX - 10, lp.markerY - 10, 20, 20);

                // Solid marker dot
                g2d.setColor(lp.color);
                g2d.fillOval(lp.markerX - 6, lp.markerY - 6, 12, 12);

                // Label background
                g2d.setColor(new Color(30, 30, 35, 200));
                g2d.fillRoundRect(lp.labelX - 4, lp.labelY - lp.labelHeight + 3,
                    lp.labelWidth + 8, lp.labelHeight + 2, 4, 4);

                // Label text
                g2d.setColor(lp.color);
                g2d.drawString(lp.text, lp.labelX, lp.labelY);

                // Dashed horizontal line to Y-axis
                g2d.setStroke(new BasicStroke(1f, BasicStroke.CAP_BUTT, BasicStroke.JOIN_MITER,
                    10f, new float[]{4f, 4f}, 0f));
                g2d.setColor(new Color(lp.color.getRed(), lp.color.getGreen(), lp.color.getBlue(), 100));
                g2d.drawLine(MARGIN, lp.markerY, lp.markerX - 6, lp.markerY);
                g2d.setStroke(defaultStroke);
            }
        }

        /**
         * Creates a LabelPosition for an inflection point.
         */
        private LabelPosition createLabelPosition(InflectionPoint ip, FontMetrics fm,
                int chartW, int chartH, int panelW, int panelH,
                double powerRange, double[] powerCurve, int curveIndex) {

            if (ip.altitudeM > MAX_DISPLAY_ALT) return null;

            LabelPosition lp = new LabelPosition();
            lp.ip = ip;
            lp.color = ip.color;
            lp.curveIndex = curveIndex;
            lp.labelHeight = fm.getHeight();

            // Map to pixel coordinates
            lp.markerX = MARGIN + (int) ((ip.power - displayMinPower) / powerRange * chartW);
            lp.markerY = MARGIN + chartH - (int) ((double) ip.altitudeM / MAX_DISPLAY_ALT * chartH);

            // Determine if valley (label goes left) or peak (label goes right)
            int altIdx = (int) (ip.altitudeM / ALT_STEP);
            int lookback = Math.max(altIdx - 2, 0);
            int lookahead = Math.min(altIdx + 2, powerCurve.length - 1);
            boolean fallingBefore = powerCurve[altIdx] < powerCurve[lookback];
            boolean risingAfter = powerCurve[lookahead] > powerCurve[altIdx];
            boolean isValley = fallingBefore && risingAfter;

            // Label text
            lp.text = String.format("%s: %.0fhp / %dm", ip.label, ip.power, ip.altitudeM);
            lp.labelWidth = fm.stringWidth(lp.text);

            // Initial position
            lp.labelY = lp.markerY - 10;
            if (isValley) {
                lp.labelX = lp.markerX - lp.labelWidth - 14;
                lp.isLeftSide = true;
            } else {
                lp.labelX = lp.markerX + 14;
                lp.isLeftSide = false;
            }

            // Boundary clamping
            if (lp.labelX + lp.labelWidth > panelW - 4) {
                lp.labelX = lp.markerX - lp.labelWidth - 14;
                lp.isLeftSide = true;
            }
            if (lp.labelX < 4) lp.labelX = 4;
            if (lp.labelY - lp.labelHeight + 3 < 4) lp.labelY = lp.labelHeight + 1;
            if (lp.labelY + 2 > panelH - 4) lp.labelY = panelH - 6;

            return lp;
        }

        /**
         * Resolves label collisions by adjusting positions.
         */
        private void resolveCollisions(List<LabelPosition> labels, int panelW, int panelH, int labelH) {
            if (labels.size() < 2) return;

            // Sort by Y coordinate (altitude)
            labels.sort((a, b) -> Integer.compare(a.markerY, b.markerY));

            // Check each pair for collisions
            for (int i = 0; i < labels.size(); i++) {
                LabelPosition lp1 = labels.get(i);
                Rectangle r1 = lp1.getBounds();

                for (int j = i + 1; j < labels.size(); j++) {
                    LabelPosition lp2 = labels.get(j);
                    Rectangle r2 = lp2.getBounds();

                    if (r1.intersects(r2)) {
                        // Collision detected - try to resolve

                        // Strategy 1: Flip FM1 label to other side (if it's FM1)
                        if (lp2.curveIndex == 1 && !triedFlip(lp2)) {
                            lp2.flipSide();
                            // Clamp to panel bounds
                            if (lp2.labelX + lp2.labelWidth > panelW - 4) {
                                lp2.labelX = panelW - 4 - lp2.labelWidth;
                            }
                            if (lp2.labelX < 4) lp2.labelX = 4;

                            Rectangle r2New = lp2.getBounds();
                            if (!r1.intersects(r2New)) {
                                continue;  // Resolved
                            }
                        }

                        // Strategy 2: Offset Y position
                        int overlap = r1.y + r1.height - r2.y;
                        if (overlap > 0) {
                            lp2.offsetY(overlap + 5);
                            // Clamp
                            if (lp2.labelY + 2 > panelH - 4) {
                                lp2.labelY = panelH - 6;
                            }
                        }
                    }
                }
            }
        }

        /** Marker for whether we've already tried flipping this label. */
        private boolean triedFlip(LabelPosition lp) {
            // Simple heuristic: if it's already on left side, we've tried
            return lp.isLeftSide;
        }

        /**
         * Draws the legend for dual curve mode.
         */
        private void drawLegend(Graphics2D g2d, int chartW, int chartH) {
            g2d.setFont(Application.defaultFont.deriveFont(Font.PLAIN, 11f));
            FontMetrics fm = g2d.getFontMetrics();

            int legendX = MARGIN + chartW - 200;
            int legendY = MARGIN + 20;
            int lineLen = 30;
            int spacing = 10;

            // Background
            g2d.setColor(new Color(30, 30, 35, 220));
            g2d.fillRoundRect(legendX - 10, legendY - 15, 190, 50, 6, 6);

            // FM0 entry
            g2d.setColor(CURVE_COLOR_FM0);
            g2d.setStroke(new BasicStroke(2.5f));
            g2d.drawLine(legendX, legendY, legendX + lineLen, legendY);
            g2d.setColor(Color.WHITE);
            g2d.drawString(fm0Name, legendX + lineLen + spacing, legendY + 4);

            // FM1 entry
            int y2 = legendY + 22;
            g2d.setColor(CURVE_COLOR_FM1);
            g2d.drawLine(legendX, y2, legendX + lineLen, y2);
            g2d.setColor(Color.WHITE);
            g2d.drawString(fm1Name, legendX + lineLen + spacing, y2 + 4);
        }
    }
}
