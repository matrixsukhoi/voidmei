package prog;

import prog.i18n.Lang;
import prog.audio.VoiceWarning;

import java.awt.Color;

import parser.Blkx;
import parser.AttributePool;
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

public class Controller implements ConfigProvider {

	public volatile ControllerState State = ControllerState.INIT;

	public boolean logon = false;

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

	public int lastEvt;
	public int lastDmg;
	public int step;

	private boolean showStatus;

	// Event Handlers
	private java.util.function.Consumer<Object> configChangedHandler;
	private java.util.function.Consumer<Object> uiReadyHandler;

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

	public String cur_fmtype;
	private AutoMeasure aM;
	private Thread aM1;

	public void onGameStatusChanged(prog.event.GameStatusEvent event) {
		prog.event.GameStatusEvent.Status status = event.getStatus();
		if (status == prog.event.GameStatusEvent.Status.CONNECTED) {
			// 状态2，状态条连接成功，等待进入游戏
			if (State == ControllerState.FLYING || State == ControllerState.PREVIEW) {
				// 退出战局或结束预览
				closepad();
				State = ControllerState.CONNECTED;
			} else if (State == ControllerState.CONNECTED || State == ControllerState.INIT) {
				if (showStatus && SB != null)
					SB.S2();
				State = ControllerState.CONNECTED; // Let's correct this semantic. "CONNECTED" means 8111 ok.
			}
		} else if (status == prog.event.GameStatusEvent.Status.IN_GAME) {
			// 状态3，连接成功，释放状态条，打开面板
			// if (State == ControllerState.IN_GAME) { // Original check

			// Initialize MapObj, etc.
			cur_fmtype = S.sIndic.type;

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
			State = ControllerState.FLYING;

			// Open panels directly
			openpad();
		} else if (status == prog.event.GameStatusEvent.Status.INIT) {
			// S4toS1 Logic: Game returned / Disconnected
			if (State == ControllerState.FLYING || State == ControllerState.PREVIEW) {
				closepad();
			}
			if (Application.debug && O != null) {
				lastEvt = O.lastEvt;
				lastDmg = O.lastDmg;
				O.close();
				O = null;
				O1 = null;
			}
			S.clear();
			State = ControllerState.INIT;
		}
	}

	// Legacy stubs to prevent build errors if others call them (though Service was
	// updated)
	public void changeS2() {
		onGameStatusChanged(new prog.event.GameStatusEvent(prog.event.GameStatusEvent.Status.CONNECTED));
	}

	public void changeS3() {
		onGameStatusChanged(new prog.event.GameStatusEvent(prog.event.GameStatusEvent.Status.IN_GAME));
	}

	public void S4toS1() {
		onGameStatusChanged(new prog.event.GameStatusEvent(prog.event.GameStatusEvent.Status.INIT));
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

	public Controller() {
		configService = new ConfigurationService();
		fmService = new prog.service.FlightModelService(this);
		configService.initConfig();// 装载设置文件
		// 接收频率
		// Application.debugPrint("controller执行了");
		loadFromConfig();
		initDynamicOverlays();
		registerHotkeyListener();

		// Initialize OverlayManager and register overlays
		overlayManager = new OverlayManager(this);
		registerGameModeOverlays();

		// Listen for live config changes for WYSIWYG
		configChangedHandler = key -> {
			// Only refresh if we are in PREVIEW state.
			// In INIT state (startup), we don't want to trigger FM loads yet.
			if (State == ControllerState.PREVIEW) {
				// prog.util.Logger.info("Controller", "ACTION: Controller: Refreshing Previews
				// (" + key + ")");

				// Offload to background thread to avoid blocking UI/Animation
				new Thread(() -> {
					if (key instanceof String) {
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
			// Ensure overlays are visible if we started in preview mode
			setDynamicOverlaysVisible(true, false);
		};
		prog.event.UIStateBus.getInstance().subscribe(prog.event.UIStateEvents.UI_READY, uiReadyHandler);

		// Event-Driven State Machine
		prog.event.UIStateBus.getInstance().subscribe(prog.event.UIStateEvents.GAME_STATUS, e -> {
			if (e instanceof prog.event.GameStatusEvent) {
				onGameStatusChanged((prog.event.GameStatusEvent) e);
			}
		});

		// 刷新频率
		State = ControllerState.INIT;
		lastEvt = 0;
		lastDmg = 0;

		// 状态0，初始化主界面和设置文件
		// Application.debugPrint("状态0，初始化主界面");

		M = new MainForm(this);
		M.startRepaintTimer();

		// Check for live aircraft on startup (lazy fallback - only loads if live)
		ensureBlkxLoaded();

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
						"enableFlapAngleBar", "hudMach");

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

		// UsefulData (FMUnpackedDataOverlay) - supports preview
		overlayManager.registerWithPreview("enableFMPrint",
				() -> new FMUnpackedDataOverlay(),
				overlay -> {
					prog.config.OverlaySettings fmSettings = configService.getOverlaySettings("FM拆包数据");
					((FMUnpackedDataOverlay) overlay).init(this, fmSettings);
				}, overlay -> {
					prog.config.OverlaySettings fmSettings = configService.getOverlaySettings("FM拆包数据");
					((FMUnpackedDataOverlay) overlay).initPreview(this, fmSettings);
				}, null, // No reConfig needed
				true).withInterest("displayFmKey", "selectedFM");

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
		// Temporarily disabled - DynamicOverlay removed
		// Clean up existing
		// for (ui.overlay.DynamicOverlay overlay : dynamicOverlays) {
		// overlay.doit = false;
		// overlay.dispose();
		// }
		// dynamicOverlays.clear();

		// dynamicOverlays.clear();

		// Use shared layout config from ConfigurationService
		// Always reload to support ConfigWatcher updates
		configService.loadLayout("ui_layout.cfg");
		dynamicConfigs = configService.getLayoutConfigs();
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
						// int code = e.getKeyCode();
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
			// Explicit save on Application Exit (MainForm dispose)
			if (configService != null)
				configService.saveConfig();
			return;
		}

		// if (S1 == null) {
		// return;
		// }
		if (State == ControllerState.FLYING || State == ControllerState.PREVIEW) {
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

	public void Preview() {
		prog.util.Logger.info("Controller", "Enabling Preview mode...");
		State = ControllerState.PREVIEW;
		// Offload I/O to background, similar to config change
		new Thread(() -> {
			refreshPreviews();
		}).start();
	}

	// --- Flight Model Service Delegation ---
	private final prog.service.FlightModelService fmService;

	public Blkx getBlkx() {
		return fmService.getBlkx();
	}

	public void ensureBlkxLoaded() {
		fmService.ensureBlkxLoaded();
	}

	public void loadFMData(String planename) {
		fmService.loadFMData(planename);
	}

	public AttributePool globalPool() {
		return fmService.getGlobalPool();
	}

	public void refreshPreviews() {
		prog.util.Logger.debug("Controller", "Refreshing overlays for preview/config change...");
		loadFromConfig();
		ensureBlkxLoaded();
		// Schedule UI update on EDT to prevent race conditions/NPEs
		javax.swing.SwingUtilities.invokeLater(() -> {
			overlayManager.refreshAllPreviews();
		});
	}

	public void endPreview() {
		prog.util.Logger.info("Controller", "Exiting Preview mode...");
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

}