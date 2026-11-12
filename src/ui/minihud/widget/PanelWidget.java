package ui.minihud.widget;

import java.awt.Color;
import java.awt.Dimension;
import java.awt.Graphics2D;

import prog.util.ColorHelper;
import ui.component.HUDComponent;
import ui.minihud.MiniHudProps;
import ui.minihud.WidgetBuildContext;
import ui.overlay.MinimalHUDContext;

/** 原子背景板：只画一块圆角矩形，不承载任何子组件。 */
public final class PanelWidget implements HUDComponent {
    private final String id;
    private final int width;
    private final int height;
    private final int radius;
    private final Color color;
    private boolean visible = true;

    public PanelWidget(WidgetBuildContext c) {
        this.id = c.widget.id;
        MinimalHUDContext ctx = c.ctx;
        this.width = Math.max(1, (int) Math.round(
                c.widget.getDouble(MiniHudProps.WIDTH_UNITS, 6.0) * ctx.hudFontSize));
        this.height = Math.max(1, (int) Math.round(
                c.widget.getDouble(MiniHudProps.HEIGHT_UNITS, 3.0) * ctx.hudFontSize));
        this.radius = Math.max(0, (int) Math.round(
                c.widget.getDouble(MiniHudProps.RADIUS_UNITS, 0.25) * ctx.hudFontSize));
        this.color = ColorHelper.parseColor(c.widget.get(MiniHudProps.PANEL_COLOR, "#00000000"),
                new Color(0, 0, 0, 0));
    }

    @Override
    public String getId() {
        return id;
    }

    @Override
    public Dimension getPreferredSize() {
        return new Dimension(width, height);
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        g2d.setColor(color);
        if (radius > 0)
            g2d.fillRoundRect(x, y, width, height, radius, radius);
        else
            g2d.fillRect(x, y, width, height);
    }

    @Override
    public boolean isVisible() {
        return visible;
    }

    @Override
    public void setVisible(boolean visible) {
        this.visible = visible;
    }
}
