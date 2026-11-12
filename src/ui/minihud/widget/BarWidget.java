package ui.minihud.widget;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Dimension;
import java.awt.Font;
import java.awt.Graphics2D;
import java.util.function.DoubleSupplier;

import prog.Application;
import ui.component.AbstractHUDComponent;
import ui.component.HUDComponent;
import ui.component.LinearGauge;
import ui.minihud.MiniHudProps;
import ui.minihud.MiniHudWidget;
import ui.minihud.WidgetBuildContext;
import ui.model.TelemetrySource;
import ui.overlay.MinimalHUDContext;
import ui.overlay.model.HUDData;
import ui.util.FastNumberFormatter;
import ui.util.ReflectBinder;

/**
 * 原子条/量表组件：一个实例只画一根条（或一根条及其刻度标注）。
 * 数据来源：襟翼角、速度比、油门、AoA、机动、任意配置字段。
 */
public final class BarWidget extends AbstractHUDComponent implements HUDComponent {

    private static final int[] FLAP_TICKS = { 20, 33, 60, 100 };
    private static final int FLAP_MAX = 125;

    private final String id;
    private final MinimalHUDContext ctx;
    private final String source;
    private final int width;
    private final int height;
    private final int barWidth;
    private final int leftExtent;
    private final String orientation;
    private final String field;
    private final double min;
    private final double max;
    private final DoubleSupplier valueSupplier;
    private final Font tickFont;
    private final int mainAscent;
    private final LinearGauge throttleDelegate;
    private final BasicStroke thickTick = new BasicStroke(2);
    private final BasicStroke thinStroke = new BasicStroke(1, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
    private final char[] speedValueBuf = new char[8];

    private double flapCurrent;
    private double flapMaxSafe = 100;
    private double speedRatio;
    private double stallRatio;
    private double unitMachRatio;
    private double aileronLockRatio;
    private double rudderLockRatio;
    private double aoaRatio;
    private double maneuverIndex;
    private int throttleValue;
    private Color throttleColor = Application.colorNum;
    private double fieldValue;

    private Color aoaColor = Application.colorNum;

    public BarWidget(WidgetBuildContext c) {
        this.id = c.widget.id;
        this.ctx = c.ctx;
        this.source = c.widget.get(MiniHudProps.BAR_SOURCE, MiniHudProps.BAR_FIELD);
        this.orientation = c.widget.get(MiniHudProps.ORIENTATION, MiniHudProps.ORIENT_H);
        this.field = c.widget.get(MiniHudProps.FIELD, "");
        this.min = c.widget.getDouble(MiniHudProps.MIN, 0);
        this.max = c.widget.getDouble(MiniHudProps.MAX, 1);
        this.tickFont = ctx.drawFontSSmall;
        this.mainAscent = METRICS.getFontMetrics(ctx.drawFont).getAscent();
        double lengthUnits = c.widget.getDouble(MiniHudProps.LENGTH_UNITS, defaultLength(source));
        int length = Math.max(4, (int) Math.round(lengthUnits * ctx.hudFontSize));

        switch (source) {
            case MiniHudProps.BAR_FLAP_ANGLE:
                this.width = length;
                this.height = ctx.lineWidth + 2;
                this.barWidth = ctx.lineWidth + 2;
                this.leftExtent = 0;
                this.throttleDelegate = null;
                this.valueSupplier = null;
                break;
            case MiniHudProps.BAR_AOA:
                this.width = length;
                this.height = ctx.lineWidth + 3;
                this.barWidth = ctx.lineWidth + 3;
                this.leftExtent = 0;
                this.throttleDelegate = null;
                this.valueSupplier = null;
                break;
            case MiniHudProps.BAR_MANEUVER:
                this.width = length;
                this.height = ctx.hudFontSize;
                this.barWidth = 0;
                this.leftExtent = 0;
                this.throttleDelegate = null;
                this.valueSupplier = null;
                break;
            case MiniHudProps.BAR_SPEED_RATIO:
                // 与旧版一致：节点框只包含竖条，左侧读数画在框外、由窗口 padding 兜住
                this.leftExtent = ctx.hudFontSize;
                this.barWidth = Math.max(6, (int) (ctx.hudFontSize * 0.25));
                this.width = barWidth;
                this.height = length;
                this.throttleDelegate = null;
                this.valueSupplier = null;
                break;
            case MiniHudProps.BAR_THROTTLE:
                this.throttleDelegate = new LinearGauge("ThrottleBar", 110, true, false);
                this.throttleDelegate.setStyleContext(length, ctx.barWidth, ctx.drawFontSSmall, ctx.drawFontSSmall);
                this.leftExtent = ctx.drawFontSSmall.getSize();
                this.barWidth = ctx.barWidth;
                this.width = Math.max(1, throttleDelegate.getPreferredSize().width);
                this.height = length;
                this.valueSupplier = null;
                break;
            case MiniHudProps.BAR_FIELD:
            default:
                this.width = length;
                this.height = Math.max(3, (int) Math.round(
                        c.widget.getDouble(MiniHudProps.THICKNESS_UNITS, 0.25) * ctx.hudFontSize));
                this.barWidth = height;
                this.leftExtent = 0;
                this.throttleDelegate = null;
                this.valueSupplier = c.source != null && !field.isEmpty()
                        ? ReflectBinder.resolveDouble(c.source, field)
                        : null;
                break;
        }
    }

    private static double defaultLength(String source) {
        switch (source) {
            case MiniHudProps.BAR_FLAP_ANGLE:
                return 6.0;
            case MiniHudProps.BAR_SPEED_RATIO:
                return 5.5;
            case MiniHudProps.BAR_THROTTLE:
                return 4.8;
            case MiniHudProps.BAR_AOA:
                return 4.8332;
            case MiniHudProps.BAR_MANEUVER:
                return 5.5;
            default:
                return 4.0;
        }
    }

    @Override
    public String getId() {
        return id;
    }

    @Override
    public Dimension getPreferredSize() {
        return new Dimension(width, height);
    }

    /** 速度条左侧读数画在节点框外，命中区向左扩，便于点选。 */
    public int getHitPadLeft() {
        return MiniHudProps.BAR_SPEED_RATIO.equals(source) ? leftExtent : 0;
    }

    /** 配置字段条每帧更新。 */
    public void updateData(TelemetrySource source) {
        if (valueSupplier == null || source == null)
            return;
        double v = valueSupplier.getAsDouble();
        // 8111 退化帧哨兵值：条按 0 处理，避免画到满格/越界
        fieldValue = (Double.isNaN(v) || Double.isInfinite(v) || v == -65535) ? min : v;
    }

    @Override
    public void onDataUpdate(HUDData data) {
        if (data == null)
            return;
        switch (source) {
            case MiniHudProps.BAR_FLAP_ANGLE:
                flapCurrent = data.flaps;
                flapMaxSafe = data.flapAllowAngle;
                break;
            case MiniHudProps.BAR_SPEED_RATIO:
                speedRatio = data.speedBar_speedRatio;
                stallRatio = data.speedBar_stallRatio;
                unitMachRatio = data.speedBar_unitMachLimitRatio;
                aileronLockRatio = data.speedBar_aileronLockRatio;
                rudderLockRatio = data.speedBar_rudderLockRatio;
                break;
            case MiniHudProps.BAR_THROTTLE:
                throttleValue = data.throttle;
                throttleColor = data.throttleColor != null ? data.throttleColor : Application.colorNum;
                if (throttleDelegate != null) {
                    throttleDelegate.onDataUpdate(data);
                }
                break;
            case MiniHudProps.BAR_AOA:
                aoaRatio = data.aoaRatio;
                aoaColor = data.aoaBarColor != null ? data.aoaBarColor : Application.colorNum;
                break;
            case MiniHudProps.BAR_MANEUVER:
                maneuverIndex = data.maneuverIndex;
                break;
            default:
                break;
        }
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        switch (source) {
            case MiniHudProps.BAR_FLAP_ANGLE:
                drawFlapBar(g2d, x, y);
                break;
            case MiniHudProps.BAR_AOA:
                drawAoaBar(g2d, x, y);
                break;
            case MiniHudProps.BAR_MANEUVER:
                drawManeuverBar(g2d, x, y);
                break;
            case MiniHudProps.BAR_SPEED_RATIO:
                drawSpeedRatioBar(g2d, x, y);
                break;
            case MiniHudProps.BAR_THROTTLE:
                if (throttleDelegate != null)
                    throttleDelegate.draw(g2d, x, y);
                break;
            case MiniHudProps.BAR_FIELD:
            default:
                drawFieldBar(g2d, x, y);
                break;
        }
    }

    private void drawFlapBar(Graphics2D g2d, int x, int y) {
        int used = (int) (clamp01(flapCurrent / FLAP_MAX) * width);
        int safe = (int) (clamp01(flapMaxSafe / FLAP_MAX) * width);
        int margin;
        int over;
        if (flapCurrent <= flapMaxSafe) {
            margin = Math.max(0, safe - used);
            over = Math.max(0, width - used - margin);
        } else {
            margin = 0;
            over = Math.max(0, width - used);
        }
        g2d.setColor(Application.colorLabel);
        g2d.setStroke(thickTick);
        for (int tick : FLAP_TICKS) {
            int tx = x + (int) (tick * width / (double) FLAP_MAX);
            int ext = tick == 100 ? height : height / 4;
            g2d.drawLine(tx, y - ext - 2, tx, y);
        }
        if (used > 0) {
            g2d.setColor(Application.colorShadeShape);
            g2d.fillRect(x, y, used, height);
        }
        if (margin > 0) {
            g2d.setColor(Application.colorNum);
            g2d.fillRect(x + used, y, margin, height);
        }
        if (over > 0) {
            g2d.setColor(Application.colorWarning);
            g2d.fillRect(x + used + margin, y, over, height);
        }
    }

    private void drawAoaBar(Graphics2D g2d, int x, int y) {
        int len = (int) (clamp01(aoaRatio) * width);
        if (len <= 0)
            return;
        int bx = x + width - len;
        g2d.setStroke(thinStroke);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawRect(bx, y, len - 1, height - 1);
        g2d.setColor(aoaColor);
        g2d.fillRect(bx + 1, y + 1, Math.max(0, len - 2), Math.max(0, height - 2));
    }

    private void drawManeuverBar(Graphics2D g2d, int x, int y) {
        int baseY = y + mainAscent;
        int len = (int) Math.round(clamp01(maneuverIndex / 0.5) * width);
        drawTick(g2d, x, baseY, (int) Math.round(0.1 / 0.5 * width));
        if (maneuverIndex >= 0.1)
            drawTick(g2d, x, baseY, (int) Math.round(0.2 / 0.5 * width));
        if (maneuverIndex >= 0.2)
            drawTick(g2d, x, baseY, (int) Math.round(0.3 / 0.5 * width));
        if (maneuverIndex >= 0.3)
            drawTick(g2d, x, baseY, (int) Math.round(0.4 / 0.5 * width));
        if (maneuverIndex >= 0.4)
            drawTick(g2d, x, baseY, width);
        int nx = x + width;
        int ny = baseY + ctx.halfLine + ctx.lineWidth;
        g2d.setStroke(ctx.strokeThick);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawLine(nx, ny, nx - len, ny);
        g2d.setStroke(ctx.strokeThin);
        g2d.setColor(Application.colorNum);
        g2d.drawLine(nx, ny, nx - len, ny);
    }

    private void drawTick(Graphics2D g2d, int x, int baseY, int len) {
        g2d.setColor(Application.colorNum);
        g2d.drawLine(x + width - len, baseY + ctx.halfLine + ctx.lineWidth * 2,
                x + width - len, baseY + ctx.halfLine);
    }

    private void drawSpeedRatioBar(Graphics2D g2d, int x, int y) {
        int barX = x;
        int barH = height;
        int barW = barWidth;
        if (barW <= 0)
            barW = 1;
        g2d.setStroke(thickTick);
        if (aileronLockRatio > 0 && aileronLockRatio < 1.0) {
            int lockY = y + barH - Math.round((float) (barH * aileronLockRatio));
            g2d.setColor(Application.colorNum);
            g2d.drawLine(barX - 4, lockY, barX + barW / 2, lockY);
        }
        if (rudderLockRatio > 0 && rudderLockRatio < 1.0) {
            int lockY = y + barH - Math.round((float) (barH * rudderLockRatio));
            g2d.setColor(Application.colorNum);
            g2d.drawLine(barX + barW / 2, lockY, barX + barW + 4, lockY);
        }
        g2d.setColor(Application.colorNum);
        g2d.fillRect(barX, y, barW, barH);
        int greenH = Math.round((float) (barH * clamp01(speedRatio)));
        if (greenH > 0) {
            g2d.setColor(Application.colorShadeShape);
            g2d.fillRect(barX, y + barH - greenH, barW, greenH);
        }
        int tickY = y + barH - Math.round((float) (barH * clamp01(speedRatio)));
        int templateWidth = g2d.getFontMetrics(tickFont).stringWidth("888");
        int tickExtend = 4;
        int labelSpacing = 2;
        int tickStartX = barX - tickExtend - labelSpacing - templateWidth;
        g2d.setStroke(thickTick);
        g2d.setColor(Application.colorNum);
        g2d.drawLine(tickStartX, tickY, tickStartX + templateWidth + labelSpacing + tickExtend + barW - 1, tickY);
        int displayValue = (int) Math.round(speedRatio * 100);
        int valueLen = FastNumberFormatter.format(displayValue, speedValueBuf, 0);
        int actualTextWidth = g2d.getFontMetrics(tickFont).charsWidth(speedValueBuf, 0, valueLen);
        int textX = barX - tickExtend - labelSpacing - actualTextWidth;
        int textY = tickY - 3;
        g2d.setFont(tickFont);
        g2d.setColor(Application.colorShadeShape);
        g2d.drawChars(speedValueBuf, 0, valueLen, textX + 1, textY + 1);
        g2d.setColor(Application.colorNum);
        g2d.drawChars(speedValueBuf, 0, valueLen, textX, textY);
        int stallH = Math.round((float) (barH * clamp01(stallRatio)));
        if (stallH > 0) {
            g2d.setColor(Application.colorWarning);
            int stallW = Math.max(2, barW / 2);
            g2d.fillRect(barX + barW - stallW, y + barH - stallH, stallW, stallH);
        }
        if (unitMachRatio > 0 && unitMachRatio < 1.0) {
            int machY = y + barH - Math.round((float) (barH * unitMachRatio));
            g2d.setStroke(thickTick);
            g2d.setColor(Application.colorWarning);
            g2d.drawLine(barX, machY, barX + barW, machY);
        }
    }

    private void drawFieldBar(Graphics2D g2d, int x, int y) {
        double range = max - min;
        double ratio = range > 0 ? (fieldValue - min) / range : 0;
        ratio = clamp01(ratio);
        if (MiniHudProps.ORIENT_V.equals(orientation)) {
            int fill = (int) Math.round(height * ratio);
            g2d.setColor(Application.colorShadeShape);
            g2d.drawRect(x, y, Math.max(1, width - 1), Math.max(1, height - 1));
            if (fill > 0) {
                g2d.setColor(Application.colorNum);
                g2d.fillRect(x + 1, y + height - fill, Math.max(1, width - 2), fill);
            }
        } else {
            int fill = (int) Math.round(width * ratio);
            g2d.setColor(Application.colorShadeShape);
            g2d.drawRect(x, y, Math.max(1, width - 1), Math.max(1, height - 1));
            if (fill > 0) {
                g2d.setColor(Application.colorNum);
                g2d.fillRect(x + 1, y + 1, fill, Math.max(1, height - 2));
            }
        }
    }

    private static double clamp01(double v) {
        if (v < 0)
            return 0;
        if (v > 1)
            return 1;
        return v;
    }

    private static final java.awt.image.BufferedImage METRICS_IMG = new java.awt.image.BufferedImage(1, 1,
            java.awt.image.BufferedImage.TYPE_INT_ARGB);
    private static final Graphics2D METRICS = METRICS_IMG.createGraphics();
}
