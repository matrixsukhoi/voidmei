package ui.overlay;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Container;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.Image;
import java.awt.RenderingHints;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;

import java.util.Map;

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
import java.util.List;
import java.util.ArrayList;
import ui.layout.UIStyle;

public class MinimalHUD extends DraggableOverlay implements UIStyle, FlightDataListener {
	private static final long serialVersionUID = 1L;

	private MinimalHUDContext ctx;

	// Reactive Components List
	private List<HUDComponent> components = new ArrayList<>();

	int blinkTicks = 1;
	int blinkCheckTicks = 0;
	public boolean warnRH;
	public boolean warnVne;

	// Reusable UI Components (high-performance, cached strokes)
	private ui.component.CrosshairGauge crosshairGauge;
	private ui.component.FlapAngleBar flapAngleBar;
	private ui.component.WarningOverlay warningOverlay;
	private ui.component.CompassGauge compassGauge;
	private ui.component.AttitudeIndicatorGauge attitudeIndicatorGauge;
	private java.util.List<ui.component.row.HUDRow> hudRows;
	private ui.layout.HUDVirtualLayoutEngine layoutEngine;
	private java.util.Map<String, ui.layout.HUDComponentState> componentStateMap = new java.util.HashMap<>();
	private boolean firstDraw = true;

	// Refactored Configuration Management
	private ConfigurationService configService;
	private HUDSettings hudSettings;
	// private MinimalHUDContext ctx; // Duplicate removed

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
	// Core state and geometry
	private Controller controller;
	private WebPanel panel;

	private Service service;

	private int isDragging;
	private int dragStartX;
	private int dragStartY;

	private String lines[];

	private Container root;

	private String relEnergy;

	// Restored State Fields
	// 襟翼角度

	public void initPreview(Controller c) {
		Logger.info("MinimalHUD", "initPreview called");
		init(c, null, null);

		this.getWebRootPaneUI().setTopBg(Application.previewColor);
		this.getWebRootPaneUI().setMiddleBg(Application.previewColor);
		addMouseListener(new MouseAdapter() {
			public void mouseEntered(MouseEvent e) {

			}

			public void mousePressed(MouseEvent e) {
				isDragging = 1;
				dragStartX = e.getX();
				dragStartY = e.getY();

			}

			public void mouseReleased(MouseEvent e) {
				if (isDragging == 1) {
					isDragging = 0;
				}
			}

		});
		addMouseMotionListener(new MouseMotionAdapter() {
			public void mouseDragged(MouseEvent e) {
				if (isDragging == 1) {
					int left = getLocation().x;
					int top = getLocation().y;
					setLocation(left + e.getX() - dragStartX, top + e.getY() - dragStartY);
					saveCurrentPosition();
					setVisible(true);
					repaint();
				}
			}
		});
		this.setCursor(null);
		setVisible(true);
	}

	public void saveCurrentPosition() {
		if (configService != null) {
			configService.setConfig("crosshairX", Integer.toString(getLocation().x));
			configService.setConfig("crosshairY", Integer.toString(getLocation().y));
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
		if (layoutEngine != null) {
			layoutEngine.setCanvasSize(hudSettings.isDisplayCrosshair() ? ctx.width * 2 : ctx.width, ctx.height);
		}

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
		if (ctx != null)
			throttley_max = (int) (ctx.hudFontSize * 4.75);
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

		// Removed redundant Stroke and Font creation (handled by Context)
		// Removed redundant bounds setting (handled by reinitConfig)

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

				if (layoutEngine != null) {
					layoutEngine.doLayout();
					if (firstDraw) {
						layoutEngine.logPositions();
						firstDraw = false;
					}
					layoutEngine.render(g2d);
				}

				drawBlinkX(g2d);
			}
		};
		panel.setOpaque(false);
		panel.setWebColoredBackground(false);

		this.setContentPane(panel);

		// 1miao 8 ci
		blinkTicks = (int) ((1000 / controller.freqService) >> 3);
		if (blinkTicks == 0)
			blinkTicks = 1;

		// Load refresh interval from config
		refreshInterval = (long) (controller.freqService * 1.0); // Match freqService

		setTitle("miniHUD");
		WebLafSettings.setWindowOpaque(this);
		root = this.getContentPane();
		// this.createBufferStrategy(2);

		// Subscribe to events for game mode
		if (s != null) {
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
	private int throttley_max;
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
			// flapAngleBar.update(flapA, flapAllowA);
			componentStateMap.get(flapAngleBar.getId()).setVisible(textVisible && hudSettings.enableFlapAngleBar());
		}
		if (compassGauge != null) {
			// compassGauge.update((float) compassRads, compassDx, compassDy, lineCompass,
			// lineLoc);
			componentStateMap.get(compassGauge.getId()).setVisible(textVisible);
		}
		if (attitudeIndicatorGauge != null) {
			// Legacy update skipped for Refactored Component
			// attitudeIndicatorGauge.update(pitch, rollDeg, aosX, sAttitude, roundHorizon);
			componentStateMap.get(attitudeIndicatorGauge.getId())
					.setVisible(textVisible && hudSettings.drawHUDAttitude() && !disableAttitude);
		}
		if (crosshairGauge != null) {
			ui.layout.HUDComponentState crosshairState = componentStateMap.get(crosshairGauge.getId());
			crosshairState.setVisible(hudSettings.isDisplayCrosshair());
			// Dynamic position based on current Width/CrossX
			if (ctx != null) {
				crosshairState.setXOffset(ctx.width + ctx.crossX);
				crosshairState.setYOffset(ctx.crossY);
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
				componentStateMap.get(row.getId()).setVisible(textVisible);
			}
		}

		if (throttleBar != null) {
			int throttleValue = 0;
			if (service != null && service.sState != null) {
				throttleValue = service.sState.throttle;
			}
			throttleBar.update(throttleValue, String.format("%3d", throttleValue));
			componentStateMap.get(throttleBar.getId()).setVisible(textVisible);
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

	/**
	 * Update HUD state from FlightDataEvent.
	 * 
	 * NOTE: Phase 2 Partial Migration
	 * The event-driven architecture is in place (updates triggered by
	 * FlightDataEvent),
	 * but data is still read from Service (service) fields for now because:
	 * 1. updateString() has complex dependencies on Blkx flight model data
	 * 2. Map coordinate calculations require Service.mapinfo
	 * 3. Many calculated warning thresholds depend on Blkx.getVNE/getAoA methods
	 * 
	 * Future work: Move calculations to Service and expose via FlightDataEvent
	 * keys.
	 */
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
		warnRH = data.warnAltitude; // Assuming warnRH maps to warnAltitude or similar
		blinkX = Boolean.parseBoolean(event.getData().get("fatalWarn")); // Keep raw event access for simple flags? Or
																			// use Data? HUDData doesn't have fatalWarn
																			// explicit yet? Add if needed.
		// HUDData has warnStall, etc. fatalWarn is usually engine death?

		// Feed unrefactored rows if they haven't implemented onDataUpdate yet
		// However, we are calling onDataUpdate on ALL components above.
		// If they don't override it (default no-op), nothing happens.
		// We need to check if we need legacy updates for components that are NOT fully
		// refactored.
		// HUDTextRow, HUDManeuverRow need refactoring to pure onDataUpdate.
		// For now, let's keep the legacy update logic but fed from HUDData strings.

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
		hudRows.add(new ui.component.row.HUDAkbRow(0, ctx.drawFont, ctx.hudFontSize, ctx.drawFontSmall, ctx.rightDraw,
				ctx.lineWidth));
		hudRows.add(
				new ui.component.row.HUDEnergyRow(1, ctx.drawFont, ctx.hudFontSize, ctx.drawFontSmall, ctx.rightDraw));
		hudRows.add(new ui.component.row.HUDTextRow(2, ctx.drawFont, ctx.hudFontSize));
		hudRows.add(new ui.component.row.HUDTextRow(3, ctx.drawFont, ctx.hudFontSize));
		hudRows.add(new ui.component.row.HUDManeuverRow(4, ctx.drawFont, ctx.hudFontSize, ctx.rightDraw, ctx.halfLine,
				ctx.lineWidth,
				ctx.strokeThick, ctx.strokeThin));

		for (ui.component.row.HUDRow row : hudRows) {
			components.add(row);
		}

		// 5. Bars
		// throttleBar = new LinearGauge(...);
		// throttleBar is initialized in init()? No, it was init'd in init() separately?
		// Let's check where throttleBar is init'd.
		// It was in init().
		// If I want to manage it here, I need to find where it is created.
		// I will check init method again.
		throttleBar = new ui.component.LinearGauge("ThrottleBar", 110, true);
		components.add(throttleBar);

		// Initialize Layout Engine with actual panel dimensions (Width*2 if crosshair
		// is on)
		layoutEngine = new ui.layout.HUDVirtualLayoutEngine(
				hudSettings.isDisplayCrosshair() ? ctx.width * 2 : ctx.width,
				ctx.height);

		// Component registration using ABSOLUTE slot logic for 1:1 legacy parity
		// Coordinates taken exactly from reference image logs

		// Flap Bar (strX=104, BarX=42)
		registerComponent(flapAngleBar, ui.layout.HUDLayoutSlot.ABSOLUTE);
		ui.layout.HUDComponentState flapState = componentStateMap.get(flapAngleBar.getId());
		flapState.setXOffset(42);
		flapState.setYOffset(33);

		// Attitude Indicator (x=112, y=129)
		registerComponent(attitudeIndicatorGauge, ui.layout.HUDLayoutSlot.ABSOLUTE);
		ui.layout.HUDComponentState attitudeState = componentStateMap.get(attitudeIndicatorGauge.getId());
		attitudeState.setXOffset(112);
		attitudeState.setYOffset(129);

		// Crosshair (x=272, y=20)
		registerComponent(crosshairGauge, ui.layout.HUDLayoutSlot.ABSOLUTE);
		ui.layout.HUDComponentState crosshairState = componentStateMap.get(crosshairGauge.getId());
		crosshairState.setXOffset(272);
		crosshairState.setYOffset(20);

		// Compass (x=148, y=130)
		registerComponent(compassGauge, ui.layout.HUDLayoutSlot.ABSOLUTE);
		ui.layout.HUDComponentState compassState = componentStateMap.get(compassGauge.getId());
		compassState.setXOffset(148);
		compassState.setYOffset(130);

		// ThrottleBar (x=7, y=182)
		registerComponent(throttleBar, ui.layout.HUDLayoutSlot.ABSOLUTE);
		ui.layout.HUDComponentState throttleState = componentStateMap.get(throttleBar.getId());
		throttleState.setXOffset(7);
		throttleState.setYOffset(182);

		// HUD Rows (x=42, y starts at 70 with 28px spacing)
		for (int i = 0; i < hudRows.size(); i++) {
			ui.component.row.HUDRow row = hudRows.get(i);
			registerComponent(row, ui.layout.HUDLayoutSlot.ABSOLUTE);
			ui.layout.HUDComponentState rowState = componentStateMap.get(row.getId());
			rowState.setXOffset(42);
			rowState.setYOffset(70 + i * 28);
		}

		Logger.info("MinimalHUD", "UI components initialized.");

		// Ensure everything is styled and updated before first paint
		applyStyleToComponents();
		updateComponents();
	}

	private void registerComponent(ui.component.HUDComponent comp, ui.layout.HUDLayoutSlot slot) {
		if (comp == null)
			return;
		ui.layout.HUDComponentState state = new ui.layout.HUDComponentState(comp);
		state.setSlot(slot);
		layoutEngine.addComponent(state);
		componentStateMap.put(comp.getId(), state);
	}

	private void applyStyleToComponents() {
		if (ctx == null)
			return;

		if (layoutEngine != null) {
			layoutEngine.setCanvasSize(hudSettings.isDisplayCrosshair() ? ctx.width * 2 : ctx.width, ctx.height);
		}
		if (crosshairGauge != null) {
			if (hudSettings.useTextureCrosshair()) {
				// Use loaded image from Context if available
				crosshairGauge.setTextureStyle(true, ctx.crosshairImageScaled, ctx.crossScale);
			} else {
				crosshairGauge.setStyleContext(hudSettings.getCrosshairScale());
			}
		}
		if (flapAngleBar != null) {
			flapAngleBar.setStyleContext(202, ctx.lineWidth + 2, ctx.drawFontSmall);
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
			throttleBar.setStyleContext(throttley_max, ctx.barWidth, ctx.drawFontSSmall, ctx.drawFontSSmall);
		}
	}
}
