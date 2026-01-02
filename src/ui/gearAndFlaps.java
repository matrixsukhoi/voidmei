package ui;

import java.awt.Color;
import java.awt.Container;
import java.awt.Font;
import java.awt.Graphics;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;
import java.awt.event.MouseMotionAdapter;
import java.awt.event.MouseMotionListener;

import javax.swing.SwingConstants;

import com.alee.extended.label.WebStepLabel;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.slider.WebSlider;

import prog.app;
import prog.controller;
import prog.lang;
import prog.service;

public class gearAndFlaps extends WebFrame implements Runnable {

	/**
	 * 
	 */
	private static final long serialVersionUID = 1925654763907916351L;
	int xx;
	int yy;
	int isDragging;
	service xs;
	controller xc;
	WebStepLabel s1;
	WebSlider slider;
	// Color transParentWhite = new Color(133, 133, 133, 67);
	Color transParentWhite = app.colorNum;
	Color transParentWhitePlus = app.colorNum;
	Color warning = app.colorWarning;
	public volatile boolean doit = true;
	Color transparent = new Color(0, 0, 0, 0);
	private Container root;
	private parser.state state;
	private WebPanel panel;
	private String FontName;
	private int fontadd;
	private int fontSize;
	private Font fontNum;
	private Font fontLabel;
	private Font fontUnit;
	private String warnText;
	private Color warnColor;
	private int barWidth;
	private int barHeight;
	private int flapPix;
	private String flapText;
	private int width;
	private int height;
	private int gap;

	public void initPreview(controller xc) {
		init(xc, null);
		// app.debugPrint("初始化");
		// setShadeWidth(10);
		this.setVisible(false);
		// this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 50));
		this.getWebRootPaneUI().setMiddleBg(app.previewColor);// 中部透明
		this.getWebRootPaneUI().setTopBg(app.previewColor);// 顶部透明
		// setFocusableWindowState(true);
		// setFocusable(true);

		addMouseListener(new MouseAdapter() {
			public void mouseEntered(MouseEvent e) {
				/*
				 * if(A.tag==0){ if(f.mode==1){ A.setVisible(false);
				 * A.visibletag=0; } }
				 */
			}

			public void mousePressed(MouseEvent e) {
				isDragging = 1;
				xx = e.getX();
				yy = e.getY();

			}

			public void mouseReleased(MouseEvent e) {
				if (isDragging == 1) {
					isDragging = 0;
				}
				/*
				 * if(A.tag==0){ A.setVisible(false); }
				 */
			}
			/*
			 * public void mouseReleased(MouseEvent e){ if(A.tag==0){
			 * A.setVisible(true); } }
			 */
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
		//
		// slider.addMouseListener(new MouseAdapter() {
		// public void mouseEntered(MouseEvent e) {
		// /*
		// * if(A.tag==0){ if(f.mode==1){ A.setVisible(false);
		// * A.visibletag=0; } }
		// */
		// }
		//
		// public void mousePressed(MouseEvent e) {
		// isDragging = 1;
		// xx = e.getX();
		// yy = e.getY();
		//
		// }
		//
		// public void mouseReleased(MouseEvent e) {
		// if (isDragging == 1) {
		// isDragging = 0;
		// }
		// /*
		// * if(A.tag==0){ A.setVisible(false); }
		// */
		// }
		// /*
		// * public void mouseReleased(MouseEvent e){ if(A.tag==0){
		// * A.setVisible(true); } }
		// */
		// });
		// slider.addMouseMotionListener(new MouseMotionAdapter() {
		// public void mouseDragged(MouseEvent e) {
		// if (isDragging == 1) {
		// int left = getLocation().x;
		// int top = getLocation().y;
		// setLocation(left + e.getX() - xx, top + e.getY() - yy);
		// setVisible(true);
		// repaint();
		// }
		// }
		// });
		this.setCursor(null);
		setVisible(true);
	}

	public void initslider(WebSlider slider1) {
		slider1.setMinimum(0);
		slider1.setMaximum(100);
		slider1.setValue(0);
		slider1.setDrawProgress(true);
		slider1.setMinorTickSpacing(25);
		slider1.setMajorTickSpacing(50);
		slider1.setOrientation(SwingConstants.VERTICAL);
		// slider1.t(ComponentOrientation.RIGHT_TO_LEFT);
		slider1.setPaintTicks(true);
		slider1.setPaintLabels(false);
		slider1.setDrawThumb(false);

		slider1.setProgressRound(0);
		slider1.setTrackRound(1);
		// slider1.setSharpThumbAngle(true);
		// slider1.setAngledThumb(true);
		// slider1.setThumbAngleLength(5);
		// slider1.setPreferredHeight(120);
		// slider1.setSnapToTicks(true);
		slider1.setProgressShadeWidth(0);
		slider1.setTrackShadeWidth(0);
		// slider1.setDrawThumb(false);
		slider1.setThumbShadeWidth(0);
		slider1.setThumbBgBottom(transparent);
		slider1.setThumbBgTop(transparent);
		slider1.setTrackBgBottom(transparent);
		slider1.setTrackBgTop(transparent);
		slider1.setProgressBorderColor(transparent);
		slider1.setProgressTrackBgBottom(transParentWhitePlus);
		slider1.setProgressTrackBgTop(transParentWhite);
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
		l1.setShadeColor(app.colorShade);
		l1.setDrawShade(true);
		l1.setForeground(app.colorLabel);
		l1.setFont(new Font(app.defaultFontName, Font.PLAIN, 12));
		return l1;
	}

	public void initpanel() {
		/*
		 * WebRadioButton rdbtnNewRadioButton = new
		 * WebRadioButton("\u8D77\u843D\u67B6"); rdbtnNewRadioButton.setFont(new
		 * Font(app.DefaultFontName,Font.PLAIN,12));
		 * rdbtnNewRadioButton.setForeground(Color.WHITE);
		 * rdbtnNewRadioButton.getWebUI().setShadeWidth(5);
		 * rdbtnNewRadioButton.setHorizontalAlignment(SwingConstants.CENTER);
		 * rdbtnNewRadioButton.setBounds(0, 0, 90, 30);
		 */

		s1 = new WebStepLabel("Gear");
		// s1.setFont(new Font(app.DefaultNumfontName,Font.PLAIN,10));

		s1.setBounds(0, 0, 90, 30);
		s1.setFontSize(10);
		s1.setForeground(app.colorLabel);
		s1.setDrawShade(true);
		s1.setShadeColor(app.colorShade);
		s1.setBottomBgColor(new Color(0, 0, 0, 0));
		s1.setTopBgColor(new Color(0, 0, 0, 0));
		s1.setFont(app.defaultFont);
		// s1.setSelected(true);
		s1.setSelectedBgColor(warning);

		getContentPane().add(s1);

		WebPanel panel = new WebPanel();
		panel.setBounds(0, 30, 90, 120);
		getContentPane().add(panel);
		panel.setLayout(null);
		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));

		slider = new WebSlider();
		initslider(slider);
		slider.setBounds(0, 30, 90, 90);
		panel.add(slider);

		WebLabel lblNewLabel = createWebLabel(lang.gFlaps);
		lblNewLabel.setHorizontalAlignment(SwingConstants.CENTER);
		lblNewLabel.setBounds(0, 0, 90, 30);
		panel.add(lblNewLabel);
	}

	public void init(controller c, service s) {
		xc = c;
		xs = s;
		if (s != null)
			state = s.sState;
		int lx = 0;
		int ly = 0;

		if (xc.getconfig("gearAndFlapsX") != "")
			lx = Integer.parseInt(xc.getconfig("gearAndFlapsX"));
		else
			lx = 0;
		if (xc.getconfig("gearAndFlapsY") != "")
			ly = Integer.parseInt(xc.getconfig("gearAndFlapsY"));
		else
			ly = 0;

		if (xc.getconfig("flightInfoFontC") != "")
			FontName = xc.getconfig("flightInfoFontC");
		else
			FontName = app.defaultFont.getFontName();
		if (xc.getconfig("flightInfoFontaddC") != "")
			fontadd = Integer.parseInt(xc.getconfig("flightInfoFontaddC"));
		else
			fontadd = 0;

		fontSize = 24 + fontadd;
		fontNum = new Font(FontName, Font.BOLD, fontSize);
		fontLabel = new Font(FontName, Font.BOLD, Math.round(fontSize / 2.0f));
		fontUnit = new Font(FontName, Font.PLAIN, Math.round(fontSize / 2.0f));

		barWidth = fontSize >> 1;
		barHeight = 4 * fontSize;

		warnText = String.format("%s", lang.gGear);
		warnColor = app.colorNum;

		flapPix = barHeight * 50 / 100;
		flapText = String.format("%3d", 50);

		width = 2 * fontSize;
		height = 5 * fontSize;

		this.setCursor(app.blankCursor);
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明
		setShadeWidth(0);
		setBounds(lx, ly, width, height);

		gap = (int) (0.2 * fontSize);
		// initpanel();
		panel = new WebPanel() {

			/**
			 * 
			 */
			private static final long serialVersionUID = 21520599493310317L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				// 开始绘图
				g2d.setPaintMode();
				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, app.graphAASetting);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, app.textAASetting);
				// g2d.setRenderingHint(RenderingHints.KEY_RENDERING,
				// RenderingHints.VALUE_RENDER_QUALITY);
				g2d.setRenderingHint(RenderingHints.KEY_ALPHA_INTERPOLATION,
						RenderingHints.VALUE_ALPHA_INTERPOLATION_SPEED);
				g2d.setRenderingHint(RenderingHints.KEY_COLOR_RENDERING, RenderingHints.VALUE_COLOR_RENDER_SPEED);

				// 绘制襟翼百分比,起落架百分比
				int dy = fontSize >> 1;
				uiBaseElem.__drawStringShade(g2d, 0, dy, 1, warnText, fontLabel, warnColor);
				// dy+=1.5 * fontSize;
				dy += barHeight + gap;
				uiBaseElem.drawVBarTextNum(g2d, 0, dy, barWidth, barHeight, flapPix, 1, app.colorNum, "",
						"F" + flapText, fontLabel, fontLabel);
				// g2d.drawLine(0, 0, 100, 100);
				// dy+=1.5 * fontSize;

				g.dispose();
			}
		};
		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));
		// panel.setBounds(lx, ly, 100, 160);

		this.add(panel);
		// setShowWindowButtons(false);
		// setShowTitleComponent(false);
		// setShowResizeCorner(false);
		// setDefaultCloseOperation(3);
		// setTitle(lang.gTitle);
		// setAlwaysOnTop(true);
		// root = this.getContentPane();
		// setFocusable(false);
		// setFocusableWindowState(false);// 取消窗口焦点
		// setVisible(true);
		// this.setCursor(app.blankCursor)
		setTitle("gearAndFlaps");
		uiWebLafSetting.setWindowOpaque(this);
		root = this.getContentPane();
		if (xc.getconfig("enablegearAndFlapsEdge").equals("true"))
			setShadeWidth(10);

	}

	long gearCheckMili;

	public void drawTick() {
		// if (xs.sState != null) {
		if (xs.sState.gear >= 0) {
			if (xs.sState.gear == 0) {
				// s1.setSelected(false);
				// if (xs.sState.airbrake > 0) {
				// warnText= warnText + " " + lang.gBrake;
				// warnColor = app.colorWarning;
				// // app.debugPrint(xs.sState.airbrake);
				// } else {
				warnText = "";
				warnColor = app.colorNum;
				// app.debugPrint(xs.sState.airbrake);
				// s1.setText(language.gBrake);
				// }

			} else {
				if (xs.sState.gear == 100) {
					// s1.setSelected(true);
					// s1.setText(lang.gGear);
					warnText = lang.gGear;
					warnColor = app.colorNum;
				} else {
					// s1.setSelected(true);
					warnText = lang.gGearDown;
					// s1.setText(lang.gGearDown);
					warnColor = app.colorWarning;
				}
			}
			if (xs.sState.airbrake > 0) {
				// s1.setSelected(true);
				// s1.setText(lang.gBrake);
				warnText = warnText + " " + lang.gBrake;
				warnColor = app.colorWarning;
			}
		}
		int flps = state.flaps;
		if (flps >= 0)
			flapPix = flps * barHeight / 100;
		else {
			flapPix = 0;
			flps = 0;
		}

		flapText = String.format("%3d", flps);

		// // app.debugPrint("gearandFlaps执行了");
		// slider.setValue(xs.sState.flaps);

		root.repaint();
		// }
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
			long t = xs.SystemTime;
			if (t - gearCheckMili > 2 * xc.freqService) {
				gearCheckMili = t;
				drawTick();
			}
		}
	}

}