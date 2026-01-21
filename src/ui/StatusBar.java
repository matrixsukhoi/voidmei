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

import prog.Application;
import prog.Controller;
import prog.i18n.Lang;
import prog.Service;

public class StatusBar extends WebFrame implements Runnable {
	/**
	 * 
	 */
	private static final long serialVersionUID = 285137206980711202L;
	public volatile boolean doit = true;
	private prog.config.OverlaySettings settings;
	Controller xc;
	Service xs;
	WebLabel statusLabel;
	WebPanel panel;
	int WIDTH;
	int HEIGHT;

	public void init(Controller c, prog.config.OverlaySettings settings) {
		xc = c;
		this.settings = settings;
		WIDTH = 250;
		HEIGHT = 80;

		this.setUndecorated(true);

		int initialX = Toolkit.getDefaultToolkit().getScreenSize().width - WIDTH;
		int initialY = 50;

		if (settings != null) {
			initialX = settings.getWindowX(WIDTH);
			initialY = settings.getWindowY(HEIGHT);
		}

		this.setBounds(initialX, initialY, WIDTH, HEIGHT);
		WebPanel panel = new WebPanel();
		panel.setSize(WIDTH, HEIGHT);

		// ImageIcon I = new
		// ImageIcon(Toolkit.getDefaultToolkit().createImage("image/loader.gif"));
		GifIcon gif = null;
		try {

			gif = new GifIcon("image/facebook.gif");
		} catch (FileNotFoundException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
		}

		String FontName = "";
		if (settings != null) {
			FontName = settings.getString("flightInfoFontC", "");
		}

		Font f;
		if (FontName != null && !FontName.isEmpty()) {
			// Application.debugPrint(FontName);
			f = new Font(FontName, Font.PLAIN, 14);
		} else {
			f = Application.defaultFont;
		}

		// WebImage webimage1=new WebImage(I);
		// java.awt.Image L=Toolkit.getDefaultToolkit().createImage("image/loader.gif");
		// Application.debugPrint(I);
		// WebDecoratedImage Image1=new WebDecoratedImage(I);
		// TooltipManager.setTooltip ( Image1, "Simple preferred-size image",
		// TooltipWay.up );
		// WebImage webimage1=new WebImage(I);
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明

		this.setUndecorated(true);

		statusLabel = new WebLabel("", gif/* ,I */);
		statusLabel.setHorizontalAlignment(SwingConstants.CENTER);
		statusLabel.setDrawShade(true);
		statusLabel.setForeground(new Color(245, 248, 250, 240));
		statusLabel.setShadeColor(Application.colorShadeShape);
		statusLabel.setFont(f);
		statusLabel.setIconTextGap(3);

		panel.setLayout(new BorderLayout());

		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));

		this.add(panel);

		// panel.add(webimage1,BorderLayout.LINE_START);
		panel.add(statusLabel, BorderLayout.CENTER);
		setTitle("StatusBar");
		setShowWindowButtons(false);
		setShowTitleComponent(false);
		setShowResizeCorner(false);
		setDefaultCloseOperation(3);
		setTitle(Lang.sTitle);
		setAlwaysOnTop(true);

		setFocusable(false);
		setFocusableWindowState(false);// 取消窗口焦点
		setVisible(true);

	}

	public void S1() {
		statusLabel.setText(Lang.sWait);
		repaint();
	}

	public void S2() {
		statusLabel.setText(Lang.sEnter);
		repaint();
	}

	public void S3() {
		statusLabel.setText(Lang.sCheck);
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
				Thread.sleep(100);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			// Application.debugPrint("StatusBar执行了");
			// Application.debugPrint("刷新了");
			this.repaint();
		}
	}
}