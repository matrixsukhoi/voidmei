//! Controller — Java src/prog/Controller.java 的生命周期核 (主线程独占)。
//! 重构波2 自 app_shell.rs 拆出。

use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use vm_core::base::bus::Subscription;
use vm_core::config::config_api::ConfigProvider;
use vm_core::config::configuration_service::ConfigurationService;
use crate::controller_state::ControllerState;
use vm_core::base::event::flight_data_event::FlightDataEvent;
use vm_core::base::event::ui_state_events;
use vm_core::base::bus::flight_data_bus::FlightDataBus;
use vm_core::derived::flight_log::FlightLogSlot;
use vm_core::fm::{FMManager, FMStatus};
use vm_core::telemetry::http::HttpHelper;
use vm_core::lang::Lang;
use vm_core::base::logger;
use vm_core::base::bus::ui_state_bus::UIStateBus;
use vm_core::audio::voice_resource_manager::VoiceResourceManager;

use vm_data::service_loop::{
    flight_log_snapshot, start as spawn_service_thread, Service, ServiceAnalyzerSource,
    ServiceConfig, ServiceHandle,
};

use vm_overlay::platform::hotkey::{HotkeyManager, VC_P};

use crate::commands::{MainEvent, UiCommand};
use crate::controller_shared::{ControllerShared, FLIGHT_SILENT_EXIT_MS};
use crate::env::{current_time_millis, java_parse_boolean, Env};
use crate::voice_setup::SnapshotConfigProvider;
use crate::win32::ChannelFocusBridge;

// =====================================================================
// FlightLog 接线辅助 (Controller.java:366-376/402-411 的依赖注入面)
// =====================================================================

/// Java `Controller.logon` 布尔 (Controller.java:44) 的写入面对位: 唯一写点 =
/// FlightLog.init 失败路径的 `xc.logon = false` (FlightLog.java:409), 语义为
/// "停 tick" — Rust 以清槽表达 (槽 None ⇒ Service 轮询 logTick 短路)。
/// true 分支无 Java 写点, 空实现。
struct LogonSink(FlightLogSlot);
impl vm_core::derived::flight_log::ControllerLogSink for LogonSink {
    fn set_logon(&self, logon: bool) {
        if !logon {
            *self.0.lock().expect("flight_log 槽锁中毒") = None;
        }
    }
}

/// Controller 构造依赖 (AppShell 分发; 对位 Java 构造器从单例/静态取的全部输入)
pub struct ControllerDeps {
    pub config: ConfigurationService,
    pub ui_bus: Arc<UIStateBus>,
    pub flight_bus: Arc<FlightDataBus>,
    pub fm: Arc<FMManager>,
    pub hotkey: Arc<Mutex<HotkeyManager>>,
    /// 共享语音资源管理器 (Java VoiceResourceManager.getInstance(); loadFromConfig
    /// 的音量同步写点在 Controller 内 — 见 load_from_config)
    pub voice: Arc<VoiceResourceManager>,
    pub shared: Arc<ControllerShared>,
    pub ui_cmd_tx: Sender<UiCommand>,
    pub main_event_tx: Sender<MainEvent>,
    pub env: Env,
    /// 8111 live 机型网络探测开关 (生产 true)。
    /// PORT(测试注入面): get_live_aircraft_type 硬编码 127.0.0.1:8111 (Java 保真),
    /// 单测环境该端口可能被 mock/游戏占用 (项目惯例: 端口占用即跳过/隔离),
    /// 测试置 false 使 FM-Detect/Preview 刷新只走 selectedFM0 兜底, 不触网。
    pub probe_network: bool,
}

/// 可重建应用核 (Java Controller; 恒留主线程 — config 字段 !Send)
pub struct Controller {
    pub config: ConfigurationService,
    shared: Arc<ControllerShared>,
    fm: Arc<FMManager>,
    flight_bus: Arc<FlightDataBus>,
    hotkey: Arc<Mutex<HotkeyManager>>,
    /// 共享语音资源管理器 (AppShell 分发; load_from_config 的音量同步面)
    voice: Arc<VoiceResourceManager>,
    ui_cmd_tx: Sender<UiCommand>,
    env: Env,
    /// stop 步2 退订的订阅句柄 (RAII Drop = unsubscribe, 对位 Java unsubscribe+置 null)
    subs: Vec<Subscription<vm_core::base::bus::ui_state_bus::UiStateEvent>>,
    fm_sub: Option<Subscription<vm_core::fm::FMHandle>>,
    /// live 事件活跃度订阅 (B1 补偿信号, 见 ControllerShared.last_flight_event_ms;
    /// start 建 / stop 退 — 回调在 Service 发布线程, 只写原子时间戳不碰 UI)
    live_sub: Option<Subscription<FlightDataEvent>>,
    /// FlightLog 共享槽 (Java Controller.java:44 `logon` + `Log` 字段二位一体的
    /// 收敛形态): openpad/closepad/换机换入换出, Service 轮询线程每轮 logTick
    /// (Service.java:1824-1828)。随核销毁 (stop 的 closepad 路径保存)
    pub(crate) flight_log: FlightLogSlot,
    /// Service 线程句柄 (stop 步4: take + stop)
    pub service: Option<ServiceHandle>,
    /// Java `public MainForm M` 的存活位 (真窗归主线程 iced/W2; 此处只承载 null 判定)
    pub(crate) main_form_alive: bool,
    /// 网络探测开关 (ControllerDeps.probe_network, 测试注入面)
    probe_network: bool,
}

impl Controller {
    /// Java Controller(boolean isInitialLaunch) 构造器 (Controller.java:469-610)。
    /// PORT(侧序): configService.initConfig() 与 initDynamicOverlays 的文件装载
    /// 挪至 AppShell 构造面 (配置服务先于 Controller 存在, 免测试写盘副作用);
    /// overlayManager 注册挪至 win32 线程一次性注册 (host 跨重建存活, 条目为
    /// 无状态配置记录 — 见 register_live_overlays 头注)。
    pub fn new(deps: ControllerDeps, is_initial_launch: bool) -> Controller {
        let ControllerDeps {
            config,
            ui_bus,
            flight_bus,
            fm,
            hotkey,
            voice,
            shared,
            ui_cmd_tx,
            main_event_tx,
            env,
            probe_network,
        } = deps;

        load_from_config(&config, &shared, &voice);

        if let Ok(mut hm) = hotkey.lock() {
            if let Err(e) = hm.init() {
                // Java init 内部 catch 记日志不中断构造
                logger::error("HotkeyManager", &format!("init 失败: {}", e));
            }
        }
        let mut c = Controller {
            config,
            shared: Arc::clone(&shared),
            fm: Arc::clone(&fm),
            flight_bus,
            hotkey: Arc::clone(&hotkey),
            voice,
            ui_cmd_tx,
            env,
            subs: Vec::new(),
            fm_sub: None,
            live_sub: None,
            flight_log: Arc::new(Mutex::new(None)),
            service: None,
            main_form_alive: false,
            probe_network,
        };
        c.bind_fm_hotkey_initial();

        // (订阅回调只做转发到监督通道, 实际处理在主线程
        // AppShell::handle_main_event, 对位 Java handler 内联执行的语义)
        // 重构波1: 路由总线按 event_type 精确订阅 (原桩总线广播+手工过滤退役)
        let tx = main_event_tx.clone();
        c.subs.push(ui_bus.subscribe(
            ui_state_events::CONFIG_CHANGED,
            move |ev: &vm_core::base::bus::ui_state_bus::UiStateEvent| {
                let _ = tx.send(MainEvent::ConfigChanged(ev.data.clone().unwrap_or_default()));
            },
        ));
        let tx = main_event_tx.clone();
        c.subs.push(ui_bus.subscribe(
            ui_state_events::UI_READY,
            move |_ev: &vm_core::base::bus::ui_state_bus::UiStateEvent| {
                let _ = tx.send(MainEvent::UiReady);
            },
        ));
        let tx = main_event_tx.clone();
        c.fm_sub = Some(fm.fm_changed_bus().subscribe(move |handle| {
            // 摘要转发由主线程记日志 + Preview 刷新调度; 缺失/损坏的 toast 面由
            // vm-webui bridge_fm_changed 直连同一总线 (main.rs 接线)
            if handle.is_missing_like() {
                let _ = tx.send(MainEvent::FmChanged {
                    name: handle.name.clone(),
                    corrupt: handle.status == FMStatus::Corrupt,
                });
            }
            // PREVIEW 判定收敛到主线程 handle_main_event (状态真值在主线程)
            let _ = tx.send(MainEvent::FmChanged {
                name: None,
                corrupt: false,
            });
        }));


        let auto_start = if is_initial_launch {
            java_parse_boolean(&c.config.get_config("autoStartGameMode").unwrap_or_default())
        } else {
            false
        };
        if auto_start {
            logger::info(
                "Controller",
                "Auto-start enabled, entering game mode directly...",
            );
            c.spawn_fm_detect();
            c.start(&mut || {}); // M 恒 null (自启动路径), 释放步空转
        } else {
            c.main_form_alive = true;
            c.spawn_fm_detect();
        }
        c
    }

    /// Java:479-489 构造器内的 FM 热键绑定
    fn bind_fm_hotkey_initial(&mut self) {
        let enable =
            java_parse_boolean(&self.config.get_config("enableFMPrint").unwrap_or_default());
        let code = self
            .config
            .get_config("displayFmKey")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(VC_P);
        if let Ok(hm) = self.hotkey.lock() {
            if enable && code != 0 {
                hm.bind(code, ui_state_events::FM_OVERLAY_TOGGLE);
            }
        }
        self.shared
            .flags
            .lock()
            .expect("flags 锁中毒")
            .current_fm_hotkey_code = code;
        // load_app_check 自配置维护, 不双写
    }

    /// Java:601/608 "FM-Detect" 一次性线程 → detectAndIdentify (865-877)。
    /// PORT: selectedFM0 在主线程预读 (配置 !Send 不入线程); HttpHelper/FMManager Send。
    fn spawn_fm_detect(&self) {
        let selected = self.config.get_config("selectedFM0").unwrap_or_default();
        let http_header = self.env.http_header.clone();
        let fm = Arc::clone(&self.fm);
        let probe = self.probe_network;
        std::thread::Builder::new()
            .name("FM-Detect".to_string())
            .spawn(move || detect_and_identify(&selected, &http_header, &fm, probe))
            .expect("FM-Detect 线程创建失败");
    }

    /// Java:447-454 loadFromConfig — loadAppCheck + showStatus 同步
    pub(crate) fn load_from_config_(&self) {
        load_from_config(&self.config, &self.shared, &self.voice);
    }

    /// Java:823-847 handleFmHotkeyConfigChange — 解绑旧键/绑新键
    pub(crate) fn handle_fm_hotkey_config_change(&self) {
        let enable =
            java_parse_boolean(&self.config.get_config("enableFMPrint").unwrap_or_default());
        let new_code = self
            .config
            .get_config("displayFmKey")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(VC_P);
        if let Ok(hm) = self.hotkey.lock() {
            let mut flags = self.shared.flags.lock().expect("flags 锁中毒");
            if flags.current_fm_hotkey_code != 0 {
                hm.unbind(flags.current_fm_hotkey_code);
                logger::info(
                    "Controller",
                    &format!(
                        "Unbound old FM hotkey: {}",
                        flags.current_fm_hotkey_code
                    ),
                );
            }
            if enable && new_code != 0 {
                hm.bind(new_code, ui_state_events::FM_OVERLAY_TOGGLE);
                logger::info("Controller", &format!("Bound new FM hotkey: {}", new_code));
            }
            flags.current_fm_hotkey_code = new_code;
        }
    }

    // ------------------------------------------------------------------
    // 模式切换 (状态机核心)
    // ------------------------------------------------------------------

    /// Java:612-645 start() — INIT 守卫; 释放设置窗 → 起 Service → 存配置。
    /// `release_main_form`: 步"释放设置窗"注入点 (M.stopRepaintTimer+dispose 的
    /// 主线程对位物, AppShell 传入真窗释放闭包)。
    pub fn start(&mut self, release_main_form: &mut dyn FnMut()) {
        if self.shared.state() != ControllerState::Init {
            return;
        }
        // PORT(叠加态守卫, 审查 A-W6): 托盘 Start (Rust 多出面, 见 TrayCommand::Start)
        // 起 Service 后 State 仍 Init, 用户再点 MainForm 确认时 confirm 链的
        // end_preview 无条件置 Init — 仅 Java 守卫拦不住二次 start, 会二次 spawn
        // Service (旧句柄 Drop 兜底 = 会话重启中断)。Java 无托盘 Start 入口故无
        // 此形态; 此处以"Service 已在跑"为幂等条件丢弃, 保留首次会话。
        if self.service.is_some() {
            logger::info("Controller", "Service 已在运行, 忽略重复 start (托盘 Start 与确认叠加)");
            return;
        }
        if self.main_form_alive {
            release_main_form();
            self.main_form_alive = false;
        }
        logger::info("Controller", "--------------------------------------------------");
        logger::info("Controller", "ACTION: Starting Game Mode Services...");
        logger::info("Controller", "--------------------------------------------------");
        // (MAX_PRIORITY 不可移植 — service_loop.rs 同注)
        let interval = self
            .shared
            .intervals
            .lock()
            .expect("intervals 锁中毒")
            .service_loop_interval_ms;
        // cfg httpPort 覆盖 (Java Service 每请求读 Application.requestDest 静态 —
        // loadAppCheck 已写毕; 审查轮 1-A: 曾恒 env(Lang) 值, 打桩 9222 场景失效)。
        // None/0 = cfg 未写或未跑 loadFromConfig → 回退 env 启动值 (Java main 同位)
        let cfg_port = self
            .config
            .application_state()
            .request_dest
            .as_ref()
            .map(|a| a.port)
            .filter(|p| *p > 0)
            .map(|p| p as u16);
        let mut service = Service::new(
            ServiceConfig {
                // load_app_check 缺省 50; 字段 0 = 未跑过 loadFromConfig 的防御回退
                service_loop_interval_ms: if interval > 0 { interval } else { 50 },
                // 白盒 CLI 覆盖 > cfg httpPort 键 > Lang 启动值 (见 Env.port_override 注)
                app_port: self.env.port_override.or(cfg_port).unwrap_or(self.env.app_port),
                http_header: self.env.http_header.clone(),
            },
            Arc::clone(&self.fm),
            // FlightDataBus 单例语义 (LIFETIMES §1.1): AppShell 分发同一 Arc
            Arc::clone(&self.flight_bus),
        );
        // FocusMonitor 装配 (轮 2-C 收口, Java Controller.java:353-360 语义:
        // 会话启动按 cfg 启停 — 与 Service 同生共死, closepad 停 Service 即失效,
        // 等价 Java openpad 时 setEnabled + closepad setEnabled(false)):
        // tick 在 Service 轮询线程, 失焦回调经通道桥送 win32 执行 host hide/show
        {
            let auto_hide = self
                .config
                .get_config("autoHideOnFocusLoss")
                .map(|v| java_parse_boolean(&v))
                .unwrap_or(false);
            let mut fm = vm_core::platform::focus_monitor::FocusMonitor::new(
                Arc::new(vm_overlay::platform::extras::WindowsFocusDetector),
                Arc::new(ChannelFocusBridge {
                    tx: self.ui_cmd_tx.clone(),
                    shared: Arc::clone(&self.shared),
                }),
            );
            fm.set_enabled(auto_hide);
            logger::info(
                "Controller",
                if auto_hide { "焦点监控已启用 (随 Service 装配)" } else { "焦点监控未启用 (autoHideOnFocusLoss=false)" },
            );
            service.set_focus_monitor(fm);
        }
        // FlightLog 槽注入 (Service 轮询线程每轮 logTick, Service.java:1824-1828;
        // Controller.java:44 logon/Log 字段的共享面) — spawn 前随其余注入一次
        service.set_flight_log(Arc::clone(&self.flight_log));
        // 公式系统桥 (公式编辑器 tab 的直算命令面): Service 构造时已装载
        // formulas.cfg+user 并进 live 集, 此处挂 Arc 供 vm-webui 命令线程访问
        vm_webui::commands_formula::publish_formula_bridge(Arc::clone(&service.formula));
        let handle = spawn_service_thread(service);
        // 波4: 跨线程读面 = 帧仓 (零锁); handle.data 的锁面仅供 Service 内部
        *self.shared.live.write().expect("live 锁中毒") = Some(Arc::clone(&handle.frames));
        self.service = Some(handle);
        // B1 补偿信号接线 (ControllerShared.last_flight_event_ms 注): 新会话从 0 起,
        // live 事件只记到达时间 (回调在 Service 发布线程, 原子写无锁)
        self.shared.last_flight_event_ms.store(0, Ordering::SeqCst);
        let stamp_shared = Arc::clone(&self.shared);
        self.live_sub = Some(self.flight_bus.register(move |_ev: &FlightDataEvent| {
            stamp_shared
                .last_flight_event_ms
                .store(current_time_millis(), Ordering::SeqCst);
        }));
        self.config.save_config();
        self.config.save_layout_config();
    }

    /// Java Controller.stop() 五步销毁 (Controller.java:763-817, LIFETIMES §4.2 规范)。
    /// 顺序逐字保留; 步3 释放设置窗经注入闭包 (MainForm 归主线程 iced, W2 接线)。
    pub fn stop(&mut self, release_main_form: &mut dyn FnMut()) {
        // 1. 先关闭所有 overlay (预览/游戏模式) — 必须在 dispose MainForm 之前
        //    (Controller.java:763-779: previewGeneration++ 作废在途回调)
        if self.shared.state() == ControllerState::Preview {
            self.shared
                .preview_generation
                .fetch_add(1, Ordering::SeqCst);
            if self.service.is_some() {
                self.closepad(); // 游戏模式: closepad 完整清理
            } else {
                let _ = self.ui_cmd_tx.send(UiCommand::CloseAllOverlays);
            }
        }
        // 2. 取消事件订阅 (防重建后旧实例响应; RAII Drop = unsubscribe, Java 781-795;
        //    live_sub 为 Rust 侧 B1 补偿订阅, 同步退订防旧核刷新新核时间戳)
        self.subs.clear();
        self.fm_sub = None;
        self.live_sub = None;
        // 3. 清理 MainForm (Java:797-802 M.stopRepaintTimer + dispose + M=null)
        if self.main_form_alive {
            release_main_form();
            self.main_form_alive = false;
        }
        // 4. 清理 Service 线程 (Java:804-809 S=null; S1.interrupt())
        *self.shared.live.write().expect("live 锁中毒") = None;
        if let Some(mut h) = self.service.take() {
            let clean = h.stop(); // stop 标志 + join (interrupt 的电平形态)
            if !clean {
                logger::error("Controller", "Service 线程非正常退出 (§6 契约破坏观测点)");
            }
        }
        // 5. 保存配置 (Java:811-814; save_config 为空实现 — 全量在 ui_layout.cfg)
        self.config.save_config();
    }

    /// Java:849-857 Preview() — State=PREVIEW; 世代号快照; 后台线程刷新预览。
    pub fn preview(&mut self) {
        logger::info("Controller", "Enabling Preview mode...");
        self.shared.set_state(ControllerState::Preview);
        let generation = self.shared.preview_generation.load(Ordering::SeqCst);
        // PORT: loadFromConfig 在主线程先行 (Java 在后台线程写 Controller 字段 =
        // ★2 违规族; 值等价 — 配置已由发布方写毕), 线程内只做网络探测 + 送命令
        self.load_from_config_();
        let selected = self.config.get_config("selectedFM0").unwrap_or_default();
        let http_header = self.env.http_header.clone();
        let fm = Arc::clone(&self.fm);
        let tx = self.ui_cmd_tx.clone();
        let probe = self.probe_network;
        std::thread::Builder::new()
            .name("Preview-Refresh".to_string())
            .spawn(move || {
                logger::debug(
                    "Controller",
                    "Refreshing overlays for preview/config change...",
                );
                detect_and_identify(&selected, &http_header, &fm, probe);
                let _ = tx.send(UiCommand::RefreshPreviews {
                    changed_key: None,
                    generation,
                });
            })
            .expect("Preview-Refresh 线程创建失败");
    }

    /// Java:912-920 endPreview() — Preview 退出 (MainForm.confirm 前半)。
    pub fn end_preview(&mut self) {
        logger::info("Controller", "Exiting Preview mode...");
        self.shared
            .preview_generation
            .fetch_add(1, Ordering::SeqCst); // 作废在途回调
        let _ = self.ui_cmd_tx.send(UiCommand::CloseAllOverlays);
        self.config.save_config();
        self.shared.set_state(ControllerState::Init);
    }

    /// Java MainForm.confirm 的 tc 侧序列 (MainForm.java:265-278):
    /// endPreview → tc.saveConfig → tc.loadFromConfig → tc.start。
    /// (MainForm 自身的 saveConfig/hide 归 vm-ui 侧, W2 接线。)
    pub fn confirm_start_game(&mut self, release_main_form: &mut dyn FnMut()) {
        self.end_preview();
        self.config.save_config();
        self.load_from_config_();
        self.start(release_main_form);
    }

    // ------------------------------------------------------------------
    // Service 轮询驱动的状态转移 (Java Service→Controller 协作面;
    // vm-data 侧调用点已由 AppShell::pump → drive_from_live 顶替)
    // ------------------------------------------------------------------

    /// Java:155-175 initStatusBar — INIT → CONNECTED。
    /// PORT: StatusBar 窗口未移植 (状态条是 Swing 组件, C 类后续), 只保状态转移。
    pub fn init_status_bar(&mut self) {
        if self.shared.state() == ControllerState::Init {
            self.shared.set_state(ControllerState::Connected);
        }
    }

    /// Java:177-188 changeS2 — CONNECTED → IN_GAME
    pub fn change_s2(&mut self) {
        if self.shared.state() == ControllerState::Connected {
            self.shared.set_state(ControllerState::InGame);
        }
    }

    /// Java:202-249 changeS3 — IN_GAME → PREVIEW; 识别 FM; 延迟开面板。
    pub fn change_s3(&mut self, indic_type: Option<&str>) {
        if self.shared.state() != ControllerState::InGame {
            return;
        }
        self.fm.identify(indic_type);
        self.shared.set_state(ControllerState::Preview);
        // PORT(时序偏差备案, A-W7): Java openpad 全部内容 (含 FocusMonitor enable/
        // FlightLog) 都在延迟线程内执行; Rust 仅 OpenAllOverlays 走延迟, 其余面
        // (openpad_rest) 即时执行 — FocusMonitor 现随 Service 装配 (start()),
        // 时序差无观察面, 备案。
        // PORT(偏离声明, B-W5): Java 延迟线程 100ms 后**无守卫**发 openpad (停止窗口
        // 内 overlay 被 CloseAll 后重开, bug 形态保真); Rust 加 state/世代号守卫 —
        // Rust 侧重开残留形态比 Java 重 (overlay_ctx_preview 翻 false + 6 窗口全量
        // live 喂入), 守卫为显式改进: 停止 (stop/end_preview 的 gen++) 或
        // 退出 (S4toS1 → 非 Preview) 后丢弃本命令。
        let tx = self.ui_cmd_tx.clone();
        let shared = Arc::clone(&self.shared);
        let generation = self.shared.preview_generation.load(Ordering::SeqCst);
        std::thread::Builder::new()
            .name("Openpad-Delay".to_string())
            .spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                if shared.state() != ControllerState::Preview {
                    return; // 已退出 (S4toS1) — 丢弃
                }
                if shared.preview_generation.load(Ordering::SeqCst) != generation {
                    return; // stop()/end_preview() 已作废本回调
                }
                // openpad 的 overlay 面 (Java:363 openAll); 其余面见 openpad_rest
                let _ = tx.send(UiCommand::OpenAllOverlays);
            })
            .expect("Openpad-Delay 线程创建失败");
        self.openpad_rest();
    }

    /// Java openpad (344-386) 中非 overlay 窗口的其余面
    fn openpad_rest(&mut self) {
        // FocusMonitor 随 Service 装配 (start() 内按 cfg 启停, 与会话同生共死),
        // 本处不再重复读键
        self.open_flight_log();
        // (elapsed_time 基准; 缺写者时 elapsed=epoch 巨值, 污染 FlightLog/CSV 首列)
        // 波4: start_time 真相源 = 帧仓原子 (跨线程写点免锁)
        if let Some(handle) = self.service.as_ref() {
            handle.frames.set_start_time(current_time_millis());
        }
    }

    /// Java openpad 的 FlightLog 段 (Controller.java:366-376): enableLogging 开 →
    /// 通知 + `Log = new FlightLog(); Log.init(this, S, configService); logon = true`。
    /// onAircraftChanged 换机开新 (331-333) 复用本方法。
    fn open_flight_log(&mut self) {
        if !java_parse_boolean(&self.config.get_config("enableLogging").unwrap_or_default()) {
            return;
        }
        // logger 顶位留痕
        logger::info("Controller", Lang::init_lang().c_startlog);
        // 与 Java 同时刻从 live ServiceData 取快照。Service 缺失 = 测试 fixture
        // 手塞 live 绕过 start() 的专有形态 (Java 轮询链 openpad 必有 S), 跳过
        let Some(handle) = self.service.as_ref() else { return };
        let data = Arc::clone(&handle.data);
        let snap = flight_log_snapshot(&data.read().unwrap_or_else(|e| e.into_inner()));
        // 修 (批3裁决, 运行期兜底): records/ 相对 CWD 硬编码, 目录缺失时 init 的
        // 三个文件句柄全失败 (Java 同 bug — FileNotFoundException → "记录文件创建
        // 失败" toast + WARN, 基线冒烟可见)。生产路径启动记录前先建目录消除;
        // FlightLog 本体的降级行为不动 (vm-core tests.rs "records/ 缺失" 用例钉住)。
        // 创建失败 (只读介质等) 不拦截 — 后续 init 按原降级路径走。
        let _ = std::fs::create_dir_all("records");
        let mut log = vm_core::derived::flight_log::FlightLog::new();
        log.init(
            Arc::new(LogonSink(Arc::clone(&self.flight_log))),
            &snap,
            // config !Send (ConfigurationService 主线程独占) 不能整件入 FlightLog
            // (Service 线程 tick); 消费面仅 FlightAnalyzer.init 一次性读
            // "enableAltInformation" (flight_analyzer.rs:154-160, 此后 is_information
            // 固化不再读) — 单键快照与 Java 同一时刻同值, 语义等价
            // (原 FlightLogConfig 单键适配器 — 重构波2 SnapshotConfigProvider 三合一)
            Some(Arc::new(SnapshotConfigProvider::from_pairs([(
                "enableAltInformation",
                self.config.get_config("enableAltInformation"),
            )]))),
            Arc::new(|t: &str| logger::info("FlightLog", t)) as vm_core::derived::flight_log::NotifySink,
            Arc::new(ServiceAnalyzerSource::new(data)),
        );
        // 注: init 失败路径的 xc.logon=false (FlightLog.java:409) 被 Java openpad:375
        // 的无条件 logon=true 覆盖 (失败也 tick, 每轮 write 失败 warn — Java 行为),
        // 保真跟随无条件置 Some
        *self.flight_log.lock().expect("flight_log 槽锁中毒") = Some(Arc::new(Mutex::new(log)));
    }

    /// Java closepad 的 FlightLog 段 (Controller.java:402-411): 保存通知 + 爬升档数
    /// 判断弹 DrawFrame + `Log.close(); Log = null`。onAircraftChanged 关旧 (319-328)
    /// 与 stop 的 closepad 路径复用本方法。
    fn close_flight_log(&mut self) {
        if !java_parse_boolean(&self.config.get_config("enableLogging").unwrap_or_default()) {
            return;
        }
        let log = self
            .flight_log
            .lock()
            .expect("flight_log 槽锁中毒")
            .take();
        let Some(log) = log else { return };
        let mut log = log.lock().expect("flight_log 实例锁中毒");
        // toast 未移植 (豁免), logger 顶位
        let lang = Lang::init_lang();
        logger::info(
            "Controller",
            &format!("{}{}{}", lang.c_savelog, log.file_name, lang.c_plsopen),
        );
        // (爬升档数判断弹 DrawFrame 未移植, 对应的 Java bug 形态随 DrawFrame 一并豁免)
        // Log.close() 的 NPE 逃逸 closepad 时, 由 Service 轮询线程顶层 catch(Exception)
        // 吞掉 (Service.java:1850) — 本方法在主线程 (pump) 无该兜底, catch_unwind
        // 复刻 "崩方法不崩应用" 的 Java 净效果
        if let Err(payload) =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| log.close()))
        {
            let msg = if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = payload.downcast_ref::<&'static str>() {
                (*s).to_string()
            } else {
                "null".to_string()
            };
            logger::error(
                "Controller",
                &format!("FlightLog close 异常 (Java NPE 对位, 已吞): {msg}"),
            );
        }
    }

    /// Java closepad (388-421) — overlay 关闭 (命令) + 其余收尾面
    pub fn closepad(&mut self) {
        let _ = self.ui_cmd_tx.send(UiCommand::CloseAllOverlays);
        self.close_flight_log();
    }

    /// Java:251-283 S4toS1 — PREVIEW → INIT (退出游戏)。
    pub fn s4to_s1(&mut self) {
        if self.shared.state() != ControllerState::Preview {
            return;
        }
        // closepad 内含 FlightLog 保存 (Java:260 → 402-411)
        self.closepad();
        self.fm.clear_target();
        self.shared
            .flags
            .lock()
            .expect("flags 锁中毒")
            .session_aircraft_type = None;
        self.shared.set_state(ControllerState::Init);
    }

    /// Java:298-342 onAircraftChanged — 换机轻量 swap (幂等, 不重启 Controller)。
    pub fn on_aircraft_changed(&mut self, new_type: Option<&str>) {
        let Some(t) = new_type else { return };
        if t.is_empty() {
            return;
        }
        let mut flags = self.shared.flags.lock().expect("flags 锁中毒");
        if flags.session_aircraft_type.as_deref() == Some(t) {
            return;
        }
        let is_switch = flags.session_aircraft_type.is_some();
        flags.session_aircraft_type = Some(t.to_string());
        drop(flags);
        if !is_switch {
            return;
        }
        logger::info(
            "Controller",
            &format!(
                "Aircraft type changed to: {}. Lightweight FM swap (no Controller restart).",
                t
            ),
        );
        // 319-328 段含保存通知; 开新 = openpad 的 331-333 段; DrawFrame D8 豁免)
        self.close_flight_log();
        self.open_flight_log();
    }

    /// Service 轮询驱动的状态机推进 (AppShell::pump 调用)。
    /// PORT: Java Service.processPollingCycle 内联调用 c.initStatusBar/changeS2/
    /// changeS3/S4toS1 (vm-data 侧不再回调 Controller — 本方法轮询帧快照
    /// 顶替该调用面)。strState/strIndic 原始串在 HttpHelper
    /// 内部不可见: "flag 丢失" 分支 (Java:746-754, 串非空 + update 后 flag=false)
    /// 以 flags 假值直接顶替; "串空" 分支 (Java:755-761, 游戏退出/8111 消失 →
    /// HTTP 失败 → 串复位空, update 不执行, flags 保留**陈旧真值**) 无法从
    /// ServiceData 观测 — 以事件流静默超时补偿 (last_flight_event_ms 注/B1):
    /// flags/playerLive 均真但事件停发超阈值 → 判定串空, S4toS1。
    /// 残余偏差 (PORT 备案): 坠机 (playerLive=false) 后再退出游戏的组合不触发
    /// (静默判定含 playerLive 真前置 — 防误杀 Java 的着陆停机等待态,
    /// Service.java:746-754 sleep 等待路径), overlay 残留至托盘重建;
    /// vm-data 外泄 raw_strings_valid 后两分支可逐字保真。
    ///
    /// PORT(时序偏差声明, 审查 A-W3 — 均无观察面, StatusBar 未移植):
    /// a) changeS2 仅在 flags 双真时调用; Java (Service.java:1718) 串非空轮内
    ///    update 后**无条件** changeS2 再判 flag — "串非空+flag 假" 轮 Java 停
    ///    IN_GAME (changeS2 已推), Rust 停 Connected/Init; flags 转真的下一轮
    ///    两者同轮可达 Preview, 收敛等价。
    /// b) 串空补偿分支 (下方 silent) return 前不跑 init_status_bar; Java 串空轮
    ///    (Service.java:1711/1785-1790) 每轮仍先 initStatusBar (INIT→CONNECTED)
    ///    再 S4toS1 — 游戏退出后 Java 稳态 CONNECTED (等待重连), Rust 稳态停
    ///    Init (下方注释的"不能照跑"论证即为此让步, 特此声明稳态差异)。
    pub fn drive_from_live(&mut self) {
        let live = self.shared.live.read().expect("live 锁中毒").clone();
        let Some(frames) = live else { return };
        let Some(f) = frames.latest() else { return }; // 尚无首帧 = 等待数据
        let s_flag = f.s_state.as_ref().map(|s| s.flag).unwrap_or(false);
        let i_flag = f.s_indic.as_ref().map(|i| i.flag).unwrap_or(false);
        let i_type = f.s_indic.as_ref().and_then(|i| i.r#type.clone());
        let player_live = f.player_live;
        drop(f);
        // B1 补偿判定先行: 事件流静默 + flags/playerLive 陈旧真值 = 串空 (游戏退出)。
        // 该轮对位 Java 串空分支 (L755-761): 只 S4toS1, 不 initStatusBar/changeS2
        // (两者在 Java 串非空分支内 — 若照跑, 退出后状态会被 initStatusBar 重新
        // 推到 Connected/InGame, s4to_s1 的 Preview 守卫即永久拦断)。
        // last=0 视为非静默 (flags 真值必经串非空轮, 该轮 player_live 置真即同轮
        // 发布事件, 竞态窗口远小于阈值; 保守侧防首轮误判)。
        let last = self.shared.last_flight_event_ms.load(Ordering::SeqCst);
        let silent = last != 0 && current_time_millis().saturating_sub(last) > FLIGHT_SILENT_EXIT_MS;
        if silent && s_flag && i_flag && player_live {
            self.s4to_s1();
            return;
        }
        self.init_status_bar();
        if s_flag && i_flag {
            self.change_s2();
            if player_live {
                let t = i_type.clone();
                self.change_s3(t.as_deref());
                // 目标未变零成本 — 换机时 FMManager 异步切句柄, P4 轻量 swap 语义)
                self.fm.identify(t.as_deref());
                self.on_aircraft_changed(i_type.as_deref());
            }
            // else: Java 649 前的 playerLive 探测等待, 无 Controller 调用
        } else {
            self.s4to_s1();
        }
    }

    /// Controller 状态快照 (主线程读)
    pub fn state(&self) -> ControllerState {
        self.shared.state()
    }
}

/// Java:447-454 loadFromConfig (独立函数: 订阅转发面不持 config, 主线程统一调用)
fn load_from_config(
    config: &ConfigurationService,
    shared: &ControllerShared,
    voice: &VoiceResourceManager,
) {
    let mut intervals = shared.intervals.lock().expect("intervals 锁中毒");
    config.load_app_check(&mut intervals);
    drop(intervals);
    // VoiceResourceManager.applyVolume 读它 (跨线程非 volatile 隐患) — Rust 侧
    // ApplicationState.voice_volumn 与管理器内原子是两消费面 (voice_resource_manager.rs
    // PORT 注), 在此单一写点同步 (§2.9 状态分裂禁令的收口: 配置 !Send 恒留主线程,
    // 管理器经原子跨线程读; Java 的三处 loadFromConfig 调用路径均经本函数)
    voice.set_voice_volumn(config.application_state().voice_volumn);
    let mut flags = shared.flags.lock().expect("flags 锁中毒");
    flags.show_status = true;
    if let Some(v) = config.get_config("enableStatusBar") {
        if !v.is_empty() {
            flags.show_status = java_parse_boolean(&v);
        }
    }
}

/// Java:865-877 detectAndIdentify — live 机型探测 → selectedFM0 兜底 → identify。
/// `probe_network=false` 跳过 live 探测只走配置兜底 (测试注入面, 见 ControllerDeps)。
fn detect_and_identify(selected_fm0: &str, http_header: &str, fm: &FMManager, probe_network: bool) {
    // getLiveAircraftType 自带异常兜底 (失败/无游戏 → None)
    let live = if probe_network {
        let fetcher = HttpHelper::new(http_header);
        fetcher.get_live_aircraft_type()
    } else {
        None
    };
    let target = live.unwrap_or_else(|| selected_fm0.to_string());
    if !target.is_empty() {
        fm.identify(Some(&target));
    }
}
