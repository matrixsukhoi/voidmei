package ui.overlay;

import java.awt.Color;
import java.awt.Container;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;

import com.alee.laf.panel.WebPanel;

import prog.Application;
import prog.util.Logger;
import prog.Controller;
import prog.Service;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;
import prog.event.FlightDataListener;
import prog.config.HUDSettings;
import prog.config.MiniHudFieldCatalog;
import ui.WebLafSettings;
import ui.base.DraggableOverlay;
import ui.util.OverlayStyleHelper;

import ui.overlay.model.HUDData;
import ui.overlay.logic.HUDCalculator;
import ui.component.HUDComponent;
import ui.minihud.MiniHudDefaultLayout;
import ui.minihud.MiniHudLayoutCodec;
import ui.minihud.MiniHudLayoutDoc;
import ui.minihud.MiniHudScene;
import ui.minihud.MiniHudSceneBuilder;
import ui.minihud.TextRowWidget;
import ui.minihud.editor.MiniHudEditor;
import ui.minihud.editor.MiniHudEditorHost;

/**
 * MiniHUD：组件化自绘 overlay。
 * 内容与布局由 MiniHudLayoutDoc 描述（旧硬编码布局已迁移为默认文档），
 * 编辑器（预览态）直接修改文档并重建场景。
 */
public class MiniHUDOverlay extends DraggableOverlay implements FlightDataListener, MiniHudEditorHost {

    private static final long serialVersionUID = 1L;
    private static final int LAYOUT_PADDING = 45;

    public MiniHUDOverlay() {
        super();
        setTitle("MiniHUD");
    }

    private MinimalHUDContext ctx;
    private MiniHudLayoutDoc doc;
    private MiniHudScene scene;
    private MiniHudEditor editor;
    private boolean contentVisible = true;
    private boolean editorEditing; // 编辑器处于编辑态（显示布局调试框）
    private boolean settingsLayoutDebug; // 设置里的“布局调试”开关

    int blinkTicks = 1;
    int blinkCheckTicks = 0;
    public boolean warnRH;
    public boolean warnVne;
    public Boolean blinkX = false;
    public Boolean blinkActing = false;

    private ui.component.WarningOverlay warningOverlay;
    private WebPanel panel;

    // HUDSettings 通过 init() 参数传入，不应从 Controller 获取 configService
    private HUDSettings hudSettings;

    private Controller controller;
    private Service service;
    private Container root;

    public void setFrameOpaque() {
        OverlayStyleHelper.applyTransparentStyle(this);
    }

    public void initpanel() {
        panel.setWebColoredBackground(false);
        panel.setBackground(new Color(0, 0, 0, 0));
    }

    public void drawBlinkX(Graphics2D g) {
        // 高度警告标记 - 由系统层 WarningOverlay 统一绘制（不进布局文档）
        if (blinkX && ctx != null) {
            if (warningOverlay != null) {
                warningOverlay.draw(g, 0, 0, ctx.width, ctx.height, blinkActing);
            }
            blinkCheckTicks += 1;
            if (blinkCheckTicks % blinkTicks == 0) {
                blinkActing = !blinkActing;
            }
        }
    }

    public void initPreview(Controller c, HUDSettings settings) {
        Logger.info("MinimalHUD", "initPreview called");
        init(c, null, settings);

        this.getWebRootPaneUI().setTopBg(Application.previewColor);
        this.getWebRootPaneUI().setMiddleBg(Application.previewColor);
        // 预览态由编辑器统一处理鼠标：条目拖动 / 空白拖窗（不再用 setupDragListeners）
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

    public MiniHudScene getScene() {
        return scene;
    }

    public MiniHudLayoutDoc getDoc() {
        return doc;
    }

    public MinimalHUDContext getCtx() {
        return ctx;
    }

    /** 仅设置窗预览态允许编辑；游戏态（service != null）一律只读。 */
    public boolean isPreviewMode() {
        return service == null;
    }

    /** 编辑器把当前文档写回配置（拖动结束/增删改时调用）。 */
    @Override
    public void commitLayout() {
        if (hudSettings != null && doc != null) {
            hudSettings.saveMiniHudLayout(MiniHudLayoutCodec.encode(doc));
        }
    }

    @Override
    public void repaintEditor() {
        repaint();
    }

    @Override
    public java.awt.Point getEditorLocation() {
        return getLocation();
    }

    @Override
    public void setEditorLocation(int x, int y) {
        setLocation(x, y);
    }

    @Override
    public void saveEditorWindowPosition() {
        saveCurrentPosition();
    }

    public void saveCurrentPosition() {
        if (hudSettings != null) {
            hudSettings.saveWindowPosition(getLocation().x, getLocation().y);
        }
    }

    public void reinitConfig() {
        Logger.info("MinimalHUD", "reinitConfig called");
        if (hudSettings == null)
            return;
        // 拖动中不做外部刷新，避免坐标被重置；松手保存后会再次触发刷新
        if (editor != null && editor.isDragging())
            return;

        // 1. 重建不可变样式快照
        ctx = MinimalHUDContext.create(hudSettings);

        // 2. 读取布局文档；空/坏串回退默认布局
        doc = MiniHudLayoutCodec.decodeOrDefault(hudSettings.getMiniHudLayout());

        // 3. 构建组件与布局 DAG（预览态用演示值，游戏态接 TelemetrySource）
        buildScene();
        this.setBounds(ctx.windowX, ctx.windowY, ctx.width, ctx.height);
        applyAutoSizing();
        repaint();
    }

    private void applyAutoSizing() {
        scene.engine.applyAutoSizing(this, LAYOUT_PADDING);
        // 预览工具条需要最小窗口，避免删空后无法点“默认/添加”
        if (service == null) {
            int minW = Math.max(260, ctx.hudFontSize * 12);
            if (getWidth() < minW)
                setSize(minW, getHeight());
            if (getHeight() < 90)
                setSize(getWidth(), 90);
        }
    }

    private void buildScene() {
        contentVisible = hudSettings.isMiniHudContentVisible();
        settingsLayoutDebug = hudSettings.getBool("enableLayoutDebug", false);
        scene = MiniHudSceneBuilder.build(doc, ctx, hudSettings, hudSettings.getMiniHudFieldCatalog(),
                service == null, service, contentVisible);
        applyLayoutDebug();
    }

    /** 编辑态强制显示布局调试框；只读态遵循设置里的“布局调试”开关。 */
    private void applyLayoutDebug() {
        if (scene != null) {
            scene.engine.setDebug(editorEditing || settingsLayoutDebug);
        }
    }

    @Override
    public void setEditorEditing(boolean editing) {
        editorEditing = editing;
        applyLayoutDebug();
        repaint();
    }

    @Override
    public void rebuildSceneFromDoc() {
        if (doc == null || ctx == null)
            return;
        buildScene();
        applyAutoSizing();
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
    public java.awt.Component getEditorComponent() {
        return panel;
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

                if (scene != null) {
                    scene.engine.doLayout();
                    scene.engine.render(g2d);
                }
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

    private static final long DEFAULT_REFRESH_INTERVAL = 100; // ms
    private long refreshInterval = DEFAULT_REFRESH_INTERVAL;
    private long lastRefreshTime = 0;

    @Override
    public void onFlightData(FlightDataEvent event) {
        long now = System.currentTimeMillis();
        if (now - lastRefreshTime < refreshInterval) {
            return;
        }
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

        for (HUDComponent comp : scene.components) {
            comp.onDataUpdate(data);
        }
        // 自定义数据字段：EDT 上直接读 TelemetrySource（FlightInfo/FieldOverlay 同款先例）
        for (TextRowWidget w : scene.liveFields) {
            w.updateData(service);
        }

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
