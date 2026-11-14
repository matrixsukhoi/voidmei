package ui.minihud;

import java.util.Collections;
import java.util.List;
import java.util.function.Supplier;

import parser.Airfield;
import prog.config.HUDSettings;
import prog.config.MiniHudFieldCatalog;
import ui.component.HUDComponent;
import ui.minihud.widget.BarWidget;
import ui.minihud.widget.TextWidget;
import ui.model.TelemetrySource;
import ui.overlay.MinimalHUDContext;

/** 自由画布文档 → 原子组件实例 + 绝对像素场景。 */
public final class MiniHudSceneBuilder {

    /** 预览机场列表每帧复用同一实例，避免机场面板反复重建。 */
    private static final List<Airfield> PREVIEW_AIRFIELDS =
            Collections.unmodifiableList(MiniHudPreviewData.airfields());

    private MiniHudSceneBuilder() {
    }

    public static MiniHudScene build(MiniHudDocument doc, MinimalHUDContext baseCtx, HUDSettings settings,
            MiniHudFieldCatalog catalog, boolean preview, TelemetrySource source, boolean masterVisible) {
        return build(doc, baseCtx, settings, catalog, preview, source, masterVisible, null);
    }

    public static MiniHudScene build(MiniHudDocument doc, MinimalHUDContext baseCtx, HUDSettings settings,
            MiniHudFieldCatalog catalog, boolean preview, TelemetrySource source, boolean masterVisible,
            Supplier<List<Airfield>> airfields) {
        MiniHudWidgets.registerDefaults();
        doc.validate();

        Supplier<List<Airfield>> airfieldSource = preview ? () -> PREVIEW_AIRFIELDS : airfields;
        MiniHudScene scene = new MiniHudScene(doc, baseCtx, masterVisible);
        for (MiniHudWidget w : doc.widgets) {
            MinimalHUDContext ctx = baseCtx.withScale(w.scale);
            WidgetBuildContext wc = new WidgetBuildContext(ctx, settings, w, preview, source, catalog,
                    airfieldSource);
            HUDComponent comp = MiniHudWidgets.create(wc);
            comp.setVisible(masterVisible && w.visible);
            MiniHudScene.Runtime rt = new MiniHudScene.Runtime(w, comp);
            scene.runtimes.add(rt);
            scene.byId.put(w.id, rt);
            scene.components.add(comp);
            if (comp instanceof TextWidget)
                scene.textWidgets.add((TextWidget) comp);
            else if (comp instanceof BarWidget)
                scene.barWidgets.add((BarWidget) comp);
        }

        // 预览态：喂演示数据，保证所见即所得
        if (preview) {
            ui.overlay.model.HUDData demo = MiniHudPreviewData.create(baseCtx);
            for (HUDComponent comp : scene.components)
                comp.onDataUpdate(demo);
        }
        scene.layout();
        return scene;
    }
}
