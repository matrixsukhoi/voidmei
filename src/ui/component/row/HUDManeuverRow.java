package ui.component.row;

import java.awt.BasicStroke;
import java.awt.Font;
import java.awt.Graphics2D;

import prog.Application;

public class HUDManeuverRow extends HUDTextRow {

    private int rightDraw;
    private int halfLine;
    private int lineWidth;
    private double maneuverIndex;

    /** 组件级可见性开关：G-force 文字（左侧主文字） */
    private boolean showGLoad = true;
    /** 组件级可见性开关：机动条（右侧 bar + tick marks） */
    private boolean showManeuverBar = true;

    // Lengths
    private int maneuverIndexLen;
    private int maneuverIndexLen10;
    private int maneuverIndexLen20;
    private int maneuverIndexLen30;
    private int maneuverIndexLen40;
    private int maneuverIndexLen50;

    // Strokes (could be passed or created, let's create similarly to others or use
    // cached if passed)
    private BasicStroke strokeThick;
    private BasicStroke strokeThin;

    public HUDManeuverRow(int index, Font font, int height, int rightDraw, int halfLine, int lineWidth,
            BasicStroke strokeThick, BasicStroke strokeThin) {
        super(index, font, height);
        this.rightDraw = rightDraw;
        this.halfLine = halfLine;
        this.lineWidth = lineWidth;
        this.strokeThick = strokeThick;
        this.strokeThin = strokeThin;
    }

    public void setStyle(Font font, int height, int rightDraw, int halfLine, int lineWidth,
            BasicStroke strokeThick, BasicStroke strokeThin) {
        super.setStyle(font, height);
        this.rightDraw = rightDraw;
        this.halfLine = halfLine;
        this.lineWidth = lineWidth;
        this.strokeThick = strokeThick;
        this.strokeThin = strokeThin;
    }

    /** 组件级可见性开关 */
    public void setShowGLoad(boolean v) { this.showGLoad = v; }
    public void setShowManeuverBar(boolean v) { this.showManeuverBar = v; }

    public void update(String text, boolean isWarning, double maneuverIndex,
            int len, int len10, int len20, int len30, int len40, int len50) {
        super.update(text, isWarning);
        this.maneuverIndex = maneuverIndex;
        this.maneuverIndexLen = len;
        this.maneuverIndexLen10 = len10;
        this.maneuverIndexLen20 = len20;
        this.maneuverIndexLen30 = len30;
        this.maneuverIndexLen40 = len40;
        this.maneuverIndexLen50 = len50;
    }

    @Override
    public void draw(Graphics2D g, int x, int y) {
        // G-force 主文字：仅在 showGLoad 开关打开时绘制
        if (showGLoad) {
            super.draw(g, x, y);
        }

        // 机动条 + tick marks：仅在 showManeuverBar 开关打开时绘制
        if (!showManeuverBar) {
            return;
        }

        // Calculate Baseline Y for line positioning consistency
        // Lines were relative to Baseline in legacy code (y + halfLine).
        int ascent = g.getFontMetrics(font).getAscent();
        int baseY = y + ascent;

        // Common draw logic
        drawLineMark(g, x, baseY, maneuverIndexLen10);

        // 根据当前值绘制一个或多个刻度线
        if (maneuverIndex >= 0.1) {
            drawLineMark(g, x, baseY, maneuverIndexLen20);
        }
        if (maneuverIndex >= 0.2) {
            drawLineMark(g, x, baseY, maneuverIndexLen30);
        }
        if (maneuverIndex >= 0.3) {
            drawLineMark(g, x, baseY, maneuverIndexLen40);
        }
        if (maneuverIndex >= 0.4) {
            drawLineMark(g, x, baseY, maneuverIndexLen50);
        }

        // Final line relative to Baseline
        int newX = x + rightDraw;
        int newY = baseY + halfLine;

        g.setStroke(strokeThick);
        g.setColor(Application.colorShadeShape);
        g.drawLine(newX, newY + lineWidth, newX - maneuverIndexLen, newY + lineWidth);

        g.setStroke(strokeThin);
        g.setColor(Application.colorNum);
        g.drawLine(newX, newY + lineWidth, newX - maneuverIndexLen, newY + lineWidth);
    }

    private void drawLineMark(Graphics2D g, int x, int y, int len) {
        g.drawLine(x + rightDraw - len, y + halfLine + lineWidth + lineWidth,
                x + rightDraw - len, y + halfLine - lineWidth + lineWidth);
    }

    @Override
    public java.awt.Dimension getPreferredSize() {
        // 始终使用模板估算完整宽度，隐藏的组件保留占位符，保持布局稳定
        java.awt.Dimension base = super.getPreferredSize();
        int w = Math.max(base.width, rightDraw + 5);
        return new java.awt.Dimension(w, height);
    }
}
