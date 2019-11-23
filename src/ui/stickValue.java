package ui;

import java.awt.BasicStroke;
import java.awt.Color;
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

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.slider.WebSlider;

import prog.app;
import prog.controller;
import prog.language;
import prog.service;

public class stickValue extends WebFrame implements Runnable{

	
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
	int px;
	int py;
	public volatile boolean doit=true;
	public void setFrameOpaque() {
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明
		setShadeWidth(0);
	}
	public void initpreview(controller c){
		init(c, null);
		
		setShadeWidth(10);
		this.setVisible(false);
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 1));
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 1));
		//setFocusableWindowState(true);
		//setFocusable(true);

		addMouseListener(new MouseAdapter() {
			public void mouseEntered(MouseEvent e) {
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
		setVisible(true);
		
	}
	public void locater(Graphics2D g2d,int x,int y,int width){
		
		
		//绘制边框
		g2d.setStroke(new BasicStroke(1));
		g2d.setColor(Color.gray);
		g2d.drawLine(0, 0,0 ,200 );
		g2d.drawLine(0, 0,200 ,0 );
		g2d.drawLine(0, 199,199,199 );
		g2d.drawLine(199, 0,199 ,199 );
		
		g2d.setStroke(new BasicStroke(3));
		g2d.setColor(Color.white);
		
		
		//横线
		g2d.drawLine(x-width/2-1, y-1,x+width/2-1 ,y-1 );
		//竖线
		g2d.drawLine(x-1, y-width/2-1, x-1,y+width/2-1);
	}
	public void initslider(WebSlider slider1){
		slider1.setMinimum(-100);
		slider1.setMaximum(100);
		slider1.setValue(0);
		slider1.setDrawProgress(true);
		slider1.setMinorTickSpacing(25);
		slider1.setMajorTickSpacing(50);
		slider1.setPaintTicks(true);
		slider1.setPaintLabels(true);
		//slider1.setPreferredHeight(120);
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
	
	public void initpanel(WebPanel toppanel){
		toppanel.setLayout(null);
		px=100;
		py=100;
		WebPanel panel = new WebPanel(){
			private static final long serialVersionUID = -9061280572815010060L;

			public void paintComponent(Graphics g) {
				Graphics2D g2d = (Graphics2D) g;
				// 开始绘图
				// g2d.draw

				g2d.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);
				g2d.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
				//g2d.setColor(Color.white);
				//g2d.fillRect(0, 0, 200, 200);
				//绘制十字星
				locater(g2d,px,py,6);
			}
		};
		
		//panel.setBackground(new Color(0,0,0,0));
		panel.setBounds(0, 0, 200, 200);
		toppanel.add(panel);
		
		slider = new WebSlider();
		initslider(slider);
		
		slider.setBounds(0, 200, 200, 64);
		toppanel.add(slider);
		
		WebPanel panel_1 = new WebPanel();
		panel_1.setLayout(null);
		panel_1.setBackground(new Color(0,0,0,0));
		panel_1.setBounds(200, 0, 128, 64);
		toppanel.add(panel_1);
		
		WebLabel lblX = createWebLabel(language.vAileron);
		lblX.setVerticalAlignment(SwingConstants.BOTTOM);
		lblX.setHorizontalAlignment(SwingConstants.LEFT);
		lblX.setForeground(Color.WHITE);
		lblX.setFont(new Font(app.DefaultFontName, Font.PLAIN, 10));
		lblX.setBounds(92, 0, 36, 31);
		panel_1.add(lblX);
		
		label_1 = createWebLabel("-50");
		label_1.setHorizontalAlignment(SwingConstants.RIGHT);
		label_1.setForeground(Color.WHITE);
		label_1.setFont(new Font(NumFont, Font.PLAIN, 28));
		label_1.setBounds(0, 0, 82, 64);
		panel_1.add(label_1);
		
		WebLabel label_2 = createWebLabel("%");
		label_2.setVerticalAlignment(SwingConstants.TOP);
		label_2.setHorizontalAlignment(SwingConstants.LEFT);
		label_2.setForeground(Color.WHITE);
		label_2.setFont(new Font(NumFont, Font.BOLD, 12));
		label_2.setBounds(92, 33, 36, 31);
		panel_1.add(label_2);
		
		WebPanel panel_2 = new WebPanel();
		panel_2.setLayout(null);
		panel_2.setBackground(new Color(0,0,0,0));
		panel_2.setBounds(200, 64, 128, 64);
		toppanel.add(panel_2);
		
		WebLabel lblY = createWebLabel(language.vElevator);
		lblY.setVerticalAlignment(SwingConstants.BOTTOM);
		lblY.setHorizontalAlignment(SwingConstants.LEFT);
		lblY.setForeground(Color.WHITE);
		lblY.setFont(new Font(app.DefaultFontName, Font.PLAIN, 10));
		lblY.setBounds(92, 0, 36, 31);
		panel_2.add(lblY);
		
		label_3 = createWebLabel("16");
		label_3.setHorizontalAlignment(SwingConstants.RIGHT);
		label_3.setForeground(Color.WHITE);
		label_3.setFont(new Font(NumFont, Font.PLAIN, 28));
		label_3.setBounds(0, 0, 82, 64);
		panel_2.add(label_3);
		
		WebLabel label_4 = createWebLabel("%");
		label_4.setVerticalAlignment(SwingConstants.TOP);
		label_4.setHorizontalAlignment(SwingConstants.LEFT);
		label_4.setForeground(Color.WHITE);
		label_4.setFont(new Font(NumFont, Font.BOLD, 12));
		label_4.setBounds(92, 33, 36, 31);
		panel_2.add(label_4);
		
		WebPanel panel_3 = new WebPanel();
		panel_3.setLayout(null);
		panel_3.setBackground(new Color(0,0,0,0));
		panel_3.setBounds(200, 200, 128, 64);
		toppanel.add(panel_3);
		
		WebLabel lblZ = createWebLabel(language.vRudder);
		lblZ.setVerticalAlignment(SwingConstants.BOTTOM);
		lblZ.setHorizontalAlignment(SwingConstants.LEFT);
		lblZ.setForeground(Color.WHITE);
		lblZ.setFont(new Font(app.DefaultFontName, Font.PLAIN, 10));
		lblZ.setBounds(92, 0, 36, 31);
		panel_3.add(lblZ);
		
		label_6 = createWebLabel("0");
		label_6.setHorizontalAlignment(SwingConstants.RIGHT);
		label_6.setForeground(Color.WHITE);
		label_6.setFont(new Font(NumFont, Font.PLAIN, 28));
		label_6.setBounds(0, 0, 82, 64);
		panel_3.add(label_6);
		
		WebLabel label_7 = createWebLabel("%");
		label_7.setVerticalAlignment(SwingConstants.TOP);
		label_7.setHorizontalAlignment(SwingConstants.LEFT);
		label_7.setForeground(Color.WHITE);
		label_7.setFont(new Font(NumFont, Font.BOLD, 12));
		label_7.setBounds(92, 33, 36, 31);
		panel_3.add(label_7);
	}
	
	public void init(controller c,service s){
		int lx=0;
		int ly=0;
	
		xc=c;
		xs=s;
		//System.out.println("stickValue初始化了");
		if(xc.getconfig("GlobalNumFont")!="")NumFont=xc.getconfig("GlobalNumFont");
		else NumFont=app.DefaultNumfontName;
		
		if (xc.getconfig("stickValueX") != "")
			lx = Integer.parseInt(xc.getconfig("stickValueX"));
		else
			lx = 0;
		if (xc.getconfig("stickValueY") != "")
			ly = Integer.parseInt(xc.getconfig("stickValueY"));
		else
			ly = 0;
		
		setFrameOpaque();
	
	
		this.setBounds(lx, ly, 328, 264);
	
		topPanel=new WebPanel();
		topPanel.setWebColoredBackground(false);
		topPanel.setBackground(new Color(0, 0, 0, 0));
		
		initpanel(topPanel);
		add(topPanel);

		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(false);
		setDefaultCloseOperation(3);
		setTitle(language.vTitle);
		setAlwaysOnTop(true);

		setFocusable(false);
		setFocusableWindowState(false);// 取消窗口焦点
		setVisible(true);
		if(xc.getconfig("enableAxisEdge").equals("true"))setShadeWidth(10);
	
	}
	
	
	@Override
	public void run() {
		// TODO Auto-generated method stub
		while(doit){
			try {
				Thread.sleep(80);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			px=100+xs.sState.aileron;
			label_1.setText(xs.aileron);
			py=100+xs.sState.elevator;
			label_3.setText(xs.elevator);
			slider.setValue(xs.sState.rudder);
			label_6.setText(xs.rudder);
			//System.out.println("stickValue执行了");
			this.repaint();
		}
	}
	
	
	
}