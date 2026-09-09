//! gauges_composite — W3B 图形族复合组件 (现有 overlay state 的黑盒包装)。
//!
//! 两组件: 操纵面十字 / 独立地平仪窗。复合模式 = fields.grid 先例:
//! Pipeline 包现有生产 state (overlays/*.rs 不动, 仅消费其 pub 面),
//! 数据/节流/绘制语义与旧 `*_overlay_spec` 工厂逐位同源;
//! 几何与字体构造期定 (reinit 整体重建)。
//! 已拆解: 引擎控制面板 → widgets::engine_gauge 原子仪表;
//! 起落襟翼 → widgets::gear_flaps_atom (flapbar/warn);
//! 操纵面十字收缩为仅十字图 (BOS 标签行/方向舵条 → data.field +
//! widgets::axes_atom::rudderbar)。
//!
//! 绘制走伴画布: state 在 (0,0) 内容区自绘 → `composite_straight_frame_at`
//! 整帧桥入页面 (满足 ControlSurfaces/Attitude 的画布尺寸防呆断言 —
//! 它们的窗口裁剪语义钉内容尺寸)。

use vm_core::fm::data::FmData;
use vm_core::formula::registry::FormulaView;

use crate::layout::hud_layout_node::Dimension;
use crate::overlays::attitude::AttitudeOverlay;
use crate::overlays::control_surfaces::ControlSurfacesOverlay;
use crate::render::canvas::PixCanvas;

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
// core.axes.crosshair — 操纵面十字
// =====================================================================

/// 操纵面十字复合组件 (包 [`ControlSurfacesOverlay`], 拆解后仅十字图)。
/// 构造参数口径 (旧 spec 工厂同源, W3 组件化继承):
/// init_preview 几何; spec 尺寸口径 = 十字区 width×width (6fs 边长,
/// 右侧 BOS 标签列/底部方向舵条已拆为独立组件)。
pub struct AxesWidget {
    state: ControlSurfacesOverlay,
    canvas: PixCanvas,
}

fn f_axes(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let cfg = cfg_of(fctx);
    // R4 字号合一: 组增量退役 → 页面字号折回加量域 (init_preview 的 add 参数)
    let font_add = fctx.fonts.draw.size - 24;
    let mut cs = ControlSurfacesOverlay::new();
    // win_x/win_y = 0: 定位归页面布局 (旧工厂同款)
    cs.init_preview(font_add, cfg.dpi_scale, cfg.axis_show_edge, 0, 0);
    let canvas = PixCanvas::new(cs.width, cs.width)?;
    Ok(Box::new(AxesWidget { state: cs, canvas }))
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
        // has_service = 会话形态 (frame 有 = live; preview None 数据不更新)。
        // state 整包更新保留 (十字游标 px/py 消费 aileron/elevator;
        // rudder/wing_sweep 字段为 state 保真面, 绘制已拆至 rudderbar/data.field)
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
        if !self.canvas.clear(self.state.width, self.state.width) {
            return;
        }
        self.state.draw_crosshair(&mut self.canvas, aa);
        blit(cv, &mut self.canvas, x, y, aa);
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        Dimension::new(self.state.width, self.state.width)
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
        default_props: "{}",
        factory,
    }
}

/// W3B 注册表 (顺序 = palette 展示序)
pub(super) const REGISTRY_ENTRIES: &[WidgetMeta] = &[
    meta(
        "core.axes.crosshair",
        "操纵面十字",
        WidgetCategory::Gauge,
        &["enableAxis", "enableAxisEdge", "fontSize"],
        &["aileron", "elevator"],
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
