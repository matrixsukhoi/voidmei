package ui.component.row;

import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics2D;

import ui.UIBaseElements;

public class HUDEnergyRow extends HUDTextRow {

    private String energyText;
    private int rightDraw;
    private Font smallFont;
    private Color energyColor;

    public HUDEnergyRow(int index, Font font, int height, Font smallFont, int rightDraw) {
        super(index, font, height);
        this.smallFont = smallFont;
        this.rightDraw = rightDraw;
        this.energyText = "";
        this.energyColor = Color.YELLOW;
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
        // Energy color logic? Default yellow?
        // MinimalHUD uses 'relEnergy' string, but didn't seem to set color dynamically
        // in updateString.
        // So assuming default or passed static.
        // We'll keep default yellow for now or use Application.colorNum if appropriate?
        // MinimalHUD legacy: no explicit color change found for energy.
        // We will respect default initialization or update if needed.
    }

    public void update(String text, boolean isWarning, String energyText, Color energyColor) {
        super.update(text, isWarning);
        this.energyText = energyText;
        this.energyColor = energyColor;
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        // Draw Energy Text
        // Needs to align with main text baseline.
        int ascent = g2d.getFontMetrics(font).getAscent();
        int baseY = y + ascent;

        UIBaseElements.__drawStringShade(g2d, x + rightDraw, baseY, 1, energyText, smallFont, energyColor);

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
