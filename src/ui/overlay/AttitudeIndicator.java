package ui.overlay;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Container;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.Point;
import java.awt.Polygon;
import java.awt.RenderingHints;
import java.awt.event.MouseListener;
import java.awt.event.MouseMotionListener;
import java.awt.geom.AffineTransform;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.slider.WebSlider;

import prog.Application;
import prog.Controller;
import prog.Service;
import ui.WebLafSettings;
import ui.base.DraggableOverlay;
import prog.config.OverlaySettings;

public class AttitudeIndicator extends DraggableOverlay {

	public volatile boolean doit = true;
	private OverlaySettings settings;
	private Controller xc;
	private Service xs;
	private int lx;
	private int ly;
	private Container root;

	WebPanel topPanel;
	WebSlider slider;
	WebLabel label_1;
	WebLabel label_3;
	WebLabel label_6;
	String NumFont;
	long AoA;
	long AoS;
	Boolean showDirection;

	Boolean showAoALimits;

	long bomb_dy;

	long AoALimitU;
	long AoALimitD;
	long AoAFLimitU;
	long AoAFLimitD;

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
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明
		setShadeWidth(0);
	}

	public void initPreview(Controller c, OverlaySettings settings) {
		init(c, null, settings);
		applyPreviewStyle();
		setupDragListeners();
		setVisible(true);
	}

	@Override
	public void saveCurrentPosition() {
		if (settings != null) {
			settings.saveWindowPosition(getLocation().x, getLocation().y);
		}
	}

	public void locater(Graphics2D g2d, int width, int height, int x, int y, int pitch, int center_round,
			int locator_size) {

		// 绘制边框
		g2d.setStroke(new BasicStroke(1));
		g2d.setColor(Application.colorShadeShape);
		g2d.drawLine(0, 0, 0, height);
		g2d.drawLine(0, 0, width, 0);
		g2d.drawLine(0, height - 1, width - 1, height - 1);
		g2d.drawLine(width - 1, 0, width - 1, height - 1);

		for (int i = 0; i < 2 * tickLine; i++) {
			g2d.drawLine(pT[4 + 2 * i].x, pT[4 + 2 * i].y, pT[4 + 2 * i + 1].x, pT[4 + 2 * i + 1].y);
		}

		g2d.setColor(Application.colorUnit);
		g2d.fillPolygon(pX, pY, 4);
		if (showDirection)
			g2d.drawLine(width / 2, height / 2, (int) (width / 2 + compassX), (int) (height / 2 + compassY));

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
		g2d.drawLine(0, (int) AoAFLimitU, width - 1, (int) AoAFLimitU);
		g2d.drawLine(0, (int) AoAFLimitD, width - 1, (int) AoAFLimitD);

		g2d.setColor(Application.colorLabel);
		// 两条线
		g2d.drawLine(0, (int) AoALimitU, width - 1, (int) AoALimitU);
		g2d.drawLine(0, (int) AoALimitD, width - 1, (int) AoALimitD);

		if (showDirection) {
			g2d.drawLine(width / 2, height / 2, (int) (width / 2 + compassX), (int) (height / 2 + compassY));

			g2d.setColor(Application.colorUnit);
			g2d.drawLine(width / 2, height / 2, (int) (width / 2 - compassX), (int) (height / 2 - compassY));
		}
	}

	public void initslider(WebSlider slider1) {
		slider1.setMinimum(-100);
		slider1.setMaximum(100);
		slider1.setValue(0);
		slider1.setDrawProgress(true);
		slider1.setMinorTickSpacing(25);
		slider1.setMajorTickSpacing(50);
		slider1.setPaintTicks(true);
		slider1.setPaintLabels(true);
		slider1.setProgressShadeWidth(0);
		slider1.setTrackShadeWidth(1);
		slider1.setThumbShadeWidth(2);
		slider1.setThumbBgBottom(Color.white);
		slider1.setThumbBgTop(Color.white);
		slider1.setTrackBgBottom(new Color(0, 0, 0, 0));
		slider1.setTrackBgTop(new Color(0, 0, 0, 0));
		slider1.setProgressBorderColor(new Color(0, 0, 0, 0));
		slider1.setProgressTrackBgBottom(new Color(0, 0, 0, 0));
		slider1.setProgressTrackBgTop(new Color(0, 0, 0, 0));
		slider1.setFocusable(false);
		// 取消slider1响应
		MouseListener[] mls = slider1.getMouseListeners();
		MouseMotionListener[] mmls = slider1.getMouseMotionListeners();
		for (int t = 0; t < mls.length; t++) {
			slider1.removeMouseListener(mls[t]);
		}
		for (int t = 0; t < mmls.length; t++) {
			slider1.removeMouseMotionListener(mmls[t]);
		}
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
		if (settings != null) {
			NumFont = settings.getFontName();
		} else {
			NumFont = Application.defaultNumfontName;
		}

		if (settings != null) {
			xWidth = settings.getInt("attitudeIndicatorWidth", 150);
			xHeight = settings.getInt("attitudeIndicatorHeight", 300);

			int sw = 0;
			if (settings.getBool("enableAttitudeIndicatorEdge", false)) {
				sw = 10;
			}

			int totalWidth = xWidth + 4 + (sw * 2);
			int totalHeight = xHeight + 4 + (sw * 2);

			lx = settings.getWindowX(totalWidth);
			ly = settings.getWindowY(totalHeight);

			freqMili = settings.getInt("attitudeIndicatorFreqMs", 40);

			if (settings.getBool("attitudeIndicatorUseNumColor", false)) {
				transParentWhite = Application.colorNum;
			}

			showDirection = settings.getBool("attitudeIndicatorDisplayDirection", false);
			showAoALimits = settings.getBool("attitudeIndicatorDisplayAoALimits", true);

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

		setFrameOpaque();

		// 旋转中心需要更新
		pC = new Point(xWidth / 2, xHeight / 2);

		repaint();
	}

	public void init(Controller c, Service s, OverlaySettings settings) {
		this.xc = c;
		this.xs = s;
		this.settings = settings;

		reinitConfig();

		pX = new int[4];
		pY = new int[4];

		// 原始地面图形
		pS = new Point[4 + tickLine * 4];
		pS[0] = new Point(-10, 10);
		pS[1] = new Point(10, 10);
		pS[2] = new Point(10, -10);
		pS[3] = new Point(-10, -10);

		// 刻度线
		for (int i = 4; i < 4 + tickLine * 4; i++) {
			pS[i] = new Point(0, 0);
		}

		// 旋转中心
		pC = new Point(xWidth / 2, xHeight / 2);

		// pT
		pT = new Point[4 + tickLine * 4];
		for (int i = 0; i < 4 + tickLine * 4; i++) {
			pT[i] = new Point(0, 0);
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

		if (s != null)
			setVisible(true);

	}

	public void rotatePointMatrix(Point[] origPoints, double angle, Point center, Point[] storeTo, int numPoints) {
		AffineTransform.getRotateInstance(Math.toRadians(angle), center.x, center.y).transform(origPoints, 0, storeTo,
				0, numPoints);
	}

	long freqCheckMili;

	public void drawTick() {
		AoA = Math.round((-xs.sState.AoA + MaxAoA) * xHeight / (2 * MaxAoA));
		AoS = Math.round((xs.sState.AoS + MaxAoS) * xWidth / (2 * MaxAoS));
		Pitch = Math.round((-xs.sIndic.aviahorizon_pitch + MaxAoA) * xHeight / (2 * MaxAoA));

		if (showDirection) {
			double compassRads = (double) Math.toRadians(xs.sIndic.compass);
			compassX = (int) (xWidth / 4 * Math.sin(compassRads));
			compassY = (int) (xWidth / 4 * Math.cos(compassRads));
		}

		parser.Blkx b = xc.getBlkx();
		if (b != null && b.valid) {
			if (showAoALimits) {
				if (xs.sState.AoA >= 0) {
					AoALimitU = Math.round(
							(-(b.NoFlapsWing.AoACritHigh - xs.sState.AoA) + MaxAoA) * xHeight / (2 * MaxAoA));

					AoAFLimitU = Math
							.round((-(b.aoaFuselageHigh - xs.sState.AoA) + MaxAoA) * xHeight / (2 * MaxAoA));
					AoALimitD = -10;
					AoAFLimitD = -10;
				} else {

					AoALimitD = Math.round(
							(-(b.NoFlapsWing.AoACritLow - xs.sState.AoA) + MaxAoA) * xHeight / (2 * MaxAoA));

					AoAFLimitD = Math
							.round((-(b.aoaFuselageLow - xs.sState.AoA) + MaxAoA) * xHeight / (2 * MaxAoA));
					AoALimitU = -10;
					AoAFLimitU = -10;
				}

			} else {
				if (0.3f * b.aoaHigh <= xs.sState.AoA)
					AoALimitU = Math.round((-(b.aoaHigh - xs.sState.AoA) + MaxAoA) * xHeight / (2 * MaxAoA));
				else
					AoALimitU = -10;
				if (0.3f * b.aoaLow >= xs.sState.AoA)
					AoALimitD = Math.round((-(b.aoaLow - xs.sState.AoA) + MaxAoA) * xHeight / (2 * MaxAoA));
				else
					AoALimitD = -10;
			}
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

		rotatePointMatrix(pS, xs.sIndic.aviahorizon_roll, pC, pT, pS.length);

		for (int i = 0; i < 4; i++) {
			pX[i] = pT[i].x;
			pY[i] = pT[i].y;
		}

		root.repaint();
	}

	@Override
	public void run() {
		while (doit) {
			try {
				Thread.sleep(Application.threadSleepTime);
			} catch (InterruptedException e) {
				e.printStackTrace();
			}
			if (xs != null && xs.SystemTime - freqCheckMili > freqMili) {
				freqCheckMili = xs.SystemTime;
				if (xs.sState != null && xs.sIndic != null) {
					drawTick();
				}
			}
		}
	}
}