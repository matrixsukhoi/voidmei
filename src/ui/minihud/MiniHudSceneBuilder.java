package ui.minihud;

import prog.config.HUDSettings;
import prog.config.MiniHudFieldCatalog;
import ui.component.HUDComponent;
import ui.layout.HUDLayoutNode;
import ui.layout.ModernHUDLayoutEngine;
import ui.model.TelemetrySource;
import ui.overlay.MinimalHUDContext;
import ui.overlay.model.HUDData;

/** 布局文档 → 组件实例 + HUDLayoutNode DAG。 */
public final class MiniHudSceneBuilder {

    private MiniHudSceneBuilder() {
    }

    public static MiniHudScene build(MiniHudLayoutDoc doc, MinimalHUDContext baseCtx, HUDSettings settings,
            MiniHudFieldCatalog catalog, boolean preview, TelemetrySource source, boolean masterVisible) {
        MiniHudWidgets.registerDefaults();
        doc.validate();

        // 复刻旧版全局标签的影响：速度/高度/SEP 标签全关时右列宽度 3.5 行高，否则 5.5
        MinimalHUDContext ctxBase = hasAnyLegacyLabel(doc) ? baseCtx
                : baseCtx.withRightDraw((int) (baseCtx.hudFontSize * 3.5f));

        ModernHUDLayoutEngine engine = new ModernHUDLayoutEngine(ctxBase.width, ctxBase.height);
        engine.setLineHeight(ctxBase.hudFontSize);
        if (settings != null) {
            engine.setDebug(settings.getBool("enableLayoutDebug", false));
        }
        MiniHudScene scene = new MiniHudScene(doc, engine, masterVisible);

        // 1) 实例化组件（多实例各自独立）
        for (MiniHudEntry e : doc.entries) {
            MinimalHUDContext ctx = ctxBase.withScale(e.scale);
            WidgetBuildContext wc = new WidgetBuildContext(ctx, settings, e, preview, source, catalog);
            HUDComponent comp = MiniHudWidgets.create(wc);
            boolean selfVisible = e.visible && MiniHudWidgets.isSelfVisible(e);
            comp.setVisible(masterVisible && selfVisible);

            HUDLayoutNode node = new HUDLayoutNode(e.id, comp);
            node.setRelativePosition(e.unitX, e.unitY);
            node.setAnchors(e.parentAnchor, e.selfAnchor);
            node.setZOrder(e.z);

            MiniHudScene.EntryRuntime rt = new MiniHudScene.EntryRuntime(e, comp, node, selfVisible);
            scene.byId.put(e.id, rt);
            scene.components.add(comp);
            if (comp instanceof TextRowWidget)
                scene.liveFields.add((TextRowWidget) comp);
        }

        // 2) 挂父子关系（缺失父级降根），再交给引擎拓扑解算
        for (MiniHudScene.EntryRuntime rt : scene.byId.values()) {
            String pid = rt.entry.parentId;
            MiniHudScene.EntryRuntime parent = pid != null ? scene.byId.get(pid) : null;
            if (parent != null && parent != rt)
                rt.node.setParent(parent.node);
            engine.addNode(rt.node);
        }

        // 预览态演示值：行组件已在工厂内按条目属性写了占位文本，这里只喂仪表/条
        if (preview) {
            HUDData demo = MiniHudPreviewData.create(ctxBase);
            for (MiniHudScene.EntryRuntime rt : scene.byId.values()) {
                if (!rt.entry.type.startsWith("legacy.") && !(rt.component instanceof TextRowWidget))
                    rt.component.onDataUpdate(demo);
            }
        }

        engine.doLayout();
        return scene;
    }

    private static boolean hasAnyLegacyLabel(MiniHudLayoutDoc doc) {
        for (MiniHudEntry e : doc.entries) {
            if ("legacy.akb".equals(e.type) && !e.get(MiniHudProps.SPEED_LABEL, "SPD").isEmpty())
                return true;
            if ("legacy.energy".equals(e.type) && !e.get(MiniHudProps.ALT_LABEL, "ALT").isEmpty())
                return true;
            if ("legacy.sep".equals(e.type) && !e.get(MiniHudProps.SEP_LABEL, "SEP").isEmpty())
                return true;
        }
        return false;
    }
}
