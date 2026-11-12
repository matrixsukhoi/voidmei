package ui.overlay;

import java.awt.Color;
import java.awt.Component;
import java.awt.Container;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.Point;
import java.awt.Rectangle;
import java.awt.RenderingHints;

import com.alee.laf.panel.WebPanel;

import prog.Application;
import prog.Controller;
import prog.Service;
import prog.config.HUDSettings;
import prog.config.MiniHudFieldCatalog;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;
import prog.event.FlightDataListener;
import prog.util.Logger;
import ui.WebLafSettings;
import ui.base.DraggableOverlay;
import ui.component.HUDComponent;
import ui.minihud.MiniHudDefaultLayout;
import ui.minihud.MiniHudDocument;
import ui.minihud.MiniHudLayoutCodec;
import ui.minihud.MiniHudLegacyMigrator;
import ui.minihud.MiniHudScene;
import ui.minihud.MiniHudSceneBuilder;
import ui.minihud.editor.MiniHudEditor;
import ui.minihud.editor.MiniHudEditorHost;
import ui.overlay.logic.HUDCalculator;
import ui.overlay.model.HUDData;
import ui.util.OverlayStyleHelper;

/**
 * MiniHUD：自由画布驱动的组件化自绘 overlay。
 * 文档中的组件全部是绝对坐标的原子组件；窗口跟随内容包围盒，
 * 编辑态额外留出工作区余量，游戏态紧贴内容。
 */
public class MiniHUDOverlay extends DraggableOverlay implements FlightDataListener, MiniHudEditorHost {

    private static final long serialVersionUID = 1L;
    private static final int LAYOUT_PADDING = 45;

    public MiniHUDOverlay() {
        super();
        setTitle("MiniHUD");
    }

    private MinimalHUDContext ctx;
    private MiniHudDocument doc;
    private MiniHudScene scene;
    private MiniHudEditor editor;
    private boolean contentVisible = true;
    private boolean editorEditing;
    private boolean settingsLayoutDebug;

    int blinkTicks = 1;
    int blinkCheckTicks = 0;
    public boolean warnRH;
    public boolean warnVne;
    public Boolean blinkX = false;
    public Boolean blinkActing = false;

    private ui.component.WarningOverlay warningOverlay;
    private WebPanel panel;

    private HUDSettings hudSettings;
    private Controller controller;
    private Service service;
    private Container root;

    // 文档原点的屏幕锚点：内容增删/字体变化时保持 HUD 不整体漂移
    private int anchorX;
    private int anchorY;
    private boolean anchorInitialized;
    private int lastWorkX;
    private int lastWorkY;
    private int lastPadLeft = LAYOUT_PADDING;
    private boolean windowFrozen;

    public void setFrameOpaque() {
        OverlayStyleHelper.applyTransparentStyle(this);
    }

    public void initpanel() {
        panel.setWebColoredBackground(false);
        panel.setBackground(new Color(0, 0, 0, 0));
    }

    public void drawBlinkX(Graphics2D g) {
        if (blinkX && warningOverlay != null) {
            warningOverlay.draw(g, 0, 0, getWidth(), getHeight(), blinkActing);
            blinkCheckTicks += 1;
            if (blinkCheckTicks % blinkTicks == 0)
                blinkActing = !blinkActing;
        }
    }

    public void initPreview(Controller c, HUDSettings settings) {
        Logger.info("MinimalHUD", "initPreview called");
        init(c, null, settings);
        this.getWebRootPaneUI().setTopBg(Application.previewColor);
        this.getWebRootPaneUI().setMiddleBg(Application.previewColor);
        installPreviewEditor();
        applyPreviewStyle();
        this.setCursor(null);
        setVisible(true);
    }

    /** 安装预览编辑器（幂等）；离屏快照也可直接调用。 */
    public void installPreviewEditor() {
        if (editor == null && panel != null) {
            editor = new MiniHudEditor(this);
            editor.install(panel);
        }
    }

    public MiniHudEditor getEditor() {
        return editor;
    }

    @Override
    public MiniHudScene getScene() {
        return scene;
    }

    @Override
    public MiniHudDocument getDoc() {
        return doc;
    }

    @Override
    public void setDoc(MiniHudDocument doc) {
        this.doc = doc != null ? doc : MiniHudDefaultLayout.create();
    }

    @Override
    public MinimalHUDContext getCtx() {
        return ctx;
    }

    @Override
    public boolean isPreviewMode() {
        return service == null;
    }

    @Override
    public void commitLayout() {
        if (hudSettings == null || doc == null)
            return;
        hudSettings.saveMiniHudLayout(MiniHudLayoutCodec.encode(doc));
        // 组件移动可能改变内容包围盒并带动窗口位置；同步保存新的窗口位置
        saveCurrentPosition();
    }

    @Override
    public void repaintEditor() {
        repaint();
    }

    @Override
    public Point getEditorLocation() {
        return getLocation();
    }

    @Override
    public void setEditorLocation(int x, int y) {
        setLocation(x, y);
    }

    @Override
    public void saveEditorWindowPosition() {
        updateAnchorFromWindow();
        saveCurrentPosition();
    }

    public void saveCurrentPosition() {
        if (hudSettings == null)
            return;
        // 持久化“只读态窗口位置”，而不是编辑工作区窗口位置：
        // 编辑态窗口额外含工作区余量，直接存左上角会让重启后的 HUD 漂移。
        int sx = getLocation().x;
        int sy = getLocation().y;
        if (anchorInitialized && scene != null) {
            Rectangle b = scene.bounds(false);
            sx = anchorX - padLeftForScene() + b.x;
            sy = anchorY - LAYOUT_PADDING + b.y;
        }
        hudSettings.saveWindowPosition(sx, sy);
    }

    private int padLeftForScene() {
        return scene != null ? Math.max(LAYOUT_PADDING, scene.getMaxDrawPadLeft()) : LAYOUT_PADDING;
    }

    private void updateAnchorFromWindow() {
        anchorX = getX() + lastPadLeft - lastWorkX;
        anchorY = getY() + LAYOUT_PADDING - lastWorkY;
        anchorInitialized = true;
    }

    public void reinitConfig() {
        Logger.info("MinimalHUD", "reinitConfig called");
        if (hudSettings == null)
            return;
        if (editor != null && editor.isDragging())
            return;

        ctx = MinimalHUDContext.create(hudSettings);
        String encoded = hudSettings.getMiniHudLayout();
        MiniHudDocument decoded = MiniHudLayoutCodec.decode(encoded);
        if (decoded == null && encoded != null && encoded.startsWith("V1;"))
            decoded = MiniHudLegacyMigrator.fromV1(encoded);
        if (decoded == null) {
            if (encoded != null && !encoded.isEmpty())
                Logger.warn("MinimalHUD", "MiniHUD 布局无效/版本过旧，回退默认布局");
            decoded = MiniHudDefaultLayout.create();
        }
        doc = decoded;

        buildScene();
        layoutAndFit();
        // 字体/缩放变化会改变包围盒并移动窗口，保存新位置避免重启漂移
        saveCurrentPosition();
        repaint();
    }

    private void buildScene() {
        contentVisible = hudSettings.isMiniHudContentVisible();
        settingsLayoutDebug = hudSettings.getBool("enableLayoutDebug", false);
        scene = MiniHudSceneBuilder.build(doc, ctx, hudSettings, hudSettings.getMiniHudFieldCatalog(),
                service == null, service, contentVisible);
        applyLayoutDebug();
    }

    private void applyLayoutDebug() {
        if (scene != null)
            scene.setDebug(editorEditing || settingsLayoutDebug);
    }

    @Override
    public void setEditorEditing(boolean editing) {
        if (editorEditing == editing)
            return;
        editorEditing = editing;
        applyLayoutDebug();
        layoutAndFit();
        repaint();
    }

    @Override
    public void setWindowFrozen(boolean frozen) {
        this.windowFrozen = frozen;
        if (!frozen)
            layoutAndFit();
    }

    @Override
    public void rebuildSceneFromDoc() {
        if (doc == null || ctx == null)
            return;
        buildScene();
        layoutAndFit();
        repaint();
    }

    @Override
    public void refreshLayout() {
        if (scene == null)
            return;
        scene.layout();
        repaint();
    }

    @Override
    public void resetToDefaultLayout() {
        doc = MiniHudDefaultLayout.create();
        rebuildSceneFromDoc();
        commitLayout();
    }

    @Override
    public MiniHudFieldCatalog getFieldCatalog() {
        return hudSettings != null ? hudSettings.getMiniHudFieldCatalog() : MiniHudFieldCatalog.empty();
    }

    @Override
    public Component getEditorComponent() {
        return panel;
    }

    /**
     * 解算场景并按内容包围盒设置窗口。
     * 编辑态在内容外留工作区余量，游戏态紧贴内容，避免透明区域过大。
     */
    private void layoutAndFit() {
        if (scene == null || ctx == null)
            return;
        scene.layout();
        if (windowFrozen) {
            repaint();
            return;
        }

        Rectangle b = scene.bounds(editorEditing);
        int margin = editorEditing ? Math.max(60, ctx.hudFontSize * 3) : 0;
        int workX = b.x - margin;
        int workY = b.y - margin;
        int workW = b.width + margin * 2;
        int workH = b.height + margin * 2;

        int padLeft = padLeftForScene();
        int windowW = workW + padLeft + LAYOUT_PADDING;
        int windowH = workH + LAYOUT_PADDING * 2;
        // 预览工具条/最小可操作区域
        if (service == null) {
            int minW = Math.max(260, ctx.hudFontSize * 12);
            if (windowW < minW)
                windowW = minW;
            if (windowH < 90)
                windowH = 90;
        }

        if (!anchorInitialized) {
            anchorX = ctx.windowX + padLeft - workX;
            anchorY = ctx.windowY + LAYOUT_PADDING - workY;
            anchorInitialized = true;
        }
        int wx = anchorX - padLeft + workX;
        int wy = anchorY - LAYOUT_PADDING + workY;
        lastWorkX = workX;
        lastWorkY = workY;
        lastPadLeft = padLeft;

        scene.setOffset(padLeft - workX, LAYOUT_PADDING - workY);
        setBounds(wx, wy, windowW, windowH);
    }

    public void init(Controller c, Service s, HUDSettings settings) {
        Logger.info("MinimalHUD", "init called");
        service = s;
        controller = c;
        this.hudSettings = settings;
        this.setLayout(new java.awt.BorderLayout());
        setFrameOpaque();

        warningOverlay = new ui.component.WarningOverlay();
        reinitConfig();

        panel = new WebPanel() {
            private static final long serialVersionUID = -9061280572815010060L;

            public void paintComponent(Graphics g) {
                Graphics2D g2d = (Graphics2D) g;
                g2d.setPaintMode();
                g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
                g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);
                g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
                        RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
                g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING, RenderingHints.VALUE_COLOR_RENDER_SPEED);
                if (scene != null)
                    scene.render(g2d);
                if (editor != null)
                    editor.paint(g2d);
                drawBlinkX(g2d);
            }
        };
        panel.setOpaque(false);
        panel.setWebColoredBackground(false);

        this.setContentPane(panel);

        long loopMsLong = controller != null ? controller.serviceLoopIntervalMs : DEFAULT_REFRESH_INTERVAL;
        int loopMs = (int) loopMsLong;
        blinkTicks = (int) ((1000 / loopMs) >> 3);
        if (blinkTicks == 0)
            blinkTicks = 1;
        refreshInterval = loopMs;

        setTitle("miniHUD");
        WebLafSettings.setWindowOpaque(this);
        root = this.getContentPane();

        if (service != null) {
            subscribeToEvents();
            setVisible(true);
        }
    }

    /**
     * @deprecated 事件驱动后无轮询；保留以兼容 DraggableOverlay 接口。
     */
    public void updateString() {
        if (root != null)
            root.repaint();
    }

    public void drawTick() {
        if (root != null)
            root.repaint();
    }

    // --- Event-Driven Update ---

    private static final long DEFAULT_REFRESH_INTERVAL = 100;
    private long refreshInterval = DEFAULT_REFRESH_INTERVAL;
    private long lastRefreshTime = 0;

    @Override
    public void onFlightData(FlightDataEvent event) {
        long now = System.currentTimeMillis();
        if (now - lastRefreshTime < refreshInterval)
            return;
        lastRefreshTime = now;

        javax.swing.SwingUtilities.invokeLater(() -> {
            updateFromEvent(event);
            if (root != null)
                root.repaint();
        });
    }

    private void updateFromEvent(FlightDataEvent event) {
        if (ctx == null || scene == null)
            return;

        HUDData data = event.getHudData();
        if (data == null) {
            data = HUDCalculator.calculate(event, service,
                    prog.fm.FMManager.getInstance().current().blkx, hudSettings, ctx);
        }
        for (HUDComponent comp : scene.components)
            comp.onDataUpdate(data);
        // 配置字段：EDT 上直接读 TelemetrySource（FlightInfo/FieldOverlay 同款先例）
        scene.updateFieldData(service);
        scene.updateDynamicVisibility(data, contentVisible);

        warnVne = data.warnVne;
        warnRH = data.warnAltitude;
        blinkX = event.getPayload().fatalWarn;
    }

    public void subscribeToEvents() {
        FlightDataBus.getInstance().register(this);
    }

    public void unsubscribeFromEvents() {
        FlightDataBus.getInstance().unregister(this);
    }

    @Override
    public void run() {
        // 事件驱动 - 无轮询
    }

    @Override
    public void dispose() {
        if (editor != null) {
            editor.dispose();
            editor = null;
        }
        unsubscribeFromEvents();
        super.dispose();
    }
}
