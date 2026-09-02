//! win32 线程 (D8: host 泵 + 托盘 + 热键事件消费)。重构波2 自 app_shell.rs 拆出。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use vm_core::activation_strategy::{ActivationContext, ActivationStrategy};
use vm_core::bus::Subscription;
use vm_core::config_api::ConfigProvider;
use vm_core::event::event_payload::EventPayload;
use vm_core::event::flight_data_event::FlightDataEvent;
use vm_core::event::ui_state_events;
use vm_core::flight_data_bus::FlightDataBus;
use vm_core::fm::FMManager;
use vm_core::formula::registry::FormulaView as _; // var_value 取数唯一接口 (W10 后 TelemetrySource 已删)
use vm_core::hud_calculator::HudColors;
use vm_core::lang::Lang;
use vm_core::logger;
use vm_core::ui_state_bus::UIStateBus;
use vm_core::voice_resource_manager::VoiceResourceManager;

use vm_overlay::host::OverlayHost;
use vm_overlay::hotkey::HotkeyEvent;
use vm_overlay::{
    attitude_overlay_spec, control_surfaces_overlay_spec, draw_frame_simpl_spec,
    engine_control_overlay_spec, flight_info_overlay_spec, fm_unpacked_data_overlay_spec,
    gear_flaps_overlay_spec, minihud_overlay_spec, power_info_overlay_spec,
    AttitudeOverlayHandle, ControlSurfacesHandle, DfsFlight, DrawFrameSimplFeed,
    DrawFrameSimplHandle, EngineControlHandle, FlightInfoHandle, FmUnpackedDataHandle,
    FmUnpackedFeed, GearFlapsHandle, MiniHudHandle, PowerInfoHandle,
};

#[cfg(target_os = "windows")]
use vm_overlay::tray::{TrayConfig, TrayIcon, TrayHandler};

use crate::commands::{MainEvent, TrayCommand, UiCommand};
use crate::controller_shared::{is_stale_refresh, ControllerShared};
use crate::env::{current_time_millis, java_parse_boolean, Env};
use crate::keys::{FM_UNPACKED_INTEREST_KEYS, MINIHUD_INTEREST_KEYS, OVERLAY_SECTIONS};
use crate::overlay_inputs::{ActivationCache, OverlayInputs};
use crate::voice_setup::{
    open_voice_warning, voice_warn_refresh_reaches, SnapshotConfigProvider, VoiceWarnSession,
};

/// win32 线程装配输入 (全部 Send; 配置以快照形态入线程, 见模块头)
pub struct Win32ThreadConfig {
    pub env: Env,
    pub inputs: OverlayInputs,
    pub ui_bus: Arc<UIStateBus>,
    pub flight_bus: Arc<FlightDataBus>,
    pub fm: Arc<FMManager>,
    pub shared: Arc<ControllerShared>,
    pub activation: ActivationCache,
    /// 共享语音资源管理器 (AppShell.voice; VoiceWarning 告警线程的 reload 面)
    pub voice: Arc<VoiceResourceManager>,
    /// voice_* 配置键快照 (AppShell.voice_config; 配置 !Send 的跨线程桥)
    pub voice_config: Arc<Mutex<HashMap<String, String>>>,
    /// FM show* 配置键快照 (AppShell.fm_field_config; 同上跨线程桥,
    /// FMUnpackedData generate_lines 的读面)
    pub fm_field_config: Arc<Mutex<HashMap<String, String>>>,
    pub ui_cmd_rx: Receiver<UiCommand>,
    pub hotkey_rx: Receiver<HotkeyEvent>,
    pub main_event_tx: Sender<MainEvent>,
    /// overlay 初始位置快照 (id → 归一化; 主线程 spawn 前从 GroupConfig.x/y 取,
    /// win32 线程不能碰 !Send 配置树 — 见 ChannelPositionStore 头注)
    pub position_snapshot: HashMap<String, (f64, f64)>,
}

/// win32 线程内注册的 overlay 数据句柄 (Rc — 恒留本线程)。
/// None = spec 工厂失败 (字体缺失等, 注册点已 logger::error), 喂入跳过
pub(crate) struct OverlayHandles {
    /// MiniHUD live 喂入口 (Java onFlightData → EDT 的单线程 host 对位)
    pub(crate) minihud: Option<MiniHudHandle>,
    /// 动力信息 (Java PowerInfoOverlay.onFlightData 50ms 节流)
    pub(crate) power_info: Option<PowerInfoHandle>,
    /// 引擎控制 (Java EngineControlOverlay.onFlightData, 间隔配置驱动 ×2)
    pub(crate) engine_control: Option<EngineControlHandle>,
    /// 起落襟翼 (Java GearFlapsOverlay.onFlightData 100ms 节流)
    pub(crate) gear_flaps: Option<GearFlapsHandle>,
    /// 地平仪 (Java AttitudeOverlay.drawTick, freqMili 节流归喂入侧)
    pub(crate) attitude: Option<AttitudeOverlayHandle>,
    /// 操纵面 (Java ControlSurfacesOverlay.onFlightData 50ms 节流)
    pub(crate) control_surfaces: Option<ControlSurfacesHandle>,
    /// 飞行信息 (Java FlightInfoOverlay.onFlightData 字段行; POC 专径收编批接入)
    pub(crate) flight_info: Option<FlightInfoHandle>,
    /// FM拆包数据 (Java FMUnpackedDataOverlay: FM_CHANGED 重载 + 热键切换自管可见;
    /// 无 FlightDataBus 订阅 — 不进 feed_overlays_live, 事件面在 win32 循环驱动)
    pub(crate) fm_unpacked: Option<FmUnpackedDataHandle>,
    /// 推力曲线 (Java DrawFrameSimpl: FM_CHANGED 重载 (两会话) + 热键切换自管可见
    /// (仅游戏); run 循环含 displayFmKey==0 收腿退场 — DrawFrameSimplFeed 驱动)
    pub(crate) draw_frame_simpl: Option<DrawFrameSimplHandle>,
}

/// CloseAllOverlays 时数据面回 preview 静态初值 (win32 命令处理点调用)。
/// 覆盖面 = reinit 闭包只重建几何/资源、不重建数据态的 4 个 overlay:
/// 动力信息 (RenderContext 重载)、飞行信息 (字体/画布重载, rows 保留)、
/// 舵面值 (几何派生)、地平仪 (尺寸/开关)。
/// 不重置: MiniHUD (reinit 刷新 mock 模板 + update_components(None)) /
/// 引擎控制 (build_engine_state 整建) / 起落襟翼 (GearFlapsState::new 整建) —
/// preview 冷激活路径 refresh_preview_idx 先跑 reinit 即自愈。
/// FM拆包数据另加会话形态复位 (reset_preview: 可见/预览态/lastData 清空 — Java
/// closeAll 销毁实例 + 预览工厂新建 initPreview 的形态)。
/// 推力曲线同族 (reset_preview: visible=true / is_preview=true — Java closeAll
/// 销毁 + 预览工厂新建 initPreview 恒可见)。
pub(crate) fn reset_handles_preview_values(handles: &OverlayHandles) {
    if let Some(h) = handles.power_info.as_ref() {
        h.borrow_mut().reset_preview();
    }
    if let Some(h) = handles.flight_info.as_ref() {
        h.borrow_mut().reset_preview_rows();
    }
    if let Some(h) = handles.control_surfaces.as_ref() {
        h.borrow_mut().reset_preview();
    }
    if let Some(h) = handles.attitude.as_ref() {
        h.borrow_mut().reset_preview();
    }
    if let Some(h) = handles.fm_unpacked.as_ref() {
        h.borrow_mut().reset_preview();
    }
    if let Some(h) = handles.draw_frame_simpl.as_ref() {
        h.borrow_mut().reset_preview();
    }
}

/// Java OverlayContext 的 win32 侧替身: 激活探测访问面
/// (get_bool/isDebug/isJet/isPreviewMode/has_blkx — activation_strategy.rs trait 注)
pub(crate) struct HostActivationCtx {
    pub(crate) activation: ActivationCache,
    pub(crate) fm: Arc<FMManager>,
    pub(crate) shared: Arc<ControllerShared>,
    pub(crate) debug: bool,
}

impl ActivationContext for HostActivationCtx {
    fn get_bool(&self, key: &str) -> bool {
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
            .fmdata
            .as_ref()
            .map(|b| b.is_jet)
            .unwrap_or(false)
    }
    fn is_preview_mode(&self) -> bool {
        self.shared.overlay_ctx_preview.load(Ordering::SeqCst)
    }
    fn has_fmdata(&self) -> bool {
        self.fm.current().fmdata.is_some()
    }
}

/// 注册键 → 激活策略 (Java registerWithPreview 默认 config(key);
/// 两处复合策略来自 registerWithStrategy, Controller.java:717-752)
pub(crate) fn strategy_for(config_key: &str) -> ActivationStrategy {
    match config_key {
        "enableVoiceWarn" => ActivationStrategy::config(config_key)
            .and(&ActivationStrategy::live_only()),
        "thrustdFS" => {
            ActivationStrategy::config("enableFMPrint").and(&ActivationStrategy::jet_only())
        }
        _ => ActivationStrategy::config(config_key),
    }
}

/// FocusMonitor 的通道桥 (轮 2-C 收口): Service 轮询线程内 FocusMonitor tick →
/// coordinator 回调 → UiCommand 送 win32 线程执行 host hide/show (配置/窗口
/// !Send 不能进 Service 线程 — ChannelPositionStore 同款模式)。
/// is_overlays_hidden 读 ControllerShared 镜像 (win32 处理命令时同步)
pub(crate) struct ChannelFocusBridge {
    pub(crate) tx: Sender<UiCommand>,
    pub(crate) shared: Arc<ControllerShared>,
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

/// Java Controller.registerGameModeOverlays (651-753) 的 win32 侧一次性注册
/// (live 模式 overlay 全集: 真实遥测数据态; 旧名 register_game_mode_overlays)。
/// PORT(偏差备案): Java 每 Controller 重建 OverlayManager + 重注册; Rust host 跨
/// 重建存活 (D8), 条目是无状态配置记录 (id/config_key/尺寸/渲染闭包), 重建语义
/// 由激活探测 (实时配置) + 命令通道承载 — 重注册无信息增量。
///
/// 注册键 10/10 落位 (P6 收口 + 人工验收补口 + 本批 enableFMPrint/thrustdFS):
/// - 窗口条目 9: enableEngineControl / engineInfoSwitch / crosshairSwitch /
///   flightInfoSwitch (POC window.rs 专径收编, vm-overlay flight_info.rs) /
///   enablegearAndFlaps / enableAxis / enableAttitudeIndicator /
///   enableFMPrint (FMUnpackedData, P5 组装契约三点销号 — 动态窗口高经
///   FmUnpackedFeed pump 落 resize_entry, 逐条目可见性经 host set_entry_visible,
///   spec 工厂 vm-overlay fm_unpacked_data_overlay_spec) /
///   thrustdFS (DrawFrameSimpl, 本批全量翻译装配 — vm-overlay draw_frame_simpl.rs:
///   激活策略 config("enableFMPrint").and(jetOnly) 经 [`strategy_for`] 实际生效,
///   固定几何 (0, screenH-500, 900, 500) 经 host set_entry_fixed_pos,
///   run 循环 (自管可见性 + displayFmKey==0 收腿退场) 经 DrawFrameSimplFeed)。
/// - 非窗口 1 (键在激活缓存 ACTIVATION_KEYS / strategy_for 留有映射, 不建窗口):
///   - enableVoiceWarn: VoiceWarning 为线程形态非窗口 — 装配在 OpenAllOverlays/
///     CloseAllOverlays 命令处理点 ([`open_voice_warning`]/VoiceWarnSession,
///     激活探测与窗口条目同源), 不走 host 注册面
#[allow(clippy::too_many_arguments)] // 组装面参数包 (host/handles/env/inputs/仓/lang/
                                     // shared/fm/fm_config) — 对位 Java registerGameModeOverlays 的 this 域
pub(crate) fn register_live_overlays(
    host: &mut OverlayHost,
    handles: &mut OverlayHandles,
    env: &Env,
    inputs: &OverlayInputs,
    // WYSIWYG reinit 参数仓 (CONFIG_CHANGED 后 ReinitOverlays 命令覆写;
    // 各 spec 工厂 reinit 闭包持引用读取 — 见 vm-overlay reinit.rs 头注)
    params: &Rc<RefCell<vm_overlay::ReinitParams>>,
    lang: &Rc<Lang>,
    shared: &ControllerShared,
    // FM拆包数据: reinit 闭包的 blkx 直读源 (Java FMManager.getInstance())
    fm: &Arc<FMManager>,
    // FM show* 配置键快照 (generate_lines 逐 tick 读面, 配置 !Send 的跨线程桥)
    fm_field_config: &Arc<Mutex<HashMap<String, String>>>,
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
    // FM拆包数据 (Java:726-743, 键 enableFMPrint, previewEnabled=true) — 本批补齐
    // (P5 组装契约三点销号; 事件面/tick 泵在 win32 循环驱动, 见 win32_thread_main)
    match fm_unpacked_data_overlay_spec(
        fonts,
        env.dpi.get_logical_screen_height(),
        params,
        // 原 FmFieldConfigSnapshot (FM show* 快照) — SnapshotConfigProvider 三合一
        Some(Arc::new(SnapshotConfigProvider::new(Arc::clone(fm_field_config)))
            as Arc<dyn ConfigProvider>),
        fm,
    ) {
        Ok((h, spec)) => {
            shared.note_registered_overlay(&spec.id);
            handles.fm_unpacked = Some(h);
            host.register(spec).with_interest(&FM_UNPACKED_INTEREST_KEYS);
        }
        Err(e) => logger::error("Controller", &format!("FM拆包数据 overlay 注册失败: {}", e)),
    }
    // 推力曲线 (Java:745-752 registerWithStrategy("thrustdFS"), 键 =
    // enableFMPrint && jetOnly, previewEnabled=true/needsThread) — 本批补齐
    // (D8 降级清单 P6 尾巴收口; 事件面/run 泵在 win32 循环驱动)
    match draw_frame_simpl_spec(fonts, fm) {
        Ok((h, spec)) => {
            shared.note_registered_overlay(&spec.id);
            handles.draw_frame_simpl = Some(h);
            host.register(spec);
            // Java init/initPreview 的 setBounds(0, screenH-500, 900, 500) — 每次
            // 实例化固定几何 (thrustdFSX/Y 只写不读, 不参与定位)。
            // PORT: Java Toolkit.getScreenSize() 在生产 JVM 标志 -Dsun.java2d.
            // uiScale=1 下与 DPIHelper 逻辑高同值 (恒等), 取逻辑高
            host.set_entry_fixed_pos(
                "thrustdFS",
                0,
                env.dpi.get_logical_screen_height() - 500,
            );
        }
        Err(e) => logger::error("Controller", &format!("推力曲线 overlay 注册失败: {}", e)),
    }
}

/// 托盘 handler: 动作转发主线程 (Java 托盘回调在 EDT, Rust 泵线程→channel)。
/// 关于项 (Application.java:236-245) 已接线: About → 主循环 emit → 前端 Modal。
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
    fn about(&mut self) {
        let _ = self.tx.send(MainEvent::Tray(TrayCommand::About));
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
pub(crate) struct AttitudeFeedState {
    pub(crate) freq_ms: i64,
    pub(crate) last_ms: i64,
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
/// PORT(MiniHUD 喂入形态, W-B 事件瘦身后): 转发链只送 EventPayload (瘦事件,
/// 纯节拍+标量); state/indicators 取自 FrameStore 最近帧直传 hud_calculator
/// (按喂入时刻取最新值, 与 Java EDT 读共享可变引用同一时序语义; 曾长期传
/// None/None 致襟翼/油门/姿态/G 值全 0 = "bar 恒 0" 根因); HUDData 由
/// minihud::update_from_event 现场计算 (hud_data 通道已删)。
///
/// PORT(B-W2 已兑现, 重构波4): 本函数原持 ServiceData 读锁跨纯计算段 (备形态
/// 已消) — 现取 FrameStore 不可变帧 (零锁), 各 update 签名 &dyn FormulaView
/// 直收 &Frame。与 Java 的 EDT 回退路径 (MiniHUDOverlay EDT 内直读 Service
/// 公开字段无锁计算) 同形态。
///
/// PORT(panic 边界): ServiceData 的保真 panic 点 (get_pitch/get_thrust 的空引擎
/// 数组索引, service_fields.rs 注) 在畸形 s_state (update 失败 pitch/thrust 未填)
/// 下可达 — Java NPE 由 AWT EDT 吞掉 (UI 存活), Rust win32 线程 panic 会杀整个
/// host 泵, 故整帧 catch_unwind (AssertUnwindSafe: 状态可能半更新, 对位 Java
/// EDT 半更新后吞 NPE 的形态), ERROR 留痕丢帧继续。
pub(crate) fn feed_overlays_live(
    handles: &OverlayHandles,
    payload: &EventPayload,
    shared: &ControllerShared,
    fm: &FMManager,
    settings: &vm_core::config_api::HudSettingsSnapshot,
    lang: &Lang,
    attitude_feed: &mut AttitudeFeedState,
) {
    // preview 门控 (见函数头注 PORT(preview 门控))
    if shared.overlay_ctx_preview.load(Ordering::SeqCst) {
        return;
    }
    let live = shared.live.read().expect("live 锁中毒").clone();
    let Some(frames) = live else { return };
    let Some(frame) = frames.latest() else {
        return; // 尚无首帧 (Service 已装配, 等待首个轮询周期)
    };
    let now = current_time_millis();
    let fm_handle = fm.current();
    // getload 已落地 (reader.rs, 真机位级对拍): READY 句柄的 blkx 翼数据/
    // is_v_wing 恒被 populate, 原过渡期降级守卫 (is_v_wing=None → 无 FM 路径)
    // 已随该波次移除 — VNE/AoA 告警/flapAllowAngle/机动指数全量走 FM 数据
    let fmdata = fm_handle.fmdata.as_ref();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // 1. MiniHUD (Java MiniHUDOverlay.onFlightData → invokeLater)
        if let Some(h) = handles.minihud.as_ref() {
            // W-B: State/Indicators 直接从共享 guard 借引用下传 (hud_calculator
            // 读 flaps/throttle/gear/airbrake/aoa/ny/姿态), 不再装箱重建事件
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
                frame.s_state.as_ref(),
                frame.s_indic.as_ref(),
                payload,
                Some(&*frame),
                fmdata,
                settings,
                &colors,
            );
        }
        // 2. 动力信息 (Java FieldOverlay.onFlightData 50ms 节流闩内置)
        if let Some(h) = handles.power_info.as_ref() {
            h.borrow_mut().update(now, &*frame);
        }
        // 3. 引擎控制 (节流闩 = refreshInterval 配置驱动; compressorStages 档位数 =
        //    Java FMManager.current().compressorStages, 非 READY/喷气机 → None)
        if let Some(h) = handles.engine_control.as_ref() {
            let stages = fm_handle
                .compressor_stages
                .as_ref()
                .map(|v| v.len() as i32);
            h.borrow_mut().update(now, &*frame, payload, stages);
        }
        // 4. 起落襟翼 (100ms 节流闩内置)
        if let Some(h) = handles.gear_flaps.as_ref() {
            h.borrow_mut().update_tick(now, lang, &*frame);
        }
        // 5. 操纵面 (50ms 节流内置; has_service = Java init(S) 的 xs!=null 数据门控,
        //    单实例形态下由喂入点随游戏窗口形态置位 — 见工厂头注 PORT(数据门控))
        // 飞行信息 (Java FlightInfoOverlay.onFlightData 字段行更新, 无节流 —
        // host 50ms 渲染节拍 + 像素指纹兜底; W2: 数据 = TelemetrySource 散字段)
        if let Some(h) = handles.flight_info.as_ref() {
            h.borrow_mut().update(&*frame);
        }
        if let Some(h) = handles.control_surfaces.as_ref() {
            let mut cs = h.borrow_mut();
            cs.has_service = true;
            // W7: var_value 桥取值 (getter 实现已消解)
            cs.on_flight_data(
                now,
                frame.var_value("aileron").unwrap_or(0.0),
                frame.var_value("elevator").unwrap_or(0.0),
                frame.var_value("rudder").unwrap_or(0.0),
                frame.var_value("wing_sweep").unwrap_or(0.0),
                frame.var_value("wing_sweep_valid").unwrap_or(0.0) != 0.0,
            );
        }
        // 6. 地平仪 (节流 = freqMili 40ms 配置驱动, 喂入侧承载;
        //    aoa_limits = blkx.NoFlapsWing.AoACritHigh/Low, 无 FM → None 不显示)
        if let Some(h) = handles.attitude.as_ref() {
            if now - attitude_feed.last_ms > attitude_feed.freq_ms {
                attitude_feed.last_ms = now;
                let aoa_limits = fmdata
                    .and_then(|b| b.no_flaps_wing.as_ref())
                    .map(|w| (w.aoa_crit_high, w.aoa_crit_low));
                h.borrow_mut().update_telemetry(
                    frame.var_value("aoa").unwrap_or(0.0),
                    frame.var_value("aos").unwrap_or(0.0),
                    frame.var_value("aviahorizon_pitch").unwrap_or(0.0),
                    frame.var_value("aviahorizon_roll").unwrap_or(0.0),
                    frame.var_value("compass").unwrap_or(0.0),
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
/// hotkey_rx 中转后 publish ui_bus) — DrawFrameSimpl/FMUnpacked 的订阅消费
/// (渲染节拍块) 已按此拓扑接线, 后续 DrawFrame (P6 批三) 照此办理。
pub fn win32_thread_main(cfg: Win32ThreadConfig) {
    let Win32ThreadConfig {
        env,
        inputs,
        ui_bus,
        flight_bus,
        fm,
        shared,
        activation,
        voice,
        voice_config,
        fm_field_config,
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
        fm_unpacked: None,
        draw_frame_simpl: None,
    };
    // Lang 一次构造 (GearFlaps update_tick 的标签源; 注册面与喂入共用)。
    // Rc 共享: engine 工厂的 reinit 闭包重建 state 需要标签源 (Lang !Clone)
    let lang = Rc::new(Lang::init_lang());
    // WYSIWYG reinit 参数仓 (初始 = 注册快照投影; CONFIG_CHANGED 后
    // UiCommand::ReinitOverlays 覆写, 各 spec 工厂 reinit 闭包读取)
    let params = Rc::new(RefCell::new(vm_overlay::ReinitParams::from(&inputs)));
    register_live_overlays(
        &mut host,
        &mut handles,
        &env,
        &inputs,
        &params,
        &lang,
        &shared,
        &fm,
        &fm_field_config,
    );
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
    // VoiceWarning 会话槽 (非窗口 overlay 条目: Java registerWithStrategy 的
    // needsThread 形态 — openAll 建/closeAll 停; 激活策略与窗口条目同源探测)
    let mut voice_warn: Option<VoiceWarnSession> = None;

    // ---- FMUnpackedData 事件订阅 (Java FMUnpackedDataOverlay.init 的两处
    // UIStateBus.subscribe: FM_OVERLAY_TOGGLE 翻转 / FM_CHANGED reload;
    // overlays_field2.rs 头注契约 "由组装层的事件循环驱动" — 句柄 !Send (Rc),
    // 经 channel 中转到本循环消费; 订阅句柄随线程 Drop = Java dispose 退订链) ----
    let (fm_toggle_tx, fm_toggle_rx) = std::sync::mpsc::channel::<()>();
    let _fm_toggle_sub = ui_bus.subscribe(ui_state_events::FM_OVERLAY_TOGGLE, move |_ev| {
        let _ = fm_toggle_tx.send(());
    });
    // FM_CHANGED 载荷 = FMHandle (fm_manager 强类型总线, Java instanceof 过滤由
    // 类型免除)。blkx 深拷一次进通道 (FMHandle.blkx 值字段 → 句柄侧 Arc<Blkx>;
    // 换机事件低频, 成本可忽略)
    let (fm_data_tx, fm_data_rx) = std::sync::mpsc::channel::<Option<vm_core::fmdata::FmData>>();
    let _fm_changed_sub = fm.fm_changed_bus().subscribe(move |h| {
        let _ = fm_data_tx.send(h.fmdata.clone());
    });
    // FMUnpackedData 的 run() 轮询泵 (Java BaseOverlay.run 线程的单线程驱动侧,
    // 200ms 节流 + 可见门控 + 高度自适应, 见 FmUnpackedFeed 头注)
    let mut fm_unpacked_feed = FmUnpackedFeed::new();
    // DrawFrameSimpl 的 run() 循环泵 (Java :737-767: 1000ms 节流 + 自管可见性 +
    // displayFmKey==0 收腿 10s 退场, 见 DrawFrameSimplFeed 头注)
    let mut dfs_feed = DrawFrameSimplFeed::new();

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
            // FMUnpackedData 事件面 + run 泵 (Java: 游戏实例订阅 toggle/FM_CHANGED —
            // initPreview 不订阅, 保持 fm_live 门控; run 线程 needsThread=true 两会话
            // 均在跑 (OverlayManager.refreshPreview :326-331, 审查 B2-2 修正 — 原
            // "预览实例无 run 线程" 为假前提), 泵不再门控, 仅条目未激活 (Java 无
            // 实例 = host 槽位空) 时跳过。事件恒排空防积压跨会话误触发)
            let fm_live = !shared.overlay_ctx_preview.load(Ordering::SeqCst);
            while fm_toggle_rx.try_recv().is_ok() {
                if fm_live {
                    if let Some(h) = handles.fm_unpacked.as_ref() {
                        h.borrow_mut().toggle(); // FM_OVERLAY_TOGGLE handler (:72-75)
                    }
                    // DrawFrameSimpl toggle handler (Java :526-529, 仅游戏 init 挂接 —
                    // Java 双订阅方之二)
                    if let Some(h) = handles.draw_frame_simpl.as_ref() {
                        h.borrow_mut().toggle();
                    }
                }
            }
            while let Ok(fmdata) = fm_data_rx.try_recv() {
                if fm_live {
                    if let Some(h) = handles.fm_unpacked.as_ref() {
                        // FM_CHANGED handler reloadFMData (:130-136)
                        h.borrow_mut().reload_fm_data(fmdata.clone().map(Arc::new));
                    }
                }
                // DrawFrameSimpl 的 FM_CHANGED (Java initFmHandleCache :79-88 被
                // init 与 initPreview 共用) — 两会话均刷新缓存 (预览实例同样订阅,
                // repaint 由渲染节拍脏检查承接)
                if let Some(h) = handles.draw_frame_simpl.as_ref() {
                    h.borrow_mut().reload_fm(fmdata.map(Arc::new));
                }
            }
            if host.is_active("enableFMPrint") {
                if let Some(h) = handles.fm_unpacked.as_ref() {
                    fm_unpacked_feed.pump(&mut host, "enableFMPrint", h, current_time_millis());
                }
            }
            // DrawFrameSimpl run 泵: displayFmKey = Application.displayFmKey 的
            // ControllerShared.flags 对位 (bind/handleFmHotkeyConfigChange 同步);
            // flight = live Service 快照 (None = 预览无 Service — Java NPE 杀线程
            // 的对位为冻结判定, 见 pump 头注)
            if host.is_active("thrustdFS") {
                if let Some(h) = handles.draw_frame_simpl.as_ref() {
                    let display_fm_key = shared
                        .flags
                        .lock()
                        .expect("flags 锁中毒")
                        .current_fm_hotkey_code;
                    let flight = shared
                        .live
                        .read()
                        .expect("live 锁中毒")
                        .as_ref()
                        .and_then(|frames| frames.latest())
                        .map(|f| {
                            // Java sState 恒非 null (Service 构造即建) — None 轮按
                            // 缺省 0 (同 Java State 字段初值)
                            DfsFlight {
                                gear: f.s_state.as_ref().map(|s| s.gear).unwrap_or(0),
                                speedv: f.var_value("speedv").unwrap_or(0.0),
                                throttle: f.s_state.as_ref().map(|s| s.throttle).unwrap_or(0),
                            }
                        });
                    dfs_feed.pump(
                        &mut host,
                        "thrustdFS",
                        h,
                        current_time_millis(),
                        display_fm_key,
                        flight,
                    );
                }
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
                    // 专径自接, thrustdFS 无 FlightDataBus 订阅 (事件面/run 泵在
                    // 渲染节拍块驱动)。
                    shared.overlay_ctx_preview.store(false, Ordering::SeqCst); // for_live (Java forGameMode)
                    // 操纵面数据门控 (overlays_field2.rs PORT(数据门控)): Java init(S)
                    // 的 xs!=null 在此翻转 — openpad 即游戏形态 (has_service=true)
                    if let Some(h) = handles.control_surfaces.as_ref() {
                        h.borrow_mut().has_service = true;
                    }
                    // FM拆包数据游戏形态 (Java init :57-94 的单实例对位):
                    // :730 fmDataAdapter.setBlkx(current().blkx) + :64 isPreview=false
                    // + :67 Game mode: initially hidden (表头谓词/setupFont 与
                    // preview 形态同值, 免重设)
                    if let Some(h) = handles.fm_unpacked.as_ref() {
                        let mut fmov = h.borrow_mut();
                        fmov.base.is_preview = false;
                        fmov.visible = false;
                        fmov.reload_fm_data(fm.current().fmdata.clone().map(Arc::new));
                    }
                    // 推力曲线游戏形态 (Java init :514-528 的单实例对位):
                    // initFmHandleCache (current 快照) + isPreview=false + 隐藏起步
                    if let Some(h) = handles.draw_frame_simpl.as_ref() {
                        h.borrow_mut().init(fm.current().fmdata.clone().map(Arc::new));
                    }
                    if let Err(e) = host.open_all() {
                        logger::error("OverlayHost", &format!("open_all: {}", e));
                    }
                    // Java init 末尾 setVisible(false): 窗口隐藏起步 (热键切换),
                    // 免首个 tick (≤200ms) 前的可见闪现
                    host.set_entry_visible("enableFMPrint", false);
                    // DrawFrameSimpl 同理 (init 末 setVisible(true) 后 run 首轮即
                    // 隐藏 — Java 有 ≤1 线程轮的闪现, 此处同 FMUnpacked 先例预消)
                    host.set_entry_visible("thrustdFS", false);
                    // VoiceWarning (非窗口条目): Java openAll 对 enableVoiceWarn 走
                    // 同一 OverlayEntry.open — 激活探测 (config+live_only, 此刻
                    // preview 已翻 false = forGameMode ctx) 命中即 init(this,S) +
                    // 起告警线程 (100ms tick + fatalWarn 回写)。幂等守卫对位
                    // Java "instance != null 跳过"
                    if voice_warn.is_none() {
                        let vctx = HostActivationCtx {
                            activation: Arc::clone(&activation),
                            fm: Arc::clone(&fm),
                            shared: Arc::clone(&shared),
                            debug: env.debug,
                        };
                        if strategy_for("enableVoiceWarn").should_activate(&vctx) {
                            let live = shared.live.read().expect("live 锁中毒").clone();
                            match open_voice_warning(
                                &voice,
                                &ui_bus,
                                &voice_config,
                                &fm,
                                &flight_bus,
                                live,
                            ) {
                                Some(s) => {
                                    logger::info(
                                        "OverlayManager",
                                        "Started thread for: enableVoiceWarn",
                                    );
                                    voice_warn = Some(s);
                                }
                                None => logger::info(
                                    "OverlayManager",
                                    "Skipping open for enableVoiceWarn: no live Service",
                                ),
                            }
                        }
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
                    // 数据面重置 (同上"实例销毁"语义的另一半): Java close 即实例
                    // 死亡, preview 重开经工厂全新实例 + initPreview 静态值; Rust
                    // handle 跨 close 存活 (render 闭包持同一 Rc), 不重置则下次
                    // preview 窗渲染上次 live 残留值 (托盘 live→preview 复现)
                    reset_handles_preview_values(&handles);
                    // 推力曲线 run 循环复位 (Java closeAll → 实例/线程销毁; 下次
                    // open/refreshPreview 重建 — 自动退场后的重生入口)
                    dfs_feed.reset();
                    host.close_all(); // close 销毁链 (存位置 → drop)
                    // Java overlay dispose → Bus.unregister (drop 槽位即退订)
                    drop(std::mem::take(&mut flight_sub));
                    // VoiceWarning 停 (Java OverlayEntry.close: interrupt 告警线程;
                    // Drop 兜底 = doit 翻 false + join, 双订阅同时被退订)
                    if voice_warn.is_some() {
                        logger::info("OverlayManager", "Closing overlay: enableVoiceWarn");
                        voice_warn = None;
                    }
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
                    // ---- voice_warn 条目的 refreshPreview 重估面 (审查 W1 修复) ----
                    // Java refreshPreviews 对触达条目调 entry.refreshPreview
                    // (forPreviewMode ctx): shouldBeOpen = config &&
                    // gameModeOnly, preview ctx 下 gameModeOnly=false → 在场
                    // 即 close (关开关即时生效, Controller.java:498-536 →
                    // OverlayManager.java:320-340)。Rust 原实现只走 host 窗口
                    // 条目, voice_warn 无重估面 — 关掉开关后告警继续响到会话
                    // 结束 (CloseAllOverlays 才停), 用户可感知偏差。补齐: 探测
                    // 窗口内 (preview=true → live_only=false, 与 Java forPreviewMode
                    // 下 gameModeOnly=false 同源) 对触达键在场即停。
                    // 开方向不重建 — Java preview-ctx 下 shouldBeOpen 恒 false
                    // 同样不 open (怪癖保真), 重起等下次 OpenAllOverlays。
                    if voice_warn.is_some() && voice_warn_refresh_reaches(changed_key.as_deref()) {
                        let vctx = HostActivationCtx {
                            activation: Arc::clone(&activation),
                            fm: Arc::clone(&fm),
                            shared: Arc::clone(&shared),
                            debug: env.debug,
                        };
                        if !strategy_for("enableVoiceWarn").should_activate(&vctx) {
                            logger::info(
                                "OverlayManager",
                                "Closing overlay (inactive strategy): enableVoiceWarn",
                            );
                            voice_warn = None; // Drop → stop (doit 翻 false + join), 订阅退订兜底
                        }
                    }
                    let r = match changed_key.as_deref() {
                        Some(k) => host.refresh_preview_key(Some(k)),
                        None => host.refresh_preview(),
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
            ui_bus.publish(&hk.event_type, Some("HotkeyManager"), Some(&hk.key_code.to_string()));
        }
        std::thread::sleep(Duration::from_millis(10)); // 事件泵 10ms (host.run 同款)
    }
}
