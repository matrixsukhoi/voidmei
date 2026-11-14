package ui.minihud;

import java.util.List;
import java.util.function.Supplier;

import parser.Airfield;
import prog.config.HUDSettings;
import ui.model.TelemetrySource;
import ui.overlay.MinimalHUDContext;

/** 组件构建上下文。ctx 已按 widget.scale 缩放；预览时 source 为 null。 */
public final class WidgetBuildContext {
    public final MinimalHUDContext ctx;
    public final HUDSettings settings;
    public final MiniHudWidget widget;
    public final boolean preview;
    public final TelemetrySource source;
    public final prog.config.MiniHudFieldCatalog catalog;
    /** 机场快照来源；预览态由 SceneBuilder 注入演示数据。 */
    public final Supplier<List<Airfield>> airfields;

    public WidgetBuildContext(MinimalHUDContext ctx, HUDSettings settings, MiniHudWidget widget,
            boolean preview, TelemetrySource source, prog.config.MiniHudFieldCatalog catalog) {
        this(ctx, settings, widget, preview, source, catalog, null);
    }

    public WidgetBuildContext(MinimalHUDContext ctx, HUDSettings settings, MiniHudWidget widget,
            boolean preview, TelemetrySource source, prog.config.MiniHudFieldCatalog catalog,
            Supplier<List<Airfield>> airfields) {
        this.ctx = ctx;
        this.settings = settings;
        this.widget = widget;
        this.preview = preview;
        this.source = source;
        this.catalog = catalog;
        this.airfields = airfields;
    }
}
