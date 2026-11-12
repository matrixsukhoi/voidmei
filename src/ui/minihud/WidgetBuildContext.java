package ui.minihud;

import prog.config.HUDSettings;
import ui.model.TelemetrySource;
import ui.overlay.MinimalHUDContext;

/** 组件构建上下文。ctx 已按 entry.scale 缩放；preview 时 source 为 null。 */
public final class WidgetBuildContext {
    public final MinimalHUDContext ctx;
    public final HUDSettings settings;
    public final MiniHudEntry entry;
    public final boolean preview;
    public final TelemetrySource source;
    public final prog.config.MiniHudFieldCatalog catalog;

    public WidgetBuildContext(MinimalHUDContext ctx, HUDSettings settings, MiniHudEntry entry,
            boolean preview, TelemetrySource source, prog.config.MiniHudFieldCatalog catalog) {
        this.ctx = ctx;
        this.settings = settings;
        this.entry = entry;
        this.preview = preview;
        this.source = source;
        this.catalog = catalog;
    }
}
