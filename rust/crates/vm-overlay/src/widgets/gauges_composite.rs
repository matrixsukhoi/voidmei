//! gauges_composite — W3B 图形族复合组件 (现有 overlay state 的黑盒包装)。
//!
//! 四组件: 引擎控制面板 / 起落襟翼状态 / 操纵面十字 / 独立地平仪窗。
//! 复合模式 = fields.grid 先例: Pipeline 包现有生产 state (overlays/*.rs 不动,
//! 仅消费其 pub 面), 数据/节流/绘制语义与旧 `*_overlay_spec` 工厂逐位同源;
//! 几何与字体构造期定 (reinit 整体重建)。
//!
//! 绘制走伴画布: state 在 (0,0) 内容区自绘 → `composite_straight_frame_at`
//! 整帧桥入页面 (满足 ControlSurfaces/Attitude 的画布尺寸防呆断言 —
//! 它们的窗口裁剪语义钉内容尺寸)。

use vm_core::fm::data::FmData;
use vm_core::formula::registry::FormulaView;

use crate::layout::hud_layout_node::Dimension;
use vm_core::base::format::java_round_f32;
use crate::overlays::attitude::AttitudeOverlay;
use crate::overlays::control_surfaces::{ControlSurfacesOverlay, CsFonts};
use crate::overlays::gear_flaps::GearFlapsState;
use crate::render::canvas::PixCanvas;
use crate::render::font::LoadedFont;

use super::env::{FactoryCtx, GaugeCfg, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::registry::PageFonts;
use super::registry::{HudWidget, PropSchema, WidgetCategory, WidgetMeta};

// =====================================================================
// 公共小件
// =====================================================================

/// gauge_cfg 缺席回退缺省 (preview/测试容忍; 页面编排器注入真值)
fn cfg_of(fctx: &FactoryCtx) -> GaugeCfg {
    fctx.gauge_cfg.cloned().unwrap_or_default()
}

/// bold 字体路径 (各旧工厂同款: fonts_dir/sarasa-mono-sc-bold.ttf)
fn bold_path(fctx: &FactoryCtx) -> Result<std::path::PathBuf, String> {
    Ok(fctx
        .fonts_dir
        .as_deref()
        .ok_or("复合组件需要 FactoryCtx.fonts_dir")?
        .join("sarasa-mono-sc-bold.ttf"))
}

/// regular 字体路径
fn regular_path(fctx: &FactoryCtx) -> Result<std::path::PathBuf, String> {
    Ok(fctx
        .fonts_dir
        .as_deref()
        .ok_or("复合组件需要 FactoryCtx.fonts_dir")?
        .join("sarasa-mono-sc-regular.ttf"))
}

/// attitude 几何 (旧 attitude 工厂 attitude_geom 同式):
/// base 宽高 × dpi 的 floor(x+0.5) + 开关族直通
fn attitude_geom(cfg: &GaugeCfg) -> (i32, i32, bool, bool) {
    let (w, h, dir, aoa) = cfg.attitude;
    (
        (w as f64 * cfg.dpi_scale + 0.5).floor() as i32,
        (h as f64 * cfg.dpi_scale + 0.5).floor() as i32,
        dir,
        aoa,
    )
}

/// 伴画布重绘后整帧桥入 (fields.grid 直通管线同款; 越界裁剪由桥入承载)
fn blit(cv: &mut PixCanvas, canvas: &mut PixCanvas, x: i32, y: i32, aa: bool) {
    let (w, h) = (canvas.width(), canvas.height());
    let frame = canvas.straight_frame();
    if !cv.composite_straight_frame_at(x, y, frame, w, h, aa) {
        vm_core::base::logger::warn("gauges_composite", "伴画布尺寸与缓冲不符, 本帧丢弃");
    }
}

// =====================================================================
// core.gearflaps.status — 起落架/襟翼状态
// =====================================================================

/// 起落襟翼复合组件 (包 [`GearFlapsState`])。
/// 构造参数口径 (旧 spec 工厂同源, W3 组件化继承):
/// 字号 = round((24+fontadd)×dpi), 边缘开关 sw=10,
/// fontNum = BOLD(fontSize) / fontLabel = BOLD(round(fontSize/2))。
pub struct GearFlapsWidget {
    state: GearFlapsState,
    font_num: LoadedFont,
    font_label: LoadedFont,
    canvas: PixCanvas,
    /// 构造参数留档 (reset_preview 重建 state — Java refreshPreview 新实例语义)
    build: (i32, f64, bool),
}

fn f_gear_flaps(
    _props: &serde_json::Value,
    fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    let cfg = cfg_of(fctx);
    let (font_add, show_edge) = cfg.gear;
    let state = GearFlapsState::new(font_add, cfg.dpi_scale, show_edge);
    let bold = bold_path(fctx)?;
    let font_num = LoadedFont::new(&bold, state.font_size)?;
    let font_label = LoadedFont::new(&bold, java_round_f32(state.font_size as f32 / 2.0))?;
    let canvas = PixCanvas::new(state.total_width, state.total_height)?;
    Ok(Box::new(GearFlapsWidget {
        state,
        font_num,
        font_label,
        canvas,
        build: (font_add, cfg.dpi_scale, show_edge),
    }))
}

impl GearFlapsWidget {
    /// 内部 state 只读借出 (测试断言面; 生产勿用)
    pub fn state(&self) -> &GearFlapsState {
        &self.state
    }
}

impl HudWidget for GearFlapsWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {
        // 预览初值已由 GearFlapsState::new 落位 (襟翼 50% 无告警)
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // preview (frame/lang 缺席) 保持静态 — 对位 feed_overlays_live 门控
        let (Some(frame), Some(lang)) = (env.frame, env.lang) else {
            return;
        };
        self.state.update_tick(env.now_ms, lang, frame);
    }

    fn reset_preview(&mut self) {
        // 数据面回构造初值 (flap 50%/告警清空/节流基准归零; Java 新实例等价)
        self.state = GearFlapsState::new(self.build.0, self.build.1, self.build.2);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        if !self
            .canvas
            .clear(self.state.total_width, self.state.total_height)
        {
            return;
        }
        self.state
            .draw(&mut self.canvas, &self.font_num, &self.font_label, aa);
        blit(cv, &mut self.canvas, x, y, aa);
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        Dimension::new(self.state.total_width, self.state.total_height)
    }
}

// =====================================================================
// core.axes.crosshair — 操纵面十字
// =====================================================================

/// 操纵面复合组件 (包 [`ControlSurfacesOverlay`])。
/// 构造参数口径 (旧 spec 工厂同源, W3 组件化继承):
/// init_preview 几何 + 三字体 (num=BOLD(fs) / label=BOLD(fs/2) / unit=PLAIN(fs/2)),
/// spec 尺寸口径 = 内容区 content_width×content_height (sw 边距不承载)。
pub struct AxesWidget {
    state: ControlSurfacesOverlay,
    fonts: (LoadedFont, LoadedFont, LoadedFont),
    canvas: PixCanvas,
}

fn f_axes(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let cfg = cfg_of(fctx);
    let (font_add, edge) = cfg.axis;
    let mut cs = ControlSurfacesOverlay::new();
    // win_x/win_y = 0: 定位归页面布局 (旧工厂同款)
    cs.init_preview(font_add, cfg.dpi_scale, edge, 0, 0);
    let bold = bold_path(fctx)?;
    let regular = regular_path(fctx)?;
    let fonts = (
        LoadedFont::new(&bold, cs.font_size)?,
        LoadedFont::new(&bold, cs.label_font_size)?,
        LoadedFont::new(&regular, cs.label_font_size)?,
    );
    let canvas = PixCanvas::new(cs.content_width, cs.content_height)?;
    Ok(Box::new(AxesWidget { state: cs, fonts, canvas }))
}

impl AxesWidget {
    /// 内部 state 只读借出 (测试断言面; 生产勿用)
    pub fn state(&self) -> &ControlSurfacesOverlay {
        &self.state
    }
}

impl HudWidget for AxesWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {
        // 预览初值 (50) 已由 init_preview 落位
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // has_service = 会话形态 (frame 有 = live; preview None 数据不更新)
        self.state.has_service = env.frame.is_some();
        let Some(_frame): Option<&dyn FormulaView> = env.frame else {
            return; // preview 保持静态
        };
        self.state.on_flight_data(
            env.now_ms,
            env.val("aileron"),
            env.val("elevator"),
            env.val("rudder"),
            env.val("wing_sweep"),
            env.val("wing_sweep_valid") != 0.0,
        );
    }

    fn reset_preview(&mut self) {
        self.state.reset_preview();
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        if !self
            .canvas
            .clear(self.state.content_width, self.state.content_height)
        {
            return;
        }
        let fonts = CsFonts {
            num: &self.fonts.0,
            label: &self.fonts.1,
            unit: &self.fonts.2,
        };
        self.state.draw(&mut self.canvas, &fonts, aa);
        blit(cv, &mut self.canvas, x, y, aa);
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        Dimension::new(self.state.content_width, self.state.content_height)
    }
}

// =====================================================================
// core.attitude.window — 独立地平仪窗
// =====================================================================

/// 地平仪窗复合组件 (包 [`AttitudeOverlay`])。
/// 构造参数口径 = 旧工厂的 attitude_geom (base 宽高 × DPI 缩放);
/// 数据节流从喂入侧收进组件 (freq = attitude_freq_ms, Java onFlightData
/// freqMili 语义)。
pub struct AttitudeWidget {
    state: AttitudeOverlay,
    /// 数据节流 ms + 基准 (原宿主 AttitudeFeedState 的组件化)
    freq_ms: i64,
    last_ms: i64,
    canvas: PixCanvas,
}

fn f_attitude_window(
    _props: &serde_json::Value,
    fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    let cfg = cfg_of(fctx);
    let (xw, xh, dir, aoa) = attitude_geom(&cfg);
    let mut state = AttitudeOverlay::new();
    state.reinit(xw, xh, dir, aoa);
    let canvas = PixCanvas::new(xw, xh)?;
    Ok(Box::new(AttitudeWidget {
        state,
        freq_ms: cfg.attitude_freq_ms,
        last_ms: 0,
        canvas,
    }))
}

/// aoa_limits = FM 翼数据 (NoFlapsWing.AoACritHigh/Low; 无 FM → None 不显示)
fn aoa_limits_of(fmdata: Option<&FmData>) -> Option<(f64, f64)> {
    fmdata
        .and_then(|b| b.no_flaps_wing.as_ref())
        .map(|w| (w.aoa_crit_high, w.aoa_crit_low))
}

impl AttitudeWidget {
    /// 内部 state 只读借出 (测试断言面; 生产勿用)
    pub fn state(&self) -> &AttitudeOverlay {
        &self.state
    }

    /// 组件内节流基准 (40ms 闩的 last_ms; 测试断言面 — 喂入是否被节流)
    pub fn last_ms(&self) -> i64 {
        self.last_ms
    }
}

impl HudWidget for AttitudeWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {
        // 预览初值 = 未飞形态 (drawTick 未跑), 构造即落位
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // 组件内节流 (原喂入侧 attitude_feed 的 freqMili 闩)
        if env.now_ms.saturating_sub(self.last_ms) < self.freq_ms {
            return;
        }
        self.last_ms = env.now_ms;
        // preview (frame 缺席) 保持静态 — 对位 feed_overlays_live 门控
        if env.frame.is_none() {
            return;
        }
        self.state.update_telemetry(
            env.val("aoa"),
            env.val("aos"),
            env.val("aviahorizon_pitch"),
            env.val("aviahorizon_roll"),
            env.val("compass"),
            aoa_limits_of(env.fmdata),
        );
    }

    fn reset_preview(&mut self) {
        self.state.reset_preview();
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        if !self.canvas.clear(self.state.x_width, self.state.x_height) {
            return;
        }
        self.state.draw(&mut self.canvas, aa);
        blit(cv, &mut self.canvas, x, y, aa);
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        Dimension::new(self.state.x_width, self.state.x_height)
    }
}

// =====================================================================
// 注册表
// =====================================================================

const EMPTY_PROPS: &[PropSchema] = &[];

/// 工厂速记 (const 上下文可用; composite 恒 true — 黑盒包整窗 state)
const fn meta(
    type_name: &'static str,
    display_zh: &'static str,
    category: WidgetCategory,
    config_keys: &'static [&'static str],
    data_shorts: &'static [&'static str],
    factory: super::registry::WidgetFactory,
) -> WidgetMeta {
    WidgetMeta {
        type_name,
        display_zh,
        category,
        composite: true,
        props_schema: EMPTY_PROPS,
        config_keys,
        data_shorts,
        factory,
    }
}

/// W3B 注册表 (顺序 = palette 展示序)
pub(super) const REGISTRY_ENTRIES: &[WidgetMeta] = &[
    meta(
        "core.gearflaps.status",
        "起落架/襟翼状态",
        WidgetCategory::Gauge,
        &["enablegearAndFlaps", "enablegearAndFlapsEdge", "fontSize"],
        &["gear", "flaps", "airbrake"],
        f_gear_flaps,
    ),
    meta(
        "core.axes.crosshair",
        "操纵面十字",
        WidgetCategory::Gauge,
        &["enableAxis", "enableAxisEdge", "fontSize"],
        &["aileron", "elevator", "rudder", "wing_sweep"],
        f_axes,
    ),
    meta(
        "core.attitude.window",
        "地平仪窗",
        WidgetCategory::Gauge,
        &[
            "enableAttitudeIndicator",
            "attitudeIndicator",
            "attitudeIndicatorFreqMs",
        ],
        &[
            "aoa",
            "aos",
            "aviahorizon_pitch",
            "aviahorizon_roll",
            "compass",
        ],
        f_attitude_window,
    ),
];
