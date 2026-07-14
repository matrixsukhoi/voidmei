package ui.component.row;

import java.awt.Font;
import java.awt.Graphics2D;

import ui.UIBaseElements;

/**
 * Row 2 组件级拆分：襟翼/可变翼 + 减速板 + 起落架。
 * 三个子组件各有一个独立的可见性开关。
 */
public class HUDMechanizationRow extends HUDTextRow {

    /** 组件级可见性开关：襟翼/可变翼 */
    private boolean showFlaps = true;
    /** 组件级可见性开关：减速板 */
    private boolean showAirbrake = true;
    /** 组件级可见性开关：起落架 */
    private boolean showGear = true;

    private String flapsWingStr = "";
    private String airbrakeStr = "";
    private String gearStr = "";

    /** 各子组件模板字符串（用于宽度估算） */
    private String flapsTemplate = "W100";
    private String airbrakeTemplate = "BRK";
    private String gearTemplate = "GEA";

    public HUDMechanizationRow(int index, Font font, int height) {
        super(index, font, height);
    }

    /** 组件级可见性开关 */
    public void setShowFlaps(boolean v) { this.showFlaps = v; }
    public void setShowAirbrake(boolean v) { this.showAirbrake = v; }
    public void setShowGear(boolean v) { this.showGear = v; }

    /** 设置子组件数据（游戏模式） */
    public void updateParts(String flapsWingStr, String airbrakeStr, String gearStr, boolean isWarning) {
        super.update("", isWarning); // 清空主文字（不使用）
        this.flapsWingStr = flapsWingStr;
        this.airbrakeStr = airbrakeStr;
        this.gearStr = gearStr;
    }

    /** 预览模式更新（兼容旧接口） */
    @Override
    public void update(String text, boolean isWarning) {
        super.update(text, isWarning);
        // 从合并字符串解析回子组件（预览用，格式: "F100BRKGEA" 或 "    BRKGEA"）
        if (text != null && text.length() >= 10) {
            this.flapsWingStr = text.substring(0, 4).trim();
            this.airbrakeStr = text.substring(4, 7).trim();
            this.gearStr = text.substring(7, 10).trim();
        } else {
            this.flapsWingStr = "";
            this.airbrakeStr = "";
            this.gearStr = "";
        }
    }

    @Override
    public void onDataUpdate(ui.overlay.model.HUDData data) {
        if (data == null) return;
        this.flapsWingStr = data.flapsWingStr;
        this.airbrakeStr = data.airbrakeStr;
        this.gearStr = data.gearStr;
        this.isWarning = data.warnConfiguration;
    }

    /** 设置模板（预览模式），格式同旧 mechanizationStr */
    public void setTemplate(String template) {
        super.setTemplate(template);
        if (template != null && template.length() >= 10) {
            this.flapsTemplate = template.substring(0, 4).trim();
            if (flapsTemplate.isEmpty()) flapsTemplate = "F100";
            this.airbrakeTemplate = template.substring(4, 7).trim();
            this.gearTemplate = template.substring(7, 10).trim();
        }
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        int ascent = g2d.getFontMetrics(font).getAscent();
        int baseY = y + ascent;

        int curX = x;

        // 襟翼/可变翼：始终占位推进 curX，隐藏时仅不绘制文字
        int flapsWidth = !flapsTemplate.isEmpty()
                ? ui.overlay.logic.HUDCalculator.getStringWidth(flapsTemplate + " ", font) : 0;
        if (showFlaps && !flapsWingStr.isEmpty()) {
            UIBaseElements.__drawStringShade(g2d, curX, baseY, 1, flapsWingStr, font,
                    isWarning ? prog.Application.colorWarning : prog.Application.colorNum);
        }
        curX += flapsWidth;

        // 减速板：始终占位推进 curX，隐藏时仅不绘制文字
        int brkWidth = !airbrakeTemplate.isEmpty()
                ? ui.overlay.logic.HUDCalculator.getStringWidth(airbrakeTemplate + " ", font) : 0;
        if (showAirbrake && !airbrakeStr.isEmpty()) {
            UIBaseElements.__drawStringShade(g2d, curX, baseY, 1, airbrakeStr, font,
                    isWarning ? prog.Application.colorWarning : prog.Application.colorNum);
        }
        curX += brkWidth;

        // 起落架：始终占位推进 curX，隐藏时仅不绘制文字
        if (showGear && !gearStr.isEmpty()) {
            UIBaseElements.__drawStringShade(g2d, curX, baseY, 1, gearStr, font,
                    isWarning ? prog.Application.colorWarning : prog.Application.colorNum);
        }
    }

    @Override
    public java.awt.Dimension getPreferredSize() {
        int w = 0;
        if (font == null) return new java.awt.Dimension(w, height);

        // 始终使用模板估算完整宽度，隐藏的组件保留占位符，保持布局稳定
        if (!flapsTemplate.isEmpty()) {
            w += ui.overlay.logic.HUDCalculator.getStringWidth(flapsTemplate + " ", font);
        }
        if (!airbrakeTemplate.isEmpty()) {
            w += ui.overlay.logic.HUDCalculator.getStringWidth(airbrakeTemplate + " ", font);
        }
        if (!gearTemplate.isEmpty()) {
            w += ui.overlay.logic.HUDCalculator.getStringWidth(gearTemplate, font);
        }
        return new java.awt.Dimension(w, height);
    }
}
