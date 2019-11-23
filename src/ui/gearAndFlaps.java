package ui;

import java.awt.Color;
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
import prog.language;
import prog.service;

public class gearAndFlaps extends WebFrame implements Runnable{

	
	
	
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
	Color transParentWhite = new Color(255, 255, 255, 100);
	public volatile boolean doit=true;
	public void initPreview(controller xc){
		init(xc,null);
		// System.out.println("初始化");
		setShadeWidth(10);
		this.setVisible(false);
		// this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 50));
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 1));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 1));// 顶部透明
		//setFocusableWindowState(true);
		//setFocusable(true);

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
	}
	public void initslider(WebSlider slider1){
		slider1.setMinimum(0);
		slider1.setMaximum(100);
		slider1.setValue(0);
		slider1.setDrawProgress(true);
		slider1.setMinorTickSpacing(25);
		slider1.setMajorTickSpacing(50);
		slider1.setOrientation(SwingConstants.VERTICAL);
		//slider1.setComponentOrientation(ComponentOrientation.RIGHT_TO_LEFT);
		slider1.setPaintTicks(true);
		slider1.setPaintLabels(false);
		
		//slider1.setPreferredHeight(120);
		// slider1.setSnapToTicks(true);
		slider1.setProgressShadeWidth(0);
		slider1.setTrackShadeWidth(1);
		// slider1.setDrawThumb(false);
		slider1.setThumbShadeWidth(2);
		slider1.setThumbBgBottom(new Color(0, 0, 0, 0));
		slider1.setThumbBgTop(new Color(0, 0, 0, 0));
		slider1.setTrackBgBottom(new Color(0, 0, 0, 0));
		slider1.setTrackBgTop(new Color(0, 0, 0, 0));
		slider1.setProgressBorderColor(new Color(0, 0, 0, 0));
		slider1.setProgressTrackBgBottom(transParentWhite);
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
		l1.setShadeColor(new Color(0, 0, 0));
		l1.setDrawShade(true);
		l1.setForeground(Color.WHITE);
		l1.setFont(new Font(app.DefaultFontName,Font.PLAIN,12));
		return l1;
	}
	
	public void initpanel(){
		/*
		WebRadioButton rdbtnNewRadioButton = new WebRadioButton("\u8D77\u843D\u67B6");
		rdbtnNewRadioButton.setFont(new Font(app.DefaultFontName,Font.PLAIN,12));
		rdbtnNewRadioButton.setForeground(Color.WHITE);
		rdbtnNewRadioButton.getWebUI().setShadeWidth(5);
		rdbtnNewRadioButton.setHorizontalAlignment(SwingConstants.CENTER);
		rdbtnNewRadioButton.setBounds(0, 0, 90, 30);
		*/
		
		s1 = new WebStepLabel ( "Gear" );
		//s1.setFont(new Font(app.DefaultNumfontName,Font.PLAIN,10));
		s1.setBounds(0, 0, 90, 30);
		s1.setFontSize(10);
		s1.setForeground(Color.white);
		s1.setDrawShade(false);
		s1.setBottomBgColor(new Color(0,0,0,0));
		s1.setTopBgColor(new Color(0,0,0,0));
		s1.setFont(app.DefaultFont);
		//s1.setSelected(true);
		s1.setSelectedBgColor(new Color(0,0,0,100));
		
		getContentPane().add(s1);
		
		WebPanel panel = new WebPanel();
		panel.setBounds(0, 30, 90, 120);
		getContentPane().add(panel);
		panel.setLayout(null);
		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0,0,0,0));
		
		slider = new WebSlider();
		initslider(slider);
		slider.setBounds(0, 30, 90, 90);
		panel.add(slider);
		
		
		WebLabel lblNewLabel = createWebLabel(language.gFlaps);
		lblNewLabel.setHorizontalAlignment(SwingConstants.CENTER);
		lblNewLabel.setBounds(0, 0, 90, 30);
		panel.add(lblNewLabel);
	}
	public void init(controller c,service s){
		xc=c;
		xs=s;
		int lx=0;
		int ly=0;
		
		
		if (xc.getconfig("gearAndFlapsX") != "")
			lx = Integer.parseInt(xc.getconfig("gearAndFlapsX"));
		else
			lx = 0;
		if (xc.getconfig("gearAndFlapsY") != "")
			ly = Integer.parseInt(xc.getconfig("gearAndFlapsY"));
		else
			ly = 0;
		
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明
		setShadeWidth(0);
		setBounds(lx,ly,100,160);
		
		
		getContentPane().setLayout(null);
		
		initpanel();
		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(false);
		setDefaultCloseOperation(3);
		setTitle(language.gTitle);
		setAlwaysOnTop(true);
		
		setFocusable(false);
		setFocusableWindowState(false);// 取消窗口焦点
		setVisible(true);
		if(xc.getconfig("enablegearAndFlapsEdge").equals("true"))setShadeWidth(10);
	}
	@Override
	public void run() {
		// TODO Auto-generated method stub
		while(doit){
			try {
				Thread.sleep(200);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			if(xs.sState.gear==0){
				s1.setSelected(false);
				s1.setText("");
				
			}
			else {
				s1.setSelected(true);
				s1.setText(language.gGear);
			}
			if(xs.sState.airbrake>0){
				s1.setSelected(true);
				s1.setText(language.gBrake);
			}
			//System.out.println("gearandFlaps执行了");
			slider.setValue(xs.sState.flaps);
			repaint();
		}
	}
	
	
}