package ui.component.row;

import java.awt.Font;
import java.awt.Graphics2D;

import ui.UIBaseElements;

public class HUDEnergyRow extends HUDTextRow {

    private String energyText;
    private int rightDraw;
    private Font smallFont;

    public HUDEnergyRow(int index, Font font, int height, Font smallFont, int rightDraw) {
        super(index, font, height);
        this.smallFont = smallFont;
        this.rightDraw = rightDraw;
        this.energyText = "";
    }

    private String energyTemplate;

    public void setTemplate(String mainTemplate, String energyTemplate) {
        setTemplate(mainTemplate);
        this.energyTemplate = energyTemplate;
    }

    public void setStyle(Font font, int height, Font smallFont, int rightDraw) {
        super.setStyle(font, height);
        this.smallFont = smallFont;
        this.rightDraw = rightDraw;
    }

    @Override
    public void onDataUpdate(ui.overlay.model.HUDData data) {
        if (data == null)
            return;

        this.update(data.altStr, data.warnAltitude);
        this.energyText = data.energyStr;
    }

    /**
     * 预览模式更新方法（简化版）
     * 能量颜色已统一使用 Application.colorNum，不再需要传入颜色参数
     */
    public void update(String text, boolean isWarning, String energyText) {
        super.update(text, isWarning);
        this.energyText = energyText;
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        // Draw Energy Text
        // Needs to align with main text baseline.
        int ascent = g2d.getFontMetrics(font).getAscent();
        int baseY = y + ascent;

        // 能量颜色应该使用状态字体颜色，与其他HUD组件保持一致
        UIBaseElements.__drawStringShade(g2d, x + rightDraw, baseY, 1, energyText, smallFont, prog.Application.colorNum);

        super.draw(g2d, x, y);
    }

    @Override
    public java.awt.Dimension getPreferredSize() {
        java.awt.Dimension base = super.getPreferredSize();
        int extraW = 0;
        String measureEn = (energyTemplate != null) ? energyTemplate : energyText;
        if (measureEn != null && smallFont != null) {
            extraW = rightDraw + ui.overlay.logic.HUDCalculator.getStringWidth(measureEn, smallFont);
        }
        int w = Math.max(base.width, extraW);
        return new java.awt.Dimension(w, height);
    }
}
