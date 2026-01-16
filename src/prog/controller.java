package prog;

import java.awt.Color;
import java.awt.Font;
import java.awt.Image;
import java.awt.RenderingHints;
import java.awt.Toolkit;

import javax.swing.ImageIcon;

import com.alee.extended.panel.GroupPanel;
import com.alee.extended.time.ClockType;
import com.alee.extended.time.WebClock;
import com.alee.laf.label.WebLabel;
import com.alee.managers.notification.NotificationIcon;
import com.alee.managers.notification.NotificationManager;
import com.alee.managers.notification.WebNotification;

import parser.blkx;
import parser.AttributePool;
import parser.flightAnalyzer;
import parser.flightLog;
import ui.statusBar;
import ui.stickValue;
import ui.attitudeIndicator;
import ui.minimalHUD;
import ui.drawFrame;
import ui.drawFrameSimpl;
import ui.engineInfo;
import ui.engineControl;
import ui.flightInfo;
import ui.gearAndFlaps;
import ui.mainform;
import ui.situationAware;
import ui.someUsefulData;
import ui.model.ConfigProvider;

public class controller implements ConfigProvider {

	public ControllerState state = ControllerState.INIT;

	public boolean logon = false;

	public blkx blkx;
	public AttributePool globalPool = new AttributePool();

	// Robot robot;

	statusBar SB;
	public mainform M;
	public otherService O;
	flightLog Log;
	drawFrame dF;
	flapsControl flc;

	public OverlayManager overlayManager;

	public java.util.List<ui.overlay.DynamicOverlay> dynamicOverlays = new java.util.ArrayList<>();
	public java.util.List<ui.util.ConfigLoader.GroupConfig> dynamicConfigs = new java.util.ArrayList<>();

	// Core Threads
	Thread S1; // Service
	Thread SB1; // StatusBar
	Thread M1; // Mainform
	Thread O1; // OtherService

	public service S;
	public config cfg;
	// 存储参数
	// 主参数

	public long freqService;// Service取数据与计算周期
	// 发动机面板
	public long freqEngineInfo;// engineInfo刷新周期
	public long freqFlightInfo;
	// 人工地平仪
	public long freqAltitude;

	public long freqGearAndFlap;

	public long freqStickValue;
	//

	gcThread G;

	public static boolean engineInfoSwitch;// engineInfo面板开启
	public static boolean engineInfoEdge;// engineInfo面板边缘开启

	public static int engineInfoX;// engineInfo窗口位置
	public static int engineInfoY;

	public static Font engineInfoFont;

	public static int engineInfoOpaque;// engineInfo背景透明度

	public static boolean usetempratureInformation;

	public int lastEvt;
	public int lastDmg;
	public int step;

	Thread gc;

	private uiThread uT;

	private Thread uT1;

	private boolean showStatus;

	// public void hideTaskbarSw() {
	// if (app.debug) {
	// robot.keyPress(17);
	// robot.keyPress(192);
	// try {
	// Thread.sleep(50);
	// } catch (InterruptedException e) {
	// // TODO Auto-generated catch block
	// e.printStackTrace();
	// }
	// robot.keyRelease(17);
	// robot.keyRelease(192);
	// }
	// }

	public void initStatusBar() {

		// 测试全局

		// 状态1，初始化状态条
		if (state == ControllerState.INIT) {
			// app.debugPrint("状态1，初始化状态条");

			if (showStatus) {
				SB = new statusBar();
				SB.init(this);
				SB.S1();
				SB1 = new Thread(SB);
				SB1.start();
			}

			state = ControllerState.CONNECTED;

		}
		// SB.repaint();
	}

	public void changeS2() {
		// 状态2，状态条连接成功，等待进入游戏
		// app.debugPrint(flag);
		// SB.repaint();
		if (state == ControllerState.CONNECTED) {
			// app.debugPrint("状态2，状态条连接成功，等待进入游戏");
			// NotificationManager.showNotification(createWebNotification("您已连接成功，请加入游戏"));
			if (showStatus)
				SB.S2();
			state = ControllerState.IN_GAME;
		}
	}

	public String cur_fmtype;

	private autoMeasure aM;

	private Thread aM1;

	public void changeS3() {
		// 状态3，连接成功，释放状态条，打开面板
		// SB.repaint();
		if (state == ControllerState.IN_GAME) {

			// 自动隐藏任务栏

			// 初始化MapObj以及Msg、gamechat
			// app.debugPrint(S.iIndic.type);
			cur_fmtype = S.sIndic.type;
			getfmdata(cur_fmtype);
			// app.debugPrint("状态3，连接成功，释放状态条，打开面板");
			// usetempratureInformation =
			// Boolean.parseBoolean(getconfig("usetempInfoSwitch"));
			// app.debugPrint(usetempratureInformation);
			// NotificationManager.showNotification(createWebNotificationTime(3000));
			if (showStatus) {
				SB.S3();
				SB.doit = false;
				SB.dispose();
				SB = null;
			}
			System.gc();
			if (app.debug) {
				O = new otherService();
				O.init(this);
				O1 = new Thread(O);
				O1.start();
			}
			state = ControllerState.PREVIEW;
			openpad();

		}
	}

	public void S4toS1() {
		// 状态4，游戏返回，返回至状态1
		if (state == ControllerState.PREVIEW) {
			// app.debugPrint("状态4，游戏退出，释放Service资源，返回至状态1");
			// 不触发燃油低告警
			// S.fuelPercent = 100;

			closepad();
			// 释放资源
			if (app.debug) {
				lastEvt = O.lastEvt;
				lastDmg = O.lastDmg;
				// app.debugPrint("最后DMGID"+lastDmg);
				O.close();
				O = null;
				O1 = null;
			}

			S.clear();
			state = ControllerState.INIT;

			// 自动显示任务栏
			// hideTaskbarSw();
		}

	}

	public void openpad() {
		// Special case: autoMeasure (debug only)
		if (app.fmTesting) {
			aM = new autoMeasure(S);
			aM1 = new Thread(aM);
			aM1.start();
		}

		// Open all registered overlays via OverlayManager
		overlayManager.openAll();

		// Special case: flightLog (has notification and special init)
		if (Boolean.parseBoolean(getconfig("enableLogging"))) {
			if (dF != null) {
				dF.doit = false;
				dF = null;
			}
			notification(lang.cStartlog);
			Log = new flightLog();
			Log.init(this, S);
			logon = true;
		}

		// UI Thread (always runs)
		uT = new uiThread(this);
		uT1 = new Thread(uT);
		uT1.setPriority(Thread.MAX_PRIORITY);
		uT1.start();
		S.startTime = System.currentTimeMillis();
	}

	public void closepad() {
		// Special case: autoMeasure
		if (app.fmTesting && aM != null) {
			aM.doit = false;
			aM1 = null;
			aM = null;
		}

		// Close all managed overlays via OverlayManager
		overlayManager.closeAll();

		// Special case: flightLog (has notification and drawFrame logic)
		if (Boolean.parseBoolean(getconfig("enableLogging")) && (Log != null)) {
			notification(lang.cSavelog + Log.fileName + lang.cPlsopen);
			if (Log.fA.curaltStage - Log.fA.initaltStage >= 1) {
				dF = new drawFrame();
				showdrawFrame(Log.fA);
			}
			Log.close();
			Log = null;
		}

		// UI Thread
		if (uT != null) {
			uT.doit = false;
			uT1.interrupt();
			uT1 = null;
		}

		System.gc();
	}

	public void initconfig() {
		cfg = new config("./config/config.properties");
		if (cfg.getValue("firstTime").equals("True")) {
			// config_init()?
		}
		// NotificationManager.showNotification(createWebNotification("配置信息读入完毕"));
	}

	public String getconfig(String key) {
		return cfg.getValue(key);
	}

	// ConfigProvider interface implementation
	@Override
	public String getConfig(String key) {
		return getconfig(key);
	}

	@Override
	public void setConfig(String key, String value) {
		setconfig(key, value);
	}

	public Color getColorConfig(String key) {
		int R, G, B, A;
		R = Integer.parseInt(getconfig(key + "R"));
		G = Integer.parseInt(getconfig(key + "G"));
		B = Integer.parseInt(getconfig(key + "B"));
		A = Integer.parseInt(getconfig(key + "A"));
		return new Color(R, G, B, A);
	}

	public void setColorConfig(String key, Color c) {
		int R = c.getRed();
		int G = c.getGreen();
		int B = c.getBlue();
		int A = c.getAlpha();

		setconfig(key + "R", Integer.toString(R));
		setconfig(key + "G", Integer.toString(G));
		setconfig(key + "B", Integer.toString(B));
		setconfig(key + "A", Integer.toString(A));
	}

	public void loadFromConfig() {
		// 修改完设置在读取
		freqService = Long.parseLong(getconfig("Interval"));
		// 刷新频率比例
		freqEngineInfo = (long) (freqService * 2f);

		freqFlightInfo = (long) (freqService * 1.5f);
		freqAltitude = (long) (freqService * 1.5f);

		freqGearAndFlap = (long) (freqService * 2f);
		freqStickValue = (long) (freqService * 1f);
		// 取频率的3分之一作为休眠时间
		app.threadSleepTime = (long) (freqService / 3);

		// app.debugPrint(freqService);

		// 颜色

		// 修改颜色
		// app.debugPrint(R +", " + G + ", " + B + "," +A);
		app.colorNum = getColorConfig("fontNum");

		// 标签颜色
		app.colorLabel = getColorConfig("fontLabel");

		// 单位颜色
		app.colorUnit = getColorConfig("fontUnit");

		// 警告颜色
		app.colorWarning = getColorConfig("fontWarn");

		// 描边颜色
		app.colorShadeShape = getColorConfig("fontShade");

		// 声音
		app.voiceVolumn = Integer.parseInt(getconfig("voiceVolume"));
		// fontLabelR=32
		// fontLabelG=222
		// fontLabelB=64
		// fontLabelA=140
		//
		// fontUnitR=166
		// fontUnitG=166
		// fontUnitB=166
		// fontUnitA=220
		//
		// fontWarnR=216
		// fontWarnG=33
		// fontWarnB=13
		// fontWarnA=100
		//
		// fontShadeR=0
		// fontShadeG=0
		// fontShadeB=0
		// fontShadeA=42
		showStatus = true;
		if (getconfig("enableStatusBar") != "")
			showStatus = Boolean.parseBoolean(getconfig("enableStatusBar"));
		// 读取字体绘制方式
		app.drawFontShape = !Boolean.parseBoolean(getconfig("simpleFont"));

		// 读取抗锯齿
		app.aaEnable = Boolean.parseBoolean(getconfig("AAEnable"));
		if (app.aaEnable) {
			// app.textAASetting = RenderingHints.VALUE_TEXT_ANTIALIAS_GASP;
			app.textAASetting = RenderingHints.VALUE_TEXT_ANTIALIAS_ON;
			app.graphAASetting = RenderingHints.VALUE_ANTIALIAS_ON;
		} else {
			app.textAASetting = RenderingHints.VALUE_TEXT_ANTIALIAS_OFF;
			app.graphAASetting = RenderingHints.VALUE_ANTIALIAS_OFF;
		}
	}

	public controller() {
		initconfig();// 装载设置文件
		// 接收频率
		// app.debugPrint("controller执行了");
		loadFromConfig();
		initDynamicOverlays();
		registerHotkeyListener();
		usetempratureInformation = false;

		// Initialize OverlayManager and register overlays
		overlayManager = new OverlayManager(this);
		registerGameModeOverlays();

		// 刷新频率
		state = ControllerState.INIT;
		lastEvt = 0;
		lastDmg = 0;

		// 状态0，初始化主界面和设置文件
		// app.debugPrint("状态0，初始化主界面");

		M = new mainform(this);
		M1 = new Thread(M);
		M1.start();

		// G = new gcThread();
		// G.init(this);
		// gc = new Thread(G);
		// gc.start();
		// start();

		// 初始化ROBOT
		// try {
		// robot = new Robot();
		// } catch (AWTException e) {
		// // TODO Auto-generated catch block
		// e.printStackTrace();
		// }

	}

	public void start() {
		if (state == ControllerState.INIT) {

			// app.debugPrint(freqService);
			// 状态1，释放设置窗口传参初始化后台
			// app.debugPrint("状态1，传参初始化Service");
			M.doit = false;
			M1 = null;
			M.dispose();
			M = null;

			System.gc();
			// NotificationManager.showNotification(createWebNotification("程序最小化至托盘，注意右上角状态条提示"));

			S = new service(this);
			S1 = new Thread(S);
			/* 设置高优先级 */
			S1.setPriority(Thread.MAX_PRIORITY);
			S1.start();

		}

	}

	/**
	 * Register all game mode overlays with OverlayManager.
	 * Uses registerWithPreview for overlays that support preview mode.
	 */
	private void registerGameModeOverlays() {
		// engineInfo - supports preview
		overlayManager.registerWithPreview("engineInfoSwitch",
				() -> new engineInfo(),
				overlay -> ((engineInfo) overlay).init(this, S, blkx),
				overlay -> ((engineInfo) overlay).initPreview(this),
				overlay -> ((engineInfo) overlay).reinitConfig(),
				false);

		// engineControl - supports preview
		overlayManager.registerWithPreview("enableEngineControl",
				() -> new engineControl(),
				overlay -> ((engineControl) overlay).init(this, S, blkx),
				overlay -> ((engineControl) overlay).initPreview(this),
				overlay -> ((engineControl) overlay).reinitConfig(),
				false);

		// minimalHUD (crosshair) - supports preview
		overlayManager.registerWithPreview("crosshairSwitch",
				() -> new minimalHUD(),
				overlay -> ((minimalHUD) overlay).init(this, S, O),
				overlay -> ((minimalHUD) overlay).initPreview(this),
				overlay -> ((minimalHUD) overlay).reinitConfig(),
				false);

		// flightInfo - supports preview
		overlayManager.registerWithPreview("flightInfoSwitch",
				() -> new flightInfo(),
				overlay -> ((flightInfo) overlay).init(this, globalPool, ui.model.FlightInfoConfig.createDefault(this)),
				overlay -> ((flightInfo) overlay).initPreview(this, globalPool,
						ui.model.FlightInfoConfig.createDefault(this)),
				overlay -> ((flightInfo) overlay).reinitConfig(),
				false);

		// stickValue - supports preview (uses initpreview lowercase)
		overlayManager.registerWithPreview("enableAxis",
				() -> new stickValue(),
				overlay -> ((stickValue) overlay).init(this, S),
				overlay -> ((stickValue) overlay).initpreview(this),
				overlay -> ((stickValue) overlay).reinitConfig(),
				false);

		// attitudeIndicator - supports preview (uses initpreview lowercase)
		overlayManager.registerWithPreview("enableAttitudeIndicator",
				() -> new attitudeIndicator(),
				overlay -> ((attitudeIndicator) overlay).init(this, S),
				overlay -> ((attitudeIndicator) overlay).initpreview(this),
				overlay -> ((attitudeIndicator) overlay).reinitConfig(),
				false);

		// gearAndFlaps - supports preview
		overlayManager.registerWithPreview("enablegearAndFlaps",
				() -> new gearAndFlaps(),
				overlay -> ((gearAndFlaps) overlay).init(this, S),
				overlay -> ((gearAndFlaps) overlay).initPreview(this),
				overlay -> ((gearAndFlaps) overlay).reinitConfig(),
				false);

		// voiceWarning - game mode only, no preview
		overlayManager.registerWithStrategy("enableVoiceWarn",
				() -> new voiceWarning(),
				overlay -> ((voiceWarning) overlay).init(this, S),
				null, // No preview initializer
				null, // No re-initializer
				true,
				ActivationStrategy.config("enableVoiceWarn").and(ActivationStrategy.gameModeOnly()));

		// someUsefulData (FMPrint) - supports preview
		overlayManager.registerWithPreview("enableFMPrint",
				() -> new someUsefulData(),
				overlay -> ((someUsefulData) overlay).init(this, blkx),
				overlay -> ((someUsefulData) overlay).initPreview(this, blkx),
				overlay -> ((someUsefulData) overlay).reinitConfig(blkx),
				true);

		// thrustdFS - requires enableFMPrint AND isJet
		overlayManager.registerWithStrategy("thrustdFS",
				() -> new drawFrameSimpl(),
				overlay -> ((drawFrameSimpl) overlay).init(this),
				overlay -> ((drawFrameSimpl) overlay).initPreview(this),
				overlay -> ((drawFrameSimpl) overlay).reinitConfig(),
				true,
				ActivationStrategy.config("enableFMPrint").and(ActivationStrategy.jetOnly()));

		// situationAware - debug only
		overlayManager.registerWithStrategy("situationAware",
				() -> new situationAware(),
				overlay -> {
					((situationAware) overlay).init(this, O);
				},
				overlay -> ((situationAware) overlay).initPreview(this),
				overlay -> ((situationAware) overlay).reinitConfig(),
				true,
				ActivationStrategy.debugOnly());
	}

	public void initDynamicOverlays() {
		// Clean up existing
		for (ui.overlay.DynamicOverlay overlay : dynamicOverlays) {
			overlay.doit = false;
			overlay.dispose();
		}
		dynamicOverlays.clear();

		dynamicConfigs = ui.util.ConfigLoader.loadConfig("ui_layout.cfg");
		for (ui.util.ConfigLoader.GroupConfig config : dynamicConfigs) {
			ui.overlay.DynamicOverlay overlay = new ui.overlay.DynamicOverlay(this, config);
			dynamicOverlays.add(overlay);
			// Start the self-refresh thread
			new Thread(overlay).start();
		}
	}

	public void setDynamicOverlaysVisible(boolean visible, boolean respectHotkey) {
		for (ui.overlay.DynamicOverlay overlay : dynamicOverlays) {
			if (visible) {
				// Entering Game Mode (respectHotkey=true)
				if (respectHotkey) {
					// Step 4: If Visible=true, start HIDDEN. Otherwise, start HIDDEN.
					overlay.setVisible(false);
				} else {
					// Step 2 & 4 (Preview Mode): show if config says Visible=true
					overlay.setVisible(overlay.getGroupConfig().visible);
				}
			} else {
				// Global hide
				overlay.setVisible(false);
			}
		}
	}

	public void registerHotkeyListener() {
		try {
			app.silenceNativeHookLogger();
			if (!com.github.kwhat.jnativehook.GlobalScreen.isNativeHookRegistered()) {
				com.github.kwhat.jnativehook.GlobalScreen.registerNativeHook();
			}
		} catch (com.github.kwhat.jnativehook.NativeHookException ex) {
			ex.printStackTrace();
		}

		com.github.kwhat.jnativehook.GlobalScreen
				.addNativeKeyListener(new com.github.kwhat.jnativehook.keyboard.NativeKeyListener() {
					@Override
					public void nativeKeyPressed(com.github.kwhat.jnativehook.keyboard.NativeKeyEvent e) {
						int code = e.getKeyCode();
						// 动态 Overlay 切换
						if (dynamicOverlays != null) {
							for (ui.overlay.DynamicOverlay overlay : dynamicOverlays) {
								// Step 2 & 4: Only listen if Visible=true in config
								if (overlay.getGroupConfig().visible && overlay.getGroupConfig().hotkey != 0
										&& overlay.getGroupConfig().hotkey == code) {
									overlay.toggleVisibility();
								}
							}
						}
					}
				});
	}

	public void stop() {
		// Clean up dynamic overlays
		if (dynamicOverlays != null) {
			for (ui.overlay.DynamicOverlay overlay : dynamicOverlays) {
				overlay.doit = false;
				overlay.dispose();
			}
			dynamicOverlays.clear();
		}

		if (M != null) {
			M.doit = false;
			M1 = null;
			M.dispose();
			M = null;
			System.gc();
			return;
		}

		// if (S1 == null) {
		// return;
		// }
		if (state == ControllerState.PREVIEW) {
			closepad();
		}

		S = null;
		S1.interrupt();
		S1 = null;
		System.gc();
	}

	public void Preview() {
		refreshPreviews();
	}

	/**
	 * Ensure blkx data is loaded, either from live source or config.
	 */
	private void ensureBlkxLoaded() {
		httpHelper httpDataFetcher = new httpHelper();
		String livePlaneName = httpDataFetcher.getLiveAircraftType();

		if (livePlaneName != null) {
			getfmdata(livePlaneName);
		}

		// Ensure blkx is initialized (fallback to default)
		if (blkx == null || !blkx.valid) {
			String planeName = getconfig("selectedFM0");
			if (planeName != null && !planeName.isEmpty()) {
				getfmdata(planeName);
			}
		}
	}

	public void refreshPreviews() {
		loadFromConfig();
		ensureBlkxLoaded();
		overlayManager.refreshAllPreviews();
	}

	public void endPreview() {
		overlayManager.closeAll();

		// Save UI Layout
		// ui.model.FlightInfoConfig.saveConfig(globalPool, "ui_layout.cfg");

		// Clean up Configurable Overlays
		for (ui.overlay.DynamicOverlay overlay : dynamicOverlays) {
			overlay.saveCurrentPosition();
			overlay.dispose();
		}
		dynamicOverlays.clear();

		System.gc();
	}

	public void setconfig(String key, String value) {
		cfg.setValue(key, value);
	}

	public void saveconfig() {
		cfg.saveFile("./config/config.properties", "8111");
		// NotificationManager.showNotification(createWebNotification("配置信息写入完毕"));
	}

	public static void notificationtime(String text, int time) {
		ui.util.NotificationService.showTimed(text, time);
	}

	public static void notificationtimeAbout(String text, int time) {
		ui.util.NotificationService.showAbout(text, time);
	}

	public static void notification(String text) {
		ui.util.NotificationService.show(text);
	}

	// Legacy methods - now delegate to NotificationService
	static WebNotification createWebNotificationTime(long time) {
		// Used internally, keeping for backwards compatibility
		return createTimerNotification(time, lang.cOpenpad);
	}

	static WebNotification createWebNotificationEngineTime(long time) {
		return createTimerNotification(time, lang.cEnginedmg);
	}

	private static WebNotification createTimerNotification(long time, String pattern) {
		WebNotification a = new WebNotification();
		a.setFont(app.defaultFont);
		a.setIcon(NotificationIcon.clock.getIcon());
		a.setWindowOpacity((float) (0.5));
		WebClock clock = new WebClock();
		clock.setClockType(ClockType.timer);
		clock.setTimeLeft(time);
		clock.setFont(app.defaultFont);
		clock.setTimePattern(pattern);
		a.setContent(new GroupPanel(clock));
		clock.start();
		a.setDisplayTime(time);
		a.setFocusable(false);
		return a;
	}

	static WebNotification createWebNotifications(String text, int time) {
		WebNotification a = new WebNotification();
		WebLabel text1 = new WebLabel(text);
		text1.setFont(app.defaultFont);
		a.setFont(app.defaultFont);
		a.setIcon(NotificationIcon.information.getIcon());
		a.add(text1);
		a.setDisplayTime(time);
		a.setFocusable(false);
		return a;
	}

	static WebNotification createWebNotificationsAbout(String text, int time) {
		WebNotification a = new WebNotification();
		WebLabel text1 = new WebLabel(text);
		text1.setFont(new Font(app.defaultFontName, Font.PLAIN, 14));
		Image I = Toolkit.getDefaultToolkit().createImage("image/fubuki.jpg");
		ImageIcon A = new ImageIcon(I);
		a.setFont(app.defaultFont);
		a.setIcon(A);
		a.add(text1);
		a.setDisplayTime(time);
		a.setFocusable(false);
		return a;
	}

	static WebNotification createWebNotification(String text) {
		WebNotification a = new WebNotification();
		WebLabel text1 = new WebLabel(text);
		text1.setFont(app.defaultFont);
		a.setFont(app.defaultFont);
		a.setIcon(NotificationIcon.information.getIcon());
		a.add(text1);
		a.setDisplayTime(5000);
		a.setFocusable(false);
		return a;
	}

	static WebNotification createWebNotificationEngineBomb(String text) {
		WebNotification a = new WebNotification();
		WebLabel text1 = new WebLabel(text);
		text1.setFont(app.defaultFont);
		a.setFont(app.defaultFont);
		a.setIcon(NotificationIcon.error);
		a.add(text1);
		a.setDisplayTime(3000);
		a.setFocusable(false);
		return a;
	}

	public void showdrawFrame(flightAnalyzer fA) {
		dF.init(this, fA);
	}

	public void writeDown() {

		if (logon) {
			if (Log.doit == false) {
				Log.doit = true;
				// Log1.start();

			} else {
				Log.doit = true;
				app.debugPrint("线程同步错误");
			}
			// app.debugPrint(Log.doit);
		}
	}

	void getfmdata(String planename) {
		String fmfile = null;
		// String unitSystem;
		// 读入fm
		String planeFileName = planename.toLowerCase();
		blkx = new blkx("./data/aces/gamedata/flightmodels/" + planeFileName + ".blkx", planeFileName + ".blk");
		if (blkx.valid == true) {
			fmfile = blkx.getlastone("fmfile");
			/* 去除多余的空格 */
			if (fmfile != null) {
				fmfile = fmfile.substring(fmfile.indexOf("\"") + 1, fmfile.length() - 1);
				app.debugPrint(fmfile);
				/* 去除多余的/ */
				if (fmfile.charAt(0) == '/')
					fmfile = fmfile.substring(1);
			}
		}
		if (fmfile == null) {
			/* 直接读取 */
			fmfile = "fm/" + planeFileName + ".blk";
		}

		/* F**k Gaijin's shit-mountain code */
		if (-1 == fmfile.indexOf(".blk")) {
			fmfile += ".blk";
		}
		app.debugPrint("fmfile :" + fmfile);

		// 读入fmfile
		blkx = new blkx("./data/aces/gamedata/flightmodels/" + fmfile + "x", fmfile);

		if (blkx.valid == true) {// app.debugPrint(blkx.data);
			blkx.getAllplotdata();
			// blkx.getload();
			blkx.data = null;

			// Dump FM data to Global Pool
			globalPool.putAll(blkx.getVariableMap());
		}

	}

}