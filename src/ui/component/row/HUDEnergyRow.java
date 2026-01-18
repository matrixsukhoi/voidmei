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
    }

    public void update(String text, boolean isWarning, String energyText, Color energyColor) {
        super.update(text, isWarning);
        this.energyText = energyText;
        this.energyColor = energyColor;
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        // Draw Energy Text
        // Original: if(i==1) ... UIBaseElements.__drawStringShade(g, x + rightDraw, n +
        // liney - 1, 1, relEnergy...)
        // where liney = 1 + y (original y). And text is at n+y.
        // So energy text is at: n + (1+y) - 1 = n + y.
        // It's at the same Y level as the main text.

        UIBaseElements.__drawStringShade(g2d, x + rightDraw, y, 1, energyText, smallFont, energyColor);

        super.draw(g2d, x, y);
    }
}
