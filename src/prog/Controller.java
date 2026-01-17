package prog;

import prog.i18n.Lang;
import prog.audio.VoiceWarning;
import prog.util.HttpHelper;

import java.awt.Color;
import java.awt.Font;

import parser.Blkx;
import parser.AttributePool;
import parser.FlightAnalyzer;
import parser.FlightLog;
import ui.StatusBar;
import ui.overlay.StickValue;
import ui.overlay.AttitudeIndicator;
import ui.overlay.MinimalHUD;
import ui.overlay.DrawFrame;
import ui.overlay.DrawFrameSimpl;
import ui.overlay.EngineInfo;
import ui.overlay.EngineControl;
import ui.overlay.FlightInfo;
import ui.overlay.GearAndFlaps;
import ui.MainForm;
import ui.overlay.SituationAware;
import prog.model.UsefulData;
import prog.config.ConfigProvider;
import prog.config.ConfigurationService;

public class Controller implements ConfigProvider {

	public ControllerState State = ControllerState.INIT;

	public boolean logon = false;

	public Blkx Blkx;
	public AttributePool globalPool = new AttributePool();

	// Robot robot;

	StatusBar SB;
	public MainForm M;
	public OtherService O;
	FlightLog Log;
	DrawFrame dF;
	FlapsControl flc;

	public OverlayManager overlayManager;

	// Temporarily disabled - DynamicOverlay removed
	// public java.util.List<ui.overlay.DynamicOverlay> dynamicOverlays = new
	// java.util.ArrayList<>();
	public java.util.List<prog.config.ConfigLoader.GroupConfig> dynamicConfigs = new java.util.ArrayList<>();

	// Core Threads
	Thread S1; // Service
	Thread SB1; // StatusBar
	Thread M1; // Mainform
	Thread O1; // OtherService

	public Service S;
	// Legacy support via ConfigurationService
	public ConfigurationService configService;
	// public config cfg; // Removed
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

	private UIThread uT;

	private Thread uT1;

	private boolean showStatus;

	// public void hideTaskbarSw() {
	// if (Application.debug) {
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
		if (State == ControllerState.INIT) {
			// Application.debugPrint("状态1，初始化状态条");

			if (showStatus) {
				SB = new StatusBar();
				SB.init(this);
				SB.S1();
				SB1 = new Thread(SB);
				SB1.start();
			}

			State = ControllerState.CONNECTED;

		}
		// SB.repaint();
	}

	public void changeS2() {
		// 状态2，状态条连接成功，等待进入游戏
		// Application.debugPrint(flag);
		// SB.repaint();
		if (State == ControllerState.CONNECTED) {
			// Application.debugPrint("状态2，状态条连接成功，等待进入游戏");
			// NotificationManager.showNotification(createWebNotification("您已连接成功，请加入游戏"));
			if (showStatus)
				SB.S2();
			State = ControllerState.IN_GAME;
		}
	}

	public String cur_fmtype;

	private AutoMeasure aM;

	private Thread aM1;

	public void changeS3() {
		// 状态3，连接成功，释放状态条，打开面板
		// SB.repaint();
		if (State == ControllerState.IN_GAME) {

			// 自动隐藏任务栏

			// 初始化MapObj以及Msg、gamechat
			// Application.debugPrint(S.iIndic.type);
			cur_fmtype = S.sIndic.type;
			getfmdata(cur_fmtype);
			// Application.debugPrint("状态3，连接成功，释放状态条，打开面板");
			// usetempratureInformation =
			// Boolean.parseBoolean(getconfig("usetempInfoSwitch"));
			// Application.debugPrint(usetempratureInformation);
			// NotificationManager.showNotification(createWebNotificationTime(3000));
			if (showStatus) {
				SB.S3();
				SB.doit = false;
				SB.dispose();
				SB = null;
			}
			System.gc();
			if (Application.debug) {
				O = new OtherService();
				O.init(this);
				O1 = new Thread(O);
				O1.start();
			}
			State = ControllerState.PREVIEW;
			openpad();

		}
	}

	public void S4toS1() {
		// 状态4，游戏返回，返回至状态1
		if (State == ControllerState.PREVIEW) {
			// Application.debugPrint("状态4，游戏退出，释放Service资源，返回至状态1");
			// 不触发燃油低告警
			// S.fuelPercent = 100;

			closepad();
			// 释放资源
			if (Application.debug) {
				lastEvt = O.lastEvt;
				lastDmg = O.lastDmg;
				// Application.debugPrint("最后DMGID"+lastDmg);
				O.close();
				O = null;
				O1 = null;
			}

			S.clear();
			State = ControllerState.INIT;

			// 自动显示任务栏
			// hideTaskbarSw();
		}

	}

	public void openpad() {
		// Special case: AutoMeasure (debug only)
		if (Application.fmTesting) {
			aM = new AutoMeasure(S);
			aM1 = new Thread(aM);
			aM1.start();
		}

		// Open all registered overlays via OverlayManager
		overlayManager.openAll();

		// Special case: FlightLog (has notification and special init)
		if (Boolean.parseBoolean(getconfig("enableLogging"))) {
			if (dF != null) {
				dF.doit = false;
				dF = null;
			}
			ui.util.NotificationService.show(Lang.cStartlog);
			Log = new FlightLog();
			Log.init(this, S);
			logon = true;
		}

		// UI Thread (always runs)
		uT = new UIThread(this);
		uT1 = new Thread(uT);
		uT1.setPriority(Thread.MAX_PRIORITY);
		uT1.start();
		S.startTime = System.currentTimeMillis();
	}

	public void closepad() {
		// Special case: AutoMeasure
		if (Application.fmTesting && aM != null) {
			aM.doit = false;
			aM1 = null;
			aM = null;
		}

		// Close all managed overlays via OverlayManager
		overlayManager.closeAll();

		// Special case: FlightLog (has notification and DrawFrame logic)
		if (Boolean.parseBoolean(getconfig("enableLogging")) && (Log != null)) {
			ui.util.NotificationService.show(Lang.cSavelog + Log.fileName + Lang.cPlsopen);
			if (Log.fA.curaltStage - Log.fA.initaltStage >= 1) {
				dF = new DrawFrame();
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

	// Removed initconfig() - moved to ConfigurationService

	// --- Config Delegation ---

	@Override
	public String getConfig(String key) {
		return configService.getConfig(key);
	}

	@Override
	public void setConfig(String key, String value) {
		configService.setConfig(key, value);
	}

	// Legacy lowercase methods
	public String getconfig(String key) {
		return getConfig(key);
	}

	public void setconfig(String key, String value) {
		setConfig(key, value);
	}

	public void saveConfig() {
		configService.saveConfig();
	}

	public Color getColorConfig(String key) {
		return configService.getColorConfig(key);
	}

	public void setColorConfig(String key, Color c) {
		configService.setColorConfig(key, c);
	}

	public void loadFromConfig() {
		configService.loadAppCheck(this);
		// Sync local flags
		showStatus = true;
		if (getconfig("enableStatusBar") != "")
			showStatus = Boolean.parseBoolean(getconfig("enableStatusBar"));
	}

	public Controller() {
		configService = new ConfigurationService();
		configService.initConfig();// 装载设置文件
		// 接收频率
		// Application.debugPrint("controller执行了");
		loadFromConfig();
		initDynamicOverlays();
		registerHotkeyListener();
		usetempratureInformation = false;

		// Initialize OverlayManager and register overlays
		overlayManager = new OverlayManager(this);
		registerGameModeOverlays();

		// 刷新频率
		State = ControllerState.INIT;
		lastEvt = 0;
		lastDmg = 0;

		// 状态0，初始化主界面和设置文件
		// Application.debugPrint("状态0，初始化主界面");

		M = new MainForm(this);
		M.startRepaintTimer();

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
		if (State == ControllerState.INIT) {

			// Application.debugPrint(freqService);
			// 状态1，释放设置窗口传参初始化后台
			// Application.debugPrint("状态1，传参初始化Service");
			M.stopRepaintTimer();
			M.dispose();
			M = null;

			// Suggest GC after disposing the main settings window (significant memory
			// release)
			System.gc();
			// NotificationManager.showNotification(createWebNotification("程序最小化至托盘，注意右上角状态条提示"));

			S = new Service(this);
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
		// EngineInfo - supports preview
		overlayManager.registerWithPreview("engineInfoSwitch",
				() -> new EngineInfo(),
				overlay -> ((EngineInfo) overlay).init(this, S, Blkx),
				overlay -> ((EngineInfo) overlay).initPreview(this),
				overlay -> ((EngineInfo) overlay).reinitConfig(),
				true);

		// EngineControl - supports preview
		overlayManager.registerWithPreview("enableEngineControl",
				() -> new EngineControl(),
				overlay -> ((EngineControl) overlay).init(this, S, Blkx),
				overlay -> ((EngineControl) overlay).initPreview(this),
				overlay -> ((EngineControl) overlay).reinitConfig(),
				true);

		// MinimalHUD (crosshair) - supports preview
		overlayManager.registerWithPreview("crosshairSwitch",
				() -> new MinimalHUD(),
				overlay -> ((MinimalHUD) overlay).init(this, S, O),
				overlay -> ((MinimalHUD) overlay).initPreview(this),
				overlay -> ((MinimalHUD) overlay).reinitConfig(),
				false);

		// FlightInfo - supports preview
		overlayManager.registerWithPreview("flightInfoSwitch",
				() -> new FlightInfo(),
				overlay -> ((FlightInfo) overlay).init(this, globalPool, ui.model.FlightInfoConfig.createDefault(this)),
				overlay -> ((FlightInfo) overlay).initPreview(this, globalPool,
						ui.model.FlightInfoConfig.createDefault(this)),
				overlay -> ((FlightInfo) overlay).reinitConfig(),
				false);

		// StickValue - supports preview (uses initpreview lowercase)
		overlayManager.registerWithPreview("enableAxis",
				() -> new StickValue(),
				overlay -> ((StickValue) overlay).init(this, S),
				overlay -> ((StickValue) overlay).initpreview(this),
				overlay -> ((StickValue) overlay).reinitConfig(),
				false);

		// AttitudeIndicator - supports preview (uses initpreview lowercase)
		overlayManager.registerWithPreview("enableAttitudeIndicator",
				() -> new AttitudeIndicator(),
				overlay -> ((AttitudeIndicator) overlay).init(this, S),
				overlay -> ((AttitudeIndicator) overlay).initpreview(this),
				overlay -> ((AttitudeIndicator) overlay).reinitConfig(),
				false);

		// GearAndFlaps - supports preview
		overlayManager.registerWithPreview("enablegearAndFlaps",
				() -> new GearAndFlaps(),
				overlay -> ((GearAndFlaps) overlay).init(this, S),
				overlay -> ((GearAndFlaps) overlay).initPreview(this),
				overlay -> ((GearAndFlaps) overlay).reinitConfig(),
				false);

		// VoiceWarning - game mode only, no preview
		overlayManager.registerWithStrategy("enableVoiceWarn",
				() -> new VoiceWarning(),
				overlay -> ((VoiceWarning) overlay).init(this, S),
				null, // No preview initializer
				null, // No re-initializer
				true,
				ActivationStrategy.config("enableVoiceWarn").and(ActivationStrategy.gameModeOnly()));

		// UsefulData (FMPrint) - supports preview
		overlayManager.registerWithPreview("enableFMPrint",
				() -> new UsefulData(),
				overlay -> ((UsefulData) overlay).init(this, Blkx),
				overlay -> ((UsefulData) overlay).initPreview(this, Blkx),
				overlay -> ((UsefulData) overlay).reinitConfig(Blkx),
				true);

		// thrustdFS - requires enableFMPrint AND isJet
		overlayManager.registerWithStrategy("thrustdFS",
				() -> new DrawFrameSimpl(),
				overlay -> ((DrawFrameSimpl) overlay).init(this),
				overlay -> ((DrawFrameSimpl) overlay).initPreview(this),
				overlay -> ((DrawFrameSimpl) overlay).reinitConfig(),
				true,
				ActivationStrategy.config("enableFMPrint").and(ActivationStrategy.jetOnly()));

		// SituationAware - debug only
		overlayManager.registerWithStrategy("SituationAware",
				() -> new SituationAware(),
				overlay -> {
					((SituationAware) overlay).init(this, O);
				},
				overlay -> ((SituationAware) overlay).initPreview(this),
				overlay -> ((SituationAware) overlay).reinitConfig(),
				true,
				ActivationStrategy.debugOnly());
	}

	public void initDynamicOverlays() {
		// Temporarily disabled - DynamicOverlay removed
		// Clean up existing
		// for (ui.overlay.DynamicOverlay overlay : dynamicOverlays) {
		// overlay.doit = false;
		// overlay.dispose();
		// }
		// dynamicOverlays.clear();

		dynamicConfigs = prog.config.ConfigLoader.loadConfig("ui_layout.cfg");
		// for (prog.config.ConfigLoader.GroupConfig config : dynamicConfigs) {
		// ui.overlay.DynamicOverlay overlay = new ui.overlay.DynamicOverlay(this,
		// config);
		// dynamicOverlays.add(overlay);
		// // Start the self-refresh thread
		// new Thread(overlay).start();
		// }
	}

	public void setDynamicOverlaysVisible(boolean visible, boolean respectHotkey) {
		// Temporarily disabled - DynamicOverlay removed
		// for (ui.overlay.DynamicOverlay overlay : dynamicOverlays) {
		// if (visible) {
		// // Entering Game Mode (respectHotkey=true)
		// if (respectHotkey) {
		// // Step 4: If Visible=true, start HIDDEN. Otherwise, start HIDDEN.
		// overlay.setVisible(false);
		// } else {
		// // Step 2 & 4 (Preview Mode): show if config says Visible=true
		// overlay.setVisible(overlay.getGroupConfig().visible);
		// }
		// } else {
		// // Global hide
		// overlay.setVisible(false);
		// }
		// }
	}

	public void registerHotkeyListener() {
		try {
			Application.silenceNativeHookLogger();
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
						// TODO: Dynamic Overlay hotkey handling disabled
					}
				});
	}

	public void stop() {
		// Temporarily disabled - DynamicOverlay removed
		// if (dynamicOverlays != null) {
		// for (ui.overlay.DynamicOverlay overlay : dynamicOverlays) {
		// overlay.doit = false;
		// overlay.dispose();
		// }
		// dynamicOverlays.clear();
		// }

		if (M != null) {
			M.stopRepaintTimer();
			M.dispose();
			M = null;
			System.gc();
			return;
		}

		// if (S1 == null) {
		// return;
		// }
		if (State == ControllerState.PREVIEW) {
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
	 * Ensure Blkx data is loaded, either from live source or config.
	 */
	private void ensureBlkxLoaded() {
		HttpHelper httpDataFetcher = new HttpHelper();
		String livePlaneName = httpDataFetcher.getLiveAircraftType();

		if (livePlaneName != null) {
			getfmdata(livePlaneName);
		}

		// Ensure Blkx is initialized (fallback to default)
		if (Blkx == null || !Blkx.valid) {
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

		// Temporarily disabled - DynamicOverlay removed
		// for (ui.overlay.DynamicOverlay overlay : dynamicOverlays) {
		// overlay.saveCurrentPosition();
		// overlay.dispose();
		// }
		// dynamicOverlays.clear();

		// Suggest GC after closing all overlays (significant graphics resources
		// released)
		System.gc();
	}

	// saveconfig() already replaced in delegation block

	public void showdrawFrame(FlightAnalyzer fA) {
		dF.init(this, fA);
	}

	public void writeDown() {

		if (logon) {
			if (Log.doit == false) {
				Log.doit = true;
				// Log1.start();

			} else {
				Log.doit = true;
				Application.debugPrint("线程同步错误");
			}
			// Application.debugPrint(Log.doit);
		}
	}

	void getfmdata(String planename) {
		String fmfile = null;
		// String unitSystem;
		// 读入fm
		String planeFileName = planename.toLowerCase();
		Blkx = new Blkx("./data/aces/gamedata/flightmodels/" + planeFileName + ".Blkx", planeFileName + ".blk");
		if (Blkx.valid == true) {
			fmfile = Blkx.getlastone("fmfile");
			/* 去除多余的空格 */
			if (fmfile != null) {
				fmfile = fmfile.substring(fmfile.indexOf("\"") + 1, fmfile.length() - 1);
				Application.debugPrint(fmfile);
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
		Application.debugPrint("fmfile :" + fmfile);

		// 读入fmfile
		Blkx = new Blkx("./data/aces/gamedata/flightmodels/" + fmfile + "x", fmfile);

		if (Blkx.valid == true) {// Application.debugPrint(Blkx.data);
			Blkx.getAllplotdata();
			// Blkx.getload();
			Blkx.data = null;

			// Dump FM data to Global Pool
			globalPool.putAll(Blkx.getVariableMap());
		}

	}

}