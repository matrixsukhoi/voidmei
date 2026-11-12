package ui.component;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Dimension;
import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Graphics2D;

import prog.Application;
import ui.UIBaseElements;

/**
 * High-performance Compass Gauge component.
 * Extracted from MinimalHUD.
 */
public class CompassGauge extends AbstractHUDComponent {

    private int radius;
    private BasicStroke outBs;
    private BasicStroke inBs;
    private int cachedStrokeWidth = -1;

    // Static/Style context
    private int lineWidth;
    private int HUDFontSize;
    private int HUDFontSizeSmall;
    private java.awt.Font fontSmall;
    private Font fontDistance; // 机场距离数字
    private Font fontNorth;    // 正北 N

    // State
    private float compassRads;
    private int compassDx;
    private int compassDy;
    private String lineCompass;
    private String lineLoc;

    // Coordinate system mode (logic inverted to match AttitudeIndicatorGauge):
    //   false (config OFF, default) = 离体/Inertial: pointer rotates, North fixed at top
    //   true  (config ON)           = 随体/Body-fixed: aircraft fixed at top, North rotates
    private boolean inertialMode = false;

    // 机场指示状态 (所有飞机场; 数据由 HUDCalculator 预计算)
    private boolean airfieldAligned;
    private ui.overlay.model.HUDData.AirfieldMarkUI[] airfields = EMPTY_AIRFIELDS;
    private boolean previewAirfield = false;

    private static final ui.overlay.model.HUDData.AirfieldMarkUI[] EMPTY_AIRFIELDS =
            new ui.overlay.model.HUDData.AirfieldMarkUI[0];

    // Pre-allocated arrays for triangle/aircraft drawing (zero-GC)
    private int[] triangleXPoints = new int[3];
    private int[] triangleYPoints = new int[3];
    private int[] airfieldXPoints = new int[3];
    private int[] airfieldYPoints = new int[3];
    private int[] planeXPoints = new int[7];
    private int[] planeYPoints = new int[7];

    // 距离标签避让 (零分配): 候选矩形/排序/已接受矩形/障碍矩形(N、航向、网格)
    private final int[] labelX = new int[16];
    private final int[] labelY = new int[16];
    private final int[] labelW = new int[16];
    private final int[] labelH = new int[16];
    private final int[] labelBaseline = new int[16];
    private final int[] labelMarkIdx = new int[16];
    private final double[] labelDist = new double[16];
    private final int[] labelOrder = new int[16];
    private final int[] accX = new int[16];
    private final int[] accY = new int[16];
    private final int[] accW = new int[16];
    private final int[] accH = new int[16];
    private int acceptedLabelCount = 0;
    private final int[] obsX = new int[3];
    private final int[] obsY = new int[3];
    private final int[] obsW = new int[3];
    private final int[] obsH = new int[3];

    // Thin stroke for triangle border (1 pixel)
    private static final BasicStroke THIN_STROKE = new BasicStroke(1);

    // 航向飞机图标局部坐标: 前向 f, 侧向 l (机头朝 +f)
    private static final double[] PLANE_F = { 0.50, 0.00, 0.05, -0.50, -0.50, 0.05, 0.00 };
    private static final double[] PLANE_L = { 0.00, -0.50, -0.18, -0.35, 0.35, 0.18, 0.50 };

    public CompassGauge(int radius) {
        this.radius = radius;
        this.lineCompass = "";
        this.lineLoc = "";
    }

    @Override
    public String getId() {
        return "gauge.compass";
    }

    @Override
    public Dimension getPreferredSize() {
        // Radius * 2 for circle. Removed excess padding to fit layout tightly.
        return new Dimension(radius * 2, radius * 2);
    }

    public void setStyleContext(int radius, int lineWidth, int HUDFontSize, int HUDFontSizeSmall,
            java.awt.Font fontSmall) {
        this.radius = radius;
        this.lineWidth = lineWidth;
        this.HUDFontSize = HUDFontSize;
        this.HUDFontSizeSmall = HUDFontSizeSmall;
        this.fontSmall = fontSmall;
        // 派生字体在配置更新时算一次, 保证 draw() 零分配
        this.fontDistance = fontSmall != null ? fontSmall.deriveFont(fontSmall.getSize2D() * 0.85f) : null;
        this.fontNorth = fontSmall != null ? fontSmall.deriveFont(fontSmall.getSize2D() * 0.78f) : null;
    }

    /**
     * Sets the coordinate system mode for the compass.
     * Note: Logic is inverted to match AttitudeIndicatorGauge behavior.
     * @param inertial false (default) = 离体/Inertial mode: pointer rotates to show heading, North fixed at top;
     *                 true = 随体/Body-fixed mode: aircraft symbol fixed at top, North triangle rotates
     */
    public void setInertialMode(boolean inertial) {
        this.inertialMode = inertial;
    }

    /** 预览模式无实时数据, 用演示机场标记体现开关/样式 */
    public void setPreviewAirfield(boolean preview) {
        this.previewAirfield = preview;
    }

    @Override
    public void onDataUpdate(ui.overlay.model.HUDData data) {
        if (data == null)
            return;

        // 1. Heading Logic
        this.compassRads = (float) Math.toRadians(data.heading);
        // Render Logic moved from MinimalHUD:
        // compassDx = (int) ((ctx.roundCompass * 1.3f) * Math.sin(compassRads));
        // Use 'radius' here which is same as 'roundCompass'
        this.compassDx = (int) ((this.radius * 1.3f) * Math.sin(compassRads));
        this.compassDy = (int) ((this.radius * 1.3f) * Math.cos(compassRads));

        this.lineCompass = String.format("%3.0f", data.heading);

        // 2. Location
        this.lineLoc = data.mapGrid;

        // 3. 所有飞机场 (罗盘环上方位标记 + 距离)
        this.airfields = data.airfields != null ? data.airfields : EMPTY_AIRFIELDS;
        this.airfieldAligned = data.airfieldAligned;
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {

        // Cache strokes based on lineWidth
        if (lineWidth != cachedStrokeWidth) {
            outBs = new BasicStroke(lineWidth + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            inBs = new BasicStroke(lineWidth, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
            cachedStrokeWidth = lineWidth;
        }

        int r = radius;
        int centerX = x + r;
        int centerY = y + r;
        acceptedLabelCount = 0;

        // Layer 1 (bottom): Draw North triangle first
        // Logic inverted to match AttitudeIndicatorGauge behavior
        if (inertialMode) {
            // Config ON: Body-fixed mode - rotating North triangle
            drawNorthTriangle(g2d, centerX, centerY, r, -compassRads);
        } else {
            // Config OFF: Inertial mode - fixed North triangle at 12 o'clock
            drawNorthTriangle(g2d, centerX, centerY, r, 0);
        }

        // Layer 2: Draw the compass circle
        g2d.setStroke(outBs);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawOval(x, y, r + r, r + r);

        g2d.setStroke(inBs);
        g2d.setColor(Application.colorNum);
        g2d.drawOval(x, y, r + r, r + r);

        boolean blinkOn = (System.currentTimeMillis() / 400) % 2 == 0;
        // 机场方位标记先画(友方→敌方), 让航向线盖在它上面
        if (previewAirfield) {
            drawAirfieldMarker(g2d, centerX, centerY, r, inertialMode ? -90 : 270, Application.colorLabel);
            drawAirfieldMarker(g2d, centerX, centerY, r, inertialMode ? 45 : 135, Application.colorWarningStrong);
        } else {
            for (int pass = 0; pass < 2; pass++) {
                boolean enemyPass = pass == 1;
                for (ui.overlay.model.HUDData.AirfieldMarkUI m : airfields) {
                    if (m.enemy != enemyPass || m.label == null || m.label.isEmpty())
                        continue;
                    drawAirfieldMarker(g2d, centerX, centerY, r, markAngle(m), markColor(m, blinkOn));
                }
            }
        }

        // 机头对准最近友方机场时, 最长的航向指针线变绿(回场指示)
        Color headingColor = airfieldAligned ? Application.colorGo : Application.colorNum;

        // Layer 3 (top): Draw heading pointer or aircraft line
        // Logic inverted to match AttitudeIndicatorGauge behavior
        if (inertialMode) {
            // Config ON: Body-fixed mode - aircraft symbol fixed at 12 o'clock
            int fixedTipY = centerY - (int) (0.618 * r);
            int fixedEndY = centerY - (int) (1.3 * r);

            g2d.setStroke(outBs);
            g2d.setColor(Application.colorShadeShape);
            g2d.drawLine(centerX, fixedTipY, centerX, fixedEndY);

            g2d.setStroke(inBs);
            g2d.setColor(headingColor);
            g2d.drawLine(centerX, fixedTipY, centerX, fixedEndY);
            drawHeadingPlane(g2d, centerX, centerY, r, 0, headingColor);

        } else {
            // Config OFF: Inertial mode - pointer rotates to show heading
            int tipX = centerX + (int) (0.618 * r * Math.sin(compassRads));
            int tipY = centerY - (int) (0.618 * r * Math.cos(compassRads));
            int endX = centerX + compassDx;
            int endY = centerY - compassDy;

            g2d.setStroke(outBs);
            g2d.setColor(Application.colorShadeShape);
            g2d.drawLine(tipX, tipY, endX, endY);

            g2d.setStroke(inBs);
            g2d.setColor(headingColor);
            g2d.drawLine(tipX, tipY, endX, endY);
            drawHeadingPlane(g2d, centerX, centerY, r, compassRads, headingColor);
        }

        // Draw text labels (same for both modes)
        if (fontSmall != null) {
            UIBaseElements.drawStringShade(g2d, x + lineWidth + 3, y + HUDFontSize - (r - HUDFontSize) / 2, 1,
                    lineCompass, fontSmall);
            UIBaseElements.drawStringShade(g2d, x + lineWidth + 3, y + r + HUDFontSizeSmall / 2 + HUDFontSize, 1,
                    lineLoc,
                    fontSmall);
        }

        // 距离数字最后画, 压在静止文字之上可读
        if (previewAirfield) {
            drawAirfieldLabel(g2d, centerX, centerY, r, inertialMode ? -90 : 270, "74", Application.colorLabel);
            drawAirfieldLabel(g2d, centerX, centerY, r, inertialMode ? 45 : 135, "21",
                    Application.colorWarningStrong);
        } else {
            drawAirfieldLabels(g2d, centerX, centerY, r, x, y, blinkOn);
        }

        // 正北 N 最后画: 与任何已显示的距离数字重叠时隐藏, 距离优先
        drawNorthLabelIfClear(g2d, centerX, centerY, r);
    }

    /**
     * 所有机场距离标签: 按距离从近到远绘制; 与已绘制标签/N/航向/网格重叠的次级标签直接跳过。
     * 重叠时保留最近的那个(不区分敌我)。
     */
    private void drawAirfieldLabels(Graphics2D g2d, int centerX, int centerY, int r,
            int x, int y, boolean blinkOn) {
        acceptedLabelCount = 0;
        int n = 0;
        for (int i = 0; i < airfields.length && n < labelX.length; i++) {
            ui.overlay.model.HUDData.AirfieldMarkUI m = airfields[i];
            if (m.label == null || m.label.isEmpty())
                continue;
            computeAirfieldLabelRect(g2d, centerX, centerY, r, markAngle(m), m.label, n);
            labelMarkIdx[n] = i;
            labelDist[n] = m.distKm;
            labelOrder[n] = n;
            n++;
        }
        if (n == 0)
            return;

        // 插入排序: 最近的标签优先
        for (int i = 1; i < n; i++) {
            int key = labelOrder[i];
            int j = i - 1;
            while (j >= 0 && labelDist[labelOrder[j]] > labelDist[key]) {
                labelOrder[j + 1] = labelOrder[j];
                j--;
            }
            labelOrder[j + 1] = key;
        }

        int obsCount = buildObstacleRects(g2d, centerX, centerY, r, x, y);
        int accCount = 0;
        for (int k = 0; k < n; k++) {
            int idx = labelOrder[k];
            if (overlapsObstacle(idx, obsCount) || overlapsAccepted(idx, accCount))
                continue;
            ui.overlay.model.HUDData.AirfieldMarkUI m = airfields[labelMarkIdx[idx]];
            UIBaseElements.__drawStringShade(g2d, labelX[idx], labelBaseline[idx], 1, m.label,
                    fontDistance, markColor(m, blinkOn));
            accX[accCount] = labelX[idx];
            accY[accCount] = labelY[idx];
            accW[accCount] = labelW[idx];
            accH[accCount] = labelH[idx];
            accCount++;
        }
        acceptedLabelCount = accCount;
    }

    private void computeAirfieldLabelRect(Graphics2D g2d, int centerX, int centerY, int r,
            double deg, String label, int slot) {
        double rad = Math.toRadians(deg);
        double sin = Math.sin(rad), cos = Math.cos(rad);
        FontMetrics fm = g2d.getFontMetrics(fontDistance);
        int tw = fm.stringWidth(label);
        int th = fm.getHeight();
        double gap = Math.max(3, r * 0.12);
        double extent = Math.abs(sin) * tw * 0.5 + Math.abs(cos) * th * 0.5;
        double anchorR = r + gap + extent;
        int ax = centerX + (int) Math.round(sin * anchorR);
        int ay = centerY - (int) Math.round(cos * anchorR);
        labelX[slot] = ax - tw / 2;
        labelW[slot] = tw;
        labelH[slot] = th;
        labelBaseline[slot] = ay + fm.getAscent() / 2 - 1;
        labelY[slot] = labelBaseline[slot] - fm.getAscent();
    }

    private int buildObstacleRects(Graphics2D g2d, int centerX, int centerY, int r, int x, int y) {
        int n = 0;
        // 正北 N 不作为障碍: 机场在正北时距离数字允许压住 N
        // 环内航向/网格文字
        if (fontSmall != null) {
            FontMetrics fm = g2d.getFontMetrics(fontSmall);
            int tx = x + lineWidth + 3;
            int headingBase = y + HUDFontSize - (r - HUDFontSize) / 2;
            obsX[n] = tx;
            obsY[n] = headingBase - fm.getAscent();
            obsW[n] = fm.stringWidth(lineCompass);
            obsH[n] = fm.getHeight();
            n++;
            int gridBase = y + r + HUDFontSizeSmall / 2 + HUDFontSize;
            obsX[n] = tx;
            obsY[n] = gridBase - fm.getAscent();
            obsW[n] = fm.stringWidth(lineLoc);
            obsH[n] = fm.getHeight();
            n++;
        }
        return n;
    }

    private boolean overlapsObstacle(int labelIdx, int obstacleCount) {
        for (int i = 0; i < obstacleCount; i++) {
            if (rectsOverlap(labelX[labelIdx], labelY[labelIdx], labelW[labelIdx], labelH[labelIdx],
                    obsX[i], obsY[i], obsW[i], obsH[i]))
                return true;
        }
        return false;
    }

    private boolean overlapsAccepted(int labelIdx, int acceptedCount) {
        for (int i = 0; i < acceptedCount; i++) {
            if (rectsOverlap(labelX[labelIdx], labelY[labelIdx], labelW[labelIdx], labelH[labelIdx],
                    accX[i], accY[i], accW[i], accH[i]))
                return true;
        }
        return false;
    }

    private static boolean rectsOverlap(int ax, int ay, int aw, int ah,
            int bx, int by, int bw, int bh) {
        return ax < bx + bw + 2 && bx < ax + aw + 2 && ay < by + bh + 2 && by < ay + ah + 2;
    }

    private double markAngle(ui.overlay.model.HUDData.AirfieldMarkUI m) {
        return inertialMode ? m.relDeg : m.bearingDeg;
    }

    private static Color markColor(ui.overlay.model.HUDData.AirfieldMarkUI m, boolean blinkOn) {
        if (!m.enemy)
            return Application.colorLabel;
        // 敌方接近告警距离时红↔主题色闪烁, 平时红色常亮
        return m.alert && !blinkOn ? Application.colorLabel : Application.colorWarningStrong;
    }

    /** 环上方位标记: 底边压环、尖端朝内的小三角 */
    private void drawAirfieldMarker(Graphics2D g2d, int centerX, int centerY, int r,
            double deg, Color color) {
        double rad = Math.toRadians(deg);
        double sin = Math.sin(rad), cos = Math.cos(rad);
        double tipR = r - Math.max(3, Math.round(r * 0.15f));
        double baseR = r + 1;
        int halfBase = Math.max(3, Math.round(r * 0.11f));
        airfieldXPoints[0] = centerX + (int) Math.round(sin * tipR);
        airfieldYPoints[0] = centerY - (int) Math.round(cos * tipR);
        int bx = centerX + (int) Math.round(sin * baseR);
        int by = centerY - (int) Math.round(cos * baseR);
        airfieldXPoints[1] = bx + (int) Math.round(cos * halfBase);
        airfieldYPoints[1] = by + (int) Math.round(sin * halfBase);
        airfieldXPoints[2] = bx - (int) Math.round(cos * halfBase);
        airfieldYPoints[2] = by - (int) Math.round(sin * halfBase);
        g2d.setColor(color);
        g2d.fillPolygon(airfieldXPoints, airfieldYPoints, 3);
        g2d.setColor(Application.colorShadeShape);
        g2d.setStroke(THIN_STROKE);
        g2d.drawPolygon(airfieldXPoints, airfieldYPoints, 3);
    }

    /** 距离数字贴环外侧对齐(左半边右对齐环边, 右半边左对齐), 随标记转动 */
    private void drawAirfieldLabel(Graphics2D g2d, int centerX, int centerY, int r,
            double deg, String label, Color color) {
        double rad = Math.toRadians(deg);
        double sin = Math.sin(rad), cos = Math.cos(rad);
        FontMetrics fm = g2d.getFontMetrics(fontDistance);
        int tw = fm.stringWidth(label);
        int th = fm.getHeight();
        double gap = Math.max(3, r * 0.12);
        double extent = Math.abs(sin) * tw * 0.5 + Math.abs(cos) * th * 0.5;
        double anchorR = r + gap + extent;
        int ax = centerX + (int) Math.round(sin * anchorR);
        int ay = centerY - (int) Math.round(cos * anchorR);
        UIBaseElements.__drawStringShade(g2d, ax - tw / 2, ay + fm.getAscent() / 2 - 1, 1, label, fontDistance, color);
    }

    /** 正北 N: 与已显示的距离数字重叠时隐藏(距离优先), 否则画在北向三角外侧 */
    private void drawNorthLabelIfClear(Graphics2D g2d, int centerX, int centerY, int r) {
        if (fontNorth == null)
            return;
        FontMetrics fm = g2d.getFontMetrics(fontNorth);
        int tw = fm.stringWidth("N");
        double nr = r * 1.7;
        double angle = inertialMode ? -compassRads : 0;
        int x = centerX + (int) Math.round(Math.sin(angle) * nr) - tw / 2;
        int baseline = centerY - (int) Math.round(Math.cos(angle) * nr) + fm.getAscent() / 2 - 1;
        int y = baseline - fm.getAscent();
        int w = tw;
        int h = fm.getHeight();
        for (int i = 0; i < acceptedLabelCount; i++) {
            if (rectsOverlap(x, y, w, h, accX[i], accY[i], accW[i], accH[i]))
                return;
        }
        UIBaseElements.__drawStringShade(g2d, x, baseline, 1, "N", fontNorth, Application.colorUnit);
    }

    /** 航向线末端的小飞机轮廓(机头朝外), 让航向可视化更直观 */
    private void drawHeadingPlane(Graphics2D g2d, int centerX, int centerY, int r,
            double angleRad, Color color) {
        double length = r * 0.55;
        double span = r * 0.5;
        double sf = Math.sin(angleRad), cf = Math.cos(angleRad);
        double baseF = r * 1.3;
        for (int i = 0; i < PLANE_F.length; i++) {
            double f = baseF + PLANE_F[i] * length;
            double l = PLANE_L[i] * span;
            planeXPoints[i] = centerX + (int) Math.round(f * sf + l * cf);
            planeYPoints[i] = centerY - (int) Math.round(f * cf - l * sf);
        }
        g2d.setColor(color);
        g2d.fillPolygon(planeXPoints, planeYPoints, PLANE_F.length);
        g2d.setColor(Application.colorShadeShape);
        g2d.setStroke(THIN_STROKE);
        g2d.drawPolygon(planeXPoints, planeYPoints, PLANE_F.length);
    }

    /**
     * Draws a north indicator triangle at the specified angle from center.
     * The triangle points outward away from the center.
     *
     * @param g2d     Graphics context
     * @param centerX Center X of the compass
     * @param centerY Center Y of the compass
     * @param radius  Radius of the compass
     * @param angleRads Angle in radians (0 = 12 o'clock / North)
     */
    private void drawNorthTriangle(Graphics2D g2d, int centerX, int centerY, int radius, double angleRads) {
        // Triangle size: height is about half the aircraft line length
        int triangleHeight = (int) (radius * 0.35);
        int triangleHalfBase = (int) (radius * 0.30);

        // Triangle tip points outward, base sits on the circle edge
        // Tip is outside the circle
        double tipDist = radius + triangleHeight;
        int tipX = centerX + (int) (tipDist * Math.sin(angleRads));
        int tipY = centerY - (int) (tipDist * Math.cos(angleRads));

        // Base center sits on the circle edge
        int baseX = centerX + (int) (radius * Math.sin(angleRads));
        int baseY = centerY - (int) (radius * Math.cos(angleRads));

        // Perpendicular direction (tangent to circle): use (cos(θ), sin(θ))
        int corner1X = baseX + (int) (triangleHalfBase * Math.cos(angleRads));
        int corner1Y = baseY + (int) (triangleHalfBase * Math.sin(angleRads));
        int corner2X = baseX - (int) (triangleHalfBase * Math.cos(angleRads));
        int corner2Y = baseY - (int) (triangleHalfBase * Math.sin(angleRads));

        // Fill triangle arrays (reuse pre-allocated arrays for zero-GC)
        triangleXPoints[0] = tipX;
        triangleXPoints[1] = corner1X;
        triangleXPoints[2] = corner2X;

        triangleYPoints[0] = tipY;
        triangleYPoints[1] = corner1Y;
        triangleYPoints[2] = corner2Y;

        // Draw filled triangle using unit color
        g2d.setColor(Application.colorUnit);
        g2d.fillPolygon(triangleXPoints, triangleYPoints, 3);

        // Draw thin border on top using shade color
        g2d.setColor(Application.colorShadeShape);
        g2d.setStroke(THIN_STROKE);
        g2d.drawPolygon(triangleXPoints, triangleYPoints, 3);
    }
}
