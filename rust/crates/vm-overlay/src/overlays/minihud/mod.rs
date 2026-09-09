//! minihud: MiniHUD 编排器 (PageDoc 驱动, W2 组件化改造)。
//! - ctx: MinimalHudContext + MiniHudFonts 上下文 (配置快照)
//! - 本文件: MiniHudOverlay 编排器 + minihud_overlay_spec 工厂
//! - 组件契约/注册表/建树: [`crate::widgets`] 域 (原 comp.rs 的枚举装配层退役)
//!
//! - [`MinimalHudContext`] — 不可变配置快照: 全部派生量 (字号/线宽/罗盘直径/
//!   rightDraw) 从 crossScale×dpiScale 级联; 字体 = 三份 BOLD 字号档。
//! - [`MiniHudOverlay`] — 编排器: refreshTemplates (preview 串) → 组件建树
//!   (widgets::build_page_layout, PageDoc 数据驱动) → 风格/模板注入 →
//!   外壳可见性 (配置组合门控) → 渲染循环 (doLayout+render+drawBlinkX)。
//! - [`minihud_overlay_spec`] — OverlayHost 挂载 (注册键 crosshairSwitch):
//!   render 闭包持共享句柄, 数据侧经 [`MiniHudHandle`] 外部喂入。
//!
//! 渲染循环 (Java paintComponent):
//! `engine.do_layout()` (惰性拓扑 + 锚点求解) → `engine.render(cb)` (可见节点按拓扑序
//! 逐个 `component.draw(g,x,y)`, debug 开启时紧跟 1px 调试框) → `draw_blink_x`
//! (致命警告 X, 压在 HUD 内容之上)。
//!
//! 零分配纪律 (手册 §11.4): draw 路径不 new — 字体/颜色经 [`MiniHudFonts`] Rc 共享,
//! 组件句柄 [`WidgetCell`] 克隆仅是引用计数。
//!
//! W2 裁决 (组件自治与编排器职责分界; 行族原子化后修订):
//! - 风格注入 → HudWidget::apply_style (组件自取); 行内细粒度开关已随
//!   复合行拆解退役 — show* 键改控原子件外壳 visible (下行第二则);
//! - 外壳 visible 的**组合门控** (drawHudText && enableFlapAngleBar 等跨键组合)
//!   → 编排器 update_component_visibility / update_row_visibility (cells[id]);
//! - preview 模板与静态值推送 → HudWidget::push_templates (MiniHudTemplates 值包);
//! - visibleWhen (displayCrosshair) → 建树门控 (widgets::page_layout, 不逐帧)。

mod ctx;

pub use ctx::{MiniHudFonts, MinimalHudContext};

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use vm_core::base::format::{fmt_f, pad_width};
use vm_core::base::event::event_payload::EventPayload;
use vm_core::config::config_api::{HUDSettings, HudSettingsSnapshot};
use vm_core::config::json_model::PageDoc;
use vm_core::derived::hud_calculator::{self, HudColors};
use vm_core::fm::data::FmData;
use vm_core::formula::registry::FormulaView;
use vm_core::game_api::parser::{Indicators, State};

use crate::layout::hud_layout_node::HUDLayoutNodeExt;
use crate::layout::minihud_layout::AutoSizingPlan;
use crate::overlays::rows::{TickScale, MANEUVER_FULL_SCALE, MANEUVER_TICK_STEPS};
use crate::overlays::spec_common::keyed_spec_id;
use crate::overlays::warning::WarningBlinkHost;
use crate::platform::host::{OverlaySpec, ReinitFn};
use crate::platform::reinit::ReinitParams;
use crate::render::canvas::PixCanvas;
use crate::render::palette::{aa, colors};
use crate::render::primitives;
use crate::widgets::{
    build_page_layout, FactoryCtx, MiniHudTemplates, PageBuildInputs, StyleEnv, UpdateEnv,
    WidgetCell,
};

// ---------------------------------------------------------------------------
// Java Math / printf 复刻
// ---------------------------------------------------------------------------

/// Java `(int) Math.round(double)`: round 返回 long, (int) 窄化取低 32 位
fn java_round_long_narrowed(x: f64) -> i32 {
    let l = (x + 0.5).floor() as i64;
    (l as u32) as i32
}

/// Java `String.format("%Nd", v)` = pad_width(十进制) 组合 (测试基线专用)
#[cfg(test)]
fn fmt_d(v: i32, width: usize) -> String {
    pad_width(v.to_string(), width, false)
}

// ---------------------------------------------------------------------------
// MiniHUDOverlay 编排器
// ---------------------------------------------------------------------------

/// overlay 的共享数据句柄 (host render 闭包与数据喂入方各持一份;
/// 单线程 RefCell — host 是主循环单线程独占, 上层 Controller 须同线程喂入)
pub type MiniHudHandle = Rc<RefCell<MiniHudOverlay>>;

/// MinimalHUD overlay for displaying compact flight information. (Java 类 javadoc 原文)
pub struct MiniHudOverlay {
    ctx: MinimalHudContext,
    /// 字体快照 (与 ctx.fonts 同源; reinit_config 重建 ctx 时同步换)
    fonts: Rc<MiniHudFonts>,
    /// 字体文件路径 (reinit_config 重建字体用)
    font_path: PathBuf,
    /// Application.dpiScale 参数注入 (LIFETIMES Env 只读快照)
    dpi_scale: f64,
    /// Java service 字段的在场语义 (null = 预览模式; 遥测数据经参数喂入)
    #[allow(dead_code)] // 保真保留: Java s != null 的在场标记 (数据面已参数化)
    service_present: bool,

    /// 组件句柄表 (id → cell; 建树产物, 编排器/数据面分发用 —
    /// 原 components 清单 + 具名字段的收敛)
    cells: HashMap<String, WidgetCell>,
    /// 布局引擎 + 自动尺寸计划 (build_page_layout 产物)
    layout: crate::widgets::BuiltPageLayout,

    // 0. Aux Overlays — warningOverlay 组合于 WarningBlinkHost (drawBlinkX 链)
    warning: WarningBlinkHost,

    // Java 遗留/只写字段 (§2.10 保真保留)
    /// refreshTemplates 的预览行串 (lines[5] 未用, Java 数组长 6 原样)
    lines: [String; 6],
    rel_energy: String,
    line_aoa: String,
    /// Java public int throttley (refreshTemplates 写 100; 无读者)
    throttley: i32,
    /// refreshTemplates 写 10 → init 钳 ctx.rightDraw; preview row0.update 入参
    aoa_y: i32,
    /// Java public Color throttleColor (写无读; Application.colorShadeShape)
    throttle_color: [u8; 4],
    aoa_color: [u8; 4],
    aoa_bar_color: [u8; 4],
    /// Java public boolean inAction (恒 false; row2 预览 update 入参)
    in_action: bool,
    /// Java private boolean disableAttitude (恒 false; 姿态仪可见性入参)
    disable_attitude: bool,
    /// Java private double realSpdPitch (死字段 — 全库无读写, 声明保真保留)
    #[allow(dead_code)] // PORT: Java MiniHUDOverlay 同名死字段
    real_spd_pitch: f64,
    /// Java private boolean firstDraw (reinitConfig 写 true; 无读者)
    first_draw: bool,
    /// Java public long hudCheckMili (死字段 — 全库无读写, 声明保真保留)
    #[allow(dead_code)] // PORT: Java MiniHUDOverlay 同名死字段
    hud_check_mili: i64,
    /// update_maneuver_session 更新 (原 update_legacy_components 尾段)
    maneuver_index: f64,
    maneuver_index_len: i32,
    /// 各档刻度距离 (原 len10..len50 五连字段收敛, 档位表在 rows.rs)
    tick_scale: TickScale,

    // Java public boolean warnRH / warnVne (updateFromEvent 写; 外层消费)
    pub warn_rh: bool,
    pub warn_vne: bool,

    // Throttling for refresh rate
    refresh_interval: i64,
    last_refresh_time: i64,
}

impl MiniHudOverlay {
    /// Java init(Controller c, Service s, HUDSettings settings) + W2 页面参数。
    /// `service_present` = (s != null); 遥测经 [`on_flight_data`] 参数喂入。
    pub fn init<S: HUDSettings>(
        service_present: bool,
        service_loop_interval_ms: i64,
        settings: &S,
        dpi_scale: f64,
        font_path: &Path,
        doc: &PageDoc,
    ) -> Result<Self, String> {
        vm_core::base::logger::info("MinimalHUD", "init called");
        let ctx = MinimalHudContext::create(settings, dpi_scale, font_path)?;
        vm_core::base::logger::info(
            "MinimalHUD",
            &format!(
                "MinimalHUD Config: Width={}, Height={}, CrossWidth={}",
                ctx.width, ctx.height, ctx.cross_scale
            ),
        );
        let fonts = Rc::new(ctx.fonts.clone());
        let mut overlay = MiniHudOverlay {
            fonts,
            font_path: font_path.to_path_buf(),
            dpi_scale,
            service_present,
            cells: HashMap::new(),
            layout: crate::widgets::BuiltPageLayout::empty(ctx.width, ctx.height),
            warning: WarningBlinkHost::new(service_loop_interval_ms),
            lines: std::array::from_fn(|_| String::new()),
            rel_energy: String::new(),
            line_aoa: String::new(),
            throttley: 0,
            aoa_y: 0,
            throttle_color: colors().shade_shape,
            aoa_color: colors().num,
            aoa_bar_color: colors().num,
            in_action: false,
            disable_attitude: false,
            real_spd_pitch: 0.0,
            first_draw: true,
            hud_check_mili: 0,
            maneuver_index: 0.0,
            maneuver_index_len: 0,
            tick_scale: TickScale::default(),
            warn_rh: false,
            warn_vne: false,
            refresh_interval: service_loop_interval_ms,
            last_refresh_time: 0,
            ctx,
        };

        overlay.reinit_config(settings, doc)?;

        if overlay.aoa_y > overlay.ctx.right_draw {
            overlay.aoa_y = overlay.ctx.right_draw;
        }
        overlay.aoa_color = colors().num;
        overlay.aoa_bar_color = colors().num;

        Ok(overlay)
    }

    /// Java reinitConfig() — ctx 快照重建 + 模板 + 组件建树 + 风格注入。
    pub fn reinit_config<S: HUDSettings>(
        &mut self,
        settings: &S,
        doc: &PageDoc,
    ) -> Result<(), String> {
        vm_core::base::logger::info("MinimalHUD", "reinitConfig called");

        // Create Immutable Context
        self.ctx = MinimalHudContext::create(settings, self.dpi_scale, &self.font_path)?;
        self.fonts = Rc::new(self.ctx.fonts.clone());
        vm_core::base::logger::info(
            "MinimalHUD",
            &format!(
                "MinimalHUD Config: Width={}, Height={}, CrossWidth={}",
                self.ctx.width, self.ctx.height, self.ctx.cross_scale
            ),
        );

        // 1. Refresh mock data and templates (WYSIWYG support)
        let templates = self.refresh_templates(settings);
        // 统一快照 (StyleEnv 的 settings 面与可见性门控共用)
        let snap = HudSettingsSnapshot::build(settings);

        // 2. 组件建树 (PageDoc 数据驱动; visibleWhen 门控在此)
        self.init_page_layout(&snap, doc, &templates);

        self.first_draw = true;
        // repaint() → 宿主 render_tick 标脏 (host 脏检查逐字节, 无需显式)
        Ok(())
    }

    /// Java refreshTemplates() — preview 串构造; W2 返回组件模板值包
    /// (推送统一在建树后, 原 set_row_templates 尾段并入 push_templates)。
    fn refresh_templates<S: HUDSettings>(&mut self, settings: &S) -> MiniHudTemplates {
        let t = preview_templates(
            settings,
            (self.maneuver_index, self.maneuver_index_len, self.tick_scale),
            self.in_action,
        );
        // self 会话缓存保留 (编排器状态面; lines = [String;6] 前 5 槽)
        self.lines[..5].clone_from_slice(&t.lines);
        self.line_aoa = t.line_aoa.clone();
        self.rel_energy = t.rel_energy.clone();
        self.aoa_y = t.aoa_y;
        self.throttley = 100;
        self.throttle_color = colors().shade_shape;
        self.aoa_color = t.aoa_color;
        self.aoa_bar_color = t.aoa_bar_color;
        t
    }

    /// 建树 + 风格/模板注入 + 外壳可见性 (原 init_components_layout +
    /// apply_style_to_components + update_components 的编排段, W2 数据驱动)。
    fn init_page_layout(
        &mut self,
        settings: &HudSettingsSnapshot,
        doc: &PageDoc,
        templates: &MiniHudTemplates,
    ) {
        let fctx = FactoryCtx {
            minihud_ctx: Some(&self.ctx),
            fonts: Rc::clone(&self.fonts),
            lang: None,
            fonts_dir: None,
            gauge_cfg: None,
        };
        // visibleWhen 求值源: HUDSettings 快照键 (displayCrosshair; W3 泛化
        // 为 config bool + 遥测短名)
        let visible_src = |k: &str| -> Option<bool> {
            match k {
                "displayCrosshair" => Some(settings.display_crosshair),
                _ => None,
            }
        };
        let show_crosshair = settings.display_crosshair;
        let layout_width = if show_crosshair {
            self.ctx.width * 2
        } else {
            self.ctx.width
        };
        let inputs = PageBuildInputs {
            doc,
            fctx: &fctx,
            visible_src: &visible_src,
            visible_default: false, // 整树缺失关准星的 Java getBool 兜底
            canvas_w: layout_width,
            canvas_h: self.ctx.height,
            // lineHeight from font size for responsive scaling (原注)
            line_height: self.ctx.hud_font_size as f64,
            debug: settings.bools.get("enableLayoutDebug").copied().unwrap_or(false),
        };
        self.layout = build_page_layout(&inputs);
        self.cells = std::mem::take(&mut self.layout.cells);

        // 风格注入 (组件细粒度开关在此 — apply_style 自取)
        let style = StyleEnv {
            fonts: Rc::clone(&self.fonts),
            settings,
            minihud_ctx: Some(&self.ctx),
        };
        for cell in self.cells.values() {
            cell.apply_style(&style);
            cell.push_templates(templates);
        }
        // 外壳可见性 (组合门控, 编排器职责)
        self.update_component_visibility(settings);
        self.update_row_visibility(settings);
    }

    /// updateComponents 仪表可见性段 (外壳 visible 的组合门控; cells 版)
    fn update_component_visibility<S: HUDSettings>(&mut self, settings: &S) {
        let text_visible = settings.draw_hud_text();
        let vis = |cells: &HashMap<String, WidgetCell>, id: &str, v: bool| {
            if let Some(c) = cells.get(id) {
                c.set_visible(v);
            }
        };
        let enable_flap_bar = settings.enable_flap_angle_bar();
        vis(&self.cells, "flap", text_visible && enable_flap_bar);
        let show_attitude = settings.show_attitude_gauge();
        vis(
            &self.cells,
            "compass",
            text_visible && !show_attitude,
        );
        vis(
            &self.cells,
            "attitude",
            text_visible && show_attitude && !self.disable_attitude,
        );
        // crosshair 的门控在建树 (visibleWhen), 此处幂等补设
        vis(
            &self.cells,
            "crosshair",
            settings.is_display_crosshair(),
        );
        let show_speed = settings.show_speed_bar();
        vis(&self.cells, "throttle", text_visible && !show_speed);
        vis(&self.cells, "speedBar", text_visible && show_speed);
    }

    /// updateComponents 行可见性段 (外壳 visible; 行族原子化后 show* 键
    /// 直控各原子件 — 原复合行的"行级 = 各段之或"随拆解消解为逐件门控)
    fn update_row_visibility<S: HUDSettings>(&mut self, settings: &S) {
        // Java: master = drawHudText() (保真保留)
        let master = settings.draw_hud_text();
        let vis = |cells: &HashMap<String, WidgetCell>, id: &str, v: bool| {
            if let Some(c) = cells.get(id) {
                c.set_visible(v);
            }
        };
        vis(&self.cells, "speed", master && settings.show_hud_speed());
        vis(&self.cells, "aoa", master && settings.show_hud_aoa());
        vis(&self.cells, "altitude", master && settings.show_hud_altitude());
        vis(&self.cells, "energy", master && settings.show_hud_energy());
        vis(&self.cells, "flaps", master && settings.show_hud_flaps());
        vis(&self.cells, "airbrake", master && settings.show_hud_airbrake());
        vis(&self.cells, "gear", master && settings.show_hud_gear());
        vis(&self.cells, "sep", master && settings.show_hud_sep());
        vis(&self.cells, "gload", master && settings.show_hud_g_load());
        vis(&self.cells, "maneuverbar", master && settings.show_hud_maneuver_bar());
    }

    // --- Event-Driven Update ---

    /// Java onFlightData(FlightDataEvent)。
    /// 返回 false = 节流跳过; true = 已进入 update_from_event。
    #[allow(clippy::too_many_arguments)]
    pub fn on_flight_data<S: HUDSettings>(
        &mut self,
        now_ms: i64,
        state: Option<&State>,
        indic: Option<&Indicators>,
        payload: &EventPayload,
        service: Option<&dyn FormulaView>,
        fmdata: Option<&FmData>,
        settings: &S,
        colors: &HudColors,
    ) -> bool {
        // 节流防高频事件任务堆积
        if now_ms - self.last_refresh_time < self.refresh_interval {
            return false;
        }
        self.last_refresh_time = now_ms;

        self.update_from_event(state, indic, payload, service, fmdata, settings, colors);
        true
    }

    /// Java updateFromEvent(FlightDataEvent)
    #[allow(clippy::too_many_arguments)]
    fn update_from_event<S: HUDSettings>(
        &mut self,
        state: Option<&State>,
        indic: Option<&Indicators>,
        payload: &EventPayload,
        service: Option<&dyn FormulaView>,
        fmdata: Option<&FmData>,
        settings: &S,
        colors: &HudColors,
    ) {
        let data =
            hud_calculator::calculate(state, indic, payload, service, fmdata, settings, colors);

        // Dispatch to Reactive Components (W2: trait 分发, 细粒度开关在组件内)
        let env = UpdateEnv {
            data: &data,
            frame: None, // HUDData 已含全部 minihud 派生量
            fmdata: None,
            payload: None,
            compressor_stages: None,
            now_ms: 0,
            maneuver_len: self.maneuver_index_len,
            maneuver_ticks: self.tick_scale,
            lang: None,
        };
        for cell in self.cells.values() {
            cell.on_data_update(&env);
        }

        // Update Global State
        self.warn_vne = data.warn_vne;
        self.warn_rh = data.warn_altitude;
        self.warning.set_blink_x(payload.fatal_warn);

        // maneuver 会话量 (原 update_legacy_components 尾段 — 更新在分发后,
        // Row4 用上一帧 len, 保真)
        self.maneuver_index = data.maneuver_index;
        let right_draw = self.ctx.right_draw;
        self.maneuver_index_len =
            java_round_long_narrowed(data.maneuver_index / MANEUVER_FULL_SCALE * right_draw as f64);
        self.tick_scale = TickScale {
            ticks: MANEUVER_TICK_STEPS.map(|step| {
                java_round_long_narrowed(step / MANEUVER_FULL_SCALE * right_draw as f64)
            }),
        };
    }

    /// Java paintComponent 主体: doLayout + render + drawBlinkX。
    pub fn draw(&mut self, cv: &mut PixCanvas, aa: bool) {
        {
            self.layout.engine.do_layout();
            let engine = &self.layout.engine;
            engine.render(|node, x, y, dbg| {
                match dbg {
                    None => {
                        let comp = node.borrow().component.clone();
                        comp.draw(cv, x, y, aa);
                    }
                    Some(color) => {
                        let r = node.get_pixel_rect();
                        primitives::ring1px(cv, x, y, r.width, r.height, color);
                    }
                }
            });
        }
        // drawBlinkX(g2d) — X 只盖 ctx.width × ctx.height
        let (w, h) = (self.ctx.width, self.ctx.height);
        self.warning.draw_blink_x(cv, w, h, aa);
    }

    /// 自动尺寸计划 (None = 组件空, 宿主保持初始尺寸)
    pub fn sizing(&self) -> Option<AutoSizingPlan> {
        self.layout.sizing
    }

    pub fn ctx(&self) -> &MinimalHudContext {
        &self.ctx
    }
}

/// preview 模板纯构造 (refresh_templates 的计算体; W4 编辑器快照复用)。
/// maneuver/in_action = 编排器会话量 (编辑器传缺省)。
pub fn preview_templates<S: HUDSettings>(
    settings: &S,
    maneuver: (f64, i32, TickScale),
    in_action: bool,
) -> MiniHudTemplates {
    let spd_pre = if settings.is_speed_label_disabled() {
        ""
    } else {
        "SPD"
    };
    let alt_pre = if settings.is_altitude_label_disabled() {
        ""
    } else {
        "ALT"
    };
    let sep_pre = if settings.is_sep_label_disabled() {
        ""
    } else {
        "SEP"
    };

    let mut lines: [String; 5] = std::array::from_fn(|_| String::new());
    if settings.draw_hud_mach() {
        // "M%5.2f" (0.85) — M 前缀在宽度域外
        lines[0] = format!("M{}", pad_width(fmt_f(0.85, 2), 5, false));
    } else {
        lines[0] = format!("{spd_pre}{}", pad_width("360".to_string(), 5, false));
    }
    // Format must match HUDCalculator: radar = "R%5.0f", barometric = "%6.0f"
    lines[1] = if settings.always_show_radar_altitude() {
        format!("{alt_pre}R{}", pad_width("1024".to_string(), 5, false))
    } else {
        format!("{alt_pre}{}", pad_width("1024".to_string(), 6, false))
    };
    // "↑%-4s"("30") — ↑ 是格式串字面量 (前缀, 不占 %-4s 宽度域)
    lines[3] = format!("{sep_pre}↑{}", pad_width("30".to_string(), 4, true));
    lines[4] = format!("G{}", pad_width("2.0".to_string(), 5, false));
    if settings.enable_flap_angle_bar() {
        lines[2] = pad_width(String::new(), 4, false); // "%4s"%""
    } else {
        lines[2] = format!("F{}", pad_width("100".to_string(), 3, false));
    }
    lines[2].push_str("BRK");
    lines[2].push_str("GEAR");

    MiniHudTemplates {
        lines,
        line_aoa: format!("α{}", pad_width(fmt_f(20.0, 0), 3, false)),
        rel_energy: "E114514".to_string(),
        aoa_y: 10,
        aoa_color: colors().num,
        aoa_bar_color: colors().num,
        in_action,
        throttle: 0, // update_components 的 service=None 分支值
        maneuver,
    }
}

// -------------------------------------------------------------------
// OverlayHost 挂载 (Controller registerWithPreview("crosshairSwitch"))
// ---------------------------------------------------------------------------

/// MiniHUD 的 OverlayHost 注册件: 返回 (共享句柄, spec)。
/// `doc` = 出厂页 (minihud-default; reinit 闭包从 ReinitParams.pages 重取最新)。
pub fn minihud_overlay_spec<S: HUDSettings>(
    service_present: bool,
    service_loop_interval_ms: i64,
    settings: &S,
    dpi_scale: f64,
    font_path: &Path,
    params: &Rc<RefCell<ReinitParams>>,
) -> Result<(MiniHudHandle, OverlaySpec), String> {
    // 页面来源: params 仓的最新 pages (首次注册 = OverlayInputs 组装的出厂页)
    let doc = {
        let p = params.borrow();
        p.pages
            .iter()
            .find(|d| d.id == "minihud-default")
            .cloned()
    };
    let Some(doc) = doc else {
        return Err("MiniHUD 出厂页缺失 (factory_default.json pages)".to_string());
    };
    let overlay = MiniHudOverlay::init(
        service_present,
        service_loop_interval_ms,
        settings,
        dpi_scale,
        font_path,
        &doc,
    )?;
    let (w, h) = match overlay.sizing() {
        Some(p) => (p.new_width, p.new_height),
        None => (overlay.ctx().width, overlay.ctx().height),
    };
    let handle: MiniHudHandle = Rc::new(RefCell::new(overlay));
    let render_handle = Rc::clone(&handle);
    let reinit_handle = Rc::clone(&handle);
    let reinit_params = Rc::clone(params);
    let reinit: ReinitFn = Box::new(move || {
        let (hud, doc) = {
            let p = reinit_params.borrow();
            (
                p.hud.clone(),
                p.pages
                    .iter()
                    .find(|d| d.id == "minihud-default")
                    .cloned(),
            )
        };
        let Some(doc) = doc else {
            vm_core::base::logger::warn("MinimalHUD", "reinit: 出厂页缺失, 保持旧状态");
            return None;
        };
        let mut o = reinit_handle.borrow_mut();
        if let Err(e) = o.reinit_config(&hud, &doc) {
            vm_core::base::logger::error("MinimalHUD", &format!("reinit_config 失败: {}", e));
            return None;
        }
        let (w, h) = match o.sizing() {
            Some(p) => (p.new_width, p.new_height),
            None => (o.ctx().width, o.ctx().height),
        };
        Some((w, h))
    });
    Ok((
        handle,
        keyed_spec_id(
            // R3: 条目键 = 页 id (激活/位置档统一); 激活键 = crosshairSwitch
            "minihud-default",
            "crosshairSwitch",
            w,
            h,
            Box::new(move |cv: &mut PixCanvas| {
                render_handle.borrow_mut().draw(cv, aa());
            }),
            Some(reinit),
        ),
    ))
}

#[cfg(test)]
mod tests;
