package ui;

import java.awt.Color;
import java.awt.Font;
import java.awt.SystemColor;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;

import javax.swing.SwingConstants;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.separator.WebSeparator;

import prog.app;
import prog.controller;
import prog.language;
import prog.service;

public class flightInfo extends WebFrame implements Runnable {
	/**
	 * 
	 */
	public volatile boolean doit=true;
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
	WebLabel label_6;
	String NumFont;
	private static final long serialVersionUID = 6759127498151892589L;
	int isDragging;
	int xx;
	int yy;
	Color Red = Color.BLACK;
	Color White = new Color(255, 255, 255, 255);

	public void initPreview(controller c) {
		init(c, null);
		// System.out.println("初始化");
		setShadeWidth(10);
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
		setVisible(true);
		//setFocusableWindowState(true);
		//setFocusable(true);
	}

	public WebLabel createWebLabel(String text) {
		WebLabel l1 = new WebLabel(text);
		l1.setShadeColor(new Color(0, 0, 0));
		l1.setDrawShade(true);
		return l1;
	}

	public void init(controller c, service s) {
		xc = c;
		xs = s;
		int lx;
		int ly;

		if(xc.getconfig("GlobalNumFont")!="")NumFont=xc.getconfig("GlobalNumFont");
		else NumFont=app.DefaultNumfontName;
		

		if (xc.getconfig("flightInfoFontC") != "")
			FontName = xc.getconfig("flightInfoFontC");
		else
			FontName = app.DefaultFont.getFontName();
		if (xc.getconfig("flightInfoFontaddC") != "")
			fontadd = Integer.parseInt(xc.getconfig("flightInfoFontaddC"));
		else
			fontadd = 0;
		// System.out.println(fontadd);

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

		this.setBounds(lx, ly, 580, 250);

		setAutoRequestFocus(false);
		setDefaultCloseOperation(WebFrame.EXIT_ON_CLOSE);
		setFont(new Font(FontName, Font.PLAIN, 12 + fontadd));
		// getContentPane().setBackground(Color.WHITE);
		setLayout(null);

		WebPanel panel = new WebPanel();
		panel.setBackground(new Color(0, 0, 0, 0));
		panel.setBounds(128, 0, 128, 64);
		this.add(panel);
		panel.setLayout(null);

		WebLabel label = createWebLabel(language.fIAS);
		label.setVerticalAlignment(SwingConstants.BOTTOM);
		label.setForeground(SystemColor.controlHighlight);
		label.setFont(new Font(FontName, Font.BOLD, 10 + fontadd));
		label.setHorizontalAlignment(SwingConstants.LEFT);
		label.setBounds(92, 0, 36, 31);
		panel.add(label);

		label_1 = createWebLabel("370");
		label_1.setForeground(Color.WHITE);
		label_1.setHorizontalAlignment(SwingConstants.RIGHT);
		label_1.setFont(new Font(NumFont, Font.PLAIN, 30+ fontadd));
		label_1.setBounds(0, 0, 82, 64);
		panel.add(label_1);

		WebLabel lblKmh = createWebLabel("Km/h");
		lblKmh.setHorizontalAlignment(SwingConstants.LEFT);
		lblKmh.setVerticalAlignment(SwingConstants.TOP);
		lblKmh.setForeground(Color.lightGray);
		lblKmh.setFont(new Font(NumFont, Font.PLAIN, 12 + fontadd));
		lblKmh.setBounds(92, 33, 36, 31);
		panel.add(lblKmh);

		WebPanel panel_1 = new WebPanel();
		panel_1.setLayout(null);
		panel_1.setBackground(new Color(0, 0, 0, 0));
		panel_1.setBounds(128, 64, 128, 64);
		this.add(panel_1);

		WebLabel label_2 = createWebLabel(language.fTAS);
		label_2.setVerticalAlignment(SwingConstants.BOTTOM);
		label_2.setHorizontalAlignment(SwingConstants.LEFT);
		label_2.setForeground(SystemColor.controlHighlight);
		label_2.setFont(new Font(FontName, Font.BOLD, 10 + fontadd));
		label_2.setBounds(92, 0, 36, 31);
		panel_1.add(label_2);

		label_3 = createWebLabel("370");
		label_3.setHorizontalAlignment(SwingConstants.RIGHT);
		label_3.setForeground(Color.WHITE);
		label_3.setFont(new Font(NumFont, Font.PLAIN, 30+ fontadd));
		label_3.setBounds(0, 0, 82, 64);
		panel_1.add(label_3);

		WebLabel lblKmh_1 = createWebLabel("Km/h");
		lblKmh_1.setVerticalAlignment(SwingConstants.TOP);
		lblKmh_1.setHorizontalAlignment(SwingConstants.LEFT);
		lblKmh_1.setForeground(Color.lightGray);
		lblKmh_1.setFont(new Font(NumFont, Font.PLAIN, 12 + fontadd));
		lblKmh_1.setBounds(92, 33, 36, 31);
		panel_1.add(lblKmh_1);

		WebPanel panel_2 = new WebPanel();
		panel_2.setLayout(null);
		panel_2.setBackground(new Color(0, 0, 0, 0));
		panel_2.setBounds(0, 0, 128, 64);
		this.add(panel_2);

		WebLabel label_5 = createWebLabel(language.fCompass);
		label_5.setVerticalAlignment(SwingConstants.BOTTOM);
		label_5.setHorizontalAlignment(SwingConstants.LEFT);
		label_5.setForeground(SystemColor.controlHighlight);
		label_5.setFont(new Font(FontName, Font.BOLD, 10 + fontadd));
		label_5.setBounds(92, 0, 36, 31);
		panel_2.add(label_5);

		label_6 = createWebLabel("181");
		label_6.setHorizontalAlignment(SwingConstants.RIGHT);
		label_6.setForeground(Color.WHITE);
		label_6.setFont(new Font(NumFont, Font.PLAIN, 30+ fontadd));
		label_6.setBounds(0, 0, 82, 64);
		panel_2.add(label_6);

		WebLabel lblDeg = createWebLabel("Deg");
		lblDeg.setVerticalAlignment(SwingConstants.TOP);
		lblDeg.setHorizontalAlignment(SwingConstants.LEFT);
		lblDeg.setForeground(Color.lightGray);
		lblDeg.setFont(new Font(NumFont, Font.PLAIN, 12 + fontadd));
		lblDeg.setBounds(92, 33, 36, 31);
		panel_2.add(lblDeg);

		WebPanel panel_4 = new WebPanel();
		panel_4.setLayout(null);
		panel_4.setBackground(new Color(0, 0, 0, 0));
		panel_4.setBounds(128, 128, 128, 64);
		this.add(panel_4);

		WebLabel label_7 = createWebLabel(language.fMach);
		label_7.setVerticalAlignment(SwingConstants.BOTTOM);
		label_7.setHorizontalAlignment(SwingConstants.LEFT);
		label_7.setForeground(SystemColor.controlHighlight);
		label_7.setFont(new Font(FontName, Font.BOLD, 10 + fontadd));
		label_7.setBounds(92, 0, 36, 31);
		panel_4.add(label_7);

		label_10 = createWebLabel("0.5");
		label_10.setHorizontalAlignment(SwingConstants.RIGHT);
		label_10.setForeground(Color.WHITE);
		label_10.setFont(new Font(NumFont, Font.PLAIN, 30+ fontadd));
		label_10.setBounds(0, 0, 82, 64);
		panel_4.add(label_10);

		WebLabel lblach = createWebLabel("Mach");
		lblach.setVerticalAlignment(SwingConstants.TOP);
		lblach.setHorizontalAlignment(SwingConstants.LEFT);
		lblach.setForeground(Color.lightGray);
		lblach.setFont(new Font(NumFont, Font.PLAIN, 12 + fontadd));
		lblach.setBounds(92, 33, 36, 31);
		panel_4.add(lblach);

		WebPanel panel_5 = new WebPanel();
		panel_5.setLayout(null);
		panel_5.setBackground(new Color(0, 0, 0, 0));
		panel_5.setBounds(0, 128, 128, 64);
		this.add(panel_5);

		WebLabel label_11 = createWebLabel(language.fWx);
		label_11.setVerticalAlignment(SwingConstants.BOTTOM);
		label_11.setHorizontalAlignment(SwingConstants.LEFT);
		label_11.setForeground(SystemColor.controlHighlight);
		label_11.setFont(new Font(FontName, Font.BOLD, 10 + fontadd));
		label_11.setBounds(92, 0, 36, 31);
		panel_5.add(label_11);

		label_12 = createWebLabel("20");
		label_12.setHorizontalAlignment(SwingConstants.RIGHT);
		label_12.setForeground(Color.WHITE);
		label_12.setFont(new Font(NumFont, Font.PLAIN, 30+ fontadd));
		label_12.setBounds(0, 0, 82, 64);
		panel_5.add(label_12);

		WebLabel lblDegs = createWebLabel("Deg/s");
		lblDegs.setVerticalAlignment(SwingConstants.TOP);
		lblDegs.setHorizontalAlignment(SwingConstants.LEFT);
		lblDegs.setForeground(Color.lightGray);
		lblDegs.setFont(new Font(NumFont, Font.PLAIN,12 + fontadd));
		lblDegs.setBounds(92, 33, 36, 31);
		panel_5.add(lblDegs);

		WebPanel panel_6 = new WebPanel();
		panel_6.setLayout(null);
		panel_6.setBackground(new Color(0, 0, 0, 0));
		panel_6.setBounds(256, 0, 128, 64);
		this.add(panel_6);

		WebLabel label_14 = createWebLabel(language.fAlt);
		label_14.setVerticalAlignment(SwingConstants.BOTTOM);
		label_14.setHorizontalAlignment(SwingConstants.LEFT);
		label_14.setForeground(SystemColor.controlHighlight);
		label_14.setFont(new Font(FontName, Font.BOLD, 10 + fontadd));
		label_14.setBounds(92, 0, 36, 31);
		panel_6.add(label_14);

		label_15 = createWebLabel("1024");
		label_15.setHorizontalAlignment(SwingConstants.RIGHT);
		label_15.setForeground(Color.WHITE);
		label_15.setFont(new Font(NumFont, Font.PLAIN, 30+ fontadd));
		label_15.setBounds(0, 0, 82, 64);
		panel_6.add(label_15);

		WebLabel lblM = createWebLabel("M");
		lblM.setVerticalAlignment(SwingConstants.TOP);
		lblM.setHorizontalAlignment(SwingConstants.LEFT);
		lblM.setForeground(Color.lightGray);
		lblM.setFont(new Font(NumFont, Font.PLAIN, 12 + fontadd));
		lblM.setBounds(92, 33, 36, 31);
		panel_6.add(lblM);

		WebPanel panel_7 = new WebPanel();
		panel_7.setLayout(null);
		panel_7.setBackground(new Color(0, 0, 0, 0));
		panel_7.setBounds(383, 0, 128, 64);
		this.add(panel_7);

		WebLabel label_16 = createWebLabel(language.fVario);
		label_16.setVerticalAlignment(SwingConstants.BOTTOM);
		label_16.setHorizontalAlignment(SwingConstants.LEFT);
		label_16.setForeground(SystemColor.controlHighlight);
		label_16.setFont(new Font(FontName, Font.BOLD, 10 + fontadd));
		label_16.setBounds(92, 0, 36, 31);
		panel_7.add(label_16);

		label_17 = createWebLabel("20");
		label_17.setHorizontalAlignment(SwingConstants.RIGHT);
		label_17.setForeground(Color.WHITE);
		label_17.setFont(new Font(NumFont, Font.PLAIN, 30+ fontadd));
		label_17.setBounds(0, 0, 82, 64);
		panel_7.add(label_17);

		WebLabel lblMs = createWebLabel("M/s");
		lblMs.setVerticalAlignment(SwingConstants.TOP);
		lblMs.setHorizontalAlignment(SwingConstants.LEFT);
		lblMs.setForeground(Color.lightGray);
		lblMs.setFont(new Font(NumFont, Font.PLAIN, 12 + fontadd));
		lblMs.setBounds(92, 33, 36, 31);
		panel_7.add(lblMs);

		WebPanel panel_8 = new WebPanel();
		panel_8.setLayout(null);
		panel_8.setBackground(new Color(0, 0, 0, 0));
		panel_8.setBounds(256, 64, 128, 64);
		this.add(panel_8);

		WebLabel lblSep = createWebLabel(language.fSEP);
		lblSep.setVerticalAlignment(SwingConstants.BOTTOM);
		lblSep.setHorizontalAlignment(SwingConstants.LEFT);
		lblSep.setForeground(SystemColor.controlHighlight);
		lblSep.setFont(new Font(FontName, Font.BOLD, 10 + fontadd));
		lblSep.setBounds(92, 0, 36, 31);
		panel_8.add(lblSep);

		label_19 = createWebLabel("20");
		label_19.setHorizontalAlignment(SwingConstants.RIGHT);
		label_19.setForeground(Color.WHITE);
		label_19.setFont(new Font(NumFont, Font.PLAIN, 30+ fontadd));
		label_19.setBounds(0, 0, 82, 64);
		panel_8.add(label_19);

		WebLabel lblMs_1 = createWebLabel("M/s");
		lblMs_1.setVerticalAlignment(SwingConstants.TOP);
		lblMs_1.setHorizontalAlignment(SwingConstants.LEFT);
		lblMs_1.setForeground(Color.lightGray);
		lblMs_1.setFont(new Font(NumFont, Font.PLAIN,12 + fontadd));
		lblMs_1.setBounds(92, 33, 36, 31);
		panel_8.add(lblMs_1);

		WebPanel panel_9 = new WebPanel();
		panel_9.setLayout(null);
		panel_9.setBackground(new Color(0, 0, 0, 0));
		panel_9.setBounds(0, 64, 128, 64);
		this.add(panel_9);

		WebLabel label_4 = createWebLabel(language.fGL);
		label_4.setVerticalAlignment(SwingConstants.BOTTOM);
		label_4.setHorizontalAlignment(SwingConstants.LEFT);
		label_4.setForeground(SystemColor.controlHighlight);
		label_4.setFont(new Font(FontName, Font.BOLD, 10 + fontadd));
		label_4.setBounds(92, 0, 36, 31);
		panel_9.add(label_4);

		label_13 = createWebLabel("4.0");
		label_13.setHorizontalAlignment(SwingConstants.RIGHT);
		label_13.setForeground(Color.WHITE);
		label_13.setFont(new Font(NumFont, Font.PLAIN, 30+ fontadd));
		label_13.setBounds(0, 0, 82, 64);
		panel_9.add(label_13);

		WebLabel lblGload = createWebLabel("G");
		lblGload.setVerticalAlignment(SwingConstants.TOP);
		lblGload.setHorizontalAlignment(SwingConstants.LEFT);
		lblGload.setForeground(Color.lightGray);
		lblGload.setFont(new Font(NumFont, Font.PLAIN, 12 + fontadd));
		lblGload.setBounds(92, 33, 36, 31);
		panel_9.add(lblGload);

		WebSeparator separator2 = new WebSeparator();
		//separator2.setBounds(0, 64, 512, 1);
		separator2.setBounds(128, 15, 640, 1);
		//this.getRootPane().add(separator2);

		WebSeparator separator1 = new WebSeparator();
		separator1.setOrientation(SwingConstants.VERTICAL);
		separator1.setBounds(515, -64, 1, 240);
		//this.getRootPane().add(separator1);

		//WebSeparator separator3 = new WebSeparator();
		//separator3.setBounds(0, 192, 640, 1);
		//this.add(separator3);

		// WebSeparator separator4 = new WebSeparator();
		// separator4.setBounds(0, 256, 128, 1);
		// this.add(separator4);
		// this.add(panel);
		// setBackground(Color.WHITE);


		setTitle(language.fTitle);
		setAlwaysOnTop(true);
		setFocusableWindowState(false);
		setFocusable(false);
		//System.out.println(this.isAlwaysOnTopSupported());
		//setAlwaysOnTop(true);
		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(false);
		setVisible(true);
		if (xc.getconfig("flightInfoEdge").equals("true"))setShadeWidth(10);
		else {
			setShadeWidth(0);
			this.getRootPane().add(separator1);
			//this.getRootPane().add(separator2);
		}
		//setAlwaysOnTop(true);

	}

	public void setNyRed() {
		label_13.setForeground(Red);
		//label_13.setForeground(White);
		label_13.setShadeColor(Color.WHITE);

	}
	public void setNyWhite() {
		label_13.setForeground(White);
		//label_13.setForeground(Red);
		label_13.setShadeColor(Color.black);

	}
	public void setSEPRed(){
		label_19.setForeground(Red);
	}

	@Override
	public void run() {
		// TODO Auto-generated method stub
		while (doit) {
			try {
				Thread.sleep(100);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			/*
			if(xs.Ny.charAt(0)-'6'>=0)setNyRed();
			else setNyWhite();
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
			//System.out.println("flightInfo执行了");
			repaint();

		}
	}

}