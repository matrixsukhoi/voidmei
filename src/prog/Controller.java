package prog;

import prog.i18n.Lang;
import prog.audio.VoiceWarning;
import prog.util.HttpHelper;
import prog.fm.FMManager;

import java.awt.Color;
import java.awt.Font;

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
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.TimeUnit;

public class Controller {

	public ControllerState State = ControllerState.INIT;

	/** Generation counter for detecting stale preview callbacks */
	private final AtomicLong previewGeneration = new AtomicLong(0);

	public boolean logon = false;

	// P2 清理: Blkx/loadedFMName/identifiedFMName/failedFMName 等九个 FM 状态字段
	// 已由 prog.fm.FMManager 单一真相源取代——issue #55 死循环的根源清除

	/** 配置变更防抖：延迟执行器，避免滑块拖动时频繁触发 FM 加载 */
	private ScheduledFuture<?> pendingConfigRefresh;
	/** 单线程定时执行器，daemon 线程随 JVM 退出自动终止 */
	private static final ScheduledExecutorService configDebouncer =
		Executors.newSingleThreadScheduledExecutor(r -> {
			Thread t = new Thread(r, "ConfigDebounce");
			t.setDaemon(true);
			return t;
		});
	/** 防抖延迟毫秒数：200ms 内无新变更才执行 */
	private static final long CONFIG_DEBOUNCE_MS = 200;

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
	/** P2: FM 句柄变化处理器（FMManager 在 FM-Loader 后台线程发布，订阅方须自行避开 EDT 直碰） */
	private java.util.function.Consumer<Object> fmChangedHandler;

	// Track current FM hotkey binding for rebind on config change
	private int currentFmHotkeyCode = 0;

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

	/**
	 * 当前游戏会话的机型名（P4 取代旧 cur_fmtype 的会话级记忆）。
	 * 由 onAircraftChanged() 唯一写入：null = 会话尚未开始（或已随 S4toS1 结束），
	 * 非 null = openpad/换机时的机型。仅用于换机检测的幂等去重，
	 * FM 真相源在 prog.fm.FMManager.currentTargetName()。
	 */
	private String sessionAircraftType = null;

	private AutoMeasure aM;

	private Thread aM1;

	public void changeS3() {
		// 状态3，连接成功，释放状态条，打开面板
		// SB.repaint();
		if (State == ControllerState.IN_GAME) {

			// 自动隐藏任务栏

			// 初始化MapObj以及Msg、gamechat
			// P4: cur_fmtype 已删——机型识别唯一写者收敛到 Service 轮询链路
			//（processPollingCycle 每轮 identify + onAircraftChanged），changeS3 只做首次触发
			// P2: FM 单一真相源 —— 进游戏识别到机型即通知 FMManager 异步加载
			//（identify 高频安全：目标未变零成本；缺失机型走负缓存不再重试）
			FMManager.getInstance().identify(S.sIndic.type);
			// Removed getfmdata call - Service will trigger load via calculate or start
			// Application.debugPrint("状态3，连接成功，释放状态条，打开面板");
			// usetempratureInformation =
			// Boolean.parseBoolean(getConfig("usetempInfoSwitch"));
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
		// P4 起本方法只服务"退出游戏"语义（processPollingCycle 的 sState/sIndic flag 丢失
		// 与 8111 无数据两条路径仍在调用）；换机不再走此重启，改由 onAircraftChanged 轻量 swap
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
			// P4: 会话结束——清除 FMManager 识别目标（刻意保留已加载句柄，重进同机型时
			// identify 走"句柄已在"分支秒开）；同时清会话机型记忆，重进游戏由 openpad
			// 重建 FlightLog，首轮 onAircraftChanged 只记名不换日志
			FMManager.getInstance().clearTarget();
			sessionAircraftType = null;
			State = ControllerState.INIT;

			// 自动显示任务栏
			// hideTaskbarSw();
		}

	}

	/**
	 * 换机轻量 swap（P4）：机型变化时的会话级切换，取代旧版"检测到换机 → S4toS1
	 * 重启整个生命周期（销毁全部 overlay → 下轮 changeS3 重建）"。
	 *
	 * <p>overlay 全部保留（HUD 无闪断）；FM 句柄由轮询侧 FMManager.identify 异步切换
	 * （加载期间 HUD 用旧 FM 平滑过渡，READY 后 FM_CHANGED 驱动刷新）。本方法只做收尾：
	 * FlightLog 关旧开新（复用 closepad）+ 会话变量重置（resetvaria，不从旧机继承）。
	 *
	 * <p>由 Service.processPollingCycle 每轮调用（~10Hz），幂等：机型未变直接返回；
	 * 会话首机只记名不切换。调用线程为 Service 轮询线程，与读 c.Log 的线程一致，无竞态。
	 *
	 * @param newType 新机型名（sIndic.type，Service 已 update 完，即当前真实机型）
	 */
	public void onAircraftChanged(String newType) {
		if (newType == null || newType.isEmpty())
			return;
		// 幂等守卫：同机型零成本返回（Service 轮询高频调用）
		if (newType.equals(sessionAircraftType))
			return;
		// null = 会话首机：openpad 已按当前机型建好 FlightLog，此处只记名，不做任何切换
		boolean isSwitch = sessionAircraftType != null;
		sessionAircraftType = newType;
		if (!isSwitch)
			return;

		prog.util.Logger.info("Controller",
				"Aircraft type changed to: " + newType + ". Lightweight FM swap (no Controller restart).");

		if (Boolean.parseBoolean(configService.getConfig("enableLogging"))) {
			// 关掉上一机遗留的爬升曲线窗口（复用 openpad 对旧 dF 的清理方式）
			if (dF != null) {
				dF.doit = false;
				dF = null;
			}
			// FlightLog 关旧开新 —— 收尾逻辑复用 closepad：保存通知 + 爬升档数≥1 弹 DrawFrame
			if (Log != null) {
				ui.util.NotificationService.show(Lang.cSavelog + Log.fileName + Lang.cPlsopen);
				// fA 可能为 null（旧机全程未触发高度分析），防护避免 NPE 中断换机流程
				if (Log.fA != null && Log.fA.curaltStage - Log.fA.initaltStage >= 1) {
					dF = new DrawFrame();
					showdrawFrame(Log.fA);
				}
				Log.close();
				Log = null;
			}
			// 按新机型新建：FlightLog.init 内部用 s.sIndic.type 命名 records/<TYPE>_日期.csv
			Log = new FlightLog();
			Log.init(this, S, configService);
			logon = true;
		}

		// 会话变量重置：燃油/能量累计等不从旧机继承（与加油检测共用同一入口）。
		// 此刻新 FM 可能仍在异步加载（resetEngLoad 有 hasFM 守卫），新句柄 READY 后
		// 引擎耐久由 Blkx 解析时的初始化保证满值，无脏数据
		if (S != null) {
			S.resetvaria();
		}
	}

	public void openpad() {
		// Special case: AutoMeasure (debug only)
		if (Application.fmTesting) {
			aM = new AutoMeasure(S);
			aM1 = new Thread(aM);
			aM1.start();
		}

		// 启用游戏失焦时自动隐藏overlay功能（如果配置开启）
		String autoHideStr = configService.getConfig("autoHideOnFocusLoss");
		prog.util.Logger.info("Controller", "autoHideOnFocusLoss 配置值: " + autoHideStr);
		if (autoHideStr != null && Boolean.parseBoolean(autoHideStr)) {
			S.getFocusMonitor().setEnabled(true);
			prog.util.Logger.info("Controller", "焦点监控已启用");
		} else {
			prog.util.Logger.info("Controller", "焦点监控未启用（配置为 false 或未设置）");
		}

		// Open all registered overlays via OverlayManager
		overlayManager.openAll();

		// Special case: FlightLog (has notification and special init)
		if (Boolean.parseBoolean(configService.getConfig("enableLogging"))) {
			if (dF != null) {
				dF.doit = false;
				dF = null;
			}
			ui.util.NotificationService.show(Lang.cStartlog);
			Log = new FlightLog();
			// 使用 configService 作为 ConfigProvider，而不是 Controller (this)
			Log.init(this, S, configService);
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
		// 禁用焦点监控（会自动恢复被隐藏的overlay）
		S.getFocusMonitor().setEnabled(false);

		// Special case: AutoMeasure
		if (Application.fmTesting && aM != null) {
			aM.doit = false;
			aM1 = null;
			aM = null;
		}

		// Close all managed overlays via OverlayManager
		overlayManager.closeAll();

		// Special case: FlightLog (has notification and DrawFrame logic)
		if (Boolean.parseBoolean(configService.getConfig("enableLogging")) && (Log != null)) {
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

	/**
	 * 获取配置提供者接口，供需要访问配置的组件使用。
	 * 这允许组件依赖 ConfigProvider 接口而不是 Controller 类。
	 *
	 * @return ConfigProvider 接口，实际由 ConfigurationService 实现
	 */
	public ConfigProvider getConfigProvider() {
		return configService;
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
		String statusBarConfig = configService.getConfig("enableStatusBar");
		if (statusBarConfig != null && !statusBarConfig.isEmpty())
			showStatus = Boolean.parseBoolean(statusBarConfig);
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
		boolean enableFMPrint = Boolean.parseBoolean(configService.getConfig("enableFMPrint"));
		try {
			currentFmHotkeyCode = Integer.parseInt(configService.getConfig("displayFmKey"));
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
			// 导入/重置配置后也需要更新热键绑定，因为此时 key 是 ACTION_RESET_COMPLETED
			// 而不是具体的配置项名称，所以上面的 if 不会触发
			if (isResetCompleted) {
				handleFmHotkeyConfigChange();
			}

			// Only refresh if we are in PREVIEW state.
			// In INIT state (startup), we don't want to trigger FM loads yet.
			if (State == ControllerState.PREVIEW) {
				// prog.util.Logger.info("Controller", "ACTION: Controller: Refreshing Previews
				// (" + key + ")");

				// 防抖处理：取消之前未执行的任务，延迟执行以避免滑块拖动时频繁触发
				// 只有 200ms 安静期内的最后一次变更会被执行
				if (pendingConfigRefresh != null && !pendingConfigRefresh.isDone()) {
					pendingConfigRefresh.cancel(false);
				}
				pendingConfigRefresh = configDebouncer.schedule(() -> {
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
				}, CONFIG_DEBOUNCE_MS, TimeUnit.MILLISECONDS);
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

		// P2: 订阅 FM 句柄变化 —— FMManager 加载落定（READY/MISSING/CORRUPT）后
		// 刷新全部预览, 让 overlay 通过 getBlkx() 拿到新 FM。
		// 发布线程为 FM-Loader 后台线程, 这里只做防抖排队, 实际刷新在防抖线程/EDT 执行
		fmChangedHandler = data -> {
			// FM 缺失/损坏: 右下角 toast 告知用户 (换机才广播一次, 天然不刷屏;
			// 负缓存保证后续 identify 同名机型零重复, 无需额外冷却)
			if (data instanceof prog.fm.FMHandle) {
				prog.fm.FMHandle h = (prog.fm.FMHandle) data;
				if (h.isMissingLike()) {
					String msg = h.status == prog.fm.FMStatus.CORRUPT
							? prog.i18n.Lang.fmCorruptToast : prog.i18n.Lang.fmMissingToast;
					ui.util.NotificationService.showBottomRight(h.name + "\n" + msg, 5000);
				}
			}
			if (State == ControllerState.PREVIEW) {
				// 复用 configDebouncer 200ms 防抖: 连续换机/identify 抖动时只刷一次
				if (pendingConfigRefresh != null && !pendingConfigRefresh.isDone()) {
					pendingConfigRefresh.cancel(false);
				}
				pendingConfigRefresh = configDebouncer.schedule(() -> {
					loadFromConfig();
					overlayManager.refreshAllPreviews();
				}, CONFIG_DEBOUNCE_MS, TimeUnit.MILLISECONDS);
			}
		};
		prog.event.UIStateBus.getInstance().subscribe(prog.event.UIStateEvents.FM_CHANGED, fmChangedHandler);

		// 刷新频率
		State = ControllerState.INIT;
		lastEvt = 0;
		lastDmg = 0;

		// 状态0，初始化主界面和设置文件
		// Application.debugPrint("状态0，初始化主界面");

		// Check for auto-start game mode (only on initial launch, not tray restore)
		boolean autoStart = false;
		if (isInitialLaunch) {
			String autoStartStr = configService.getConfig("autoStartGameMode");
			if (autoStartStr != null && !autoStartStr.isEmpty()) {
				autoStart = Boolean.parseBoolean(autoStartStr);
			}
		}

		if (autoStart) {
			prog.util.Logger.info("Controller", "Auto-start enabled, entering game mode directly...");
			// P2: FM 识别改由 FMManager 异步承担——探测放后台线程, 不阻塞构造/start
			new Thread(this::detectAndIdentify, "FM-Detect").start();
			start();
		} else {
			M = new MainForm(this);
			M.startRepaintTimer();
			// Check for live aircraft on startup (lazy fallback - only loads if live)
			// P2: 同上, 网络探测放后台线程, 避免 MainForm 弹出被 8111 超时拖慢
			new Thread(this::detectAndIdentify, "FM-Detect").start();
		}
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
				true).withInterest("fontName", "fontSize", "hudColumns", "S.");

		// MiniHUDOverlay (crosshair) - supports preview
		// HUDSettings 直接传入 init()，不通过 Controller 获取 configService（遵循解耦原则）
		overlayManager.registerWithPreview("crosshairSwitch",
				() -> new MiniHUDOverlay(),
				overlay -> ((MiniHUDOverlay) overlay).init(this, S, configService.getHUDSettings()),
				overlay -> ((MiniHUDOverlay) overlay).initPreview(this, configService.getHUDSettings()),
				overlay -> ((MiniHUDOverlay) overlay).reinitConfig(),
				false)
				.withInterest("displayCrosshair", "drawHUD", "disableHUD", "crosshair", "miniHUD", "enableLayoutDebug",
						"enableFlapAngleBar", "hudMach", "showSpeedBar", "showAttitudeGauge", "attitudeIndicatorInertialMode",
						"alwaysShowRadarAltitude", "showHUD");

		// FlightInfoOverlay - supports preview
		overlayManager.registerWithPreview("flightInfoSwitch",
				() -> new FlightInfoOverlay(),
				overlay -> ((FlightInfoOverlay) overlay).init(this, S, configService.getOverlaySettings("飞行信息")),
				overlay -> ((FlightInfoOverlay) overlay).initPreview(this, configService.getOverlaySettings("飞行信息")),
				overlay -> ((FlightInfoOverlay) overlay).reinitConfig(),
				true).withInterest("flightInfo", "fontSize", "disableFlightInfo");

		// ControlSurfacesOverlay - supports preview
		// Controller 参数已移除，此 overlay 不需要访问配置
		overlayManager.registerWithPreview("enableAxis",
				() -> new ControlSurfacesOverlay(),
				overlay -> ((ControlSurfacesOverlay) overlay).init(S, configService.getOverlaySettings("舵面值")),
				overlay -> ((ControlSurfacesOverlay) overlay).initPreview(
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
		// Controller 参数已移除，此 overlay 不需要访问配置
		overlayManager.registerWithPreview("enablegearAndFlaps",
				() -> new GearFlapsOverlay(),
				overlay -> ((GearFlapsOverlay) overlay).init(S, configService.getOverlaySettings("起落襟翼")),
				overlay -> ((GearFlapsOverlay) overlay).initPreview(configService.getOverlaySettings("起落襟翼")),
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
					// 直读 FMManager 句柄（P5 收尾: 桥接方法已删, 与各 overlay 的直读模式一致）
					fmDataAdapter.setBlkx(FMManager.getInstance().current().blkx);
					prog.config.OverlaySettings fmSettings = configService.getOverlaySettings("FM拆包数据");
					((FMUnpackedDataOverlay) overlay).init(this, fmDataAdapter, fmSettings);
				}, overlay -> {
					fmDataAdapter.setBlkx(FMManager.getInstance().current().blkx);
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
		// 1. 先关闭所有overlay（预览模式或游戏模式）
		//    必须在dispose MainForm之前执行，确保overlay被正确清理
		//    修复：任务栏图标点击导致Overlay叠加问题
		if (State == ControllerState.PREVIEW) {
			// 使所有pending的stale回调失效
			previewGeneration.incrementAndGet();

			// 根据当前状态选择清理路径
			if (S != null) {
				// 游戏模式：使用closepad()完整清理（包含FocusMonitor、FlightLog等）
				closepad();
			} else {
				// 预览模式：只需关闭overlay
				overlayManager.closeAll();
			}
		}

		// 2. 取消事件订阅（防止重启时重复处理）
		if (configChangedHandler != null) {
			prog.event.UIStateBus.getInstance().unsubscribe(prog.event.UIStateEvents.CONFIG_CHANGED,
					configChangedHandler);
			configChangedHandler = null;
		}
		if (uiReadyHandler != null) {
			prog.event.UIStateBus.getInstance().unsubscribe(prog.event.UIStateEvents.UI_READY, uiReadyHandler);
			uiReadyHandler = null;
		}
		// P2: 退订 FM 句柄变化（防止重启时重复刷新）
		if (fmChangedHandler != null) {
			prog.event.UIStateBus.getInstance().unsubscribe(prog.event.UIStateEvents.FM_CHANGED, fmChangedHandler);
			fmChangedHandler = null;
		}

		// 3. 清理MainForm
		if (M != null) {
			M.stopRepaintTimer();
			M.dispose();
			M = null;
		}

		// 4. 清理Service线程
		S = null;
		if (S1 != null) {
			S1.interrupt();
			S1 = null;
		}

		// 5. 保存配置
		if (configService != null) {
			configService.saveConfig();
		}

		System.gc();
	}

	/**
	 * Handle changes to FM hotkey configuration (displayFmKey or enableFMPrint).
	 * Unbinds the old hotkey and binds the new one if enabled.
	 */
	private void handleFmHotkeyConfigChange() {
		boolean enableFMPrint = Boolean.parseBoolean(configService.getConfig("enableFMPrint"));
		int newHotkeyCode = 0;
		try {
			newHotkeyCode = Integer.parseInt(configService.getConfig("displayFmKey"));
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
	 * 后台一次性探测当前机型并驱动 FMManager 识别加载（P2 桥接）。
	 * live 数据（8111 /indicators）优先，未进游戏时回退 selectedFM0 配置的默认机。
	 * 网络探测有阻塞风险，只在后台线程调用（构造器起 FM-Detect 线程 / refreshPreviews
	 * 所在的防抖线程）。
	 */
	private void detectAndIdentify() {
		// getLiveAircraftType 自带异常兜底（失败/无游戏返回 null）
		HttpHelper httpDataFetcher = new HttpHelper();
		String livePlaneName = httpDataFetcher.getLiveAircraftType();
		String target = livePlaneName;
		if (target == null) {
			// Fallback to config
			target = configService.getConfig("selectedFM0");
		}
		if (target != null && !target.isEmpty()) {
			FMManager.getInstance().identify(target);
		}
	}

	// P5 收尾: getBlkx() 桥接方法已删 —— FM 数据统一经 FMManager.getInstance().current() 直读

	/**
	 * Refresh previews with generation check to detect stale callbacks.
	 * @param generation the generation captured when the refresh was initiated
	 */
	public void refreshPreviews(long generation) {
		prog.util.Logger.debug("Controller", "Refreshing overlays for preview/config change...");
		loadFromConfig();
		// P2: 机型识别（live 优先, selectedFM0 兜底）改走 FMManager；
		// identify 目标未变时零成本，此处同步调用与旧 ensureBlkxLoaded 行为等价
		detectAndIdentify();
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

	// P2: loadFMData(String) 已整体移除——解析迁移至 FMLoader，调度/负缓存/FM_CHANGED
	// 广播由 FMManager 承担（差异点见 FMLoader 类注释）
	// P5: getBlkx()/getCompressorStages() 等桥接方法已删——统一改读 FMManager.current()

}