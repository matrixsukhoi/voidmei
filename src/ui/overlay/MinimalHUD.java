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

	private parser.State sState;
	private parser.Indicators sIndic;
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
	private OtherService otherService;

	private int isDragging;
	private int dragStartX;
	private int dragStartY;

	private String lineCompass;
	private String lines[];

	private int pitch;

	private boolean disableAoA;
	private Container root;
	private int rollDeg;

	private String relEnergy;

	private String lineLoc;

	// Restored State Fields
	private int velocityX;
	private String lineHorizon;
	private int compass;
	private int AoAFuselagePix;

	// 襟翼角度
	private double flapA;
	private double flapAllowA;

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
		velocityX = 0;
		otherService = os;
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
		lineCompass = String.format("%3s", "102");
		lineLoc = "A1";
		lineHorizon = String.format("%3s", "45");
		throttley = 100;
		if (ctx != null)
			throttley_max = (int) (ctx.hudFontSize * 4.75);
		aoaY = 10;
		disableAoA = false;
		throttleColor = Application.colorShadeShape;
		lineAoA = String.format("α%3.0f", 20.0);
		relEnergy = "E114514";
		sAttitude = "";

		if (hudSettings.isAoADisabled()) {
			lineAoA = "";
			disableAoA = true;
			relEnergy = "";
		}
		aosX = 0;
		rollDeg = 0;
		// aoaLength = ... (Removed, using ctx.aoaLength)
		if (ctx != null && aoaY > ctx.rightDraw)
			aoaY = ctx.rightDraw;
		aoaColor = Application.colorNum;
		aoaBarColor = Application.colorNum;

		// Removed redundant Stroke and Font creation (handled by Context)
		// Removed redundant bounds setting (handled by reinitConfig)

		flapA = 20.0;
		flapAllowA = 100.0;

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
	private int compassDx;
	private int compassDy;
	private double availableAoA;
	private String lineAoA;
	private int aosX;

	double realSpdPitch;

	private String sAttitude;

	private double compassRads;
	private int roundHorizon;

	private double maneuverIndex;
	private int maneuverIndexLen;

	private int maneuverIndexLen30;
	private int maneuverIndexLen10;
	private int maneuverIndexLen20;
	private int maneuverIndexLen40;
	private int maneuverIndexLen50;
	private int throttley_max;
	private boolean disableAttitude;

	public void updateString() {
		if (ctx == null)
			return;

		// 1. Calculate Data Snapshot
		HUDData data = HUDCalculator.calculate(service, controller.getBlkx(), hudSettings, ctx);

		// 2. Dispatch to Reactive Components
		for (HUDComponent comp : components) {
			comp.onDataUpdate(data);
		}

		warnVne = false;
		warnRH = false;
		blinkX = service.fatalWarn;
		int throttle = service.sState.throttle;
		if (throttle > 101) {
			throttleColor = Application.colorWarning;
		} else {
			throttleColor = Application.colorNum;
		}
		throttley = throttle * throttley_max / 110;

		compass = (int) service.dCompass;
		compassRads = (double) Math.toRadians(service.dCompass);

		compassDx = (int) ((ctx.roundCompass * 1.3f) * Math.sin(compassRads));
		compassDy = (int) ((ctx.roundCompass * 1.3f) * Math.cos(compassRads));
		double aoa = service.sState.AoA;

		double p = service.curLoadMinWorkTime < service.fueltime ? service.curLoadMinWorkTime : service.fueltime;
		OilX = (int) (p * 360 / 600000);
		if (OilX > 360)
			OilX = 360;
		OilX = OilX - 360;
		double aviahp = service.sIndic.aviahorizon_pitch;
		double aviar = service.sIndic.aviahorizon_roll;

		int pitchLimit = ctx.hudFontSize;
		realSpdPitch = -(aviahp + aoa);
		if (aviahp != -65535)
			pitch = (int) ((-aviahp * pitchLimit / 90.0f));
		else
			pitch = 0;

		int slideLimit = 4 * ctx.hudFontSize;
		if (service.sState.AoS != -65535) {
			aosX = (int) (-service.sState.AoS * slideLimit / 30.0f);
		} else
			aosX = 0;
		rollDeg = (int) (-aviar);
		lineCompass = String.format("%3s", service.compass);
		char map_x = (char) ('A' + (service.loc[1] * service.mapinfo.mapStage) + service.mapinfo.inGameOffset);
		int map_y = (int) (service.loc[0] * service.mapinfo.mapStage + service.mapinfo.inGameOffset + 1);

		lineLoc = String.format("%c%d", map_x, map_y);

		String spdPre = hudSettings.isSpeedLabelDisabled() ? "" : "SPD";
		String altPre = hudSettings.isAltitudeLabelDisabled() ? "" : "ALT";
		String sepPre = hudSettings.isSEPLabelDisabled() ? "" : "SEP";

		if (hudSettings.drawHudMach())
			lines[0] = String.format("M%5s", service.M);
		else
			lines[0] = String.format("%s%6s", spdPre, service.IAS);

		/* 近地告警 */
		if (service.radioAltValid && service.radioAlt <= 500)
			lines[1] = altPre + String.format("R%5s", service.sRadioAlt);
		else
			lines[1] = altPre + String.format("%6s", service.salt);

		if (service.SEP > 0) {
			lines[3] = String.format("%s↑%4s", sepPre, service.sSEP);
		} else {
			lines[3] = String.format("%s↓%4s", sepPre, service.sSEP);
		}
		if (service.sState.Ny > 1.5f || service.sState.Ny < -0.5f)
			lines[4] = String.format("G%5s", service.Ny);
		else {
			String s = service.sfueltime;
			String compressor;
			switch (service.sState.compressorstage) {
				case 1:
					compressor = "C";
					break;
				case 2:
					compressor = "CC";
					break;
				case 3:
					compressor = "CCC";
					break;
				default:
					compressor = "";
			}
			if (service.sState.gear <= 0) {
				lines[4] = String.format("L%5s%s", s, compressor);
			} else {
				lines[4] = String.format("E%5s", service.sTime);
			}
		}
		String brk = "";
		String gear = "";
		inAction = false;
		if (service.sState.airbrake > 0) {
			brk = "BRK";
			if (service.sState.airbrake != 100) {
				inAction |= true;
			}
			if (service.sState.airbrake == 100)
				warnVne = true;
		}

		if (service.sState.gear > 0) {
			gear = "GEA";
			if (service.sState.gear != 100)
				inAction |= true;
		}

		if (service.sState.flaps > 0) {
			lines[2] = String.format("F%3s%s%s", service.flaps, brk, gear);
		} else {
			if (service.hasWingSweepVario) {
				lines[2] = String.format("W%3s%s%s", service.sWingSweep, brk, gear);
			} else {
				lines[2] = String.format("%4s%s%s", "", brk, gear);
			}
		}

		if (service.sState.IAS > service.flapAllowSpeed * 0.95) {
			inAction = true;
		}

		parser.Blkx b = controller.getBlkx();
		if (b != null && b.valid) {
			double nfweight = b.nofuelweight;
			maneuverIndex = 1 - (nfweight / (nfweight + service.fTotalFuel));
			maneuverIndexLen = (int) Math.round(maneuverIndex / 0.5 * ctx.rightDraw);
			maneuverIndexLen10 = (int) Math.round(0.10 / 0.5 * ctx.rightDraw);
			maneuverIndexLen20 = (int) Math.round(0.20 / 0.5 * ctx.rightDraw);
			maneuverIndexLen30 = (int) Math.round(0.30 / 0.5 * ctx.rightDraw);
			maneuverIndexLen40 = (int) Math.round(0.40 / 0.5 * ctx.rightDraw);
			maneuverIndexLen50 = (int) Math.round(0.50 / 0.5 * ctx.rightDraw);
			double vwing = 0;
			if (b.isVWing) {
				vwing = service.sIndic.wsweep_indicator;
			}

			if ((service.IASv >= b.getVNEVWing(vwing) * 0.95) || (service.sState.M >= b.getMNEVWing(vwing) * 0.95f)) {
				warnVne = true;
			}

			int flaps = service.sState.flaps > 0 ? service.sState.flaps : 0;

			double maxAvailableAoA = b.getAoAHighVWing(vwing, flaps);

			availableAoA = maxAvailableAoA - aoa;

			if (availableAoA < hudSettings.getAoAWarningRatio() * maxAvailableAoA) {
				aoaColor = Application.colorWarning;
			} else {
				aoaColor = Application.colorNum;
			}
			if (availableAoA < hudSettings.getAoABarWarningRatio() * maxAvailableAoA) {
				aoaBarColor = Application.colorUnit;
			} else {
				aoaBarColor = Application.colorNum;
			}

			aoaY = (int) ((availableAoA * ctx.aoaLength) / maxAvailableAoA);

			if (aoaY > ctx.rightDraw)
				aoaY = ctx.rightDraw;

			AoAFuselagePix = (int) ((b.NoFlapsWing.AoACritHigh - b.Fuselage.AoACritHigh) * ctx.aoaLength
					/ b.NoFlapsWing.AoACritHigh);
		} else {
			AoAFuselagePix = (int) (aoa * ctx.aoaLength / 15);
			aoaY = (int) (aoa * ctx.aoaLength / 30);
		}

		if (hudSettings.isAoADisabled()) {
			lineAoA = "";
			disableAoA = true;
			relEnergy = "";
		} else {
			disableAoA = false;
			lineAoA = String.format("α%3.0f", aoa);
			relEnergy = String.format("E%5.0f", service.energyM);
		}

		sAttitude = "";

		disableAttitude = false;
		roundHorizon = (int) Math.round(-aviahp);
		if (aviahp != -65535) {
			if (roundHorizon > 0)
				sAttitude = String.format("%3d", roundHorizon);
			if (roundHorizon < 0)
				sAttitude = String.format("%3d", -roundHorizon);
		} else {
			disableAttitude = true;
		}

		if (aviar == -65535) {
			disableAttitude = true;
		}

		flapA = service.sState.flaps;
		flapAllowA = service.getFlapAllowAngle(service.sState.IAS, service.isDowningFlap);

		updateComponents();
	}

	private void updateComponents() {
		boolean textVisible = hudSettings.drawHUDText();

		if (flapAngleBar != null) {
			flapAngleBar.update(flapA, flapAllowA);
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
			// Row 0: AoA/Speed
			((ui.component.row.HUDAkbRow) hudRows.get(0)).update(lines[0], warnVne, lineAoA, aoaY, aoaColor,
					aoaBarColor);
			// Row 1: Energy/Altitude
			((ui.component.row.HUDEnergyRow) hudRows.get(1)).update(lines[1], warnRH, relEnergy, aoaColor);
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
		Map<String, String> data = event.getData();

		// Read simple values from event where available
		String fatalWarnStr = data.get("fatalWarn");
		if (fatalWarnStr != null) {
			blinkX = Boolean.parseBoolean(fatalWarnStr);
		}

		// Delegate complex calculations to legacy method
		// which still reads from service (Service) for Blkx-dependent logic
		if (service != null) {
			updateString();
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
					ctx.lineWidth);
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
