package ui.minihud.widget;

import java.awt.Dimension;
import java.awt.Graphics2D;

import ui.component.AttitudeIndicatorGauge;
import ui.component.HUDComponent;
import ui.minihud.MiniHudProps;
import ui.minihud.WidgetBuildContext;
import ui.overlay.MinimalHUDContext;
import ui.overlay.model.HUDData;

/** 原子姿态仪组件：只负责一个地平仪表盘。 */
public final class AttitudeWidget implements HUDComponent {
    private final String id;
    private final AttitudeIndicatorGauge gauge;
    private boolean visible = true;

    public AttitudeWidget(WidgetBuildContext c) {
        this.id = c.widget.id;
        MinimalHUDContext ctx = c.ctx;
        this.gauge = new AttitudeIndicatorGauge();
        gauge.setStyleContext(ctx.compassDiameter, ctx.compassRadius, ctx.compassInnerMarkRadius,
                ctx.lineWidth, ctx.halfLine, ctx.drawFontSmall);
        gauge.setInertialMode(MiniHudProps.MODE_INERTIAL.equals(
                c.widget.get(MiniHudProps.COORDINATE_MODE, MiniHudProps.MODE_BODY)));
    }

    @Override
    public String getId() {
        return id;
    }

    @Override
    public Dimension getPreferredSize() {
        return gauge.getPreferredSize();
    }

    @Override
    public void onDataUpdate(HUDData data) {
        gauge.onDataUpdate(data);
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        gauge.draw(g2d, x, y);
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
