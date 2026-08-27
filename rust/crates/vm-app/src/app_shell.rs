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

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use vm_core::activation_strategy::{ActivationContext, ActivationStrategy};
use vm_core::bus::{EventBus, Subscription};
use vm_core::config_api::{ConfigProvider, HudSettingsSnapshot, OverlaySettings};
use vm_core::configuration_service::{ConfigurationService, ControllerIntervals, GlobalColors, UiStateEvent};
use vm_core::controller_state::ControllerState;
use vm_core::event::event_payload::EventPayload;
use vm_core::event::flight_data_event::{FlightDataEvent, OpaqueObject};
use vm_core::event::ui_state_events;
use vm_core::flight_data_bus::FlightDataBus;
use vm_core::flight_log::{
    ControllerLogSink, FlightLog, FlightLogSlot, NotifySink,
};
use vm_core::fm::{FMManager, FMStatus};
use vm_core::http_helper::HttpHelper;
use vm_core::hud_calculator::HudColors;
use vm_core::lang::Lang;
use vm_core::logger;
use vm_core::ui_model::TelemetrySource as _;

use vm_data::service_fields::ServiceData;
use vm_data::service_loop::{
    flight_log_snapshot, snapshot_indicators, snapshot_state, start as spawn_service_thread,
    Service, ServiceAnalyzerSource, ServiceConfig, ServiceHandle,
};

use vm_overlay::host::OverlayHost;
use vm_overlay::hotkey::{HotkeyEvent, HotkeyManager, VC_P};
use vm_overlay::platform_extras::DpiHelper;
use vm_overlay::{
    attitude_overlay_spec, control_surfaces_overlay_spec, engine_control_overlay_spec,
    flight_info_overlay_spec, gear_flaps_overlay_spec, minihud_overlay_spec,
    power_info_overlay_spec, AttitudeOverlayHandle, ControlSurfacesHandle,
    EngineControlHandle, FlightInfoHandle, GearFlapsHandle, MiniHudHandle, PowerInfoHandle,
};

#[cfg(target_os = "windows")]
use vm_overlay::tray::{TrayConfig, TrayIcon, TrayHandler};

/// Java Controller.java:59 `CONFIG_DEBOUNCE_MS = 200`
pub const CONFIG_DEBOUNCE_MS: u64 = 200;

// =====================================================================
// FlightLog 接线辅助 (Controller.java:366-376/402-411 的依赖注入面)
// =====================================================================

/// Java `Controller.logon` 布尔 (Controller.java:44) 的写入面对位: 唯一写点 =
/// FlightLog.init 失败路径的 `xc.logon = false` (FlightLog.java:409), 语义为
/// "停 tick" — Rust 以清槽表达 (槽 None ⇒ Service 轮询 logTick 短路)。
/// true 分支无 Java 写点, 空实现。
struct LogonSink(FlightLogSlot);
impl ControllerLogSink for LogonSink {
    fn set_logon(&self, logon: bool) {
        if !logon {
            *self.0.lock().expect("flight_log 槽锁中毒") = None;
        }
    }
}

/// ConfigurationService (!Send, 主线程独占) 的单键快照适配: FlightLog 侧 config
/// 的唯一消费是 FlightAnalyzer.init 的一次性 `getConfig("enableAltInformation")`
/// (flight_analyzer.rs:154-160), 与 Java 同一时刻同值 (见 open_flight_log 注)。
struct FlightLogConfig(Option<String>);
impl ConfigProvider for FlightLogConfig {
    fn get_config(&self, key: &str) -> Option<String> {
        if key == "enableAltInformation" {
            self.0.clone()
        } else {
            // FlightLog 链无其它读键 (Java 侧 FlightLog/FlightAnalyzer 只读此键)
            None
        }
    }
    fn set_config(&self, _key: &str, _value: &str) {}
    fn is_field_disabled(&self, _key: &str) -> bool {
        false
    }
}

/// FlightDataBus 事件流静默判定阈值 (审查 B1 补偿, 见
/// [`ControllerShared::last_flight_event_ms`] 注): player_live 轮每 ~50ms 发布
/// 一帧, 2s = 40 轮静默 — 比 Java 的串空即时判定更宽容 (网络抖动/加载切换不误判)。
pub const FLIGHT_SILENT_EXIT_MS: i64 = 2000;

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

/// 仓库模板 ui_layout.cfg 探测。
/// 生产 CWD=仓库根 (java -jar / rust_run.sh); 测试 CWD=crate 根 (cargo 惯例),
/// 上溯三级 (vm-app → crates → rust → 仓库根) — vm-core/vm-overlay 测试同款路径
fn locate_template_cfg() -> Option<String> {
    let mut candidates: Vec<PathBuf> =
        [PathBuf::from("ui_layout.cfg"), PathBuf::from("../ui_layout.cfg")].to_vec();
    candidates.push(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../ui_layout.cfg"),
    );
    candidates.into_iter().find(|p| p.exists()).map(|p| {
        p.to_string_lossy().into_owned()
    })
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
    /// MainForm 底部"结束游戏"按钮 (MainForm.java:92-98 保存 + System.exit(0)) —
    /// **主线程属主** (退出经 exit_requested, 见 dispatch 处理注)
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
    /// WYSIWYG reinit 参数直送 (PORT 新增命令, 五色直送同款模式): 主线程
    /// CONFIG_CHANGED 时即时读配置重建 [`vm_overlay::ReinitParams`] (纯值 Send),
    /// 先于 RefreshPreviews/ReinitActiveOverlays 入队 — win32 线程存入线程局部
    /// 参数仓供各 spec 工厂 reinit 闭包读取 (配置 !Send, 值随命令进线程) — win32 属主
    /// Box: 参数包 ~272B 远大于其余变体, 装箱拉平枚举尺寸 (clippy large_enum_variant)
    ReinitOverlays { params: Box<vm_overlay::ReinitParams> },
    /// 游戏失焦隐藏全部 overlay (Java FocusMonitor → hideAllOverlays;
    /// 不销毁实例) — win32 属主
    HideAllOverlays,
    /// 游戏复焦恢复 (Java showAllOverlays) — win32 属主
    ShowAllOverlays,
    /// AA 开关更新 (cfg AAEnable — Java 同开同关 graph/text 两 hint, Rust 仓单值;
    /// 直读 cfg 即时值, 配置 !Send) — win32 属主
    SetAa(bool),
    /// 全局五色更新 (Java: 改色 → CONFIG_CHANGED(font 前缀全局键) → 刷新;
    /// Rust 配置 !Send, 色值随命令直送 win32 线程的 global_colors 仓) — win32 属主
    SetGlobalColors(GlobalColors),
    /// win32 线程退出 (host 停泵 + 托盘 NIM_DELETE)
    Shutdown,
}

/// 托盘动作 (win32 线程 AppTrayHandler → 主线程监督循环)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    /// 左键/"设置" (Application.java:251-273: ctr.stop(); ctr = new Controller())
    Activate,
    /// 菜单"开始" — PORT(多出能力, 非 Java 菜单项): Java 托盘菜单仅 about/close
    /// (Application.java:223-247), 无"开始"项; Rust tray.rs 提供独立 start 入口,
    /// handler 语义 = Controller.start() 的服务启动部分 (保真)。多出面的回收
    /// 归 tray.rs 波次, 本侧仅忠实转发。
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
    /// overlay 位置存档 (win32 线程拖拽松手/销毁链 → 主线程落盘)。
    /// section = 配置组标题 (Java OverlaySettings 按 sectionName 查 GroupConfig),
    /// 坐标归一化 (Java saveWindowPosition 的 gc.x/y 量纲)
    PositionSaved { section: String, x: f64, y: f64 },
}

/// 分相监督循环 ([`AppShell::run_supervisor_phase`]) 的退出形态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorOutcome {
    /// 进程退出请求 (EndGame / 托盘 Exit / 监督通道关闭)
    Exit,
    /// 托盘 Activate 已重建核, 请求弹设置窗 (主循环回相 A)
    MainFormRequested,
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
    /// host.overlays_hidden 的跨线程镜像 (Java AlwaysOnTopCoordinator.
    /// overlaysHidden volatile — FocusMonitor 经通道桥查询; win32 处理
    /// Hide/Show 命令时与 host 同步置位)
    pub overlays_hidden: AtomicBool,
    /// 低频杂项标志 (showStatus/sessionAircraftType/currentFmHotkeyCode)
    pub flags: Mutex<ControllerFlags>,
    /// 游戏模式 Service 数据快照句柄 (start() 建 / stop() 清;
    /// win32 线程 live 喂入 + 主线程 tick 驱动读)
    pub live: RwLock<Option<Arc<RwLock<ServiceData>>>>,
    /// OverlayContext.isPreviewMode 的跨线程替身 (Java: forPreviewMode/forGameMode
    /// 两种 ctx 构建)。语义 = **会话窗口形态** (审查 blocker 收口): openpad→false /
    /// CloseAll/重建核→true; RefreshPreviews 仅在激活探测期临时置 true (对位 Java
    /// refreshPreviews 传 forPreviewMode ctx, 见 win32 命令处理点 PORT 注)。
    pub overlay_ctx_preview: AtomicBool,
    /// 最后一次 FlightDataEvent 到达时间 (ms epoch; 0 = 本核会话未见)。
    /// PORT(B1 补偿): vm-data 不外泄原始串 (http_client 轮询线程独占),
    /// 游戏退出 (HTTP 失败 → 串复位空串, http_helper NSTRING) 时 State/Indicators
    /// 的 update 不执行, flags 保留陈旧真值 — Java 的 "串空 → S4toS1" 路径
    /// (Service.java:1785-1790) 在 flags 判定下不可达。以 "事件流静默超时"
    /// 顶替: player_live 轮每 ~50ms 发布一帧, 游戏退出即停发; 静默超过
    /// [`FLIGHT_SILENT_EXIT_MS`] 且 flags/playerLive 陈旧真值 → 判定会话结束。
    /// vm-data 后续波次补 raw_strings_valid 外泄后回收本补偿。
    pub last_flight_event_ms: AtomicI64,
    /// overlay present 帧数 (win32 线程 50ms 渲染节拍, 活跃 overlay 存在时 +1;
    /// host 跨重建存活 → 跨核单调累积, 冒烟断言面)。host 无逐窗 present 计数
    /// 外泄 (render_tick Result 不分首帧/脏检查抑制), 以"活跃窗口在场的成功
    /// render_tick 次数"为 present 帧数的保守代理 (首帧必 present, 计数≥它)。
    pub render_frames: AtomicU64,
    /// 逐 overlay present 帧数 (注册面以 0 落键, 渲染节拍逐活跃窗口 +1;
    /// 从未激活/注册失败的项如实暴露 — 冒烟"全部注册 overlay present>0"判据)。
    /// 代理语义同 render_frames 注 (在场成功 render_tick ≥ 真实 present 数)。
    pub overlay_present: Mutex<std::collections::BTreeMap<String, u64>>,
}

/// Controller 低频杂项字段 (Java Controller.java:122-134/196)
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct ControllerFlags {
    /// `private boolean showStatus` (loadFromConfig 同步; StatusBar 未移植, 仅保位)
    pub show_status: bool,
    /// `private String sessionAircraftType` (onAircraftChanged 幂等去重, Controller.java:196)
    pub session_aircraft_type: Option<String>,
    /// `private int currentFmHotkeyCode` (热键重绑定跟踪, Controller.java:153)
    pub current_fm_hotkey_code: i32,
}


impl ControllerShared {
    pub fn new() -> Self {
        ControllerShared {
            preview_generation: AtomicU64::new(0),
            state: RwLock::new(ControllerState::Init),
            intervals: Mutex::new(ControllerIntervals::default()),
            overlays_hidden: AtomicBool::new(false),
            flags: Mutex::new(ControllerFlags::default()),
            live: RwLock::new(None),
            overlay_ctx_preview: AtomicBool::new(true),
            last_flight_event_ms: AtomicI64::new(0),
            render_frames: AtomicU64::new(0),
            overlay_present: Mutex::new(std::collections::BTreeMap::new()),
        }
    }

    /// 托盘重建新核前复位 (Java 构造器 L582 `State = ControllerState.INIT` 显式赋值;
    /// 审查 A-W1: sessionAircraftType 是 Controller 实例字段, Java 每次托盘重建随新
    /// 实例归 null (Controller.java:196) — Rust flags 跨核共享, 需显式复位, 否则
    /// 重建后首个不同机型被误判 is_switch。overlay_ctx_preview 同理回预览态初值,
    /// 防残留游戏模式值影响 INIT 期的激活探测)
    pub fn reset_for_rebuild(&self) {
        *self.state.write().expect("Controller 状态锁中毒") = ControllerState::Init;
        self.flags
            .lock()
            .expect("flags 锁中毒")
            .session_aircraft_type = None;
        self.overlay_ctx_preview.store(true, Ordering::SeqCst);
        self.last_flight_event_ms.store(0, Ordering::SeqCst);
    }

    /// State 快照读 (跨线程安全; 主线程写点: 各状态转移方法)
    pub fn state(&self) -> ControllerState {
        *self.state.read().expect("Controller 状态锁中毒")
    }

    /// 注册面落键: overlay id → 0 (逐窗 present 计数起点)。注册失败不落键 —
    /// 冒烟断言按 6 键全集判, 缺键即注册失败如实暴露 (不假通过)
    fn note_registered_overlay(&self, id: &str) {
        self.overlay_present
            .lock()
            .expect("overlay_present 锁中毒")
            .entry(id.to_string())
            .or_insert(0);
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
    tx: Option<Sender<DebounceMsg>>,
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
            tx: Some(tx),
            join: Some(join),
        }
    }

    pub fn sender(&self) -> Sender<DebounceMsg> {
        // shutdown 后取空句柄 (send 即 Err, 调用方一律 let _ 忽略)
        self.tx
            .clone()
            .unwrap_or_else(|| std::sync::mpsc::channel().0)
    }

    pub fn shutdown(&mut self) {
        // 先 drop 全部自有发送端 → recv 返回 Disconnected → 线程退出 → join。
        // (调用方持有的克隆 drop 前线程可能不退出 — join 前 Controller 已先行
        // drop, AppShell 字段逆序声明保证该次序)
        if let Some(j) = self.join.take() {
            self.tx = None;
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

/// overlay 注册面的 Send 参数快照 (win32 线程一次性注册用, D8: 字体→win32 线程)。
/// PORT(WYSIWYG 收口, 原审查 A-W4): 本快照仍只喂 spawn 期初始注册; 配置变更后的
/// 重建经 [`ReinitParams`] 走 `UiCommand::ReinitOverlays` (见 vm-overlay reinit.rs
/// 头注) — 主线程 CONFIG_CHANGED 时即时重建参数包直送 win32 线程的线程局部仓,
/// 各 spec 工厂的 reinit 闭包消费, 不再冻结在 spawn 时刻。
pub struct OverlayInputs {
    pub dpi_scale: f64,
    /// MiniHUD 全量设置快照
    pub hud: HudSettingsSnapshot,
    /// 引擎控制面板字号增量 (getOverlaySettings("引擎控制").get_font_size_add)
    pub font_add_engine: i32,
    /// 动力信息字号增量 + 列数 (getOverlaySettings("动力信息"))
    pub font_add_power: i32,
    pub power_columns: i32,
    /// 飞行信息字号增量 + 列数 (getOverlaySettings("飞行信息"); Java Controller:683)
    pub font_add_flight: i32,
    pub flight_columns: i32,
    /// 起落襟翼字号增量 + 边缘模式 (getOverlaySettings("起落襟翼"))
    pub font_add_gear: i32,
    pub gear_show_edge: bool,
    /// 舵面值字号增量 + 边缘模式 (getOverlaySettings("舵面值"); Java :683)
    pub font_add_axis: i32,
    pub axis_show_edge: bool,
    /// 地平仪几何/开关 (getOverlaySettings("地平仪"); 缺省 = Java reinitConfig 默认:
    /// 150×300 / 40ms / direction false / AoA 极限 true, AttitudeOverlay.java:232-248)
    pub attitude_width: i32,
    pub attitude_height: i32,
    pub attitude_freq_ms: i64,
    pub attitude_show_direction: bool,
    pub attitude_show_aoa_limits: bool,
    /// Service 轮询间隔 (MiniHUD blinkTicks/refreshInterval 同源;
    /// EngineControl loadRefreshInterval 读的 dataPollIntervalMs 亦同源)
    pub service_loop_interval_ms: i64,
    /// 全局五色快照 (Java Application.colorNum 族静态; cfg fontNum/fontLabel/
    /// fontUnit/fontWarn/fontShade → win32 线程 global_colors 仓)
    pub colors: GlobalColors,
    /// AA 开关快照 (cfg AAEnable, Java cfg 缺省 false; → global_aa 仓)
    pub aa: bool,
    /// 引擎控制 7 仪表 disable 开关 (ENGINE_DISABLE_KEYS 序; 曾 never-wired
    /// 恒 false — 用户关仪表 Rust 恒显全部, 启动首帧即错, 审查轮 1-B)
    pub engine_disables: [bool; 7],
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
        let axis = config.get_overlay_settings("舵面值");
        let attitude = config.get_overlay_settings("地平仪");
        let flight = config.get_overlay_settings("飞行信息");
        OverlayInputs {
            dpi_scale: env.dpi.get_scale(),
            hud: HudSettingsSnapshot::build(&config.get_hud_settings()),
            font_add_engine: engine.get_font_size_add(),
            font_add_power: power.get_font_size_add(),
            power_columns: power.get_int("hudColumns", 1),
            font_add_flight: flight.get_font_size_add(),
            flight_columns: flight.get_int("flightInfoColumn", 1),
            font_add_gear: gear.get_font_size_add(),
            gear_show_edge: gear.get_bool("enablegearAndFlapsEdge", false),
            font_add_axis: axis.get_font_size_add(),
            axis_show_edge: axis.get_bool("enableAxisEdge", false),
            attitude_width: attitude.get_int("attitudeIndicatorWidth", 150),
            attitude_height: attitude.get_int("attitudeIndicatorHeight", 300),
            attitude_freq_ms: attitude.get_int("attitudeIndicatorFreqMs", 40) as i64,
            attitude_show_direction: attitude
                .get_bool("attitudeIndicatorDisplayDirection", false),
            attitude_show_aoa_limits: attitude
                .get_bool("attitudeIndicatorDisplayAoALimits", true),
            // load_app_check 缺省 50 (ConfigurationService.java 同源)
            service_loop_interval_ms: if interval > 0 { interval } else { 50 },
            colors: config.global_colors(),
            aa: config.application_state().aa_enable,
            engine_disables: std::array::from_fn(|i| {
                config
                    .get_config(vm_overlay::ENGINE_DISABLE_KEYS[i])
                    .map(|v| java_parse_boolean(&v))
                    .unwrap_or(false)
            }),
        }
    }
}

/// 注册快照 → WYSIWYG reinit 参数包 (同源配置键的子集投影; 颜色/AA 有专命令不入包)
impl From<&OverlayInputs> for vm_overlay::ReinitParams {
    fn from(i: &OverlayInputs) -> Self {
        vm_overlay::ReinitParams {
            dpi_scale: i.dpi_scale,
            font_add_engine: i.font_add_engine,
            engine_disables: i.engine_disables,
            service_loop_interval_ms: i.service_loop_interval_ms,
            font_add_power: i.font_add_power,
            power_columns: i.power_columns,
            font_add_flight: i.font_add_flight,
            flight_columns: i.flight_columns,
            font_add_gear: i.font_add_gear,
            gear_show_edge: i.gear_show_edge,
            font_add_axis: i.font_add_axis,
            axis_show_edge: i.axis_show_edge,
            attitude_width: i.attitude_width,
            attitude_height: i.attitude_height,
            attitude_freq_ms: i.attitude_freq_ms,
            attitude_show_direction: i.attitude_show_direction,
            attitude_show_aoa_limits: i.attitude_show_aoa_limits,
            hud: i.hud.clone(),
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
    ui_cmd_tx: Sender<UiCommand>,
    env: Env,
    /// stop 步2 退订的订阅句柄 (RAII Drop = unsubscribe, 对位 Java unsubscribe+置 null)
    subs: Vec<Subscription<UiStateEvent>>,
    fm_sub: Option<Subscription<vm_core::fm::FMHandle>>,
    /// live 事件活跃度订阅 (B1 补偿信号, 见 ControllerShared.last_flight_event_ms;
    /// start 建 / stop 退 — 回调在 Service 发布线程, 只写原子时间戳不碰 UI)
    live_sub: Option<Subscription<FlightDataEvent>>,
    /// FlightLog 共享槽 (Java Controller.java:44 `logon` + `Log` 字段二位一体的
    /// 收敛形态): openpad/closepad/换机换入换出, Service 轮询线程每轮 logTick
    /// (Service.java:1824-1828)。随核销毁 (stop 的 closepad 路径保存)
    flight_log: FlightLogSlot,
    /// Service 线程句柄 (stop 步4: take + stop)
    pub service: Option<ServiceHandle>,
    /// Java `public MainForm M` 的存活位 (真窗归主线程 iced/W2; 此处只承载 null 判定)
    main_form_alive: bool,
    /// 网络探测开关 (ControllerDeps.probe_network, 测试注入面)
    probe_network: bool,
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
            main_event_tx,
            env,
            probe_network,
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
            env,
            subs: Vec::new(),
            fm_sub: None,
            live_sub: None,
            flight_log: Arc::new(Mutex::new(None)),
            service: None,
            main_form_alive: false,
            probe_network,
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
        let probe = self.probe_network;
        std::thread::Builder::new()
            .name("FM-Detect".to_string())
            .spawn(move || detect_and_identify(&selected, &http_header, &fm, probe))
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
        // PORT(叠加态守卫, 审查 A-W6): 托盘 Start (Rust 多出面, 见 TrayCommand::Start)
        // 起 Service 后 State 仍 Init, 用户再点 MainForm 确认时 confirm 链的
        // end_preview 无条件置 Init — 仅 Java 守卫拦不住二次 start, 会二次 spawn
        // Service (旧句柄 Drop 兜底 = 会话重启中断)。Java 无托盘 Start 入口故无
        // 此形态; 此处以"Service 已在跑"为幂等条件丢弃, 保留首次会话。
        if self.service.is_some() {
            logger::info("Controller", "Service 已在运行, 忽略重复 start (托盘 Start 与确认叠加)");
            return;
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
                app_port: cfg_port.unwrap_or(self.env.app_port),
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
            let mut fm = vm_core::focus_monitor::FocusMonitor::new(
                Arc::new(vm_overlay::platform_extras::WindowsFocusDetector),
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
        let handle = spawn_service_thread(service);
        *self.shared.live.write().expect("live 锁中毒") = Some(Arc::clone(&handle.data));
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
        let generation = self.shared.preview_generation.load(Ordering::SeqCst); // Java:852 capture
        // Java:853-856 new Thread(() -> refreshPreviews(generation)).start()
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
        // Java:237-246 延迟 100ms 建 overlay 防数据闪烁 (小睡线程 + openpad)。
        // PORT(时序偏差备案, A-W7): Java openpad 全部内容 (含 FocusMonitor enable/
        // FlightLog) 都在延迟线程内执行; Rust 仅 OpenAllOverlays 走延迟, 其余面
        // (openpad_rest) 即时执行 — FocusMonitor 现为 TODO 无功能差, 备案。
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
        // Java:352-360 autoHideOnFocusLoss → setEnabled — 已收口 (轮 2-C):
        // FocusMonitor 随 Service 装配 (start() 内按 cfg 启停, 与会话同生共死),
        // 本处不再重复读键
        // Java:366-376 FlightLog (enableLogging) — 已接线 (下方法)
        self.open_flight_log();
        // Java:378-382 UIThread — D7 弃译清单 (空转轮询线程已废)
        // Java:383-385 S.startTime — Service 内部时间面, vm-data 未外泄 (TODO(port))
    }

    /// Java openpad 的 FlightLog 段 (Controller.java:366-376): enableLogging 开 →
    /// 通知 + `Log = new FlightLog(); Log.init(this, S, configService); logon = true`。
    /// onAircraftChanged 换机开新 (331-333) 复用本方法。
    fn open_flight_log(&mut self) {
        // Java: Boolean.parseBoolean(getConfig("enableLogging"))
        if !java_parse_boolean(&self.config.get_config("enableLogging").unwrap_or_default()) {
            return;
        }
        // Java:367-370 关旧 DrawFrame — D8 豁免 (DrawFrame×2 属 P6, 不碰)
        // Java:371 NotificationService.show(Lang.cStartlog) — toast 未移植 (豁免),
        // logger 顶位留痕
        logger::info("Controller", Lang::init_lang().c_startlog);
        // Java:372-374 Log.init(this, S, configService) — init 只读 `s.sIndic.type`,
        // 与 Java 同时刻从 live ServiceData 取快照。Service 缺失 = 测试 fixture
        // 手塞 live 绕过 start() 的专有形态 (Java 轮询链 openpad 必有 S), 跳过
        let Some(handle) = self.service.as_ref() else { return };
        let data = Arc::clone(&handle.data);
        let snap = flight_log_snapshot(&data.read().unwrap_or_else(|e| e.into_inner()));
        let mut log = FlightLog::new();
        log.init(
            Arc::new(LogonSink(Arc::clone(&self.flight_log))),
            &snap,
            // config !Send (ConfigurationService 主线程独占) 不能整件入 FlightLog
            // (Service 线程 tick); 消费面仅 FlightAnalyzer.init 一次性读
            // "enableAltInformation" (flight_analyzer.rs:154-160, 此后 is_information
            // 固化不再读) — 单键快照与 Java 同一时刻同值, 语义等价
            Some(Arc::new(FlightLogConfig(
                self.config.get_config("enableAltInformation"),
            ))),
            Arc::new(|t: &str| logger::info("FlightLog", t)) as NotifySink,
            Arc::new(ServiceAnalyzerSource::new(data)),
        );
        // Java:375 logon = true → 槽 Some (Service 轮询开始 logTick)。
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
        // Java:403 `(Log != null)` 守卫 + :410 `Log = null` — take 一次完成
        let log = self
            .flight_log
            .lock()
            .expect("flight_log 槽锁中毒")
            .take();
        let Some(log) = log else { return };
        let mut log = log.lock().expect("flight_log 实例锁中毒");
        // Java:404 NotificationService.show(cSavelog + fileName + cPlsopen) —
        // toast 未移植 (豁免), logger 顶位
        let lang = Lang::init_lang();
        logger::info(
            "Controller",
            &format!("{}{}{}", lang.c_savelog, log.file_name, lang.c_plsopen),
        );
        // Java:405-408 爬升档数 ≥1 弹 DrawFrame — D8 豁免 (该行 fA==null 时 NPE
        // 的 Java bug 形态随 DrawFrame 一并豁免)
        // Java:409 Log.close(): fA==null (全程未过 |checkAlt|>10) 时 save*Data 抛
        // NPE 逃逸 closepad, 由 Service 轮询线程顶层 catch(Exception) 吞掉
        // (Service.java:1850) — 本方法在主线程 (pump) 无该兜底, catch_unwind
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
        // Java:390 FocusMonitor disable (随 Service 装配已收口, 见 openpad_rest 注)
        let _ = self.ui_cmd_tx.send(UiCommand::CloseAllOverlays); // Java:400 closeAll
        // Java:402-411 FlightLog 保存 (已接线; DrawFrame 段 D8 豁免)
        self.close_flight_log();
        // Java:413-418 UIThread 停 (D7 弃译); Java:420 System.gc() 无对应物
    }

    /// Java:251-283 S4toS1 — PREVIEW → INIT (退出游戏)。
    pub fn s4to_s1(&mut self) {
        if self.shared.state() != ControllerState::Preview {
            return;
        }
        // closepad 内含 FlightLog 保存 (Java:260 → 402-411)
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
        // Java:313-334 enableLogging → FlightLog 关旧开新 (关旧 = closepad 的
        // 319-328 段含保存通知; 开新 = openpad 的 331-333 段; DrawFrame D8 豁免)
        self.close_flight_log();
        self.open_flight_log();
        // Java:336-341 S.resetvaria() (vm-data 未外泄, TODO(port))
    }

    /// Service 轮询驱动的状态机推进 (AppShell::pump 调用)。
    /// PORT: Java Service.processPollingCycle 内联调用 c.initStatusBar/changeS2/
    /// changeS3/S4toS1 (vm-data service_loop.rs 对应位置留 TODO(port) — 本方法以
    /// ServiceData 公开字段顶替该调用面)。strState/strIndic 原始串在 HttpHelper
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
        let Some(data) = live else { return };
        let d = data.read().unwrap_or_else(|e| e.into_inner()); // 中毒穿透 (§6 契约)
        let s_flag = d.s_state.as_ref().map(|s| s.flag).unwrap_or(false);
        let i_flag = d.s_indic.as_ref().map(|i| i.flag).unwrap_or(false);
        let i_type = d.s_indic.as_ref().and_then(|i| i.r#type.clone());
        let player_live = d.player_live;
        drop(d);
        // B1 补偿判定先行: 事件流静默 + flags/playerLive 陈旧真值 = 串空 (游戏退出)。
        // 该轮对位 Java 串空分支 (L755-761): 只 S4toS1, 不 initStatusBar/changeS2
        // (两者在 Java 串非空分支内 — 若照跑, 退出后状态会被 initStatusBar 重新
        // 推到 Connected/InGame, s4to_s1 的 Preview 守卫即永久拦断)。
        // last=0 视为非静默 (flags 真值必经串非空轮, 该轮 player_live 置真即同轮
        // 发布事件, 竞态窗口远小于阈值; 保守侧防首轮误判)。
        let last = self.shared.last_flight_event_ms.load(Ordering::SeqCst);
        let silent = last != 0 && current_time_millis().saturating_sub(last) > FLIGHT_SILENT_EXIT_MS;
        if silent && s_flag && i_flag && player_live {
            self.s4to_s1(); // Java:758 串空路径的补偿触发
            return;
        }
        self.init_status_bar(); // Java:570 (每轮, 守卫在方法内)
        if s_flag && i_flag {
            self.change_s2(); // Java:598
            if player_live {
                let t = i_type.clone();
                self.change_s3(t.as_deref()); // Java:649 打开面板 (首进, guarded)
                // Java:656-659 每轮 identify (service_loop TODO(port) 的顶替调用面;
                // 目标未变零成本 — 换机时 FMManager 异步切句柄, P4 轻量 swap 语义)
                self.fm.identify(t.as_deref());
                self.on_aircraft_changed(i_type.as_deref()); // Java:668 换机
            }
            // else: Java 649 前的 playerLive 探测等待, 无 Controller 调用
        } else {
            self.s4to_s1(); // Java:750 flag 丢失路径 (flags 新值假, 真实可达)
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
    /// `game_mode`: 对齐 `autoStartGameMode=true` 配置 (CLI --game-mode / e2e —
    /// Java 无此开关, 由用户配置表达; 此处以等效配置注入, Controller 自启动
    /// 判定路径零特判)。
    pub fn new(debug: bool, game_mode: bool) -> Result<AppShell, String> {
        AppShell::new_with_port(debug, game_mode, None)
    }

    /// 白盒端口覆盖 (`--port` CLI / mock-smoke 的 9222 约定): Env 只读区在 probe
    /// 后覆写, 语义 = Lang.httpPort 解析结果的等价替换 (bkp 同步 +1111 保持
    /// 备用端口关系)。生产路径 (desktop_main) 不传 — 端口仍由 Lang/配置表达;
    /// 白盒测试统一走 9222 (游戏本地 API 恒占 8111, 备用端口域游戏永不监听,
    /// 真机在跑也不再挤掉测试)。
    pub fn new_with_port(
        debug: bool,
        game_mode: bool,
        port_override: Option<u16>,
    ) -> Result<AppShell, String> {
        let lang = Lang::init_lang();
        let mut env = Env::probe(&lang, debug);
        if let Some(p) = port_override {
            env.app_port = p;
            // 与 probe 同式 (域内恒 p+1111, u16 加法无回绕面)
            env.app_port_bkp = p + 1111;
        }
        let ui_bus = Arc::new(EventBus::new());
        let config = ConfigurationService::new(Some(Arc::clone(&ui_bus)));
        // Java Controller 构造器: configService.initConfig() 装载设置文件
        config.init_config();
        if game_mode {
            // 对位 Java e2e 的 autoStartGameMode=true 配置 (Controller.java:589-606
            // 自启动分支: 跳过 MainForm 直接 start Service)
            use vm_core::config_api::ConfigProvider as _;
            config.set_config("autoStartGameMode", "true");
        }
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
        // Java:470 每核 new ConfigurationService + initConfig (配置树随核重建)。
        // 首核复用注入配置 (AppShell::new 已 initConfig / 测试 tmp cfg — 免重复装载
        // 与写盘副作用); 托盘重建核走磁盘新装载
        let config = match self.initial_config.take() {
            Some(c) => c,
            None => {
                let config = ConfigurationService::new(Some(Arc::clone(&self.ui_bus)));
                config.init_config();
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
                // Java:508-512 导入/重置后热键绑定同步 (key=RESET_COMPLETED)
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

    /// 阻塞监督循环 (无 MainForm 场景: --game-mode / 冒烟; Java 托盘+EDT 泵的对位)。
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
            hm.shutdown(); // Java shutdown() 不清全局钩子; Rust 实例自持钩子, 卸钩收线程
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
    /// overlay 初始位置快照 (id → 归一化; 主线程 spawn 前从 GroupConfig.x/y 取,
    /// win32 线程不能碰 !Send 配置树 — 见 ChannelPositionStore 头注)
    pub position_snapshot: HashMap<String, (f64, f64)>,
}

/// win32 线程内注册的 overlay 数据句柄 (Rc — 恒留本线程)。
/// None = spec 工厂失败 (字体缺失等, 注册点已 logger::error), 喂入跳过
struct OverlayHandles {
    /// MiniHUD live 喂入口 (Java onFlightData → EDT 的单线程 host 对位)
    minihud: Option<MiniHudHandle>,
    /// 动力信息 (Java PowerInfoOverlay.onFlightData 50ms 节流)
    power_info: Option<PowerInfoHandle>,
    /// 引擎控制 (Java EngineControlOverlay.onFlightData, 间隔配置驱动 ×2)
    engine_control: Option<EngineControlHandle>,
    /// 起落襟翼 (Java GearFlapsOverlay.onFlightData 100ms 节流)
    gear_flaps: Option<GearFlapsHandle>,
    /// 地平仪 (Java AttitudeOverlay.drawTick, freqMili 节流归喂入侧)
    attitude: Option<AttitudeOverlayHandle>,
    /// 操纵面 (Java ControlSurfacesOverlay.onFlightData 50ms 节流)
    control_surfaces: Option<ControlSurfacesHandle>,
    /// 飞行信息 (Java FlightInfoOverlay.onFlightData 字段行; POC 专径收编批接入)
    flight_info: Option<FlightInfoHandle>,
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

/// MiniHUD withInterest 键 (Java Controller.java:676-678 逐字对齐; 测试
/// minihud_interest_keys_hit_ui_layout_cfg 以此为源核对 cfg 键空间 — 审查 W1:
/// 曾笔误 "showAttitudeIndicator", 前缀匹配下不命中任何 cfg 键, 开关失效)
const MINIHUD_INTEREST_KEYS: [&str; 13] = [
    "displayCrosshair",
    "drawHUD",
    "disableHUD",
    "crosshair",
    "miniHUD",
    "enableLayoutDebug",
    "enableFlapAngleBar",
    "hudMach",
    "showSpeedBar",
    "showAttitudeGauge",
    "attitudeIndicatorInertialMode",
    "alwaysShowRadarAltitude",
    "showHUD",
];

/// FocusMonitor 的通道桥 (轮 2-C 收口): Service 轮询线程内 FocusMonitor tick →
/// coordinator 回调 → UiCommand 送 win32 线程执行 host hide/show (配置/窗口
/// !Send 不能进 Service 线程 — ChannelPositionStore 同款模式)。
/// is_overlays_hidden 读 ControllerShared 镜像 (win32 处理命令时同步)
struct ChannelFocusBridge {
    tx: Sender<UiCommand>,
    shared: Arc<ControllerShared>,
}

impl vm_core::focus_monitor::AlwaysOnTopCoordinatorApi for ChannelFocusBridge {
    fn is_overlays_hidden(&self) -> bool {
        self.shared.overlays_hidden.load(Ordering::SeqCst)
    }
    fn hide_all_overlays(&self) {
        let _ = self.tx.send(UiCommand::HideAllOverlays);
    }
    fn show_all_overlays(&self) {
        let _ = self.tx.send(UiCommand::ShowAllOverlays);
    }
}

/// 全局五色 cfg 键 (ui_layout.cfg:379-383; Java ConfigurationService.java:136-140
/// loadFromConfig 读入 Application 静态)
const GLOBAL_COLOR_KEYS: [&str; 5] = ["fontNum", "fontLabel", "fontUnit", "fontWarn", "fontShade"];

/// 窗口 overlay id → 配置组标题 (Java Controller 各 init 的 getOverlaySettings
/// 字面量, Controller.java:656-714; MiniHUD 经 getHUDSettings → sectionName
/// "MiniHUD", ConfigurationService.java:569)。位置持久化按此映射读写
/// GroupConfig.x/y; 测试 overlay_sections_hit_ui_layout_cfg 以 cfg 为源核对。
/// (flightInfoSwitch 走 window.rs 专径无 host 条目, 不列; enableVoiceWarn/
/// enableFMPrint/thrustdFS 非窗口条目同不列)
const OVERLAY_SECTIONS: [(&str, &str); 7] = [
    ("enableEngineControl", "引擎控制"),
    ("engineInfoSwitch", "动力信息"),
    ("crosshairSwitch", "MiniHUD"),
    ("flightInfoSwitch", "飞行信息"),
    ("enableAxis", "舵面值"),
    ("enableAttitudeIndicator", "地平仪"),
    ("enablegearAndFlaps", "起落襟翼"),
];

/// 位置存档后端 (win32 线程侧): 启动快照直读 + 保存经 MainEvent 回传主线程落盘。
/// PORT(线程桥): Java overlay 直接持 OverlaySettings (EDT 单世界); Rust 配置树
/// !Send 不能进 win32 线程, 位置面拆成 读=启动快照 (位置仅拖拽改变, 而拖拽存档
/// 双写快照, 快照不滞后) 写=回传 (PositionSaved → save_group_position 落盘,
/// 对齐 Java saveWindowPosition 即时 saveLayoutConfig)。
struct ChannelPositionStore {
    snapshot: HashMap<String, (f64, f64)>,
    tx: Sender<MainEvent>,
}

impl vm_overlay::host::PositionStore for ChannelPositionStore {
    fn load(&mut self, id: &str) -> Option<(f64, f64)> {
        self.snapshot.get(id).copied()
    }
    fn store(&mut self, id: &str, x: f64, y: f64) {
        self.snapshot.insert(id.to_string(), (x, y));
        if let Some((_, section)) = OVERLAY_SECTIONS.iter().find(|(sid, _)| *sid == id) {
            let _ = self.tx.send(MainEvent::PositionSaved {
                section: section.to_string(),
                x,
                y,
            });
        }
    }
}

/// Java Controller.registerGameModeOverlays (651-753) 的 win32 侧一次性注册。
/// PORT(偏差备案): Java 每 Controller 重建 OverlayManager + 重注册; Rust host 跨
/// 重建存活 (D8), 条目是无状态配置记录 (id/config_key/尺寸/渲染闭包), 重建语义
/// 由激活探测 (实时配置) + 命令通道承载 — 重注册无信息增量。
///
/// 注册键 10/10 落位 (P6 收口 + 人工验收补口):
/// - 窗口条目 7: enableEngineControl / engineInfoSwitch / crosshairSwitch /
///   flightInfoSwitch (POC window.rs 专径收编, vm-overlay flight_info.rs) /
///   enablegearAndFlaps / enableAxis / enableAttitudeIndicator。
/// - 非窗口/降级 3 (键在激活缓存 ACTIVATION_KEYS / strategy_for 留有映射, 不建窗口):
///   - enableVoiceWarn: VoiceWarning 为 FlightDataBus 订阅者形态非窗口 (TODO(port))
///   - enableFMPrint: FMUnpackedData 需 host 扩展 (动态窗口高 resize + 逐条目可见性,
///     overlays_field2.rs 头注 P5 组装契约), 键留激活缓存无窗口条目
///   - thrustdFS (DrawFrameSimpl): D8 降级清单 P6 尾巴 (live 喂数覆盖的唯一豁口)
fn register_game_mode_overlays(
    host: &mut OverlayHost,
    handles: &mut OverlayHandles,
    env: &Env,
    inputs: &OverlayInputs,
    // WYSIWYG reinit 参数仓 (CONFIG_CHANGED 后 ReinitOverlays 命令覆写;
    // 各 spec 工厂 reinit 闭包持引用读取 — 见 vm-overlay reinit.rs 头注)
    params: &Rc<RefCell<vm_overlay::ReinitParams>>,
    lang: &Rc<Lang>,
    shared: &ControllerShared,
) {
    let fonts = &env.fonts_dir;
    // 引擎控制 (Java:654-659, 键 enableEngineControl); dataPollIntervalMs 经
    // loadRefreshInterval ×2 → refreshInterval (preview 工厂传不了此参恒默认 100)
    match engine_control_overlay_spec(fonts, Rc::clone(lang), params) {
        Ok((h, spec)) => {
            shared.note_registered_overlay(&spec.id);
            handles.engine_control = Some(h);
            host.register(spec)
                .with_interest(&["disableEngineInfo", "fontSize"]);
        }
        Err(e) => logger::error("Controller", &format!("引擎控制 overlay 注册失败: {}", e)),
    }
    // 动力信息 (Java:662-667, 键 engineInfoSwitch)
    match power_info_overlay_spec(fonts, params) {
        Ok((h, spec)) => {
            shared.note_registered_overlay(&spec.id);
            handles.power_info = Some(h);
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
        params,
    ) {
        Ok((h, spec)) => {
            shared.note_registered_overlay(&spec.id);
            handles.minihud = Some(h);
            host.register(spec).with_interest(&MINIHUD_INTEREST_KEYS);
        }
        Err(e) => logger::error("Controller", &format!("MiniHUD overlay 注册失败: {}", e)),
    }
    // 飞行信息 (Java:683-686, 键 flightInfoSwitch) — POC window.rs 专径收编
    // (渲染栈复用 fields/layout/render 对拍三件套, 见 vm-overlay flight_info.rs)
    match flight_info_overlay_spec(fonts, params) {
        Ok((h, spec)) => {
            shared.note_registered_overlay(&spec.id);
            handles.flight_info = Some(h);
            host.register(spec)
                .with_interest(&["flightInfo", "fontSize", "disableFlightInfo"]);
        }
        Err(e) => logger::error("Controller", &format!("飞行信息 overlay 注册失败: {}", e)),
    }
    // 起落襟翼 (Java:709-714, 键 enablegearAndFlaps)
    match gear_flaps_overlay_spec(fonts, params) {
        Ok((h, spec)) => {
            shared.note_registered_overlay(&spec.id);
            handles.gear_flaps = Some(h);
            host.register(spec)
                .with_interest(&["enablegearAndFlapsEdge", "fontSize"]);
        }
        Err(e) => logger::error("Controller", &format!("起落襟翼 overlay 注册失败: {}", e)),
    }
    // 操纵面 (Java:680-687, 键 enableAxis) — 本批补齐 (批十四 A-W5 备案收口)
    match control_surfaces_overlay_spec(fonts, params) {
        Ok((h, spec)) => {
            shared.note_registered_overlay(&spec.id);
            handles.control_surfaces = Some(h);
            host.register(spec)
                .with_interest(&["enableAxisEdge", "fontSize"]);
        }
        Err(e) => logger::error("Controller", &format!("操纵面 overlay 注册失败: {}", e)),
    }
    // 地平仪 (Java:690-697, 键 enableAttitudeIndicator) — 本批补齐 (同上)
    match attitude_overlay_spec(params) {
        Ok((h, spec)) => {
            shared.note_registered_overlay(&spec.id);
            handles.attitude = Some(h);
            host.register(spec)
                .with_interest(&["attitudeIndicator", "enableAttitudeIndicator"]);
        }
        Err(e) => logger::error("Controller", &format!("地平仪 overlay 注册失败: {}", e)),
    }
}

/// 托盘 handler: 动作转发主线程 (Java 托盘回调在 EDT, Rust 泵线程→channel)。
/// TODO(port) (P6 NotificationService 族, 审查 A-W5): Java 托盘菜单 about 项
/// (Application.java:236-245, NotificationService.showAbout×3) 未移植 —
/// tray.rs 菜单面加 about 项后在此转发 (tray.rs 头注已声明归组装层挂接)。
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

/// 地平仪喂入节流状态 (Java AttitudeOverlay.java:96 freqMili + :352 freqCheckMili;
/// update_telemetry 无节流闩 — 组件头注 "40ms 节流在 onFlightData 组装层")
struct AttitudeFeedState {
    freq_ms: i64,
    last_ms: i64,
}

/// 全部窗口 overlay 的 live 喂入 (Java 各 overlay init(S) 时自订 FlightDataBus 的
/// 单点对位; Rust 订阅生命周期由 OpenAllOverlays/CloseAllOverlays 承载, 本函数在
/// 订阅期由 win32 渲染节拍调用, drain_latest 只留最新帧 = EDT repaint 合并)。
///
/// PORT(preview 门控): Java preview 实例 (initPreview) 不订阅 FlightDataBus, 恒显
/// previewValue 静态; Rust host 单条目跨 open/refresh_preview 存活 (D8), 预览窗口
/// 形态 (overlay_ctx_preview=true, CloseAll/重建核置位 — 会话窗口形态语义) 喂入
/// 整帧跳过 — MiniHUD 此前的无条件喂入一并收口 (原 B-W3 备案族的收窄, 游戏内设
/// 置窗期不再渗 live 数据)。游戏稳态 (openpad 后) 恒 false, RefreshPreviews 不再
/// 翻转本标志 (Java refreshPreviews 对在场实例只调 reinitializer, 订阅不动 —
/// OverlayManager.java:332-336)。
///
/// PORT(MiniHUD 事件重构): FlightDataEvent 不可 Clone (OpaqueObject) — 转发链只送
/// EventPayload, 本侧重构事件对象; **state/indicators 从 live guard 现值重打快照**
/// (Java 事件携带 sState/sIndic 共享可变引用, EDT 回退路径读到的是 EDT 时刻的
/// 最新值 — 按喂入时刻快照即同一时序语义; 曾长期传 None/None, hud_calculator 的
/// sState 整块被跳过, 襟翼/油门/姿态/G 值全 0 = "bar 恒 0" 根因); hud_data 恒
/// None → 走 MiniHudOverlay 的 EDT 回退计算路径 (minihud.rs update_from_event
/// 的 None 分支)。
///
/// PORT(锁内计算形态备案, 审查 B-W2): 各 update 需要 &ServiceData 完整视图, 而
/// 签名归 vm-overlay (不可越文件改) 且 ServiceData 无 Clone — 读锁跨纯计算段
/// (无回调/IO/回写, 读锁共享不阻塞读者, 仅推迟 Service 写者排队)。与 Java 的 EDT
/// 回退路径 (MiniHUDOverlay EDT 内直接读 Service 公开字段无锁计算) 同形态。
/// vm-data 后续波次出 Clone/字段快照 API 后, 改锁内快照释放再算。
///
/// PORT(panic 边界): ServiceData 的保真 panic 点 (get_pitch/get_thrust 的空引擎
/// 数组索引, service_fields.rs 注) 在畸形 s_state (update 失败 pitch/thrust 未填)
/// 下可达 — Java NPE 由 AWT EDT 吞掉 (UI 存活), Rust win32 线程 panic 会杀整个
/// host 泵, 故整帧 catch_unwind (AssertUnwindSafe: 状态可能半更新, 对位 Java
/// EDT 半更新后吞 NPE 的形态), ERROR 留痕丢帧继续。
fn feed_overlays_live(
    handles: &OverlayHandles,
    payload: &EventPayload,
    shared: &ControllerShared,
    fm: &FMManager,
    settings: &HudSettingsSnapshot,
    lang: &Lang,
    attitude_feed: &mut AttitudeFeedState,
) {
    // preview 门控 (见函数头注 PORT(preview 门控))
    if shared.overlay_ctx_preview.load(Ordering::SeqCst) {
        return;
    }
    let live = shared.live.read().expect("live 锁中毒").clone();
    let Some(data) = live else { return };
    let Ok(guard) = data.read() else {
        return; // 中毒帧跳过 (Java 无锁无此形态; §6 契约下 Service 线程仍在轮询)
    };
    let now = current_time_millis();
    let fm_handle = fm.current();
    // PORT(getload 过渡期禁令, hud_calculator.rs L197-205 ⚠): Blkx::parse 等价
    // doLoad=false — is_v_wing 恒 None, calculate() 的 FM 分支 unwrap 必 panic
    // ("getload 波次落地前禁止把 calculate() 接入 service_loop")。W1 设计的
    // None-hud_data 回退路径在此形态不可达; 过渡期处置: 该形态 FM 不进喂入
    // (走无 FM 降级路径, CLAUDE.md "指标按 0/上次值/MAX_VALUE 降级" 语义),
    // 一次性 ERROR 上报 (禁令反对的是静默吞 panic — 显式留痕 + 帧继续渲染);
    // getload 波次落地 (is_v_wing 被 populate) 后本守卫自然失效, 随该波次移除。
    let blkx = match fm_handle.blkx.as_ref() {
        Some(b) if b.is_v_wing.is_none() => {
            static WARNED: std::sync::Once = std::sync::Once::new();
            WARNED.call_once(|| {
                logger::error(
                    "Controller",
                    "FM 已装载但 getload 未译 (is_v_wing=None) — MiniHUD live 喂入走无 FM 降级路径 (hud_calculator.rs 过渡期禁令)",
                );
            });
            None
        }
        other => other,
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // 1. MiniHUD (Java MiniHUDOverlay.onFlightData → invokeLater)
        if let Some(h) = handles.minihud.as_ref() {
            // state/indicators 快照重建 (函数头 PORT(MiniHUD 事件重构) 注):
            // hud_calculator 从事件读 sState/sIndic (flaps/throttle/gear/airbrake/
            // aoa/ny/姿态), 丢掉即整块 0
            let state_box = guard
                .s_state
                .as_ref()
                .map(|s| Box::new(snapshot_state(s)) as OpaqueObject);
            let indic_box = guard
                .s_indic
                .as_ref()
                .map(|i| Box::new(snapshot_indicators(i)) as OpaqueObject);
            let event = FlightDataEvent::new(payload.clone(), state_box, indic_box);
            // AoA 告警/状态色 = 全局仓 (Java HUDCalculator.java:132-155 每次计算
            // 直读 Application 静态; 曾传编译期常量冻结 — 审查轮 1-B)
            let gc = vm_overlay::global_colors::colors();
            let colors = HudColors {
                color_warning: gc.warning,
                color_num: gc.num,
                color_unit: gc.unit,
            };
            h.borrow_mut().on_flight_data(
                now,
                &event,
                Some(&*guard),
                blkx,
                settings,
                &colors,
            );
        }
        // 2. 动力信息 (Java FieldOverlay.onFlightData 50ms 节流闩内置)
        if let Some(h) = handles.power_info.as_ref() {
            h.borrow_mut().update(now, &*guard);
        }
        // 3. 引擎控制 (节流闩 = refreshInterval 配置驱动; compressorStages 档位数 =
        //    Java FMManager.current().compressorStages, 非 READY/喷气机 → None)
        if let Some(h) = handles.engine_control.as_ref() {
            let stages = fm_handle
                .compressor_stages
                .as_ref()
                .map(|v| v.len() as i32);
            h.borrow_mut().update(now, &*guard, payload, stages);
        }
        // 4. 起落襟翼 (100ms 节流闩内置)
        if let Some(h) = handles.gear_flaps.as_ref() {
            h.borrow_mut().update_tick(now, lang, &*guard);
        }
        // 5. 操纵面 (50ms 节流内置; has_service = Java init(S) 的 xs!=null 数据门控,
        //    单实例形态下由喂入点随游戏窗口形态置位 — 见工厂头注 PORT(数据门控))
        // 飞行信息 (Java FlightInfoOverlay.onFlightData 字段行更新, 无节流 —
        // host 50ms 渲染节拍 + 像素指纹兜底; 数据 = Deriver 整包快照)
        if let Some(h) = handles.flight_info.as_ref() {
            h.borrow_mut().update_from_values(&guard.flight_values);
        }
        if let Some(h) = handles.control_surfaces.as_ref() {
            let mut cs = h.borrow_mut();
            cs.has_service = true;
            cs.on_flight_data(
                now,
                guard.get_aileron(),
                guard.get_elevator(),
                guard.get_rudder(),
                guard.get_wing_sweep(),
                guard.is_wing_sweep_valid(),
            );
        }
        // 6. 地平仪 (节流 = freqMili 40ms 配置驱动, 喂入侧承载;
        //    aoa_limits = blkx.NoFlapsWing.AoACritHigh/Low, 无 FM → None 不显示)
        if let Some(h) = handles.attitude.as_ref() {
            if now - attitude_feed.last_ms > attitude_feed.freq_ms {
                attitude_feed.last_ms = now;
                let aoa_limits = blkx
                    .and_then(|b| b.no_flaps_wing.as_ref())
                    .map(|w| (w.aoa_crit_high, w.aoa_crit_low));
                h.borrow_mut().update_telemetry(
                    guard.get_aoa(),
                    guard.get_aos(),
                    guard.get_aviahorizon_pitch(),
                    guard.get_aviahorizon_roll(),
                    guard.get_compass(),
                    aoa_limits,
                );
            }
        }
    }));
    if result.is_err() {
        logger::error(
            "Controller",
            "live 喂入帧 panic 已吞 (畸形数据帧, 对位 Java EDT NPE 吞), 帧丢弃继续",
        );
    }
}

/// win32 线程入口 (D8 拓扑): OverlayHost 泵 + 托盘 + 热键事件消费。
///
/// PORT(热键拓扑豁免记录, hotkey.rs 头注 D8 偏差): WH_KEYBOARD_LL 钩子固化在
/// HotkeyManager 自管的独立钩子线程 (jnativehook 独立派发线程的保真形态);
/// D8 的"并入单泵"需 hotkey.rs 提供外部线程装钩入口 (未提供, 本批次不越文件改),
/// 豁免期内钩子事件经 channel 汇入本线程统一消费 — 与托盘/overlay 共享的
/// 泵约束 (安装线程需泵) 由钩子线程自泵满足, 行为面一致。
/// 跟踪项 (审查 B-W4): 豁免收口 = hotkey.rs 提供外部线程装钩入口; 且
/// FM_OVERLAY_TOGGLE 的发布线程从 Java 的钩子线程变为本 win32 线程 (经
/// hotkey_rx 中转后 publish ui_bus) — 后续接 DrawFrame 系订阅方时注意此差异。
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
        position_snapshot,
    } = cfg;

    // 全局五色注入 (Java Application.colorNum 族静态的运行时值; cfg 经
    // loadFromConfig 覆盖, 此前组件用编译期 Java 初始默认 — 人工验收发现的
    // 颜色不一致根源)。须先于任何组件渲染
    vm_overlay::global_colors::set(inputs.colors);
    vm_overlay::global_colors::set_aa(inputs.aa);

    // ---- host 构建 + 激活探测 (Java new OverlayManager + ActivationStrategy) ----
    let mut host = OverlayHost::new();
    // 位置存档后端 (Java overlay 的 OverlaySettings 位置面; 快照读 + 回传写)
    host.with_position_store(Box::new(ChannelPositionStore {
        snapshot: position_snapshot,
        tx: main_event_tx.clone(),
    }));
    let ctx = HostActivationCtx {
        activation: Arc::clone(&activation),
        fm: Arc::clone(&fm),
        shared: Arc::clone(&shared),
        debug: env.debug,
    };
    host.with_activation(Box::new(move |key: &str| {
        strategy_for(key).should_activate(&ctx)
    }));
    let mut handles = OverlayHandles {
        minihud: None,
        power_info: None,
        engine_control: None,
        gear_flaps: None,
        attitude: None,
        control_surfaces: None,
        flight_info: None,
    };
    // Lang 一次构造 (GearFlaps update_tick 的标签源; 注册面与喂入共用)。
    // Rc 共享: engine 工厂的 reinit 闭包重建 state 需要标签源 (Lang !Clone)
    let lang = Rc::new(Lang::init_lang());
    // WYSIWYG reinit 参数仓 (初始 = 注册快照投影; CONFIG_CHANGED 后
    // UiCommand::ReinitOverlays 覆写, 各 spec 工厂 reinit 闭包读取)
    let params = Rc::new(RefCell::new(vm_overlay::ReinitParams::from(&inputs)));
    register_game_mode_overlays(&mut host, &mut handles, &env, &inputs, &params, &lang, &shared);
    // live 喂入用设置快照 (注册面同源; ReinitOverlays 命令同步覆写 — MiniHUD
    // on_flight_data 的 settings 参数不再冻结在 spawn 时刻)
    let mut hud_settings = inputs.hud;
    // 地平仪 40ms 喂入节流 (freqMili 配置快照; last_ms=0 = 首帧放行;
    // ReinitOverlays 命令同步刷新 freq_ms)
    let mut attitude_feed = AttitudeFeedState {
        freq_ms: inputs.attitude_freq_ms,
        last_ms: 0,
    };

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
            let tick_active = match host.render_tick() {
                Err(e) => {
                    logger::error("OverlayHost", &format!("render_tick: {}", e));
                    Vec::new()
                }
                Ok(()) => host.active_ids(),
            };
            if !tick_active.is_empty() {
                // present 帧数代理计数 (见 ControllerShared.render_frames 注)
                shared.render_frames.fetch_add(1, Ordering::SeqCst);
                // 逐窗计数 (见 ControllerShared.overlay_present 注): 注册面 0 落键,
                // 此处在场即 +1 — 从未激活的注册项保留 0, 冒烟断言可判
                let mut counts = shared
                    .overlay_present
                    .lock()
                    .expect("overlay_present 锁中毒");
                for id in &tick_active {
                    *counts.entry(id.clone()).or_insert(0) += 1;
                }
            }
            // live 数据喂入 (只留最新帧; preview 期整帧跳过 — feed_overlays_live 门控)
            if let Some(payload) = drain_latest(&flight_rx) {
                feed_overlays_live(
                    &handles,
                    &payload,
                    &shared,
                    &fm,
                    &hud_settings,
                    &lang,
                    &mut attitude_feed,
                );
            }
        }
        // UI 命令 (生命周期/WYSIWYG 的 win32 属主面)
        while let Ok(cmd) = ui_cmd_rx.try_recv() {
            match cmd {
                UiCommand::OpenAllOverlays => {
                    // Java openpad → openAll; live 订阅随建 (overlay 订阅生命周期)。
                    // P6 收口 (原审查 B-W3): live 喂入已覆盖全部 6 个窗口 overlay
                    // (feed_overlays_live — MiniHUD/PowerInfo/EngineControl/GearFlaps/
                    // ControlSurfaces/Attitude 共享句柄形态); FlightInfo 走 window.rs
                    // 专径自接, thrustdFS 为 D8 降级尾巴。
                    shared.overlay_ctx_preview.store(false, Ordering::SeqCst); // forGameMode
                    // 操纵面数据门控 (overlays_field2.rs PORT(数据门控)): Java init(S)
                    // 的 xs!=null 在此翻转 — openpad 即游戏形态 (has_service=true)
                    if let Some(h) = handles.control_surfaces.as_ref() {
                        h.borrow_mut().has_service = true;
                    }
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
                    drop(flight_sub.replace(sub));
                }
                UiCommand::CloseAllOverlays => {
                    // 会话窗口形态回预览态 (审查 blocker 收口): Java closeAll → 实例
                    // 销毁, 之后 refreshPreviews 重建的是 initPreview 实例 (无 live
                    // 订阅); overlay_ctx_preview 的窗口形态门控在此复位, 防游戏会话
                    // 结束后 preview 窗渗 live 残帧
                    shared.overlay_ctx_preview.store(true, Ordering::SeqCst);
                    // 操纵面门控同步复位 (Java preview 实例 xs=null 恒显静态值)
                    if let Some(h) = handles.control_surfaces.as_ref() {
                        h.borrow_mut().has_service = false;
                    }
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
                    // PORT(forPreviewMode 仅激活探测期, 审查 blocker 修复): Java
                    // refreshPreviews 以 forPreviewMode ctx 判定激活 (OverlayManager
                    // .java:203), 但对在场实例只调 reinitializer — 实例保留、
                    // FlightDataBus 订阅不动, live 流持续 (OverlayManager.java:332-336)。
                    // 原实现把 overlay_ctx_preview 永久置 true: 游戏稳态 State=Preview
                    // 下 FM_CHANGED/ConfigChanged 必经本分支 → feed_overlays_live
                    // 永久 early-return, 全部 overlay 冻结在 FM 加载完成瞬间的值。
                    // 改为激活探测期临时置 preview, 完毕恢复会话窗口形态
                    // (openpad→false / CloseAll/重建核→true)。
                    let session_preview = shared.overlay_ctx_preview.swap(true, Ordering::SeqCst);
                    let r = match changed_key.as_deref() {
                        Some(k) => host.refresh_preview_key(Some(k)), // Java refreshPreviews(key)
                        None => host.refresh_preview(),               // Java refreshAllPreviews
                    };
                    shared.overlay_ctx_preview.store(session_preview, Ordering::SeqCst);
                    if let Err(e) = r {
                        logger::error("OverlayHost", &format!("refresh_previews: {}", e));
                    }
                }
                UiCommand::ReinitActiveOverlays => host.reinit_active_overlays(),
                // WYSIWYG reinit 参数仓覆写 (不直接触发刷新 — 后继
                // RefreshPreviews/ReinitActiveOverlays 消费最新参数, 命令入队序
                // 即消费序; MiniHUD live 喂入快照与地平仪节流同步解冻)
                UiCommand::ReinitOverlays { params: new_params } => {
                    attitude_feed.freq_ms = new_params.attitude_freq_ms;
                    hud_settings = new_params.hud.clone();
                    *params.borrow_mut() = *new_params;
                }
                // 全局五色更新: 仓内直写, 下帧渲染生效 (reinit 标脏不必须 —
                // 色变本身改变渲染输出, host 像素指纹自然触发重绘)
                UiCommand::SetGlobalColors(c) => vm_overlay::global_colors::set(c),
                UiCommand::SetAa(on) => vm_overlay::global_colors::set_aa(on),
                // FocusMonitor 通道桥目标 (Java hideAllOverlays/showAllOverlays;
                // host 幂等标志防重复, shared 镜像供桥回读)
                UiCommand::HideAllOverlays => {
                    host.hide_all_overlays();
                    shared.overlays_hidden.store(true, Ordering::SeqCst);
                }
                UiCommand::ShowAllOverlays => {
                    host.show_all_overlays();
                    shared.overlays_hidden.store(false, Ordering::SeqCst);
                },
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
        fixture_cfg(
            "(panel \"T\" :visible true\n\
             \x20 (item \"hud\" :type switch :target \"crosshairSwitch\" :value true)\n\
             \x20 (item \"engine\" :type switch :target \"enableEngineControl\" :value false)\n\
             \x20 (item \"auto\" :type switch :target \"autoStartGameMode\" :value false))\n\
            ",
        )
    }

    /// fixture 内容直装 (autoStartGameMode 变体等)
    fn fixture_cfg(content: &str) -> String {
        tmp_cfg(content)
    }

    /// 自启动变体配置 (对位 --game-mode / Java autoStartGameMode=true)
    fn auto_start_cfg() -> String {
        fixture_cfg(
            "(panel \"T\" :visible true\n\
             \x20 (item \"auto\" :type switch :target \"autoStartGameMode\" :value true))\n\
            ",
        )
    }

    /// AppShell 测试装配: tmp cfg (无 init_config 写盘副作用) + 30ms 短防抖 +
    /// **网络隔离** (Service 指向 9 号死端口 — 连接立即拒绝; FM-Detect 探测关闭
    /// — 8111 可能被 mock/游戏占用, 项目惯例端口占用即隔离, 不做假通过);
    /// 不起 win32 线程 — ui_cmd 接收端留在 shell 内供测试观察。
    fn fixture() -> AppShell {
        fixture_with_debounce(30)
    }

    fn fixture_with_debounce(ms: u64) -> AppShell {
        fixture_full(ms, test_cfg())
    }

    /// 全参 fixture (自定义 cfg 内容; 见 fixture_with_debounce 注)
    fn fixture_full(ms: u64, cfg: String) -> AppShell {
        let ui_bus = Arc::new(EventBus::new());
        let config = ConfigurationService::new(Some(Arc::clone(&ui_bus)));
        config.load_layout(&cfg);
        let (hotkey, hotkey_rx) = HotkeyManager::with_channel();
        let mut env = Env::probe(&Lang::init_lang(), false);
        env.app_port = 9; // discard 端口: 无服务监听, connect 立即 RST
        env.app_port_bkp = 9;
        // 字体目录钉在仓库根 (cargo 测试 CWD=crate 根, CWD 相对探测不稳;
        // win32 线程注册面的 spec 工厂需要真实字体文件)
        env.fonts_dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts");
        let mut shell = AppShell::with_parts(ShellParts {
            env,
            config,
            ui_bus,
            flight_bus: Arc::new(FlightDataBus::new()),
            fm: Arc::new(FMManager::new(Arc::new(EventBus::new()))),
            hotkey,
            hotkey_rx,
            debounce_delay: Duration::from_millis(ms),
        });
        // 网络探测关闭 (见 fixture 注): FM-Detect/Preview 只走 selectedFM0 兜底
        shell.probe_network_for_test(false);
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

    /// flags 丢失 → S4toS1: Preview → Init + FM 目标清除 (会话结束语义)。
    /// 真实形态 (审查 B 掩蔽指正): 串非空 + valid=false → State/Indicators 的
    /// update 照常执行后 flag=false — 对象保留, **不是** ServiceData 回 default
    /// (真实断连时快照从不整体复位)
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
        // flags 丢失 (Java Service.java:1780 路径): 仅 flag 翻假, 其余保留
        {
            let mut d = data.write().unwrap();
            d.s_state.as_mut().unwrap().flag = false;
            d.s_indic.as_mut().unwrap().flag = false;
        }
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

    /// B1 补偿核心: 串空路径 (游戏进程退出/8111 消失 → HTTP 失败串复位空 →
    /// update 不执行 → flags/playerLive 保留**陈旧真值**) — 事件流静默超时
    /// 触发 S4toS1; 且退出后状态稳定停 Init (串空轮不跑 initStatusBar/
    /// changeS2, 对位 Java 串空分支)
    #[test]
    fn drive_from_live_silent_stream_exits_on_game_exit() {
        let mut shell = fixture();
        {
            let c = shell.controller.as_mut().unwrap();
            c.init_status_bar();
            c.change_s2();
        }
        let data = Arc::new(std::sync::RwLock::new(live_service_data("p1")));
        *shell.shared.live.write().unwrap() = Some(Arc::clone(&data));
        // 首轮事件新鲜 (100ms 前): 正常进 Preview + identify
        shell
            .shared
            .last_flight_event_ms
            .store(current_time_millis() - 100, Ordering::SeqCst);
        shell.controller.as_mut().unwrap().drive_from_live();
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Preview
        );
        // 游戏退出: ServiceData 一字不动 (flags/playerLive 陈旧真), 仅事件停发
        shell.shared.last_flight_event_ms.store(
            current_time_millis() - FLIGHT_SILENT_EXIT_MS - 100,
            Ordering::SeqCst,
        );
        shell.controller.as_mut().unwrap().drive_from_live();
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Init,
            "事件静默超时应触发 S4toS1 (串空路径补偿, B1)"
        );
        assert_eq!(shell.fm.current_target_name(), None, "会话结束清识别目标");
        assert!(shell
            .shared
            .flags
            .lock()
            .unwrap()
            .session_aircraft_type
            .is_none());
        // 再一轮 (仍静默): 稳定停 Init, 不回弹 Connected/InGame
        shell.controller.as_mut().unwrap().drive_from_live();
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Init
        );
    }

    /// 静默不误杀: playerLive=false (着陆停机/坠机等待, Java Service.java:746-754
    /// 的 sleep 等待路径) + 事件停发 — 补偿判定含 playerLive 真前置, 不触发
    #[test]
    fn drive_from_live_silent_player_not_live_waits() {
        let mut shell = fixture();
        {
            let c = shell.controller.as_mut().unwrap();
            c.init_status_bar();
            c.change_s2();
        }
        let data = Arc::new(std::sync::RwLock::new(live_service_data("p1")));
        *shell.shared.live.write().unwrap() = Some(Arc::clone(&data));
        shell
            .shared
            .last_flight_event_ms
            .store(current_time_millis() - 100, Ordering::SeqCst);
        shell.controller.as_mut().unwrap().drive_from_live();
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Preview
        );
        // 坠机/停机: playerLive 翻假 + 事件停发超阈值
        data.write().unwrap().player_live = false;
        shell.shared.last_flight_event_ms.store(
            current_time_millis() - FLIGHT_SILENT_EXIT_MS - 100,
            Ordering::SeqCst,
        );
        shell.controller.as_mut().unwrap().drive_from_live();
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Preview,
            "playerLive=false 的静默是等待态, 不应退出会话"
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

    /// FlightLog 开/存生命周期 (Controller.java:366-376 openpad / 402-411 closepad):
    /// enableLogging=true → 进 Preview (openpad_rest) 槽置 Some + records/ 建
    /// 机型命名 CSV; s4to_s1 → closepad 保存 (槽清 None, 表头落盘)。
    /// tick 行数语义由 vm-data 的 flight_log_tick 测试锁定 (此处 Service 真线程
    /// HTTP 失败, 行数时序不定, 只断言 ≥ 表头)。
    #[test]
    fn flight_log_open_close_lifecycle() {
        // CWD 沙箱: FlightLog 的 records/ 是相对 CWD 的硬编码 (与 Java 一致);
        // 串行化 + 用完恢复 (vm-core/vm-data 同款; 跨 crate 测试进程天然互斥)
        static FL_CWD_LOCK: Mutex<()> = Mutex::new(());
        let _guard = FL_CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let root = std::env::temp_dir().join(format!("vm_app_fl_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("records")).unwrap();
        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // enableLogging=true 配置 (fixture 其余键同 test_cfg)
            let cfg = fixture_cfg(
                "(panel \"T\" :visible true\n\
                 \x20 (item \"log\" :type switch :target \"enableLogging\" :value true)\n\
                 \x20 (item \"auto\" :type switch :target \"autoStartGameMode\" :value false))\n\
                ",
            );
            let mut shell = fixture_full(30, cfg);
            // 进游戏: UI_READY → StartGame (spawn Service) → live → Preview
            publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
            pump_events(&mut shell);
            shell.dispatch(UiCommand::StartGame);
            // 生产语义: shared.live 即 handle.data 的同一 Arc (start() 内 clone);
            // fixture 手塞场景下需同步塞真 Service 侧 (open_flight_log 的机型名
            // 与 ServiceAnalyzerSource 都读 handle.data)
            let svc_data =
                Arc::clone(&shell.controller.as_ref().unwrap().service.as_ref().unwrap().data);
            *shell.shared.live.write().unwrap() = Some(Arc::clone(&svc_data));
            *svc_data.write().unwrap() = live_service_data("s1");
            shell.pump();
            assert_eq!(
                shell.controller.as_ref().unwrap().state(),
                ControllerState::Preview
            );

            // openpad_rest → open_flight_log: 槽 Some, 主 CSV 已建 (表头 flush)
            let file_name = {
                let c = shell.controller.as_ref().unwrap();
                let slot = c.flight_log.lock().expect("flight_log 槽锁中毒");
                let log = slot.as_ref().expect("openpad 应建 FlightLog").lock().unwrap();
                log.file_name.clone()
            };
            assert!(
                file_name.starts_with("records/S1_"),
                "机型大写命名: {file_name}"
            );
            let content = std::fs::read_to_string(&file_name).unwrap();
            assert!(
                content.lines().count() >= 1 && content.starts_with("时间/s,"),
                "init 写表头: {content:?}"
            );

            // 换机: 关旧开新 (onAircraftChanged:320-333) — 新文件名随新机型
            *svc_data.write().unwrap() = live_service_data("b2");
            let c = shell.controller.as_mut().unwrap();
            c.on_aircraft_changed(Some("b2"));
            {
                let slot = c.flight_log.lock().expect("flight_log 槽锁中毒");
                let log = slot.as_ref().expect("换机应开新 FlightLog").lock().unwrap();
                assert!(
                    log.file_name.starts_with("records/B2_"),
                    "换机开新: {}",
                    log.file_name
                );
            }

            // 退出 (S4toS1 → closepad): 保存 + 槽清 None (Log = null)
            c.s4to_s1();
            assert_eq!(c.state(), ControllerState::Init);
            assert!(
                c.flight_log.lock().unwrap().is_none(),
                "closepad 后槽应清空 (Log = null)"
            );
            // 旧/新两份主 CSV 均留存 (fA==null 的 close NPE 路径由 catch_unwind 吞,
            // 不中断 closepad — Java 由 Service 顶层 catch 兜住的净效果)。
            // 主 CSV 与分析 CSV 以 _climb/_roll/_ny 后缀区分
            let is_main = |n: &str| {
                n.ends_with(".csv")
                    && !n.ends_with("_climb.csv")
                    && !n.ends_with("_roll.csv")
                    && !n.ends_with("_ny.csv")
            };
            let mut s1_seen = false;
            let mut b2_seen = false;
            for e in std::fs::read_dir("records").unwrap() {
                let name = e.unwrap().file_name().to_string_lossy().to_string();
                s1_seen |= name.starts_with("S1_") && is_main(&name);
                b2_seen |= name.starts_with("B2_") && is_main(&name);
            }
            assert!(s1_seen, "s1 主 CSV 留存");
            assert!(b2_seen, "b2 主 CSV 留存");

            // 停 Service 线程 (恢复 CWD / 删沙箱前必须 — 免线程写已删目录)
            shell.controller.as_mut().unwrap().stop(&mut || {});
        }));
        std::env::set_current_dir(old).unwrap();
        let _ = std::fs::remove_dir_all(&root);
        drop(_guard); // 先放锁再重抛
        if let Err(e) = r {
            std::panic::resume_unwind(e);
        }
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

        // 步③注入: 游戏模式下 M 已被 start() 释放 (Java start:619-623 dispose M),
        // stop 的步③ (M!=null 判定) 不触发 — 断言不调用 (步③的触发面见
        // stop_preview_releases_main_form_after_overlays_closed)
        let step3_seen = Arc::new(Mutex::new(false));
        let seen = Arc::clone(&step3_seen);
        let mut release = move || {
            *seen.lock().unwrap() = true;
        };
        shell.controller.as_mut().unwrap().stop(&mut release);

        // ①: 世代号 ++ (作废在途回调) — Preview 分支
        assert_eq!(
            shell.shared.preview_generation.load(Ordering::SeqCst),
            gen_before + 1
        );
        // ①: CloseAllOverlays 命令已入队 (容忍 openpad 延迟线程 OpenAll 抢先)
        let ui_rx = shell.ui_cmd_rx.take().unwrap();
        let mut got_close = false;
        for _ in 0..3 {
            match ui_rx.try_recv() {
                Ok(UiCommand::CloseAllOverlays) => {
                    got_close = true;
                    break;
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        assert!(got_close, "步①应发 CloseAllOverlays (closepad 路径)");
        // ②: 订阅全部退订
        assert_eq!(shell.ui_bus.subscriber_count(), ui_subs_before - 2);
        assert_eq!(
            shell.fm.fm_changed_bus().subscriber_count(),
            fm_subs_before - 1
        );
        // ③: 游戏模式 M 已在 start() 释放 — 步③跳过 (Java M=null 判定)
        assert!(
            !*step3_seen.lock().unwrap(),
            "游戏模式 stop 不应再释放 MainForm"
        );
        // ④: Service 句柄已收 + live 清空
        assert!(shell.controller.as_ref().unwrap().service.is_none());
        assert!(shell.shared.live.read().unwrap().is_none());
        // ⑤: save_config 空实现 (全量在 ui_layout.cfg), 无可断言面 — 顺序由代码序保证
    }

    /// 步①→③ 顺序 (预览模式, M 存活): 步③执行时步①的 CloseAllOverlays
    /// 已入队 (Java: "必须在 dispose MainForm 之前执行" 注释的顺序语义)
    #[test]
    fn stop_preview_releases_main_form_after_overlays_closed() {
        let mut shell = fixture();
        // 进 Preview (M 存活, 无 Service — 纯预览路径)
        publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
        pump_events(&mut shell);
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Preview
        );

        let ui_rx = shell.ui_cmd_rx.take().unwrap();
        let order_ok = Arc::new(Mutex::new(false));
        let ok = Arc::clone(&order_ok);
        let mut release = move || {
            // 步③执行点: 步①的 CloseAll 应已先入队 (容忍 Preview-Refresh 线程的
            // RefreshPreviews 抢先)
            let mut got_close = false;
            for _ in 0..3 {
                match ui_rx.recv_timeout(Duration::from_millis(500)) {
                    Ok(UiCommand::CloseAllOverlays) => {
                        got_close = true;
                        break;
                    }
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
            *ok.lock().unwrap() = got_close;
        };
        shell.controller.as_mut().unwrap().stop(&mut release);
        assert!(
            *order_ok.lock().unwrap(),
            "步③执行时步①的 CloseAllOverlays 应已入队 (overlay 先于 MainForm)"
        );
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
        drop(tx); // shutdown 前 drop 全部发送端克隆, 否则 join 等 Disconnected 永阻塞
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
        {
            let tx = deb.sender();
            tx.send(DebounceMsg::FmChanged).unwrap();
        } // 块尾 drop 发送端克隆
        match out_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(UiCommand::RefreshPreviews { changed_key: None, .. }) => {}
            other => panic!("FmChanged 应产全量刷新: {:?}", other),
        }
        {
            let tx = deb.sender();
            tx.send(DebounceMsg::ConfigKey(
                ui_state_events::ACTION_RESET_COMPLETED.to_string(),
            ))
            .unwrap();
        }
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
        // 防抖产出直达 win32 命令通道 — 接收端留在 shell (未 spawn win32)。
        // 容忍 UI_READY→preview() 的 Preview-Refresh 线程全量刷新 (None) 抢先入队
        let mut keyed = None;
        let mut generation_seen = 0u64;
        for _ in 0..4 {
            match shell
                .ui_cmd_rx
                .as_ref()
                .unwrap()
                .recv_timeout(Duration::from_millis(500))
            {
                Ok(UiCommand::RefreshPreviews {
                    changed_key: Some(k),
                    generation,
                }) => {
                    keyed = Some(k);
                    generation_seen = generation;
                    break;
                }
                Ok(_) => continue, // 全量刷新 (Preview-Refresh/FmChanged) — 竞态容忍
                Err(e) => panic!("防抖后应有键控刷新: {:?}", e),
            }
        }
        assert_eq!(keyed.as_deref(), Some("showSpeedBar"), "最后一条变更生效");
        assert_eq!(
            generation_seen,
            shell.shared.preview_generation.load(Ordering::SeqCst)
        );
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
            .expect("fontNum 触发五色直送 + ReinitActiveOverlays");
        // 五色键: 先 SetGlobalColors (fixture cfg 五色全白 #FFFFFFFF), 后 Reinit
        let all_white = GlobalColors {
            num: [255, 255, 255, 255],
            label: [255, 255, 255, 255],
            unit: [255, 255, 255, 255],
            warning: [255, 255, 255, 255],
            shade_shape: [255, 255, 255, 255],
        };
        assert_eq!(cmd, UiCommand::SetGlobalColors(all_white));
        // WYSIWYG reinit 参数直送 (ReinitActiveOverlays 前置): 参数包随命令进
        // win32 线程 (配置 !Send — 五色直送同款模式)
        let cmd_rp = shell
            .ui_cmd_rx
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_millis(200))
            .expect("ReinitActiveOverlays 前应先到 ReinitOverlays");
        match cmd_rp {
            UiCommand::ReinitOverlays { params } => {
                // fixture cfg 无地平仪组 → Java reinitConfig 缺省 150×300
                assert_eq!(params.attitude_width, 150);
            }
            other => panic!("应是 ReinitOverlays: {:?}", other),
        }
        let cmd2 = shell
            .ui_cmd_rx
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_millis(200))
            .expect("非 Preview 态应立即 ReinitActiveOverlays");
        assert_eq!(cmd2, UiCommand::ReinitActiveOverlays);
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

    /// CONFIG_CHANGED 的 reinit 参数直送: 配置写入值即时进 ReinitOverlays 参数包
    /// (写 "地平仪" 组的 attitudeIndicatorWidth 行 → attitude_width)
    #[test]
    fn config_change_reinit_params_carry_written_config() {
        // cfg 需含目标行 (Java setConfig 只改既有行, 无行即 no-op)
        let cfg = fixture_cfg(
            "(panel \"地平仪\" :visible true\n\
             \x20 (item \"宽\" :type slider :target \"attitudeIndicatorWidth\" :value 150))\n",
        );
        let mut shell = fixture_full(30, cfg);
        // 发布方 (渲染器) 已写配置树 — 本测试直接经 set_config 模拟写点
        shell
            .controller
            .as_ref()
            .unwrap()
            .config
            .set_config("attitudeIndicatorWidth", "222");
        publish_ui_event(
            &shell.ui_bus,
            ui_state_events::CONFIG_CHANGED,
            "attitudeIndicatorWidth",
        );
        assert!(pump_events(&mut shell));
        let mut saw_reinit = None;
        for _ in 0..4 {
            match shell
                .ui_cmd_rx
                .as_ref()
                .unwrap()
                .recv_timeout(Duration::from_millis(200))
            {
                Ok(UiCommand::ReinitOverlays { params }) => {
                    saw_reinit = Some(params);
                    break;
                }
                Ok(_) => continue,
                Err(e) => panic!("应有 ReinitOverlays: {:?}", e),
            }
        }
        let params = saw_reinit.expect("ReinitOverlays 应到达");
        // 写值即时进参数包 (初值 150, 写 222 — 证明非 spawn 期冻结快照)
        assert_eq!(params.attitude_width, 222, "写值应即时进参数包");
    }

    /// Preview 态: ReinitOverlays 先于防抖的 RefreshPreviews 入队
    /// (win32 消费序 = 参数先刷新, 再跑各 overlay reinit — Java refreshPreviews →
    /// reinitConfig 读即时配置的时序等价)
    #[test]
    fn preview_reinit_params_precede_debounced_refresh() {
        let mut shell = fixture_with_debounce(30);
        publish_ui_event(&shell.ui_bus, ui_state_events::UI_READY, "");
        pump_events(&mut shell);
        publish_ui_event(&shell.ui_bus, ui_state_events::CONFIG_CHANGED, "showSpeedBar");
        assert!(pump_events(&mut shell));
        let mut reinit_at: Option<usize> = None;
        let mut refresh_at: Option<usize> = None;
        for i in 0..6 {
            match shell
                .ui_cmd_rx
                .as_ref()
                .unwrap()
                .recv_timeout(Duration::from_millis(400))
            {
                Ok(UiCommand::ReinitOverlays { .. }) => reinit_at = Some(i),
                Ok(UiCommand::RefreshPreviews { changed_key: Some(k), .. }) if k == "showSpeedBar" => {
                    refresh_at = Some(i)
                }
                Ok(_) => {}
                Err(_) => break,
            }
            if reinit_at.is_some() && refresh_at.is_some() {
                break;
            }
        }
        let (r, f) = (reinit_at.expect("ReinitOverlays 应到达"), refresh_at.expect("键控 RefreshPreviews 应到达"));
        assert!(r < f, "参数直送 ({}) 应先于防抖刷新 ({})", r, f);
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

    /// A-W1: 托盘重建复位 — sessionAircraftType / overlay_ctx_preview /
    /// last_flight_event_ms 随新核归零 (Java 均为 Controller 实例字段,
    /// 每次重建随新构造器归默认; Rust 跨核共享故需显式复位)
    #[test]
    fn reset_for_rebuild_clears_session_state() {
        let shared = ControllerShared::new();
        shared.flags.lock().unwrap().session_aircraft_type = Some("old-plane".into());
        shared.overlay_ctx_preview.store(false, Ordering::SeqCst);
        shared.last_flight_event_ms.store(12345, Ordering::SeqCst);
        *shared.state.write().unwrap() = ControllerState::Preview;
        shared.reset_for_rebuild();
        assert_eq!(*shared.state.read().unwrap(), ControllerState::Init);
        assert!(shared.flags.lock().unwrap().session_aircraft_type.is_none());
        assert!(shared.overlay_ctx_preview.load(Ordering::SeqCst));
        assert_eq!(shared.last_flight_event_ms.load(Ordering::SeqCst), 0);
    }

    /// A-W2: EndGame = 保存 + 退出请求 (Java mCancel 的 System.exit(0) 归属)
    #[test]
    fn end_game_requests_exit() {
        let mut shell = fixture();
        assert!(!shell.is_exit_requested());
        shell.dispatch(UiCommand::EndGame);
        assert!(shell.is_exit_requested(), "EndGame 应置退出请求");
    }

    /// A-W6: 托盘 Start 与 MainForm 确认叠加 — start() 对"Service 已在跑"幂等:
    /// 不二次 spawn (live 句柄不换新), 首次会话不中断
    #[test]
    fn start_guard_prevents_double_spawn_on_tray_start_overlay() {
        let mut shell = fixture();
        shell.handle_main_event(MainEvent::Tray(TrayCommand::Start));
        assert!(
            shell.controller.as_ref().unwrap().service.is_some(),
            "托盘 Start 应起 Service"
        );
        let live1 = shell.shared.live.read().unwrap().clone();
        // 用户再点 MainForm "开始游戏" (叠加态): confirm 链被 service.is_some() 守卫拦下
        shell.dispatch(UiCommand::StartGame);
        let live2 = shell.shared.live.read().unwrap().clone();
        assert!(live2.is_some(), "首次 Service 应存活");
        assert!(
            Arc::ptr_eq(&live1.unwrap(), &live2.unwrap()),
            "叠加 start 不应重建 Service (live 句柄不变)"
        );
    }

    /// B-W5: change_s3 延迟开面板的停止守卫 — 100ms 窗口内游戏退出 (S4toS1 →
    /// Init) 则 OpenAllOverlays 被丢弃 (Java 无守卫; Rust 侧重开残留形态更重,
    /// 显式改进偏离, 见 change_s3 的 PORT 注)
    #[test]
    fn change_s3_openpad_delay_guarded_on_exit() {
        let mut shell = fixture();
        {
            let c = shell.controller.as_mut().unwrap();
            c.init_status_bar();
            c.change_s2();
        }
        let data = Arc::new(std::sync::RwLock::new(live_service_data("g1")));
        *shell.shared.live.write().unwrap() = Some(data);
        shell
            .shared
            .last_flight_event_ms
            .store(current_time_millis(), Ordering::SeqCst);
        shell.controller.as_mut().unwrap().drive_from_live(); // changeS3 → 延迟线程
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Preview
        );
        // 100ms 窗口内游戏退出 (静默) → S4toS1 → state=Init
        shell.shared.last_flight_event_ms.store(
            current_time_millis() - FLIGHT_SILENT_EXIT_MS - 1,
            Ordering::SeqCst,
        );
        shell.controller.as_mut().unwrap().drive_from_live();
        assert_eq!(shell.controller.as_ref().unwrap().state(), ControllerState::Init);
        std::thread::sleep(Duration::from_millis(250)); // 越过延迟窗口
        // 通道内可有 CloseAllOverlays (s4to_s1 的 closepad), 但不应有 OpenAllOverlays
        let ui_rx = shell.ui_cmd_rx.take().unwrap();
        loop {
            match ui_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(UiCommand::OpenAllOverlays) => {
                    panic!("退出后的 OpenAllOverlays 应被守卫丢弃")
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }
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
    // 组装接线 (本批新增面)
    // ------------------------------------------------------------------

    /// autoStartGameMode=true → Controller 自启动分支 (跳过 MainForm, Service 直起;
    /// --game-mode/--mock-smoke 的配置注入对位, Controller.java:589-606)
    #[test]
    fn auto_start_game_mode_skips_main_form() {
        let shell = fixture_full(30, auto_start_cfg());
        let c = shell.controller.as_ref().unwrap();
        assert!(!c.main_form_alive, "自启动路径 M 恒 null (Java:604-606)");
        assert!(c.service.is_some(), "自启动应直起 Service 线程");
    }

    /// 托盘 Activate 置设置窗请求位 (一次性取走; 组装层主循环的相位切换信号)
    #[test]
    fn tray_activate_requests_main_form() {
        let mut shell = fixture();
        shell.handle_main_event(MainEvent::Tray(TrayCommand::Activate));
        assert!(shell.take_form_request(), "托盘 Activate 应置请求位");
        assert!(!shell.take_form_request(), "取走后复位 (一次性)");
        assert_eq!(
            shell.controller.as_ref().unwrap().state(),
            ControllerState::Init,
            "Activate 同时重建核 (INIT)"
        );
    }

    /// 分相监督循环: 退出请求 → Exit (EndGame/托盘 Exit 的相位出口;
    /// 阻断自动 spawn win32 — 纯状态机断言, 不开真窗)
    #[test]
    fn supervisor_phase_returns_exit_on_request() {
        let mut shell = fixture();
        shell.ui_cmd_rx.take();
        shell.handle_main_event(MainEvent::Tray(TrayCommand::Exit));
        assert_eq!(shell.run_supervisor_phase(), SupervisorOutcome::Exit);
    }

    /// 分相监督循环: 托盘 Activate → MainFormRequested (重建核 + 请求开窗)
    #[test]
    fn supervisor_phase_returns_form_request_on_tray_activate() {
        let mut shell = fixture();
        shell.ui_cmd_rx.take();
        shell.handle_main_event(MainEvent::Tray(TrayCommand::Activate));
        assert_eq!(
            shell.run_supervisor_phase(),
            SupervisorOutcome::MainFormRequested
        );
    }

    /// win32 渲染帧计数: 预览刷新打开 overlay 后 present 帧数递增
    /// (--mock-smoke 核心断言的库内等价; 字体目录钉仓库根, 见 fixture 注)
    #[test]
    fn win32_render_frames_advance_with_active_overlays() {
        let mut shell = fixture();
        shell.spawn_win32_thread().expect("win32 线程启动");
        // Preview 态 + 有效世代号 → 全量刷新 (MiniHUD crosshairSwitch=true 激活)
        *shell.shared.state.write().unwrap() = ControllerState::Preview;
        let gen = shell.shared.preview_generation.load(Ordering::SeqCst);
        shell
            .ui_cmd_tx
            .send(UiCommand::RefreshPreviews {
                changed_key: None,
                generation: gen,
            })
            .unwrap();
        // 泵消费 (10ms 节拍) + 窗口物化 + 至少数个 50ms 渲染节拍
        std::thread::sleep(Duration::from_millis(800));
        let frames = shell.shared.render_frames.load(Ordering::SeqCst);
        shell.ui_cmd_tx.send(UiCommand::Shutdown).unwrap();
        let join = shell.win32.take().unwrap();
        assert!(join.join().is_ok());
        assert!(
            frames > 0,
            "活跃 overlay 的 present 帧数应递增 (实测 {frames})"
        );
    }

    /// 逐 overlay present 计数: 游戏模式全开 (open_all) 后 6 注册键全部 present>0
    /// (--mock-smoke 断言 3 的库内等价; 字体目录钉仓库根, 见 fixture 注)
    #[test]
    fn win32_overlay_present_counts_per_registered_overlay() {
        let all_on_cfg = fixture_cfg(
            "(panel \"T\" :visible true\n\
             \x20 (item \"a\" :type switch :target \"crosshairSwitch\" :value true)\n\
             \x20 (item \"b\" :type switch :target \"engineInfoSwitch\" :value true)\n\
             \x20 (item \"c\" :type switch :target \"enableEngineControl\" :value true)\n\
             \x20 (item \"d\" :type switch :target \"enablegearAndFlaps\" :value true)\n\
             \x20 (item \"e\" :type switch :target \"enableAxis\" :value true)\n\
             \x20 (item \"f\" :type switch :target \"enableAttitudeIndicator\" :value true))\n\
            ",
        );
        let mut shell = fixture_full(30, all_on_cfg);
        shell.spawn_win32_thread().expect("win32 线程启动");
        shell.ui_cmd_tx.send(UiCommand::OpenAllOverlays).unwrap();
        // 泵消费 (10ms 节拍) + 6 窗物化 + 至少数个 50ms 渲染节拍
        std::thread::sleep(Duration::from_millis(1200));
        let counts = shell
            .shared
            .overlay_present
            .lock()
            .expect("overlay_present 锁中毒")
            .clone();
        shell.ui_cmd_tx.send(UiCommand::Shutdown).unwrap();
        let join = shell.win32.take().unwrap();
        assert!(join.join().is_ok());
        for id in [
            "enableEngineControl",
            "engineInfoSwitch",
            "crosshairSwitch",
            "enablegearAndFlaps",
            "enableAxis",
            "enableAttitudeIndicator",
        ] {
            let c = counts.get(id).copied().unwrap_or(0);
            assert!(c > 0, "overlay {id} present 应 >0 (实测 {c}, 全量 {counts:?})");
        }
    }

    /// RefreshPreviews 不翻转会话窗口形态 (审查 blocker 回归锚): 游戏稳态
    /// (openpad 后 overlay_ctx_preview=false, State=Preview) 下 FM_CHANGED/
    /// ConfigChanged 防抖必发 RefreshPreviews — 原实现置 true 后生产路径无复位点,
    /// feed_overlays_live 整帧 early-return, 6 窗口数据冻结; 修复 = 激活探测期
    /// 临时置位 (Java refreshPreviews 的 forPreviewMode ctx), 完毕恢复会话形态。
    /// CloseAllOverlays 复位 true (会话结束回预览态)。
    ///
    /// 断言用轮询终态: refresh_preview 的激活探测期内物化真窗口 (Win32
    /// CreateWindow 冷启动可达百 ms 级), 标志短暂为 true 是修复的合法瞬态 —
    /// 轮询穿过后核对终值; 原 bug 形态下永久 true, 轮询超时必失败。
    #[test]
    fn refresh_previews_keep_session_window_mode() {
        fn wait_flag(flag: &AtomicBool, want: bool) -> bool {
            let start = Instant::now();
            while flag.load(Ordering::SeqCst) != want {
                if start.elapsed() > Duration::from_millis(2000) {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            true
        }
        let mut shell = fixture();
        shell.spawn_win32_thread().expect("win32 线程启动");
        *shell.shared.state.write().unwrap() = ControllerState::Preview;
        let gen = shell.shared.preview_generation.load(Ordering::SeqCst);
        // 模拟游戏形态 (openpad → OpenAllOverlays 处理点置 false)
        shell.shared.overlay_ctx_preview.store(false, Ordering::SeqCst);
        shell
            .ui_cmd_tx
            .send(UiCommand::RefreshPreviews {
                changed_key: None,
                generation: gen,
            })
            .unwrap();
        assert!(
            wait_flag(&shell.shared.overlay_ctx_preview, false),
            "游戏稳态刷新预览后 live 门控应恢复开启 (原 bug: 置 true 后 live 冻结)"
        );
        // CloseAll (stop/end_preview/S4toS1 的公共出口) → 回预览态
        shell.ui_cmd_tx.send(UiCommand::CloseAllOverlays).unwrap();
        assert!(
            wait_flag(&shell.shared.overlay_ctx_preview, true),
            "CloseAll 后窗口形态应回预览态"
        );
        shell.ui_cmd_tx.send(UiCommand::Shutdown).unwrap();
        let join = shell.win32.take().unwrap();
        assert!(join.join().is_ok());
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

    // ------------------------------------------------------------------
    // P6 收口: 注册面 6 窗口条目 / live 喂数全链 / 注册键 ↔ ui_layout.cfg 核对
    // ------------------------------------------------------------------

    /// 测试用空窗口 (field2 测试 MiniWin 同款; 免真窗依赖)
    struct NullWin;

    impl vm_overlay::platform::OverlayWindow for NullWin {
        fn present(&mut self, _buf: &[u8]) -> Result<(), String> {
            Ok(())
        }
        fn set_position(&mut self, _x: i32, _y: i32) {}
        fn position(&self) -> (i32, i32) {
            (0, 0)
        }
        fn set_click_through(&mut self, _on: bool) {}
        fn poll_event(&mut self) -> Option<vm_overlay::platform::OverlayEvent> {
            None
        }
        fn screen_size(&self) -> (i32, i32) {
            (1920, 1080)
        }
    }

    fn test_overlay_inputs() -> OverlayInputs {
        OverlayInputs {
            dpi_scale: 1.0,
            hud: HudSettingsSnapshot::build(
                &fixture().controller.as_ref().unwrap().config.get_hud_settings(),
            ),
            font_add_engine: 0,
            font_add_power: 0,
            power_columns: 1,
            font_add_flight: 0,
            flight_columns: 1,
            font_add_gear: 0,
            gear_show_edge: false,
            font_add_axis: 0,
            axis_show_edge: false,
            attitude_width: 150,
            attitude_height: 300,
            attitude_freq_ms: 40,
            attitude_show_direction: false,
            attitude_show_aoa_limits: true,
            service_loop_interval_ms: 50,
            colors: GlobalColors::JAVA_DEFAULT,
            aa: true,
            engine_disables: [false; 7],
        }
    }

    /// 注册面: Java registerGameModeOverlays 的 7 个窗口条目全部落位
    /// (open_all 默认激活全真; 剩 3 键非窗口/降级备案见 register_game_mode_overlays 头注)
    #[test]
    fn register_game_mode_overlays_seven_window_entries() {
        let mut host = OverlayHost::with_factory(Box::new(|_cfg| {
            Ok(Box::new(NullWin) as Box<dyn vm_overlay::platform::OverlayWindow>)
        }));
        let mut handles = OverlayHandles {
            minihud: None,
            power_info: None,
            engine_control: None,
            gear_flaps: None,
            attitude: None,
            control_surfaces: None,
        flight_info: None,
        };
        let shell = fixture();
        let lang = Rc::new(Lang::init_lang());
        let params = Rc::new(RefCell::new(vm_overlay::ReinitParams::from(
            &test_overlay_inputs(),
        )));
        register_game_mode_overlays(
            &mut host,
            &mut handles,
            &shell.env,
            &test_overlay_inputs(),
            &params,
            &lang,
            &shell.shared,
        );
        // 注册面逐窗计数落键: 6 键全部以 0 落位 (present 计数起点)
        let reg_keys: Vec<String> = shell
            .shared
            .overlay_present
            .lock()
            .expect("overlay_present 锁中毒")
            .keys()
            .cloned()
            .collect();
        assert_eq!(reg_keys.len(), 7, "注册落键应恰为 7 键 (实测 {reg_keys:?})");
        // 7 个共享句柄全部登记 (spec 工厂成功)
        assert!(handles.minihud.is_some(), "MiniHUD 句柄");
        assert!(handles.power_info.is_some(), "动力信息句柄");
        assert!(handles.engine_control.is_some(), "引擎控制句柄");
        assert!(handles.flight_info.is_some(), "飞行信息句柄");
        assert!(handles.gear_flaps.is_some(), "起落襟翼句柄");
        assert!(handles.attitude.is_some(), "地平仪句柄");
        assert!(handles.control_surfaces.is_some(), "操纵面句柄");
        host.open_all().expect("全激活 open_all");
        let mut ids: Vec<String> = host.active_ids();
        ids.sort();
        assert_eq!(
            ids,
            vec![
                "crosshairSwitch",
                "enableAttitudeIndicator",
                "enableAxis",
                "enableEngineControl",
                "enablegearAndFlaps",
                "engineInfoSwitch",
                "flightInfoSwitch",
            ],
            "注册键 10 键中的 7 窗口条目 (Java 键一一对应)"
        );
    }

    /// live 喂数全链: 一帧 payload 喂 6 个 overlay, 各 state 推进到遥测值
    /// (ServiceData 的引擎数组必须非空 — get_pitch/get_thrust 的保真 panic 点,
    /// 真实链路由 State.update 填满; catch_unwind 吞帧路径由 malformed 变体覆盖)
    #[test]
    fn feed_overlays_live_updates_all_handles() {
        let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts");
        let lang = Rc::new(Lang::init_lang());
        let inputs = test_overlay_inputs();
        let (h_mini, _) = vm_overlay::minihud_overlay_spec(
            false,
            50,
            &inputs.hud,
            1.0,
            &fonts.join("sarasa-mono-sc-bold.ttf"),
            &Rc::new(RefCell::new(vm_overlay::ReinitParams::default())),
        )
        .unwrap();
        let (h_power, _) = vm_overlay::power_info_overlay_spec(
            &fonts,
            &Rc::new(RefCell::new(vm_overlay::ReinitParams::default())),
        )
        .unwrap();
        let (h_engine, _) = vm_overlay::engine_control_overlay_spec(
            &fonts,
            Rc::clone(&lang),
            &Rc::new(RefCell::new(vm_overlay::ReinitParams {
                service_loop_interval_ms: 50,
                ..Default::default()
            })),
        )
        .unwrap();
        let (h_gear, _) = vm_overlay::gear_flaps_overlay_spec(
            &fonts,
            &Rc::new(RefCell::new(vm_overlay::ReinitParams::default())),
        )
        .unwrap();
        let (h_att, _) = vm_overlay::attitude_overlay_spec(&Rc::new(RefCell::new(
            vm_overlay::ReinitParams::default(),
        )))
        .unwrap();
        let (h_cs, _) = vm_overlay::control_surfaces_overlay_spec(
            &fonts,
            &Rc::new(RefCell::new(vm_overlay::ReinitParams::default())),
        )
        .unwrap();
        let (h_fi, _) = vm_overlay::flight_info_overlay_spec(
            &fonts,
            &Rc::new(RefCell::new(vm_overlay::ReinitParams::default())),
        )
        .unwrap();
        let handles = OverlayHandles {
            minihud: Some(h_mini),
            power_info: Some(h_power),
            engine_control: Some(h_engine),
            gear_flaps: Some(h_gear),
            attitude: Some(h_att),
            control_surfaces: Some(h_cs),
            flight_info: Some(h_fi),
        };

        // live 快照: throttle 55 / flaps 25 / gear 100 / aileron 100 / aoa 10 /
        // aviahorizon pitch 5 / 功率 1200 / airbrake 100 (引擎数组填满防 panic 点)
        let mut d = live_service_data("feed-plane");
        {
            let st = d.s_state.as_mut().unwrap();
            st.throttle = 55;
            st.flaps = 25;
            st.gear = 100;
            st.airbrake = 100;
            st.aileron = 100;
            st.aoa = 10.0;
            st.pitch = vec![0.0; 8];
            st.thrust = vec![0; 8];
            st.power = vec![0.0; 8];
            st.efficiency = vec![0.0; 8];
            st.throttles = vec![0; 8];
            st.rpm_throttle = 60;
        }
        d.s_indic.as_mut().unwrap().aviahorizon_pitch = 5.0;
        d.total_hp = 1200;
        let shared = ControllerShared::new();
        shared.overlay_ctx_preview.store(false, Ordering::SeqCst); // 游戏窗口形态
        *shared.live.write().unwrap() = Some(Arc::new(std::sync::RwLock::new(d)));
        let fm = FMManager::new(Arc::new(EventBus::new()));
        let settings = inputs.hud.clone();
        let payload = EventPayload::builder().build();
        let mut attitude_feed = AttitudeFeedState { freq_ms: 40, last_ms: 0 };

        feed_overlays_live(&handles, &payload, &shared, &fm, &settings, &lang, &mut attitude_feed);

        // 动力信息: 功率 1200 → 首字段 buffer (50ms 节流: now-0 恒放行)
        let p = handles.power_info.as_ref().unwrap().borrow();
        assert_eq!(p.fields()[0].buffer, "1200", "PowerInfo 功率字段");
        drop(p);
        // 引擎控制: throttle 55 (refreshInterval=100, 首帧放行)
        let e = handles.engine_control.as_ref().unwrap().borrow();
        assert_eq!(e.gauge_by_key("throttle").unwrap().gauge.gauge.cur_value, 55);
        drop(e);
        // 起落襟翼: gear=100 + airbrake=100 → "起落架 减速板" 告警; flaps=25 → flap_pix
        let g = handles.gear_flaps.as_ref().unwrap().borrow();
        assert_eq!(g.warn_text, "起落架 减速板");
        assert_eq!(g.flap_pix, 24);
        drop(g);
        // 操纵面: aileron=100 → px = (100+100)*144/200 = 144 (has_service 喂入点置位)
        let cs = handles.control_surfaces.as_ref().unwrap().borrow();
        assert_eq!(cs.px, 144);
        drop(cs);
        // 地平仪: aoa=10 → AoA = round((10+30)·300/60) = 200
        let a = handles.attitude.as_ref().unwrap().borrow();
        assert_eq!(a.aoa_y, 200);
        drop(a);
        // 地平仪节流: 40ms 窗口内第二帧不重算 (last_ms 已推进)
        assert!(attitude_feed.last_ms > 0);
        // MiniHUD: 重构事件携带 state 快照 → hud_calculator 读到 sState.airbrake=100
        // → warnVne 置真 (state 丢失时该块整体跳过, warn_vne 恒 false = "bar 恒 0"
        // 根因的回归哨)
        assert!(
            handles.minihud.as_ref().unwrap().borrow().warn_vne,
            "MiniHUD 应收到 sState 快照 (airbrake=100 → warnVne)"
        );

        // preview 门控: overlay_ctx_preview=true → 整帧跳过 (值不推进)
        shared.overlay_ctx_preview.store(true, Ordering::SeqCst);
        {
            let live = shared.live.read().unwrap().clone();
            let mut st = live.as_ref().unwrap().write().unwrap();
            st.s_state.as_mut().unwrap().throttle = 99;
        }
        feed_overlays_live(&handles, &payload, &shared, &fm, &settings, &lang, &mut attitude_feed);
        let e = handles.engine_control.as_ref().unwrap().borrow();
        assert_eq!(
            e.gauge_by_key("throttle").unwrap().gauge.gauge.cur_value,
            55,
            "preview 期不喂入 (Java initPreview 不订阅)"
        );
    }

    /// 畸形 s_state (引擎数组空, update 未跑) 的保真 panic 点: catch_unwind 吞帧不杀线程
    #[test]
    fn feed_overlays_live_swallows_malformed_frame() {
        let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts");
        let lang = Lang::init_lang();
        // 只接 PowerInfo (get_pitch 空 Vec panic 点; 其余 handle 缺省 None)
        let (h_power, _) = vm_overlay::power_info_overlay_spec(
            &fonts,
            &Rc::new(RefCell::new(vm_overlay::ReinitParams::default())),
        )
        .unwrap();
        let handles = OverlayHandles {
            minihud: None,
            power_info: Some(h_power),
            engine_control: None,
            gear_flaps: None,
            attitude: None,
            control_surfaces: None,
        flight_info: None,
        };
        let shared = ControllerShared::new();
        shared.overlay_ctx_preview.store(false, Ordering::SeqCst);
        // State::new() 的 pitch/thrust 空 Vec — get_pitch 的 s.pitch[0] panic (保真)
        *shared.live.write().unwrap() =
            Some(Arc::new(std::sync::RwLock::new(live_service_data("bad"))));
        let fm = FMManager::new(Arc::new(EventBus::new()));
        let settings = test_overlay_inputs().hud;
        let payload = EventPayload::builder().build();
        let mut attitude_feed = AttitudeFeedState { freq_ms: 40, last_ms: 0 };
        // 不 panic 即通过 (吞帧 + ERROR 留痕; Java NPE 由 EDT 吞的同位形态)
        feed_overlays_live(&handles, &payload, &shared, &fm, &settings, &lang, &mut attitude_feed);
    }

    /// 注册键 ↔ ui_layout.cfg 核对: 9 个激活键 (ACTIVATION_KEYS) 全部以 panel switch
    /// 形式存在 (Java 端第 10 键 thrustdFS 无 cfg 项 — 策略读 enableFMPrint,
    /// DrawFrameSimpl 无独立开关, Java 同形态)
    #[test]
    fn activation_keys_match_ui_layout_cfg() {
        let cfg_path =
            locate_template_cfg().expect("仓库模板 ui_layout.cfg 应可达 (上溯三级)");
        let text = std::fs::read_to_string(&cfg_path).unwrap();
        for key in ACTIVATION_KEYS {
            let target = format!(":target \"{}\"", key);
            assert!(
                text.contains(&target),
                "激活键 {key} 应以 :target 开关项存在于 ui_layout.cfg"
            );
        }
        // 6 个窗口条目键 (注册面) 与 cfg 面板一一对应 (10 panel 中 9 有 switch 键,
        // thrustdFS 例外; 欢迎/飞行记录/全局设置 无 overlay 开关)
        for key in [
            "crosshairSwitch",
            "flightInfoSwitch",
            "engineInfoSwitch",
            "enableEngineControl",
            "enableAxis",
            "enablegearAndFlaps",
            "enableAttitudeIndicator",
            "enableFMPrint",
            "enableVoiceWarn",
        ] {
            assert!(text.contains(&format!(":target \"{}\"", key)), "键 {key} 缺失");
        }
    }

    /// 提取 ui_layout.cfg 全部 :target 键值 (兴趣键命中核对的键空间源)
    fn cfg_target_keys(text: &str) -> Vec<String> {
        let mut keys = Vec::new();
        let mut rest = text;
        while let Some(i) = rest.find(":target \"") {
            rest = &rest[i + 9..];
            match rest.find('"') {
                Some(j) => {
                    keys.push(rest[..j].to_string());
                    rest = &rest[j..];
                }
                None => break,
            }
        }
        keys
    }

    /// MiniHUD 兴趣键 ↔ ui_layout.cfg 键空间核对 (审查 W1 回归锚): with_interest
    /// 为前缀匹配 (host is_interested_in), 死键不命中任何 cfg 键 → WYSIWYG 开关
    /// 切换时 MiniHUD 不刷新 (Java 会刷新)。曾笔误 "showAttitudeIndicator"
    /// (正确键 showAttitudeGauge, ui_layout.cfg:63 / Java Controller.java:676)。
    /// 注: "S." (PowerInfo) 为 Java 原样搬移的死前缀, cfg 无此键族 — 不在本测试面。
    #[test]
    fn minihud_interest_keys_hit_ui_layout_cfg() {
        let cfg_path =
            locate_template_cfg().expect("仓库模板 ui_layout.cfg 应可达 (上溯三级)");
        let text = std::fs::read_to_string(&cfg_path).unwrap();
        let keys = cfg_target_keys(&text);
        assert!(!keys.is_empty(), "cfg 键空间非空 (解析自检)");
        for p in MINIHUD_INTEREST_KEYS {
            assert!(
                keys.iter().any(|k| k.starts_with(p)),
                "MiniHUD 兴趣键 {p} 应命中 ui_layout.cfg 的 :target 键 (前缀匹配)"
            );
        }
    }

    /// FocusMonitor 通道桥: coordinator 回调 → UiCommand 命令 + shared 镜像
    #[test]
    fn focus_bridge_sends_commands_and_mirrors_hidden() {
        use vm_core::focus_monitor::AlwaysOnTopCoordinatorApi as _;
        let (tx, rx) = std::sync::mpsc::channel::<UiCommand>();
        let shared = ControllerShared::default();
        let bridge = ChannelFocusBridge { tx, shared: Arc::new(shared) };
        assert!(!bridge.is_overlays_hidden(), "初始未隐藏");
        bridge.hide_all_overlays();
        assert_eq!(rx.recv_timeout(Duration::from_millis(200)).unwrap(), UiCommand::HideAllOverlays);
        bridge.show_all_overlays();
        assert_eq!(rx.recv_timeout(Duration::from_millis(200)).unwrap(), UiCommand::ShowAllOverlays);
    }

    /// 位置映射 ↔ ui_layout.cfg panel 标题核对: OVERLAY_SECTIONS 的 section 查不到
    /// GroupConfig → group_position 返回 None → 该 overlay 恒居中, 位置持久化静默失效
    #[test]
    fn overlay_sections_hit_ui_layout_cfg() {
        let cfg_path =
            locate_template_cfg().expect("仓库模板 ui_layout.cfg 应可达 (上溯三级)");
        let text = std::fs::read_to_string(&cfg_path).unwrap();
        // 顶层 panel 标题集 (行首 `(panel "标题"`; GroupConfig.x/y 挂在顶层标题上)
        let mut titles: Vec<&str> = Vec::new();
        for line in text.lines() {
            let t = line.trim_start();
            if let Some(rest) = t.strip_prefix("(panel \"") {
                if let Some(end) = rest.find('"') {
                    titles.push(&rest[..end]);
                }
            }
        }
        assert!(titles.len() >= 6, "cfg 顶层 panel 数量自检 (实得 {})", titles.len());
        for (id, section) in OVERLAY_SECTIONS {
            assert!(
                titles.contains(&section),
                "overlay {id} 的 section {section} 不在 ui_layout.cfg 顶层 panel 标题中 — \
                 位置读写将永远落空"
            );
        }
    }
}
