package ui.component.row;

import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics2D;

import ui.UIBaseElements;

public class HUDAkbRow extends HUDTextRow {

    private String aoaText;
    private int aoaY;
    private int rightDraw;
    private int lineWidth;
    private Font smallFont;
    private Color aoaColor;
    private Color aoaBarColor;

    /** 组件级可见性开关：速度文字（左侧主文字） */
    private boolean showSpeed = true;
    /** 组件级可见性开关：攻角指示器（AoA bar + α文字，右侧） */
    private boolean showAoa = true;

    public HUDAkbRow(int index, Font font, int height, Font smallFont, int rightDraw, int lineWidth) {
        super(index, font, height);
        this.smallFont = smallFont;
        this.rightDraw = rightDraw;
        this.lineWidth = lineWidth;
        this.aoaText = "";
        this.aoaColor = Color.YELLOW;
        this.aoaBarColor = Color.YELLOW;
    }

    private int aoaLength = 100; // Default

    private String aoaTemplate;

    /** 组件级可见性开关 */
    public void setShowSpeed(boolean v) { this.showSpeed = v; }
    public void setShowAoa(boolean v) { this.showAoa = v; }

    public void setTemplate(String mainTemplate, String aoaTemplate) {
        setTemplate(mainTemplate);
        this.aoaTemplate = aoaTemplate;
    }

    public void setStyle(Font font, int height, Font smallFont, int rightDraw, int lineWidth, int aoaLength) {
        super.setStyle(font, height);
        this.smallFont = smallFont;
        this.rightDraw = rightDraw;
        this.lineWidth = lineWidth;
        this.aoaLength = aoaLength;
    }

    @Override
    public void onDataUpdate(ui.overlay.model.HUDData data) {
        if (data == null)
            return;

        // Speed Text (uses default text field from HUDTextRow)
        super.update(data.speedStr, data.warnVne);

        // AoA Text and Bar
        this.aoaText = data.aoaStr;
        this.aoaColor = data.aoaColor;
        this.aoaBarColor = data.aoaBarColor;

        // Bar Calculation
        this.aoaY = (int) (data.aoaRatio * this.aoaLength);
        if (this.aoaY > this.rightDraw) {
            this.aoaY = this.rightDraw;
        }
    }

    public void update(String text, boolean isWarning, String aoaText, int aoaY, Color aoaColor, Color aoaBarColor) {
        super.update(text, isWarning);
        this.aoaText = aoaText;
        this.aoaY = aoaY;
        this.aoaColor = aoaColor;
        this.aoaBarColor = aoaBarColor;
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        int ascent = g2d.getFontMetrics(font).getAscent(); // Main font ascent
        int baseY = y + ascent;
        int liney = baseY + 1;

        // AoA bar + text：仅在 showAoa 开关打开时绘制
        if (showAoa) {
            UIBaseElements.drawHRect(g2d, x + (rightDraw - aoaY), liney, aoaY, lineWidth + 3, 1, aoaBarColor);
            UIBaseElements.__drawStringShade(g2d, x + rightDraw, liney - 1, 1, aoaText, smallFont, aoaColor);
        }

        // Speed 主文字：仅在 showSpeed 开关打开时绘制
        if (showSpeed) {
            super.draw(g2d, x, y);
        }
    }

    @Override
    public java.awt.Dimension getPreferredSize() {
        // 始终使用模板估算完整宽度，隐藏的组件保留占位符，保持布局稳定
        java.awt.Dimension base = super.getPreferredSize();
        int w = base.width;
        String measureAoa = (aoaTemplate != null) ? aoaTemplate : aoaText;
        if (measureAoa != null && smallFont != null) {
            int extraW = rightDraw + ui.overlay.logic.HUDCalculator.getStringWidth(measureAoa, smallFont);
            w = Math.max(w, extraW);
        }
        return new java.awt.Dimension(w, height);
    }
}
