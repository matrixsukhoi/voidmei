package ui.minihud;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import ui.component.HUDComponent;
import ui.layout.HUDLayoutNode;
import ui.layout.ModernHUDLayoutEngine;
import ui.overlay.model.HUDData;

/** 一次构建的 MiniHUD 场景：文档 + 组件 + 布局 DAG。 */
public final class MiniHudScene {

    public static final class EntryRuntime {
        public final MiniHudEntry entry;
        public final HUDComponent component;
        public final HUDLayoutNode node;
        public final boolean selfVisible;

        EntryRuntime(MiniHudEntry entry, HUDComponent component, HUDLayoutNode node, boolean selfVisible) {
            this.entry = entry;
            this.component = component;
            this.node = node;
            this.selfVisible = selfVisible;
        }
    }

    public final MiniHudLayoutDoc doc;
    public final ModernHUDLayoutEngine engine;
    public final List<HUDComponent> components = new ArrayList<>();
    public final List<TextRowWidget> liveFields = new ArrayList<>();
    public final Map<String, EntryRuntime> byId = new LinkedHashMap<>();
    private final boolean masterVisible;

    MiniHudScene(MiniHudLayoutDoc doc, ModernHUDLayoutEngine engine, boolean masterVisible) {
        this.doc = doc;
        this.engine = engine;
        this.masterVisible = masterVisible;
    }

    public HUDLayoutNode node(String id) {
        EntryRuntime rt = byId.get(id);
        return rt != null ? rt.node : null;
    }

    public HUDComponent component(String id) {
        EntryRuntime rt = byId.get(id);
        return rt != null ? rt.component : null;
    }

    /** 动态可见性：总开关 + 条目开关 + 复合行槽位 + 无襟翼机隐藏襟翼条。 */
    public void updateDynamicVisibility(HUDData data, boolean master) {
        for (EntryRuntime rt : byId.values()) {
            boolean v = master && rt.entry.visible && rt.selfVisible;
            if (v && "gauge.flapBar".equals(rt.entry.type) && data != null && !data.hasFlaps)
                v = false;
            rt.component.setVisible(v);
        }
    }

    public boolean isMasterVisible() {
        return masterVisible;
    }
}
