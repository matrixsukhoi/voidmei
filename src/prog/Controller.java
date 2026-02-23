package prog;

import prog.i18n.Lang;
import prog.audio.VoiceWarning;
import prog.util.HttpHelper;
import prog.util.FMPowerExtractor;

import java.awt.Color;
import java.awt.Font;

import parser.Blkx;
import parser.FlightAnalyzer;
import parser.FlightLog;
import ui.StatusBar;
import ui.overlay.ControlSurfacesOverlay;
import ui.overlay.AttitudeOverlay;
import ui.overlay.MiniHUDOverlay;
import ui.overlay.DrawFrame;
import ui.overlay.DrawFrameSimpl;
import ui.overlay.EngineControlOverlay;
import ui.overlay.PowerInfoOverlay;
import ui.overlay.GearFlapsOverlay;
import ui.overlay.FMUnpackedDataOverlay;
import ui.overlay.FlightInfoOverlay;
import ui.MainForm;
import prog.config.ConfigProvider;
import prog.config.ConfigurationService;
import prog.hotkey.HotkeyManager;
import prog.event.UIStateEvents;
import com.github.kwhat.jnativehook.keyboard.NativeKeyEvent;

import java.util.concurrent.atomic.AtomicLong;

public class Controller implements ConfigProvider {

	public ControllerState State = ControllerState.INIT;

	/** Generation counter for detecting stale preview callbacks */
	private final AtomicLong previewGeneration = new AtomicLong(0);

	public boolean logon = false;

	private Blkx Blkx;
	private String loadedFMName = null;
	private String identifiedFMName = null;
	private long lastBlkxCheckTime = 0;
	private static final long BLKX_CHECK_INTERVAL = 5000;

	/** Cached compressor stage parameters for multi-stage supercharger aircraft */
	private prog.util.PistonPowerModel.CompressorStageParams[] compressorStages;

	/** Cached peak WEP power for piston aircraft (hp) */
	private double peakWepPower = 0;

	/** Cached peak afterburner thrust for jet aircraft (kgf) */
	private double peakThrust = 0;

	/** FM data adapter for FMUnpackedDataOverlay */
	private ui.model.FMDataAdapter fmDataAdapter = new ui.model.FMDataAdapter();

	// Robot robot;

	StatusBar SB;
	public MainForm M;
	public OtherService O;
	FlightLog Log;
	DrawFrame dF;
	FlapsControl flc;

	public OverlayManager overlayManager;

	/**
	 * Gets the OverlayManager instance for overlay z-order coordination.
	 */
	public OverlayManager getOverlayManager() {
		return overlayManager;
	}

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

	/**
	 * Gets the ConfigurationService instance.
	 */
	public ConfigurationService getConfigService() {
		return configService;
	}
	// 存储参数
	// 主参数

	/** Service data polling and calculation interval (ms). Previously named freqService. */
	public long serviceLoopIntervalMs;
	// 发动机面板
	/** Engine info overlay refresh interval (ms). Previously named freqEngineInfo. */
	public long engineInfoIntervalMs;
	/** Flight info overlay refresh interval (ms). Previously named freqFlightInfo. */
	public long flightInfoIntervalMs;
	// 人工地平仪
	/** Altitude/attitude display refresh interval (ms). Previously named freqAltitude. */
	public long altitudeIntervalMs;

	/** Gear and flaps overlay refresh interval (ms). Previously named freqGearAndFlap. */
	public long gearFlapsIntervalMs;

	/** Control surface (stick values) overlay refresh interval (ms). Previously named freqStickValue. */
	public long controlInputIntervalMs;
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

	// Event Handlers
	private java.util.function.Consumer<Object> configChangedHandler;
	private java.util.function.Consumer<Object> uiReadyHandler;

	// Track current FM hotkey binding for rebind on config change
	private int currentFmHotkeyCode = 0;

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
				SB.init(this, configService.getOverlaySettings("StatusBar"));
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
			cur_fmtype = S.sIndic.type;
			identifiedFMName = cur_fmtype;
			// Removed getfmdata call - Service will trigger load via calculate or start
			// Application.debugPrint("状态3，连接成功，释放状态条，打开面板");
			// usetempratureInformation =
			// Boolean.parseBoolean(getconfig("usetempInfoSwitch"));
			// Application.debugPrint(usetempratureInformation);
			// NotificationManager.showNotification(createWebNotificationTime(3000));
			if (showStatus && SB != null) {
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

			// Delay overlay creation to allow data to populate (prevents flash)
			new Thread(() -> {
				try {
					// overlay创建的太快了, 可能有数据闪烁, 小睡一下
					Thread.sleep(100);
				} catch (InterruptedException e) {
					e.printStackTrace();
				}
				// Ensure openpad runs, it handles its own threads/UI
				openpad();
			}).start();

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
			Log.init(this, S, this);
			logon = true;
		}

		// UI Thread (always runs)
		uT = new UIThread(this);
		uT1 = new Thread(uT);
		uT1.setPriority(Thread.MAX_PRIORITY);
		uT1.start();
		if (S != null) {
			S.startTime = System.currentTimeMillis();
		}
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

	@Override
	public boolean isFieldDisabled(String key) {
		return configService.isFieldDisabled(key);
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

	/**
	 * Default constructor - used when restoring MainForm from tray icon.
	 * Does NOT check autoStartGameMode, always shows MainForm.
	 */
	public Controller() {
		this(false);
	}

	/**
	 * Constructor with initial launch flag.
	 * @param isInitialLaunch true if this is the application's initial startup (from main()),
	 *                        false if restoring from tray icon click
	 */
	public Controller(boolean isInitialLaunch) {
		configService = new ConfigurationService();
		configService.initConfig();// 装载设置文件
		// 接收频率
		// Application.debugPrint("controller执行了");
		loadFromConfig();
		initDynamicOverlays();

		// Initialize HotkeyManager and bind FM overlay hotkey
		HotkeyManager.getInstance().init();
		boolean enableFMPrint = Boolean.parseBoolean(getconfig("enableFMPrint"));
		try {
			currentFmHotkeyCode = Integer.parseInt(getconfig("displayFmKey"));
		} catch (NumberFormatException e) {
			currentFmHotkeyCode = NativeKeyEvent.VC_P;
		}
		if (enableFMPrint && currentFmHotkeyCode != 0) {
			HotkeyManager.getInstance().bind(currentFmHotkeyCode, UIStateEvents.FM_OVERLAY_TOGGLE);
		}
		// Keep Application.displayFmKey in sync for backward compatibility
		Application.displayFmKey = currentFmHotkeyCode;

		usetempratureInformation = false;

		// Initialize OverlayManager and register overlays
		overlayManager = new OverlayManager(this);
		registerGameModeOverlays();

		// Listen for live config changes for WYSIWYG
		configChangedHandler = key -> {
			// Check if this is a global reset completed event
			boolean isResetCompleted = prog.event.UIStateEvents.ACTION_RESET_COMPLETED.equals(key);
			// Handle FM hotkey config changes
			if (key instanceof String) {
				String keyStr = (String) key;
				if ("displayFmKey".equals(keyStr) || "enableFMPrint".equals(keyStr)) {
					handleFmHotkeyConfigChange();
				}
			}

			// Only refresh if we are in PREVIEW state.
			// In INIT state (startup), we don't want to trigger FM loads yet.
			if (State == ControllerState.PREVIEW) {
				// prog.util.Logger.info("Controller", "ACTION: Controller: Refreshing Previews
				// (" + key + ")");

				// Offload to background thread to avoid blocking UI/Animation
				new Thread(() -> {
					// Always reload global config first to update Application.colorXXX fields
					loadFromConfig();
					if (isResetCompleted) {
						// Global reset: refresh all overlays
						overlayManager.refreshAllPreviews();
					} else if (key instanceof String) {
						overlayManager.refreshPreviews((String) key);
					} else {
						overlayManager.refreshAllPreviews();
					}
				}).start();
			} else {
				// Just update local config without full refresh/data load
				prog.util.Logger.info("Controller", "ACTION: Controller: Reloading config (" + key + ")");
				loadFromConfig();
				// Also re-init active overlays to reflect config changes (e.g. EngineInfo)
				overlayManager.reinitActiveOverlays();
			}
		};
		prog.event.UIStateBus.getInstance().subscribe(prog.event.UIStateEvents.CONFIG_CHANGED, configChangedHandler);

		// Listen for UI Ready event to start preview
		uiReadyHandler = data -> {
			prog.util.Logger.info("Controller", "ACTION: Controller: UI Ready. Initializing Preview...");
			Preview();
		};
		prog.event.UIStateBus.getInstance().subscribe(prog.event.UIStateEvents.UI_READY, uiReadyHandler);

		// 刷新频率
		State = ControllerState.INIT;
		lastEvt = 0;
		lastDmg = 0;

		// 状态0，初始化主界面和设置文件
		// Application.debugPrint("状态0，初始化主界面");

		// Check for auto-start game mode (only on initial launch, not tray restore)
		boolean autoStart = false;
		if (isInitialLaunch) {
			String autoStartStr = getconfig("autoStartGameMode");
			if (autoStartStr != null && !autoStartStr.isEmpty()) {
				autoStart = Boolean.parseBoolean(autoStartStr);
			}
		}

		if (autoStart) {
			prog.util.Logger.info("Controller", "Auto-start enabled, entering game mode directly...");
			ensureBlkxLoaded();
			start();
		} else {
			M = new MainForm(this);
			M.startRepaintTimer();
			// Check for live aircraft on startup (lazy fallback - only loads if live)
			ensureBlkxLoaded();
		}

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
			// Dispose MainForm if exists (may not exist in auto-start mode)
			if (M != null) {
				M.stopRepaintTimer();
				M.dispose();
				M = null;
			}

			// Suggest GC after disposing the main settings window (significant memory
			// release)
			System.gc();
			// NotificationManager.showNotification(createWebNotification("程序最小化至托盘，注意右上角状态条提示"));

			prog.util.Logger.info("Controller", "--------------------------------------------------");
			prog.util.Logger.info("Controller", "ACTION: Starting Game Mode Services...");
			prog.util.Logger.info("Controller", "--------------------------------------------------");
			S = new Service(this);
			S1 = new Thread(S);
			/* 设置高优先级 */
			S1.setPriority(Thread.MAX_PRIORITY);
			S1.start();

			// Save config when entering game mode
			configService.saveConfig();
			configService.saveLayoutConfig();

		}

	}

	/**
	 * Register all game mode overlays with OverlayManager.
	 * Uses registerWithPreview for overlays that support preview mode.
	 */
	private void registerGameModeOverlays() {

		// EngineControlOverlay - supports preview (fully event-driven)
		overlayManager.registerWithPreview("enableEngineControl",
				() -> new EngineControlOverlay(),
				overlay -> ((EngineControlOverlay) overlay).init(this, S, configService.getOverlaySettings("引擎控制")),
				overlay -> ((EngineControlOverlay) overlay).initPreview(this, configService.getOverlaySettings("引擎控制")),
				overlay -> ((EngineControlOverlay) overlay).reinitConfig(),
				true).withInterest("disableEngineInfo", "fontSize");

		// PowerInfoOverlay (moved from hardcoded to layout config)
		overlayManager.registerWithPreview("engineInfoSwitch",
				() -> new PowerInfoOverlay(),
				overlay -> ((PowerInfoOverlay) overlay).init(this, S, configService.getOverlaySettings("动力信息")),
				overlay -> ((PowerInfoOverlay) overlay).initPreview(this, configService.getOverlaySettings("动力信息")),
				overlay -> ((PowerInfoOverlay) overlay).reinitConfig(),
				true).withInterest("fontName", "fontSize", "columns", "S.");

		// MiniHUDOverlay (crosshair) - supports preview
		overlayManager.registerWithPreview("crosshairSwitch",
				() -> new MiniHUDOverlay(),
				overlay -> ((MiniHUDOverlay) overlay).init(this, S, O),
				overlay -> ((MiniHUDOverlay) overlay).initPreview(this),
				overlay -> ((MiniHUDOverlay) overlay).reinitConfig(),
				false)
				.withInterest("displayCrosshair", "drawHUD", "disableHUD", "crosshair", "miniHUD", "enableLayoutDebug",
						"enableFlapAngleBar", "hudMach", "showSpeedBar", "showAttitudeGauge", "attitudeIndicatorInertialMode",
						"alwaysShowRadarAltitude");

		// FlightInfoOverlay - supports preview
		overlayManager.registerWithPreview("flightInfoSwitch",
				() -> new FlightInfoOverlay(),
				overlay -> ((FlightInfoOverlay) overlay).init(this, S, configService.getOverlaySettings("飞行信息")),
				overlay -> ((FlightInfoOverlay) overlay).initPreview(this, configService.getOverlaySettings("飞行信息")),
				overlay -> ((FlightInfoOverlay) overlay).reinitConfig(),
				true).withInterest("flightInfo", "fontSize", "disableFlightInfo");

		// ControlSurfacesOverlay - supports preview (uses initpreview lowercase)
		overlayManager.registerWithPreview("enableAxis",
				() -> new ControlSurfacesOverlay(),
				overlay -> ((ControlSurfacesOverlay) overlay).init(this, S, configService.getOverlaySettings("舵面值")),
				overlay -> ((ControlSurfacesOverlay) overlay).initPreview(this,
						configService.getOverlaySettings("舵面值")),
				overlay -> ((ControlSurfacesOverlay) overlay).reinitConfig(),
				false).withInterest("enableAxisEdge", "fontSize");

		// AttitudeOverlay - supports preview
		overlayManager.registerWithPreview("enableAttitudeIndicator",
				() -> new AttitudeOverlay(),
				overlay -> ((AttitudeOverlay) overlay).init(this, S, configService.getOverlaySettings("地平仪")),
				overlay -> ((AttitudeOverlay) overlay).initPreview(this, configService.getOverlaySettings("地平仪")),
				overlay -> ((AttitudeOverlay) overlay).reinitConfig(),
				false).withInterest("attitudeIndicator", "enableAttitudeIndicator");

		// GearFlapsOverlay - supports preview
		overlayManager.registerWithPreview("enablegearAndFlaps",
				() -> new GearFlapsOverlay(),
				overlay -> ((GearFlapsOverlay) overlay).init(this, S, configService.getOverlaySettings("起落襟翼")),
				overlay -> ((GearFlapsOverlay) overlay).initPreview(this, configService.getOverlaySettings("起落襟翼")),
				overlay -> ((GearFlapsOverlay) overlay).reinitConfig(),
				false).withInterest("enablegearAndFlapsEdge", "fontSize");

		// VoiceWarning - game mode only, no preview
		overlayManager.registerWithStrategy("enableVoiceWarn",
				() -> new VoiceWarning(),
				overlay -> ((VoiceWarning) overlay).init(this, S),
				null, // No preview initializer
				null, // No re-initializer
				true,
				ActivationStrategy.config("enableVoiceWarn").and(ActivationStrategy.gameModeOnly()));

		// FMUnpackedDataOverlay - per-field toggles via switch items in ui_layout.cfg
		overlayManager.registerWithPreview("enableFMPrint",
				() -> new FMUnpackedDataOverlay(),
				overlay -> {
					fmDataAdapter.setBlkx(getBlkx());
					prog.config.OverlaySettings fmSettings = configService.getOverlaySettings("FM拆包数据");
					((FMUnpackedDataOverlay) overlay).init(this, fmDataAdapter, fmSettings);
				}, overlay -> {
					fmDataAdapter.setBlkx(getBlkx());
					prog.config.OverlaySettings fmSettings = configService.getOverlaySettings("FM拆包数据");
					((FMUnpackedDataOverlay) overlay).initPreview(this, fmDataAdapter, fmSettings);
				},
				overlay -> ((FMUnpackedDataOverlay) overlay).reinitConfig(),
				true).withInterest("displayFmKey", "selectedFM", "fmInfoColumn", "fontName",
					"showWeight", "showCritSpeed", "showGLoadLimits",
					"showFlapLimits", "showControlEffectiveness", "showNitro", "showHeatRecovery",
					"showMaxLiftLoad", "showInertia", "showLift", "showDrag",
					"showNoFlapsWing", "showFullFlapsWing", "showFuselage", "showFin", "showStab");

		// thrustdFS - requires enableFMPrint AND isJet
		overlayManager.registerWithStrategy("thrustdFS",
				() -> new DrawFrameSimpl(),
				overlay -> ((DrawFrameSimpl) overlay).init(this),
				overlay -> ((DrawFrameSimpl) overlay).initPreview(this),
				overlay -> ((DrawFrameSimpl) overlay).reinitConfig(),
				true,
				ActivationStrategy.config("enableFMPrint").and(ActivationStrategy.jetOnly()));
	}

	public void initDynamicOverlays() {
		// Use shared layout config from ConfigurationService
		// Always reload to support ConfigWatcher updates
		configService.loadLayout(prog.config.ConfigManager.getUserConfigPath());
		dynamicConfigs = configService.getLayoutConfigs();
	}


	public void stop() {
		if (M != null) {
			M.stopRepaintTimer();
			M.dispose();
			M = null;
			System.gc();
			// Explicit save on Application Exit (MainForm dispose)
			if (configService != null)
				configService.saveConfig();
			return;
		}

		// if (S1 == null) {
		// return;
		// }
		if (State == ControllerState.PREVIEW) {
			closepad();
		}

		// Unsubscribe from event bus to prevent duplicate handling on restart
		if (configChangedHandler != null) {
			prog.event.UIStateBus.getInstance().unsubscribe(prog.event.UIStateEvents.CONFIG_CHANGED,
					configChangedHandler);
		}
		if (uiReadyHandler != null) {
			prog.event.UIStateBus.getInstance().unsubscribe(prog.event.UIStateEvents.UI_READY, uiReadyHandler);
		}

		S = null;
		if (S1 != null) {
			S1.interrupt();
			S1 = null;
		}
		System.gc();
	}

	/**
	 * Handle changes to FM hotkey configuration (displayFmKey or enableFMPrint).
	 * Unbinds the old hotkey and binds the new one if enabled.
	 */
	private void handleFmHotkeyConfigChange() {
		boolean enableFMPrint = Boolean.parseBoolean(getconfig("enableFMPrint"));
		int newHotkeyCode = 0;
		try {
			newHotkeyCode = Integer.parseInt(getconfig("displayFmKey"));
		} catch (NumberFormatException e) {
			newHotkeyCode = NativeKeyEvent.VC_P;
		}

		// Unbind old hotkey if it was bound
		if (currentFmHotkeyCode != 0) {
			HotkeyManager.getInstance().unbind(currentFmHotkeyCode);
			prog.util.Logger.info("Controller", "Unbound old FM hotkey: " + currentFmHotkeyCode);
		}

		// Bind new hotkey if enabled and valid
		if (enableFMPrint && newHotkeyCode != 0) {
			HotkeyManager.getInstance().bind(newHotkeyCode, UIStateEvents.FM_OVERLAY_TOGGLE);
			prog.util.Logger.info("Controller", "Bound new FM hotkey: " + newHotkeyCode);
		}

		// Update tracked value and Application for backward compatibility
		currentFmHotkeyCode = newHotkeyCode;
		Application.displayFmKey = newHotkeyCode;
	}

	public void Preview() {
		prog.util.Logger.info("Controller", "Enabling Preview mode...");
		State = ControllerState.PREVIEW;
		final long generation = previewGeneration.get();  // Capture current generation
		// Offload I/O to background, similar to config change
		new Thread(() -> {
			refreshPreviews(generation);  // Pass generation for staleness check
		}).start();
	}

	/**
	 * IDENTIFY the current aircraft from live source or config.
	 * Does NOT trigger parsing unless specifically requested via getBlkx().
	 */
	private void ensureBlkxLoaded() {
		// Throttled live polling
		long now = System.currentTimeMillis();
		if (now - lastBlkxCheckTime < BLKX_CHECK_INTERVAL) {
			return;
		}
		lastBlkxCheckTime = now;

		HttpHelper httpDataFetcher = new HttpHelper();
		String livePlaneName = httpDataFetcher.getLiveAircraftType();

		if (livePlaneName != null) {
			identifiedFMName = livePlaneName;
			return;
		}

		// Fallback to config
		String configPlane = getConfig("selectedFM0");
		if (configPlane != null && !configPlane.isEmpty()) {
			identifiedFMName = configPlane;
		}
	}

	/**
	 * Just-In-Time getter for Flight Model data.
	 * Triggers parsing only if target aircraft differs from loaded one.
	 */
	public synchronized Blkx getBlkx() {
		// If we don't even have an identified plane yet, try to find one
		if (identifiedFMName == null) {
			ensureBlkxLoaded();
		}

		if (identifiedFMName == null)
			return null;

		// Skip if already loaded and valid
		if (identifiedFMName.equalsIgnoreCase(loadedFMName) && Blkx != null && Blkx.valid) {
			return Blkx;
		}

		// Trigger actual parsing
		loadFMData(identifiedFMName);
		return Blkx;
	}

	/**
	 * Refresh previews with generation check to detect stale callbacks.
	 * @param generation the generation captured when the refresh was initiated
	 */
	public void refreshPreviews(long generation) {
		prog.util.Logger.debug("Controller", "Refreshing overlays for preview/config change...");
		loadFromConfig();
		ensureBlkxLoaded();
		// Schedule UI update on EDT to prevent race conditions/NPEs
		javax.swing.SwingUtilities.invokeLater(() -> {
			// Check if callback is stale (state changed or generation incremented)
			if (State != ControllerState.PREVIEW || previewGeneration.get() != generation) {
				prog.util.Logger.info("Controller",
					"Skipping stale preview refresh (gen=" + generation +
					", current=" + previewGeneration.get() + ", state=" + State + ")");
				return;
			}
			overlayManager.refreshAllPreviews();
		});
	}

	/**
	 * Refresh previews using current generation.
	 * Used by configChangedHandler for live config updates.
	 */
	public void refreshPreviews() {
		refreshPreviews(previewGeneration.get());
	}

	public void endPreview() {
		prog.util.Logger.info("Controller", "Exiting Preview mode...");
		previewGeneration.incrementAndGet();  // Invalidate any pending preview callbacks
		overlayManager.closeAll();
		// Explicit save when exiting preview
		configService.saveConfig();
		State = ControllerState.INIT;
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

	public void loadFMData(String planename) {
		if (planename == null || planename.isEmpty())
			return;

		// Skip if truly already loaded (double check for safety)
		if (planename.equalsIgnoreCase(loadedFMName) && Blkx != null && Blkx.valid) {
			return;
		}

		prog.util.Logger.info("Controller", "Lazily Loading Flight Model for: " + planename);

		String fmfile = null;
		String planeFileName = planename.toLowerCase();
		Blkx lookupBlkx = new Blkx("./data/aces/gamedata/flightmodels/" + planeFileName + ".Blkx",
				planeFileName + ".blk",
				false);

		// Extract fuel modifications from Central file before discarding it
		// Note: use parser.Blkx to avoid shadowing by the 'Blkx' field
		parser.Blkx.FuelModification fuelMod = null;
		if (lookupBlkx.valid && lookupBlkx.data != null) {
			fuelMod = parser.Blkx.extractFuelModifications(lookupBlkx.data);
			if (fuelMod.type != parser.Blkx.FuelModification.FuelType.NONE) {
				prog.util.Logger.info("Controller", "Fuel modification detected: " + fuelMod.type
						+ " (HP bonus=" + fuelMod.sovietOctaneHpBonus + ")");
			}

			fmfile = lookupBlkx.getlastone("fmfile");
			if (fmfile != null) {
				fmfile = fmfile.substring(fmfile.indexOf("\"") + 1, fmfile.length() - 1);
				if (fmfile.charAt(0) == '/')
					fmfile = fmfile.substring(1);
			}
		}
		if (fmfile == null) {
			fmfile = "fm/" + planeFileName + ".blk";
		}

		if (-1 == fmfile.indexOf(".blk")) {
			fmfile += ".blk";
		}

		// Final parse
		Blkx = new Blkx("./data/aces/gamedata/flightmodels/" + fmfile + "x", fmfile);

		if (Blkx.valid == true) {
			Blkx.getAllplotdata();
			Blkx.finalizeLoading();

			// Extract compressor stages for multi-stage supercharger aircraft
			if (FMPowerExtractor.isPistonEngine(Blkx)) {
				compressorStages = FMPowerExtractor.extractStages(Blkx, fuelMod);
				// Multiply by engineNum for multi-engine aircraft (consistent with jet thrust calculation)
				peakWepPower = prog.util.PistonPowerModel.peakWepPower(compressorStages) * Blkx.engineNum;
				peakThrust = 0;
			} else {
				compressorStages = null;
				peakWepPower = 0;
				peakThrust = Blkx.peakThrust(true);  // Always use afterburner thrust
			}

			// Explicitly trigger GC after loading the massive FM data structures
			System.gc();

			loadedFMName = planename;
			cur_fmtype = planename;

			// Update FM data adapter for FMUnpackedDataOverlay
			fmDataAdapter.setBlkx(Blkx);

			// Notify observers that FM data is ready
			prog.event.UIStateBus.getInstance().publish(prog.event.UIStateEvents.FM_DATA_LOADED, planename);
		}
	}

	/**
	 * Gets the cached compressor stage parameters for the current aircraft.
	 *
	 * @return array of CompressorStageParams, or null if jet/single-stage/no FM loaded
	 */
	public synchronized prog.util.PistonPowerModel.CompressorStageParams[] getCompressorStages() {
		return compressorStages;
	}

	/**
	 * Gets the cached peak WEP power for piston aircraft.
	 * @return peak WEP power in hp, or 0 if jet/no FM loaded
	 */
	public synchronized double getPeakWepPower() {
		return peakWepPower;
	}

	/**
	 * Gets the cached peak afterburner thrust for jet aircraft.
	 * @return peak thrust in kgf, or 0 if piston/no FM loaded
	 */
	public synchronized double getPeakThrust() {
		return peakThrust;
	}

}