package ui.minihud.widget;

import java.awt.Dimension;
import java.awt.Graphics2D;
import java.util.Collections;
import java.util.List;
import java.util.function.Supplier;

import parser.Airfield;
import ui.component.HUDComponent;
import ui.minihud.MiniHudAirfieldPanel;
import ui.minihud.MiniHudPreviewData;
import ui.minihud.WidgetBuildContext;
import ui.overlay.model.HUDData;

/**
 * 机场距离面板的画布组件。
 * 把原来的原生面板纳入 scene，使其支持选中、拖动、图层与持久化。
 */
public final class AirfieldWidget implements HUDComponent {

    private final String id;
    private final MiniHudAirfieldPanel panel = new MiniHudAirfieldPanel();
    private final Supplier<List<Airfield>> source;
    private final boolean preview;
    private boolean visible = true;

    public AirfieldWidget(WidgetBuildContext c) {
        this.id = c.widget.id;
        this.source = c.airfields;
        this.preview = c.preview;
        panel.configure(c.ctx,
                c.settings.isMiniHudAirfieldArrow(),
                c.settings.isMiniHudAirfieldShowAll(),
                c.settings.isMiniHudAirfieldShowGrid(),
                c.settings.getMiniHudAirfieldFriendlyColor(),
                c.settings.getMiniHudAirfieldEnemyColor(),
                c.settings.getMiniHudAirfieldWarnKm());
    }

    @Override
    public String getId() {
        return id;
    }

    @Override
    public Dimension getPreferredSize() {
        return new Dimension(panel.getPanelWidth(), panel.getPanelHeight());
    }

    @Override
    public void draw(Graphics2D g2d, int x, int y) {
        panel.paint(g2d, x, y);
    }

    @Override
    public boolean isVisible() {
        return visible && panel.isVisible();
    }

    @Override
    public void setVisible(boolean visible) {
        this.visible = visible;
    }

    @Override
    public void onDataUpdate(HUDData data) {
        List<Airfield> list = source != null ? source.get() : Collections.emptyList();
        // 预览用固定航向，保证演示箭头与旧原生面板一致；游戏态跟随实时航向
        double heading = preview ? MiniHudPreviewData.heading() : (data != null ? data.heading : 0);
        panel.update(list, heading);
    }
}
