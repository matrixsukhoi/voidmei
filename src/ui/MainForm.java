package ui;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Container;
import java.awt.Font;
import java.awt.Image;
import java.awt.Toolkit;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;

import javax.swing.ImageIcon;
import javax.swing.SwingUtilities;
import com.alee.extended.panel.WebButtonGroup;
import com.alee.laf.button.WebButton;
import com.alee.laf.panel.WebPanel;
import com.alee.laf.rootpane.WebFrame;
import com.alee.laf.tabbedpane.TabbedPaneStyle;
import com.alee.laf.tabbedpane.WebTabbedPane;

import prog.Application;
import prog.Controller;
import prog.i18n.Lang;

public class MainForm extends WebFrame {
	/**
	 * 
	 */
	private static final long serialVersionUID = 5917570099029563038L;
	public int width;
	public int height;
	public Controller tc;
	// Store dynamic pages for updates
	private java.util.List<ui.layout.DynamicDataPage> dynamicPages;
	private prog.config.ConfigWatcherService configWatcher;
	private javax.swing.Timer repaintTimer;
	Container root;
	WebTabbedPane tabbedPane;

	// AdvancedPanel migrated to ui_layout.cfg
	// FlightInfoPanel migrated to ui_layout.cfg
	// LoggingPanel migrated to ui_layout.cfg
	// MiniHUDPanel migrated to ui_layout.cfg

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
		a.setFont(Application.defaultFontBig);
		a.setTopBgColor(new Color(0, 0, 0, 0));
		a.setBottomBgColor(new Color(0, 0, 0, 0));
		// a.setUndecorated(false);
		// a.setShadeWidth(1);
		a.setBorderPainted(false);

		return a;

	}

	public WebButtonGroup createbuttonGroup() {
		WebButton A = createButton(Lang.mCancel);
		WebButton B = createButton(Lang.mStart);

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
		displayPreview = createButton(Lang.mDisplayPreview);
		WebButton C = createButton(Lang.mClosePreview);
		WebButtonGroup G = new WebButtonGroup(true, displayPreview, C);
		displayPreview.setPreferredWidth(120);

		C.setPreferredWidth(120);
		displayPreview.setFont(Application.defaultFontBig);
		C.setFont(Application.defaultFontBig);
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

	public void initPanel() {
		// Use BorderLayout for frame content
		this.setLayout(new BorderLayout());

		tabbedPane = new WebTabbedPane();
		tabbedPane.setTabPlacement(WebTabbedPane.LEFT);

		// Add empty tabs (content migrated to config)
		// We keep them if user expects them, but really they should be gone if config
		// covers them.
		// For now, let's keep them as placeholders or remove if fully migrated.
		// Task says "Remove LoggingPanel", "AdvancedPanel", etc.
		// If I remove them from UI, the user only sees Dynamic Tabs.
		// Phase 16 says "Final Cleanup: Delete old panel classes"
		// So I should remove these tabs too if they are empty.

		// Add Dynamic Tabs from Config
		dynamicPages = new java.util.ArrayList<>();
		if (tc.dynamicConfigs.isEmpty()) {
			ui.layout.UIBuilder.addRightAlignedTab(tabbedPane, "Data (Empty)", new ui.layout.DynamicDataPage(this),
					Application.defaultFontBig);
		} else {
			for (prog.config.ConfigLoader.GroupConfig group : tc.dynamicConfigs) {
				ui.layout.DynamicDataPage page = new ui.layout.DynamicDataPage(this, group);
				dynamicPages.add(page);
				ui.layout.UIBuilder.addRightAlignedTab(tabbedPane, group.title, page, Application.defaultFontBig);
			}
		}

		tabbedPane.setSelectedIndex(0);
		tabbedPane.setOpaque(false);
		tabbedPane.setBackground(new Color(0, 0, 0, 0));
		tabbedPane.setFont(Application.defaultFontBig);
		tabbedPane.setPaintOnlyTopBorder(true);
		tabbedPane.setPaintBorderOnlyOnSelectedTab(true);
		tabbedPane.setTabbedPaneStyle(TabbedPaneStyle.attached);

		this.add(tabbedPane, BorderLayout.CENTER);

		tabbedPane.addChangeListener(e -> {
			updateDynamicSize();
		});

		startFileWatcher();
	}

	private void startFileWatcher() {
		configWatcher = new prog.config.ConfigWatcherService("ui_layout.cfg", this::reloadDynamicConfig);
		configWatcher.start(2000);
	}

	private void reloadDynamicConfig() {
		// Re-initialize Controller's overlays and configs from disk
		tc.initDynamicOverlays();

		if (dynamicPages == null || tc.dynamicConfigs.isEmpty())
			return;

		// Sync existing pages with the new GroupConfig objects from Controller
		for (prog.config.ConfigLoader.GroupConfig group : tc.dynamicConfigs) {
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
		// 这里是特殊适配, 不能修改.
		int targetWidth = width - 30; // weblaf会加15px+15px==30px的边框, 所以要减去30px.
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
		// flightInfoPanel now loaded from ui_layout.cfg
		// miniHUDPanel now loaded from ui_layout.cfg
		// advancedPanel now loaded from ui_layout.cfg
		// loggingPanel now loaded from ui_layout.cfg
		// engineControlPanel now loaded from ui_layout.cfg
	}

	public void initDefaults() {
		// FlightInfoPanel defaults handled by ConfigurationService
		// MiniHUDPanel defaults handled by ConfigurationService
		// LoggingPanel defaults handled by ConfigurationService
		// AdvancedPanel defaults handled by ConfigurationService

		// crosshairName now handled by MiniHUD section in ui_layout.cfg
	}

	public void saveConfig() {
		if (isInitializing)
			return;

		// flightInfoPanel now saved to ui_layout.cfg
		// advancedPanel now saved to ui_layout.cfg
		// engineInfoPanel.saveConfig(tc.configService);
		// engineControlPanel now saved to ui_layout.cfg
		// loggingPanel now saved to ui_layout.cfg
		// displayFmKey handled by HotkeyRowRenderer and direct config setting

	}

	public void confirm() {
		tc.endPreview();
		moveCheckFlag = false;
		saveConfig();
		tc.saveConfig();
		tc.loadFromConfig();

		this.setVisible(false);
		tc.State = prog.ControllerState.INIT;
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

	public MainForm(Controller c) {
		// System.setProperty("awt.useSystemAAFontSettings", "on");
		// Application.debugPrint("mainForm初始化了");
		width = 800;
		height = 480;
		Image I = Toolkit.getDefaultToolkit().getImage("image/form1.png");
		I = I.getScaledInstance(32, 32, Image.SCALE_SMOOTH);
		this.setIconImage(I);

		tc = c;
		moveCheckFlag = false;

		this.setUndecorated(true);
		this.setLocation(Application.screenWidth / 2 - width / 2, Application.screenHeight / 2 - height / 2);
		this.setFont(Application.defaultFont);
		this.setSize(width, height);

		// setFrameOpaque();// 窗口透明
		String ti = Application.appName + " v" + Application.version;
		if (Application.debug)
			ti = ti + "beta";
		// ti=ti+" ————"+Application.appTooltips;
		setTitle(ti);
		this.setShowMaximizeButton(false);
		this.getWebRootPaneUI().getTitleComponent().getComponent(1)
				.setFont(new Font(Application.defaultFont.getName(), Font.PLAIN, 14));// 设置title字体
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
		isInitializing = false;

		// Execute resize check structure updates
		updateDynamicSize();

		setShowResizeCorner(false);
		setDefaultCloseOperation(3);
		this.setShadeWidth(10);
		// tc.Preview(); // Removed - driven by UI_READY event
		moveCheckFlag = true;
		// tc.setDynamicOverlaysVisible(true, false); // Removed - driven by UI_READY
		// event

		// Show frame only after all resizing is done to prevent jitter
		setVisible(true);

		// Secondary layout pass to ensure WebLaF components settle
		SwingUtilities.invokeLater(() -> {
			updateDynamicSize();
			// Publish UI Ready event to trigger preview and overlay visibility
			prog.event.UIStateBus.getInstance().publish(prog.event.UIStateEvents.UI_READY, null);
		});
	}

	/**
	 * Starts the EDT-safe repaint timer.
	 */
	public void startRepaintTimer() {
		if (repaintTimer != null) {
			repaintTimer.stop();
		}
		repaintTimer = new javax.swing.Timer(33, e -> {
			if (root != null) {
				root.repaint();
			}
		});
		repaintTimer.start();
	}

	/**
	 * Stops the repaint timer.
	 */
	public void stopRepaintTimer() {
		if (repaintTimer != null) {
			repaintTimer.stop();
		}
	}

	public void startPreview() {
		if (moveCheckFlag == null || moveCheckFlag == false) {
			ui.util.NotificationService.show(prog.i18n.Lang.mMovePanel);
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
		java.util.List<prog.config.ConfigLoader.GroupConfig> configs = new java.util.ArrayList<>();
		for (ui.layout.DynamicDataPage page : dynamicPages) {
			if (page.getGroupConfig() != null) {
				configs.add(page.getGroupConfig());
			}
		}
		prog.config.ConfigLoader.saveConfig("ui_layout.cfg", configs);
	}
}