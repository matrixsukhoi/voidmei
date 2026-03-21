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
        // Draw AoA Bar stuff
        // y is Top-Left. Convert to Baseline-relative logic if needed,
        // or just relative to Top.
        // Original logic seemed to rely on y being baseline.
        // Let's establish baseline y.
        int ascent = g2d.getFontMetrics(font).getAscent(); // Main font ascent
        int baseY = y + ascent;

        // Original: liney = y + 1. (Baseline + 1)
        // New: liney = baseY + 1. (Bottom of text + 1)
        int liney = baseY + 1;

        UIBaseElements.drawHRect(g2d, x + (rightDraw - aoaY), liney, aoaY, lineWidth + 3, 1, aoaBarColor);
        UIBaseElements.__drawStringShade(g2d, x + rightDraw, liney - 1, 1, aoaText, smallFont, aoaColor);

        super.draw(g2d, x, y);
    }

    @Override
    public java.awt.Dimension getPreferredSize() {
        java.awt.Dimension base = super.getPreferredSize();
        int extraW = 0;
        String measureAoa = (aoaTemplate != null) ? aoaTemplate : aoaText;
        if (measureAoa != null && smallFont != null) {
            extraW = rightDraw + ui.overlay.logic.HUDCalculator.getStringWidth(measureAoa, smallFont);
        }
        // Add a small margin for bar if needed, or just text.
        // The bar is drawn at x + (rightDraw - aoaY), so it stays within [x ..
        // x+rightDraw].
        // So max width is determined by the text at rightDraw.
        int w = Math.max(base.width, extraW);
        return new java.awt.Dimension(w, height);
    }
}
