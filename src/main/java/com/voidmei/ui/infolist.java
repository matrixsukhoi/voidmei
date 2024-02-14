package com.voidmei.ui;
import java.awt.Color;
import java.awt.Font;
import java.awt.Graphics2D;
import java.awt.GridLayout;
import java.awt.Image;
import java.awt.Toolkit;
import javax.swing.SwingConstants;

import com.alee.extended.label.WebMultiLineLabel;
import com.alee.extended.panel.WebComponentPanel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.voidmei.controller;
import com.voidmei.service;

public class infolist extends WebFrame implements Runnable {
	/**
	 * 
	 */
	private static final long serialVersionUID = 6827339811420945540L;
	public controller c;
	public service s;
	public String status;
	Graphics2D h;
	Image buffer;
	long MainCheckMili;
	int columns;
	int rows;
	int WIDTH;
	int HEIGHT;
	int lx;
	int ly;
	int OP;
	WebPanel panel;
	WebComponentPanel webComponentPanel;

	WebMultiLineLabel xm;
	WebMultiLineLabel data;
	WebMultiLineLabel dw;

	public void init(controller xc, service ts) {
		this.c = xc;
		this.s = ts;


		Toolkit.getDefaultToolkit();
		lx = 0;
		ly = 0;
		WIDTH = 400;
		HEIGHT = 800;
		OP = 100;
		setSize(WIDTH, HEIGHT);
		WebPanel panel = new WebPanel();


		
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明
		
		this.setUndecorated(true);

		//布局
		panel.setLayout(new GridLayout(1,3));
		panel.setMargin(20);
		
        //final TableLayout groupLayout = new TableLayout ( new double[][]{ columns, rows } );
       //groupLayout.setHGap ( 4 );
        //groupLayout.setVGap ( 4 );
        //panel.setLayout ( groupLayout );
        
		//标签
		xm = new WebMultiLineLabel("");
		xm.setDrawShade(true);
		xm.setForeground(new Color(245, 248, 250, 240));
		xm.setShadeColor(Color.BLACK);
		xm.setFont(new Font("Microsoft Yahei UI",Font.PLAIN,20));
		xm.setHorizontalAlignment(SwingConstants.LEFT);;
		xm.setVerticalAlignment(SwingConstants.TOP);
		
		data = new WebMultiLineLabel("");
		data.setDrawShade(true);
		data.setForeground(new Color(245, 248, 250, 240));
		data.setShadeColor(Color.BLACK);
		data.setFont(new Font("Microsoft Yahei UI",Font.PLAIN,20));
		data.setHorizontalAlignment(SwingConstants.RIGHT);;
		data.setVerticalAlignment(SwingConstants.TOP);
		
		data = new WebMultiLineLabel("");
		data.setDrawShade(true);
		data.setForeground(new Color(245, 248, 250, 240));
		data.setShadeColor(Color.BLACK);
		data.setFont(new Font("Microsoft Yahei UI",Font.PLAIN,20));
		data.setHorizontalAlignment(SwingConstants.RIGHT);;
		data.setVerticalAlignment(SwingConstants.TOP);
	
		
		
		//panel透明
		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));
		// panel.setOpaque(false);
		
		//添加绘制物件至panel
		panel.add(xm);
		panel.add(data);
		

		
		
		add(panel);
		this.setOpacity((float) (OP / 100.0f));
		//setFocusableWindowState(false);//取消窗口焦点
		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(true);
		setAlwaysOnTop(true);
		setVisible(true);
		setDefaultCloseOperation(3);

	}

	public void update() {

	}

	public void run() {
		while (true) {
			try {
				
				xm.setText("表　速\n" +"真空速\n");
				data.setText(s.sState.IAS+"\n"+s.sState.TAS+"\n");
				repaint();
				Thread.sleep(50);
				// this.status="infolist就绪";
				// App.debugPrint(this.status);
				// 绘图

				// this.status="infolist等待";
				// App.debugPrint(this.status);

			} catch (Exception e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
		}
	}
}