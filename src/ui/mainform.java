package ui;

import static javax.swing.JSplitPane.VERTICAL_SPLIT;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Container;
import java.awt.Font;
import java.awt.Image;
import java.awt.Toolkit;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;

import javax.swing.ImageIcon;
import com.alee.extended.panel.WebButtonGroup;
import com.alee.laf.button.WebButton;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.splitpane.WebSplitPane;
import com.alee.laf.tabbedpane.TabbedPaneStyle;
import com.alee.laf.tabbedpane.WebTabbedPane;
import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent;

import ui.layout.UIBuilder;
import ui.panels.AdvancedPanel;
import ui.panels.FlightInfoPanel;
import ui.panels.EngineInfoPanel;
import ui.panels.EngineControlPanel;
import ui.panels.LoggingPanel;
import ui.panels.MiniHUDPanel;

import prog.app;
import prog.controller;
import prog.lang;

public class mainform extends WebFrame implements Runnable {
	/**
	 * 
	 */
	private static final long serialVersionUID = 5917570099029563038L;
	public volatile Boolean doit = true;
	public int width;
	public int height;
	public controller tc;
	// Store dynamic pages for updates
	private java.util.List<ui.layout.DynamicDataPage> dynamicPages;
	private ui.util.ConfigWatcherService configWatcher;
	int gcCount = 0;
	Container root;
	WebTabbedPane tabbedPane;

	AdvancedPanel advancedPanel;
	FlightInfoPanel flightInfoPanel;
	EngineInfoPanel engineInfoPanel;
	EngineControlPanel engineControlPanel;
	LoggingPanel loggingPanel;
	MiniHUDPanel miniHUDPanel;

	WebButton bDisplayFmKey;

	public Boolean moveCheckFlag;
	public boolean isInitializing = false;

	Color whiteBg = new Color(255, 255, 255, 255);

	public void setFrameOpaque() {
		this.getWebRootPaneUI().setMiddleBg(new Color(255, 255, 255));// 纯白背景以匹配水印
		this.getWebRootPaneUI().setTopBg(new Color(255, 255, 255));
		this.getWebRootPaneUI().setBorderColor(new Color(255, 255, 255, 255));// 内描边透明
		this.getWebRootPaneUI().setInnerBorderColor(new Color(255, 255, 255, 255));// 外描边透明
	}

	// JP1布局
	public WebButton createButton(String text) {
		WebButton a = new WebButton(text);
		a.setShadeWidth(1);
		a.setDrawShade(true);
		// a.getWebUI().setInnerShadeColor(new Color(255,255,255,200));
		// a.getWebUI().setInnerShadeWidth(10);
		a.setFont(app.defaultFontBig);
		a.setTopBgColor(new Color(0, 0, 0, 0));
		a.setBottomBgColor(new Color(0, 0, 0, 0));
		// a.setUndecorated(false);
		// a.setShadeWidth(1);
		a.setBorderPainted(false);

		return a;

	}

	public WebButtonGroup createbuttonGroup() {
		WebButton A = createButton(lang.mCancel);
		WebButton B = createButton(lang.mStart);

		WebButtonGroup G = new WebButtonGroup(true, A, B);
		A.setPreferredWidth(120);
		B.setPreferredWidth(120);
		B.setRound(10);
		G.setButtonsDrawSides(false, false, false, true);
		G.setButtonsForeground(new Color(0, 0, 0, 200));
		G.setButtonsShadeWidth(3);

		A.addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {
				saveConfig();
				tc.saveConfig();
				System.exit(0);
			}
		});
		B.addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {
				confirm();
			}
		});
		return G;
	}

	public WebButton displayPreview;

	public WebButtonGroup createLBGroup(WebPanel bottomPanel) {
		displayPreview = createButton(lang.mDisplayPreview);
		WebButton C = createButton(lang.mClosePreview);
		WebButtonGroup G = new WebButtonGroup(true, displayPreview, C);
		displayPreview.setPreferredWidth(120);

		C.setPreferredWidth(120);
		displayPreview.setFont(app.defaultFontBig);
		C.setFont(app.defaultFontBig);
		G.setButtonsShadeWidth(3);

		// WebLabel lb=createWebLabel("调整位置");
		displayPreview.addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {
				startPreview();
			}
		});
		C.addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {
				stopPreview();
			}
		});
		G.setButtonsDrawSides(false, false, false, true);

		bottomPanel.add(G, BorderLayout.LINE_START);
		return G;
	}

	private void setupTab(WebPanel tab, WebPanel content) {
		UIBuilder.decorateStandardPanel(tab);
		WebPanel topPanel = new WebPanel(new BorderLayout());
		WebPanel bottomPanel = new WebPanel(new BorderLayout());
		UIBuilder.decorateInsidePanel(topPanel);
		UIBuilder.decorateInsidePanel(bottomPanel);

		WebSplitPane splitPane = new WebSplitPane(VERTICAL_SPLIT, topPanel, bottomPanel);
		splitPane.setOpaque(false);
		splitPane.setBackground(new Color(0, 0, 0, 0));
		splitPane.setDividerLocation(320);
		splitPane.setDividerSize(0);
		splitPane.setContinuousLayout(false);
		splitPane.setDrawDividerBorder(false);
		splitPane.setOneTouchExpandable(false);
		splitPane.setEnabled(false);

		topPanel.add(content, BorderLayout.CENTER);

		bottomPanel.add(createbuttonGroup(), BorderLayout.LINE_END);
		bottomPanel.add(createLBGroup(bottomPanel), BorderLayout.LINE_START);

		tab.add(splitPane);
	}

	public void initJP1(WebPanel jp1) {
		advancedPanel = new AdvancedPanel();
		advancedPanel.setOnChange(() -> {
			if (isInitializing)
				return;
			saveConfig();
			tc.refreshPreviews();
		});
		setupTab(jp1, advancedPanel);
	}

	public void initJP2(WebPanel jp2) {
		engineInfoPanel = new EngineInfoPanel();
		engineInfoPanel.setOnChange(() -> {
			saveConfig();
			if (!isInitializing) {
				tc.refreshPreviews();
			}
		});
		setupTab(jp2, engineInfoPanel);
	}

	public void initJP3(WebPanel jp3) {
		miniHUDPanel = new MiniHUDPanel(this);
		miniHUDPanel.setOnChange(() -> {
			saveConfig();
			if (!isInitializing) {
				tc.refreshPreviews();
			}
		});
		miniHUDPanel.setOnSave(() -> saveConfig());
		setupTab(jp3, miniHUDPanel);
	}

	public void initJP4(WebPanel jp4) {
		flightInfoPanel = new FlightInfoPanel();
		flightInfoPanel.setOnChange(() -> {
			if (isInitializing)
				return;
			saveConfig();
			tc.refreshPreviews();
		});
		// 同步“飞行信息”页面的开关状态到“记录分析”页面
		flightInfoPanel.bFMPrintSwitch.addActionListener(e -> {
			if (loggingPanel != null && loggingPanel.bFMPrintLogSwitch != null
					&& loggingPanel.bFMPrintLogSwitch.isSelected() != flightInfoPanel.bFMPrintSwitch.isSelected())
				loggingPanel.bFMPrintLogSwitch.setSelected(flightInfoPanel.bFMPrintSwitch.isSelected());
		});
		setupTab(jp4, flightInfoPanel);
	}

	public void initJP5(WebPanel jp5) {
		loggingPanel = new LoggingPanel(this);
		loggingPanel.setOnChange(() -> {
			saveConfig();
			if (!isInitializing) {
				tc.refreshPreviews();
			}
		});
		loggingPanel.setOnSave(() -> saveConfig());
		// Synchronization with FlightInfoPanel
		loggingPanel.bFMPrintLogSwitch.addActionListener(e -> {
			if (flightInfoPanel != null && flightInfoPanel.bFMPrintSwitch != null
					&& flightInfoPanel.bFMPrintSwitch.isSelected() != loggingPanel.bFMPrintLogSwitch.isSelected())
				flightInfoPanel.bFMPrintSwitch.setSelected(loggingPanel.bFMPrintLogSwitch.isSelected());
		});
		setupTab(jp5, loggingPanel);
	}

	public void initJP6(WebPanel jp6) {
		engineControlPanel = new EngineControlPanel();
		engineControlPanel.setOnChange(() -> {
			saveConfig();
			if (!isInitializing) {
				tc.refreshPreviews();
			}
		});
		setupTab(jp6, engineControlPanel);
	}

	public void initPanel() {
		tabbedPane = new WebTabbedPane();
		tabbedPane.setTabPlacement(WebTabbedPane.LEFT);
		// tabbedPane.setTabLayoutPolicy(JTabbedPane.SCROLL_TAB_LAYOUT);

		WebPanel jp1 = new WebPanel();
		WebPanel jp2 = new WebPanel();
		WebPanel jp3 = new WebPanel();
		WebPanel jp4 = new WebPanel();
		WebPanel jp5 = new WebPanel();
		WebPanel jp6 = new WebPanel();
		initJP1(jp1);
		initJP2(jp2);
		initJP3(jp3);
		initJP4(jp4);
		initJP5(jp5);
		initJP6(jp6);

		tabbedPane.addTab(lang.mFlightInfo, jp4);
		tabbedPane.addTab(lang.mEngineInfo, jp2);
		tabbedPane.addTab(lang.mControlInfo, jp6);
		tabbedPane.addTab(lang.mLoggingAndAnalysis, jp5);
		tabbedPane.addTab(lang.mCrosshair, jp3);
		tabbedPane.addTab(lang.mAdvancedOption, jp1);

		// Dynamic Tabs from Config - use already initialized configs from controller
		dynamicPages = new java.util.ArrayList<>();
		if (tc.dynamicConfigs.isEmpty()) {
			// Fallback if no config or empty
			ui.layout.UIBuilder.addRightAlignedTab(tabbedPane, "Data (Empty)", new ui.layout.DynamicDataPage(this),
					app.defaultFontBig);
		} else {
			for (ui.util.ConfigLoader.GroupConfig group : tc.dynamicConfigs) {
				ui.layout.DynamicDataPage page = new ui.layout.DynamicDataPage(this, group);
				dynamicPages.add(page);
				ui.layout.UIBuilder.addRightAlignedTab(tabbedPane, group.title, page, app.defaultFontBig);
			}
		}
		// tabbedPane.setTabBorderColor(new Color(0, 0, 0, 0));
		// tabbedPane.setContentBorderColor(new Color(0, 0, 0, 0));
		// tabbedPane.setShadeWidth(1);
		// tabbedPane.setSelectedBottomBg(new Color(0, 0, 0,20));
		// tabbedPane.setSelectedTopBg(new Color(0, 0, 0, 20));
		// tabbedPane.setBackgroundAt(1, new Color(0, 0, 0, 0));

		// tabbedPane.setSelectedForegroundAt(0,(new Color(0, 0, 0, 0));
		tabbedPane.setSelectedIndex(0);

		tabbedPane.setOpaque(false);
		tabbedPane.setBackground(new Color(0, 0, 0, 0));
		// tabbedPane.getWebUI().setBackgroundColor(new Color(0,0,0,0));
		tabbedPane.setFont(app.defaultFontBig);
		tabbedPane.setPaintOnlyTopBorder(true);
		tabbedPane.setPaintBorderOnlyOnSelectedTab(true);
		tabbedPane.setTabbedPaneStyle(TabbedPaneStyle.attached);
		this.add(tabbedPane);

		tabbedPane.addChangeListener(e -> {
			updateDynamicSize();
		});

		startFileWatcher();
	}

	private void startFileWatcher() {
		configWatcher = new ui.util.ConfigWatcherService("ui_layout.cfg", this::reloadDynamicConfig);
		configWatcher.start(2000);
	}

	private void reloadDynamicConfig() {
		// Re-initialize controller's overlays and configs from disk
		tc.initDynamicOverlays();

		if (dynamicPages == null || tc.dynamicConfigs.isEmpty())
			return;

		// Sync existing pages with the new GroupConfig objects from controller
		for (ui.util.ConfigLoader.GroupConfig group : tc.dynamicConfigs) {
			for (ui.layout.DynamicDataPage page : dynamicPages) {
				if (page.getGroupConfig().title.equals(group.title)) {
					page.setGroupConfig(group);
					break;
				}
			}
		}
	}

	public void updateDynamicSize() {
		if (isInitializing || tabbedPane == null)
			return;
		java.awt.Component selected = tabbedPane.getSelectedComponent();
		int targetWidth = width - 30; // weblaf好像会加15px+15px==30px的边框, 所以要减去30px.
		if (selected instanceof ui.layout.DynamicDataPage) {
			ui.layout.DynamicDataPage page = (ui.layout.DynamicDataPage) selected;
			int reqH = page.getRequiredHeight();
			if (reqH > 480) {
				setSize(targetWidth, reqH);
			} else {
				setSize(targetWidth, 480);
			}
		} else {
			if (getHeight() != 480) {
				setSize(targetWidth, 480);
			}
		}

		this.getRootPane().revalidate();
		this.getRootPane().repaint();
		validate();
		repaint();
	}

	public void loadConfig() {
		flightInfoPanel.loadConfig(tc.configService);
		engineInfoPanel.loadConfig(tc.configService);
		miniHUDPanel.loadConfig(tc.configService);
		advancedPanel.loadConfig(tc.configService);
		loggingPanel.loadConfig(tc.configService);
		engineControlPanel.loadConfig(tc.configService);
	}

	public void initDefaults() {
		FlightInfoPanel.initDefaults(tc.configService);
		MiniHUDPanel.initDefaults(tc.configService);
		LoggingPanel.initDefaults(tc.configService);
		AdvancedPanel.initDefaults(tc.configService);

		// Special case that needs instance
		tc.setconfig("crosshairName", miniHUDPanel.sCrosshairName.getSelectedItem().toString());
	}

	public void saveConfig() {
		if (isInitializing)
			return;

		flightInfoPanel.saveConfig(tc.configService);
		advancedPanel.saveConfig(tc.configService);
		engineInfoPanel.saveConfig(tc.configService);
		engineControlPanel.saveConfig(tc.configService);
		loggingPanel.saveConfig(tc.configService);
		miniHUDPanel.saveConfig(tc.configService);

		tc.setconfig("displayFmKey", Integer.toString(app.displayFmKey));

	}

	public void confirm() {
		tc.endPreview();
		moveCheckFlag = false;
		saveConfig();
		tc.saveConfig();
		tc.loadFromConfig();

		this.setVisible(false);
		tc.state = prog.ControllerState.INIT;
		tc.start();
		// Show dynamic overlays for game mode, respecting hotkey hidden-at-start rule
		tc.setDynamicOverlaysVisible(true, true);
		// this.dispose();
	}

	@Override
	public void dispose() {
		if (dynamicPages != null) {
			for (ui.layout.DynamicDataPage page : dynamicPages) {
				page.dispose();
			}
		}
		super.dispose();
	}

	public mainform(controller c) {
		// System.setProperty("awt.useSystemAAFontSettings", "on");
		// app.debugPrint("mainForm初始化了");
		width = 800;
		height = 480;
		doit = true;
		Image I = Toolkit.getDefaultToolkit().getImage("image/form1.png");
		I = I.getScaledInstance(32, 32, Image.SCALE_SMOOTH);
		this.setIconImage(I);

		tc = c;
		moveCheckFlag = false;

		this.setUndecorated(true);
		this.setLocation(app.screenWidth / 2 - width / 2, app.screenHeight / 2 - height / 2);
		this.setFont(app.defaultFont);
		this.setSize(width, height);

		// setFrameOpaque();// 窗口透明
		String ti = app.appName + " v" + app.version;
		if (app.debug)
			ti = ti + "beta";
		// ti=ti+" ————"+app.appTooltips;
		setTitle(ti);
		this.setShowMaximizeButton(false);
		this.getWebRootPaneUI().getTitleComponent().getComponent(1)
				.setFont(new Font(app.defaultFont.getName(), Font.PLAIN, 14));// 设置title字体
		// this.getWebRootPaneUI().getWindowButtons().setButtonsInnerShadeWidth(0);
		// this.getWebRootPaneUI().getWindowButtons().setButtonsShadeWidth(0);
		this.getWebRootPaneUI().getWindowButtons().setBorderColor(new Color(0, 0, 0, 0));
		// this.getWebRootPaneUI().getWindowButtons().setButtonsDrawBottom(false);
		this.getWebRootPaneUI().getWindowButtons().setButtonsDrawSides(false, false, false, false);
		this.getWebRootPaneUI().getWindowButtons().getWebButton(0).setTopBgColor(new Color(0, 0, 0, 0));
		this.getWebRootPaneUI().getWindowButtons().getWebButton(0).setBottomBgColor(new Color(0, 0, 0, 0));
		this.getWebRootPaneUI().getWindowButtons().getWebButton(1).setTopBgColor(new Color(0, 0, 0, 0));
		this.getWebRootPaneUI().getWindowButtons().getWebButton(1).setBottomBgColor(new Color(0, 0, 0, 0));

		root = this.getContentPane();
		if (root instanceof javax.swing.JComponent) {
			((javax.swing.JComponent) root).setOpaque(false);
		}
		root.setBackground(Color.WHITE);
		setFrameOpaque();

		setDrawWatermark(true);
		setWatermark(new ImageIcon("image/watermark.png"));

		// this.getTitleComponent().setForeground(new Color(0,0,0,255));
		// this.getWebRootPaneUI().getWindowButtons().getWebButton(2).setBorderPainted(false);
		isInitializing = true;
		initPanel();
		loadConfig();// 读入Config
		try {
			String keyStr = tc.getconfig("displayFmKey");
			if (keyStr != null && !keyStr.isEmpty() && !keyStr.equals("null")) {
				app.displayFmKey = Integer.parseInt(keyStr);
				loggingPanel.bDisplayFmKey.setText(NativeKeyEvent.getKeyText(app.displayFmKey));
			}
		} catch (Exception e) {
		}

		isInitializing = false;

		setShowResizeCorner(false);
		setDefaultCloseOperation(3);
		setVisible(true);
		this.setShadeWidth(10);
		tc.Preview();
		moveCheckFlag = true;
		// Ensure overlays are visible if we started in preview mode
		tc.setDynamicOverlaysVisible(true, false);

	}

	@Override
	public void run() {
		// TODO Auto-generated method stub
		while (doit) {
			try {
				Thread.sleep(33);
			} catch (InterruptedException e) {
				// TODO Auto-generated catch block
				e.printStackTrace();
			}
			// gcCount++;
			// app.debugPrint("As");
			// app.debugPrint("MainFrame执行了");
			// Update dynamic data pages - now handled by uiThread or controller?
			// But for preview mode, we might still want to update them if they are visible.
			// Since controller.dynamicOverlays are always Updated by uiThread now (if
			// isVisible),
			// we don't need to manually update pages here.
			root.repaint();
			if (gcCount++ % 256 == 0) {
				// app.debugPrint("MainFrameGC");
				System.gc();
			}

		}

	}

	public void startPreview() {
		if (moveCheckFlag == null || moveCheckFlag == false) {
			ui.util.NotificationService.show(prog.lang.mMovePanel);
			saveConfig();
			tc.Preview();
			moveCheckFlag = true;

			// Show dynamic overlays for preview
			tc.setDynamicOverlaysVisible(true, false); // In preview, show all regardless of hotkey
		}
	}

	public void stopPreview() {
		if (moveCheckFlag != null && moveCheckFlag) {
			tc.endPreview();
			moveCheckFlag = false;

			// Hide dynamic overlays
			tc.setDynamicOverlaysVisible(false, false);
		}
	}

	public void saveDynamicConfig() {
		if (dynamicPages == null)
			return;
		if (configWatcher != null)
			configWatcher.ignoreNext(); // Prevents reloading the file we just wrote
		java.util.List<ui.util.ConfigLoader.GroupConfig> configs = new java.util.ArrayList<>();
		for (ui.layout.DynamicDataPage page : dynamicPages) {
			if (page.getGroupConfig() != null) {
				configs.add(page.getGroupConfig());
			}
		}
		ui.util.ConfigLoader.saveConfig("ui_layout.cfg", configs);
	}
}