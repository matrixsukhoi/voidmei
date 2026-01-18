package ui.component.row;

import java.awt.BasicStroke;
import java.awt.Font;
import java.awt.Graphics2D;

import prog.Application;

public class HUDManeuverRow extends HUDTextRow {

    private int rightDraw;
    private int halfLine;
    private int lineWidth;
    private double maneuverIndex;

    // Lengths
    private int maneuverIndexLen;
    private int maneuverIndexLen10;
    private int maneuverIndexLen20;
    private int maneuverIndexLen30;
    private int maneuverIndexLen40;
    private int maneuverIndexLen50;

    // Strokes (could be passed or created, let's create similarly to others or use
    // cached if passed)
    private BasicStroke strokeThick;
    private BasicStroke strokeThin;

    public HUDManeuverRow(int index, Font font, int height, int rightDraw, int halfLine, int lineWidth,
            BasicStroke strokeThick, BasicStroke strokeThin) {
        super(index, font, height);
        this.rightDraw = rightDraw;
        this.halfLine = halfLine;
        this.lineWidth = lineWidth;
        this.strokeThick = strokeThick;
        this.strokeThin = strokeThin;
    }

    public void setStyle(Font font, int height, int rightDraw, int halfLine, int lineWidth,
            BasicStroke strokeThick, BasicStroke strokeThin) {
        super.setStyle(font, height);
        this.rightDraw = rightDraw;
        this.halfLine = halfLine;
        this.lineWidth = lineWidth;
        this.strokeThick = strokeThick;
        this.strokeThin = strokeThin;
    }

    @Override
    public void update(Object data) {
        if (data instanceof Object[]) {
            Object[] params = (Object[]) data;
            if (params.length >= 9) {
                super.update((String) params[0], (Boolean) params[1]);
                this.maneuverIndex = (Double) params[2];
                this.maneuverIndexLen = (Integer) params[3];
                this.maneuverIndexLen10 = (Integer) params[4];
                this.maneuverIndexLen20 = (Integer) params[5];
                this.maneuverIndexLen30 = (Integer) params[6];
                this.maneuverIndexLen40 = (Integer) params[7];
                this.maneuverIndexLen50 = (Integer) params[8];
            }
        }
    }

    public void update(String text, boolean isWarning, double maneuverIndex,
            int len, int len10, int len20, int len30, int len40, int len50) {
        super.update(text, isWarning);
        this.maneuverIndex = maneuverIndex;
        this.maneuverIndexLen = len;
        this.maneuverIndexLen10 = len10;
        this.maneuverIndexLen20 = len20;
        this.maneuverIndexLen30 = len30;
        this.maneuverIndexLen40 = len40;
        this.maneuverIndexLen50 = len50;
    }

    @Override
    public void draw(Graphics2D g, int x, int y) {
        super.draw(g, x, y);

        Application.debugPrint("Component: ManeuverIndex (Line 4), x=" + x + ", y=" + y);

        // Common draw logic
        drawLineMark(g, x, y, maneuverIndexLen10);

        if (maneuverIndex >= 0.1) {
            Application.debugPrint("Component: ManeuverIndex (>=0.1), x=" + (x + rightDraw - maneuverIndexLen20)
                    + ", y=" + (y + halfLine));
            drawLineMark(g, x, y, maneuverIndexLen20);
        }
        if (maneuverIndex >= 0.2) {
            Application.debugPrint("Component: ManeuverIndex (>=0.2), x=" + (x + rightDraw - maneuverIndexLen30)
                    + ", y=" + (y + halfLine));
            drawLineMark(g, x, y, maneuverIndexLen30);
        }
        if (maneuverIndex >= 0.3) {
            drawLineMark(g, x, y, maneuverIndexLen40);
        }
        if (maneuverIndex >= 0.4) {
            drawLineMark(g, x, y, maneuverIndexLen50);
        }

        // Final line
        g.setStroke(strokeThick);
        g.setColor(Application.colorShadeShape);
        g.drawLine(x + rightDraw, y + halfLine + lineWidth, x + rightDraw - maneuverIndexLen,
                y + halfLine + lineWidth);

        g.setStroke(strokeThin);
        g.setColor(Application.colorNum);
        g.drawLine(x + rightDraw, y + halfLine + lineWidth, x + rightDraw - maneuverIndexLen,
                y + halfLine + lineWidth);
    }

    private void drawLineMark(Graphics2D g, int x, int y, int len) {
        g.setColor(Application.colorShadeShape);
        g.setStroke(strokeThick);
        g.drawLine(x + rightDraw - len, y + halfLine + lineWidth + lineWidth,
                x + rightDraw - len, y + halfLine - lineWidth + lineWidth);
        g.setColor(Application.colorNum);
        g.setStroke(strokeThin);
        g.drawLine(x + rightDraw - len, y + halfLine + lineWidth + lineWidth,
                x + rightDraw - len, y + halfLine - lineWidth + lineWidth);
    }
}
