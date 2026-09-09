//! 渲染线程 (D8: host 泵 + 托盘 + 热键事件消费)。重构波2 自 app_shell.rs 拆出,
//! 波16 自 win32.rs 更名 — 本文件是渲染线程**装配层**, 真正的 Win32 API 胶水
//! 在 vm-overlay::platform (窗口/托盘/热键) 与本 crate 的 winmm_player。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use vm_core::activation::strategy::{ActivationContext, ActivationStrategy};
use vm_core::audio::voice_resource_manager::VoiceResourceManager;
use vm_core::base::bus::flight_data_bus::FlightDataBus;
use vm_core::base::bus::ui_state_bus::{UIStateBus, UiStateEvent};
use vm_core::base::bus::Subscription;
use vm_core::base::event::event_payload::EventPayload;
use vm_core::base::event::flight_data_event::FlightDataEvent;
use vm_core::base::event::ui_state_events;
use vm_core::base::java_compat::{current_time_millis, java_parse_boolean};
use vm_core::base::logger;
use vm_core::config::config_api::HudSettingsSnapshot;
use vm_core::derived::hud_calculator::HudColors;
use vm_core::fm::{FMHandle, FMManager};
use vm_core::formula::registry::FormulaView; // var_value 取数唯一接口 (W10 后 TelemetrySource 已删)
use vm_core::lang::Lang;

use vm_overlay::overlays::minihud::{minihud_overlay_spec, MiniHudHandle};
use vm_overlay::platform::host::{OverlayHost, OverlaySpec};
use vm_overlay::widgets::{page_font_size, page_overlay_spec, PageHandle, PageSpecParams};
use vm_overlay::platform::hotkey::HotkeyEvent;

#[cfg(target_os = "windows")]
use vm_overlay::platform::tray::{TrayConfig, TrayHandler, TrayIcon};

use crate::commands::{MainEvent, TrayCommand, UiCommand};
use crate::controller_shared::{is_stale_refresh, ControllerShared};
use crate::env::Env;
use crate::keys::{FM_UNPACKED_INTEREST_KEYS, MINIHUD_INTEREST_KEYS, OVERLAY_SECTIONS};
use crate::overlay_inputs::{ActivationCache, OverlayInputs};
use crate::voice_setup::{
    open_voice_warning, voice_warn_refresh_reaches, ConfigSnapshots, VoiceWarnSession,
};

/// 渲染线程装配输入 (全部 Send; 配置以快照形态入线程, 见模块头)
pub struct RenderThreadConfig {
    pub env: Env,
    pub inputs: OverlayInputs,
    pub ui_bus: Arc<UIStateBus>,
    pub flight_bus: Arc<FlightDataBus>,
    pub fm: Arc<FMManager>,
    pub shared: Arc<ControllerShared>,
    pub activation: ActivationCache,
    /// 共享语音资源管理器 (AppShell.voice; VoiceWarning 告警线程的 reload 面)
    pub voice: Arc<VoiceResourceManager>,
    /// 配置跨线程快照对 (AppShell.config_snapshots; 配置 !Send 的跨线程桥 —
    /// voice_* 供 VoiceWarning reload, FM show* 供 FMUnpackedData generate_lines)
    pub snapshots: ConfigSnapshots,
    pub ui_cmd_rx: Receiver<UiCommand>,
    pub hotkey_rx: Receiver<HotkeyEvent>,
    pub main_event_tx: Sender<MainEvent>,
    /// overlay 初始位置快照 (id → 归一化; 主线程 spawn 前从 GroupConfig.x/y 取,
    /// 渲染线程不能碰 !Send 配置树 — 见 ChannelPositionStore 头注)
    pub position_snapshot: HashMap<String, (f64, f64)>,
}

/// 渲染线程内注册的 overlay 数据句柄 (Rc — 恒留本线程)。
/// None = spec 工厂失败 (字体缺失等, 注册点已 logger::error), 喂入跳过
pub(crate) struct OverlayHandles {
    /// MiniHUD live 喂入口 (对位 Java onFlightData 的 UI 线程单线程派发面)
    pub(crate) minihud: Option<MiniHudHandle>,
    /// W3 通用页面 (doc_id → 编排器句柄; 喂数经 UpdateEnv 统一分发,
    /// FM 黑盒页的数据面走 WidgetSidecar)
    pub(crate) pages: Vec<(String, PageHandle)>,
}

/// CloseAllOverlays 时数据面回 preview 静态初值 (渲染线程命令处理点调用)。
/// W3 组件化后: pages 逐组件 reset_preview (trait HudWidget 覆写, 有状态
/// 组件复位数据态); FM 两页 (拆包/推力曲线) 的 sidecar 形态复位也在各自组件
/// reset_preview 内 (可见/预览态/lastData 清空 — 对位 Java closeAll 销毁实例 +
/// 预览工厂新建 initPreview 的形态)。
pub(crate) fn reset_handles_preview_values(handles: &OverlayHandles) {
    // W3 页面: 组件级 preview 复位 (trait reset_preview, 有状态组件覆写)
    for (_, page) in &handles.pages {
        let p = page.borrow_mut();
        for cell in p.cells.values() {
            cell.reset_preview();
        }
    }
}

/// Java OverlayContext 的渲染线程侧替身: 激活探测访问面
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
/// 两处复合策略来自 Java registerWithStrategy)
pub(crate) fn strategy_for(config_key: &str) -> ActivationStrategy {
    match config_key {
        // 空键 = 用户页未设开关键 → 恒显 (对位 PageDoc.switch_key 注释语义)
        "" => ActivationStrategy::always(),
        "enableVoiceWarn" => {
            ActivationStrategy::config(config_key).and(&ActivationStrategy::live_only())
        }
        "thrustdFS" => {
            ActivationStrategy::config("enableFMPrint").and(&ActivationStrategy::jet_only())
        }
        _ => ActivationStrategy::config(config_key),
    }
}

/// FocusMonitor 的通道桥 (轮 2-C 收口): Service 轮询线程内 FocusMonitor tick →
/// coordinator 回调 → UiCommand 送渲染线程执行 host hide/show (配置/窗口
/// !Send 不能进 Service 线程 — ChannelPositionStore 同款模式)。
/// is_overlays_hidden 读 ControllerShared 镜像 (渲染线程处理命令时同步)
pub(crate) struct ChannelFocusBridge {
    pub(crate) tx: Sender<UiCommand>,
    pub(crate) shared: Arc<ControllerShared>,
}

impl vm_core::platform::focus_monitor::AlwaysOnTopCoordinatorApi for ChannelFocusBridge {
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

/// 位置存档后端 (渲染线程侧): 启动快照直读 + 保存经 MainEvent 回传主线程落盘。
/// PORT(线程桥): Java overlay 直接持 OverlaySettings (UI 单线程单世界); Rust 配置树
/// !Send 不能进渲染线程, 位置面拆成 读=启动快照 (位置仅拖拽改变, 而拖拽存档
/// 双写快照, 快照不滞后) 写=回传 (PositionSaved → save_group_position 落盘,
/// 对齐 Java saveWindowPosition 即时 saveLayoutConfig)。
struct ChannelPositionStore {
    snapshot: HashMap<String, (f64, f64)>,
    tx: Sender<MainEvent>,
}

impl vm_overlay::platform::host::PositionStore for ChannelPositionStore {
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

/// Java Controller.registerGameModeOverlays 的渲染线程侧一次性注册
/// (live 模式 overlay 全集: 真实遥测数据态; 旧名 register_game_mode_overlays)。
/// PORT(偏差备案): Java 每 Controller 重建 OverlayManager + 重注册; Rust host 跨
/// 重建存活 (D8), 条目是无状态配置记录 (id/config_key/尺寸/渲染闭包), 重建语义
/// 由激活探测 (实时配置) + 命令通道承载 — 重注册无信息增量。
///
/// 注册键 10/10 落位 (P6 收口 + 人工验收补口 + enableFMPrint/thrustdFS):
/// - 窗口条目 9 = keys.rs [`OVERLAY_SECTIONS`] 的 8 键 (本函数逐键注册; 位置组
///   映射/main.rs 冒烟断言集同源该表, 新增窗口条目只改 keys.rs 一处) +
///   thrustdFS (DrawFrameSimpl — thrust-chart 页组件 fm.thrust_chart: 激活策略
///   config("enableFMPrint").and(jetOnly) 经 [`strategy_for`] 实际生效,
///   固定几何 (0, screenH-500, 900, 500) 经 host set_entry_fixed_pos,
///   run 循环 (自管可见性 + displayFmKey==0 收腿退场) 经 sidecar tick)。
/// - 特注 enableFMPrint (FM拆包数据页, 8 键之一) — 字段原子化 (core.fm.field/
///   meta 逐字段组件): 数据面 = 逐组件 sidecar tick (200ms 自节流 + FM 直读 +
///   show* 段开关), 行归零/恢复 → 页面 refresh_sizing 包围盒收敛 → resize_entry。
/// - 非窗口 1 (键在激活缓存 ACTIVATION_KEYS / strategy_for 留有映射, 不建窗口):
///   - enableVoiceWarn: VoiceWarning 为线程形态非窗口 — 装配在 OpenAllOverlays/
///     CloseAllOverlays 命令处理点 ([`open_voice_warning`]/VoiceWarnSession,
///     激活探测与窗口条目同源), 不走 host 注册面
///
/// 9 段同构样板 (工厂 → 落键/落槽/注册 / Err 日志) 经 [`register_one`] 收敛,
/// 句柄 Option 返回值直接落 [`OverlayHandles`] 槽位。
pub(crate) fn register_live_overlays(
    host: &mut OverlayHost,
    handles: &mut OverlayHandles,
    setup: &OverlayRegSetup,
) {
    let OverlayRegSetup {
        env,
        inputs,
        params,
        lang,
        shared,
    } = setup;
    let fonts = &env.fonts_dir;
    // MiniHUD (键 crosshairSwitch; HUDSettings 经快照)
    // service_present=false (注册时 Service 尚未建; 该标志影响 preview 行为集,
    // live 重接线批次随 spec 工厂参数化回收)
    handles.minihud = register_one(host, shared, "MiniHUD", &MINIHUD_INTEREST_KEYS, || {
        minihud_overlay_spec(
            false,
            inputs.service_loop_interval_ms,
            &inputs.hud,
            inputs.dpi_scale,
            &fonts.join("sarasa-mono-sc-bold.ttf"),
            params,
        )
    });
    // W3 通用页 (flight/power/engine/gear/axis/attitude + fm 两页 sidecar)
    // 出厂页 + 用户页同权注册 (P0: 编辑器新建/复制的页面同样落窗)
    for doc in inputs.pages.iter().filter(|d| is_page_overlay_entry(d)) {
        register_page(host, handles, env, lang, params, shared, doc);
    }
    // FM 两页 (fm-list/thrust-chart) 已并入上方 pages 循环 (sidecar 数据面
    // 在渲染节拍块驱动 — FM_OVERLAY_TOGGLE/FM_CHANGED 事件脉冲 + tick)
    // 推力曲线固定几何: setBounds(0, screenH-500, 900, 500) 每次实例化恒定
    if handles.pages.iter().any(|(id, _)| id == "thrust-chart-default") {
        host.set_entry_fixed_pos("thrustdFS", 0, env.dpi.get_logical_screen_height() - 500);
    }
}

/// 走通用页面编排的页面谓词: 出厂页 + 用户页全部放行, 仅排除 MiniHUD
/// 编排器专页 (minihud-default 由 minihud_overlay_spec 独占注册, 走通用
/// 面会双窗) — P0 前这里硬编码 8 个出厂 id, 用户页被全部过滤
fn is_page_overlay_entry(doc: &vm_core::config::json_model::PageDoc) -> bool {
    doc.id != "minihud-default"
}

/// 单页注册 (register_live_overlays 的 pages 段提取; sync_page_entries
/// 会话动态注册复用同一路径, 保证启动/运行时行为一致)
#[allow(clippy::too_many_arguments)]
fn register_page(
    host: &mut OverlayHost,
    handles: &mut OverlayHandles,
    env: &crate::env::Env,
    lang: &Rc<Lang>,
    params: &Rc<RefCell<vm_overlay::platform::reinit::ReinitParams>>,
    shared: &ControllerShared,
    doc: &vm_core::config::json_model::PageDoc,
) {
    if let Some(handle) = register_one(host, shared, &doc.name, page_interest_keys(&doc.id), || {
        page_overlay_spec(assemble_page_spec(doc.clone(), env, lang, params))
    }) {
        handles.pages.push((doc.id.clone(), handle));
    }
}

/// 用户页生命周期 = 配置驱动: 编辑器 save/delete/reset 页 → CONFIG_CHANGED →
/// ReinitOverlays (params.pages 覆写) → 本函数对齐条目集 — 新页注册落窗、
/// 消失的页注销摘窗。出厂页 reinit 只重建不增删 (条目集恒定, sync 无操作)。
/// 增删只发生在本渲染线程 ui_cmd 处理点, 与 feed 循环同线程串行, 无锁无竞态
fn sync_page_entries(
    host: &mut OverlayHost,
    handles: &mut OverlayHandles,
    env: &crate::env::Env,
    lang: &Rc<Lang>,
    params: &Rc<RefCell<vm_overlay::platform::reinit::ReinitParams>>,
    shared: &ControllerShared,
) {
    let docs = params.borrow().pages.clone();
    // ① 新页: 通过谓词且尚未注册 → 注册落窗
    for doc in docs.iter().filter(|d| is_page_overlay_entry(d)) {
        if !handles.pages.iter().any(|(id, _)| *id == doc.id) {
            register_page(host, handles, env, lang, params, shared, doc);
        }
    }
    // ② 消失的页: 注销 (close 完整销毁链 + 摘条目, 防僵留复活) + 摘句柄
    let live: Vec<String> = docs.iter().map(|d| d.id.clone()).collect();
    let gone: Vec<String> = handles
        .pages
        .iter()
        .filter(|(id, _)| !live.iter().any(|l| l == id))
        .map(|(id, _)| id.clone())
        .collect();
    for id in gone {
        host.unregister(&id);
    }
    handles.pages.retain(|(id, _)| live.iter().any(|l| l == id));
}

/// per-page WYSIWYG 兴趣键 (对位原 with_interest 键集; 死键已清 —
/// 行开关/列数类随字段原子化退役)
fn page_interest_keys(id: &str) -> &'static [&'static str] {
    match id {
        "flight-info-default" => &["flightInfo", "fontSize"],
        "power-info-default" => &["fontSize"],
        "engine-control-default" => &["fontSize", "dataPollIntervalMs"],
        "gear-flaps-default" => &["enablegearAndFlapsEdge", "fontSize"],
        "axis-default" => &["enableAxisEdge", "fontSize"],
        "attitude-default" => &["attitudeIndicator", "enableAttitudeIndicator"],
        "fm-list-default" => &FM_UNPACKED_INTEREST_KEYS,
        // thrustdFS 无追加键 (Java 同; 激活策略 = config(enableFMPrint)∧jetOnly)
        "thrust-chart-default" => &[],
        _ => &[],
    }
}

/// PageSpecParams 组装 (per-page 参数差异的集中点; refresh 闭包重取参数仓)
fn assemble_page_spec(
    doc: vm_core::config::json_model::PageDoc,
    env: &crate::env::Env,
    lang: &Rc<Lang>,
    params: &Rc<RefCell<vm_overlay::platform::reinit::ReinitParams>>,
) -> PageSpecParams {
    // 参数仓快照 (per-page 分差)
    let p = params.borrow();
    let dpi = env.dpi.get_scale();
    let gauge = vm_overlay::widgets::GaugeCfg {
        dpi_scale: dpi,
        service_loop_interval_ms: p.service_loop_interval_ms,
        engine_font_add: p.engine.font_add,
        gear: (p.gear.font_add, p.gear.show_edge),
        axis: (p.axis.font_add, p.axis.show_edge),
        attitude: (
            p.attitude.width,
            p.attitude.height,
            p.attitude.show_direction,
            p.attitude.show_aoa_limits,
        ),
        attitude_freq_ms: p.attitude_freq_ms,
        fm_font_add: p.fm.font_add,
        logical_height: env.dpi.get_logical_screen_height(),
    };
    let font_size = match doc.id.as_str() {
        "flight-info-default" => page_font_size(24, p.flight.font_add, dpi),
        "power-info-default" => page_font_size(24, p.power.font_add, dpi),
        "engine-control-default" => page_font_size(24, p.engine.font_add, dpi),
        "gear-flaps-default" => page_font_size(24, p.gear.font_add, dpi),
        "axis-default" => page_font_size(24, p.axis.font_add, dpi),
        "fm-list-default" | "thrust-chart-default" => 24,
        // 用户页/其余: 页文档自身字号增量 (PageDoc.font.sizeAdd 落地)
        _ => page_font_size(24, doc.font.size_add, dpi),
    };
    let hud = p.hud.clone();
    drop(p);

    // refresh 闭包: 重取参数仓 (reinit 语义 — CONFIG_CHANGED 后 ReinitOverlays 覆写)
    let refresh_params = Rc::clone(params);
    let refresh_env_dpi = env.dpi.get_scale();
    let refresh_doc = doc.clone();
    let refresh_lang = (**lang).clone();
    let refresh: Box<dyn Fn() -> PageSpecParams> = Box::new(move || {
        let p = refresh_params.borrow();
        // 重取最新页面文档 (CONFIG_CHANGED 时 ReinitParams.pages 被主线程
        // 覆写 — minihud reinit 同款; 不重取则编辑器改 props/增删组件游戏态
        // 不生效)
        let doc = p
            .pages
            .iter()
            .find(|d| d.id == refresh_doc.id)
            .cloned()
            .unwrap_or_else(|| refresh_doc.clone());
        let fs = match doc.id.as_str() {
            "flight-info-default" => page_font_size(24, p.flight.font_add, refresh_env_dpi),
            "power-info-default" => page_font_size(24, p.power.font_add, refresh_env_dpi),
            "engine-control-default" => page_font_size(24, p.engine.font_add, refresh_env_dpi),
            "gear-flaps-default" => page_font_size(24, p.gear.font_add, refresh_env_dpi),
            "axis-default" => page_font_size(24, p.axis.font_add, refresh_env_dpi),
            // 用户页/其余: 页文档自身字号增量 (PageDoc.font.sizeAdd 落地)
            _ => page_font_size(24, doc.font.size_add, refresh_env_dpi),
        };
        let hud = p.hud.clone();
        drop(p);
        PageSpecParams {
            entry_key: doc.entry_key.clone(),
            gauge_cfg: refresh_gauge(&refresh_params, refresh_env_dpi),
            doc: doc.clone(),
            font_path: refresh_font_path(&doc),
            font_size: fs,
            lang: refresh_lang.clone(),
            settings: hud,
            debug: false,
            refresh: Box::new(|| unreachable!("refresh 的 refresh 不可达")),
        }
    });

    PageSpecParams {
        entry_key: doc.entry_key.clone(),
        gauge_cfg: gauge,
        doc,
        font_path: env.fonts_dir.join("sarasa-mono-sc-bold.ttf"),
        font_size,
        lang: (**lang).clone(),
        settings: hud,
        debug: false,
        refresh,
    }
}

/// refresh 闭包的 GaugeCfg 重取 (参数仓 + dpi 快照)
fn refresh_gauge(
    params: &Rc<RefCell<vm_overlay::platform::reinit::ReinitParams>>,
    dpi: f64,
) -> vm_overlay::widgets::GaugeCfg {
    let p = params.borrow();
    vm_overlay::widgets::GaugeCfg {
        dpi_scale: dpi,
        service_loop_interval_ms: p.service_loop_interval_ms,
        engine_font_add: p.engine.font_add,
        gear: (p.gear.font_add, p.gear.show_edge),
        axis: (p.axis.font_add, p.axis.show_edge),
        attitude: (
            p.attitude.width,
            p.attitude.height,
            p.attitude.show_direction,
            p.attitude.show_aoa_limits,
        ),
        attitude_freq_ms: p.attitude_freq_ms,
        fm_font_add: p.fm.font_add,
        logical_height: 1080,
    }
}

/// refresh 闭包的字体路径 (捕获 env 不可 Clone 的 PathBuf 重取)
fn refresh_font_path(_doc: &vm_core::config::json_model::PageDoc) -> std::path::PathBuf {
    crate::env::Env::probe(&Lang::init_lang(), false)
        .fonts_dir
        .join("sarasa-mono-sc-bold.ttf")
}

/// [`register_live_overlays`] 的装配上下文 (原 9 参收敛的参数包, 对位 Java
/// registerGameModeOverlays 的 this 域; host/handles 为可变目标不入包)
pub(crate) struct OverlayRegSetup<'a> {
    pub(crate) env: &'a Env,
    pub(crate) inputs: &'a OverlayInputs,
    /// WYSIWYG reinit 参数仓 (CONFIG_CHANGED 后 ReinitOverlays 命令覆写;
    /// 各 spec 工厂 reinit 闭包持引用读取 — 见 vm-overlay reinit.rs 头注)
    pub(crate) params: &'a Rc<RefCell<vm_overlay::platform::reinit::ReinitParams>>,
    pub(crate) lang: &'a Rc<Lang>,
    pub(crate) shared: &'a ControllerShared,
}

/// 单条窗口 overlay 的注册样板收敛 (原 9 段同构 match 的公共面):
/// spec 工厂 → Ok: [`ControllerShared::note_registered_overlay`] 落键 +
/// `host.register(spec).with_interest(keys)` / Err: error 日志。
/// 返回句柄 Option (None = 工厂失败, 喂入跳过 — 见 [`OverlayHandles`] 头注),
/// 调用点直接落对应槽位; `keys` 空集 = 无追加键 (对位不调 with_interest)
fn register_one<H>(
    host: &mut OverlayHost,
    shared: &ControllerShared,
    label: &str,
    keys: &[&str],
    factory: impl FnOnce() -> Result<(H, OverlaySpec), String>,
) -> Option<H> {
    match factory() {
        Ok((h, spec)) => {
            shared.note_registered_overlay(&spec.id);
            host.register(spec).with_interest(keys);
            Some(h)
        }
        Err(e) => {
            logger::error("Controller", &format!("{} overlay 注册失败: {}", label, e));
            None
        }
    }
}

/// 托盘 handler: 动作转发主线程 (Java 托盘回调在 UI 事件线程, Rust 泵线程→channel)。
/// 关于项 (Java Application 的 about 菜单) 已接线: About → 主循环 emit → 前端 Modal。
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

/// 通道排空取最新 (live 数据合并: 对位 Java UI 线程 repaint 合并 — 只留最新帧)
fn drain_latest<T>(rx: &Receiver<T>) -> Option<T> {
    let mut latest = rx.try_recv().ok()?;
    while let Ok(next) = rx.try_recv() {
        latest = next;
    }
    Some(latest)
}


/// 全部窗口 overlay 的 live 喂入 (Java 各 overlay init(S) 时自订 FlightDataBus 的
/// 单点对位; Rust 订阅生命周期由 OpenAllOverlays/CloseAllOverlays 承载, 本函数在
/// 订阅期由渲染线程节拍调用, drain_latest 只留最新帧 = repaint 合并)。
///
/// PORT(preview 门控): Java preview 实例 (initPreview) 不订阅 FlightDataBus, 恒显
/// previewValue 静态; Rust host 单条目跨 open/refresh_preview 存活 (D8), 预览窗口
/// 形态 (overlay_ctx_preview=true, CloseAll/重建核置位 — 会话窗口形态语义) 喂入
/// 整帧跳过 — MiniHUD 此前的无条件喂入一并收口 (原 B-W3 备案族的收窄, 游戏内设
/// 置窗期不再渗 live 数据)。游戏稳态 (openpad 后) 恒 false, RefreshPreviews 不再
/// 翻转本标志 (Java refreshPreviews 对在场实例只调 reinitializer, 订阅不动)。
///
/// PORT(MiniHUD 喂入形态, W-B 事件瘦身后): 转发链只送 EventPayload (瘦事件,
/// 纯节拍+标量); state/indicators 取自 FrameStore 最近帧直传 hud_calculator
/// (按喂入时刻取最新值, 与 Java UI 线程读共享可变引用同一时序语义; 曾长期传
/// None/None 致襟翼/油门/姿态/G 值全 0 = "bar 恒 0" 根因); HUDData 由
/// minihud::update_from_event 现场计算 (hud_data 通道已删)。
///
/// PORT(B-W2 已兑现, 重构波4): 本函数原持 ServiceData 读锁跨纯计算段 (备形态
/// 已消) — 现取 FrameStore 不可变帧 (零锁), 各 update 签名 &dyn FormulaView
/// 直收 &Frame。与 Java 的回退路径 (MiniHUDOverlay 在 UI 线程内直读 Service
/// 公开字段无锁计算) 同形态。
///
/// PORT(panic 边界): ServiceData 的保真 panic 点 (get_pitch/get_thrust 的空引擎
/// 数组索引, service_fields.rs 注) 在畸形 s_state (update 失败 pitch/thrust 未填)
/// 下可达 — Java NPE 由 AWT 的 UI 事件线程吞掉 (UI 存活), Rust 渲染线程 panic 会杀整个
/// host 泵, 故整帧 catch_unwind (AssertUnwindSafe: 状态可能半更新, 对位 Java
/// UI 线程半更新后吞 NPE 的形态), ERROR 留痕丢帧继续。
pub(crate) fn feed_overlays_live(
    handles: &OverlayHandles,
    payload: &EventPayload,
    shared: &ControllerShared,
    fm: &FMManager,
    settings: &HudSettingsSnapshot,
    lang: &Lang,
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
        // 1. MiniHUD (Java MiniHUDOverlay.onFlightData → UI 线程派发)
        if let Some(h) = handles.minihud.as_ref() {
            // W-B: State/Indicators 直接从共享 guard 借引用下传 (hud_calculator
            // 读 flaps/throttle/gear/airbrake/aoa/ny/姿态), 不再装箱重建事件
            // AoA 告警/状态色 = 全局仓 (Java HUDCalculator 每次计算
            // 直读 Application 静态; 曾传编译期常量冻结 — 审查轮 1-B)
            let gc = vm_overlay::render::palette::colors();
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
        // 2. W3 通用页 (six pages): UpdateEnv 统一组装 → 组件自取
        //    (节流闩在组件内; preview 门控已在函数头拦截)
        let empty_data = vm_core::derived::hud_data::HUDData::empty();
        let env = vm_overlay::widgets::UpdateEnv {
            data: &empty_data, // HUDData 是 minihud 页派生, 通用页不消费
            frame: Some(&*frame),
            fmdata,
            payload: Some(payload),
            // compressorStages 档位数 = Java FMManager.current().compressorStages
            compressor_stages: fm_handle.compressor_stages.as_ref().map(|v| v.len() as i32),
            now_ms: now,
            maneuver_len: 0,
            maneuver_ticks: Default::default(),
            lang: Some(lang),
        };
        for (_, page) in &handles.pages {
            page.borrow_mut().feed(&env);
        }
    }));
    if result.is_err() {
        logger::error(
            "Controller",
            "live 喂入帧 panic 已吞 (畸形数据帧, 对位 Java EDT NPE 吞), 帧丢弃继续",
        );
    }
}

/// 渲染线程入口 (D8 拓扑): OverlayHost 泵 + 托盘 + 热键事件消费。
///
/// PORT(热键拓扑豁免记录, hotkey.rs 头注 D8 偏差): WH_KEYBOARD_LL 钩子固化在
/// HotkeyManager 自管的独立钩子线程 (jnativehook 独立派发线程的保真形态);
/// D8 的"并入单泵"需 hotkey.rs 提供外部线程装钩入口 (未提供, 本批次不越文件改),
/// 豁免期内钩子事件经 channel 汇入本线程统一消费 — 与托盘/overlay 共享的
/// 泵约束 (安装线程需泵) 由钩子线程自泵满足, 行为面一致。
/// 跟踪项 (审查 B-W4): 豁免收口 = hotkey.rs 提供外部线程装钩入口; 且
/// FM_OVERLAY_TOGGLE 的发布线程从 Java 的钩子线程变为本渲染线程 (经
/// hotkey_rx 中转后 publish ui_bus) — DrawFrameSimpl/FMUnpacked 的订阅消费
/// (渲染节拍块) 已按此拓扑接线, 后续 DrawFrame (P6 批三) 照此办理。
pub fn render_thread_main(cfg: RenderThreadConfig) {
    let mut session = RenderSession::new(cfg);
    let mut last_render = Instant::now();
    loop {
        // 托盘消息泵 (创建线程亲和, tray.rs 头注)
        #[cfg(target_os = "windows")]
        {
            if let Some(t) = session.tray.as_mut() {
                t.pump();
            }
        }
        session.host.pump_events();
        // 渲染节拍 50ms (Java FieldOverlay.onFlightData 50ms 节流, host.run 同款)
        if last_render.elapsed() >= Duration::from_millis(50) {
            last_render = Instant::now();
            let tick_active = match session.host.render_tick() {
                Err(e) => {
                    logger::error("OverlayHost", &format!("render_tick: {}", e));
                    Vec::new()
                }
                Ok(()) => session.host.active_ids(),
            };
            if !tick_active.is_empty() {
                // present 帧数代理计数 (见 ControllerShared.render_frames 注)
                session.shared.render_frames.fetch_add(1, Ordering::SeqCst);
                // 逐窗计数 (见 ControllerShared.overlay_present 注): 注册面 0 落键,
                // 此处在场即 +1 — 从未激活的注册项保留 0, 冒烟断言可判
                let mut counts = session
                    .shared
                    .overlay_present
                    .lock()
                    .expect("overlay_present 锁中毒");
                for id in &tick_active {
                    // 波22: 命中路径免 id.clone (entry 强制 owned key)
                    if let Some(c) = counts.get_mut(id) {
                        *c += 1;
                    } else {
                        counts.insert(id.clone(), 1);
                    }
                }
            }
            // live 数据喂入 (只留最新帧; preview 期整帧跳过 — feed_overlays_live 门控)
            if let Some(payload) = drain_latest(&session.flight_rx) {
                feed_overlays_live(
                    &session.handles,
                    &payload,
                    &session.shared,
                    &session.fm,
                    &session.hud_settings,
                    &session.lang,
                );
            }
            // FM 事件面 (恒排空防积压跨会话误触发) → 脉冲收集,
            // 由 sidecar tick 在本节拍内消费 (W3C: 组件自管数据面)
            let fm_live = !session.shared.overlay_ctx_preview.load(Ordering::SeqCst);
            let mut toggle_pulse = false;
            while session.fm_toggle_rx.try_recv().is_ok() {
                toggle_pulse = true;
            }
            let mut fm_changed: Option<Arc<vm_core::fm::data::FmData>> = None;
            while let Ok(fmdata) = session.fm_data_rx.try_recv() {
                if fm_live {
                    fm_changed = fmdata.map(Arc::new);
                }
            }
            if toggle_pulse
                || fm_changed.is_some()
                || session.host.is_active("enableFMPrint")
                || session.host.is_active("thrustdFS")
            {
                let display_fm_key = session
                    .shared
                    .flags
                    .lock()
                    .expect("flags 锁中毒")
                    .current_fm_hotkey_code;
                let live_frame = session
                    .shared
                    .live
                    .read()
                    .expect("live 锁中毒")
                    .as_ref()
                    .and_then(|frames| frames.latest());
                let now_ms = current_time_millis();
                let fm_field = session.fm_field_snapshot.clone();
                for (page_id, page) in &session.handles.pages {
                    if !matches!(page_id.as_str(), "fm-list-default" | "thrust-chart-default") {
                        continue;
                    }
                    let entry = if page_id == "fm-list-default" {
                        "enableFMPrint"
                    } else {
                        "thrustdFS"
                    };
                    if !session.host.is_active(entry) {
                        continue; // 条目未激活 (Java 无实例 = host 槽位空)
                    }
                    let fm_field_ref = &fm_field;
                    let mut sctx = vm_overlay::widgets::SidecarCtx {
                        now_ms,
                        page_id,
                        fm: &session.fm,
                        fm_field_config: &move |k: &str| -> Option<String> {
                            fm_field_ref
                                .lock()
                                .ok()
                                .and_then(|m| m.get(k).cloned())
                        },
                        display_fm_key,
                        frame: live_frame.as_deref().map(|f| f as &dyn FormulaView),
                        is_jet: false, // jetOnly 策略由 host 激活探测承载
                        toggle_pulse,
                        game_mode_pulse: session.fm_game_mode_pending,
                        fm_changed: fm_changed.clone(),
                    };
                    // 逐组件 tick (fm-list 字段原子化: 每字段/文本行一组件, 各自
                    // 200ms 自节流 + 段开关/FM 取值; 动作只可能出自推力曲线 —
                    // 原子组件恒 None)
                    let mut action = vm_overlay::widgets::SidecarAction::None;
                    {
                        let page_ref = page.borrow();
                        for cell in page_ref.cells.values() {
                            let Some(mut sc) = cell.sidecar() else { continue };
                            let a = sc.tick(&mut sctx);
                            if a != vm_overlay::widgets::SidecarAction::None {
                                action = a;
                            }
                        }
                    }
                    // fm-list 页编排面 (原子组件无整窗语义, 由本节拍承担):
                    // 热键切换/游戏形态隐藏起步 (原 FmUnpacked 自管 visible 的
                    // 页面级承接) + 包围盒高度跟随 (替代行数滞回 Resize, 无滞回;
                    // 屏高钳制 = 原 adjustPosition 上限)
                    if page_id == "fm-list-default" {
                        if session.fm_game_mode_pending {
                            session.fm_list_visible = false; // 游戏形态隐藏起步
                        }
                        if toggle_pulse {
                            session.fm_list_visible = !session.fm_list_visible;
                        }
                        let preview_mode =
                            session.shared.overlay_ctx_preview.load(Ordering::SeqCst);
                        let want = if preview_mode {
                            true // preview 恒显 (原 isPreview 语义)
                        } else {
                            session.fm_list_visible
                        };
                        session.host.set_entry_visible(entry, want);
                        let sized = page.borrow_mut().refresh_sizing(0);
                        if let (Some(cur), Some((w, h))) =
                            (session.host.entry_size(entry), sized)
                        {
                            let h = h.min(session.fm_list_max_h);
                            if cur != (w, h) {
                                let _ = session.host.resize_entry(entry, w, h);
                            }
                        }
                    }
                    match action {
                        vm_overlay::widgets::SidecarAction::None => {}
                        vm_overlay::widgets::SidecarAction::Resize(w, h) => {
                            let _ = session.host.resize_entry(entry, w, h);
                        }
                        vm_overlay::widgets::SidecarAction::SetVisible(v) => {
                            session.host.set_entry_visible(entry, v);
                        }
                        vm_overlay::widgets::SidecarAction::SetVisibleResize(v, w, h) => {
                            session.host.set_entry_visible(entry, v);
                            let _ = session.host.resize_entry(entry, w, h);
                        }
                        vm_overlay::widgets::SidecarAction::Close => {
                            let _ = session.host.close(entry);
                        }
                    }
                }
                session.fm_game_mode_pending = false;
            }
        }
        // UI 命令 (生命周期/WYSIWYG 的渲染线程属主面; 重分支见 RenderSession::on_*)
        while let Ok(cmd) = session.ui_cmd_rx.try_recv() {
            match cmd {
                UiCommand::OpenAllOverlays => session.on_open_all(),
                UiCommand::CloseAllOverlays => session.on_close_all(),
                UiCommand::RefreshPreviews {
                    changed_key,
                    generation,
                } => session.on_refresh_previews(changed_key, generation),
                UiCommand::ReinitActiveOverlays => session.host.reinit_active_overlays(),
                // WYSIWYG reinit 参数仓覆写 (不直接触发刷新 — 后继
                // RefreshPreviews/ReinitActiveOverlays 消费最新参数, 命令入队序
                // 即消费序; MiniHUD live 喂入快照与地平仪节流同步解冻)
                UiCommand::ReinitOverlays { params: new_params } => {
                    session.on_reinit_overlays(new_params)
                }
                // 全局五色更新: 仓内直写, 下帧渲染生效 (reinit 标脏不必须 —
                // 色变本身改变渲染输出, host 像素指纹自然触发重绘)
                UiCommand::SetGlobalColors(c) => vm_overlay::render::palette::set(c),
                UiCommand::SetAa(on) => vm_overlay::render::palette::set_aa(on),
                // FocusMonitor 通道桥目标 (Java hideAllOverlays/showAllOverlays;
                // host 幂等标志防重复, shared 镜像供桥回读)
                UiCommand::HideAllOverlays => {
                    session.host.hide_all_overlays();
                    session.shared.overlays_hidden.store(true, Ordering::SeqCst);
                }
                UiCommand::ShowAllOverlays => {
                    session.host.show_all_overlays();
                    session
                        .shared
                        .overlays_hidden
                        .store(false, Ordering::SeqCst);
                }
                UiCommand::Shutdown => {
                    logger::info("AppShell", "渲染线程退出 (Shutdown)");
                    // Drop 序: return 后 session 按字段声明序销毁 — flight_sub (退订)
                    // → tray (NIM_DELETE 防僵尸 — tray.rs 退出契约) → … → host
                    // (窗口销毁最后)。字段序即为此保持 (见 RenderSession 头注),
                    // 调整序前必读
                    return;
                }
                // 主线程属主命令不经本通道 (UiCommand 文档); 防御性忽略
                UiCommand::StartGame | UiCommand::EndGame => {}
            }
        }
        // 热键事件 (钩子线程 → 本线程统一消费; Java jnativehook 派发线程直发 UIStateBus)
        while let Ok(hk) = session.hotkey_rx.try_recv() {
            session.ui_bus.publish(
                &hk.event_type,
                Some("HotkeyManager"),
                Some(&hk.key_code.to_string()),
            );
        }
        std::thread::sleep(Duration::from_millis(10)); // 事件泵 10ms (host.run 同款)
    }
}

/// 渲染线程会话: [`render_thread_main`] 的装配产物 + 命令处理面
/// (原内联在入口函数的部件归拢, 重构波15)。
///
/// ⚠ **Drop 序契约**: Shutdown return 后 session 按字段声明序销毁 — "序敏感段"
/// 的字段序 = 原入口函数局部声明的**逆序**销毁链 (函数局部逆序 drop ↔ struct
/// 字段正序 drop 的等价换算): feed 泵 → 订阅 → 中转通道 → voice_warn (停告警
/// 线程) → flight_sub (live 订阅退订) → tray (NIM_DELETE 防僵尸, tray.rs 退出
/// 契约) → 数据态/Rc → **host (窗口销毁最后)**。调整字段序前必读 Shutdown
/// 分支注; 依赖段 (Arc 克隆/接收端) 无 Drop 契约。
struct RenderSession {
    // ---- 序敏感段 (字段序 = Drop 序, 见头注) ----
    /// FM_CHANGED 订阅 (RAII 保活: 持有即订阅, Drop 即退订 — 原局部
    /// `_fm_changed_sub` 同义; 载荷经通道中转, 句柄本身不读)
    #[allow(dead_code)] // RAII 订阅句柄, 生命周期面 (序敏感段成员)
    fm_changed_sub: Subscription<FMHandle>,
    /// FM_CHANGED 中转通道接收端 (载荷 = blkx 深拷)
    fm_data_rx: Receiver<Option<vm_core::fm::data::FmData>>,
    /// FM show* 开关快照 (sidecar tick 的 fm.field 段开关读面; 注册期借用之外的自持)
    fm_field_snapshot: Arc<Mutex<HashMap<String, String>>>,
    /// fm-list 页窗口高度上限 (屏幕逻辑高 — 原 adjustPosition 钳制语义,
    /// refresh_sizing 包围盒收敛时套用)
    fm_list_max_h: i32,
    /// fm-list 页整窗可见态 (热键切换; 游戏形态隐藏起步 — 原 FmUnpacked
    /// 自管 visible 的页面级承接)
    fm_list_visible: bool,
    /// openpad 的 FM 会话脉冲 (on_open_all 置位, 下一节拍 sidecar 消费后清除)
    fm_game_mode_pending: bool,
    /// FM_OVERLAY_TOGGLE 订阅 (RAII 保活, 同上; 热键切换经通道中转)
    #[allow(dead_code)] // RAII 订阅句柄, 生命周期面 (序敏感段成员)
    fm_toggle_sub: Subscription<UiStateEvent>,
    /// FM_OVERLAY_TOGGLE 中转通道接收端
    fm_toggle_rx: Receiver<()>,
    /// VoiceWarning 会话槽 (非窗口条目: openAll 建 / closeAll 停)
    voice_warn: Option<VoiceWarnSession>,
    /// live 数据转发通道接收端 (喂入侧 drain_latest 只留最新帧)
    flight_rx: Receiver<EventPayload>,
    /// live 数据转发通道发送端 (live 订阅闭包持克隆)
    flight_tx: Sender<EventPayload>,
    /// live 订阅槽 (OpenAll 建立 / CloseAll 撤销, 对位 overlay 订阅生命周期)
    flight_sub: Option<Subscription<FlightDataEvent>>,
    /// 托盘 (创建线程亲和; None = 创建失败继续运行)
    #[cfg(target_os = "windows")]
    tray: Option<TrayIcon>,
    /// live 喂入用设置快照 (ReinitOverlays 命令同步覆写)
    hud_settings: HudSettingsSnapshot,
    /// WYSIWYG reinit 参数仓 (各 spec 工厂 reinit 闭包读取)
    params: Rc<RefCell<vm_overlay::platform::reinit::ReinitParams>>,
    /// 标签源 (GearFlaps update_tick / engine reinit 闭包共用; Lang !Clone)
    lang: Rc<Lang>,
    /// 注册成功的 overlay 数据句柄全集
    handles: OverlayHandles,
    /// overlay 宿主 (注册/激活/渲染/开收窗)
    host: OverlayHost,
    // ---- 依赖段 (命令处理/循环所需; Drop 序无契约) ----
    /// 运行环境快照 (sync_page_entries 动态注册用户页需要 fonts_dir/dpi)
    env: crate::env::Env,
    /// UI 命令接收端 (主线程/桥发送)
    ui_cmd_rx: Receiver<UiCommand>,
    /// 热键事件接收端 (钩子线程发送)
    hotkey_rx: Receiver<HotkeyEvent>,
    /// Application.debug (Env 快照, 激活探测 ctx 用)
    debug: bool,
    ui_bus: Arc<UIStateBus>,
    flight_bus: Arc<FlightDataBus>,
    fm: Arc<FMManager>,
    shared: Arc<ControllerShared>,
    /// 激活缓存 (本身即 Arc, cfg 同源克隆)
    activation: ActivationCache,
    voice: Arc<VoiceResourceManager>,
    /// 配置跨线程快照对 (voice_* = VoiceWarning reload 读面; E9b 收敛)
    snapshots: ConfigSnapshots,
}

impl RenderSession {
    /// 会话装配 (原 render_thread_main 装配段整段原序搬迁: 五色注入 → host/
    /// 激活探测 → 注册面 → 设置快照 → 托盘 → 订阅与泵; 字段序 = Drop 序)
    fn new(cfg: RenderThreadConfig) -> Self {
        let RenderThreadConfig {
            env,
            inputs,
            ui_bus,
            flight_bus,
            fm,
            shared,
            activation,
            voice,
            snapshots,
            ui_cmd_rx,
            hotkey_rx,
            main_event_tx,
            position_snapshot,
        } = cfg;

        // 全局五色注入 (Java Application.colorNum 族静态的运行时值; cfg 经
        // loadFromConfig 覆盖, 此前组件用编译期 Java 初始默认 — 人工验收发现的
        // 颜色不一致根源)。须先于任何组件渲染
        vm_overlay::render::palette::set(inputs.colors);
        vm_overlay::render::palette::set_aa(inputs.aa);

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
            pages: Vec::new(),
        };
        // Lang 一次构造 (GearFlaps update_tick 的标签源; 注册面与喂入共用)。
        // Rc 共享: engine 工厂的 reinit 闭包重建 state 需要标签源 (Lang !Clone)
        let lang = Rc::new(Lang::init_lang());
        // WYSIWYG reinit 参数仓 (初始 = 注册快照投影; CONFIG_CHANGED 后
        // UiCommand::ReinitOverlays 覆写, 各 spec 工厂 reinit 闭包读取)
        let params = Rc::new(RefCell::new(
            vm_overlay::platform::reinit::ReinitParams::from(&inputs),
        ));
        register_live_overlays(
            &mut host,
            &mut handles,
            &OverlayRegSetup {
                env: &env,
                inputs: &inputs,
                params: &params,
                lang: &lang,
                shared: &shared,
            },
        );
        // live 喂入用设置快照 (注册面同源; ReinitOverlays 命令同步覆写 — MiniHUD
        // on_flight_data 的 settings 参数不再冻结在 spawn 时刻)
        let hud_settings = inputs.hud;
        // 地平仪 40ms 喂入节流 (freqMili 配置快照; last_ms=0 = 首帧放行;
        // ReinitOverlays 命令同步刷新 freq_ms)
        // ---- 托盘 (Java initSystemTray: 失败继续运行) ----
        #[cfg(target_os = "windows")]
        let tray = {
            let handler = AppTrayHandler { tx: main_event_tx };
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
        let flight_sub: Option<Subscription<FlightDataEvent>> = None;
        let (flight_tx, flight_rx) = std::sync::mpsc::channel::<EventPayload>();
        // VoiceWarning 会话槽 (非窗口 overlay 条目: Java registerWithStrategy 的
        // needsThread 形态 — openAll 建/closeAll 停; 激活策略与窗口条目同源探测)
        let voice_warn: Option<VoiceWarnSession> = None;

        // ---- FMUnpackedData 事件订阅 (Java FMUnpackedDataOverlay.init 的两处
        // UIStateBus.subscribe: FM_OVERLAY_TOGGLE 翻转 / FM_CHANGED reload;
        // overlays_field2.rs 头注契约 "由组装层的事件循环驱动" — 句柄 !Send (Rc),
        // 经 channel 中转到本循环消费; 订阅句柄随线程 Drop = Java dispose 退订链) ----
        let (fm_toggle_tx, fm_toggle_rx) = std::sync::mpsc::channel::<()>();
        let fm_toggle_sub = ui_bus.subscribe(ui_state_events::FM_OVERLAY_TOGGLE, move |_ev| {
            let _ = fm_toggle_tx.send(());
        });
        // FM_CHANGED 载荷 = FMHandle (fm_manager 强类型总线, Java instanceof 过滤由
        // 类型免除)。blkx 深拷一次进通道 (FMHandle.blkx 值字段 → 句柄侧 Arc<Blkx>;
        // 换机事件低频, 成本可忽略)
        let (fm_data_tx, fm_data_rx) =
            std::sync::mpsc::channel::<Option<vm_core::fm::data::FmData>>();
        let fm_changed_sub = fm.fm_changed_bus().subscribe(move |h| {
            let _ = fm_data_tx.send(h.fmdata.clone());
        });
        // FM 两页的数据泵已组件化 (WidgetSidecar tick, 渲染节拍驱动)

        let env_debug = env.debug; // env 字段 move 前先取标量
        Self {
            fm_changed_sub,
            fm_data_rx,
            fm_field_snapshot: snapshots.fm_field.clone(),
            fm_list_max_h: env.dpi.get_logical_screen_height(),
            fm_list_visible: false,
            fm_game_mode_pending: false,
            fm_toggle_sub,
            fm_toggle_rx,
            voice_warn,
            flight_rx,
            flight_tx,
            flight_sub,
            #[cfg(target_os = "windows")]
            tray,
            hud_settings,
            params,
            lang,
            handles,
            host,
            // ---- 依赖段 ----
            env,
            ui_cmd_rx,
            hotkey_rx,
            debug: env_debug,
            ui_bus,
            flight_bus,
            fm,
            shared,
            activation,
            voice,
            snapshots,
        }
    }

    /// 激活探测上下文 (Java OverlayContext 替身; 各命令点按需现场构造)
    fn vctx(&self) -> HostActivationCtx {
        HostActivationCtx {
            activation: Arc::clone(&self.activation),
            fm: Arc::clone(&self.fm),
            shared: Arc::clone(&self.shared),
            debug: self.debug,
        }
    }

    /// OpenAllOverlays 命令处理 (Java openpad → openAll): 会话翻游戏形态
    /// (句柄 is_preview/门控/句柄缓存) + host 开窗 + VoiceWarning 装配 +
    /// live 订阅重建
    fn on_open_all(&mut self) {
        // Java openpad → openAll; live 订阅随建 (overlay 订阅生命周期)。
        // P6 收口 (原审查 B-W3): live 喂入已覆盖全部 6 个窗口 overlay
        // (feed_overlays_live — MiniHUD/PowerInfo/EngineControl/GearFlaps/
        // ControlSurfaces/Attitude 共享句柄形态); FlightInfo 走 window.rs
        // 专径自接, thrustdFS 无 FlightDataBus 订阅 (事件面/run 泵在
        // 渲染节拍块驱动)。
        self.shared
            .overlay_ctx_preview
            .store(false, Ordering::SeqCst); // for_live (Java forGameMode)
                                             // 操纵面数据门控已随 W3 组件化消解 (组件按 env.frame 在场性自判)
        // FM 两页的游戏形态脉冲 (sidecar 下一节拍消费:
        // isPreview=false + 隐藏起步 + FM 缓存直读)
        self.fm_game_mode_pending = true;
        if let Err(e) = self.host.open_all() {
            logger::error("OverlayHost", &format!("open_all: {}", e));
        }
        // Java init 末尾 setVisible(false): 窗口隐藏起步 (热键切换),
        // 免首个 tick (≤200ms) 前的可见闪现
        self.host.set_entry_visible("enableFMPrint", false);
        // DrawFrameSimpl 同理 (init 末 setVisible(true) 后 run 首轮即
        // 隐藏 — Java 有 ≤1 线程轮的闪现, 此处同 FMUnpacked 先例预消)
        self.host.set_entry_visible("thrustdFS", false);
        // VoiceWarning (非窗口条目): Java openAll 对 enableVoiceWarn 走
        // 同一 OverlayEntry.open — 激活探测 (config+live_only, 此刻
        // preview 已翻 false = forGameMode ctx) 命中即 init(this,S) +
        // 起告警线程 (100ms tick + fatalWarn 回写)。幂等守卫对位
        // Java "instance != null 跳过"
        if self.voice_warn.is_none() {
            let vctx = self.vctx();
            if strategy_for("enableVoiceWarn").should_activate(&vctx) {
                let live = self.shared.live.read().expect("live 锁中毒").clone();
                match open_voice_warning(
                    &self.voice,
                    &self.ui_bus,
                    &self.snapshots.voice,
                    &self.fm,
                    &self.flight_bus,
                    live,
                ) {
                    Some(s) => {
                        logger::info("OverlayManager", "Started thread for: enableVoiceWarn");
                        self.voice_warn = Some(s);
                    }
                    None => logger::info(
                        "OverlayManager",
                        "Skipping open for enableVoiceWarn: no live Service",
                    ),
                }
            }
        }
        let tx = self.flight_tx.clone();
        let sub = self.flight_bus.register(move |ev: &FlightDataEvent| {
            // 转发线程 = Service 发布线程; 本闭包只 send 不碰 UI
            // (flight_data_bus.rs 重入死锁警戒的 channel 转发要求)
            let _ = tx.send(ev.get_payload().clone());
        });
        // 旧订阅 (如有) 显式 drop = unregister; 槽位持新订阅保活
        drop(self.flight_sub.replace(sub));
    }

    /// CloseAllOverlays 命令处理 (Java closeAll): 会话回预览态 + 数据面重置 +
    /// host 收窗 + live 订阅撤销 + VoiceWarning 停
    fn on_close_all(&mut self) {
        // 会话窗口形态回预览态 (审查 blocker 收口): Java closeAll → 实例
        // 销毁, 之后 refreshPreviews 重建的是 initPreview 实例 (无 live
        // 订阅); overlay_ctx_preview 的窗口形态门控在此复位, 防游戏会话
        // 结束后 preview 窗渗 live 残帧
        self.shared
            .overlay_ctx_preview
            .store(true, Ordering::SeqCst);
        // 数据面重置 (同上"实例销毁"语义的另一半): Java close 即实例
        // 死亡, preview 重开经工厂全新实例 + initPreview 静态值; Rust
        // handle 跨 close 存活 (render 闭包持同一 Rc), 不重置则下次
        // preview 窗渲染上次 live 残留值 (托盘 live→preview 复现)
        reset_handles_preview_values(&self.handles);
        self.host.close_all(); // close 销毁链 (存位置 → drop)
                               // Java overlay dispose → Bus.unregister (drop 槽位即退订)
        drop(std::mem::take(&mut self.flight_sub));
        // VoiceWarning 停 (Java OverlayEntry.close: interrupt 告警线程;
        // Drop 兜底 = doit 翻 false + join, 双订阅同时被退订)
        if self.voice_warn.is_some() {
            logger::info("OverlayManager", "Closing overlay: enableVoiceWarn");
            self.voice_warn = None;
        }
    }

    /// RefreshPreviews 命令处理 (WYSIWYG 预览刷新 + voice_warn 重估面);
    /// 过期世代直接返回 (原 continue — 分支尾即迭代尾, 等价)
    fn on_refresh_previews(&mut self, changed_key: Option<String>, generation: u64) {
        if is_stale_refresh(&self.shared, generation) {
            return; // 防过期守卫 (Java UI 线程派发内守卫的根治位)
        }
        // PORT(forPreviewMode 仅激活探测期, 审查 blocker 修复): Java
        // refreshPreviews 以 forPreviewMode ctx 判定激活,
        // 但对在场实例只调 reinitializer — 实例保留、
        // FlightDataBus 订阅不动, live 流持续。
        // 原实现把 overlay_ctx_preview 永久置 true: 游戏稳态 State=Preview
        // 下 FM_CHANGED/ConfigChanged 必经本分支 → feed_overlays_live
        // 永久 early-return, 全部 overlay 冻结在 FM 加载完成瞬间的值。
        // 改为激活探测期临时置 preview, 完毕恢复会话窗口形态
        // (openpad→false / CloseAll/重建核→true)。
        let session_preview = self.shared.overlay_ctx_preview.swap(true, Ordering::SeqCst);
        // ---- voice_warn 条目的 refreshPreview 重估面 (审查 W1 修复) ----
        // Java refreshPreviews 对触达条目调 entry.refreshPreview
        // (forPreviewMode ctx): shouldBeOpen = config &&
        // gameModeOnly, preview ctx 下 gameModeOnly=false → 在场
        // 即 close (关开关即时生效, Java configChanged → refreshPreviews
        // 在场条目重估)。Rust 原实现只走 host 窗口
        // 条目, voice_warn 无重估面 — 关掉开关后告警继续响到会话
        // 结束 (CloseAllOverlays 才停), 用户可感知偏差。补齐: 探测
        // 窗口内 (preview=true → live_only=false, 与 Java forPreviewMode
        // 下 gameModeOnly=false 同源) 对触达键在场即停。
        // 开方向不重建 — Java preview-ctx 下 shouldBeOpen 恒 false
        // 同样不 open (怪癖保真), 重起等下次 OpenAllOverlays。
        if self.voice_warn.is_some() && voice_warn_refresh_reaches(changed_key.as_deref()) {
            let vctx = self.vctx();
            if !strategy_for("enableVoiceWarn").should_activate(&vctx) {
                logger::info(
                    "OverlayManager",
                    "Closing overlay (inactive strategy): enableVoiceWarn",
                );
                self.voice_warn = None; // Drop → stop (doit 翻 false + join), 订阅退订兜底
            }
        }
        let r = match changed_key.as_deref() {
            Some(k) => self.host.refresh_preview_key(Some(k)),
            None => self.host.refresh_preview(),
        };
        self.shared
            .overlay_ctx_preview
            .store(session_preview, Ordering::SeqCst);
        if let Err(e) = r {
            logger::error("OverlayHost", &format!("refresh_previews: {}", e));
        }
    }

    /// ReinitOverlays 命令处理: WYSIWYG reinit 参数仓覆写 (不直接触发刷新) +
    /// 页面条目集对齐 (编辑器 save/delete 页 → CONFIG_CHANGED → 本命令)
    fn on_reinit_overlays(&mut self, new_params: Box<vm_overlay::platform::reinit::ReinitParams>) {
        // 地平仪节流已组件化 (AttitudeWidget 内闩, GaugeCfg.attitude_freq_ms 注入)
        self.hud_settings = new_params.hud.clone();
        *self.params.borrow_mut() = *new_params;
        // 用户页生命周期对齐: 新页注册落窗 / 消失的页注销摘窗 (出厂页恒定无操作)。
        // 借用拆分: env/lang/params/shared 均为独立字段, host/handles 可变独占
        let Self {
            host,
            handles,
            env,
            lang,
            params,
            shared,
            ..
        } = self;
        sync_page_entries(host, handles, env, lang, params, shared);
    }
}
