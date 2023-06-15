package ui;

import java.awt.Color;
import java.awt.Component;
import java.awt.Font;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;
import java.awt.event.MouseMotionAdapter;
import java.awt.event.MouseMotionListener;
import javax.swing.SwingConstants;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.separator.WebSeparator;

import prog.app;
import prog.controller;
import prog.lang;
import prog.service;

public class flightInfo2 extends WebFrame implements Runnable {
	/**
	 * 
	 */
	public volatile boolean doit = true;
	String FontName;
	int fontadd;
	controller xc;
	service xs;
	WebLabel label_1;
	WebLabel label_3;
	WebLabel label_9;
	WebLabel label_10;
	WebLabel label_12;
	WebLabel label_15;
	WebLabel label_17;
	WebLabel label_19;
	WebLabel label_13;
	WebLabel label_21;
	WebLabel label_6;
	WebLabel label_28;
	WebLabel lblGload;
	WebLabel label_25;
	WebLabel label_30;
	String NumFont;
	
	Color lblColor = app.colorUnit;
	Color lblNameColor = app.colorLabel;
	Color lblNumColor = app.colorNum;
	Color lblShadeColor = app.colorShade;
	
	private static final long serialVersionUID = 6759127498151892589L;
	int isDragging;
	int xx;
	int yy;
	Color Red = lblShadeColor;
	Color White = new Color(255, 255, 255, 255);
	
	public void webLabelRemoveML(WebLabel label) {
		// label.get
		Component[] tmp = label.getComponents();
		for (int i = 0; i < tmp.length; i++) {
			app.debugPrint("remove compnents" + tmp[i] + "mouseListener");
			MouseListener[] mls = tmp[i].getMouseListeners();
			MouseMotionListener[] mmls = tmp[i].getMouseMotionListeners();
			for (int j = 0; j < mls.length; j++) {
				tmp[i].removeMouseListener(mls[j]);
			}
			for (int j = 0; j < mmls.length; j++) {
				tmp[i].removeMouseMotionListener(mmls[j]);
			}
		}
	}

	public void webPanelRemoveML(WebPanel label) {
		// label.get
		// label.getMouseMotionListeners()
		// Component[] tmp = label.getComponents();

		MouseListener[] mls = label.getMouseListeners();
		MouseMotionListener[] mmls = label.getMouseMotionListeners();
		for (int j = 0; j < mls.length; j++) {
			app.debugPrint("remove ml" + mls[j]);
			label.removeMouseListener(mls[j]);
		}
		for (int j = 0; j < mmls.length; j++) {
			app.debugPrint("remove mls" + mmls[j]);
			label.removeMouseMotionListener(mmls[j]);
		}

		Component[] tmp = label.getComponents();
		for (int i = 0; i < tmp.length; i++) {
			app.debugPrint("remove compnents" + tmp[i] + "mouseListener");
			mls = tmp[i].getMouseListeners();
			mmls = tmp[i].getMouseMotionListeners();
			for (int j = 0; j < mls.length; j++) {
				tmp[i].removeMouseListener(mls[j]);
			}
			for (int j = 0; j < mmls.length; j++) {
				tmp[i].removeMouseMotionListener(mmls[j]);
			}
		}
	}
	

	
	public void initPreview(controller c) {
		init(c, null);
		// app.debugPrint("初始化");
		// setShadeWidth(10);
		this.setVisible(false);
		// this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 50));
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 1));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 1));// 顶部透明

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
		this.setCursor(null);
		setVisible(true);
		// setFocusableWindowState(true);
		// setFocusable(true);
	}


	
	public WebLabel createWebLabel(String text) {
		WebLabel l1 = new WebLabel(text);
		l1.setShadeColor(app.colorShade);
		l1.setDrawShade(true);
//		l1.setCursor(app.blankCursor);
		// l1.setEnabled(false);
		// l1.setCursor(app.blankCursor);
		// app.debugPrint(l1.text);
		return l1;
	}

	public void init(controller c, service s) {
		xc = c;
		xs = s;
		int lx;
		int ly;

		if (xc.getconfig("GlobalNumFont") != "")
			NumFont = xc.getconfig("GlobalNumFont");
		else
			NumFont = app.defaultNumfontName;

		if (xc.getconfig("flightInfoFontC") != "")
			FontName = xc.getconfig("flightInfoFontC");
		else
			FontName = app.defaultFont.getFontName();
		if (xc.getconfig("flightInfoFontaddC") != "")
			fontadd = Integer.parseInt(xc.getconfig("flightInfoFontaddC"));
		else
			fontadd = 0;
		// app.debugPrint(fontadd);

		if (xc.getconfig("flightInfoX") != "")
			lx = Integer.parseInt(xc.getconfig("flightInfoX"));
		else
			lx = 0;
		if (xc.getconfig("flightInfoY") != "")
			ly = Integer.parseInt(xc.getconfig("flightInfoY"));
		else
			ly = 0;

		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明

		// 旧的
		// this.setBounds(lx, ly, 580, 250);

		// 新的3x3
		this.setBounds(lx, ly, 460, 250);

		setAutoRequestFocus(false);
		setDefaultCloseOperation(WebFrame.EXIT_ON_CLOSE);
		setFont(new Font(FontName, Font.PLAIN, 10 + fontadd));
		// getContentPane().setBackground(lblNumColor);
		setLayout(null);

		WebPanel panel = new WebPanel();/*
										 * { public void paintComponent(Graphics
										 * g) { Graphics2D g2d = (Graphics2D) g;
										 * //g2d.setRenderingHint(RenderingHints
										 * .KEY_ANTIALIASING,
										 * RenderingHints.VALUE_ANTIALIAS_ON);
										 * //g2d.setRenderingHint(RenderingHints
										 * .KEY_TEXT_ANTIALIASING,
										 * RenderingHints.
										 * VALUE_TEXT_ANTIALIAS_ON);
										 * g2d.setTransform(g2d.getTransform().
										 * getRotateInstance(1)); repaint(); }
										 * };
										 */
		// addMouseListener(new MouseAdapter() {
		// @Override
		// public void mouseEntered(MouseEvent e) {
		// app.debugPrint(e);
		// }
		// @Override
		// public void mousePressed(MouseEvent e) {
		// app.debugPrint(e);
		//
		// }
		// @Override
		// public void mouseReleased(MouseEvent e) {
		// app.debugPrint(e);
		// }
		//
		// });
		//
		// addMouseMotionListener(new MouseMotionAdapter() {
		// @Override
		// public void mouseDragged(MouseEvent e) {
		// app.debugPrint(e);
		// }
		// @Override
		// public void mouseMoved(MouseEvent e) {
		// app.debugPrint(e);
		// }
		// });
		// this.getWebRootPaneUI().lis
		// webPanelRemoveML(panel);
//		panel.setCursor(app.blankCursor);
//		this.getRootPane().setCursor(app.blankCursor);
//		this.getContentPane().setCursor(app.blankCursor);
//		this.getGlassPane().setCursor(app.blankCursor);
//		this.getLayeredPane().setCursor(app.blankCursor);
		this.setCursor(app.blankCursor);
		// this.getContentPane().setCursor(app.blankCursor);
		// this.setCursor(app.blankCursor);
		// this.getWebRootPaneUI().setCu
		// this.setCursor
		panel.setBackground(new Color(0, 0, 0, 0));
		panel.setBounds(128, 0, 128, 64);
		this.add(panel);
		panel.setLayout(null);

		WebLabel label = createWebLabel(lang.fIAS);
		label.setVerticalAlignment(SwingConstants.BOTTOM);
		label.setForeground(lblNameColor);
		label.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
		label.setHorizontalAlignment(SwingConstants.LEFT);
		label.setBounds(92, 0, 36, 31);

		// webLabelRemoveML(label);

		panel.add(label);

		label_1 = createWebLabel("370");
		label_1.setForeground(lblNumColor);
		label_1.setHorizontalAlignment(SwingConstants.RIGHT);
		label_1.setFont(new Font(NumFont, Font.BOLD, 30 + fontadd));
		label_1.setBounds(0, 0, 82, 64);
		panel.add(label_1);

		WebLabel lblKmh = createWebLabel("Km/h");
		lblKmh.setHorizontalAlignment(SwingConstants.LEFT);
		lblKmh.setVerticalAlignment(SwingConstants.TOP);
		lblKmh.setForeground(lblColor);
		lblKmh.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblKmh.setBounds(92, 33, 36, 31);
		panel.add(lblKmh);

		// 航向和过载应该对换
		WebPanel panel_2 = new WebPanel();
		panel_2.setLayout(null);
		panel_2.setBackground(new Color(0, 0, 0, 0));
		panel_2.setBounds(256, 128, 128, 64);
		this.add(panel_2);
		{
			WebLabel label_5 = createWebLabel(lang.fCompass);
			label_5.setVerticalAlignment(SwingConstants.BOTTOM);
			label_5.setHorizontalAlignment(SwingConstants.LEFT);
			label_5.setForeground(lblNameColor);
			label_5.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
			label_5.setBounds(92, 0, 36, 31);
			panel_2.add(label_5);

			label_6 = createWebLabel("181");
			label_6.setHorizontalAlignment(SwingConstants.RIGHT);
			label_6.setForeground(lblNumColor);
			label_6.setFont(new Font(NumFont, Font.BOLD, 30 + fontadd));
			label_6.setBounds(0, 0, 82, 64);
			panel_2.add(label_6);

			WebLabel lblDeg = createWebLabel("Deg");
			lblDeg.setVerticalAlignment(SwingConstants.TOP);
			lblDeg.setHorizontalAlignment(SwingConstants.LEFT);
			lblDeg.setForeground(lblColor);
			lblDeg.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
			lblDeg.setBounds(92, 33, 36, 31);
			panel_2.add(lblDeg);
		}

		// 真空速和马赫数并列
		WebPanel panel_1 = new WebPanel();
		panel_1.setLayout(null);
		panel_1.setBackground(new Color(0, 0, 0, 0));
		panel_1.setBounds(128, 64, 128, 32);
		this.add(panel_1);

		WebLabel label_2 = createWebLabel(lang.fTAS);
		label_2.setVerticalAlignment(SwingConstants.TOP);
		label_2.setHorizontalAlignment(SwingConstants.LEFT);
		label_2.setForeground(lblNameColor);
		label_2.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
		label_2.setBounds(92, 0, 36, 31);
		panel_1.add(label_2);

		label_3 = createWebLabel("370");
		label_3.setHorizontalAlignment(SwingConstants.RIGHT);
		label_3.setForeground(lblNumColor);
		label_3.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_3.setBounds(0, 0, 82, 32);
		panel_1.add(label_3);

		WebLabel lblKmh_1 = createWebLabel("Km/h");
		lblKmh_1.setVerticalAlignment(SwingConstants.TOP);
		lblKmh_1.setHorizontalAlignment(SwingConstants.LEFT);
		lblKmh_1.setForeground(lblColor);
		lblKmh_1.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblKmh_1.setBounds(92, 16, 36, 31);
		panel_1.add(lblKmh_1);

		WebPanel panel_4 = new WebPanel();
		panel_4.setLayout(null);
		panel_4.setBackground(new Color(0, 0, 0, 0));
		panel_4.setBounds(128, 96, 128, 32);
		this.add(panel_4);

		WebLabel label_7 = createWebLabel(lang.fMach);
		label_7.setVerticalAlignment(SwingConstants.TOP);
		label_7.setHorizontalAlignment(SwingConstants.LEFT);
		label_7.setForeground(lblNameColor);
		label_7.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
		label_7.setBounds(92, 0, 36, 31);
		panel_4.add(label_7);

		label_10 = createWebLabel("0.5");
		label_10.setHorizontalAlignment(SwingConstants.RIGHT);
		label_10.setForeground(lblNumColor);
		label_10.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_10.setBounds(0, 0, 82, 32);
		panel_4.add(label_10);

		WebLabel lblach = createWebLabel("Mach");
		lblach.setVerticalAlignment(SwingConstants.TOP);
		lblach.setHorizontalAlignment(SwingConstants.LEFT);
		lblach.setForeground(lblColor);
		lblach.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblach.setBounds(92, 16, 36, 31);
		panel_4.add(lblach);

		WebPanel panel_5 = new WebPanel();
		panel_5.setLayout(null);
		panel_5.setBackground(new Color(0, 0, 0, 0));
		panel_5.setBounds(0, 128, 128, 64);
		this.add(panel_5);

		// r1
		WebLabel label_11 = createWebLabel(lang.fWx);
		label_11.setVerticalAlignment(SwingConstants.BOTTOM);
		label_11.setHorizontalAlignment(SwingConstants.LEFT);
		label_11.setForeground(lblNameColor);
		label_11.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
		label_11.setBounds(92, 0, 36, 16);
		panel_5.add(label_11);

		label_12 = createWebLabel("20");
		label_12.setHorizontalAlignment(SwingConstants.RIGHT);
		label_12.setForeground(lblNumColor);
		label_12.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_12.setBounds(0, 0, 82, 32);
		panel_5.add(label_12);

		WebLabel lblDegs = createWebLabel("Deg/s");
		lblDegs.setVerticalAlignment(SwingConstants.TOP);
		lblDegs.setHorizontalAlignment(SwingConstants.LEFT);
		lblDegs.setForeground(lblColor);
		lblDegs.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblDegs.setBounds(92, 16, 36, 16);
		panel_5.add(lblDegs);

		WebLabel label_24 = createWebLabel(lang.fTR);
		label_24.setVerticalAlignment(SwingConstants.BOTTOM);
		label_24.setHorizontalAlignment(SwingConstants.LEFT);
		label_24.setForeground(lblNameColor);
		label_24.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
		label_24.setBounds(92, 32, 36, 16);
		panel_5.add(label_24);

		label_25 = createWebLabel("1000");
		label_25.setHorizontalAlignment(SwingConstants.RIGHT);
		label_25.setForeground(lblNumColor);
		label_25.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
		label_25.setBounds(0, 32, 82, 32);
		panel_5.add(label_25);

		WebLabel label_26 = createWebLabel("M");
		label_26.setVerticalAlignment(SwingConstants.TOP);
		label_26.setHorizontalAlignment(SwingConstants.LEFT);
		label_26.setForeground(lblColor);
		label_26.setFont(new Font(FontName, Font.PLAIN, 10 + fontadd));
		label_26.setBounds(92, 48, 36, 16);
		panel_5.add(label_26);

		WebPanel panel_6 = new WebPanel();
		panel_6.setLayout(null);
		panel_6.setBackground(new Color(0, 0, 0, 0));
		panel_6.setBounds(256, 0, 128, 64);
		this.add(panel_6);

		WebLabel label_14 = createWebLabel(lang.fAlt);
		label_14.setVerticalAlignment(SwingConstants.BOTTOM);
		label_14.setHorizontalAlignment(SwingConstants.LEFT);
		label_14.setForeground(lblNameColor);
		label_14.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
		label_14.setBounds(92, 0, 36, 31);
		panel_6.add(label_14);

		label_15 = createWebLabel("1024");
		label_15.setHorizontalAlignment(SwingConstants.RIGHT);
		label_15.setForeground(lblNumColor);
		label_15.setFont(new Font(NumFont, Font.BOLD, 30 + fontadd));
		label_15.setBounds(0, 0, 82, 64);
		panel_6.add(label_15);

		WebLabel lblM = createWebLabel("M");
		lblM.setVerticalAlignment(SwingConstants.TOP);
		lblM.setHorizontalAlignment(SwingConstants.LEFT);
		lblM.setForeground(lblColor);
		lblM.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblM.setBounds(92, 33, 36, 31);
		panel_6.add(lblM);

		WebPanel panel_7 = new WebPanel();
		panel_7.setLayout(null);
		panel_7.setBackground(new Color(0, 0, 0, 0));
		panel_7.setBounds(128, 128, 128, 64);
		this.add(panel_7);
		{
			WebLabel label_16 = createWebLabel(lang.fVario);
			label_16.setVerticalAlignment(SwingConstants.BOTTOM);
			label_16.setHorizontalAlignment(SwingConstants.LEFT);
			label_16.setForeground(lblNameColor);
			label_16.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
			label_16.setBounds(92, 0, 36, 16);
			panel_7.add(label_16);

			label_17 = createWebLabel("20");
			label_17.setHorizontalAlignment(SwingConstants.RIGHT);
			label_17.setForeground(lblNumColor);
			label_17.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
			label_17.setBounds(0, 0, 82, 32);
			panel_7.add(label_17);

			WebLabel lblMs = createWebLabel("M/s");
			lblMs.setVerticalAlignment(SwingConstants.TOP);
			lblMs.setHorizontalAlignment(SwingConstants.LEFT);
			lblMs.setForeground(lblColor);
			lblMs.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
			lblMs.setBounds(92, 16, 36, 16);
			panel_7.add(lblMs);

			WebLabel label_27 = createWebLabel(lang.fAcc);
			label_27.setVerticalAlignment(SwingConstants.BOTTOM);
			label_27.setHorizontalAlignment(SwingConstants.LEFT);
			label_27.setForeground(lblNameColor);
			label_27.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
			label_27.setBounds(92, 32, 36, 16);
			panel_7.add(label_27);

			label_28 = createWebLabel("10");
			label_28.setHorizontalAlignment(SwingConstants.RIGHT);
			label_28.setForeground(lblNumColor);
			label_28.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
			label_28.setBounds(0, 32, 82, 32);
			panel_7.add(label_28);

			WebLabel lbl_29 = createWebLabel("M/s^2");
			lbl_29.setVerticalAlignment(SwingConstants.TOP);
			lbl_29.setHorizontalAlignment(SwingConstants.LEFT);
			lbl_29.setForeground(lblColor);
			lbl_29.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
			lbl_29.setBounds(92, 48, 36, 16);
			panel_7.add(lbl_29);
		}

		WebPanel panel_8 = new WebPanel();
		panel_8.setLayout(null);
		panel_8.setBackground(new Color(0, 0, 0, 0));
		panel_8.setBounds(256, 64, 128, 64);
		this.add(panel_8);

		WebLabel lblSep = createWebLabel(lang.fSEP);
		lblSep.setVerticalAlignment(SwingConstants.BOTTOM);
		lblSep.setHorizontalAlignment(SwingConstants.LEFT);
		lblSep.setForeground(lblNameColor);
		lblSep.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
		lblSep.setBounds(92, 0, 36, 31);
		panel_8.add(lblSep);

		label_19 = createWebLabel("20");
		label_19.setHorizontalAlignment(SwingConstants.RIGHT);
		label_19.setForeground(lblNumColor);
		label_19.setFont(new Font(NumFont, Font.BOLD, 30 + fontadd));
		label_19.setBounds(0, 0, 82, 64);
		panel_8.add(label_19);

		WebLabel lblMs_1 = createWebLabel("M/s");
		lblMs_1.setVerticalAlignment(SwingConstants.TOP);
		lblMs_1.setHorizontalAlignment(SwingConstants.LEFT);
		lblMs_1.setForeground(lblColor);
		lblMs_1.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
		lblMs_1.setBounds(92, 33, 36, 31);
		panel_8.add(lblMs_1);

		WebPanel panel_9 = new WebPanel();
		panel_9.setLayout(null);
		panel_9.setBackground(new Color(0, 0, 0, 0));
		panel_9.setBounds(0, 0, 128, 64);
		this.add(panel_9);
		{
			WebLabel label_4 = createWebLabel(lang.fGL);
			label_4.setVerticalAlignment(SwingConstants.BOTTOM);
			label_4.setHorizontalAlignment(SwingConstants.LEFT);
			label_4.setForeground(lblNameColor);
			label_4.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
			label_4.setBounds(92, 0, 36, 31);
			panel_9.add(label_4);

			label_13 = createWebLabel("4.0");
			label_13.setHorizontalAlignment(SwingConstants.RIGHT);
			label_13.setForeground(lblNumColor);
			label_13.setFont(new Font(NumFont, Font.BOLD, 30 + fontadd));
			label_13.setBounds(0, 0, 82, 64);
			panel_9.add(label_13);

			lblGload = createWebLabel("G");
			lblGload.setVerticalAlignment(SwingConstants.TOP);
			lblGload.setHorizontalAlignment(SwingConstants.LEFT);
			lblGload.setForeground(lblColor);
			// lblGload.setShadeColor(new Color(0,0,0,100));
			lblGload.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
			lblGload.setBounds(92, 33, 36, 31);
			panel_9.add(lblGload);
		}
		WebPanel panel_10 = new WebPanel();
		panel_10.setLayout(null);
		panel_10.setBackground(new Color(0, 0, 0, 0));
		panel_10.setBounds(0, 64, 128, 64);
		this.add(panel_10);
		{
			// aoa
			WebLabel label_20 = createWebLabel(lang.fAoA);
			label_20.setVerticalAlignment(SwingConstants.BOTTOM);
			label_20.setHorizontalAlignment(SwingConstants.LEFT);
			label_20.setForeground(lblNameColor);
			label_20.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
			label_20.setBounds(92, 0, 36, 16);
			panel_10.add(label_20);

			label_21 = createWebLabel("0.0");
			label_21.setHorizontalAlignment(SwingConstants.RIGHT);
			label_21.setForeground(lblNumColor);
			label_21.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
			label_21.setBounds(0, 0, 82, 32);
			panel_10.add(label_21);

			WebLabel lblAoA = createWebLabel("Deg");
			lblAoA.setVerticalAlignment(SwingConstants.TOP);
			lblAoA.setHorizontalAlignment(SwingConstants.LEFT);
			lblAoA.setForeground(lblColor);
			// lblAoA.setDrawShade(false);
			lblAoA.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
			lblAoA.setBounds(92, 16, 36, 16);
			panel_10.add(lblAoA);
			
			WebLabel label_29 = createWebLabel(lang.fTRr);
			label_29.setVerticalAlignment(SwingConstants.BOTTOM);
			label_29.setHorizontalAlignment(SwingConstants.LEFT);
			label_29.setForeground(lblNameColor);
			label_29.setFont(new Font(FontName, Font.BOLD, 12 + fontadd));
			label_29.setBounds(92, 32, 36, 16);
			panel_10.add(label_29);

			label_30 = createWebLabel("1.0");
			label_30.setHorizontalAlignment(SwingConstants.RIGHT);
			label_30.setForeground(lblNumColor);
			label_30.setFont(new Font(NumFont, Font.BOLD, 20 + fontadd));
			label_30.setBounds(0, 32, 82, 32);
			panel_10.add(label_30);

			WebLabel label_31 = createWebLabel("Deg/s");
			label_31.setVerticalAlignment(SwingConstants.TOP);
			label_31.setHorizontalAlignment(SwingConstants.LEFT);
			label_31.setForeground(lblColor);
			// lblAoA.setDrawShade(false);
			label_31.setFont(new Font(NumFont, Font.PLAIN, 10 + fontadd));
			label_31.setBounds(92, 48, 36, 16);
			panel_10.add(label_31);
			
		}
		WebSeparator separator2 = new WebSeparator();
		// separator2.setBounds(0, 64, 512, 1);
		separator2.setBounds(128, 15, 640, 1);
		// this.getRootPane().add(separator2);

		WebSeparator separator1 = new WebSeparator();
		separator1.setOrientation(SwingConstants.VERTICAL);
		// separator1.setSeparatorLightColor(lblNumColor);
		// separator1.setSeparatorLightUpperColor(lblNumColor);
		// separator1.setSeparatorColor(new Color(0x66,0xcc,0xff));
		separator1.setBounds(390, -64, 1, 240);
		// this.getRootPane().add(separator1);

		// WebSeparator separator3 = new WebSeparator();
		// separator3.setBounds(0, 192, 640, 1);
		// this.add(separator3);

		// WebSeparator separator4 = new WebSeparator();
		// separator4.setBounds(0, 256, 128, 1);
		// this.add(separator4);
		// this.add(panel);
		// setBackground(lblNumColor);

		setTitle(lang.fTitle);
		setAlwaysOnTop(true);
		setFocusableWindowState(false);
		setFocusable(false);

		// app.debugPrint(this.isAlwaysOnTopSupported());
		// setAlwaysOnTop(true);
		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(false);
		setVisible(true);
		if (xc.getconfig("flightInfoEdge").equals("true"))
			setShadeWidth(10);
		else {
			setShadeWidth(0);
//			this.getRootPane().add(separator1);
//			this.getRootPane().add(separator2);
		}
		// setAlwaysOnTop(true);

	}

	public void setNyRed() {
		label_13.setForeground(Red);
		// label_13.setForeground(White);
		label_13.setShadeColor(lblNumColor);

	}

	public void setNyWhite() {
		label_13.setForeground(White);
		// label_13.setForeground(Red);
		label_13.setShadeColor(lblShadeColor);

	}

	public void setSEPRed() {
		label_19.setForeground(Red);
	}
	long updateTime = 0;
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
			long systemTime = xs.SystemTime;
		
			if (systemTime - updateTime > xc.freqService) {
				updateTime = systemTime;
				/*
				 * if(xs.Ny.charAt(0)-'6'>=0)setNyRed(); else setNyWhite();
				 */
				label_6.setText(xs.compass);
				// label_9.setText("0");// 转弯率
				label_12.setText(xs.Wx);// 滚转率
				label_13.setText(xs.Ny);
				label_1.setText(xs.IAS);
				label_3.setText(xs.TAS);
				label_10.setText(xs.M);
				label_15.setText(xs.salt);
				label_19.setText(xs.sSEP);
				label_17.setText(xs.Vy);
				label_21.setText(xs.AoA);
				lblGload.setText("/"+xs.sHorizontalLoad+"G");
				label_25.setText(xs.sTurnRds);
				label_28.setText(xs.sAcc);
				label_30.setText(xs.sTurnRate);
				label_30.repaint();
				// app.debugPrint("flightInfo执行了");
//				repaint();
				this.getContentPane().repaint();
//				this.pa
//				this.getRootPane().repaint();
			}

		}
	}

}