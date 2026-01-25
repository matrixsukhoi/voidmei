package ui.overlay;

import java.awt.Color;
import java.awt.Container;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.util.List;
import java.util.ArrayList;

import com.alee.laf.panel.WebPanel;

import prog.Application;
import prog.util.Logger;
import prog.Controller;
import prog.OtherService;
import prog.Service;
import prog.event.FlightDataBus;
import prog.event.FlightDataEvent;
import prog.event.FlightDataListener;
import prog.config.ConfigurationService;
import prog.config.HUDSettings;
import ui.WebLafSettings;
import ui.base.DraggableOverlay;

/**
 * MinimalHUD overlay for displaying compact flight information.
 * Being migrated to event-driven architecture.
 */
import ui.overlay.model.HUDData;
import ui.overlay.logic.HUDCalculator;
import ui.component.HUDComponent;

public class MinimalHUD extends DraggableOverlay implements FlightDataListener {
    private static final long serialVersionUID = 1L;

    private MinimalHUDContext ctx;

    // Reactive Components List
    private List<HUDComponent> components = new ArrayList<>();

    int blinkTicks = 1;
    int blinkCheckTicks = 0;
    public boolean warnRH;
    public boolean warnVne;

    private ui.component.CrosshairGauge crosshairGauge;
    private ui.component.FlapAngleBar flapAngleBar;
    private ui.component.WarningOverlay warningOverlay;
    private ui.component.CompassGauge compassGauge;
    private ui.component.AttitudeIndicatorGauge attitudeIndicatorGauge;
    private java.util.List<ui.component.row.HUDRow> hudRows;
    private boolean firstDraw = true;

    private ConfigurationService configService;
    private HUDSettings hudSettings;

    public void setFrameOpaque() {
        this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
        this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
        this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
        this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明
        setShadeWidth(0);
    }

    public void initpanel() {
        panel.setWebColoredBackground(false);
        panel.setBackground(new Color(0, 0, 0, 0));
    }

    public Boolean blinkX = false;
    public Boolean blinkActing = false;

    public void drawBlinkX(Graphics2D g) {
        // 高度警告标记 - now using reusable component
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

    public int throttley = 0;
    private ui.component.LinearGauge throttleBar;
    public int OilX = 0;
    public int aoaY = 0;
    public boolean inAction = false;
    public Color throttleColor;
    public Color aoaColor;
    public Color aoaBarColor;
    public int throttleLineWidth = 1;

    // Core state and geometry
    private Controller controller;
    private WebPanel panel;

    private Service service;
    private String lines[];
    private Container root;

    private String relEnergy;

    public void initPreview(Controller c) {
        Logger.info("MinimalHUD", "initPreview called");
        init(c, null, null);

        this.getWebRootPaneUI().setTopBg(Application.previewColor);
        this.getWebRootPaneUI().setMiddleBg(Application.previewColor);
        setupDragListeners();
        applyPreviewStyle();
        this.setCursor(null);
        setVisible(true);
    }

    public void saveCurrentPosition() {
        if (hudSettings != null) {
            hudSettings.saveWindowPosition(getLocation().x, getLocation().y);
        }
    }

    public void reinitConfig() {
        Logger.info("MinimalHUD", "reinitConfig called");

        if (configService == null && controller != null) {
            configService = controller.configService;
        }

        if (configService != null) {
            hudSettings = configService.getHUDSettings();
        } else {
            return;
        }

        // Create Immutable Context
        ctx = MinimalHUDContext.create(hudSettings);

        // Apply dimensions
        if (hudSettings.isDisplayCrosshair())
            this.setBounds(ctx.windowX, ctx.windowY, ctx.width * 2, ctx.height);
        else
            this.setBounds(ctx.windowX, ctx.windowY, ctx.width, ctx.height);

        // Setup Layout Engine
        initModernLayout();

        applyStyleToComponents();
        updateComponents();
        firstDraw = true;

        repaint();
    }

    public void init(Controller c, Service s, OtherService os) {
        Logger.info("MinimalHUD", "init called");
        service = s;
        controller = c;
        this.setLayout(new java.awt.BorderLayout());
        setFrameOpaque();

        reinitConfig();

        lines = new String[6];
        String spdPre = hudSettings.isSpeedLabelDisabled() ? "" : "SPD";
        String altPre = hudSettings.isAltitudeLabelDisabled() ? "" : "ALT";
        String sepPre = hudSettings.isSEPLabelDisabled() ? "" : "SEP";

        lines[0] = spdPre + String.format("%5s", "360");
        lines[1] = altPre + String.format("%5s", "1024");
        lines[3] = sepPre + String.format("%5s", "30");
        lines[4] = "G" + String.format("%5s", "2.0");
        lines[2] = "F" + String.format("%3s", "100");
        lines[2] += "BRK";
        lines[2] += "GEAR";
        throttley = 100;
        aoaY = 10;
        throttleColor = Application.colorShadeShape;
        lineAoA = String.format("α%3.0f", 20.0);
        relEnergy = "E114514";

        if (hudSettings.isAoADisabled()) {
            lineAoA = "";
            relEnergy = "";
        }
        // aoaLength = ... (Removed, using ctx.aoaLength)
        if (ctx != null && aoaY > ctx.rightDraw)
            aoaY = ctx.rightDraw;
        aoaColor = Application.colorNum;
        aoaBarColor = Application.colorNum;

        initComponentsLayout();

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

                if (modernLayout != null) {
                    modernLayout.doLayout();
                    modernLayout.render(g2d);
                }

                drawBlinkX(g2d);
            }
        };
        panel.setOpaque(false);
        panel.setWebColoredBackground(false);

        this.setContentPane(panel);

        blinkTicks = (int) ((1000 / controller.freqService) >> 3);
        if (blinkTicks == 0)
            blinkTicks = 1;

        refreshInterval = (long) (controller.freqService * 1.0); // Match freqService

        setTitle("miniHUD");
        WebLafSettings.setWindowOpaque(this);
        root = this.getContentPane();
        // this.createBufferStrategy(2);

        // Subscribe to events for game mode
        if (service != null) {
            subscribeToEvents();
            setVisible(true);
        }

        updateComponents();
    }

    public long hudCheckMili;
    private String lineAoA;

    double realSpdPitch;

    private double maneuverIndex;
    private int maneuverIndexLen;

    private int maneuverIndexLen30;
    private int maneuverIndexLen10;
    private int maneuverIndexLen20;
    private int maneuverIndexLen40;
    private int maneuverIndexLen50;
    private boolean disableAttitude;

    /**
     * Legacy update logic.
     * 
     * @deprecated Replaced by event-driven implementation in updateFromEvent().
     */
    public void updateString() {
        // No-op. Logic moved to updateFromEvent().
        if (root != null)
            root.repaint();
    }

    private void updateComponents() {
        boolean textVisible = hudSettings.drawHUDText();

        if (flapAngleBar != null) {
            flapAngleBar.setVisible(textVisible && hudSettings.enableFlapAngleBar());
        }
        if (compassGauge != null) {
            compassGauge.setVisible(textVisible);
        }
        if (attitudeIndicatorGauge != null) {
            attitudeIndicatorGauge.setVisible(textVisible && hudSettings.drawHUDAttitude() && !disableAttitude);
        }
        if (crosshairGauge != null) {
            crosshairGauge.setVisible(hudSettings.isDisplayCrosshair());
            // Dynamic position based on current Width/CrossX
            if (ctx != null) {
                // Position handled by ModernHUDLayoutEngine
            }
        }
        if (throttleBar != null) {
            throttleBar.setVisible(textVisible);
        }
        if (hudRows != null) {
            for (ui.component.HUDComponent row : hudRows) {
                row.setVisible(textVisible);
            }
        }

        if (hudRows != null && hudRows.size() >= 5) {
            // Row 0: AoA/Speed (Legacy restored for Preview compatibility)
            ((ui.component.row.HUDAkbRow) hudRows.get(0)).update(lines[0], warnVne,
                    lineAoA, aoaY, aoaColor, aoaBarColor);
            // Row 1: Energy/Altitude (Legacy restored for Preview compatibility)
            ((ui.component.row.HUDEnergyRow) hudRows.get(1)).update(lines[1], warnRH,
                    relEnergy, aoaColor);
            // Row 2: Standard (Flaps/Gear)
            ((ui.component.row.HUDTextRow) hudRows.get(2)).update(lines[2], inAction);
            // Row 3: Standard (SEP)
            ((ui.component.row.HUDTextRow) hudRows.get(3)).update(lines[3], false);
            // Row 4: Maneuver (G)
            ((ui.component.row.HUDManeuverRow) hudRows.get(4)).update(lines[4], false, maneuverIndex,
                    maneuverIndexLen, maneuverIndexLen10, maneuverIndexLen20, maneuverIndexLen30,
                    maneuverIndexLen40, maneuverIndexLen50);

            for (ui.component.row.HUDRow row : hudRows) {
                row.setVisible(textVisible);
            }
        }

        if (throttleBar != null) {
            int throttleValue = 0;
            if (service != null && service.sState != null) {
                throttleValue = service.sState.throttle;
            }
            throttleBar.update(throttleValue, String.format("%3d", throttleValue));
            throttleBar.setVisible(textVisible);
        }
    }

    public void drawTick() {

        // updateString();
        root.repaint();
    }

    // --- Event-Driven Update ---

    // Throttling for refresh rate
    private static final long DEFAULT_REFRESH_INTERVAL = 100; // ms
    private long refreshInterval = DEFAULT_REFRESH_INTERVAL;
    private long lastRefreshTime = 0;

    @Override
    public void onFlightData(FlightDataEvent event) {
        // Throttle updates based on configured refresh interval
        long now = System.currentTimeMillis();
        if (now - lastRefreshTime < refreshInterval) {
            return; // Skip this update, too soon
        }
        lastRefreshTime = now;

        javax.swing.SwingUtilities.invokeLater(() -> {
            updateFromEvent(event);
            if (root != null)
                root.repaint();
        });
    }

    private void updateFromEvent(FlightDataEvent event) {
        if (ctx == null)
            return;

        // 1. Calculate Data Snapshot using Pure Event Data
        HUDData data = HUDCalculator.calculate(event, controller.getBlkx(), hudSettings, ctx);

        // 2. Dispatch to Reactive Components
        for (HUDComponent comp : components) {
            comp.onDataUpdate(data);
        }

        // 3. Update Legacy Components (Bridge) & Global State
        warnVne = data.warnVne;
        warnRH = data.warnAltitude;
        blinkX = Boolean.parseBoolean(event.getData().get("fatalWarn"));

        if (hudRows != null && hudRows.size() >= 5) {
            // Row 2: Standard (Flaps/Gear)
            // Row 3: Standard (SEP)
            // Row 4: Maneuver (G)
            // These still use legacy update() in my previous view of updateComponents()?
            // Wait, I should refactor them to uses onDataUpdate directly.
            // If I don't, I must manually call update() here.

            // Let's call a legacy bridge method explicitly
            updateLegacyComponents(data);
        }

        if (throttleBar != null) {
            throttleBar.update(data.throttle, String.format("%3d", data.throttle));
        }
    }

    private void updateLegacyComponents(HUDData data) {
        if (hudRows == null || hudRows.size() < 5)
            return;

        // Row 0, 1 are refactored (Akb, Energy). They use onDataUpdate.
        // Row 2: Flaps/Gear
        ((ui.component.row.HUDTextRow) hudRows.get(2)).update(data.flapsStr, data.warnConfiguration);
        // Row 3: SEP
        ((ui.component.row.HUDTextRow) hudRows.get(3)).update(data.sepStr, false);
        // Row 4: Maneuver
        // ManeuverRow update signature is complex.
        ((ui.component.row.HUDManeuverRow) hudRows.get(4)).update(data.maneuverRowStr, false, data.maneuverIndex,
                maneuverIndexLen, maneuverIndexLen10, maneuverIndexLen20, maneuverIndexLen30,
                maneuverIndexLen40, maneuverIndexLen50);
        // Note: maneuverIndexLen variables are member fields of MinimalHUD calculated
        // in legacy loop.
        // We need to recalculate them or move calculation to Calculator.
        // Ideally Calculator provides "maneuverBarLength" or similar?
        // Or we calculate here based on data.maneuverIndex?
        if (ctx != null) {
            int rightDraw = ctx.rightDraw;
            maneuverIndexLen = (int) Math.round(data.maneuverIndex / 0.5 * rightDraw);
            maneuverIndexLen10 = (int) Math.round(0.1 / 0.5 * rightDraw);
            maneuverIndexLen20 = (int) Math.round(0.2 / 0.5 * rightDraw);
            maneuverIndexLen30 = (int) Math.round(0.3 / 0.5 * rightDraw);
            maneuverIndexLen40 = (int) Math.round(0.4 / 0.5 * rightDraw);
            maneuverIndexLen50 = (int) Math.round(0.5 / 0.5 * rightDraw);
        }
    }

    /**
     * Subscribe to flight data events.
     */
    public void subscribeToEvents() {
        FlightDataBus.getInstance().register(this);
    }

    /**
     * Unsubscribe from flight data events.
     */
    public void unsubscribeFromEvents() {
        FlightDataBus.getInstance().unregister(this);
    }

    @Override
    public void run() {
        // Event-driven - no polling needed
        // Kept for compatibility with DraggableOverlay interface
    }

    @Override
    public void dispose() {
        unsubscribeFromEvents();
        super.dispose();
    }

    protected void initComponentsLayout() {
        components.clear(); // Ensure list is clean on re-init

        // 0. Aux Overlays
        warningOverlay = new ui.component.WarningOverlay();
        flapAngleBar = new ui.component.FlapAngleBar();
        components.add(flapAngleBar); // warningOverlay is not a HUDComponent? Check later. It draws directly.

        // 1. Compass
        compassGauge = new ui.component.CompassGauge(ctx.roundCompass);
        components.add(compassGauge);

        // 2. Attitude
        attitudeIndicatorGauge = new ui.component.AttitudeIndicatorGauge();
        components.add(attitudeIndicatorGauge);

        // 3. Crosshair
        crosshairGauge = new ui.component.CrosshairGauge();
        components.add(crosshairGauge);

        // 4. Rows
        hudRows = new java.util.ArrayList<>();

        ui.component.row.HUDAkbRow row0 = new ui.component.row.HUDAkbRow(0, ctx.drawFont, ctx.hudFontSize,
                ctx.drawFontSmall, ctx.rightDraw, ctx.lineWidth);
        row0.setTemplate(lines[0], lineAoA);
        hudRows.add(row0);

        ui.component.row.HUDEnergyRow row1 = new ui.component.row.HUDEnergyRow(1, ctx.drawFont, ctx.hudFontSize,
                ctx.drawFontSmall, ctx.rightDraw);
        row1.setTemplate(lines[1], relEnergy);
        hudRows.add(row1);

        ui.component.row.HUDTextRow row2 = new ui.component.row.HUDTextRow(2, ctx.drawFont, ctx.hudFontSize);
        row2.setTemplate(lines[2]);
        hudRows.add(row2);

        ui.component.row.HUDTextRow row3 = new ui.component.row.HUDTextRow(3, ctx.drawFont, ctx.hudFontSize);
        row3.setTemplate(lines[3]);
        hudRows.add(row3);

        ui.component.row.HUDManeuverRow row4 = new ui.component.row.HUDManeuverRow(4, ctx.drawFont, ctx.hudFontSize,
                ctx.rightDraw, ctx.halfLine,
                ctx.lineWidth,
                ctx.strokeThick, ctx.strokeThin);
        row4.setTemplate(lines[4]);
        hudRows.add(row4);

        for (ui.component.row.HUDRow row : hudRows) {
            components.add(row);
        }

        // 5. Bars
        throttleBar = new ui.component.LinearGauge("ThrottleBar", 110, true);
        components.add(throttleBar);

        initModernLayout();

        // Ensure everything is styled and updated before first paint
        applyStyleToComponents();
        updateComponents();
    }

    private void applyStyleToComponents() {
        if (ctx == null)
            return;

        if (crosshairGauge != null) {
            if (hudSettings.useTextureCrosshair()) {
                // Use loaded image from Context if available
                crosshairGauge.setTextureStyle(true, ctx.crosshairImageScaled, ctx.crossScale);
            } else {
                crosshairGauge.setStyleContext(hudSettings.getCrosshairScale());
            }
        }
        if (flapAngleBar != null) {
            // Dynamic width
            int responsiveWidth = (int) (ctx.hudFontSize * 6);
            flapAngleBar.setStyleContext(responsiveWidth, ctx.lineWidth + 2, ctx.drawFontSmall);
        }
        if (compassGauge != null) {
            compassGauge.setStyleContext(ctx.roundCompass, ctx.lineWidth, ctx.hudFontSize, ctx.hudFontSizeSmall,
                    ctx.drawFontSmall);
        }
        if (attitudeIndicatorGauge != null) {
            attitudeIndicatorGauge.setStyleContext(ctx.compassDiameter, ctx.compassRadius, ctx.compassInnerMarkRadius,
                    ctx.lineWidth, ctx.halfLine, ctx.drawFontSmall);
        }
        // Synchronize styles for Rows
        if (hudRows != null && hudRows.size() >= 5) {
            ((ui.component.row.HUDAkbRow) hudRows.get(0)).setStyle(ctx.drawFont, ctx.hudFontSize, ctx.drawFontSmall,
                    ctx.rightDraw,
                    ctx.lineWidth, (int) ctx.aoaLength);
            ((ui.component.row.HUDEnergyRow) hudRows.get(1)).setStyle(ctx.drawFont, ctx.hudFontSize, ctx.drawFontSmall,
                    ctx.rightDraw);
            ((ui.component.row.HUDTextRow) hudRows.get(2)).setStyle(ctx.drawFont, ctx.hudFontSize);
            ((ui.component.row.HUDTextRow) hudRows.get(3)).setStyle(ctx.drawFont, ctx.hudFontSize);
            ((ui.component.row.HUDManeuverRow) hudRows.get(4)).setStyle(ctx.drawFont, ctx.hudFontSize, ctx.rightDraw,
                    ctx.halfLine, ctx.lineWidth, ctx.strokeThick, ctx.strokeThin);
        }

        if (throttleBar != null) {
            // Re-calc explicit height for ThrottleBar if needed or use existing
            // throttley_max
            // Standardizing to relative size: 4.8 lines high (closer to legacy 4.75)
            int responsiveHeight = (int) (ctx.hudFontSize * 4.8);
            throttleBar.setStyleContext(responsiveHeight, ctx.barWidth, ctx.drawFontSSmall, ctx.drawFontSSmall);
        }
    }

    // --- Modern Layout Engine Integration ---
    private ui.layout.ModernHUDLayoutEngine modernLayout;

    private void initModernLayout() {
        // Apply Global Debug Setting
        boolean showCrosshair = hudSettings.isDisplayCrosshair();
        int layoutWidth = showCrosshair ? ctx.width * 2 : ctx.width;
        Logger.info("MinimalHUD", "initModernLayout: showCrosshair=" + showCrosshair + ", layoutWidth=" + layoutWidth);

        modernLayout = new ui.layout.ModernHUDLayoutEngine(layoutWidth, ctx.height);

        // Apply Global Debug Setting
        // [架构说明]
        // 这里手动传递配置而不是让 LayoutEngine 直接订阅 EventBus 是为了防止内存泄漏。
        // LayoutEngine 随 MinimalHUD 配置刷新而频繁销毁重建 (Transient Lifecycle)。
        // 如果它直接订阅全局单例 EventBus，旧实例会因无法自动注销而被长期持有，导致 "Zombie Listener" 泄漏。
        // 因此采用了由持有者 (MinimalHUD) 被动传递状态的设计。
        if (controller != null) {
            String debugVal = controller.getconfig("enableLayoutDebug");
            if (debugVal != null && !debugVal.isEmpty()) {
                modernLayout.setDebug(Boolean.parseBoolean(debugVal));
            }
        }

        // Use lineHeight from font size for responsive scaling
        modernLayout.setLineHeight(ctx.hudFontSize);

        if (components.isEmpty())
            return;

        Logger.info("MinimalHUD", "initModernLayout: Adding nodes. Components: " + components.size());

        // 3. Row 0 (New Anchor for Left Block)
        // Position: 2.1, 3.5 units
        ui.layout.HUDLayoutNode row0 = new ui.layout.HUDLayoutNode("row0", hudRows.get(0));
        row0.setRelativePosition(2.1, 3.5)
                .setAnchors(ui.layout.Anchor.TOP_LEFT, ui.layout.Anchor.TOP_LEFT);
        modernLayout.addNode(row0);

        // 4. Flap Bar (Child of Row 0)
        // Pos: 0, -1.85 (Above Row 0)
        ui.layout.HUDLayoutNode flapNode = new ui.layout.HUDLayoutNode("flap", flapAngleBar);
        flapNode.setParent(row0)
                .setRelativePosition(0, -0.1)
                .setAnchors(ui.layout.Anchor.TOP_LEFT, ui.layout.Anchor.BOTTOM_LEFT);
        modernLayout.addNode(flapNode);

        // 5. Rows Chain & Right-Side Attachments
        ui.layout.HUDLayoutNode prevRow = row0;
        ui.layout.HUDLayoutNode row2 = null;
        ui.layout.HUDLayoutNode row4 = null;

        for (int i = 1; i < hudRows.size(); i++) {
            ui.layout.HUDLayoutNode rowNode = new ui.layout.HUDLayoutNode("row" + i, hudRows.get(i));
            // Standard Line Spacing: 1.4 units (down from previous row top)
            rowNode.setParent(prevRow)
                    .setRelativePosition(0, 0.1)
                    .setAnchors(ui.layout.Anchor.BOTTOM_LEFT, ui.layout.Anchor.TOP_LEFT);
            modernLayout.addNode(rowNode);
            prevRow = rowNode;

            if (i == 2) {
                row2 = rowNode;
            } else if (i == 4) {
                row4 = rowNode;
            }
        }

        // 6. Right Side Instruments (Attached to Row 2)
        if (row2 != null) {
            // Attitude (Child of Row 2)
            // Pos: 3.5, 0.15 (Right of Row 2)
            ui.layout.HUDLayoutNode attitudeNode = new ui.layout.HUDLayoutNode("attitude", attitudeIndicatorGauge);
            attitudeNode.setParent(row2)
                    .setRelativePosition(0, 0.1)
                    .setAnchors(ui.layout.Anchor.BOTTOM_CENTER, ui.layout.Anchor.CENTER);
            modernLayout.addNode(attitudeNode);

            // Compass (Child of Attitude)
            // Pos: 1.8, 0.05 (Right of Attitude)
            ui.layout.HUDLayoutNode compassNode = new ui.layout.HUDLayoutNode("compass", compassGauge);
            compassNode.setParent(row2)
                    .setRelativePosition(0, 0.1)
                    .setAnchors(ui.layout.Anchor.BOTTOM_RIGHT, ui.layout.Anchor.TOP_RIGHT);
            modernLayout.addNode(compassNode);
        }

        // Throttle Bar (Independent Root)
        // Position: 0.5, 9.1 units
        ui.layout.HUDLayoutNode throttleNode = new ui.layout.HUDLayoutNode("throttle", throttleBar);
        throttleNode.setParent(row4)
                .setRelativePosition(-0.1, 0)
                .setAnchors(ui.layout.Anchor.BOTTOM_LEFT, ui.layout.Anchor.BOTTOM_RIGHT);
        modernLayout.addNode(throttleNode);

        // Crosshair (Independent, Center of Attention)
        if (hudSettings.isDisplayCrosshair()) {
            ui.layout.HUDLayoutNode crosshairNode = new ui.layout.HUDLayoutNode("crosshair", crosshairGauge);
            crosshairNode.setRelativePosition(0, 0)
                    .setAnchors(ui.layout.Anchor.MIDDLE_RIGHT, ui.layout.Anchor.MIDDLE_RIGHT);
            modernLayout.addNode(crosshairNode);
        }

        // Force layout calculation to populate sortedNodes for logging
        modernLayout.doLayout();
        modernLayout.logTopology();
    }
}
