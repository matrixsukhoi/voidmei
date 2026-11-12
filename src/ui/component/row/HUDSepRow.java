package ui.component.row;

import java.awt.Font;

/**
 * SEP 单行文本。
 * P0 阶段沿用 HUDCalculator 预格式化字符串；后续可改为按条目属性自格式化。
 */
public class HUDSepRow extends HUDTextRow {

    private String label = "SEP";
    private boolean arrow = true;
    private String templateText = "SEP↑30";

    public HUDSepRow(int index, Font font, int height) {
        super(index, font, height);
    }

    public void setProps(String label, boolean arrow) {
        this.label = label != null ? label : "";
        this.arrow = arrow;
        this.templateText = buildTemplate();
    }

    private String buildTemplate() {
        // 与旧 refreshTemplates 一致：箭头 + 左对齐 4 位
        return label + (arrow ? "↑" : "") + String.format("%-4s", "30");
    }

    @Override
    public void onDataUpdate(ui.overlay.model.HUDData data) {
        if (data == null)
            return;
        // 按条目属性自格式化：标签 + 箭头 + 左对齐 4 位
        String arrowChar = arrow ? (data.verticalSpeed > 0 ? "↑" : "↓") : "";
        update(label + arrowChar + String.format("%-4.0f", data.verticalSpeed), false);
    }

    @Override
    public java.awt.Dimension getPreferredSize() {
        // 用模板测宽，防止数值跳动导致窗口抖动
        if (font != null) {
            int w = ui.overlay.logic.HUDCalculator.getStringWidth(templateText, font);
            return new java.awt.Dimension(w, height);
        }
        return super.getPreferredSize();
    }
}
