package ui.overlay;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Container;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.Point;
import java.awt.Polygon;
import java.awt.RenderingHints;
import java.awt.geom.AffineTransform;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

import prog.Application;
import prog.Controller;
import prog.Service;
import ui.WebLafSettings;
import ui.base.DraggableOverlay;
import ui.util.OverlayStyleHelper;
import ui.util.SliderHelper;
import prog.config.OverlaySettings;

public class AttitudeOverlay extends DraggableOverlay implements prog.event.FlightDataListener {

	public AttitudeOverlay() {
		super();
		setTitle("地平仪");
	}

	public volatile boolean doit = true;
	private boolean isPreview = false;
	private Controller xc;
	private Service xs;
	private ui.model.TelemetrySource telemetrySource;
	private int lx;
	private int ly;
	private Container root;

	WebPanel topPanel;
	WebSlider slider;
	WebLabel label_1;
	WebLabel label_3;
	WebLabel label_6;
	String NumFont;
	String FontName;
	int fontadd;
	long AoA;
	long AoS;
	Boolean showDirection;

	Boolean showAoALimits;

	long bomb_dy;

	long AoALimitU;
	long AoALimitD;

	long compassX;
	long compassY;

	long Pitch;

	long Roll;

	Polygon groundLevel;

	// groundLevel
	// 原始图形的点
	Point[] pS;
	// 图形中间的点
	Point pC;
	// 旋转后图形的点
	Point[] pT;

	int pX[];
	int pY[];
	Color transParentWhite = Application.colorUnit;
	Color warning = Application.colorWarning;

	public static final int tickLine = 2;
	public static final int MaxAoA = 30;
	public static final int MaxAoS = 15;
	public int xWidth = 100;
	public int xHeight = 200;
	public static long freqMili = 40;

	public void rotateXY(int x[], int y[], int numPoints, double deg) {
		double rads = deg * Math.PI / 180.0;

		for (int i = 0; i < numPoints; i++) {
			x[i] = (int) Math.round(x[i] * Math.cos(rads) - y[i] * Math.sin(rads));
			y[i] = (int) Math.round(x[i] * Math.sin(rads) + y[i] * Math.cos(rads));
		}
	}

	public void moveXY(int x[], int y[], int numPoints, int dx, int dy) {
		for (int i = 0; i < numPoints; i++) {
			x[i] = x[i] + dx;
			y[i] = y[i] + dy;
		}
	}

	public void setFrameOpaque() {
		OverlayStyleHelper.applyTransparentStyle(this);
	}

	public void initPreview(Controller c, OverlaySettings settings) {
		isPreview = true;
		init(c, null, settings);
		applyPreviewStyle();
		setupDragListeners();
		setVisible(true);
		reinitConfig();
	}

	@Override
	public void saveCurrentPosition() {
		if (overlaySettings != null) {
			overlaySettings.saveWindowPosition(getLocation().x, getLocation().y);
		}
	}

	public void locater(Graphics2D g2d, int width, int height, int x, int y, int pitch, int center_round,
			int locator_size) {

		// 1. 先绘制橘红色多边形（地面层，最底层）
		g2d.setColor(Application.colorUnit);
		g2d.fillPolygon(pX, pY, 4);
		if (showDirection)
			g2d.drawLine(width / 2, height / 2, (int) (width / 2 + compassX), (int) (height / 2 + compassY));

		// 2. 绘制边框
		g2d.setStroke(new BasicStroke(1));
		g2d.setColor(Application.colorShadeShape);
		g2d.drawLine(0, 0, 0, height);
		g2d.drawLine(0, 0, width, 0);
		g2d.drawLine(0, height - 1, width - 1, height - 1);
		g2d.drawLine(width - 1, 0, width - 1, height - 1);

		// 3. 绘制刻度线（在多边形上方）
		for (int i = 0; i < 2 * tickLine; i++) {
			g2d.drawLine(pT[4 + 2 * i].x, pT[4 + 2 * i].y, pT[4 + 2 * i + 1].x, pT[4 + 2 * i + 1].y);
		}

		g2d.setStroke(new BasicStroke(3));
		g2d.setColor(Application.colorNum);

		g2d.drawLine(width / 2 - center_round / 2 - width / 8 - 1, height / 2 - 1, width / 2 - center_round / 2 - 1,
				height / 2 - 1);
		g2d.drawLine(width / 2 + center_round / 2, height / 2 - 1, width / 2 + center_round / 2 + width / 8 - 1,
				height / 2 - 1);

		g2d.drawLine(0, height / 2 - 1, width / 8 - 1, height / 2 - 1);
		g2d.drawLine(width - width / 8 + 1, height / 2 - 1, width, height / 2 - 1);

		g2d.drawArc(width / 2 - center_round / 2 - 1, height / 2 - center_round / 2 - 1, center_round, center_round,
				-180, 180);

		g2d.setStroke(new BasicStroke(2));

		g2d.drawLine(x - locator_size / 2 - 1, y - 1, x + locator_size / 2 - 1, y - 1);
		g2d.drawLine(x - 1, y - locator_size / 2 - 1, x - 1, y + locator_size / 2 - 1);

		g2d.setColor(Application.colorWarning);
		// 机翼攻角极限线 (原白色，改为红色)
		g2d.drawLine(0, (int) AoALimitU, width - 1, (int) AoALimitU);
		g2d.drawLine(0, (int) AoALimitD, width - 1, (int) AoALimitD);

		if (showDirection) {
			g2d.drawLine(width / 2, height / 2, (int) (width / 2 + compassX), (int) (height / 2 + compassY));

			g2d.setColor(Application.colorWarning);
			g2d.drawLine(width / 2, height / 2, (int) (width / 2 - compassX), (int) (height / 2 - compassY));
		}
	}

	public void initslider(WebSlider slider1) {
		SliderHelper.configureAttitudeSlider(slider1, -100, 100, Color.white);
	}

	public WebLabel createWebLabel(String text) {
		WebLabel l1 = new WebLabel(text);
		l1.setShadeColor(new Color(0, 0, 0));
		l1.setDrawShade(true);
		return l1;
	}

	public void initpanel(WebPanel toppanel) {
		toppanel.setLayout(null);
		WebPanel panel = new WebPanel() {
			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				g2d.setPaintMode();
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, Application.graphAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, Application.textAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
						RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
				g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING, RenderingHints.VALUE_COLOR_RENDER_SPEED);
				locater(g2d, xWidth, xHeight, (int) AoS, (int) AoA, (int) Pitch, 12, 6);
			}
		};

		panel.setBounds(0, 0, xWidth, xHeight);
		toppanel.add(panel);
	}

	public void reinitConfig() {
		if (overlaySettings != null) {
			FontName = overlaySettings.getFontName();
			NumFont = overlaySettings.getNumFontName();
			fontadd = overlaySettings.getFontSizeAdd();
		} else {
			FontName = Application.defaultFontName;
			NumFont = Application.defaultNumfontName;
			fontadd = 0;
		}

		if (overlaySettings != null) {
			xWidth = overlaySettings.getInt("attitudeIndicatorWidth", 150);
			xHeight = overlaySettings.getInt("attitudeIndicatorHeight", 300);

			int sw = 0;
			if (overlaySettings.getBool("enableAttitudeIndicatorEdge", false)) {
				sw = 10;
			}

			int totalWidth = xWidth + 4 + (sw * 2);
			int totalHeight = xHeight + 4 + (sw * 2);

			lx = overlaySettings.getWindowX(totalWidth);
			ly = overlaySettings.getWindowY(totalHeight);

			freqMili = overlaySettings.getInt("attitudeIndicatorFreqMs", 40);

			if (overlaySettings.getBool("attitudeIndicatorUseNumColor", false)) {
				transParentWhite = Application.colorNum;
			}

			showDirection = overlaySettings.getBool("attitudeIndicatorDisplayDirection", false);
			showAoALimits = overlaySettings.getBool("attitudeIndicatorDisplayAoALimits", true);

			setShadeWidth(sw);
			this.setBounds(lx, ly, totalWidth, totalHeight);
		} else {
			xWidth = 150;
			xHeight = 300;
			freqMili = 40;
			showDirection = false;
			showAoALimits = true;
			setShadeWidth(0);
		}

		if (isPreview) {
			applyPreviewStyle();
		} else {
			setFrameOpaque();
		}

		// 旋转中心需要更新
		pC = new Point(xWidth / 2, xHeight / 2);

		repaint();
	}

	public void init(Controller c, Service s, OverlaySettings settings) {
		this.xc = c;
		this.xs = s;
		if (s instanceof ui.model.TelemetrySource) {
			this.telemetrySource = (ui.model.TelemetrySource) s;
		}
		setOverlaySettings(settings);

		reinitConfig();

		this.setCursor(Application.blankCursor);
		setupTransparentWindow();

		groundLevel = new Polygon();
		pX = new int[4];
		pY = new int[4];

		pS = new Point[4 + tickLine * 4];
		pT = new Point[4 + tickLine * 4];

		for (int i = 0; i < 4 + tickLine * 4; i++) {
			pS[i] = new Point();
			pT[i] = new Point();
		}

		topPanel = new WebPanel() {
			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				g2d.setPaintMode();
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);
				g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
						RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
				locater(g2d, xWidth, xHeight, (int) AoS, (int) AoA, (int) Pitch, 12, 6);
			}
		};

		initpanel(topPanel);
		add(topPanel);
		root = this.getContentPane();
		setTitle("attitude");
		WebLafSettings.setWindowOpaque(this);

		if (s != null) {
			setVisible(true);
			prog.event.FlightDataBus.getInstance().register(this);
		}
	}

	@Override
	public void onFlightData(prog.event.FlightDataEvent event) {
		if (xs != null && xs.currentTimeMs - freqCheckMili > freqMili) {
			freqCheckMili = xs.currentTimeMs;
			javax.swing.SwingUtilities.invokeLater(() -> {
				drawTick();
			});
		}
	}

	@Override
	public void dispose() {
		prog.event.FlightDataBus.getInstance().unregister(this);
		super.dispose();
	}

	public void rotatePointMatrix(Point[] origPoints, double angle, Point center, Point[] storeTo, int numPoints) {
		AffineTransform.getRotateInstance(Math.toRadians(angle), center.x, center.y).transform(origPoints, 0, storeTo,
				0, numPoints);
	}

	long freqCheckMili;

	public void drawTick() {
		if (telemetrySource == null) return;

		// Use TelemetrySource interface for flight data (eliminates Feature Envy)
		double aoa = telemetrySource.getAoA();
		double aos = telemetrySource.getAoS();
		double pitch = telemetrySource.getAviahorizonPitch();
		double roll = telemetrySource.getAviahorizonRoll();
		double compass = telemetrySource.getCompass();

		AoA = Math.round((aoa + MaxAoA) * xHeight / (2 * MaxAoA));
		AoS = Math.round((-aos + MaxAoS) * xWidth / (2 * MaxAoS));
		Pitch = Math.round((-pitch + MaxAoA) * xHeight / (2 * MaxAoA));

		if (showDirection) {
			double compassRads = Math.toRadians(compass);
			compassX = (int) (xWidth / 4 * Math.sin(compassRads));
			compassY = (int) (xWidth / 4 * Math.cos(compassRads));
		}

		// FM data access is acceptable - it's aircraft configuration, not telemetry
		parser.Blkx b = xc.getBlkx();
		if (b != null && b.valid && showAoALimits) {
			// 显示机翼临界攻角极限线
			AoALimitU = Math.round((b.NoFlapsWing.AoACritHigh + MaxAoA) * xHeight / (2 * MaxAoA));
			AoALimitD = Math.round((b.NoFlapsWing.AoACritLow + MaxAoA) * xHeight / (2 * MaxAoA));
		} else {
			// 关闭时不显示攻角极限线
			AoALimitU = -10;
			AoALimitD = -10;
		}

		pS[0].x = -2 * xWidth;
		pS[0].y = 0;

		pS[1].x = 2 * xWidth;
		pS[1].y = 0;

		pS[2].x = 2 * xWidth;
		pS[2].y = 180 / MaxAoA * xHeight;

		pS[3].x = -2 * xWidth;
		pS[3].y = 180 / MaxAoA * xHeight;

		double start = -90.0f;
		double dTick = 90 / (tickLine + 1);
		for (int i = 0; i < tickLine; i++) {
			pS[4 + 4 * i].x = -xWidth;
			pS[4 + 4 * i].y = (int) Math.round((start + (dTick * (i + 1))) / (2 * MaxAoA) * xHeight);

			pS[4 + 4 * i + 1].x = xWidth;
			pS[4 + 4 * i + 1].y = (int) Math.round((start + (dTick * (i + 1))) / (2 * MaxAoA) * xHeight);

			// 对称
			pS[4 + 4 * i + 2].x = -xWidth;
			pS[4 + 4 * i + 2].y = (int) Math.round((-start - (dTick * (i + 1))) / (2 * MaxAoA) * xHeight);

			pS[4 + 4 * i + 3].x = xWidth;
			pS[4 + 4 * i + 3].y = (int) Math.round((-start - (dTick * (i + 1))) / (2 * MaxAoA) * xHeight);
		}
		// 平移
		for (int i = 0; i < pS.length; i++) {
			pS[i].x += xWidth / 2;
			pS[i].y += Pitch;
		}

		rotatePointMatrix(pS, roll, pC, pT, pS.length);

		for (int i = 0; i < 4; i++) {
			pX[i] = pT[i].x;
			pY[i] = pT[i].y;
		}

		root.repaint();
	}

}