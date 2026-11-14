package ui.minihud;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Graphics2D;
import java.awt.geom.Path2D;
import java.awt.image.BufferedImage;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;

import parser.Airfield;
import prog.Application;
import ui.UIBaseElements;
import ui.overlay.MinimalHUDContext;

/**
 * 机场距离绘制器，由 {@link ui.minihud.widget.AirfieldWidget} 包成画布组件。
 * 组件位置、缩放、图层与持久化全部走自由画布文档，不再由 overlay 特殊定位。
 *
 * 只显示跑道长度超过 600m 的机场（过滤 250m/500m 直升机停机坪），
 * 友军/敌军颜色、箭头指向、全部/最近、地图区域、告警距离均可配置。
 */
public final class MiniHudAirfieldPanel {

    private static final double MIN_RUNWAY_METERS = 600.0;
    private static final String PREFIX = "AF ";
    private static final Color DEFAULT_FRIENDLY = new Color(27, 255, 128, 240);
    private static final Color DEFAULT_ENEMY = new Color(255, 68, 68, 240);
    /** 敌军告警方框：闪烁绘制，不改文字颜色。 */
    private static final Color WARN_BOX_COLOR = new Color(255, 190, 40, 255);

    private static final BufferedImage METRICS_IMG =
            new BufferedImage(1, 1, BufferedImage.TYPE_INT_ARGB);
    private static final Graphics2D METRICS = METRICS_IMG.createGraphics();

    private static final class Row {
        final String text;
        final Color color;
        final double bearingDeg;
        final boolean alert;

        Row(String text, Color color, double bearingDeg, boolean alert) {
            this.text = text;
            this.color = color;
            this.bearingDeg = bearingDeg;
            this.alert = alert;
        }
    }

    private final List<Row> rows = new ArrayList<>();
    /** 复用路径，避免每帧为箭头分配几何对象。 */
    private final Path2D.Float arrowPath = new Path2D.Float();

    private List<Airfield> lastSource;
    private Font font;
    private BasicStroke arrowStroke;
    private int lineHeight;
    private int gap;
    private int width;
    private int height;

    private boolean showArrow = true;
    private boolean showAll = true;
    private boolean showGrid = true;
    private Color friendlyColor = DEFAULT_FRIENDLY;
    private Color enemyColor = DEFAULT_ENEMY;
    private double warnDistanceKm;
    private double heading;
    private int arrowWidth;
    private int arrowGap;

    /** 面板外观/行为配置；字体/行高随 MiniHUD 配置刷新。 */
    public void configure(MinimalHUDContext ctx, boolean showArrow, boolean showAll, boolean showGrid,
            Color friendlyColor, Color enemyColor, double warnDistanceKm) {
        if (ctx == null)
            return;
        this.font = ctx.drawFontSmall;
        this.lineHeight = Math.max(1, ctx.hudFontSize);
        this.gap = Math.max(2, lineHeight / 6);
        this.arrowStroke = ctx.strokeThin;
        this.showArrow = showArrow;
        this.showAll = showAll;
        this.showGrid = showGrid;
        this.friendlyColor = friendlyColor != null ? friendlyColor : DEFAULT_FRIENDLY;
        this.enemyColor = enemyColor != null ? enemyColor : DEFAULT_ENEMY;
        this.warnDistanceKm = Math.max(0, warnDistanceKm);
        this.arrowWidth = showArrow ? Math.max(8, lineHeight - 2) : 0;
        this.arrowGap = showArrow ? Math.max(2, lineHeight / 4) : 0;

        // 选项影响文本/过滤/宽度，立即用上一份数据重建
        List<Airfield> src = lastSource;
        lastSource = null;
        if (src != null)
            update(src, heading);
        else
            remeasure();
    }

    /**
     * 用最新机场快照重建行。
     *
     * @return 几何（可见性/宽高）是否变化，调用方据此决定是否重新适配窗口
     */
    public boolean update(List<Airfield> source, double heading) {
        this.heading = heading;
        if (source == lastSource)
            return false;
        lastSource = source;

        boolean wasVisible = isVisible();
        int oldWidth = width;
        int oldHeight = height;

        rows.clear();
        if (source != null && !source.isEmpty()) {
            List<Airfield> filtered = new ArrayList<>(source.size());
            for (Airfield a : source) {
                if (a != null && a.lengthMeters > MIN_RUNWAY_METERS)
                    filtered.add(a);
            }
            Collections.sort(filtered, Comparator.comparingDouble(a -> a.distanceMeters));
            if (!showAll && filtered.size() > 1)
                filtered = filtered.subList(0, 1);
            for (Airfield a : filtered) {
                StringBuilder sb = new StringBuilder(PREFIX);
                if (a.distanceMeters >= 1000.0)
                    sb.append(String.format("%.1fkm", a.distanceMeters / 1000.0));
                else
                    sb.append(String.format("%.0fm", Math.max(0, a.distanceMeters)));
                if (showGrid && a.mapGrid != null && !a.mapGrid.isEmpty())
                    sb.append(' ').append(a.mapGrid);
                // 友军机场不告警；只有敌军机场进入告警距离才亮方框
                boolean alert = !a.friendly && warnDistanceKm > 0
                        && a.distanceMeters <= warnDistanceKm * 1000.0;
                rows.add(new Row(sb.toString(), a.friendly ? friendlyColor : enemyColor,
                        a.bearingDeg, alert));
            }
        }
        remeasure();
        return wasVisible != isVisible() || oldWidth != width || oldHeight != height;
    }

    private void remeasure() {
        width = 0;
        height = 0;
        if (rows.isEmpty() || font == null)
            return;
        FontMetrics fm = METRICS.getFontMetrics(font);
        int maxW = lineHeight * 3;
        for (Row r : rows)
            maxW = Math.max(maxW, fm.stringWidth(r.text));
        width = maxW + 2 + (showArrow ? arrowWidth + arrowGap : 0);
        height = rows.size() * lineHeight + Math.max(0, rows.size() - 1) * gap;
    }

    public boolean isVisible() {
        return !rows.isEmpty() && font != null;
    }

    /** 面板宽度（像素）。 */
    public int getPanelWidth() {
        return width;
    }

    /** 面板高度（像素）。 */
    public int getPanelHeight() {
        return height;
    }

    public void paint(Graphics2D g2d, int x, int y) {
        if (!isVisible())
            return;
        int ascent = METRICS.getFontMetrics(font).getAscent();
        boolean blinkOn = (System.currentTimeMillis() / 400) % 2 == 0;
        int rowY = y;
        for (Row r : rows) {
            int textX = x;
            if (showArrow) {
                drawArrow(g2d, x + arrowWidth / 2, rowY + lineHeight / 2,
                        r.bearingDeg - heading, r.color);
                textX += arrowWidth + arrowGap;
            }
            UIBaseElements.__drawStringShade(g2d, textX, rowY + ascent, 1, r.text, font, r.color);
            // 告警：文字颜色不变，整行方框闪烁
            if (r.alert && blinkOn) {
                g2d.setStroke(arrowStroke);
                g2d.setColor(WARN_BOX_COLOR);
                g2d.drawRect(x, rowY, Math.max(1, width - 1), Math.max(1, lineHeight - 1));
            }
            rowY += lineHeight + gap;
        }
    }

    /** 指向机场的矢量箭头；方向 = 机场世界方位 - 当前航向。 */
    private void drawArrow(Graphics2D g2d, int cx, int cy, double relativeDeg, Color color) {
        double rad = Math.toRadians(relativeDeg);
        double dirX = Math.sin(rad);
        double dirY = -Math.cos(rad);
        float len = Math.max(5, lineHeight - 2);
        float half = len / 2f;
        // 全程用浮点几何，避免整数四舍五入让箭头底边中点偏离柄轴线
        float tailX = cx - (float) (dirX * half);
        float tailY = cy - (float) (dirY * half);
        float tipX = cx + (float) (dirX * half);
        float tipY = cy + (float) (dirY * half);

        float headLen = Math.max(3f, len / 3f);
        float baseX = tipX - (float) (dirX * headLen);
        float baseY = tipY - (float) (dirY * headLen);
        float halfW = Math.max(2f, len / 6f);
        float wingX = (float) (-dirY * halfW);
        float wingY = (float) (dirX * halfW);

        g2d.setStroke(arrowStroke);
        // 阴影柄止于箭头底边中点，与彩色柄完全同轴
        g2d.setColor(Application.colorShadeShape);
        arrowPath.reset();
        arrowPath.moveTo(tailX + 1f, tailY + 1f);
        arrowPath.lineTo(baseX + 1f, baseY + 1f);
        g2d.draw(arrowPath);

        g2d.setColor(color);
        arrowPath.reset();
        arrowPath.moveTo(tailX, tailY);
        arrowPath.lineTo(baseX, baseY);
        g2d.draw(arrowPath);

        // 等腰三角形：底边中点在柄的延长线上，两翼严格对称
        arrowPath.reset();
        arrowPath.moveTo(tipX, tipY);
        arrowPath.lineTo(baseX + wingX, baseY + wingY);
        arrowPath.lineTo(baseX - wingX, baseY - wingY);
        arrowPath.closePath();
        g2d.fill(arrowPath);
    }
}
