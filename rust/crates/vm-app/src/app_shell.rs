//! vm-app 组装层 (P5 批十四 W1): AppShell + Controller 生命周期核心 + win32 线程入口。
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
//!   (单泵共享; 热键钩子线程豁免记录见 `win32_thread_main` 头注)。
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
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use vm_core::activation_strategy::{ActivationContext, ActivationStrategy};
use vm_core::bus::{EventBus, Subscription};
use vm_core::config_api::{ConfigProvider, HUDSettings, OverlaySettings};
use vm_core::configuration_service::{ConfigurationService, ControllerIntervals, UiStateEvent};
use vm_core::controller_state::ControllerState;
use vm_core::event::event_payload::EventPayload;
use vm_core::event::flight_data_event::FlightDataEvent;
use vm_core::event::ui_state_events;
use vm_core::flight_data_bus::FlightDataBus;
use vm_core::fm::{FMManager, FMStatus};
use vm_core::http_helper::HttpHelper;
use vm_core::hud_calculator::HudColors;
use vm_core::lang::Lang;
use vm_core::logger;

use vm_data::service_fields::ServiceData;
use vm_data::service_loop::{
    start as spawn_service_thread, Service, ServiceConfig, ServiceHandle,
};

use vm_overlay::host::OverlayHost;
use vm_overlay::hotkey::{HotkeyEvent, HotkeyManager, VC_P};
use vm_overlay::platform_extras::DpiHelper;
use vm_overlay::{
    engine_control_preview_spec, gear_flaps_preview_spec, minihud_overlay_spec,
    power_info_preview_spec, MiniHudHandle,
};

#[cfg(target_os = "windows")]
use vm_overlay::tray::{TrayConfig, TrayIcon, TrayHandler};

/// Java Controller.java:59 `CONFIG_DEBOUNCE_MS = 200`
pub const CONFIG_DEBOUNCE_MS: u64 = 200;

// =====================================================================
// Env — Application 静态只读区落位 (D8 表: 启动一次后只读 → 构造注入)
// =====================================================================

/// Application.java:60-129 静态字段中"启动一次后只读"组的落位
/// (LIFETIMES §1.2 → Env; 配置驱动可变组归 ConfigurationService 的 ApplicationState)。
#[derive(Debug, Clone)]
pub struct Env {
    /// Application.version (Java 读 MANIFEST; Rust 编译期注入)
    pub version: String,
    /// Application.appName = Lang.appName (Application.java:569)
    pub app_name: String,
    /// Application.httpHeader = Lang.httpHeader (:571)
    pub http_header: String,
    /// Application.appPort (Lang.httpPort parseInt, 失败 8111; :559-563)
    pub app_port: u16,
    /// Application.appPortBkp = appPort + 1111 (:564)
    pub app_port_bkp: u16,
    /// 字体目录探测 (Java initFont 的 AWT 注册 → Rust 字体文件路径供给, D8: 字体→win32 线程)
    pub fonts_dir: PathBuf,
    /// 托盘图标 (Application.initSystemTray: "image/16x16.png")
    pub icon_path: PathBuf,
    /// 屏幕快照 (Application.getScreenSize/DPIHelper; D8: 屏幕尺寸→win32 线程启动快照)
    pub dpi: DpiHelper,
    /// Application.debug (OverlayContext.isDebug 的来源)
    pub debug: bool,
}

impl Env {
    /// Java Application.main 启动序的只读区构造 (Lang → 端口 → 字体目录 → 屏幕探测)。
    pub fn probe(lang: &Lang, debug: bool) -> Env {
        // Java: try { appPort = parseInt(Lang.httpPort) } catch { 8111 }
        let app_port = lang.http_port.parse::<u16>().unwrap_or(8111);
        Env {
            version: env!("CARGO_PKG_VERSION").to_string(),
            app_name: lang.app_name.to_string(),
            http_header: lang.http_header.to_string(),
            app_port,
            // 域内恒 8111+1111=9222, u16 加法无回绕面 (Java int 同值)
            app_port_bkp: app_port + 1111,
            fonts_dir: probe_fonts_dir(),
            icon_path: PathBuf::from("image/16x16.png"),
            dpi: detect_dpi(),
            debug,
        }
    }
}

/// 字体目录探测: ./fonts → ../fonts (vm-overlay main.rs find_fonts_dir 同款,
/// 仓库根或 rust/ 下运行均可)
fn probe_fonts_dir() -> PathBuf {
    for cand in ["./fonts", "../fonts"] {
        if Path::new(cand).is_dir() {
            return PathBuf::from(cand);
        }
    }
    PathBuf::from("./fonts")
}

/// 仓库模板 ui_layout.cfg 探测 (CWD=仓库根 / rust/ 均可)
fn locate_template_cfg() -> Option<&'static str> {
    const CANDIDATES: [&str; 2] = ["ui_layout.cfg", "../ui_layout.cfg"];
    CANDIDATES.iter().find(|p| Path::new(p).exists()).copied()
}

/// Java Application.getScreenSize → DPIHelper.init() (DpiHelper.java:52)
#[cfg(target_os = "windows")]
fn detect_dpi() -> DpiHelper {
    DpiHelper::init()
}

/// 非 Windows: 屏幕探测未移植 (x11 波次), 100% 缩放回退 + 显式注明
#[cfg(not(target_os = "windows"))]
fn detect_dpi() -> DpiHelper {
    DpiHelper::fallback(1920, 1080, "非 Windows 屏幕探测未移植 (x11 波次)")
}

/// Java `Boolean.parseBoolean`: 忽略大小写等于 "true" 才为真
fn java_parse_boolean(s: &str) -> bool {
    s.eq_ignore_ascii_case("true")
}

/// Java `System.currentTimeMillis()` (service_loop 同款: u128→i64 截断, epoch 前取 0)
fn current_time_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// =====================================================================
// 跨线程命令/事件 (D8: 全部经 channel/bus)
// =====================================================================

/// UI→shell 命令。**按变体有唯一属主** (见各变体注释); 主线程侧经
/// [`AppShell::dispatch`] 路由, win32 侧变体由发送方直达 [`AppShell::ui_cmd_tx`]。
#[derive(Debug, Clone, PartialEq)]
pub enum UiCommand {
    /// MainForm.confirm "开始游戏" (MainForm.java:265-278) — **主线程属主**
    /// (MainForm 侧 vm-ui W2 接线调 `AppShell::dispatch`)。
    StartGame,
    /// MainForm 底部"结束游戏"按钮 (MainForm.java:92-98 保存语义) — **主线程属主**
    EndGame,
    /// Java OverlayManager.openAll (Controller.openpad, Controller.java:363) — win32 属主
    OpenAllOverlays,
    /// Java OverlayManager.closeAll (closepad/endPreview/stop 步1) — win32 属主
    CloseAllOverlays,
    /// WYSIWYG 刷新 (Java refreshPreviews(changedKey)/refreshAllPreviews) — win32 属主。
    /// `generation` = 发送时 previewGeneration 快照, win32 消费侧做防过期守卫
    /// (D8 修正★2: Java 在 ConfigDebounce 线程直碰 Swing 组件, Rust 改在本线程刷新)。
    /// `changed_key`: None = 全量刷新 (refreshAllPreviews / ACTION_RESET_COMPLETED)。
    RefreshPreviews {
        changed_key: Option<String>,
        generation: u64,
    },
    /// Java OverlayManager.reinitActiveOverlays (非 PREVIEW 态配置变更) — win32 属主
    ReinitActiveOverlays,
    /// win32 线程退出 (host 停泵 + 托盘 NIM_DELETE)
    Shutdown,
}

/// 托盘动作 (win32 线程 AppTrayHandler → 主线程监督循环)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    /// 左键/"设置" (Application.java:251-273: ctr.stop(); ctr = new Controller())
    Activate,
    /// 菜单"开始" (tray.rs 拆分入口: Controller 重建的服务启动部分)
    Start,
    /// 菜单"退出" (Application.java:229-235 close → System.exit(0) 的归属方)
    Exit,
}

/// 主线程监督循环消费的事件 (Controller 订阅闭包只转发不处理 — 配置 !Send,
/// 实际处理落在主线程 [`AppShell::handle_main_event`])
#[derive(Debug, Clone)]
pub enum MainEvent {
    /// UIStateBus CONFIG_CHANGED 载荷 (Java configChangedHandler 输入)
    ConfigChanged(String),
    /// UIStateBus UI_READY (MainForm 首显 → Controller.Preview)
    UiReady,
    /// FM_CHANGED 载荷摘要 (Java fmChangedHandler: toast + 防抖全量刷新)。
    /// name=Some 即 missing/corrupt (toast 面); name=None 为纯刷新调度信号
    FmChanged {
        name: Option<String>,
        corrupt: bool,
    },
    /// 托盘动作
    Tray(TrayCommand),
}

/// ConfigDebouncer 输入 (Java 两个 handler 共用 configDebouncer 的两种任务载荷)
#[derive(Debug, Clone, PartialEq)]
pub enum DebounceMsg {
    /// CONFIG_CHANGED 的配置键 (任务体: refreshPreviews(key))
    ConfigKey(String),
    /// FM_CHANGED (任务体: refreshAllPreviews)
    FmChanged,
}

// =====================================================================
// ControllerShared — 防过期世代号 + 跨线程可读状态快照
// =====================================================================

/// Controller 实例字段中需要跨线程读的部分。
/// PORT(世代号归属): Java previewGeneration 是 Controller 实例字段 (AtomicLong,
/// 每次托盘重建归零 — 旧核在途回调持旧世代号比对旧核, 靠 stop() 的 ++ 兜底);
/// Rust 收敛为 AppShell 级单调 (跨重建不重置), 防过期判定只会更严, 无假接受面。
pub struct ControllerShared {
    /// Java Controller.java:42 `AtomicLong previewGeneration`
    pub preview_generation: AtomicU64,
    /// Java `public ControllerState State` — 主线程写, win32 线程读 (stale 守卫)。
    /// Java 无锁靠 EDT 单线程; Rust 以 RwLock 承载跨线程读
    pub state: RwLock<ControllerState>,
    /// Java loadAppCheck 写入的轮询间隔组 (ConfigurationService.load_app_check 目标)
    pub intervals: Mutex<ControllerIntervals>,
    /// 低频杂项标志 (showStatus/sessionAircraftType/currentFmHotkeyCode)
    pub flags: Mutex<ControllerFlags>,
    /// 游戏模式 Service 数据快照句柄 (start() 建 / stop() 清;
    /// win32 线程 live 喂入 + 主线程 tick 驱动读)
    pub live: RwLock<Option<Arc<RwLock<ServiceData>>>>,
    /// OverlayContext.isPreviewMode 的跨线程替身 (Java: forPreviewMode/forGameMode
    /// 两种 ctx 构建; Rust 由 win32 命令处理点按操作语义设置)
    pub overlay_ctx_preview: AtomicBool,
}

/// Controller 低频杂项字段 (Java Controller.java:122-134/196)
#[derive(Debug, Clone)]
pub struct ControllerFlags {
    /// `private boolean showStatus` (loadFromConfig 同步; StatusBar 未移植, 仅保位)
    pub show_status: bool,
    /// `private String sessionAircraftType` (onAircraftChanged 幂等去重, Controller.java:196)
    pub session_aircraft_type: Option<String>,
    /// `private int currentFmHotkeyCode` (热键重绑定跟踪, Controller.java:153)
    pub current_fm_hotkey_code: i32,
}

impl Default for ControllerFlags {
    fn default() -> Self {
        // Java 字段声明默认值 (§2.10): false / null / 0
        ControllerFlags {
            show_status: false,
            session_aircraft_type: None,
            current_fm_hotkey_code: 0,
        }
    }
}

impl ControllerShared {
    pub fn new() -> Self {
        ControllerShared {
            preview_generation: AtomicU64::new(0),
            state: RwLock::new(ControllerState::Init),
            intervals: Mutex::new(ControllerIntervals::default()),
            flags: Mutex::new(ControllerFlags::default()),
            live: RwLock::new(None),
            overlay_ctx_preview: AtomicBool::new(true),
        }
    }

    /// 托盘重建新核前复位 (Java 构造器 L582 `State = ControllerState.INIT` 显式赋值)
    pub fn reset_for_rebuild(&self) {
        *self.state.write().expect("Controller 状态锁中毒") = ControllerState::Init;
    }

    /// State 快照读 (跨线程安全; 主线程写点: 各状态转移方法)
    pub fn state(&self) -> ControllerState {
        *self.state.read().expect("Controller 状态锁中毒")
    }

    fn set_state(&self, s: ControllerState) {
        *self.state.write().expect("Controller 状态锁中毒") = s;
    }
}

impl Default for ControllerShared {
    fn default() -> Self {
        Self::new()
    }
}

/// 防过期守卫 (win32 线程消费 UiCommand::RefreshPreviews 时调用)。
/// PORT: Java Controller.refreshPreviews 的 invokeLater 内守卫
/// (Controller.java:894: `State != PREVIEW || previewGeneration.get() != generation`)。
/// Java 防抖路径 (configChanged/fmChanged 任务体) 无此守卫 (★2 违规波及面),
/// Rust 统一经本守卫 — 世代号不匹配或已离开 PREVIEW 即丢弃。
pub fn is_stale_refresh(shared: &ControllerShared, generation: u64) -> bool {
    let state = shared.state();
    let current = shared.preview_generation.load(Ordering::SeqCst);
    if state != ControllerState::Preview || current != generation {
        logger::info(
            "Controller",
            &format!(
                "Skipping stale preview refresh (gen={}, current={}, state={})",
                generation, current, state
            ),
        );
        true
    } else {
        false
    }
}

// =====================================================================
// ConfigDebouncer — Java static configDebouncer 的线程化 (Controller.java:52-59)
// =====================================================================

/// 单线程防抖器: 安静期 `delay` 内的最后一条消息触发一次 RefreshPreviews。
/// PORT(Java 语义): `pendingConfigRefresh.cancel(false)` + `schedule(200ms)`
/// —— 新变更取消未执行任务并重排, 只有最后一次变更生效。
/// 跨 Controller 重建共享 (Java static; Rust 由 AppShell 持有, tx 分发进各核)。
pub struct ConfigDebouncer {
    tx: Sender<DebounceMsg>,
    join: Option<JoinHandle<()>>,
}

impl ConfigDebouncer {
    /// `delay` 可注入 (测试用短间隔; 生产 [`CONFIG_DEBOUNCE_MS`])。
    /// 输出直送 win32 线程 UiCommand 通道 (D8 修正★2: 刷新动作离开本线程)。
    pub fn spawn(delay: Duration, out: Sender<UiCommand>, shared: Arc<ControllerShared>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<DebounceMsg>();
        let join = std::thread::Builder::new()
            .name("ConfigDebounce".to_string()) // Java 线程名 "ConfigDebounce"
            .spawn(move || {
                while let Ok(first) = rx.recv() {
                    let mut last = first;
                    // 安静期窗口: 每到一条即重排 (cancel+reschedule 的电平等价)
                    loop {
                        match rx.recv_timeout(delay) {
                            Ok(next) => last = next,
                            Err(RecvTimeoutError::Timeout) => break,
                            Err(RecvTimeoutError::Disconnected) => return,
                        }
                    }
                    // Java 防抖任务体 (Controller.java:525-536/573-576):
                    // loadFromConfig() + refreshPreviews(key)/refreshAllPreviews()。
                    // loadFromConfig 已挪至主线程调度点 (配置 !Send, 见模块头);
                    // 此处只取世代号快照送刷新命令, 消费侧 win32 做守卫。
                    let generation = shared.preview_generation.load(Ordering::SeqCst);
                    let changed_key = match last {
                        DebounceMsg::ConfigKey(ref k)
                            if k == ui_state_events::ACTION_RESET_COMPLETED =>
                        {
                            None // 全局重置: refreshAllPreviews (Controller.java:530)
                        }
                        DebounceMsg::ConfigKey(k) => Some(k),
                        DebounceMsg::FmChanged => None, // FM_CHANGED: refreshAllPreviews
                    };
                    let _ = out.send(UiCommand::RefreshPreviews {
                        changed_key,
                        generation,
                    });
                }
            })
            .expect("ConfigDebounce 线程创建失败");
        ConfigDebouncer {
            tx,
            join: Some(join),
        }
    }

    pub fn sender(&self) -> Sender<DebounceMsg> {
        self.tx.clone()
    }

    pub fn shutdown(&mut self) {
        // drop(tx) 使 recv 返回 Disconnected, 线程自然退出后 join
        if let Some(j) = self.join.take() {
            drop(self.tx.clone());
            let _ = j.join();
        }
    }
}

impl Drop for ConfigDebouncer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// =====================================================================
// HudSettingsSnapshot — HUDSettings 的 Send 快照 (win32 线程注册面输入)
// =====================================================================

/// MiniHUD 注册所需的 HUDSettings 全量值快照。
/// PORT: ConfigurationService (!Send, Rc<SExp> 配置树) 不能进 win32 线程,
/// 主线程 (AppShell) 构建本纯值快照随 [`Win32ThreadConfig`] 送入。
/// `get_window_x/y`: 窗口定位归 OverlayHost 位置存档 (host.materialize),
/// ctx.window_x/y 在 Rust 端无消费点 — 返回 0 (保位)。
#[derive(Debug, Clone)]
pub struct HudSettingsSnapshot {
    pub num_font: String,
    pub crosshair_scale: i32,
    pub crosshair_name: String,
    pub display_crosshair: bool,
    pub use_texture_crosshair: bool,
    pub draw_hud_text: bool,
    pub show_attitude_gauge: bool,
    pub aoa_warning_ratio: f64,
    pub aoa_bar_warning_ratio: f64,
    pub enable_flap_angle_bar: bool,
    pub show_speed_bar: bool,
    pub draw_hud_mach: bool,
    pub speed_label_disabled: bool,
    pub altitude_label_disabled: bool,
    pub sep_label_disabled: bool,
    pub show_hud_speed: bool,
    pub show_hud_aoa: bool,
    pub show_hud_altitude: bool,
    pub show_hud_energy: bool,
    pub show_hud_mechanization: bool,
    pub show_hud_flaps: bool,
    pub show_hud_airbrake: bool,
    pub show_hud_gear: bool,
    pub show_hud_sep: bool,
    pub show_hud_g_load: bool,
    pub show_hud_maneuver_bar: bool,
    pub attitude_indicator_inertial_mode: bool,
    pub gpu_compatibility_mode: bool,
    pub always_show_radar_altitude: bool,
    pub font_name: String,
    pub num_font_name: String,
    pub font_size_add: i32,
    pub auto_hide_on_focus_loss: bool,
    /// 通用 bool getter 快照 (minihud initModernLayout 读 "enableLayoutDebug")
    pub bools: HashMap<String, bool>,
}

impl HudSettingsSnapshot {
    /// 主线程从真实设置视图提取 (调用点持 ConfigurationService)
    pub fn build<S: HUDSettings>(s: &S) -> Self {
        HudSettingsSnapshot {
            num_font: s.get_num_font(),
            crosshair_scale: s.get_crosshair_scale(),
            crosshair_name: s.get_crosshair_name(),
            display_crosshair: s.is_display_crosshair(),
            use_texture_crosshair: s.use_texture_crosshair(),
            draw_hud_text: s.draw_hud_text(),
            show_attitude_gauge: s.show_attitude_gauge(),
            aoa_warning_ratio: s.get_aoa_warning_ratio(),
            aoa_bar_warning_ratio: s.get_aoa_bar_warning_ratio(),
            enable_flap_angle_bar: s.enable_flap_angle_bar(),
            show_speed_bar: s.show_speed_bar(),
            draw_hud_mach: s.draw_hud_mach(),
            speed_label_disabled: s.is_speed_label_disabled(),
            altitude_label_disabled: s.is_altitude_label_disabled(),
            sep_label_disabled: s.is_sep_label_disabled(),
            show_hud_speed: s.show_hud_speed(),
            show_hud_aoa: s.show_hud_aoa(),
            show_hud_altitude: s.show_hud_altitude(),
            show_hud_energy: s.show_hud_energy(),
            show_hud_mechanization: s.show_hud_mechanization(),
            show_hud_flaps: s.show_hud_flaps(),
            show_hud_airbrake: s.show_hud_airbrake(),
            show_hud_gear: s.show_hud_gear(),
            show_hud_sep: s.show_hud_sep(),
            show_hud_g_load: s.show_hud_g_load(),
            show_hud_maneuver_bar: s.show_hud_maneuver_bar(),
            attitude_indicator_inertial_mode: s.is_attitude_indicator_inertial_mode(),
            gpu_compatibility_mode: s.is_gpu_compatibility_mode(),
            always_show_radar_altitude: s.always_show_radar_altitude(),
            font_name: s.get_font_name(),
            num_font_name: s.get_num_font_name(),
            font_size_add: s.get_font_size_add(),
            auto_hide_on_focus_loss: s.auto_hide_on_focus_loss(),
            // minihud 走通用 getter 的键集 (initModernLayout L1293; 新键随接线补)
            bools: HashMap::from([(
                "enableLayoutDebug".to_string(),
                s.get_bool("enableLayoutDebug", false),
            )]),
        }
    }
}

impl OverlaySettings for HudSettingsSnapshot {
    type GroupConfig = ();
    fn get_window_x(&self, _width: i32) -> i32 {
        0 // 定位归 host 位置存档 (见类型注)
    }
    fn get_window_y(&self, _height: i32) -> i32 {
        0
    }
    fn save_window_position(&self, _x: f64, _y: f64) {
        // host.saved_positions 接管 (host.rs close 链), 无回写面
    }
    fn get_font_name(&self) -> String {
        self.font_name.clone()
    }
    fn get_num_font_name(&self) -> String {
        self.num_font_name.clone()
    }
    fn get_font_size_add(&self) -> i32 {
        self.font_size_add
    }
    fn get_bool(&self, key: &str, def: bool) -> bool {
        self.bools.get(key).copied().unwrap_or(def)
    }
    fn get_int(&self, _key: &str, def: i32) -> i32 {
        def
    }
    fn get_string(&self, _key: &str, def: &str) -> String {
        def.to_string()
    }
    fn get_group_config(&self) -> Option<&Self::GroupConfig> {
        None
    }
    fn auto_hide_on_focus_loss(&self) -> bool {
        self.auto_hide_on_focus_loss
    }
}

impl HUDSettings for HudSettingsSnapshot {
    fn get_num_font(&self) -> String {
        self.num_font.clone()
    }
    fn get_crosshair_scale(&self) -> i32 {
        self.crosshair_scale
    }
    fn get_crosshair_name(&self) -> String {
        self.crosshair_name.clone()
    }
    fn is_display_crosshair(&self) -> bool {
        self.display_crosshair
    }
    fn use_texture_crosshair(&self) -> bool {
        self.use_texture_crosshair
    }
    fn draw_hud_text(&self) -> bool {
        self.draw_hud_text
    }
    fn show_attitude_gauge(&self) -> bool {
        self.show_attitude_gauge
    }
    fn get_aoa_warning_ratio(&self) -> f64 {
        self.aoa_warning_ratio
    }
    fn get_aoa_bar_warning_ratio(&self) -> f64 {
        self.aoa_bar_warning_ratio
    }
    fn enable_flap_angle_bar(&self) -> bool {
        self.enable_flap_angle_bar
    }
    fn show_speed_bar(&self) -> bool {
        self.show_speed_bar
    }
    fn draw_hud_mach(&self) -> bool {
        self.draw_hud_mach
    }
    fn is_speed_label_disabled(&self) -> bool {
        self.speed_label_disabled
    }
    fn is_altitude_label_disabled(&self) -> bool {
        self.altitude_label_disabled
    }
    fn is_sep_label_disabled(&self) -> bool {
        self.sep_label_disabled
    }
    fn show_hud_speed(&self) -> bool {
        self.show_hud_speed
    }
    fn show_hud_aoa(&self) -> bool {
        self.show_hud_aoa
    }
    fn show_hud_altitude(&self) -> bool {
        self.show_hud_altitude
    }
    fn show_hud_energy(&self) -> bool {
        self.show_hud_energy
    }
    fn show_hud_mechanization(&self) -> bool {
        self.show_hud_mechanization
    }
    fn show_hud_flaps(&self) -> bool {
        self.show_hud_flaps
    }
    fn show_hud_airbrake(&self) -> bool {
        self.show_hud_airbrake
    }
    fn show_hud_gear(&self) -> bool {
        self.show_hud_gear
    }
    fn show_hud_sep(&self) -> bool {
        self.show_hud_sep
    }
    fn show_hud_g_load(&self) -> bool {
        self.show_hud_g_load
    }
    fn show_hud_maneuver_bar(&self) -> bool {
        self.show_hud_maneuver_bar
    }
    fn is_attitude_indicator_inertial_mode(&self) -> bool {
        self.attitude_indicator_inertial_mode
    }
    fn is_gpu_compatibility_mode(&self) -> bool {
        self.gpu_compatibility_mode
    }
    fn always_show_radar_altitude(&self) -> bool {
        self.always_show_radar_altitude
    }
}

// =====================================================================
// 激活缓存 + 注册参数快照 (win32 线程的配置面)
// =====================================================================

/// 激活策略引用的全部配置键 (Java registerGameModeOverlays 的
/// ActivationStrategy.config(...) 实参 + 复合策略依赖键)
pub const ACTIVATION_KEYS: [&str; 9] = [
    "enableEngineControl",
    "engineInfoSwitch",
    "crosshairSwitch",
    "flightInfoSwitch",
    "enableAxis",
    "enableAttitudeIndicator",
    "enablegearAndFlaps",
    "enableVoiceWarn",
    "enableFMPrint",
];

/// key → 原始配置串 (get_config 值域, Some("") 表缺失 — ConfigurationService 先例)。
/// 主线程刷新 (rebuild + 每次 CONFIG_CHANGED), win32 激活探测读。
pub type ActivationCache = Arc<Mutex<HashMap<String, String>>>;

/// 主线程从配置服务重建激活缓存 (Java: shouldActivate 经 ctx.get_bool →
/// configProvider.getConfig 实时读; Rust 以"每次配置变更即刷新缓存"等价,
/// 配置写点必发 CONFIG_CHANGED, 最后写胜出)
fn refresh_activation_cache(config: &ConfigurationService, cache: &ActivationCache) {
    let mut m = cache.lock().expect("激活缓存锁中毒");
    for key in ACTIVATION_KEYS {
        m.insert(key.to_string(), config.get_config(key).unwrap_or_default());
    }
}

/// overlay 注册面的 Send 参数快照 (win32 线程一次性注册用, D8: 字体→win32 线程)
pub struct OverlayInputs {
    pub dpi_scale: f64,
    /// MiniHUD 全量设置快照
    pub hud: HudSettingsSnapshot,
    /// 引擎控制面板字号增量 (getOverlaySettings("引擎控制").get_font_size_add)
    pub font_add_engine: i32,
    /// 动力信息字号增量 + 列数 (getOverlaySettings("动力信息"))
    pub font_add_power: i32,
    pub power_columns: i32,
    /// 起落襟翼字号增量 + 边缘模式 (getOverlaySettings("起落襟翼"))
    pub font_add_gear: i32,
    pub gear_show_edge: bool,
    /// Service 轮询间隔 (MiniHUD blinkTicks/refreshInterval 同源)
    pub service_loop_interval_ms: i64,
}

impl OverlayInputs {
    /// 主线程构建 (调用点持 ConfigurationService + Env + shared)
    pub fn build(config: &ConfigurationService, env: &Env, shared: &ControllerShared) -> Self {
        let interval = shared
            .intervals
            .lock()
            .expect("intervals 锁中毒")
            .service_loop_interval_ms;
        let engine = config.get_overlay_settings("引擎控制");
        let power = config.get_overlay_settings("动力信息");
        let gear = config.get_overlay_settings("起落襟翼");
        OverlayInputs {
            dpi_scale: env.dpi.get_scale(),
            hud: HudSettingsSnapshot::build(&config.get_hud_settings()),
            font_add_engine: engine.get_font_size_add(),
            font_add_power: power.get_font_size_add(),
            power_columns: power.get_int("hudColumns", 1),
            font_add_gear: gear.get_font_size_add(),
            gear_show_edge: gear.get_bool("enablegearAndFlapsEdge", false),
            // load_app_check 缺省 50 (ConfigurationService.java 同源)
            service_loop_interval_ms: if interval > 0 { interval } else { 50 },
        }
    }
}

// =====================================================================
// Controller — Java src/prog/Controller.java 的生命周期核 (主线程独占)
// =====================================================================

/// Controller 构造依赖 (AppShell 分发; 对位 Java 构造器从单例/静态取的全部输入)
pub struct ControllerDeps {
    pub config: ConfigurationService,
    pub ui_bus: Arc<EventBus<UiStateEvent>>,
    pub flight_bus: Arc<FlightDataBus>,
    pub fm: Arc<FMManager>,
    pub hotkey: Arc<Mutex<HotkeyManager>>,
    pub shared: Arc<ControllerShared>,
    pub ui_cmd_tx: Sender<UiCommand>,
    pub debounce_tx: Sender<DebounceMsg>,
    pub main_event_tx: Sender<MainEvent>,
    pub env: Env,
}

/// 可重建应用核 (Java Controller; 恒留主线程 — config 字段 !Send)
pub struct Controller {
    pub config: ConfigurationService,
    shared: Arc<ControllerShared>,
    fm: Arc<FMManager>,
    flight_bus: Arc<FlightDataBus>,
    hotkey: Arc<Mutex<HotkeyManager>>,
    ui_cmd_tx: Sender<UiCommand>,
    debounce_tx: Sender<DebounceMsg>,
    env: Env,
    /// stop 步2 退订的订阅句柄 (RAII Drop = unsubscribe, 对位 Java unsubscribe+置 null)
    subs: Vec<Subscription<UiStateEvent>>,
    fm_sub: Option<Subscription<vm_core::fm::FMHandle>>,
    /// Service 线程句柄 (stop 步4: take + stop)
    pub service: Option<ServiceHandle>,
    /// Java `public MainForm M` 的存活位 (真窗归主线程 iced/W2; 此处只承载 null 判定)
    main_form_alive: bool,
}

impl Controller {
    /// Java Controller(boolean isInitialLaunch) 构造器 (Controller.java:469-610)。
    /// PORT(侧序): configService.initConfig() 与 initDynamicOverlays 的文件装载
    /// 挪至 AppShell 构造面 (配置服务先于 Controller 存在, 免测试写盘副作用);
    /// overlayManager 注册挪至 win32 线程一次性注册 (host 跨重建存活, 条目为
    /// 无状态配置记录 — 见 register_game_mode_overlays 头注)。
    pub fn new(deps: ControllerDeps, is_initial_launch: bool) -> Controller {
        let ControllerDeps {
            config,
            ui_bus,
            flight_bus,
            fm,
            hotkey,
            shared,
            ui_cmd_tx,
            debounce_tx,
            main_event_tx,
            env,
        } = deps;

        // Java:474 loadFromConfig() (同步本地标志 + loadAppCheck)
        load_from_config(&config, &shared);

        // Java:477-478 HotkeyManager.getInstance().init()
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
            ui_cmd_tx,
            debounce_tx,
            env,
            subs: Vec::new(),
            fm_sub: None,
            service: None,
            main_form_alive: false,
        };
        c.bind_fm_hotkey_initial(); // Java:479-489

        // Java:498-545 订阅 CONFIG_CHANGED (闭包只转发 — 配置 !Send, 处理在主线程
        // AppShell::handle_main_event, 对位 Java handler 内联执行的语义)
        let tx = main_event_tx.clone();
        c.subs.push(ui_bus.subscribe(move |ev: &UiStateEvent| {
            // 桩总线无路由 (bus.rs 广播), 订阅方按 event_type 过滤
            if ev.event_type == ui_state_events::CONFIG_CHANGED {
                let _ = tx.send(MainEvent::ConfigChanged(ev.data.clone()));
            }
        }));
        // Java:547-552 订阅 UI_READY → Preview()
        let tx = main_event_tx.clone();
        c.subs.push(ui_bus.subscribe(move |ev: &UiStateEvent| {
            if ev.event_type == ui_state_events::UI_READY {
                let _ = tx.send(MainEvent::UiReady);
            }
        }));
        // Java:554-579 订阅 FM_CHANGED (FMManager 专用强类型通道, fm_manager.rs 裁决)
        let tx = main_event_tx.clone();
        c.fm_sub = Some(fm.fm_changed_bus().subscribe(move |handle| {
            // Java:560-567 missing/corrupt → 右下角 toast (NotificationService 未移植,
            // 摘要转发由主线程记日志, TODO(port) 接 toast)
            if handle.is_missing_like() {
                let _ = tx.send(MainEvent::FmChanged {
                    name: handle.name.clone(),
                    corrupt: handle.status == FMStatus::Corrupt,
                });
            }
            // Java:568-577 Preview 态 → 复用 configDebouncer 全量刷新。
            // PREVIEW 判定收敛到主线程 handle_main_event (状态真值在主线程)
            let _ = tx.send(MainEvent::FmChanged {
                name: None,
                corrupt: false,
            });
        }));
        let _ = c.debounce_tx; // (依赖面保位: 防抖调度经 AppShell 的 debouncer)

        // Java:582 State = INIT (AppShell.reset_for_rebuild 已置); lastEvt/lastDmg=0 无对应物

        // Java:589-609 自启动判定 (仅 initial launch)
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
            c.spawn_fm_detect(); // Java:601 new Thread(this::detectAndIdentify, "FM-Detect")
            c.start(&mut || {}); // M 恒 null (自启动路径), 释放步空转
        } else {
            // Java:604 M = new MainForm(this) — 真窗归 W2 主线程 iced; 此处记存活位
            c.main_form_alive = true;
            c.spawn_fm_detect(); // Java:608
        }
        c
    }

    /// Java:479-489 构造器内的 FM 热键绑定
    fn bind_fm_hotkey_initial(&mut self) {
        let enable =
            java_parse_boolean(&self.config.get_config("enableFMPrint").unwrap_or_default());
        // Java: try { parseInt(displayFmKey) } catch { VC_P }
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
        // Java:489 Application.displayFmKey 同步 — Rust 侧 ApplicationState 由
        // load_app_check 自配置维护, 不双写
    }

    /// Java:601/608 "FM-Detect" 一次性线程 → detectAndIdentify (865-877)。
    /// PORT: selectedFM0 在主线程预读 (配置 !Send 不入线程); HttpHelper/FMManager Send。
    fn spawn_fm_detect(&self) {
        let selected = self.config.get_config("selectedFM0").unwrap_or_default();
        let http_header = self.env.http_header.clone();
        let fm = Arc::clone(&self.fm);
        std::thread::Builder::new()
            .name("FM-Detect".to_string())
            .spawn(move || detect_and_identify(&selected, &http_header, &fm))
            .expect("FM-Detect 线程创建失败");
    }

    /// Java:447-454 loadFromConfig — loadAppCheck + showStatus 同步
    fn load_from_config_(&self) {
        load_from_config(&self.config, &self.shared);
    }

    /// Java:823-847 handleFmHotkeyConfigChange — 解绑旧键/绑新键
    fn handle_fm_hotkey_config_change(&self) {
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
            return; // Java: if (State == ControllerState.INIT) 守卫
        }
        // Java:619-623 M != null → stopRepaintTimer + dispose + M=null
        if self.main_form_alive {
            release_main_form();
            self.main_form_alive = false;
        }
        // Java:627 System.gc() — 无对应物
        logger::info("Controller", "--------------------------------------------------");
        logger::info("Controller", "ACTION: Starting Game Mode Services...");
        logger::info("Controller", "--------------------------------------------------");
        // Java:633-637 S = new Service(this); S1 = new Thread(S); S1.start()
        // (MAX_PRIORITY 不可移植 — service_loop.rs 同注)
        let interval = self
            .shared
            .intervals
            .lock()
            .expect("intervals 锁中毒")
            .service_loop_interval_ms;
        let service = Service::new(
            ServiceConfig {
                // load_app_check 缺省 50; 字段 0 = 未跑过 loadFromConfig 的防御回退
                service_loop_interval_ms: if interval > 0 { interval } else { 50 },
                app_port: self.env.app_port,
                http_header: self.env.http_header.clone(),
            },
            Arc::clone(&self.fm),
            // FlightDataBus 单例语义 (LIFETIMES §1.1): AppShell 分发同一 Arc
            Arc::clone(&self.flight_bus),
        );
        let handle = spawn_service_thread(service);
        *self.shared.live.write().expect("live 锁中毒") = Some(Arc::clone(&handle.data));
        self.service = Some(handle);
        // Java:640-641 进游戏模式即存配置
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
        // 2. 取消事件订阅 (防重建后旧实例响应; RAII Drop = unsubscribe, Java 781-795)
        self.subs.clear();
        self.fm_sub = None;
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
        let generation = self.shared.preview_generation.load(Ordering::SeqCst); // Java:852 capture
        // Java:853-856 new Thread(() -> refreshPreviews(generation)).start()
        // PORT: loadFromConfig 在主线程先行 (Java 在后台线程写 Controller 字段 =
        // ★2 违规族; 值等价 — 配置已由发布方写毕), 线程内只做网络探测 + 送命令
        self.load_from_config_();
        let selected = self.config.get_config("selectedFM0").unwrap_or_default();
        let http_header = self.env.http_header.clone();
        let fm = Arc::clone(&self.fm);
        let tx = self.ui_cmd_tx.clone();
        std::thread::Builder::new()
            .name("Preview-Refresh".to_string())
            .spawn(move || {
                logger::debug(
                    "Controller",
                    "Refreshing overlays for preview/config change...",
                );
                detect_and_identify(&selected, &http_header, &fm);
                // Java:892-901 invokeLater + stale 守卫 → Rust: 送 win32 线程消费侧守卫
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
        self.config.save_config(); // Java:917 显式保存
        self.shared.set_state(ControllerState::Init);
        // Java:919 System.gc() — 无对应物
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
    // vm-data 侧 TODO(port) 调用点由 AppShell::tick/pump 顶替, 见 drive_from_live)
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
        // Java:214 FMManager.identify(S.sIndic.type)
        self.fm.identify(indic_type);
        // Java:221-226 SB 释放 (StatusBar 未移植); Java:228-233 debug OtherService 未移植
        self.shared.set_state(ControllerState::Preview);
        // Java:237-246 延迟 100ms 建 overlay 防数据闪烁 (小睡线程 + openpad)
        let tx = self.ui_cmd_tx.clone();
        std::thread::Builder::new()
            .name("Openpad-Delay".to_string())
            .spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                // openpad 的 overlay 面 (Java:363 openAll); 其余面见 openpad_rest
                let _ = tx.send(UiCommand::OpenAllOverlays);
            })
            .expect("Openpad-Delay 线程创建失败");
        self.openpad_rest();
    }

    /// Java openpad (344-386) 中非 overlay 窗口的其余面
    fn openpad_rest(&mut self) {
        // Java:352-360 autoHideOnFocusLoss → S.getFocusMonitor().setEnabled
        // TODO(port): FocusMonitor 由 Service 线程内持有 (vm-data), 外部开关面缺,
        // 接线需 vm-data 补回调 (同 drive_from_live 的 TODO 族)
        let auto_hide_cfg = self.config.get_config("autoHideOnFocusLoss").unwrap_or_default();
        logger::info(
            "Controller",
            &format!("autoHideOnFocusLoss 配置值: {}", auto_hide_cfg),
        );
        if java_parse_boolean(&auto_hide_cfg) {
            logger::info("Controller", "焦点监控已启用 (接线待 vm-data 回调面)");
        } else {
            logger::info("Controller", "焦点监控未启用（配置为 false 或未设置）");
        }
        // Java:366-376 FlightLog (enableLogging) — FlightLog/NotificationService/
        //   DrawFrame 未接 (TODO(port), D8 降级清单 DrawFrame×2 属 P6)
        // Java:378-382 UIThread — D7 弃译清单 (空转轮询线程已废)
        // Java:383-385 S.startTime — Service 内部时间面, vm-data 未外泄 (TODO(port))
    }

    /// Java closepad (388-421) — overlay 关闭 (命令) + 其余收尾面
    pub fn closepad(&mut self) {
        // Java:390 FocusMonitor disable (TODO(port) 同上)
        let _ = self.ui_cmd_tx.send(UiCommand::CloseAllOverlays); // Java:400 closeAll
        // Java:402-411 FlightLog 保存/DrawFrame (TODO(port) P6)
        // Java:413-418 UIThread 停 (D7 弃译); Java:420 System.gc() 无对应物
    }

    /// Java:251-283 S4toS1 — PREVIEW → INIT (退出游戏)。
    pub fn s4to_s1(&mut self) {
        if self.shared.state() != ControllerState::Preview {
            return;
        }
        self.closepad();
        // Java:271 S.clear() — vm-data 未外泄 (TODO(port))
        // Java:275-276 会话结束: 清识别目标 (保留已加载句柄秒开) + 会话机型记忆
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
            return; // Java:299-300
        }
        let mut flags = self.shared.flags.lock().expect("flags 锁中毒");
        if flags.session_aircraft_type.as_deref() == Some(t) {
            return; // Java:302 幂等守卫 (10Hz 轮询安全)
        }
        // Java:304-306 null = 会话首机: 只记名不切换
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
        // Java:313-334 enableLogging → FlightLog 关旧开新 (TODO(port), P6 DrawFrame 族)
        // Java:336-341 S.resetvaria() (vm-data 未外泄, TODO(port))
    }

    /// Service 轮询驱动的状态机推进 (AppShell::pump 调用)。
    /// PORT: Java Service.processPollingCycle 内联调用 c.initStatusBar/changeS2/
    /// changeS3/S4toS1 (vm-data service_loop.rs 对应位置留 TODO(port) — 本方法以
    /// ServiceData 公开字段顶替该调用面)。strState/strIndic 原始串在 HttpHelper
    /// 内部不可见: "串空" 分支 (Java:755-761 直达 S4toS1) 与 "flag 丢失" 分支
    /// (Java:746-754 同样 S4toS1) 合并 — 终态一致, 仅丢失 CONNECTED→IN_GAME 瞬态。
    pub fn drive_from_live(&mut self) {
        let live = self.shared.live.read().expect("live 锁中毒").clone();
        let Some(data) = live else { return };
        let d = data.read().unwrap_or_else(|e| e.into_inner()); // 中毒穿透 (§6 契约)
        let s_flag = d.s_state.as_ref().map(|s| s.flag).unwrap_or(false);
        let i_flag = d.s_indic.as_ref().map(|i| i.flag).unwrap_or(false);
        let i_type = d.s_indic.as_ref().and_then(|i| i.r#type.clone());
        let player_live = d.player_live;
        drop(d);
        self.init_status_bar(); // Java:570 (每轮, 守卫在方法内)
        if s_flag && i_flag {
            self.change_s2(); // Java:598
            if player_live {
                let t = i_type.clone();
                self.change_s3(t.as_deref()); // Java:649 打开面板
                self.on_aircraft_changed(i_type.as_deref()); // Java:668 换机
            }
            // else: Java 649 前的 playerLive 探测等待, 无 Controller 调用
        } else {
            self.s4to_s1(); // Java:750/758 两条 S4toS1 路径的合并
        }
    }

    /// Controller 状态快照 (主线程读)
    pub fn state(&self) -> ControllerState {
        self.shared.state()
    }
}

/// Java:447-454 loadFromConfig (独立函数: 订阅转发面不持 config, 主线程统一调用)
fn load_from_config(config: &ConfigurationService, shared: &ControllerShared) {
    // Java:448 configService.loadAppCheck(this) — 间隔组 + ApplicationState
    let mut intervals = shared.intervals.lock().expect("intervals 锁中毒");
    config.load_app_check(&mut intervals);
    drop(intervals);
    // Java:449-453 showStatus = true; enableStatusBar 非空则 parseBoolean
    let mut flags = shared.flags.lock().expect("flags 锁中毒");
    flags.show_status = true;
    if let Some(v) = config.get_config("enableStatusBar") {
        if !v.is_empty() {
            flags.show_status = java_parse_boolean(&v);
        }
    }
}

/// Java:865-877 detectAndIdentify — live 机型探测 → selectedFM0 兜底 → identify。
fn detect_and_identify(selected_fm0: &str, http_header: &str, fm: &FMManager) {
    // getLiveAircraftType 自带异常兜底 (失败/无游戏 → None)
    let fetcher = HttpHelper::new(http_header);
    let live = fetcher.get_live_aircraft_type();
    let target = live.unwrap_or_else(|| selected_fm0.to_string());
    if !target.is_empty() {
        fm.identify(Some(&target));
    }
}

// =====================================================================
// AppShell — Application 静态态收敛 + 监督循环 (D8)
// =====================================================================

/// AppShell 装配输入 ([`AppShell::with_parts`] 注入面, 测试用 tmp 配置等)
pub struct ShellParts {
    pub env: Env,
    /// 初始配置服务 (生产: new + initConfig; 测试: tmp cfg load_layout)
    pub config: ConfigurationService,
    pub ui_bus: Arc<EventBus<UiStateEvent>>,
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
    pub ui_bus: Arc<EventBus<UiStateEvent>>,
    pub flight_bus: Arc<FlightDataBus>,
    pub fm: Arc<FMManager>,
    pub hotkey: Arc<Mutex<HotkeyManager>>,
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
}

impl AppShell {
    /// 生产构造 (Java Application.main:533-604 启动序):
    /// Lang → 端口/Env → 总线/FM/热键 → 防抖 → 初始 Controller(true)。
    pub fn new(debug: bool) -> Result<AppShell, String> {
        let lang = Lang::init_lang();
        let env = Env::probe(&lang, debug);
        let ui_bus = Arc::new(EventBus::new());
        let config = ConfigurationService::new(Some(Arc::clone(&ui_bus)));
        // Java Controller 构造器: configService.initConfig() 装载设置文件
        config.init_config();
        let (hotkey, hotkey_rx) = HotkeyManager::with_channel();
        let mut shell = AppShell::with_parts(ShellParts {
            env,
            config,
            ui_bus,
            flight_bus: Arc::new(FlightDataBus::new()),
            fm: Arc::new(FMManager::new(Arc::new(EventBus::new()))),
            hotkey,
            hotkey_rx,
            debounce_delay: Duration::from_millis(CONFIG_DEBOUNCE_MS),
        });
        shell.rebuild_controller(true); // Java:590 ctr = new Controller(true)
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
        let debounce =
            ConfigDebouncer::spawn(debounce_delay, ui_cmd_tx.clone(), Arc::clone(&shared));
        AppShell {
            env,
            ui_bus,
            flight_bus,
            fm,
            hotkey: Arc::new(Mutex::new(hotkey)),
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
        }
    }

    /// 托盘重建/初始构造 (Java Application.java:251-273 mouseClicked 与 main:590)。
    /// `is_initial_launch`: true=初始启动 (尊重 autoStartGameMode), false=托盘恢复
    /// (恒弹设置窗语义 — Java Controller(false))。
    pub fn rebuild_controller(&mut self, is_initial_launch: bool) {
        if let Some(old) = self.controller.as_mut() {
            old.stop(&mut self.release_main_form); // 旧核五步销毁
        }
        // Java:470 每核 new ConfigurationService + initConfig (配置树随核重建)
        let config = ConfigurationService::new(Some(Arc::clone(&self.ui_bus)));
        config.init_config();
        // 模板回退 (vm-ui main.rs 同款分歧备案: CWD 无用户 cfg 时以仓库模板自愈)
        if config.get_layout_configs().is_none_or(|g| g.is_empty()) {
            match locate_template_cfg() {
                Some(p) => {
                    logger::warn("AppShell", &format!("CWD 无用户配置, 回退模板 {}", p));
                    config.load_layout(p);
                }
                None => logger::warn("AppShell", "未找到 ui_layout.cfg, 配置面为空"),
            }
        }
        refresh_activation_cache(&config, &self.activation); // win32 激活面同步
        self.shared.reset_for_rebuild(); // Java:582 State = INIT
        self.controller = Some(Controller::new(
            ControllerDeps {
                config,
                ui_bus: Arc::clone(&self.ui_bus),
                flight_bus: Arc::clone(&self.flight_bus),
                fm: Arc::clone(&self.fm),
                hotkey: Arc::clone(&self.hotkey),
                shared: Arc::clone(&self.shared),
                ui_cmd_tx: self.ui_cmd_tx.clone(),
                debounce_tx: self.debounce.sender(),
                main_event_tx: self.main_event_tx.clone(),
                env: self.env.clone(),
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
        let cfg = Win32ThreadConfig {
            env: self.env.clone(),
            inputs,
            ui_bus: Arc::clone(&self.ui_bus),
            flight_bus: Arc::clone(&self.flight_bus),
            fm: Arc::clone(&self.fm),
            shared: Arc::clone(&self.shared),
            activation: Arc::clone(&self.activation),
            ui_cmd_rx,
            hotkey_rx,
            main_event_tx: self.main_event_tx.clone(),
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
                // Java MainForm.java:92-98 mCancel 保存语义 (配置落盘)
                if let Some(c) = self.controller.as_ref() {
                    c.config.save_layout_config();
                }
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
            // Java configChangedHandler (Controller.java:498-544)
            MainEvent::ConfigChanged(key) => {
                let is_reset_completed = key == ui_state_events::ACTION_RESET_COMPLETED;
                let Some(c) = self.controller.as_mut() else { return };
                if key == "displayFmKey" || key == "enableFMPrint" {
                    c.handle_fm_hotkey_config_change();
                }
                // Java:508-512 导入/重置后热键绑定同步 (key=RESET_COMPLETED)
                if is_reset_completed {
                    c.handle_fm_hotkey_config_change();
                }
                // win32 激活面同步 (配置已由发布方写毕, 最后写胜出 — 见缓存头注)
                refresh_activation_cache(&c.config, &self.activation);
                if c.state() == ControllerState::Preview {
                    // Java:514-536 防抖 (200ms 安静期最后一次生效)。
                    // loadFromConfig 在调度点先行 (Java 在任务体首行; 配置 !Send
                    // 不能进防抖线程, 值等价 — 发布→调度间配置无二次变更面)
                    c.load_from_config_();
                    let _ = self.debounce.sender().send(DebounceMsg::ConfigKey(key));
                } else {
                    // Java:537-543 非 PREVIEW: 只更新本地配置 + reinit 活跃 overlay
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
                    // Java:562-566 右下角 toast (NotificationService 未移植 → 日志顶位)
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
                        // Java:568-577 复用 configDebouncer 全量刷新
                        let _ = self.debounce.sender().send(DebounceMsg::FmChanged);
                    }
                }
            }
            MainEvent::Tray(TrayCommand::Activate) => {
                // Java Application.java:251-273: CAS 防重入在托盘层 (tray.rs) 完成,
                // 此处串行收到 — 旧核 stop + 新核构造
                self.rebuild_controller(false);
            }
            MainEvent::Tray(TrayCommand::Start) => {
                // tray.rs 拆分入口: Controller 重建的服务启动部分
                if let Some(c) = self.controller.as_mut() {
                    c.start(&mut self.release_main_form);
                }
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

    /// 阻塞监督循环 (无 MainForm 场景: --game-mode / 冒烟; Java 托盘+EDT 泵的对位)。
    /// Exit 托盘命令或通道关闭即返回 (进程退出归调用方)。
    pub fn run_supervisor(mut self) {
        loop {
            match self.main_event_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(ev) => self.handle_main_event(ev),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            self.pump();
            if self.exit_requested {
                break;
            }
        }
        self.shutdown();
    }

    /// 全量收尾: 旧核五步 → win32 线程 (先托盘 NIM_DELETE 后窗口, tray.rs 退出
    /// 契约) → 防抖线程 → 热键钩子 (AppShell Drop 兜底)。
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
            hm.shutdown(); // Java shutdown() 不清全局钩子; Rust 实例自持钩子, 卸钩收线程
        }
    }
}

// =====================================================================
// win32 线程 (D8: host 泵 + 托盘 + 热键事件消费)
// =====================================================================

/// win32 线程装配输入 (全部 Send; 配置以快照形态入线程, 见模块头)
pub struct Win32ThreadConfig {
    pub env: Env,
    pub inputs: OverlayInputs,
    pub ui_bus: Arc<EventBus<UiStateEvent>>,
    pub flight_bus: Arc<FlightDataBus>,
    pub fm: Arc<FMManager>,
    pub shared: Arc<ControllerShared>,
    pub activation: ActivationCache,
    pub ui_cmd_rx: Receiver<UiCommand>,
    pub hotkey_rx: Receiver<HotkeyEvent>,
    pub main_event_tx: Sender<MainEvent>,
}

/// win32 线程内注册的 overlay 数据句柄 (Rc — 恒留本线程)
struct OverlayHandles {
    /// MiniHUD live 喂入口 (Java onFlightData → EDT 的单线程 host 对位)
    minihud: Option<MiniHudHandle>,
}

/// Java OverlayContext 的 win32 侧替身: 激活探测访问面
/// (get_bool/isDebug/isJet/isPreviewMode/has_blkx — activation_strategy.rs trait 注)
struct HostActivationCtx {
    activation: ActivationCache,
    fm: Arc<FMManager>,
    shared: Arc<ControllerShared>,
    debug: bool,
}

impl ActivationContext for HostActivationCtx {
    fn get_bool(&self, key: &str) -> bool {
        // Java: Boolean.parseBoolean(configProvider.getConfig(key)) — 缺失/非 true 均 false
        self.activation
            .lock()
            .expect("激活缓存锁中毒")
            .get(key)
            .map(|v| java_parse_boolean(v))
            .unwrap_or(false)
    }
    fn is_debug(&self) -> bool {
        self.debug // Application.debug (Env 快照)
    }
    fn is_jet(&self) -> bool {
        // Java OverlayContext.isJet: Blkx != null && Blkx.isJet
        self.fm
            .current()
            .blkx
            .as_ref()
            .map(|b| b.is_jet)
            .unwrap_or(false)
    }
    fn is_preview_mode(&self) -> bool {
        self.shared.overlay_ctx_preview.load(Ordering::SeqCst)
    }
    fn has_blkx(&self) -> bool {
        self.fm.current().blkx.is_some()
    }
}

/// 注册键 → 激活策略 (Java registerWithPreview 默认 config(key);
/// 两处复合策略来自 registerWithStrategy, Controller.java:717-752)
fn strategy_for(config_key: &str) -> ActivationStrategy {
    match config_key {
        "enableVoiceWarn" => ActivationStrategy::config(config_key)
            .and(&ActivationStrategy::game_mode_only()),
        "thrustdFS" => {
            ActivationStrategy::config("enableFMPrint").and(&ActivationStrategy::jet_only())
        }
        _ => ActivationStrategy::config(config_key),
    }
}

/// Java Controller.registerGameModeOverlays (651-753) 的 win32 侧一次性注册。
/// PORT(偏差备案): Java 每 Controller 重建 OverlayManager + 重注册; Rust host 跨
/// 重建存活 (D8), 条目是无状态配置记录 (id/config_key/尺寸/渲染闭包), 重建语义
/// 由激活探测 (实时配置) + 命令通道承载 — 重注册无信息增量。
/// 本批可注册 = 有 spec 工厂的四件; 其余:
/// - flightInfoSwitch: POC 走 window.rs 专径, 无 OverlaySpec 工厂 (TODO(port))
/// - enableAxis/enableFMPrint (field2): 需 host 扩展 (overlays_field2.rs 头注契约)
/// - enableVoiceWarn: 非窗口 (VoiceWarning 为 FlightDataBus 订阅者形态, TODO(port))
/// - thrustdFS (DrawFrameSimpl): D8 降级清单 P6
/// - enableAttitudeIndicator: gauge_attitude 无独立 spec 工厂 (TODO(port))
fn register_game_mode_overlays(
    host: &mut OverlayHost,
    handles: &mut OverlayHandles,
    env: &Env,
    inputs: &OverlayInputs,
) {
    let fonts = &env.fonts_dir;
    // 引擎控制 (Java:654-659, 键 enableEngineControl)
    match engine_control_preview_spec(
        fonts,
        &Lang::init_lang(),
        inputs.font_add_engine,
        inputs.dpi_scale,
    ) {
        Ok(spec) => {
            host.register(spec)
                .with_interest(&["disableEngineInfo", "fontSize"]);
        }
        Err(e) => logger::error("Controller", &format!("引擎控制 overlay 注册失败: {}", e)),
    }
    // 动力信息 (Java:662-667, 键 engineInfoSwitch)
    match power_info_preview_spec(fonts, inputs.font_add_power, inputs.power_columns) {
        Ok(spec) => {
            host.register(spec)
                .with_interest(&["fontName", "fontSize", "hudColumns", "S."]);
        }
        Err(e) => logger::error("Controller", &format!("动力信息 overlay 注册失败: {}", e)),
    }
    // MiniHUD (Java:671-679, 键 crosshairSwitch; HUDSettings 经快照)
    // PORT: service_present=false (注册时 Service 尚未建; 该标志影响 preview 行为集,
    // live 重接线批次随 spec 工厂参数化回收)
    match minihud_overlay_spec(
        false,
        inputs.service_loop_interval_ms,
        &inputs.hud,
        inputs.dpi_scale,
        &fonts.join("sarasa-mono-sc-bold.ttf"),
    ) {
        Ok((h, spec)) => {
            handles.minihud = Some(h);
            host.register(spec).with_interest(&[
                "displayCrosshair",
                "drawHUD",
                "disableHUD",
                "crosshair",
                "miniHUD",
                "enableLayoutDebug",
                "enableFlapAngleBar",
                "hudMach",
                "showSpeedBar",
                "showAttitudeIndicator",
                "attitudeIndicatorInertialMode",
                "alwaysShowRadarAltitude",
                "showHUD",
            ]);
        }
        Err(e) => logger::error("Controller", &format!("MiniHUD overlay 注册失败: {}", e)),
    }
    // 起落襟翼 (Java:709-714, 键 enablegearAndFlaps)
    match gear_flaps_preview_spec(
        fonts,
        inputs.font_add_gear,
        inputs.dpi_scale,
        inputs.gear_show_edge,
    ) {
        Ok(spec) => {
            host.register(spec)
                .with_interest(&["enablegearAndFlapsEdge", "fontSize"]);
        }
        Err(e) => logger::error("Controller", &format!("起落襟翼 overlay 注册失败: {}", e)),
    }
}

/// 托盘 handler: 动作转发主线程 (Java 托盘回调在 EDT, Rust 泵线程→channel)
#[cfg(target_os = "windows")]
struct AppTrayHandler {
    tx: Sender<MainEvent>,
}

#[cfg(target_os = "windows")]
impl TrayHandler for AppTrayHandler {
    fn activate(&mut self) {
        let _ = self.tx.send(MainEvent::Tray(TrayCommand::Activate));
    }
    fn start(&mut self) {
        let _ = self.tx.send(MainEvent::Tray(TrayCommand::Start));
    }
    fn exit(&mut self) {
        // 退出序契约 (tray.rs): 进程退出前先 drop TrayIcon — 本命令驱动主线程
        // shutdown() → UiCommand::Shutdown → 本线程循环退出时 drop 托盘 (次序保证)
        let _ = self.tx.send(MainEvent::Tray(TrayCommand::Exit));
    }
}

/// 通道排空取最新 (live 数据合并: 对位 Java EDT repaint 合并 — 只留最新帧)
fn drain_latest<T>(rx: &Receiver<T>) -> Option<T> {
    let mut latest = rx.try_recv().ok()?;
    while let Ok(next) = rx.try_recv() {
        latest = next;
    }
    Some(latest)
}

/// MiniHUD live 喂入 (Java onFlightData → invokeLater → EDT 的单线程对位)。
/// PORT: FlightDataEvent 不可 Clone (OpaqueObject) — 转发链只送 EventPayload,
/// 本侧重构事件对象; hud_data 恒 None → 走 MiniHudOverlay 的 EDT 回退计算路径
/// (minihud.rs update_from_event 的 None 分支; service_loop 的 set_hud_data
/// TODO(port) 期间即设计消费路径)。
fn feed_minihud_live(
    handle: &MiniHudHandle,
    payload: &EventPayload,
    shared: &ControllerShared,
    fm: &FMManager,
    settings: &HudSettingsSnapshot,
) {
    let live = shared.live.read().expect("live 锁中毒").clone();
    let Some(data) = live else { return };
    let Ok(guard) = data.read() else {
        return; // 中毒帧跳过 (Java 无锁无此形态; §6 契约下 Service 线程仍在轮询)
    };
    let event = FlightDataEvent::new(payload.clone(), None, None);
    let colors = HudColors::application_defaults();
    let fm_handle = fm.current();
    let blkx = fm_handle.blkx.as_ref();
    handle.borrow_mut().on_flight_data(
        current_time_millis(),
        &event,
        Some(&*guard),
        blkx,
        settings,
        &colors,
    );
}

/// win32 线程入口 (D8 拓扑): OverlayHost 泵 + 托盘 + 热键事件消费。
///
/// PORT(热键拓扑豁免记录, hotkey.rs 头注 D8 偏差): WH_KEYBOARD_LL 钩子固化在
/// HotkeyManager 自管的独立钩子线程 (jnativehook 独立派发线程的保真形态);
/// D8 的"并入单泵"需 hotkey.rs 提供外部线程装钩入口 (未提供, 本批次不越文件改),
/// 豁免期内钩子事件经 channel 汇入本线程统一消费 — 与托盘/overlay 共享的
/// 泵约束 (安装线程需泵) 由钩子线程自泵满足, 行为面一致。
pub fn win32_thread_main(cfg: Win32ThreadConfig) {
    let Win32ThreadConfig {
        env,
        inputs,
        ui_bus,
        flight_bus,
        fm,
        shared,
        activation,
        ui_cmd_rx,
        hotkey_rx,
        main_event_tx,
    } = cfg;

    // ---- host 构建 + 激活探测 (Java new OverlayManager + ActivationStrategy) ----
    let mut host = OverlayHost::new();
    let ctx = HostActivationCtx {
        activation: Arc::clone(&activation),
        fm: Arc::clone(&fm),
        shared: Arc::clone(&shared),
        debug: env.debug,
    };
    host.with_activation(Box::new(move |key: &str| {
        strategy_for(key).should_activate(&ctx)
    }));
    let mut handles = OverlayHandles { minihud: None };
    register_game_mode_overlays(&mut host, &mut handles, &env, &inputs);
    // live 喂入用设置快照 (注册面同源; WYSIWYG 字号变更的重接线随 host 工厂面扩展)
    let hud_settings = inputs.hud;

    // ---- 托盘 (Java initSystemTray: 失败继续运行, Application.java:217-280) ----
    #[cfg(target_os = "windows")]
    let mut tray = {
        let handler = AppTrayHandler {
            tx: main_event_tx,
        };
        let tray_cfg = TrayConfig {
            icon_path: env.icon_path.clone(),
            ..Default::default()
        };
        match TrayIcon::new(Box::new(handler), tray_cfg) {
            Ok(t) => Some(t),
            // Java: AWTException → logAndContinue("系统托盘") — 无托盘继续运行
            Err(e) => {
                logger::warn("系统托盘", &format!("托盘创建失败, 程序继续运行: {}", e));
                None
            }
        }
    };
    #[cfg(not(target_os = "windows"))]
    let _ = main_event_tx; // 非 Windows 无托盘 (x11 波次)

    // ---- live 数据转发订阅 (Java: overlay init 时 FlightDataBus.register; Rust 由
    // 命令驱动建/撤 — OpenAll 建立, CloseAll 撤销, 对位 overlay 订阅生命周期) ----
    let mut flight_sub: Option<Subscription<FlightDataEvent>> = None;
    let (flight_tx, flight_rx) = std::sync::mpsc::channel::<EventPayload>();

    let mut last_render = Instant::now();
    loop {
        // 托盘消息泵 (创建线程亲和, tray.rs 头注)
        #[cfg(target_os = "windows")]
        {
            if let Some(t) = tray.as_mut() {
                t.pump();
            }
        }
        host.pump_events();
        // 渲染节拍 50ms (Java FieldOverlay.onFlightData 50ms 节流, host.run 同款)
        if last_render.elapsed() >= Duration::from_millis(50) {
            last_render = Instant::now();
            if let Err(e) = host.render_tick() {
                logger::error("OverlayHost", &format!("render_tick: {}", e));
            }
            // live 数据喂入 (只留最新帧)
            if let Some(payload) = drain_latest(&flight_rx) {
                if let Some(h) = handles.minihud.as_ref() {
                    feed_minihud_live(h, &payload, &shared, &fm, &hud_settings);
                }
            }
        }
        // UI 命令 (生命周期/WYSIWYG 的 win32 属主面)
        while let Ok(cmd) = ui_cmd_rx.try_recv() {
            match cmd {
                UiCommand::OpenAllOverlays => {
                    // Java openpad → openAll; live 订阅随建 (overlay 订阅生命周期)
                    shared.overlay_ctx_preview.store(false, Ordering::SeqCst); // forGameMode
                    if let Err(e) = host.open_all() {
                        logger::error("OverlayHost", &format!("open_all: {}", e));
                    }
                    let tx = flight_tx.clone();
                    let sub = flight_bus.register(move |ev: &FlightDataEvent| {
                        // 转发线程 = Service 发布线程; 本闭包只 send 不碰 UI
                        // (flight_data_bus.rs 重入死锁警戒的 channel 转发要求)
                        let _ = tx.send(ev.get_payload().clone());
                    });
                    // 旧订阅 (如有) 显式 drop = unregister; 槽位持新订阅保活
                    drop(std::mem::replace(&mut flight_sub, Some(sub)));
                }
                UiCommand::CloseAllOverlays => {
                    host.close_all(); // close 销毁链 (存位置 → drop)
                    // Java overlay dispose → Bus.unregister (drop 槽位即退订)
                    drop(std::mem::take(&mut flight_sub));
                }
                UiCommand::RefreshPreviews {
                    changed_key,
                    generation,
                } => {
                    if is_stale_refresh(&shared, generation) {
                        continue; // 防过期守卫 (Java invokeLater 内守卫的根治位)
                    }
                    shared.overlay_ctx_preview.store(true, Ordering::SeqCst); // forPreviewMode
                    let r = match changed_key.as_deref() {
                        Some(k) => host.refresh_preview_key(Some(k)), // Java refreshPreviews(key)
                        None => host.refresh_preview(),               // Java refreshAllPreviews
                    };
                    if let Err(e) = r {
                        logger::error("OverlayHost", &format!("refresh_previews: {}", e));
                    }
                }
                UiCommand::ReinitActiveOverlays => host.reinit_active_overlays(),
                UiCommand::Shutdown => {
                    logger::info("AppShell", "win32 线程退出 (Shutdown)");
                    // Drop 序: tray 最先 (NIM_DELETE 防僵尸 — tray.rs 退出契约),
                    // 继而 flight_sub / host (窗口销毁)。局部声明序 host→tray,
                    // 逆序 drop 恰好 tray 在 host 前 ✓
                    return;
                }
                // 主线程属主命令不经本通道 (UiCommand 文档); 防御性忽略
                UiCommand::StartGame | UiCommand::EndGame => {}
            }
        }
        // 热键事件 (钩子线程 → 本线程统一消费; Java jnativehook 派发线程直发 UIStateBus)
        while let Ok(hk) = hotkey_rx.try_recv() {
            // Java: UIStateBus.publish(eventType, HotkeyManager.this, code)
            ui_bus.publish(&UiStateEvent {
                event_type: hk.event_type.clone(),
                source: "HotkeyManager".to_string(),
                data: hk.key_code.to_string(),
            });
        }
        std::thread::sleep(Duration::from_millis(10)); // 事件泵 10ms (host.run 同款)
    }
}

// =====================================================================
// Tests — 状态机转移 / stop 五步序 / 防过期 generation / debounce 时序
// (wf-p5-batch14 W1 验收单; 假时钟以短 debounce 间隔替代)
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use vm_core::fm::FMHandle;

    static CFG_N: AtomicUsize = AtomicUsize::new(0);

    /// tmp 配置文件 (vm-core configuration_service 测试同款惯例)
    fn tmp_cfg(content: &str) -> String {
        let n = CFG_N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir()
            .join(format!("vm_app_shell_{}_{n}.cfg", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        std::fs::write(&p, content).unwrap();
        p
    }

    /// 测试配置: crosshairSwitch 开 / enableEngineControl 关 / 无自启动
    fn test_cfg() -> String {
        tmp_cfg(
            "(panel \"T\" :visible true\n\
             \x20 (item \"hud\" :type switch :target \"crosshairSwitch\" :value true)\n\
             \x20 (item \"engine\" :type switch :target \"enableEngineControl\" :value false)\n\
             \x20 (item \"auto\" :type switch :target \"autoStartGameMode\" :value false))\n\
            ",
        )
    }

    /// AppShell 测试装配: tmp cfg (无 init_config 写盘副作用) + 30ms 短防抖;
    /// 不起 win32 线程 — ui_cmd 接收端留在 shell 内供测试观察。
    fn fixture() -> AppShell {
        fixture_with_debounce(30)
    }

    fn fixture_with_debounce(ms: u64) -> AppShell {
        let ui_bus = Arc::new(EventBus::new());
        let config = ConfigurationService::new(Some(Arc::clone(&ui_bus)));
        config.load_layout(&test_cfg());
        let (hotkey, hotkey_rx) = HotkeyManager::with_channel();
        let mut shell = AppShell::with_parts(ShellParts {
            env: Env::probe(&Lang::init_lang(), false),
            config,
            ui_bus,
            flight_bus: Arc::new(FlightDataBus::new()),
            fm: Arc::new(FMManager::new(Arc::new(EventBus::new()))),
            hotkey,
            hotkey_rx,
            debounce_delay: Duration::from_millis(ms),
        });
        shell.rebuild_controller(true);
        shell
    }

    /// 泵监督事件 (订阅转发 → 主线程处理链), 返回是否处理过事件
    fn pump_events(shell: &mut AppShell) -> bool {
        let mut handled = false;
        while let Ok(ev) = shell.main_event_rx.try_recv() {
            shell.handle_main_event(ev);
            handled = true;
        }
        handled
    }

    fn publish_ui_event(bus: &EventBus<UiStateEvent>, event_type: &str, data: &str) {
        bus.publish(&UiStateEvent {
            event_type: event_type.to_string(),
            source: "MainForm".to_string(),
            data: data.to_string(),
        });
    }

    // ------------------------------------------------------------------
    // 状态机: preview → game → (drive) → stop
    // ------------------------------------------------------------------

    /// UI_READY → Preview() → state=Preview (Java uiReadyHandler → Preview)
    #[test]
    fn ui_ready_enters_preview_state() {
        let mut shell = fixture();
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Init
        );
        publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
        assert!(pump_events(&mut shell), "UI_READY 应经转发到达监督循环");
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Preview
        );
    }

    /// confirm 链: Preview → (endPreview) Init → start (INIT 守卫过) → Service 起
    #[test]
    fn confirm_start_game_runs_service() {
        let mut shell = fixture();
        publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
        pump_events(&mut shell);
        shell.dispatch(UiCommand::StartGame);
        let c = shell.controller.as_ref().unwrap();
        // end_preview 置 INIT; start 不改 State (Java 同 — 等 Service 轮询驱动)
        assert_eq!(c.state(), ControllerState::Init);
        assert!(c.service.is_some(), "start() 应建 Service 线程");
        assert!(
            shell.shared.live.read().unwrap().is_some(),
            "live 句柄应登记"
        );
    }

    /// Service 轮询驱动: flags 真 + playerLive → changeS2/changeS3 (InGame→Preview)
    /// + FM identify 提交 (换机轻量 swap 的会话记忆同时落位)
    #[test]
    fn drive_from_live_opens_overlays() {
        let mut shell = fixture();
        {
            let c = shell.controller.as_mut().unwrap();
            c.init_status_bar();
            c.change_s2();
            assert_eq!(c.state(), ControllerState::InGame);
        }
        // 手工装填 live 数据 (真机由 Service 线程写; ServiceData 公开字段面)
        let data = Arc::new(std::sync::RwLock::new(live_service_data("test-plane")));
        *shell.shared.live.write().unwrap() = Some(data);
        shell.controller.as_mut().unwrap().drive_from_live();
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Preview,
            "changeS3 应进 PREVIEW"
        );
        // FM 识别目标已提交 (identify 异步加载, 目标名同步可见)
        assert_eq!(
            shell.fm.current_target_name().as_deref(),
            Some("test-plane")
        );
        // 会话首机只记名 (sessionAircraftType)
        assert_eq!(
            shell.shared.flags.lock().unwrap().session_aircraft_type.as_deref(),
            Some("test-plane")
        );
    }

    /// flags 丢失 → S4toS1: Preview → Init + FM 目标清除 (会话结束语义)
    #[test]
    fn drive_from_live_exit_resets_session() {
        let mut shell = fixture();
        {
            let c = shell.controller.as_mut().unwrap();
            c.init_status_bar();
            c.change_s2();
        }
        let data = Arc::new(std::sync::RwLock::new(live_service_data("p1")));
        *shell.shared.live.write().unwrap() = Some(Arc::clone(&data));
        shell.controller.as_mut().unwrap().drive_from_live(); // 进 Preview + identify(p1)
        // 游戏退出: flags 全假 (Java S4toS1 两条路径合并)
        *data.write().unwrap() = ServiceData::default();
        shell.controller.as_mut().unwrap().drive_from_live();
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Init
        );
        assert_eq!(
            shell.fm.current_target_name(),
            None,
            "会话结束清识别目标"
        );
        assert!(
            shell.shared.flags.lock().unwrap().session_aircraft_type.is_none(),
            "会话机型记忆清除"
        );
    }

    /// 换机轻量 swap: 同机幂等; 换机 FM 目标切换且不重启 Controller
    #[test]
    fn aircraft_change_lightweight_swap() {
        let mut shell = fixture();
        {
            let c = shell.controller.as_mut().unwrap();
            c.init_status_bar();
            c.change_s2();
        }
        let data = Arc::new(std::sync::RwLock::new(live_service_data("a1")));
        *shell.shared.live.write().unwrap() = Some(Arc::clone(&data));
        let c = shell.controller.as_mut().unwrap();
        c.drive_from_live();
        assert_eq!(shell.fm.current_target_name().as_deref(), Some("a1"));
        // 同机: 幂等 (目标不变)
        c.on_aircraft_changed(Some("a1"));
        assert_eq!(shell.fm.current_target_name().as_deref(), Some("a1"));
        // 换机: FM 目标切换 (identify), 不重启 Controller (state 保持 Preview)
        *data.write().unwrap() = live_service_data("b2");
        c.drive_from_live();
        assert_eq!(shell.fm.current_target_name().as_deref(), Some("b2"));
        assert_eq!(c.state(), ControllerState::Preview, "换机不重启生命周期");
    }

    // ------------------------------------------------------------------
    // stop 五步序 (LIFETIMES §4.2)
    // ------------------------------------------------------------------

    /// 五步销毁: ①gen++/CloseAll → ②退订 → ③释放设置窗 → ④停 Service → ⑤存配置;
    /// 顺序断言: 步③执行时步①的 CloseAllOverlays 命令已入队 (①先于③);
    /// 步④后 service 句柄已收 + live 清空
    #[test]
    fn stop_five_step_order() {
        let mut shell = fixture();
        // 进 Preview + 起 Service (五步全路径)
        publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
        pump_events(&mut shell);
        shell.dispatch(UiCommand::StartGame);
        let data = Arc::new(std::sync::RwLock::new(live_service_data("s1")));
        *shell.shared.live.write().unwrap() = Some(data);
        shell.pump(); // drive: Service live 数据 → InGame → Preview
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Preview
        );

        let gen_before = shell.shared.preview_generation.load(Ordering::SeqCst);
        let ui_subs_before = shell.ui_bus.subscriber_count();
        let fm_subs_before = shell.fm.fm_changed_bus().subscriber_count();
        assert!(ui_subs_before >= 2, "Controller 应持 config/uiReady 两订阅");
        assert!(fm_subs_before >= 1, "Controller 应持 FM_CHANGED 订阅");

        // 步③注入: 执行时断言步①的 CloseAllOverlays 已可观察 (顺序 ①→③)
        let ui_rx = shell.ui_cmd_rx.take().unwrap();
        let step3_seen = Arc::new(Mutex::new(false));
        let seen = Arc::clone(&step3_seen);
        let mut release = move || {
            // 步①的 CloseAll (closepad 路径, service 存在) 应已先入队
            let cmd = ui_rx.recv_timeout(Duration::from_millis(500)).expect("步①命令未达");
            assert_eq!(cmd, UiCommand::CloseAllOverlays, "步①应为 CloseAllOverlays");
            *seen.lock().unwrap() = true;
        };
        shell.controller.as_mut().unwrap().stop(&mut release);

        // ①: 世代号 ++ (作废在途回调) — Preview 分支
        assert_eq!(
            shell.shared.preview_generation.load(Ordering::SeqCst),
            gen_before + 1
        );
        // ②: 订阅全部退订
        assert_eq!(shell.ui_bus.subscriber_count(), ui_subs_before - 2);
        assert_eq!(
            shell.fm.fm_changed_bus().subscriber_count(),
            fm_subs_before - 1
        );
        // ③: 释放设置窗已执行 (闭包内断言通过)
        assert!(*step3_seen.lock().unwrap());
        // ④: Service 句柄已收 + live 清空
        assert!(shell.controller.as_ref().unwrap().service.is_none());
        assert!(shell.shared.live.read().unwrap().is_none());
        // ⑤: save_config 空实现 (全量在 ui_layout.cfg), 无可断言面 — 顺序由代码序保证
    }

    // ------------------------------------------------------------------
    // 防过期 generation (previewGeneration)
    // ------------------------------------------------------------------

    /// is_stale_refresh 守卫三态: 状态离开 Preview / 世代号漂移 / 均正常
    #[test]
    fn stale_generation_guard() {
        let shared = ControllerShared::new();
        *shared.state.write().unwrap() = ControllerState::Preview;
        shared.preview_generation.store(5, Ordering::SeqCst);
        assert!(!is_stale_refresh(&shared, 5), "Preview + 同代 → 放行");
        assert!(is_stale_refresh(&shared, 4), "世代号不匹配 → 丢弃");
        *shared.state.write().unwrap() = ControllerState::Init;
        assert!(is_stale_refresh(&shared, 5), "已离开 Preview → 丢弃");
    }

    /// 端到端: Preview() 捕获的世代号被 end_preview() 作废后, 消费侧丢弃
    #[test]
    fn end_preview_invalidates_inflight_generation() {
        let shared = ControllerShared::new();
        *shared.state.write().unwrap() = ControllerState::Preview;
        let in_flight = shared.preview_generation.load(Ordering::SeqCst);
        shared.preview_generation.fetch_add(1, Ordering::SeqCst); // stop()/endPreview()
        *shared.state.write().unwrap() = ControllerState::Init;
        assert!(is_stale_refresh(&shared, in_flight));
    }

    // ------------------------------------------------------------------
    // debounce 时序 (Java ConfigDebounce 语义, 短间隔)
    // ------------------------------------------------------------------

    /// 连发变更合并为一次刷新, 载荷 = 最后一条键; 安静后无重复
    #[test]
    fn debounce_coalesces_rapid_changes() {
        let shared = Arc::new(ControllerShared::new());
        let (out_tx, out_rx) = std::sync::mpsc::channel::<UiCommand>();
        let mut deb =
            ConfigDebouncer::spawn(Duration::from_millis(40), out_tx, Arc::clone(&shared));
        let tx = deb.sender();
        for k in ["k1", "k2", "k3", "k4", "k5"] {
            tx.send(DebounceMsg::ConfigKey(k.to_string())).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        match out_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(UiCommand::RefreshPreviews {
                changed_key,
                generation,
            }) => {
                assert_eq!(changed_key, Some("k5".to_string()), "最后一条变更生效");
                assert_eq!(
                    generation,
                    shared.preview_generation.load(Ordering::SeqCst),
                    "世代号为发送时快照"
                );
            }
            other => panic!("防抖后应送达一次刷新: {:?}", other),
        }
        // 安静期无第二条 (合并为一次)
        assert!(
            out_rx.recv_timeout(Duration::from_millis(120)).is_err(),
            "防抖窗口内连发只触发一次"
        );
        deb.shutdown();
    }

    /// FmChanged → 全量刷新 (changed_key=None); RESET_COMPLETED → 全量
    #[test]
    fn debounce_fm_and_reset_full_refresh() {
        let shared = Arc::new(ControllerShared::new());
        let (out_tx, out_rx) = std::sync::mpsc::channel::<UiCommand>();
        let mut deb = ConfigDebouncer::spawn(Duration::from_millis(30), out_tx, shared);
        deb.sender().send(DebounceMsg::FmChanged).unwrap();
        match out_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(UiCommand::RefreshPreviews { changed_key: None, .. }) => {}
            other => panic!("FmChanged 应产全量刷新: {:?}", other),
        }
        deb.sender()
            .send(DebounceMsg::ConfigKey(
                ui_state_events::ACTION_RESET_COMPLETED.to_string(),
            ))
            .unwrap();
        match out_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(UiCommand::RefreshPreviews { changed_key: None, .. }) => {}
            other => panic!("RESET_COMPLETED 应产全量刷新: {:?}", other),
        }
        deb.shutdown();
    }

    // ------------------------------------------------------------------
    // WYSIWYG 链 (CONFIG_CHANGED → 防抖 → RefreshPreviews)
    // ------------------------------------------------------------------

    /// Preview 态: CONFIG_CHANGED → 转发 → 防抖 (30ms) → RefreshPreviews(键) 命令
    #[test]
    fn wysiwyg_config_change_refreshes_via_debounce() {
        let mut shell = fixture_with_debounce(30);
        publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
        pump_events(&mut shell);
        // MainForm 侧配置写入 (服务内联发布 CONFIG_CHANGED — vm-ui 链同源)
        publish_ui_event(&shell.ui_bus, ui_state_events::CONFIG_CHANGED, "showSpeedBar");
        assert!(pump_events(&mut shell), "CONFIG_CHANGED 应经转发到达监督循环");
        // 防抖产出直达 win32 命令通道 — 接收端留在 shell (未 spawn win32)
        let cmd = shell
            .ui_cmd_rx
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_millis(500))
            .expect("防抖后应有 RefreshPreviews 命令");
        match cmd {
            UiCommand::RefreshPreviews {
                changed_key,
                generation,
            } => {
                assert_eq!(changed_key, Some("showSpeedBar".to_string()));
                assert_eq!(
                    generation,
                    shell.shared.preview_generation.load(Ordering::SeqCst)
                );
            }
            other => panic!("应为 RefreshPreviews: {:?}", other),
        }
    }

    /// 非 Preview 态: CONFIG_CHANGED → 立即 ReinitActiveOverlays (无防抖)
    #[test]
    fn config_change_non_preview_reinits_active() {
        let mut shell = fixture_with_debounce(30);
        publish_ui_event(&shell.ui_bus, ui_state_events::CONFIG_CHANGED, "fontNum");
        assert!(pump_events(&mut shell));
        let cmd = shell
            .ui_cmd_rx
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_millis(200))
            .expect("非 Preview 态应立即 ReinitActiveOverlays");
        assert_eq!(cmd, UiCommand::ReinitActiveOverlays);
        // 防抖不排队 (无后续刷新命令)
        assert!(shell
            .ui_cmd_rx
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_millis(100))
            .is_err());
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Init
        );
    }

    /// FM_CHANGED (missing-like) → 摘要转发 (toast 面) + Preview 态防抖全量刷新
    #[test]
    fn fm_changed_missing_schedules_full_refresh() {
        let mut shell = fixture_with_debounce(30);
        publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
        pump_events(&mut shell);
        // 直接向 FM 通道发布 missing-like 句柄 (Java FM-Loader 线程发布形态)
        let handle = FMHandle {
            name: Some("nosuch-plane".to_string()),
            status: FMStatus::Missing,
            ..FMHandle::UNRESOLVED
        };
        shell.fm.fm_changed_bus().publish(&handle);
        // 两条转发 (missing 摘要 + 刷新调度) 到达监督通道
        let mut saw_missing = false;
        while let Ok(ev) = shell.main_event_rx.recv_timeout(Duration::from_millis(200)) {
            if let MainEvent::FmChanged { name: Some(_), .. } = &ev {
                saw_missing = true;
            }
            shell.handle_main_event(ev);
        }
        assert!(saw_missing, "missing-like 摘要应转发");
        // Preview 态 → 防抖全量刷新命令
        let cmd = shell
            .ui_cmd_rx
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_millis(500))
            .expect("FM_CHANGED 应触发防抖全量刷新");
        match cmd {
            UiCommand::RefreshPreviews { changed_key: None, .. } => {}
            other => panic!("应为全量刷新: {:?}", other),
        }
    }

    // ------------------------------------------------------------------
    // 托盘重建 (Application.ctr 替换)
    // ------------------------------------------------------------------

    /// 托盘 Activate: 旧核 stop (退订) → 新核 (State=INIT, 订阅重建, 无泄漏累积)
    #[test]
    fn tray_activate_rebuilds_controller() {
        let mut shell = fixture();
        let before = shell.ui_bus.subscriber_count();
        shell.handle_main_event(MainEvent::Tray(TrayCommand::Activate));
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Init,
            "新核从 INIT 开始"
        );
        assert_eq!(
            shell.ui_bus.subscriber_count(),
            before,
            "旧核退订 + 新核订阅, 净计数不变 (无泄漏累积)"
        );
    }

    /// 激活缓存随配置装载 (win32 激活面的 WYSIWYG 输入)
    #[test]
    fn activation_cache_tracks_config() {
        let shell = fixture();
        let v = shell
            .activation
            .lock()
            .unwrap()
            .get("crosshairSwitch")
            .cloned();
        assert_eq!(
            v.as_deref(),
            Some("true"),
            "tmp cfg 的 crosshairSwitch=true 应入缓存"
        );
        let v2 = shell
            .activation
            .lock()
            .unwrap()
            .get("enableEngineControl")
            .cloned();
        assert_eq!(v2.as_deref(), Some("false"));
    }

    // ------------------------------------------------------------------
    // win32 线程生命周期 (真实窗口冒烟)
    // ------------------------------------------------------------------

    /// win32 线程: 注册不 panic → 刷新命令消费 → Shutdown → join 干净退出
    /// (JoinHandle 无泄漏; 真实 Win32 窗口/托盘创建, 无桌面环境时托盘缺失仍可跑)
    #[test]
    fn win32_thread_shutdown_joins_cleanly() {
        let mut shell = fixture();
        shell.spawn_win32_thread().expect("win32 线程启动");
        // Preview 态 + 有效世代号 → 全量刷新命令 (守卫放行路径, 不 stale)
        *shell.shared.state.write().unwrap() = ControllerState::Preview;
        let gen = shell.shared.preview_generation.load(Ordering::SeqCst);
        shell
            .ui_cmd_tx
            .send(UiCommand::RefreshPreviews {
                changed_key: None,
                generation: gen,
            })
            .unwrap();
        std::thread::sleep(Duration::from_millis(150)); // 泵消费 + 渲染一拍
        // 过期命令: 世代号 +1 → 消费侧丢弃 (守卫路径)
        shell
            .ui_cmd_tx
            .send(UiCommand::RefreshPreviews {
                changed_key: None,
                generation: gen + 1,
            })
            .unwrap();
        std::thread::sleep(Duration::from_millis(60));
        shell.ui_cmd_tx.send(UiCommand::Shutdown).unwrap();
        let join = shell.win32.take().unwrap();
        assert!(join.join().is_ok(), "win32 线程应干净退出");
        assert!(shell.win32.is_none());
    }

    // ------------------------------------------------------------------
    // 辅助
    // ------------------------------------------------------------------

    /// 构造 drive_from_live 判定所需的 live ServiceData 快照
    /// (真机由 Service.update 写; 测试直填公开字段 — flags/type/playerLive)
    fn live_service_data(plane: &str) -> ServiceData {
        let mut st = vm_core::parser::State::new();
        st.flag = true;
        let mut ind = vm_core::parser::Indicators::new();
        ind.flag = true;
        ind.r#type = Some(plane.to_string());
        let mut d = ServiceData::default();
        // s_state/s_indic 构造期由 Service::new 建立 (service_fields Default 注);
        // 测试直填 Some
        d.s_state = Some(st);
        d.s_indic = Some(ind);
        d.player_live = true;
        d
    }
}
