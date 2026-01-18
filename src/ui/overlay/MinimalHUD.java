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
import ui.WebLafSettings;
import ui.base.DraggableOverlay;

/**
 * MinimalHUD overlay for displaying compact flight information.
 * Being migrated to event-driven architecture.
 */
public class MinimalHUD extends DraggableOverlay implements FlightDataListener {
	public volatile boolean doit = true;
	Boolean isHudTextVisible;
	Controller controller;
	WebPanel panel;
	int HUDFontsize;
	int Width;
	int Height;
	int CrossX;
	int CrossY;
	int AoAFuselagePix;
	int velocityX;
	int compass;
	Service service;
	OtherService otherService;
	Image crosshairImageRaw;
	Image crosshairImageScaled;
	String crosshairName;
	boolean busetexturecrosshair;
	int isDragging;
	int dragStartX;
	int dragStartY;
	private static final long serialVersionUID = -3898679368097973617L;
	String lineCompass;
	String lineHorizon;
	String lines[];
	String NumFont;
	Font drawFont;
	int CrossWidth;
	int CrossWidthVario;
	int pitch;
	int rightDraw;
	public int roundCompass;
	public boolean warnVne;
	public boolean drawAttitude;
	int blinkTicks = 1;
	int blinkCheckTicks = 0;
	public boolean warnRH;

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
		if (blinkX) {
			if (warningOverlay != null) {
				warningOverlay.draw(g, 0, 0, Width, Height, blinkActing);
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
	private boolean drawHudMach = false;
	public Color throttleColor;
	public Color aoaColor;

	public Color aoaBarColor;
	public int throttleLineWidth = 1;

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
		controller.setconfig("crosshairX", Integer.toString(getLocation().x));
		controller.setconfig("crosshairY", Integer.toString(getLocation().y));
	}

	public String speedLabelPrefix = "I";
	public String altitudeLabelPrefix = "H";
	public String sepLabelPrefix = "S";
	private boolean crossOn;
	private int barWidth;
	private int lineWidth;
	private double aoaLength;
	private Font drawFontSmall;
	private boolean disableAoA;
	private Container root;
	private int rollDeg;
	private double aoaWarningRatio;
	private double aoaBarWarningRatio;
	private int HUDFontSizeSmall;
	private String relEnergy;
	private BasicStroke strokeThick;
	private BasicStroke strokeThin;
	private int halfLine;
	private int compassDiameter;
	private int compassRadius;
	private String lineLoc;
	private int compassInnerMarkRadius;
	private Font drawFontSSmall;

	// 襟翼角度
	private double flapA;
	private double flapAllowA;

	private boolean enableFlapAngleBar;
	int windowX;
	int windowY;

	public void reinitConfig() {
		Logger.info("MinimalHUD", "reinitConfig called");

		if (controller.getconfig("MonoNumFont") != "")
			NumFont = controller.getconfig("MonoNumFont");
		else
			NumFont = Application.defaultNumfontName;
		if (controller.getconfig("crosshairX") != "")
			windowX = Integer.parseInt(controller.getconfig("crosshairX"));
		else
			windowX = (Application.screenWidth - Width) / 2;
		if (controller.getconfig("crosshairY") != "")
			windowY = Integer.parseInt(controller.getconfig("crosshairY"));
		else
			windowY = (Application.screenHeight - Height) / 2;
		if (controller.getconfig("crosshairScale") != "")
			CrossWidth = Integer.parseInt(controller.getconfig("crosshairScale"));
		else
			CrossWidth = 70;
		if (CrossWidth == 0)
			CrossWidth = 1;
		if (controller.getconfig("crosshairName") != "")
			crosshairName = controller.getconfig("crosshairName").trim();
		else
			crosshairName = "";
		// Application.debugPrint(controller.getconfig("usetexturecrosshair"));
		if (controller.getconfig("displayCrosshair") != "") {
			crossOn = Boolean.parseBoolean(controller.getconfig("displayCrosshair"));

		} else {
			crossOn = false;
		}

		if (controller.getconfig("usetexturecrosshair") != "")
			busetexturecrosshair = Boolean.parseBoolean(controller.getconfig("usetexturecrosshair"));
		else
			busetexturecrosshair = false;

		if (controller.getconfig("drawHUDtext") != "") {
			isHudTextVisible = Boolean.parseBoolean(controller.getconfig("drawHUDtext"));

		} else {
			isHudTextVisible = true;
		}
		if (controller.getconfig("drawHUDAttitude") != "") {
			drawAttitude = Boolean.parseBoolean(controller.getconfig("drawHUDAttitude"));
		} else {
			drawAttitude = true;
		}

		if (controller.getconfig("miniHUDaoaWarningRatio") != "") {
			aoaWarningRatio = Double.parseDouble(controller.getconfig("miniHUDaoaWarningRatio"));
		} else {
			aoaWarningRatio = 0.25;
		}

		if (controller.getconfig("miniHUDaoaBarWarningRatio") != "") {
			aoaBarWarningRatio = Double.parseDouble(controller.getconfig("miniHUDaoaBarWarningRatio"));
		} else {
			aoaBarWarningRatio = 0;
		}

		if (controller.getconfig("enableFlapAngleBar") != "") {
			enableFlapAngleBar = Boolean.parseBoolean(controller.getconfig("enableFlapAngleBar"));
		} else {
			enableFlapAngleBar = true;
		}

		HUDFontsize = CrossWidth / 4;
		barWidth = HUDFontsize / 4;
		lineWidth = HUDFontsize / 10;
		if (!crossOn)
			Width = (int) (CrossWidth * 2.25) - HUDFontsize;
		else {
			Width = (int) (CrossWidth * 2.25);
		}
		Height = (int) (CrossWidth * 1.5) + (int) (HUDFontsize * 3.5);
		Logger.info("MinimalHUD",
				"MinimalHUD Config: Width=" + Width + ", Height=" + Height + ", CrossWidth=" + CrossWidth);
		CrossWidthVario = CrossWidth;
		if (lineWidth == 0)
			lineWidth = 1;
		roundCompass = (int) (Math.round(HUDFontsize * 0.8f));
		rightDraw = (int) (HUDFontsize * 3.5f);

		compassDiameter = (int) Math.round(2 * HUDFontsize * 0.618);
		compassRadius = (int) Math.round(compassDiameter / 2.0);
		compassInnerMarkRadius = (int) Math.round(0.618 * compassDiameter);
		compassRadius = (int) Math.round(compassDiameter / 2.0);

		if (controller.getconfig("hudMach") != "")
			drawHudMach = Boolean.parseBoolean(controller.getconfig("hudMach"));

		if (controller.getconfig("disableHUDSpeedLabel") != "") {
			if (Boolean.parseBoolean(controller.getconfig("disableHUDSpeedLabel"))) {
				speedLabelPrefix = "";
			} else {
				speedLabelPrefix = "SPD";
			}
		}
		if (controller.getconfig("disableHUDHeightLabel") != "") {
			if (Boolean.parseBoolean(controller.getconfig("disableHUDHeightLabel"))) {
				altitudeLabelPrefix = "";
			} else {
				altitudeLabelPrefix = "ALT";
			}
		}
		if (controller.getconfig("disableHUDSEPLabel") != "") {
			if (Boolean.parseBoolean(controller.getconfig("disableHUDSEPLabel"))) {
				sepLabelPrefix = "";
			} else {
				sepLabelPrefix = "SEP";
			}
		}
		if (controller.getconfig("disableHUDAoA") != "") {
			if (Boolean.parseBoolean(controller.getconfig("disableHUDAoA"))) {
				lineAoA = "";
				disableAoA = true;
				relEnergy = "";
			} else {
				disableAoA = false;
			}
		}

		aoaLength = rightDraw - HUDFontsize / 2;
		if (aoaY > rightDraw)
			aoaY = rightDraw;

		halfLine = (lineWidth / 2 == 0) ? 1 : (int) Math.round(lineWidth / 2.0f);
		strokeThick = new BasicStroke(halfLine + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		strokeThin = new BasicStroke(halfLine, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);

		String path = "image/gunsight/" + crosshairName + ".png";
		java.io.File imgFile = new java.io.File(path);
		if (imgFile.exists()) {
			try {
				crosshairImageRaw = javax.imageio.ImageIO.read(imgFile);
				if (crosshairImageRaw != null) {
					int targetSize = CrossWidth * 2;
					java.awt.image.BufferedImage scaled = new java.awt.image.BufferedImage(targetSize, targetSize,
							java.awt.image.BufferedImage.TYPE_INT_ARGB);
					java.awt.Graphics2D g2 = scaled.createGraphics();
					g2.setRenderingHint(java.awt.RenderingHints.KEY_INTERPOLATION,
							java.awt.RenderingHints.VALUE_INTERPOLATION_BILINEAR);
					g2.drawImage(crosshairImageRaw, 0, 0, targetSize, targetSize, null);
					g2.dispose();
					crosshairImageScaled = scaled;

					Logger.info("MinimalHUD", "Loaded and scaled crosshair: " + path + " to " + targetSize + "x"
							+ targetSize);
				}
			} catch (java.io.IOException e) {
				Logger.info("MinimalHUD", "Failed to read crosshair: " + path);
				crosshairImageRaw = null;
			}
		} else {
			Logger.info("MinimalHUD", "Crosshair file not found: " + imgFile.getAbsolutePath());
			crosshairImageRaw = null;
		}

		if (crossOn)
			this.setBounds(windowX, windowY, Width * 2, Height);
		else
			this.setBounds(windowX, windowY, Width, Height);

		drawFont = new Font(NumFont, Font.BOLD, HUDFontsize);
		HUDFontSizeSmall = (int) (HUDFontsize * 0.75f);
		drawFontSmall = new Font(NumFont, Font.BOLD, HUDFontSizeSmall);
		drawFontSSmall = new Font(NumFont, Font.BOLD, HUDFontsize / 2);
		CrossX = Width / 2;
		CrossY = Height / 2;

		if (layoutEngine != null) {
			layoutEngine.setCanvasSize(crossOn ? Width * 2 : Width, Height);
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
		lines[0] = speedLabelPrefix + String.format("%5s", "360");
		lines[1] = altitudeLabelPrefix + String.format("%5s", "1024");
		lines[3] = sepLabelPrefix + String.format("%5s", "30");
		lines[4] = "G" + String.format("%5s", "2.0");
		lines[2] = "F" + String.format("%3s", "100");
		lines[2] += "BRK";
		lines[2] += "GEAR";
		lineCompass = String.format("%3s", "102");
		lineLoc = "A1";
		lineHorizon = String.format("%3s", "45");
		throttley = 100;
		throttley_max = (int) (HUDFontsize * 4.75);
		aoaY = 10;
		disableAoA = false;
		throttleColor = Application.colorShadeShape;
		lineAoA = String.format("α%3.0f", 20.0);
		relEnergy = "E114514";
		sAttitude = "";

		if (controller.getconfig("disableHUDAoA") != "") {
			if (Boolean.parseBoolean(c.getconfig("disableHUDAoA"))) {
				lineAoA = "";
				disableAoA = true;
				relEnergy = "";
			}
		}
		aosX = 0;
		rollDeg = 0;
		aoaLength = rightDraw - HUDFontsize / 2;
		if (aoaY > rightDraw)
			aoaY = rightDraw;
		aoaColor = Application.colorNum;
		aoaBarColor = Application.colorNum;

		halfLine = (lineWidth / 2 == 0) ? 1 : (int) Math.round(lineWidth / 2.0f);
		strokeThick = new BasicStroke(halfLine + 2, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		strokeThin = new BasicStroke(halfLine, BasicStroke.CAP_ROUND, BasicStroke.JOIN_ROUND);
		// Image loading is handled by reinitConfig() which is called above

		if (crossOn)
			this.setBounds(windowX, windowY, Width * 2, Height);
		else
			this.setBounds(windowX, windowY, Width, Height);
		drawFont = new Font(NumFont, Font.BOLD, HUDFontsize);
		HUDFontSizeSmall = (int) (HUDFontsize * 0.75f);
		drawFontSmall = new Font(NumFont, Font.BOLD, HUDFontSizeSmall);

		drawFontSSmall = new Font(NumFont, Font.BOLD, HUDFontsize / 2);
		CrossX = Width / 2;
		CrossY = Height / 2;

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

		// double compassRads = (double) Math.toRadians(service.sIndic.compass);

		compassDx = (int) ((roundCompass * 1.3f) * Math.sin(compassRads));
		compassDy = (int) ((roundCompass * 1.3f) * Math.cos(compassRads));
		double aoa = service.sState.AoA;
		// lineHorizon = " " + String.format("%5s", service.sPitchUp);
		double p = service.curLoadMinWorkTime < service.fueltime ? service.curLoadMinWorkTime : service.fueltime;
		OilX = (int) (p * 360 / 600000);
		if (OilX > 360)
			OilX = 360;
		OilX = OilX - 360;
		// OilX = OilX - 180;
		double aviahp = service.sIndic.aviahorizon_pitch;
		double aviar = service.sIndic.aviahorizon_roll;
		// int pitchLimit = HUDFontsize * 5;
		int pitchLimit = HUDFontsize;
		realSpdPitch = -(aviahp + aoa);
		if (aviahp != -65535)
			pitch = (int) ((-aviahp * pitchLimit / 90.0f));
		else
			pitch = 0;
		// Application.debugPrint(-(aviahp+aoa));
		int slideLimit = 4 * HUDFontsize;
		if (service.sState.AoS != -65535) {
			aosX = (int) (-service.sState.AoS * slideLimit / 30.0f);
		} else
			aosX = 0;
		rollDeg = (int) (-aviar);
		lineCompass = String.format("%3s", service.compass);
		char map_x = (char) ('A' + (service.loc[1] * service.mapinfo.mapStage) + service.mapinfo.inGameOffset);
		int map_y = (int) (service.loc[0] * service.mapinfo.mapStage + service.mapinfo.inGameOffset + 1);

		lineLoc = String.format("%c%d", map_x, map_y);
		if (drawHudMach)
			lines[0] = String.format("M%5s", service.M);
		else
			lines[0] = String.format("%s%6s", speedLabelPrefix, service.IAS);

		/* 近地告警 */
		if (service.radioAltValid && service.radioAlt <= 500)
			lines[1] = altitudeLabelPrefix + String.format("R%5s", service.sRadioAlt);
		else
			lines[1] = altitudeLabelPrefix + String.format("%6s", service.salt);

		if (service.SEP > 0) {
			lines[3] = String.format("%s↑%4s", sepLabelPrefix, service.sSEP);
		} else {
			lines[3] = String.format("%s↓%4s", sepLabelPrefix, service.sSEP);
		}
		if (service.sState.Ny > 1.5f || service.sState.Ny < -0.5f)
			lines[4] = String.format("G%5s", service.Ny);
		else {
			// 燃油量和增压器
			String s = service.sfueltime;
			String compressor;
			switch (service.sState.compressorstage) {
				case 1:
					compressor = "C";
				case 2:
					compressor = "CC";
				case 3:
					compressor = "CCC";
				default:
					compressor = "";
			}
			if (service.sState.gear <= 0) {
				lines[4] = String.format("L%5s%s", s, compressor);
			} else {
				lines[4] = String.format("E%5s", service.sTime);
			}
			// if(service.sState.compressorstage != 0){
			// lines[3] += "S"
			// +String.format("%d",service.sState.compressorstage);
			// }
			s = null;
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

		// 襟翼告警
		if (service.sState.IAS > service.flapAllowSpeed * 0.95) {
			//
			inAction = true;
		}

		parser.Blkx b = controller.getBlkx();
		if (b != null && b.valid) {
			// 机动指标(1 - 空油重/(空油重 + 油重))
			// 指标越低说明机动性越好
			double nfweight = b.nofuelweight;
			maneuverIndex = 1 - (nfweight / (nfweight + service.fTotalFuel));
			maneuverIndexLen = (int) Math.round(maneuverIndex / 0.5 * rightDraw);
			maneuverIndexLen10 = (int) Math.round(0.10 / 0.5 * rightDraw);
			maneuverIndexLen20 = (int) Math.round(0.20 / 0.5 * rightDraw);
			maneuverIndexLen30 = (int) Math.round(0.30 / 0.5 * rightDraw);
			maneuverIndexLen40 = (int) Math.round(0.40 / 0.5 * rightDraw);
			maneuverIndexLen50 = (int) Math.round(0.50 / 0.5 * rightDraw);
			// 速度
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

			if (availableAoA < aoaWarningRatio * maxAvailableAoA) {
				aoaColor = Application.colorWarning;
			} else {
				aoaColor = Application.colorNum;
			}
			if (availableAoA < aoaBarWarningRatio * maxAvailableAoA) {
				aoaBarColor = Application.colorUnit;
			} else {
				aoaBarColor = Application.colorNum;
			}

			aoaY = (int) ((availableAoA * aoaLength) / maxAvailableAoA);

			if (aoaY > rightDraw)
				aoaY = rightDraw;

			AoAFuselagePix = (int) ((b.NoFlapsWing.AoACritHigh - b.Fuselage.AoACritHigh) * aoaLength
					/ b.NoFlapsWing.AoACritHigh);
		} else {
			AoAFuselagePix = (int) (aoa * aoaLength / 15);
			aoaY = (int) (aoa * aoaLength / 30);
		}
		if (!disableAoA) {
			lineAoA = String.format("α%3.0f", aoa);

			relEnergy = String.format("E%5.0f", service.energyM);
		}
		// 姿态
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

		// 襟翼角度显示
		flapA = service.sState.flaps;
		flapAllowA = service.getFlapAllowAngle(service.sState.IAS, service.isDowningFlap);

		updateComponents();
	}

	private void updateComponents() {
		boolean textVisible = isHudTextVisible;

		if (flapAngleBar != null) {
			flapAngleBar.update(flapA, flapAllowA);
			componentStateMap.get(flapAngleBar.getId()).setVisible(textVisible && enableFlapAngleBar);
		}
		if (compassGauge != null) {
			compassGauge.update((float) compassRads, compassDx, compassDy, lineCompass, lineLoc);
			componentStateMap.get(compassGauge.getId()).setVisible(textVisible);
		}
		if (attitudeIndicatorGauge != null) {
			attitudeIndicatorGauge.update(pitch, rollDeg, aosX, sAttitude, roundHorizon);
			componentStateMap.get(attitudeIndicatorGauge.getId()).setVisible(drawAttitude && !disableAttitude);
		}
		if (crosshairGauge != null) {
			ui.layout.HUDComponentState crosshairState = componentStateMap.get(crosshairGauge.getId());
			crosshairState.setVisible(crossOn);
			// Dynamic position based on current Width/CrossX
			crosshairState.setXOffset(Width + CrossX);
			crosshairState.setYOffset(CrossY);
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

	private void initComponentsLayout() {
		// Initialize reusable UI components (high-performance)
		crosshairGauge = new ui.component.CrosshairGauge();
		flapAngleBar = new ui.component.FlapAngleBar();
		warningOverlay = new ui.component.WarningOverlay();
		compassGauge = new ui.component.CompassGauge(roundCompass);
		attitudeIndicatorGauge = new ui.component.AttitudeIndicatorGauge();

		// Initialize Rows
		hudRows = new java.util.ArrayList<>();
		// Row 0: AoA
		hudRows.add(new ui.component.row.HUDAkbRow(0, drawFont, HUDFontsize, drawFontSmall, rightDraw, lineWidth));
		// Row 1: Energy
		hudRows.add(new ui.component.row.HUDEnergyRow(1, drawFont, HUDFontsize, drawFontSmall, rightDraw));
		// Row 2: Standard (Fuel/Gear)
		hudRows.add(new ui.component.row.HUDTextRow(2, drawFont, HUDFontsize));
		// Row 3: Standard (SEP)
		hudRows.add(new ui.component.row.HUDTextRow(3, drawFont, HUDFontsize));
		// Row 4: Maneuver
		hudRows.add(new ui.component.row.HUDManeuverRow(4, drawFont, HUDFontsize, rightDraw, halfLine, lineWidth,
				strokeThick, strokeThin));

		throttleBar = new ui.component.LinearGauge("ThrottleBar", 110, true);

		// Initialize Layout Engine with actual panel dimensions (Width*2 if crosshair
		// is on)
		layoutEngine = new ui.layout.HUDVirtualLayoutEngine(crossOn ? Width * 2 : Width, Height);

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
		if (layoutEngine != null) {
			layoutEngine.setCanvasSize(crossOn ? Width * 2 : Width, Height);
		}
		if (crosshairGauge != null) {
			if (busetexturecrosshair) {
				crosshairGauge.setTextureStyle(true, crosshairImageScaled, CrossWidthVario);
			} else {
				crosshairGauge.setStyleContext(CrossWidth);
			}
		}
		if (flapAngleBar != null) {
			flapAngleBar.setStyleContext(202, lineWidth + 2, drawFontSmall);
		}
		if (compassGauge != null) {
			compassGauge.setStyleContext(roundCompass, lineWidth, HUDFontsize, HUDFontSizeSmall, drawFontSmall);
		}
		if (attitudeIndicatorGauge != null) {
			attitudeIndicatorGauge.setStyleContext(compassDiameter, compassRadius, compassInnerMarkRadius,
					lineWidth, halfLine, drawFontSmall);
		}
		// Synchronize styles for Rows
		if (hudRows != null && hudRows.size() >= 5) {
			((ui.component.row.HUDAkbRow) hudRows.get(0)).setStyle(drawFont, HUDFontsize, drawFontSmall, rightDraw,
					lineWidth);
			((ui.component.row.HUDEnergyRow) hudRows.get(1)).setStyle(drawFont, HUDFontsize, drawFontSmall,
					rightDraw);
			((ui.component.row.HUDTextRow) hudRows.get(2)).setStyle(drawFont, HUDFontsize);
			((ui.component.row.HUDTextRow) hudRows.get(3)).setStyle(drawFont, HUDFontsize);
			((ui.component.row.HUDManeuverRow) hudRows.get(4)).setStyle(drawFont, HUDFontsize, rightDraw,
					halfLine, lineWidth, strokeThick, strokeThin);
		}

		if (throttleBar != null) {
			throttleBar.setStyleContext(throttley_max, barWidth, drawFontSSmall, drawFontSSmall);
		}
	}
}
