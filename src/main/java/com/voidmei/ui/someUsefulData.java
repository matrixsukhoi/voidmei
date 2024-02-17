package com.voidmei.ui;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Font;
import java.awt.Toolkit;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;
import java.io.IOException;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.text.WebTextArea;
import com.github.kwhat.jnativehook.GlobalScreen;
import com.github.kwhat.jnativehook.NativeHookException;
import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent;
import com.github.kwhat.jnativehook.keyboard.NativeKeyListener;
import com.voidmei.App;
import com.voidmei.controller;
import com.voidmei.parser.blkx;

public class someUsefulData extends WebFrame implements Runnable {
	/**
	 * 
	 */
	private static final long serialVersionUID = 285137206980711202L;
	public volatile boolean doit = true;
	controller xc;
	blkx xp;
	WebLabel title;
	WebPanel panel;
	int WIDTH;
	int HEIGHT;
	WebTextArea textArea;
	String FontName;
	Boolean displayp = true;

	public void init(controller c, blkx p) {
		xc = c;
		xp = p;
		WIDTH = App.defaultFontsize * 32;
		HEIGHT = App.defaultFontsize * 72;

		// App.debugPrint("statusBar初始化了");
		// setSize(WIDTH, HEIGHT);
		// setLocation(Toolkit.getDefaultToolkit().getScreenSize().width -
		// WIDTH, 50);
		this.setBounds(Toolkit.getDefaultToolkit().getScreenSize().width - WIDTH, Toolkit.getDefaultToolkit().getScreenSize().height -10 - HEIGHT, WIDTH,  HEIGHT);
		WebPanel panel = new WebPanel();
		panel.setSize(WIDTH, HEIGHT);

		// WebImage webimage1=new WebImage(I);
		// java.awt.Image
		// L=Toolkit.getDefaultToolkit().createImage("image/loader.gif");
		// App.debugPrint(I);
		// WebDecoratedImage Image1=new WebDecoratedImage(I);
		// TooltipManager.setTooltip ( Image1, "Simple preferred-size image",
		// TooltipWay.up );
		// WebImage webimage1=new WebImage(I);
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明

		this.setUndecorated(true);

		if (xc.getconfig("flightInfoFontC") != "")
			FontName = xc.getconfig("flightInfoFontC");
		Font f;
		if (!FontName.isEmpty()) {
//			App.debugPrint(FontName);
			f = new Font(FontName, Font.PLAIN, 13);
		} else {
			f = App.defaultFont;
		}

		textArea = new WebTextArea();
//		if (f.equals(null)) {
//			App.debugPrint("error " + "FontName");
//		}

		textArea.setFont(f);
		
		textArea.setBackground(new Color(250, 250, 250, 85));
		
//		 title = new WebLabel(""/*,I*/);
////		 title.setHorizontalAlignment(SwingConstants.TOP);
//		 title.setDrawShade(true);
////		 title.setForeground(new Color(245, 248, 250, 240));
//		 title.setShadeColor(Color.BLACK);
//		 title.setText("----------FM读取--------");
//		 title.setFont(f);
//		 title.setIconTextGap(3);

		this.setShadeWidth(0);
//		MouseListener[] mls = textArea.getMouseListeners();
//		MouseMotionListener[] mmls = textArea.getMouseMotionListeners();
//		for (int t = 0; t < mls.length; t++) {
//			textArea.removeMouseListener(mls[t]);
//
//		}
//		for (int t = 0; t < mmls.length; t++) {
//			textArea.removeMouseMotionListener(mmls[t]);
//		}
		
		this.getRootPane().setCursor(App.blankCursor);
		this.getContentPane().setCursor(App.blankCursor);
//		this.getFocusableChilds();
		this.getGlassPane().setCursor(App.blankCursor);
		
		addMouseListener(new MouseAdapter() {
			@Override
			public void mouseEntered(MouseEvent e) {
				//App.debugPrint(e);
			}
			@Override
			public void mousePressed(MouseEvent e) {
				//App.debugPrint(e);

			}
			@Override
			public void mouseReleased(MouseEvent e) {
				//App.debugPrint(e);
			}

		});
		
		addMouseMotionListener(new MouseMotionAdapter() {
			@Override
			public void mouseDragged(MouseEvent e) {
				//App.debugPrint(e);
			}
			@Override
			public void mouseMoved(MouseEvent e) {
				//App.debugPrint(e);
			}
		});

		this.getLayeredPane().setCursor(App.blankCursor);
		textArea.setCursor(App.blankCursor);
		panel.setCursor(App.blankCursor);
		
		panel.setLayout(new BorderLayout());

		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));
		
		this.add(panel);

		// panel.add(webimage1,BorderLayout.LINE_START);
//		mls = panel.getMouseListeners();
//		mmls = panel.getMouseMotionListeners();
//		for (int t = 0; t < mls.length; t++) {
//			panel.removeMouseListener(mls[t]);
//
//		}
//		for (int t = 0; t < mmls.length; t++) {
//			panel.removeMouseMotionListener(mmls[t]);
//		}
		
//		panel.add(title);
		panel.add(textArea);
//		panel.add(title);
		uiWebLafSetting.setWindowFocus(this);

		//		this.getWindows()[0].toBack();
		
		try {
			GlobalScreen.registerNativeHook();
		}
		catch (NativeHookException ex) {
			App.debugPrint("There was a problem registering the native hook.");
			App.debugPrint(ex.getMessage());
		}

		GlobalScreen.addNativeKeyListener(new NativeKeyListener() {
			public void nativeKeyReleased(NativeKeyEvent e) {
				if (e.getKeyCode() == NativeKeyEvent.VC_P) {
					displayp = !displayp;
					// System.out.println("Key Released: " + NativeKeyEvent.getKeyText(e.getKeyCode()));
					// System.out.println("" + displayp);
				}
			}
		});
	}

	public void S() {
		title.setText(xp.fmdata);
		repaint();
		try {
			Thread.sleep(10000);
		} catch (InterruptedException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}
		this.dispose();
	}

	@Override
	public void run() {
		// while (doit) {
		// try {
		// Thread.sleep(100);
		// } catch (InterruptedException e) {
		// // TODO Auto-generated catch block
		// e.printStackTrace();
		// }
		// //App.debugPrint("statusBar执行了");
		// //App.debugPrint("刷新了");
		// this.repaint();
		// }
		while (doit) {
			if (displayp) {
				textArea.setText(xp.fmdata);
				textArea.setBackground(new Color(250, 250, 250, 85));
			} else {
				textArea.setText("");
				textArea.setBackground(new Color(0, 0, 0, 0));
			}

			this.getContentPane().repaint();
			try {
				Thread.sleep(500);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}	
			if (this.xc.S.sState.gear != 100 || (this.xc.S.speedv > 10 && this.xc.S.sState.throttle > 0)) {
				// 如果收起落架则关闭break
				/* 测试，开始传送 */
				// if (App.fmTesting){
				// 	/* 先上10000米 */
				// 	try {
				// 		xc.S.httpClient.fmCmdSetAlt(10000, App.requestDest);
				// 		xc.S.httpClient.fmCmdSetSpd(100.0f, App.requestDest);
				// 	} catch (IOException e) {
				// 		// TODO Auto-generated catch block
				// 		e.printStackTrace();
				// 	}
				// }
				try {
					Thread.sleep(10000);
				} catch (InterruptedException e) {
					// TODO Auto-generated catch block
					e.printStackTrace();
				}	
				break;
			}
//			else{
//			}
		}
		this.dispose();
	}
}