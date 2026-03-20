package ui.component;

import java.awt.BasicStroke;
import java.awt.Dimension;
import java.awt.Font;
import java.awt.Graphics2D;

import prog.Application;
import ui.UIBaseElements;

/**
 * High-performance Flap Angle Bar component.
 * Displays a tri-color bar showing current flap angle vs allowable.
 */
public class FlapAngleBar extends AbstractHUDComponent {

    // Cached stroke
    private BasicStroke tickStroke;
    private int cachedStrokeWidth = -1;

    // Styling Context
    private int totalWidth;
    private int barHeight;
    private Font font;

    // State
    private double currentAngle;
    private double maxSafeAngle;
    private String displayText;

    // Tick marks at these positions
    private static final int[] TICK_POSITIONS = { 20, 33, 60, 100 };
    private static final int MAX_SCALE = 125;

    public FlapAngleBar() {
        this.currentAngle = 0;
        this.maxSafeAngle = 100;
        this.displayText = "  0/100";
    }

    @Override
    public String getId() {
        return "bar.flaps";
    }

    @Override
    public Dimension getPreferredSize() {
        int w = totalWidth > 0 ? totalWidth : 200;
        int h = (font != null ? font.getSize() : 12) + barHeight + 5;
        return new Dimension(w, h);
    }

    public void setStyleContext(int totalWidth, int barHeight, Font font) {
        this.totalWidth = totalWidth;
        this.barHeight = barHeight;
        this.font = font;
    }

    @Override
    public void onDataUpdate(ui.overlay.model.HUDData data) {
        if (data == null)
            return;

        this.currentAngle = data.flaps;
        this.maxSafeAngle = data.flapAllowAngle;
        this.displayText = String.format("%3.0f/%3.0f", currentAngle, maxSafeAngle);
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        if (font == null)
            return;

        // Adjust Y to treat input (x,y) as Top-Left
        // 1. Text Baseline
        int ascent = g2d.getFontMetrics(font).getAscent();
        int textY = y + ascent;

        // Draw text
        int strWidth = g2d.getFontMetrics(font).stringWidth(displayText);
        int strX = x + (totalWidth - strWidth) / 2;
        UIBaseElements.__drawStringShade(g2d, strX, textY, 1, displayText, font, Application.colorNum);

        // Bar position below text (using font size as line height approximation)
        int barY = y + font.getSize() + 2;

        // 计算三个区域的宽度
        // usedWidth=已使用襟翼, marginWidth=剩余安全裕度, overspeedWidth=超速区
        int usedWidth, marginWidth, overspeedWidth;

        // 已使用区域: 0 → currentAngle (黑色始终跟随当前角度)
        usedWidth = (int) (currentAngle * totalWidth / MAX_SCALE);

        if (currentAngle <= maxSafeAngle) {
            // 正常: 有安全裕度
            marginWidth = (int) (maxSafeAngle * totalWidth / MAX_SCALE) - usedWidth;
            overspeedWidth = totalWidth - usedWidth - marginWidth;
        } else {
            // 超限: 无安全裕度，红色是右边剩余部分
            marginWidth = 0;
            overspeedWidth = totalWidth - usedWidth;
        }

        // 边界保护
        usedWidth = Math.max(0, usedWidth);
        marginWidth = Math.max(0, marginWidth);
        overspeedWidth = Math.max(0, overspeedWidth);

        // Cache stroke
        if (cachedStrokeWidth != 2) {
            tickStroke = new BasicStroke(2);
            cachedStrokeWidth = 2;
        }

        // Draw tick marks
        g2d.setColor(Application.colorLabel);
        g2d.setStroke(tickStroke);

        for (int tick : TICK_POSITIONS) {
            int tx = x + (int) (tick * totalWidth / MAX_SCALE);
            int ext = (tick == 100) ? barHeight : barHeight / 4;
            g2d.drawLine(tx, barY - ext - 2, tx, barY); // Draw ticks strictly above barY? or attached?
            // Original: barY - ext - 4 to barY.
            // Let's keep it attached to the bar top.
        }

        // 绘制已使用区域 (0 → currentAngle，黑色)
        if (usedWidth > 0) {
            g2d.setColor(Application.colorShadeShape);
            g2d.fillRect(x, barY, usedWidth, barHeight);
        }

        // 绘制安全裕度区域 (currentAngle → maxSafeAngle，白色)
        if (marginWidth > 0) {
            g2d.setColor(Application.colorNum);
            g2d.fillRect(x + usedWidth, barY, marginWidth, barHeight);
        }

        // 绘制超速区 (右边剩余部分，红色)
        if (overspeedWidth > 0) {
            g2d.setColor(Application.colorWarning);
            g2d.fillRect(x + usedWidth + marginWidth, barY, overspeedWidth, barHeight);
        }
    }
}
