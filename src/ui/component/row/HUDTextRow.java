package ui.component.row;

import java.awt.Font;
import java.awt.Graphics2D;

import prog.Application;
import ui.UIBaseElements;

public class HUDTextRow implements HUDRow {

    protected String text;
    protected Font font;
    protected int height;
    protected int index; // For debug logging
    protected boolean isWarning;

    public HUDTextRow(int index, Font font, int height) {
        this.index = index;
        this.font = font;
        this.height = height;
        this.text = "";
    }

    public void update(String text, boolean isWarning) {
        this.text = text;
        this.isWarning = isWarning;
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        Application.debugPrint("Component: TextLine[" + index + "], x=" + x + ", y=" + y);
        if (isWarning) {
            UIBaseElements.__drawStringShade(g2d, x, y, 1, text, font, Application.colorWarning);
        } else {
            UIBaseElements.__drawStringShade(g2d, x, y, 1, text, font, Application.colorNum);
        }
    }

    @Override
    public int getHeight() {
        return height;
    }
}
