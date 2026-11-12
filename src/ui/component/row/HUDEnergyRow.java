package ui.component.row;

import java.awt.Font;
import java.awt.Graphics2D;

import ui.UIBaseElements;

public class HUDEnergyRow extends HUDTextRow {

    private String energyText;
    private int rightDraw;
    private Font smallFont;

    private String altSource = "AUTO"; // AUTO / BARO / RADAR
    private String altLabel = "ALT";
    private String energyLabel = "E";

    /** 条目级显示方式：高度来源与标签。 */
    public void setAltFormat(String source, String altLabel, String energyLabel) {
        this.altSource = source != null ? source : "AUTO";
        this.altLabel = altLabel != null ? altLabel : "";
        this.energyLabel = energyLabel != null ? energyLabel : "";
    }

    /** 组件级可见性开关：高度文字（左侧主文字） */
    private boolean showAltitude = true;
    /** 组件级可见性开关：能量读数（右侧） */
    private boolean showEnergy = true;

    public HUDEnergyRow(int index, Font font, int height, Font smallFont, int rightDraw) {
        super(index, font, height);
        this.smallFont = smallFont;
        this.rightDraw = rightDraw;
        this.energyText = "";
    }

    private String energyTemplate;

    /** 组件级可见性开关 */
    public void setShowAltitude(boolean v) { this.showAltitude = v; }
    public void setShowEnergy(boolean v) { this.showEnergy = v; }

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

        boolean useRadar;
        if ("RADAR".equals(altSource)) {
            useRadar = data.radioAltitudeValid;
        } else if ("BARO".equals(altSource)) {
            useRadar = false;
        } else {
            useRadar = data.warnAltitude; // AUTO：<=500m 时用雷达高度
        }
        String alt = altLabel + (useRadar ? String.format("R%5.0f", data.radioAltitude)
                : String.format("%6.0f", data.altitude));
        this.update(alt, data.warnAltitude);
        this.energyText = energyLabel + String.format("%5.0f", data.energyM);
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
        int ascent = g2d.getFontMetrics(font).getAscent();
        int baseY = y + ascent;

        // 能量读数：仅在 showEnergy 开关打开时绘制
        if (showEnergy) {
            UIBaseElements.__drawStringShade(g2d, x + rightDraw, baseY, 1, energyText, smallFont, prog.Application.colorNum);
        }

        // 高度主文字：仅在 showAltitude 开关打开时绘制
        if (showAltitude) {
            super.draw(g2d, x, y);
        }
    }

    @Override
    public java.awt.Dimension getPreferredSize() {
        // 始终使用模板估算完整宽度，隐藏的组件保留占位符，保持布局稳定
        java.awt.Dimension base = super.getPreferredSize();
        int w = base.width;
        String measureEn = (energyTemplate != null) ? energyTemplate : energyText;
        if (measureEn != null && smallFont != null) {
            int extraW = rightDraw + ui.overlay.logic.HUDCalculator.getStringWidth(measureEn, smallFont);
            w = Math.max(w, extraW);
        }
        return new java.awt.Dimension(w, height);
    }
}
