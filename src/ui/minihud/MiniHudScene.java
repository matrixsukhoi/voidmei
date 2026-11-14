package ui.minihud;

import java.awt.Graphics2D;
import java.awt.Rectangle;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import ui.component.HUDComponent;
import ui.minihud.widget.BarWidget;
import ui.minihud.widget.TextWidget;
import ui.overlay.MinimalHUDContext;
import ui.overlay.model.HUDData;

/**
 * 一次构建的 MiniHUD 场景：自由画布文档 + 原子组件 + 绝对像素矩形。
 * 不解析父链，不做相对布局；窗口只是内容包围盒的视口。
 */
public final class MiniHudScene {

    public static final class Runtime {
        public final MiniHudWidget widget;
        public final HUDComponent component;
        public final Rectangle rect = new Rectangle();

        Runtime(MiniHudWidget widget, HUDComponent component) {
            this.widget = widget;
            this.component = component;
        }
    }

    public final MiniHudDocument doc;
    public final MinimalHUDContext baseCtx;
    public final List<Runtime> runtimes = new ArrayList<>();
    public final Map<String, Runtime> byId = new LinkedHashMap<>();
    public final List<HUDComponent> components = new ArrayList<>();
    public final List<TextWidget> textWidgets = new ArrayList<>();
    public final List<BarWidget> barWidgets = new ArrayList<>();

    private final boolean masterVisible;
    private final List<Runtime> ordered = new ArrayList<>();
    private int offsetX;
    private int offsetY;
    private boolean debug;

    MiniHudScene(MiniHudDocument doc, MinimalHUDContext baseCtx, boolean masterVisible) {
        this.doc = doc;
        this.baseCtx = baseCtx;
        this.masterVisible = masterVisible;
    }

    public Runtime runtime(String id) {
        return byId.get(id);
    }

    public HUDComponent component(String id) {
        Runtime r = byId.get(id);
        return r != null ? r.component : null;
    }

    /** 解算所有组件的绝对像素矩形；位置单位 = 基准行高。 */
    public void layout() {
        int unit = Math.max(1, baseCtx.hudFontSize);
        for (Runtime r : runtimes) {
            r.rect.x = (int) Math.round(r.widget.x * unit);
            r.rect.y = (int) Math.round(r.widget.y * unit);
            java.awt.Dimension size = r.component.getPreferredSize();
            r.rect.width = Math.max(1, size.width);
            r.rect.height = Math.max(1, size.height);
        }
        ordered.clear();
        ordered.addAll(runtimes);
        ordered.sort(Comparator.comparingInt((Runtime r) -> r.widget.z)
                .thenComparingInt(r -> doc.indexOf(r.widget.id)));
    }

    public int getOffsetX() {
        return offsetX;
    }

    public int getOffsetY() {
        return offsetY;
    }

    public void setOffset(int x, int y) {
        this.offsetX = x;
        this.offsetY = y;
    }

    public void setDebug(boolean debug) {
        this.debug = debug;
    }

    public boolean isDebug() {
        return debug;
    }

    public boolean isMasterVisible() {
        return masterVisible;
    }

    /** 组件在自身矩形左侧额外绘制的最大像素（速度条读数），用于窗口左 padding。 */
    public int getMaxDrawPadLeft() {
        int max = 0;
        for (Runtime r : runtimes) {
            if (r.component.isVisible() && r.component instanceof BarWidget)
                max = Math.max(max, ((BarWidget) r.component).getHitPadLeft());
        }
        return max;
    }

    public boolean isEffectivelyVisible(Runtime r) {
        return masterVisible && r.widget.visible && r.component.isVisible();
    }

    /** 内容包围盒（像素，未加窗口 offset）。 */
    public Rectangle bounds(boolean includeHidden) {
        int minX = Integer.MAX_VALUE;
        int minY = Integer.MAX_VALUE;
        int maxX = Integer.MIN_VALUE;
        int maxY = Integer.MIN_VALUE;
        boolean any = false;
        for (Runtime r : runtimes) {
            if (!includeHidden && !isEffectivelyVisible(r))
                continue;
            any = true;
            Rectangle b = r.rect;
            minX = Math.min(minX, b.x);
            minY = Math.min(minY, b.y);
            maxX = Math.max(maxX, b.x + b.width);
            maxY = Math.max(maxY, b.y + b.height);
        }
        if (!any)
            return new Rectangle(0, 0, 1, 1);
        return new Rectangle(minX, minY, Math.max(1, maxX - minX), Math.max(1, maxY - minY));
    }

    public void render(Graphics2D g2d) {
        for (Runtime r : ordered) {
            if (!isEffectivelyVisible(r))
                continue;
            r.component.draw(g2d, r.rect.x + offsetX, r.rect.y + offsetY);
            if (debug) {
                int hash = r.widget.id.hashCode();
                int rCol = (hash & 0xFF0000) >> 16;
                int gCol = (hash & 0x00FF00) >> 8;
                int bCol = hash & 0xFF;
                if (rCol + gCol + bCol < 380) {
                    rCol = Math.min(255, rCol + 100);
                    gCol = Math.min(255, gCol + 100);
                    bCol = Math.min(255, bCol + 100);
                }
                g2d.setColor(new java.awt.Color(rCol, gCol, bCol));
                g2d.drawRect(r.rect.x + offsetX, r.rect.y + offsetY, r.rect.width, r.rect.height);
            }
        }
    }

    /** 窗口坐标命中；可见组件优先，隐藏占位只在没有可见组件时兜底。 */
    public Runtime hitTest(int x, int y, boolean includeHidden) {
        int localX = x - offsetX;
        int localY = y - offsetY;
        Runtime hiddenHit = null;
        for (int i = ordered.size() - 1; i >= 0; i--) {
            Runtime r = ordered.get(i);
            boolean visible = isEffectivelyVisible(r);
            if (!visible && !includeHidden)
                continue;
            int pad = r.component instanceof BarWidget ? ((BarWidget) r.component).getHitPadLeft() : 0;
            boolean hit = localX >= r.rect.x - pad && localX < r.rect.x + r.rect.width
                    && localY >= r.rect.y && localY < r.rect.y + r.rect.height;
            if (!hit)
                continue;
            if (visible)
                return r;
            if (hiddenHit == null)
                hiddenHit = r;
        }
        return hiddenHit;
    }

    /** 动态可见性：总开关 + 条目开关 + 无襟翼机隐藏襟翼角条。 */
    public void updateDynamicVisibility(HUDData data, boolean master) {
        for (Runtime r : runtimes) {
            boolean v = master && r.widget.visible;
            if (v && MiniHudProps.TYPE_BAR.equals(r.widget.type)
                    && MiniHudProps.BAR_FLAP_ANGLE.equals(r.widget.get(MiniHudProps.BAR_SOURCE, ""))
                    && data != null && !data.hasFlaps)
                v = false;
            r.component.setVisible(v);
        }
    }

    public void updateFieldData(ui.model.TelemetrySource source) {
        for (TextWidget w : textWidgets)
            w.updateData(source);
        for (BarWidget b : barWidgets)
            b.updateData(source);
    }

    /** 数据更新后组件首选尺寸变化（如机场列表增减）时返回 true，由窗口重新适配。 */
    public boolean syncPreferredSizes() {
        boolean changed = false;
        for (Runtime r : runtimes) {
            java.awt.Dimension d = r.component.getPreferredSize();
            if (d == null)
                continue;
            int w = Math.max(1, d.width);
            int h = Math.max(1, d.height);
            if (r.rect.width != w || r.rect.height != h)
                changed = true;
        }
        return changed;
    }
}
