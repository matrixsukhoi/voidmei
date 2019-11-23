package ui;
import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Font;
import java.awt.Toolkit;
import java.io.FileNotFoundException;

import javax.swing.SwingConstants;

import com.alee.graphics.image.gif.GifIcon;
import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;

import prog.controller;
import prog.language;

public class statusBar extends WebFrame implements Runnable {
	/**
	 * 
	 */
	private static final long serialVersionUID = 285137206980711202L;
	public volatile boolean doit=true;
	controller xc;
	WebLabel title;
	WebPanel panel;
	int WIDTH;
	int HEIGHT;

	public void init(controller c) {
		xc = c;
		WIDTH = 250;
		HEIGHT = 80;

		//System.out.println("statusBar初始化了");
		//setSize(WIDTH, HEIGHT);
		//setLocation(Toolkit.getDefaultToolkit().getScreenSize().width - WIDTH, 50);
		this.setBounds(Toolkit.getDefaultToolkit().getScreenSize().width - WIDTH, 50,WIDTH, HEIGHT);
		WebPanel panel = new WebPanel();
		panel.setSize(WIDTH, HEIGHT);
		
		
		//ImageIcon I = new ImageIcon(Toolkit.getDefaultToolkit().createImage("image/loader.gif"));
		GifIcon gif=null;
		try {
			
			gif = new GifIcon("image/facebook.gif");
		} catch (FileNotFoundException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		//WebImage webimage1=new WebImage(I);
		//java.awt.Image L=Toolkit.getDefaultToolkit().createImage("image/loader.gif");
		// System.out.println(I);
		// WebDecoratedImage Image1=new WebDecoratedImage(I);
		// TooltipManager.setTooltip ( Image1, "Simple preferred-size image",
		// TooltipWay.up );
		//WebImage webimage1=new WebImage(I);
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明

		this.setUndecorated(true);

		title = new WebLabel("" ,gif/*,I*/);
		title.setHorizontalAlignment(SwingConstants.CENTER);
		title.setDrawShade(true);
		title.setForeground(new Color(245, 248, 250, 240));
		title.setShadeColor(Color.BLACK);
		title.setFont(new Font("Microsoft Yahei UI", Font.PLAIN, 12));
		title.setIconTextGap(3);

		panel.setLayout(new BorderLayout());

		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));

		this.add(panel);

		//panel.add(webimage1,BorderLayout.LINE_START);
		panel.add(title, BorderLayout.CENTER);
		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(false);
		setDefaultCloseOperation(3);
		setTitle(language.sTitle);
		setAlwaysOnTop(true);
		
		setFocusable(false);
		setFocusableWindowState(false);// 取消窗口焦点
		setVisible(true);
		
	}

	public void S1() {
		title.setText(language.sWait);
		repaint();
	}

	public void S2() {
		title.setText(language.sEnter);
		repaint();
	}

	public void S3() {
		title.setText(language.sCheck);
		repaint();
		try {
			Thread.sleep(3000);
		} catch (InterruptedException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		this.dispose();
	}
	@Override
	public void run() {
		while (doit) {
			try {
				Thread.sleep(15);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			//System.out.println("statusBar执行了");
			//System.out.println("刷新了");
			this.repaint();
		}
	}
}