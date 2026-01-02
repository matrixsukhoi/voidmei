package ui;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Font;
import java.awt.Toolkit;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseMotionAdapter;

import com.alee.laf.label.WebLabel;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.extended.layout.VerticalFlowLayout;

import parser.blkx;
import prog.app;
import prog.controller;

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
	WebPanel dataPanel;
	String FontName;
	Font displayFont;
	String lastFmData = "";
	int fontSize = 16;
	float scaleFactor = 1.0f;

	public void init(controller c, blkx p) {
		xc = c;
		xp = p;

		int screenHeight = Toolkit.getDefaultToolkit().getScreenSize().height;
		scaleFactor = (float) screenHeight / 1440.0f;

		fontSize = Math.round(16 * scaleFactor);
		WIDTH = Math.round(app.defaultFontsize * 36 * scaleFactor);
		HEIGHT = app.defaultFontsize * 72;

		// app.debugPrint("statusBar初始化了");
		// setSize(WIDTH, HEIGHT);
		// setLocation(Toolkit.getDefaultToolkit().getScreenSize().width -
		// WIDTH, 50);
		this.setBounds(Toolkit.getDefaultToolkit().getScreenSize().width - WIDTH,
				Toolkit.getDefaultToolkit().getScreenSize().height - 10 - HEIGHT, WIDTH, HEIGHT);
		WebPanel panel = new WebPanel();
		panel.setSize(WIDTH, HEIGHT);

		// WebImage webimage1=new WebImage(I);
		// java.awt.Image
		// L=Toolkit.getDefaultToolkit().createImage("image/loader.gif");
		// app.debugPrint(I);
		// WebDecoratedImage Image1=new WebDecoratedImage(I);
		// TooltipManager.setTooltip ( Image1, "Simple preferred-size image",
		// TooltipWay.up );
		// WebImage webimage1=new WebImage(I);
		this.getWebRootPaneUI().setMiddleBg(new Color(0, 0, 0, 0));// 中部透明
		this.getWebRootPaneUI().setTopBg(new Color(0, 0, 0, 0));// 顶部透明
		this.getWebRootPaneUI().setBorderColor(new Color(0, 0, 0, 0));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(0, 0, 0, 0));// 外描边透明

		this.setUndecorated(true);

		if (xc.getconfig("MonoNumFont") != "")
			FontName = xc.getconfig("MonoNumFont");
		else if (xc.getconfig("flightInfoFontC") != "")
			FontName = xc.getconfig("flightInfoFontC");

		Font f;
		if (!FontName.isEmpty()) {
			f = new Font(FontName, Font.PLAIN, fontSize);
		} else {
			f = new Font(app.defaultNumfontName, Font.PLAIN, fontSize);
		}

		dataPanel = new WebPanel();
		dataPanel.setLayout(new VerticalFlowLayout(0, 0));
		dataPanel.setBackground(new Color(20, 20, 20, 180));

		displayFont = f;

		// title = new WebLabel(""/*,I*/);
		//// title.setHorizontalAlignment(SwingConstants.TOP);
		// title.setDrawShade(true);
		//// title.setForeground(new Color(245, 248, 250, 240));
		// title.setShadeColor(Color.BLACK);
		// title.setText("----------FM读取--------");
		// title.setFont(f);
		// title.setIconTextGap(3);

		this.setShadeWidth(0);
		// MouseListener[] mls = textArea.getMouseListeners();
		// MouseMotionListener[] mmls = textArea.getMouseMotionListeners();
		// for (int t = 0; t < mls.length; t++) {
		// textArea.removeMouseListener(mls[t]);
		//
		// }
		// for (int t = 0; t < mmls.length; t++) {
		// textArea.removeMouseMotionListener(mmls[t]);
		// }

		this.getRootPane().setCursor(app.blankCursor);
		this.getContentPane().setCursor(app.blankCursor);
		// this.getFocusableChilds();
		this.getGlassPane().setCursor(app.blankCursor);

		addMouseListener(new MouseAdapter() {
			@Override
			public void mouseEntered(MouseEvent e) {
				// app.debugPrint(e);
			}

			@Override
			public void mousePressed(MouseEvent e) {
				// app.debugPrint(e);

			}

			@Override
			public void mouseReleased(MouseEvent e) {
				// app.debugPrint(e);
			}

		});

		addMouseMotionListener(new MouseMotionAdapter() {
			@Override
			public void mouseDragged(MouseEvent e) {
				// app.debugPrint(e);
			}

			@Override
			public void mouseMoved(MouseEvent e) {
				// app.debugPrint(e);
			}
		});

		this.getLayeredPane().setCursor(app.blankCursor);
		dataPanel.setCursor(app.blankCursor);
		panel.setCursor(app.blankCursor);

		panel.setLayout(new BorderLayout());

		panel.setWebColoredBackground(false);
		panel.setBackground(new Color(0, 0, 0, 0));

		this.add(panel);

		// panel.add(webimage1,BorderLayout.LINE_START);
		// mls = panel.getMouseListeners();
		// mmls = panel.getMouseMotionListeners();
		// for (int t = 0; t < mls.length; t++) {
		// panel.removeMouseListener(mls[t]);
		//
		// }
		// for (int t = 0; t < mmls.length; t++) {
		// panel.removeMouseMotionListener(mmls[t]);
		// }

		// panel.add(title);
		panel.add(dataPanel);
		// panel.add(title);
		uiWebLafSetting.setWindowFocus(this);

		// this.getWindows()[0].toBack();

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
		while (doit) {
			String currentText = xp.fmdata;

			if (app.displayFm) {
				// 检查数据是否变化
				if (!currentText.equals(lastFmData)) {
					lastFmData = currentText;
					dataPanel.removeAll();
					String[] lines = currentText.split("\n");
					for (int i = 0; i < lines.length; i++) {
						WebLabel label = new WebLabel(lines[i]);
						label.setFont(displayFont);
						label.setForeground(Color.WHITE);
						label.setOpaque(true);
						label.setMargin(2, 6, 2, 6);
						// 颜色逻辑：标题高亮 > 斑马纹
						if (lines[i].contains("fm器件") || lines[i].contains("FM文件")) {
							// 标题高亮色：深琥珀色
							label.setBackground(new Color(80, 60, 0, 180));
						} else if (i % 2 == 0) {
							// 斑马纹深灰
							label.setBackground(new Color(25, 25, 25, 180));
						} else {
							// 斑马纹浅灰
							label.setBackground(new Color(40, 40, 40, 180));
						}
						dataPanel.add(label);
					}
					dataPanel.revalidate();
				}

				// 动态调整高度
				int preferredHeight = dataPanel.getPreferredSize().height;
				// 限制最大高度，避免超出屏幕
				int maxHeight = Toolkit.getDefaultToolkit().getScreenSize().height - 40;
				if (preferredHeight > maxHeight)
					preferredHeight = maxHeight;

				// 如果高度变化，更新窗口位置和大小，保持底部对齐
				if (Math.abs(this.getHeight() - preferredHeight) > 2) {
					int screenHeight = Toolkit.getDefaultToolkit().getScreenSize().height;
					int screenWidth = Toolkit.getDefaultToolkit().getScreenSize().width;
					this.setBounds(screenWidth - WIDTH, screenHeight - 10 - preferredHeight, WIDTH, preferredHeight);
				}

				this.setVisible(true);
				this.getContentPane().repaint();
			} else {
				this.setVisible(false);
			}
			if (app.displayFmCtrl) {
				try {
					Thread.sleep(1000);
				} catch (InterruptedException e) {
					e.printStackTrace();
				}
			} else {
				if (this.xc.S.sState.gear != 100 || (this.xc.S.speedv > 10 && this.xc.S.sState.throttle > 0)) {
					try {
						Thread.sleep(10000);
					} catch (InterruptedException e) {
						e.printStackTrace();
					}
					break;
				}
				// 正常刷新间隔
				try {
					Thread.sleep(200);
				} catch (InterruptedException e) {
					e.printStackTrace();
				}
			}
		}
		this.dispose();
	}
}