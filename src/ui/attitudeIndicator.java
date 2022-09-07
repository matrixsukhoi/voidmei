package ui;

import java.awt.BasicStroke;
import java.awt.Color;
import java.awt.Container;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.Point;
import java.awt.Polygon;
import java.awt.RenderingHints;

import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;
import java.awt.event.MouseMotionAdapter;
import java.awt.event.MouseMotionListener;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.slider.WebSlider;

import java.awt.geom.AffineTransform;
import prog.app;
import prog.controller;
import prog.lang;
import prog.service;

public class attitudeIndicator extends WebFrame implements Runnable {

	/**
	 * 
	 */
	private static final long serialVersionUID = 4231053498040646357L;
	int isDragging;
	int xx;
	int yy;
	WebPanel topPanel;
	controller xc;
	service xs;
	WebSlider slider;
	WebLabel label_1;
	WebLabel label_3;
	WebLabel label_6;
	String NumFont;
	int AoA;
	int AoS;
	Boolean showDirection;

	Boolean showAoALimits;

	int bomb_dy;

	int AoALimitU;
	int AoALimitD;
	int AoAFLimitU;
	int AoAFLimitD;

	int compassX;
	int compassY;

	int Pitch;

	int Roll;

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
	Color transParentWhite = app.colorUnit;
	Color warning = app.colorWarning;

	// Color trans
	//
	public void rotateXY(int x[], int y[], int numPoints, float deg) {
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

	public static final int tickLine = 2;

	public static final int MaxAoA = 30;
	public static final int MaxAoS = 15;
	public int xWidth = 100;
	public int xHeight = 200;
	public boolean doit = true;
	private Container root;

	public static long freqMili = 40;

	public void setFrameOpaque() {
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明
		setShadeWidth(0);
	}

	public void initpreview(controller c) {
		init(c, null);

		// setShadeWidth(10);
		this.setVisible(false);
		this.getWebRootPaneUI().setTopBg(app.previewColor);
		this.getWebRootPaneUI().setMiddleBg(app.previewColor);
		// setFocusableWindowState(true);
		// setFocusable(true);
		attitudeIndicator t = this;
		addMouseListener(new MouseAdapter() {
			public void mouseEntered(MouseEvent e) {
				t.xWidth = t.getWidth() - 4;
				t.xHeight = t.getHeight() - 4;
				t.repaint();
			}

			public void mousePressed(MouseEvent e) {
				isDragging = 1;
				xx = e.getX();
				yy = e.getY();
				t.xWidth = t.getWidth() - 4;
				t.xHeight = t.getHeight() - 4;
				t.repaint();

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
					setLocation(left + e.getX() - xx, top + e.getY() - yy);
					setVisible(true);
					repaint();
				}
			}
		});

		this.setShowResizeCorner(true);
		this.setCursor(null);
		setVisible(true);

	}

	public void locater(Graphics2D g2d, int width, int height, int x, int y, int pitch, int center_round,
			int locator_size) {

		// 绘制边框
		g2d.setStroke(new BasicStroke(1));
		g2d.setColor(app.colorShadeShape);
		g2d.drawLine(0, 0, 0, height);
		g2d.drawLine(0, 0, width, 0);
		g2d.drawLine(0, height - 1, width - 1, height - 1);
		g2d.drawLine(width - 1, 0, width - 1, height - 1);

		for (int i = 0; i < 2 * tickLine; i++) {
			g2d.drawLine(pT[4 + 2 * i].x, pT[4 + 2 * i].y, pT[4 + 2 * i + 1].x, pT[4 + 2 * i + 1].y);
			// System.out.println("draw" + (4 + 2 * i));
		}
		// g2d.drawLine(pT[6].x, pT[6].y, pT[7].x, pT[7].y);

		// 画地面
		// g2d.fillRect(0, height/2 - 1, 2 *width, 2*height);
		// g2d.fillRect(0, pitch - 1, 2 *width, 2*height);

		// Polygon tgroundLevel = new Polygon();
		//
		// tgroundLevel.addPoint(-xWidth, pitch - 1);
		// tgroundLevel.addPoint(2 * xWidth, pitch - 1);
		// tgroundLevel.addPoint(2 * xWidth, 2 * xHeight);
		// tgroundLevel.addPoint(-xWidth, 2* xHeight);
		g2d.setColor(app.colorUnit);
		g2d.fillPolygon(pX, pY, 4);
		if (showDirection)
			g2d.drawLine(width / 2, height / 2, width / 2 + compassX, height / 2 + compassY);
		// g2d.drawRect(x, y, width, height);
		// g2d.setColor(Color.white);
		// g2d.drawLine(0, height/2 - 1, width/2 - center_round/2 - 1 , height/2
		// - 1);
		// g2d.drawLine(width/2 + center_round/2 , height/2 - 1, width ,
		// height/2 - 1);

		// Rectangle2D.Double all = new Rectangle2D.Double(0, 0, s, s);
		// Area a1 = new Area(all);
		// Area a2 = new Area(all);
		// GeneralPath aPart = new GeneralPath();
		// aPart.moveTo(0, 0);
		// aPart.lineTo(0, s);
		// aPart.lineTo(xSAxis.getX(), xSAxis.getY());
		// aPart.lineTo(xAxis.getX(), xAxis.getY());
		// aPart.closePath();
		// a1.subtract(new Area(aPart));
		// a2.subtract(a1);
		//
		//
		// 方向

		// g2d.drawLine(x, y, x + compassX, y + compassY);
		g2d.setStroke(new BasicStroke(3));

		g2d.setColor(app.colorNum);
		// g2d.drawLine(0, height / 2 - 1, width / 2 - center_round / 2 - 1,
		// height / 2 - 1);
		// g2d.drawLine(width / 2 + center_round / 2, height / 2 - 1, width,
		// height / 2 - 1);

		g2d.drawLine(width / 2 - center_round / 2 - width / 8 - 1, height / 2 - 1, width / 2 - center_round / 2 - 1,
				height / 2 - 1);
		g2d.drawLine(width / 2 + center_round / 2, height / 2 - 1, width / 2 + center_round / 2 + width / 8 - 1,
				height / 2 - 1);

		g2d.drawLine(0, height / 2 - 1, width / 8 - 1, height / 2 - 1);
		g2d.drawLine(width - width / 8 + 1, height / 2 - 1, width, height / 2 - 1);
		//

		// g2d.drawLine(0, pitch - 1, width/2 - center_round/2 - 1 , pitch - 1);
		// g2d.drawLine(width/2 + center_round/2 , pitch - 1, width , pitch -
		// 1);
		//
		//
		g2d.drawArc(width / 2 - center_round / 2 - 1, height / 2 - center_round / 2 - 1, center_round, center_round,
				-180, 180);
		// g2d.drawOval(width/2 - center_round/2 - 1 , height/2 - center_round/2
		// - 1, center_round , center_round);
		// 横线
		// g2d.drawLine(x - locator_size / 2 - 1, y - 1, x + locator_size / 2 -
		// 1, y - 1);
		// // 竖线
		// g2d.drawLine(x - 1, y - locator_size / 2 - 1, x - 1, y + locator_size
		// / 2 - 1);

		// g2d.setStroke(new BasicStroke(1));
		// g2d.drawLine(x - locator_size / 2 - 1, y - 1 , x + locator_size / 2 -
		// 1, y - 1);
		// g2d.drawLine(x - 1, y - locator_size / 2 - 1, x - 1, y + locator_size
		// / 2 - 1);

		g2d.setStroke(new BasicStroke(2));

		g2d.drawLine(x - locator_size / 2 - 1, y - 1, x + locator_size / 2 - 1, y - 1);
		g2d.drawLine(x - 1, y - locator_size / 2 - 1, x - 1, y + locator_size / 2 - 1);

		g2d.setColor(app.colorWarning);
		g2d.drawLine(0, AoAFLimitU, width - 1, AoAFLimitU);
		g2d.drawLine(0, AoAFLimitD, width - 1, AoAFLimitD);

		g2d.setColor(app.colorLabel);
		// 两条线
		g2d.drawLine(0, AoALimitU, width - 1, AoALimitU);
		g2d.drawLine(0, AoALimitD, width - 1, AoALimitD);

		if (showDirection) {
			g2d.drawLine(width / 2, height / 2, width / 2 + compassX, height / 2 + compassY);

			g2d.setColor(app.colorUnit);
			g2d.drawLine(width / 2, height / 2, width / 2 - compassX, height / 2 - compassY);
		}

		// g2d.drawOval(width/2 - 2, bomb_dy - 2, 2, 2);
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
		// slider1.setPreferredHeight(120);
		// slider1.setSnapToTicks(true);
		slider1.setProgressShadeWidth(0);
		slider1.setTrackShadeWidth(1);
		// slider1.setDrawThumb(false);
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
		// px=100;
		// py=100;
		WebPanel panel = new WebPanel() {
			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				// 开始绘图
				// g2d.draw
				g2d.setPaintMode();
				// g2d.set
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, app.graphAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, app.textAASetting);
				// g2d.setRenderingHint(RenderingHints.KEY_RENDERING,
				// RenderingHints.VALUE_RENDER_QUALITY);
				g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
						RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
				g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING, RenderingHints.VALUE_COLOR_RENDER_SPEED);
				
				// g2d.setColor(Color.white);
				// g2d.fillRect(0, 0, 200, 200);
				// 绘制十字星
				locater(g2d, xWidth, xHeight, AoS, AoA, Pitch, 12, 6);
//				g.dispose();
			}
		};

		panel.setBounds(0, 0, xWidth, xHeight);
		toppanel.add(panel);
	}

	public void init(controller c, service s) {
		int lx = 0;
		int ly = 0;

		xc = c;
		xs = s;
		// System.out.println("stickValue初始化了");
		if (xc.getconfig("GlobalNumFont") != "")
			NumFont = xc.getconfig("GlobalNumFont");
		else
			NumFont = app.DefaultNumfontName;

		if (xc.getconfig("attitudeIndicatorX") != "")
			lx = Integer.parseInt(xc.getconfig("attitudeIndicatorX"));
		else
			lx = 0;
		if (xc.getconfig("attitudeIndicatorY") != "")
			ly = Integer.parseInt(xc.getconfig("attitudeIndicatorY"));
		else
			ly = 0;

		if (xc.getconfig("attitudeIndicatorWidth") != "")
			xWidth = Integer.parseInt(xc.getconfig("attitudeIndicatorWidth"));
		else
			xWidth = 150;
		if (xc.getconfig("attitudeIndicatorHeight") != "")
			xHeight = Integer.parseInt(xc.getconfig("attitudeIndicatorHeight"));
		else
			xHeight = 300;

		if (xc.getconfig("attitudeIndicatorFreqMs") != "")
			freqMili = Integer.parseInt(xc.getconfig("attitudeIndicatorFreqMs"));
		else
			freqMili = 40;

		if (xc.getconfig("attitudeIndicatorUseNumColor") != "")
			if (Boolean.parseBoolean(xc.getconfig("attitudeIndicatorUseNumColor")))
				transParentWhite = app.colorNum;

		showDirection = false;
		if (xc.getconfig("attitudeIndicatorDisplayDirection") != "")
			if (Boolean.parseBoolean(xc.getconfig("attitudeIndicatorDisplayDirection")))
				showDirection = true;

		showAoALimits = true;
		if (xc.getconfig("attitudeIndicatorDisplayAoALimits") != "")
			showAoALimits = Boolean.parseBoolean(xc.getconfig("attitudeIndicatorDisplayDirection"));

		setFrameOpaque();

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

		this.setBounds(lx, ly, xWidth+4, xHeight+4);


		topPanel =new WebPanel() {
			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				// 开始绘图
				// g2d.draw
				g2d.setPaintMode();
				// g2d.set
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);
				// g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING,
				// RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
				g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
						RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
				// g2d.setColor(Color.white);
				// g2d.fillRect(0, 0, 200, 200);
				// 绘制十字星
				locater(g2d, xWidth, xHeight, AoS, AoA, Pitch, 12, 6);
				//g.dispose();
			}
		};

//		topPanel.setWebColoredBackground(false);
//		topPanel.setBackground(new Color(0, 0, 0, 0));

		initpanel(topPanel);
		add(topPanel);
		root = this.getContentPane();
//		setShowWindowButtons(false);
//		setShowTitleComponent(false);
//		setShowResizeCorner(false);
//		setDefaultCloseOperation(3);
//		setTitle(lang.vTitle);
//		setAlwaysOnTop(true);
//
//		setFocusable(false);
//		setFocusableWindowState(false);// 取消窗口焦点
		uiWebLafSetting.setWindowOpaque(this);
		if (xc.getconfig("enableAttituteIndicatorEdge").equals("true"))
			setShadeWidth(10);

		// if (xc.blkx.)
		// AoALimitU = Math.round((-xc.blkx.aoaHigh + MaxAoA) * xHeight / (2 *
		// MaxAoA));
		// AoALimitD = Math.round((-xc.blkx.aoaLow + MaxAoA) * xHeight / (2 *
		// MaxAoA));

	}

	public void rotatePointMatrix(Point[] origPoints, double angle, Point center, Point[] storeTo, int numPoints) {

		/*
		 * We ge the original points of the polygon we wish to rotate and rotate
		 * them with affine transform to the given angle. After the opeariont is
		 * complete the points are stored to the array given to the method.
		 */
		AffineTransform.getRotateInstance(Math.toRadians(angle), center.x, center.y).transform(origPoints, 0, storeTo,
				0, numPoints);

	}

	long freqCheckMili;

	public void drawTick() {
		AoA = Math.round((-xs.sState.AoA + MaxAoA) * xHeight / (2 * MaxAoA));
		// }
		// else AoS = 0;

		// if(xs.sState.AoS + MaxAoS >= 0){
		AoS = Math.round((xs.sState.AoS + MaxAoS) * xWidth / (2 * MaxAoS));
		// }
		// else AoS = 0;

		// if(xs.iIndic.aviahorizon_pitch + MaxAoA >= 0){
		Pitch = Math.round((-xs.sIndic.aviahorizon_pitch + MaxAoA) * xHeight / (2 * MaxAoA));

		// Roll = Math.round((-xs.iIndic.aviahorizon_roll + MaxAoA)
		// }
		// else AoS = 0;
		if (showDirection) {
			float compassRads = (float) Math.toRadians(xs.sIndic.compass);
			compassX = (int) (xWidth / 4 * Math.sin(compassRads));
			compassY = (int) (xWidth / 4 * Math.cos(compassRads));
		}

		// 使用剩余
		// System.out.println(1/2 * xc.blkx.aoaHigh);
		if (xc.blkx.valid) {
			if (showAoALimits) {
				if (xs.sState.AoA >= 0) {
					AoALimitU = Math.round(
							(-(xc.blkx.NoFlapsWing.AoACritHigh - xs.sState.AoA) + MaxAoA) * xHeight / (2 * MaxAoA));

					AoAFLimitU = Math
							.round((-(xc.blkx.aoaFuselageHigh - xs.sState.AoA) + MaxAoA) * xHeight / (2 * MaxAoA));
					AoALimitD = -10;
					AoAFLimitD = -10;
				} else {

					AoALimitD = Math.round(
							(-(xc.blkx.NoFlapsWing.AoACritLow - xs.sState.AoA) + MaxAoA) * xHeight / (2 * MaxAoA));

					AoAFLimitD = Math
							.round((-(xc.blkx.aoaFuselageLow - xs.sState.AoA) + MaxAoA) * xHeight / (2 * MaxAoA));
					AoALimitU = -10;
					AoAFLimitU = -10;
				}

			} else {
				if (0.3f * xc.blkx.aoaHigh <= xs.sState.AoA)
					AoALimitU = Math.round((-(xc.blkx.aoaHigh - xs.sState.AoA) + MaxAoA) * xHeight / (2 * MaxAoA));
				else
					AoALimitU = -10;
				if (0.3f * xc.blkx.aoaLow >= xs.sState.AoA)
					AoALimitD = Math.round((-(xc.blkx.aoaLow - xs.sState.AoA) + MaxAoA) * xHeight / (2 * MaxAoA));
				else
					AoALimitD = -10;
			}
		}
		// Point center = new Point(0,0);

		pS[0].x = -2 * xWidth;
		pS[0].y = 0;

		pS[1].x = 2 * xWidth;
		pS[1].y = 0;

		pS[2].x = 2 * xWidth;
		pS[2].y = 180 / MaxAoA * xHeight;

		pS[3].x = -2 * xWidth;
		pS[3].y = 180 / MaxAoA * xHeight;

		float start = -90.0f;
		float dTick = 90 / (tickLine + 1);
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
			// System.out.println("pS" + (4 + 4 * i) + "角度" + (start
			// +
			// (dTick * (i + 1))) + ","
			// + (-start - (dTick * (i + 1))));
			// System.out.println((int) Math.round((start + (dTick *
			// (i
			// + 1))) / MaxAoA * xHeight) + ","
			// + (int) Math.round((-start - (dTick * (i + 1))) /
			// MaxAoA
			// * xHeight));
		}
		// pS[4].x = -xWidth;
		// pS[4].y = -45/MaxAoA * xHeight;
		//
		// pS[5].x = xWidth;
		// pS[5].y = -45/MaxAoA * xHeight;
		//
		// pS[6].x = -xWidth;
		// pS[6].y = 45/MaxAoA * xHeight;
		//
		// pS[7].x = xWidth;
		// pS[7].y = 45/MaxAoA * xHeight;
		//
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

		// rotateXY(pX,pY, 4, xs.iIndic.aviahorizon_roll);

		// moveXY(pX,pY, 4, xWidth/2, Pitch - 1);
		// 平移
		//
		// pY[0] = Pitch - 1;
		//
		// pY[1] = Pitch - 1;

		// System.out.println(AoA + "," + AoS +":["+xs.AoA
		// +","+xs.AoS+"]");

		// 屏幕空间映射,乘以像素/角度
		// bomb_dy = (int)(xs.bangleR * 17.2);

		root.repaint();
	}

	@Override
	public void run() {
		// TODO Auto-generated method stub
		while (doit) {
			try {
				Thread.sleep(app.threadSleepTime);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			// px=100+xs.sState.aileron;
			// label_1.setText(xs.aileron);
			// py=100+xs.sState.elevator;
			// label_3.setText(xs.elevator);
			// slider.setValue(xs.sState.rudder);
			// label_6.setText(xs.rudder);
			// System.out.println("stickValue执行了");
			// 计算AoA偏移
			// if(xs.sState.AoA + MaxAoA >= 0){
			if (xs.SystemTime - freqCheckMili > freqMili) {
				freqCheckMili = xs.SystemTime;
				if (xs.sState != null && xs.sIndic != null) {
					drawTick();
				}
			}
		}
	}

}