//! vm-app 组装层 (P5 批十四 W1): AppShell + Controller 生命周期核心 + win32 线程入口。
//! 重构波2 九劈: 内容按职责拆至子模块 (env/commands/controller_shared/debouncer/
//! overlay_inputs/controller/voice_setup/win32/keys), 本文件保留 AppShell 装配、
//! handle_main_event/dispatch/pump/rebuild/shutdown 主体与 lib 根 re-export。
//!
//! 对应 Java:
//! - `src/prog/Controller.java` — 状态机 (INIT→CONNECTED→IN_GAME→PREVIEW)、start/stop
//!   五步销毁、previewGeneration 防过期、ConfigDebounce 防抖、registerGameModeOverlays、
//!   detectAndIdentify (FM-Detect)、openpad/closepad/changeS2/changeS3/S4toS1/
//!   onAircraftChanged。
//! - `src/prog/Application.java` — 静态字段落位 (D8: AppShell 显式持有, 禁 static mut)。
//!
//! 线程拓扑 (DECISIONS.md D8):
//! - 主线程: MainForm (iced, W2 波次接线) + AppShell 监督循环 (本文件 `run_supervisor`)。
//! - win32 线程: OverlayHost 全部 overlay 窗口 + 托盘消息窗口 + 热键事件消费
//!   (单泵共享; 热键钩子线程豁免记录见 `win32::win32_thread_main` 头注)。
//! - Service 线程: vm-data service_loop (8111 轮询, Controller 波次仅负责启停)。
//! - ConfigDebounce 线程: 200ms 防抖 (Java static configDebouncer 的跨重建存活语义)。
//!
//! 跨线程纪律 (LIFETIMES §7 / D8): 全部经 channel/bus, 禁全局静态可变态。
//! **ConfigurationService 是 !Send** (config_loader 树含 Rc<SExp>, 见
//! configuration_service.rs init_config 的 PORT 注) — 因此:
//! 1. 配置服务恒留主线程, Controller 订阅闭包只做"转发到监督通道" (Send 安全);
//! 2. win32 线程需要的配置面以 Send 快照送达 ([`HudSettingsSnapshot`] +
//!   [`ActivationCache`]), 快照刷新由监督循环在 CONFIG_CHANGED 到达时执行。

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use vm_core::config_api::ConfigProvider; // get_config/set_config trait 面 (根+tests 经 glob 消费)
use vm_core::configuration_service::{ConfigurationService, GlobalColors};
use vm_core::controller_state::ControllerState;
use vm_core::event::ui_state_events;
use vm_core::flight_data_bus::FlightDataBus;
use vm_core::fm::FMManager;
use vm_core::lang::Lang;
use vm_core::logger;
use vm_core::ui_state_bus::UIStateBus;
use vm_core::voice_resource_manager::VoiceResourceManager;

use vm_overlay::hotkey::{HotkeyEvent, HotkeyManager};

// tests.rs 经 `use super::*` 消费的外部符号 (cfg(test) 免非测试构建的 unused 警告)
#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use std::rc::Rc;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::time::Instant;
#[cfg(test)]
use vm_core::config_api::HudSettingsSnapshot;
#[cfg(test)]
use vm_core::event::event_payload::EventPayload;
#[cfg(test)]
use vm_core::fm::FMStatus;
#[cfg(test)]
use vm_data::service_fields::ServiceData;
#[cfg(test)]
use vm_overlay::host::OverlayHost;

// ---- 重构波2 子模块 (pub use 保持 main.rs/form_dispatch.rs 的 vm_app::X 路径) ----
mod commands;
mod controller;
mod controller_shared;
mod debouncer;
mod env;
mod keys;
mod overlay_inputs;
mod voice_setup;
mod win32;

pub use crate::commands::{DebounceMsg, MainEvent, SupervisorOutcome, TrayCommand, UiCommand};
pub use crate::controller::{Controller, ControllerDeps};
pub use crate::controller_shared::{
    is_stale_refresh, ControllerFlags, ControllerShared, FLIGHT_SILENT_EXIT_MS,
};
pub use crate::debouncer::{ConfigDebouncer, CONFIG_DEBOUNCE_MS};
pub use crate::env::Env;
pub use crate::keys::{
    FM_FIELD_KEYS, FM_UNPACKED_INTEREST_KEYS, GLOBAL_COLOR_KEYS, MINIHUD_INTEREST_KEYS,
    OVERLAY_SECTIONS,
};
pub use crate::overlay_inputs::{ActivationCache, OverlayInputs, ACTIVATION_KEYS};
pub use crate::win32::{win32_thread_main, Win32ThreadConfig};

// 根消费的 pub(crate) 项 (私有引入; tests 经 `use super::*` 同样可见)
use crate::env::{java_parse_boolean, locate_template_cfg};
use crate::overlay_inputs::refresh_activation_cache;
use crate::voice_setup::{
    attach_snapshot_hooks, refresh_fm_field_config_snapshot, refresh_voice_config_snapshot,
};

// tests.rs 专用符号 (经 `use super::*` 抵达; cfg(test) 免非测试构建 unused 警告)
#[cfg(test)]
use crate::env::current_time_millis;
#[cfg(test)]
use crate::voice_setup::open_voice_warning;
#[cfg(test)]
use crate::win32::{
    feed_overlays_live, register_live_overlays, reset_handles_preview_values, strategy_for,
    AttitudeFeedState, ChannelFocusBridge, HostActivationCtx, OverlayHandles,
};

/// 语音播放平台件 (winmm waveOut 每路独立流; 播放模型裁决见该模块头注)
pub mod winmm_player;

// =====================================================================
// AppShell — Application 静态态收敛 + 监督循环 (D8)
// =====================================================================

/// AppShell 装配输入 ([`AppShell::with_parts`] 注入面, 测试用 tmp 配置等)
pub struct ShellParts {
    pub env: Env,
    /// 初始配置服务 (生产: new + initConfig; 测试: tmp cfg load_layout)
    pub config: ConfigurationService,
    pub ui_bus: Arc<UIStateBus>,
    pub flight_bus: Arc<FlightDataBus>,
    pub fm: Arc<FMManager>,
    pub hotkey: HotkeyManager,
    pub hotkey_rx: Receiver<HotkeyEvent>,
    /// 防抖延迟 (生产 200ms = CONFIG_DEBOUNCE_MS; 测试短间隔)
    pub debounce_delay: Duration,
}

/// Java Application.java 静态单例带 + `public static Controller ctr` 的收敛体。
/// 恒留主线程; 跨重建存活件 (Java 单例语义, LIFETIMES §4.2 "托盘重建时存活"):
/// fm/flight_bus/ui_bus/hotkey/debouncer。
pub struct AppShell {
    pub env: Env,
    pub ui_bus: Arc<UIStateBus>,
    pub flight_bus: Arc<FlightDataBus>,
    pub fm: Arc<FMManager>,
    pub hotkey: Arc<Mutex<HotkeyManager>>,
    /// Java `VoiceResourceManager.getInstance()` 进程级单例 (静态 final INSTANCE)
    /// 的落位: AppShell 显式持有 (D8; §2.9 禁全局静态的载体替换), 跨核重建
    /// 存活 = Java static 语义; 组装层各消费面 (表单 IPC 的语音包列表/试听、
    /// VoiceWarning 告警线程) 经同一 Arc 共享。voice 目录 "voice" 与
    /// form_dispatch 旧局部实例一致 (Java "./voice/")。
    pub voice: Arc<VoiceResourceManager>,
    /// voice_* 配置键快照 ([`crate::voice_setup::SnapshotConfigProvider`] 的数据面;
    /// 配置 !Send 恒留主线程, VoiceWarning 的 reload 链经快照跨线程读 — FlightLogConfig 单键
    /// 快照先例的全键版)。重构波1: 常规写值经 ConfigurationService 的
    /// write_hook 在广播前直写 (快照新值先于订阅者), 本字段随核重建全量重刷
    pub voice_config: Arc<Mutex<HashMap<String, String>>>,
    /// FM拆包数据 show* 配置键快照 ([`crate::voice_setup::SnapshotConfigProvider`] 的数据面;
    /// FMUnpackedData 的 generate_lines 每 tick 读, CONFIG_CHANGED 逐键刷新)
    pub fm_field_config: Arc<Mutex<HashMap<String, String>>>,
    pub shared: Arc<ControllerShared>,
    /// 激活缓存 (win32 线程激活探测的配置面)
    pub activation: ActivationCache,
    /// UI 命令通道发送端 (win32 线程接收端在 spawn 时移交; 移交前测试可观察)
    pub ui_cmd_tx: Sender<UiCommand>,
    ui_cmd_rx: Option<Receiver<UiCommand>>,
    hotkey_rx: Option<Receiver<HotkeyEvent>>,
    /// 监督事件通道 (Controller 订阅转发 + 托盘动作汇聚于此)
    main_event_tx: Sender<MainEvent>,
    main_event_rx: Receiver<MainEvent>,
    debounce: ConfigDebouncer,
    /// 可重建应用核 (Java Application.ctr; 托盘 Activate 时整体替换)
    pub controller: Option<Controller>,
    /// win32 线程句柄 (spawn_win32_thread 后 Some)
    win32: Option<JoinHandle<()>>,
    /// stop 步3 / start 的设置窗释放闭包 (W2 接 iced MainForm; 默认记日志)
    pub release_main_form: Box<dyn FnMut()>,
    /// Exit 托盘命令 → run_supervisor 退出标志
    exit_requested: bool,
    /// 托盘 About 置位 (Java about 菜单的 showAbout×3 展示动作) — 组装层主循环
    /// 据此 emit `about-requested` 转发前端 Modal; 无窗形态 (run_supervisor_phase)
    /// 消费时记日志兜底
    about_requested: bool,
    /// 托盘 Activate 置位 (新核构造了 MainForm 存活位) — 组装层主循环据此
    /// 重开 iced 设置窗 (Java: 托盘点击 → ctr = new Controller(false) → 弹窗)。
    /// 相 A (窗口期) 内置位 → 关窗重开; 相 B (监督期) 内置位 → run_supervisor_phase
    /// 返回 MainFormRequested。
    form_requested: bool,
    /// 8111 网络探测开关 (生产 true; 测试隔离置 false, 见 fixture 注)
    probe_network: bool,
    /// 首次 rebuild 复用的注入配置 (AppShell::new 已 initConfig 的生产配置 /
    /// 测试 tmp cfg); 托盘重建走磁盘新装载 (Java 每核 new ConfigurationService)
    initial_config: Option<ConfigurationService>,
}

impl AppShell {
    /// 生产构造 (Java Application.main:533-604 启动序):
    /// Lang → 端口/Env → 总线/FM/热键 → 防抖 → 初始 Controller(true)。
    /// `live`: 对齐 `autoStartGameMode=true` 配置 (CLI --live / e2e —
    /// Java 无此开关, 由用户配置表达; 此处以等效配置注入, Controller 自启动
    /// 判定路径零特判)。
    pub fn new(debug: bool, live: bool) -> Result<AppShell, String> {
        AppShell::new_with_port(debug, live, None)
    }

    /// 白盒端口覆盖 (`--port` CLI / mock-smoke 的 9222 约定): Env 只读区在 probe
    /// 后覆写, 语义 = Lang.httpPort 解析结果的等价替换 (bkp 同步 +1111 保持
    /// 备用端口关系)。生产路径 (desktop_main) 不传 — 端口仍由 Lang/配置表达;
    /// 白盒测试统一走 9222 (游戏本地 API 恒占 8111, 备用端口域游戏永不监听,
    /// 真机在跑也不再挤掉测试)。
    pub fn new_with_port(
        debug: bool,
        live: bool,
        port_override: Option<u16>,
    ) -> Result<AppShell, String> {
        let lang = Lang::init_lang();
        let mut env = Env::probe(&lang, debug);
        if let Some(p) = port_override {
            env.app_port = p;
            // 与 probe 同式 (域内恒 p+1111, u16 加法无回绕面)
            env.app_port_bkp = p + 1111;
            env.port_override = Some(p);
        }
        let ui_bus = Arc::new(UIStateBus::new());
        let config = ConfigurationService::new(Some(Arc::clone(&ui_bus)));
        // Java Controller 构造器: configService.initConfig() 装载设置文件
        config.init_config();
        if live {
            // 对位 Java e2e 的 autoStartGameMode=true 配置 (Controller.java:589-606
            // 自启动分支: 跳过 MainForm 直接 start Service)
            config.set_config("autoStartGameMode", "true");
        }
        let (hotkey, hotkey_rx) = HotkeyManager::with_channel();
        let mut shell = AppShell::with_parts(ShellParts {
            env,
            config,
            ui_bus,
            flight_bus: Arc::new(FlightDataBus::new()),
            fm: Arc::new(FMManager::new(Arc::new(vm_core::bus::EventBus::new()))),
            hotkey,
            hotkey_rx,
            debounce_delay: Duration::from_millis(CONFIG_DEBOUNCE_MS),
        });
        shell.rebuild_controller(true);
        Ok(shell)
    }

    /// 注入装配 (测试/自定义装配面; 不建 Controller — 调 rebuild_controller)
    pub fn with_parts(parts: ShellParts) -> AppShell {
        let ShellParts {
            env,
            config,
            ui_bus,
            flight_bus,
            fm,
            hotkey,
            hotkey_rx,
            debounce_delay,
        } = parts;
        let (ui_cmd_tx, ui_cmd_rx) = std::sync::mpsc::channel::<UiCommand>();
        let (main_event_tx, main_event_rx) = std::sync::mpsc::channel::<MainEvent>();
        let shared = Arc::new(ControllerShared::new());
        // 激活缓存初建 (win32 线程激活探测输入)
        let activation: ActivationCache = Arc::new(Mutex::new(HashMap::new()));
        refresh_activation_cache(&config, &activation);
        // 语音资源管理共享实例 (Java getInstance() 单例; 播放器 = winmm waveOut 腿)
        let voice = Arc::new(VoiceResourceManager::new_with_voice_dir(
            winmm_player::make_player(),
            "voice".to_string(),
        ));
        // voice_* / FM show* 配置键快照 (跨线程读面, 初始全量填充);
        // 常规写值经 write_hook 在 CONFIG_CHANGED 广播前直写快照
        let voice_config: Arc<Mutex<HashMap<String, String>>> =
            Arc::new(Mutex::new(HashMap::new()));
        refresh_voice_config_snapshot(&config, &voice_config);
        let fm_field_config: Arc<Mutex<HashMap<String, String>>> =
            Arc::new(Mutex::new(HashMap::new()));
        refresh_fm_field_config_snapshot(&config, &fm_field_config);
        attach_snapshot_hooks(&config, &voice_config, &fm_field_config);
        let debounce =
            ConfigDebouncer::spawn(debounce_delay, ui_cmd_tx.clone(), Arc::clone(&shared));
        AppShell {
            env,
            ui_bus,
            flight_bus,
            fm,
            hotkey: Arc::new(Mutex::new(hotkey)),
            voice,
            voice_config,
            fm_field_config,
            shared,
            activation,
            ui_cmd_tx,
            ui_cmd_rx: Some(ui_cmd_rx),
            hotkey_rx: Some(hotkey_rx),
            main_event_tx,
            main_event_rx,
            debounce,
            controller: None,
            win32: None,
            release_main_form: Box::new(|| {
                // 默认无 MainForm (W2 前): 记日志占位
                logger::info("AppShell", "释放设置窗 (默认空操作 — W2 接线前)");
            }),
            exit_requested: false,
            about_requested: false,
            form_requested: false,
            probe_network: true,
            initial_config: Some(config),
        }
    }

    /// 测试注入面: 关闭 8111 live 探测 (须在 rebuild_controller 前调用)
    pub fn probe_network_for_test(&mut self, on: bool) {
        self.probe_network = on;
    }

    /// 托盘重建/初始构造 (Java Application.java:251-273 mouseClicked 与 main:590)。
    /// `is_initial_launch`: true=初始启动 (尊重 autoStartGameMode), false=托盘恢复
    /// (恒弹设置窗语义 — Java Controller(false))。
    pub fn rebuild_controller(&mut self, is_initial_launch: bool) {
        if let Some(old) = self.controller.as_mut() {
            old.stop(&mut self.release_main_form); // 旧核五步销毁
        }
        // 首核复用注入配置 (AppShell::new 已 initConfig / 测试 tmp cfg — 免重复装载
        // 与写盘副作用); 托盘重建核走磁盘新装载
        let config = match self.initial_config.take() {
            Some(c) => c,
            None => {
                let config = ConfigurationService::new(Some(Arc::clone(&self.ui_bus)));
                config.init_config();
                // 快照写值钩子随新配置树重挂 (voice_*/FM show* 直写面)
                attach_snapshot_hooks(&config, &self.voice_config, &self.fm_field_config);
                // 模板回退 (vm-ui main.rs 同款分歧备案: CWD 无用户 cfg 时以仓库模板自愈)
                if config.get_layout_configs().is_none_or(|g| g.is_empty()) {
                    match locate_template_cfg() {
                        Some(p) => {
                            logger::warn("AppShell", &format!("CWD 无用户配置, 回退模板 {}", p));
                            config.load_layout(&p);
                        }
                        None => logger::warn("AppShell", "未找到 ui_layout.cfg, 配置面为空"),
                    }
                }
                config
            }
        };
        refresh_activation_cache(&config, &self.activation); // win32 激活面同步
        // voice_* 快照随新配置树全量重刷 (VoiceWarning 跨线程配置读面)
        refresh_voice_config_snapshot(&config, &self.voice_config);
        // FM show* 快照同批重刷 (FMUnpackedData 的 generate_lines 读面)
        refresh_fm_field_config_snapshot(&config, &self.fm_field_config);
        self.shared.reset_for_rebuild();
        self.controller = Some(Controller::new(
            ControllerDeps {
                config,
                ui_bus: Arc::clone(&self.ui_bus),
                flight_bus: Arc::clone(&self.flight_bus),
                fm: Arc::clone(&self.fm),
                hotkey: Arc::clone(&self.hotkey),
                voice: Arc::clone(&self.voice),
                shared: Arc::clone(&self.shared),
                ui_cmd_tx: self.ui_cmd_tx.clone(),
                main_event_tx: self.main_event_tx.clone(),
                env: self.env.clone(),
                probe_network: self.probe_network,
            },
            is_initial_launch,
        ));
    }

    /// 起 win32 线程 (D8 拓扑: host 泵 + 托盘 + 热键事件消费; 单泵共享)。
    /// 配置以 Send 快照 (`OverlayInputs` + `activation`) 入线程 — 服务本体 !Send。
    pub fn spawn_win32_thread(&mut self) -> Result<(), String> {
        if self.win32.is_some() {
            return Err("win32 线程已存在".into());
        }
        let ui_cmd_rx = self
            .ui_cmd_rx
            .take()
            .ok_or_else(|| "ui_cmd 通道已移交".to_string())?;
        let hotkey_rx = self
            .hotkey_rx
            .take()
            .ok_or_else(|| "hotkey 接收端已移交".to_string())?;
        let controller = self.controller.as_ref().ok_or("controller 未构造")?;
        let inputs = OverlayInputs::build(&controller.config, &self.env, &self.shared);
        // 初始位置快照 (Java overlay init 时 loadPosition 读 gc.x/y; 配置 !Send
        // → 一次性快照进 win32 线程, 保存经 MainEvent::PositionSaved 回传落盘)
        let position_snapshot: HashMap<String, (f64, f64)> = OVERLAY_SECTIONS
            .iter()
            .filter_map(|(id, section)| {
                controller
                    .config
                    .group_position(section)
                    .map(|p| (id.to_string(), p))
            })
            .collect();
        let cfg = Win32ThreadConfig {
            env: self.env.clone(),
            inputs,
            ui_bus: Arc::clone(&self.ui_bus),
            flight_bus: Arc::clone(&self.flight_bus),
            fm: Arc::clone(&self.fm),
            shared: Arc::clone(&self.shared),
            activation: Arc::clone(&self.activation),
            voice: Arc::clone(&self.voice),
            voice_config: Arc::clone(&self.voice_config),
            fm_field_config: Arc::clone(&self.fm_field_config),
            ui_cmd_rx,
            hotkey_rx,
            main_event_tx: self.main_event_tx.clone(),
            position_snapshot,
        };
        let join = std::thread::Builder::new()
            .name("win32-pump".to_string())
            .spawn(move || win32_thread_main(cfg))
            .map_err(|e| format!("win32 线程创建失败: {}", e))?;
        self.win32 = Some(join);
        Ok(())
    }

    /// MainForm 侧命令路由 (W2: iced update 收 Message::StartGame → 此处)
    pub fn dispatch(&mut self, cmd: UiCommand) {
        match cmd {
            UiCommand::StartGame => {
                // Java MainForm.confirm → tc 侧序列
                if let Some(c) = self.controller.as_mut() {
                    c.confirm_start_game(&mut self.release_main_form);
                }
            }
            UiCommand::EndGame => {
                // Java MainForm.java:92-98 mCancel: MainForm.saveConfig + tc.saveConfig
                // + System.exit(0)。tc.saveConfig 对位 Controller.config.save_config
                // (空实现); saveLayoutConfig **不在** mCancel 链 (仅 start() 路径,
                // Controller.java:640-641) — 设置窗内未确认落盘的 layout 改动随
                // 退出丢弃, 勿在此加回 (审查 A-W1); System.exit(0) 的退出归属:
                // 置 exit_requested — run_supervisor 路径经循环尾 shutdown() 收尾
                // 退出 (比 Java 裸 exit 多做线程/托盘清理); W2 iced 外部驱动路径
                // 由调用方轮询 is_exit_requested() 决定退出 (iced 侧 exit +
                // drop AppShell = Drop 兜底 shutdown)。
                if let Some(c) = self.controller.as_ref() {
                    c.config.save_config();
                }
                self.exit_requested = true;
            }
            // win32 属主变体不经 dispatch (发送方直达 ui_cmd_tx); 防御性转发
            other => {
                let _ = self.ui_cmd_tx.send(other);
            }
        }
    }

    /// 监督事件处理 (Controller 订阅转发 + 托盘动作的落地点; 主线程)
    pub fn handle_main_event(&mut self, ev: MainEvent) {
        match ev {
            // overlay 位置存档落盘 (win32 线程拖拽松手/销毁链回传; Java
            // DraggableOverlay.saveCurrentPosition → saveWindowPosition +
            // saveLayoutConfig — 归一化直写, 免像素往返)
            MainEvent::PositionSaved { section, x, y } => {
                if let Some(c) = self.controller.as_ref() {
                    c.config.save_group_position(&section, x, y);
                }
            }
            // Java configChangedHandler (Controller.java:498-544)
            MainEvent::ConfigChanged(key) => {
                let is_reset_completed = key == ui_state_events::ACTION_RESET_COMPLETED;
                let Some(c) = self.controller.as_mut() else { return };
                if key == "displayFmKey" || key == "enableFMPrint" {
                    c.handle_fm_hotkey_config_change();
                }
                if is_reset_completed {
                    c.handle_fm_hotkey_config_change();
                }
                // 全局五色直送 (fontNum/fontLabel/fontUnit/fontWarn/fontShade —
                // Java 经 font 前缀全局键触发全量刷新; 配置 !Send 色值随命令进
                // win32 线程, 下帧渲染即新色, 不需重建窗口)。
                // 即时读 cfg 键而非 global_colors() 快照: app 状态五色由
                // load_from_config_ 刷新 (时序在下游), 发布方 (ColorRowRenderer)
                // 此刻已写服务树值
                if key == "AAEnable" {
                    // 直读 cfg 即时值 (java_parse_boolean: "true"→true 其他 false;
                    // loadFromConfig 缺省语义 false, app 状态刷新在下游)
                    let on = c
                        .config
                        .get_config("AAEnable")
                        .map(|v| java_parse_boolean(&v))
                        .unwrap_or(false);
                    let _ = self.ui_cmd_tx.send(UiCommand::SetAa(on));
                }
                if GLOBAL_COLOR_KEYS.contains(&key.as_str()) {
                    let g = GlobalColors {
                        num: c.config.get_color_config("fontNum"),
                        label: c.config.get_color_config("fontLabel"),
                        unit: c.config.get_color_config("fontUnit"),
                        warning: c.config.get_color_config("fontWarn"),
                        shade_shape: c.config.get_color_config("fontShade"),
                    };
                    let _ = self.ui_cmd_tx.send(UiCommand::SetGlobalColors(g));
                }
                // win32 激活面同步 (配置已由发布方写毕, 最后写胜出 — 见缓存头注)
                refresh_activation_cache(&c.config, &self.activation);
                // Java VoiceWarning.configHandler 的触发链 (UIStateBus 单例 →
                // CONFIG_CHANGED(voice_*) → alert.reload) 重构波1 起由统一路由
                // 总线直连 (ConfigurationService 发布 → VoiceWarning 订阅);
                // voice_*/FM show* 快照已在 set_config 的 write_hook 广播前直写,
                // 此处无需事后补刷 (原类型分裂时代的转发桥退役)
                // WYSIWYG reinit 参数直送 (五色直送同款模式): 即时读配置重建参数包,
                // 先于下方 RefreshPreviews(防抖)/ReinitActiveOverlays 入队 —
                // 对位 Java refreshPreviews → reinitConfig 即时读配置的时序
                let params = vm_overlay::ReinitParams::from(&OverlayInputs::build(
                    &c.config,
                    &self.env,
                    &self.shared,
                ));
                let _ = self
                    .ui_cmd_tx
                    .send(UiCommand::ReinitOverlays { params: Box::new(params) });
                if c.state() == ControllerState::Preview {
                    // loadFromConfig 在调度点先行 (Java 在任务体首行; 配置 !Send
                    // 不能进防抖线程, 值等价 — 发布→调度间配置无二次变更面)
                    c.load_from_config_();
                    let _ = self.debounce.sender().send(DebounceMsg::ConfigKey(key));
                } else {
                    logger::info(
                        "Controller",
                        &format!("ACTION: Controller: Reloading config ({})", key),
                    );
                    c.load_from_config_();
                    let _ = self.ui_cmd_tx.send(UiCommand::ReinitActiveOverlays);
                }
            }
            // Java uiReadyHandler (547-552): UI Ready → Preview
            MainEvent::UiReady => {
                logger::info(
                    "Controller",
                    "ACTION: Controller: UI Ready. Initializing Preview...",
                );
                if let Some(c) = self.controller.as_mut() {
                    c.preview();
                }
            }
            // Java fmChangedHandler (554-579)
            MainEvent::FmChanged { name, corrupt } => {
                if let Some(n) = name {
                    let lang = Lang::init_lang();
                    let msg = if corrupt {
                        lang.fm_corrupt_toast
                    } else {
                        lang.fm_missing_toast
                    };
                    logger::warn("Controller", &format!("{}\n{}", n, msg));
                }
                if let Some(c) = self.controller.as_ref() {
                    if c.state() == ControllerState::Preview {
                        let _ = self.debounce.sender().send(DebounceMsg::FmChanged);
                    }
                }
            }
            MainEvent::Tray(TrayCommand::Activate) => {
                // Java Application.java:251-273: 旧核 stop + 新核构造。
                // PORT(防重入窗口备案, 审查 A-W2): 托盘层 CAS (tray.rs dispatch_activate)
                // 仅覆盖 handler.activate() 的 channel send (微秒级即复位), 远窄于
                // Java CAS 覆盖整个 ctr.stop()+new Controller() 的窗口 — 快速双击
                // 会向本通道投递**两条** Activate, 此处串行 rebuild×2 (串行 ≠ 只收到
                // 一次)。与 Java 行为等价: Java 托盘回调同样在 EDT 串行, 第二次点击
                // 在第一次 finally 复位后到达, 同样重建两次; 最终态一致且无泄漏
                // (第二次 rebuild 的 stop 收掉刚建核, ServiceHandle Drop 兜底 stop+join)。
                self.rebuild_controller(false);
                // 新核 main_form_alive=true (Java Controller(false) 构造 MainForm) —
                // 真窗开合归组装层主循环, 置请求位
                self.form_requested = true;
            }
            MainEvent::Tray(TrayCommand::Start) => {
                // tray.rs 拆分入口: Controller 重建的服务启动部分
                if let Some(c) = self.controller.as_mut() {
                    c.start(&mut self.release_main_form);
                }
            }
            MainEvent::Tray(TrayCommand::About) => {
                self.about_requested = true;
            }
            MainEvent::Tray(TrayCommand::Exit) => {
                self.exit_requested = true;
            }
        }
    }

    /// 非阻塞泵: 监督事件 + 状态机推进 (W2 的 iced tick / 测试循环调用;
    /// Java 中该两项分别由 UIStateBus 同步回调与 Service 轮询线程内联承担)
    pub fn pump(&mut self) {
        while let Ok(ev) = self.main_event_rx.try_recv() {
            self.handle_main_event(ev);
        }
        if let Some(c) = self.controller.as_mut() {
            c.drive_from_live();
        }
    }

    /// 退出请求查询 (W2 iced 外部驱动路径: dispatch(EndGame)/托盘 Exit 置位;
    /// 调用方见真即应退出事件循环并 drop AppShell — Drop 兜底收尾)。
    pub fn is_exit_requested(&self) -> bool {
        self.exit_requested
    }

    /// 取走设置窗重开请求 (托盘 Activate; 见 form_requested 注)。组装层主循环:
    /// 相 A (iced 窗口期) Tick 泵查询 → true 即 iced::exit 关窗重开;
    /// 相 B (监督期) 由 run_supervisor_phase 返回值表达。
    pub fn take_form_request(&mut self) -> bool {
        std::mem::replace(&mut self.form_requested, false)
    }

    /// 取走托盘"关于"请求 (Java about 菜单的展示动作; 见 about_requested 注)。
    /// 组装层主循环查询 → emit `about-requested` 转发前端 About Modal。
    pub fn take_about_request(&mut self) -> bool {
        std::mem::replace(&mut self.about_requested, false)
    }

    /// 阻塞监督循环 (无 MainForm 场景: --live / 冒烟; Java 托盘+EDT 泵的对位)。
    /// Exit 托盘命令或通道关闭即返回 (进程退出归调用方)。
    /// 防呆 (审查 A-W3): 生产入口必须先起 win32 线程 (托盘/overlay/热键泵);
    /// 未 spawn 直接 run = 无托盘无窗口且 TrayCommand::Exit 永不可达 (通道不关
    /// 则循环不退) — 此处对未启动的 win32 线程自动补 spawn。
    /// spawn 失败 = Exit 兜底 (审查 B-W4): 线程创建失败/核缺失属 OS 级资源问题;
    /// 若仅告警继续, 监督循环无退出面 (本 shell 自持 main_event_tx, 通道恒不
    /// Disconnected) → 只能外部 kill。无桌面环境的冒烟不受影响: 托盘/窗口创建
    /// 失败在线程**内部**逐项降级 (warn + 继续跑), 走不到本兜底。
    pub fn run_supervisor(mut self) {
        if self.win32.is_none() && self.ui_cmd_rx.is_some() {
            if let Err(e) = self.spawn_win32_thread() {
                logger::error(
                    "AppShell",
                    &format!("win32 线程启动失败, 无监督退出面, 转退出: {}", e),
                );
                self.exit_requested = true;
            }
        }
        loop {
            // 无 MainForm 的运行形态: 托盘 Activate 重建核后无窗可开, 记日志继续
            // (设置窗的开合属组装层主循环的完整路径)
            match self.run_supervisor_phase() {
                SupervisorOutcome::Exit => break,
                SupervisorOutcome::MainFormRequested => logger::info(
                    "AppShell",
                    "托盘请求设置窗 — 无 MainForm 运行形态, 已重建核继续监督",
                ),
            }
        }
        self.shutdown();
    }

    /// 分相监督循环 (组装层主循环的相 B: MainForm 关闭后 — 开始游戏/窗口 X)。
    /// 对位 Java: MainForm.dispose 后 EDT 事件循环继续 (托盘/overlay 存活),
    /// Controller 的 Service 驱动状态机推进。
    /// 返回: Exit = 进程退出请求 (EndGame/托盘 Exit); MainFormRequested = 托盘
    /// Activate 已重建核并请求弹设置窗 (主循环回相 A 重开 iced 窗口)。
    /// win32 线程未启动时自动补 spawn (run_supervisor 同款防呆; spawn 失败转
    /// Exit 兜底, 见其注释 — 无退出面的悬空监督不可达)。
    pub fn run_supervisor_phase(&mut self) -> SupervisorOutcome {
        if self.win32.is_none() && self.ui_cmd_rx.is_some() {
            if let Err(e) = self.spawn_win32_thread() {
                logger::error(
                    "AppShell",
                    &format!("win32 线程启动失败, 无监督退出面, 转退出: {}", e),
                );
                self.exit_requested = true;
            }
        }
        loop {
            match self.main_event_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(ev) => self.handle_main_event(ev),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return SupervisorOutcome::Exit,
            }
            self.pump();
            if self.exit_requested {
                return SupervisorOutcome::Exit;
            }
            if self.form_requested {
                return SupervisorOutcome::MainFormRequested;
            }
            // 托盘关于 (无 web 壳形态): Java 弹 About 通知, 此处日志兜底 (语义不丢)
            if self.take_about_request() {
                logger::info("AppShell", "托盘关于请求 (无 web 壳, About 弹窗不可用)");
            }
        }
    }

    /// 全量收尾: 旧核五步 → win32 线程 (先托盘 NIM_DELETE 后窗口, tray.rs 退出
    /// 契约) → 防抖线程 → 热键钩子。幂等 (controller 置 None / 各 join take 判空),
    /// Drop 兜底与显式调用双保险。
    pub fn shutdown(&mut self) {
        if let Some(old) = self.controller.as_mut() {
            old.stop(&mut self.release_main_form);
        }
        self.controller = None;
        let _ = self.ui_cmd_tx.send(UiCommand::Shutdown);
        if let Some(j) = self.win32.take() {
            let _ = j.join();
        }
        self.debounce.shutdown();
        if let Ok(mut hm) = self.hotkey.lock() {
            hm.shutdown();
        }
    }
}

impl Drop for AppShell {
    /// 兜底收尾 (审查 A-W4/B-W1): 不经 shutdown() 直接 drop (W2 iced 外部驱动
    /// 路径可能) 时, win32/防抖线程不泄漏、热键钩子必卸 — shutdown 幂等, 与
    /// run_supervisor 尾部的显式调用双保险, 二次调用全部空转。
    /// (win32 线程唯一出口是 UiCommand::Shutdown; 若仅 drop 发送端, 线程的
    /// try_recv 恒 Disconnected 但循环仍 10ms 空转 — 必须显式 send。)
    fn drop(&mut self) {
        self.shutdown();
    }
}

// =====================================================================
// Tests — 状态机转移 / stop 五步序 / 防过期 generation / debounce 时序
// (wf-p5-batch14 W1 验收单; 假时钟以短 debounce 间隔替代)
// =====================================================================
#[cfg(test)]
mod tests;
