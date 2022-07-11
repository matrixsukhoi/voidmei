package ui;

import java.awt.Color;
import java.awt.Container;
import java.awt.Font;
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

public class gearAndFlaps2 extends WebFrame implements Runnable {

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

	public void initPreview(controller xc) {
		init(xc, null);
		// System.out.println("初始化");
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

		slider.addMouseListener(new MouseAdapter() {
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
		slider.addMouseMotionListener(new MouseMotionAdapter() {
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
		l1.setFont(new Font(app.DefaultFontName, Font.PLAIN, 12));
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
		s1.setFont(app.DefaultFont);
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
		this.setCursor(app.blankCursor);
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明
		setShadeWidth(0);
		setBounds(lx, ly, 100, 160);

		getContentPane().setLayout(null);

		initpanel();
		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(false);
		setDefaultCloseOperation(3);
		setTitle(lang.gTitle);
		setAlwaysOnTop(true);
		root = this.getContentPane();
		setFocusable(false);
		setFocusableWindowState(false);// 取消窗口焦点
		setVisible(true);
		this.setCursor(app.blankCursor);
		if (xc.getconfig("enablegearAndFlapsEdge").equals("true"))
			setShadeWidth(10);

	}

	long gearCheckMili;

	public void drawTick() {
		// if (xs.sState != null) {
		if (xs.sState.gear >= 0) {
			if (xs.sState.gear == 0) {
				s1.setSelected(false);
				if (xs.sState.airbrake > 0) {
					// s1.setText("");
					s1.setText(lang.gBrake);
					// System.out.println(xs.sState.airbrake);
				} else {
					s1.setText("");

					// System.out.println(xs.sState.airbrake);
					// s1.setText(language.gBrake);
				}

			} else {
				if (xs.sState.gear == 100) {
					s1.setSelected(true);
					s1.setText(lang.gGear);
				} else {
					s1.setSelected(true);
					s1.setText(lang.gGearDown);
				}
			}
			if (xs.sState.airbrake > 0) {
				s1.setSelected(true);
				s1.setText(lang.gBrake);
			}
		}
		// System.out.println("gearandFlaps执行了");
		slider.setValue(xs.sState.flaps);
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