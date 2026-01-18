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

    public void setStyle(Font font, int height, Font smallFont, int rightDraw, int lineWidth) {
        super.setStyle(font, height);
        this.smallFont = smallFont;
        this.rightDraw = rightDraw;
        this.lineWidth = lineWidth;
    }

    @Override
    public void update(Object data) {
        if (data instanceof Object[]) {
            Object[] params = (Object[]) data;
            if (params.length >= 6) {
                // Call super.update(text, isWarning)
                super.update((String) params[0], (Boolean) params[1]);
                this.aoaText = (String) params[2];
                this.aoaY = (Integer) params[3];
                this.aoaColor = (Color) params[4];
                this.aoaBarColor = (Color) params[5];
            }
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
        // Original logic: "liney = 1 + y" where y was original draw y.
        // And text drawn at "verticalTextOffset + y".
        // Here 'y' passed in IS 'verticalTextOffset + y'.
        // So for row 0 (verticalTextOffset=0), y passed is y_base.
        // liney should be y + 1.

        int liney = y + 1;

        UIBaseElements.drawHRect(g2d, x + (rightDraw - aoaY), liney, aoaY, lineWidth + 3, 1, aoaBarColor);
        UIBaseElements.__drawStringShade(g2d, x + rightDraw, liney - 1, 1, aoaText, smallFont, aoaColor);

        super.draw(g2d, x, y);
    }
}
