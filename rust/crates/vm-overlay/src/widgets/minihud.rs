//! MiniHUD 组件族注册 (W2: 原 MiniHudComponentInner 枚举分发的 trait 化;
//! 行族原子化: 复合行 row0/1/2/4 拆为单语义原子件)。
//!
//! impl 体 = 原 comp.rs 的各枚举臂 + mod.rs update_component/row_visibility
//! 的组件自治段。行内细粒度开关 (setShowSpeed 族) 随拆解退役 —
//! 开关语义 = 编排器对原子件外壳 visible 的门控 (update_row_visibility)
//! 或用户在页面里删组件。

use vm_core::base::format::pad_width;


use crate::layout::hud_layout_node::Dimension;
use crate::overlays::attitude::AttitudeIndicatorGauge;
use crate::overlays::bars::{FlapAngleBar, LinearGauge, SpeedRatioBar};
use crate::overlays::compass::CompassGauge;
use crate::overlays::crosshair::CrosshairGauge;
use crate::overlays::rows::{
    split_trim3, AltitudeReadout, AoaGauge, EnergyReadout, GLoadReadout, HUDTextRow, MechKind,
    MechPart, ManeuverBar, SepReadout, SpeedReadout,
};
use crate::render::canvas::PixCanvas;

use super::env::{FactoryCtx, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::registry::{HudWidget, PropSchema, WidgetCategory, WidgetMeta};
use super::registry::PageFonts;

// =====================================================================
// 行主读数 (HUDTextRow 包装; 数据槽/模板槽各異)
// =====================================================================

impl HudWidget for SpeedReadout {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        self.0.set_style(env.minihud_ctx.expect("minihud 族组件需 minihud_ctx").hud_font_size);
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        self.0.set_template(Some(&t.lines[0]));
        self.0.update(&t.lines[0], false);
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        self.0.update(&env.data.speed_str, env.data.warn_vne);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        self.0.draw(cv, x, y, &fonts.draw, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        let (w, h) = self.0.preferred_size(&fonts.draw);
        Dimension::new(w, h)
    }
}

impl HudWidget for AltitudeReadout {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        self.0.set_style(env.minihud_ctx.expect("minihud 族组件需 minihud_ctx").hud_font_size);
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        self.0.set_template(Some(&t.lines[1]));
        self.0.update(&t.lines[1], false);
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        self.0.update(&env.data.alt_str, env.data.warn_altitude);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        self.0.draw(cv, x, y, &fonts.draw, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        let (w, h) = self.0.preferred_size(&fonts.draw);
        Dimension::new(w, h)
    }
}

impl HudWidget for SepReadout {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        self.0.set_style(env.minihud_ctx.expect("minihud 族组件需 minihud_ctx").hud_font_size);
        // Row3 无行内细粒度开关 (可见性 = master, 编排器外壳门控)
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        self.0.set_template(Some(&t.lines[3]));
        self.0.update(&t.lines[3], false);
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // live: SEP (原 update_legacy_components 桥)
        self.0.update(&env.data.sep_str, false);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        self.0.draw(cv, x, y, &fonts.draw, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        let (w, h) = self.0.preferred_size(&fonts.draw);
        Dimension::new(w, h)
    }
}

impl HudWidget for GLoadReadout {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        self.0.set_style(env.minihud_ctx.expect("minihud 族组件需 minihud_ctx").hud_font_size);
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        self.0.set_template(Some(&t.lines[4]));
        self.0.update(&t.lines[4], false);
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // live: G 文字 (原 update_legacy_components 桥; is_warning 恒 false)
        self.0.update(&env.data.maneuver_state_str, false);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        self.0.draw(cv, x, y, &fonts.draw, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        let (w, h) = self.0.preferred_size(&fonts.draw);
        Dimension::new(w, h)
    }
}

// =====================================================================
// 行辅件 (AoaGauge / EnergyReadout / MechPart / ManeuverBar)
// =====================================================================

impl HudWidget for AoaGauge {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        let ctx = env.minihud_ctx.expect("minihud 族组件需 minihud_ctx");
        self.set_style(ctx.right_draw, ctx.line_width, ctx.aoa_length as i32);
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        // preview 值推送 (原 update_row_values 的 service 缺席分支)
        self.set_template(Some(&t.line_aoa));
        self.update(&t.line_aoa, t.aoa_y, t.aoa_color, t.aoa_bar_color);
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        let data = env.data;
        self.set_aoa_from_ratio(data.aoa_ratio);
        self.update(
            &data.aoa_str,
            self.aoa_y,
            data.aoa_color,
            data.aoa_bar_color,
        );
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        AoaGauge::draw(self, cv, x, y, &fonts.draw, &fonts.small, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        let (w, h) = AoaGauge::preferred_size(self, &fonts.small);
        Dimension::new(w, h)
    }
}

impl HudWidget for EnergyReadout {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        self.set_style(env.minihud_ctx.expect("minihud 族组件需 minihud_ctx").right_draw);
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        self.set_template(Some(&t.rel_energy));
        self.update(&t.rel_energy);
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        self.update(&env.data.energy_str);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        EnergyReadout::draw(self, cv, x, y, &fonts.draw, &fonts.small, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        let (w, h) = EnergyReadout::preferred_size(self, &fonts.small);
        Dimension::new(w, h)
    }
}

impl HudWidget for MechPart {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        self.set_style(env.minihud_ctx.expect("minihud ctx").hud_font_size);
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        // 三段共享 lines[2] (旧 mechanization 合并串), 各取本段槽
        let (fw, ab, g) = match split_trim3(&t.lines[2]) {
            Some((a, b, c)) => (a, b, c),
            None => (String::new(), String::new(), String::new()),
        };
        let seg = match self.kind {
            MechKind::Flaps => fw,
            MechKind::Airbrake => ab,
            MechKind::Gear => g,
        };
        self.set_template(&seg);
        self.update(&seg, false);
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        let data = env.data;
        let text = match self.kind {
            MechKind::Flaps => data.flaps_wing_str.as_str(),
            MechKind::Airbrake => data.airbrake_str.as_str(),
            MechKind::Gear => data.gear_str.as_str(),
        };
        self.update(text, data.warn_configuration);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        MechPart::draw(self, cv, x, y, &fonts.draw, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        let (w, h) = MechPart::preferred_size(self, &fonts.draw);
        Dimension::new(w, h)
    }
}

impl HudWidget for ManeuverBar {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        let ctx = env.minihud_ctx.expect("minihud 族组件需 minihud_ctx");
        self.set_style(
            ctx.hud_font_size,
            ctx.right_draw,
            ctx.half_line,
            ctx.line_width,
            ctx.stroke_thick_w,
            ctx.stroke_thin_w,
        );
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        // preview 机动条 (maneuver 长度/刻度属编排器会话量, 经模板值包透传)
        self.update(t.maneuver.0, t.maneuver.1, t.maneuver.2);
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // live: 机动条 (maneuver 长度/刻度属编排器会话量, 经 env 透传)
        self.update(env.data.maneuver_index, env.maneuver_len, env.maneuver_ticks);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        ManeuverBar::draw(self, cv, x, y, &fonts.draw, aa)
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        let (w, h) = ManeuverBar::preferred_size(self);
        Dimension::new(w, h)
    }
}

// =====================================================================
// 仪表族
// =====================================================================

impl HudWidget for FlapAngleBar {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        let ctx = env.minihud_ctx.expect("minihud 族组件需 minihud_ctx");
        // Dynamic width
        let responsive_width = (ctx.hud_font_size as f64 * 6.0) as i32;
        self.set_style_context(responsive_width, ctx.line_width + 2);
    }

    fn push_templates(&mut self, _t: &MiniHudTemplates) {
        // preview: 满襟翼 (原 updateComponents 无 flap 推值 — on_data_update 缺席)
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        let d = env.data;
        self.update(d.flaps, d.flap_allow_angle);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        FlapAngleBar::draw(self, cv, x, y, Some(&fonts.small), aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        // w = totalWidth>0 ? totalWidth : 200; h = small.size + barHeight + 5
        Dimension::new(
            if self.total_width() > 0 {
                self.total_width()
            } else {
                200
            },
            fonts.small.size + self.bar_height() + 5,
        )
    }
}

impl HudWidget for SpeedRatioBar {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        let ctx = env.minihud_ctx.expect("minihud 族组件需 minihud_ctx");
        let mut w = (ctx.hud_font_size as f64 * 0.25) as i32;
        let h = (ctx.hud_font_size as f64 * 5.5) as i32;
        if w < 6 {
            w = 6;
        }
        self.set_style_context(w, h);
    }

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, env: &UpdateEnv) {
        let d = env.data;
        self.update(
            d.speed_bar_speed_ratio,
            d.speed_bar_stall_ratio,
            d.speed_bar_unit_mach_limit_ratio,
            d.speed_bar_aileron_lock_ratio,
            d.speed_bar_rudder_lock_ratio,
        );
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        SpeedRatioBar::draw(self, cv, x, y, Some(&fonts.s_small), aa)
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        Dimension::new(self.width(), self.height())
    }
}

impl HudWidget for LinearGauge {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        let ctx = env.minihud_ctx.expect("minihud 族组件需 minihud_ctx");
        // Standardizing to relative size: 4.8 lines high (原注)
        let responsive_height = (ctx.hud_font_size as f64 * 4.8) as i32;
        self.set_style_context(responsive_height, ctx.bar_width);
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        // preview throttle (原 update_components 的 service=None 分支 → 0)
        self.update(t.throttle, &pad_width(t.throttle.to_string(), 3, false));
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        let d = env.data;
        self.update(d.throttle, &pad_width(d.throttle.to_string(), 3, false));
        self.set_value_color(Some(d.throttle_color));
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        LinearGauge::draw(self, cv, x, y, &fonts.s_small, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        // textMetric = fontNum.size*2 + thickness; height = lengthCache
        Dimension::new(
            fonts.s_small.size * 2 + self.thickness_cache(),
            self.length_cache(),
        )
    }
}

impl HudWidget for AttitudeIndicatorGauge {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        let ctx = env.minihud_ctx.expect("minihud 族组件需 minihud_ctx");
        self.set_style_context(
            ctx.compass_diameter,
            ctx.compass_radius,
            ctx.compass_inner_mark_radius,
            ctx.line_width,
            ctx.half_line,
            ctx.fonts.small.size,
        );
        self.set_inertial_mode(env.settings.attitude_indicator_inertial_mode);
    }

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, env: &UpdateEnv) {
        AttitudeIndicatorGauge::on_data_update(self, env.data);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        AttitudeIndicatorGauge::draw(self, cv, x, y, Some(&fonts.small), aa)
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        let (w, h) = AttitudeIndicatorGauge::preferred_size(self);
        Dimension::new(w, h)
    }
}

impl HudWidget for CompassGauge {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        let ctx = env.minihud_ctx.expect("minihud 族组件需 minihud_ctx");
        self.set_style_context(
            ctx.round_compass,
            ctx.line_width,
            ctx.hud_font_size,
            ctx.hud_font_size_small,
        );
        self.set_inertial_mode(env.settings.attitude_indicator_inertial_mode);
    }

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, env: &UpdateEnv) {
        self.update(env.data.heading, &env.data.map_grid);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        CompassGauge::draw(self, cv, x, y, Some(&fonts.small), aa)
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        let (w, h) = CompassGauge::preferred_size(self);
        Dimension::new(w, h)
    }
}

impl HudWidget for CrosshairGauge {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        // 软件矢量路径即唯一视觉语义 (纹理分支裁决, 原 gauge_crosshair)
        self.set_style_context(env.settings.crosshair_scale);
    }

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, _env: &UpdateEnv) {
        // CrosshairGauge 无 onDataUpdate 覆写
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        CrosshairGauge::draw(self, cv, x, y, aa)
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        let (w, h) = CrosshairGauge::preferred_size(self);
        Dimension::new(w, h)
    }
}

// =====================================================================
// 注册表
// =====================================================================

const EMPTY_PROPS: &[PropSchema] = &[];

/// 工厂速记 (const 上下文可用)
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
        composite: false,
        props_schema: EMPTY_PROPS,
        config_keys,
        data_shorts,
        default_props: "{}",
        factory,
    }
}

/// 行族原子件可见性键集 (update_row_visibility 消费; 键 = 原行内开关迁移)
const SPEED_KEYS: &[&str] = &["drawHUDtext", "showHUDSpeed"];
const AOA_KEYS: &[&str] = &["drawHUDtext", "showHUDAoA"];
const ALTITUDE_KEYS: &[&str] = &["drawHUDtext", "showHUDAltitude"];
const ENERGY_KEYS: &[&str] = &["drawHUDtext", "showHUDEnergy"];
const FLAPS_KEYS: &[&str] = &["drawHUDtext", "showHUDFlaps"];
const AIRBRAKE_KEYS: &[&str] = &["drawHUDtext", "showHUDAirbrake"];
const GEAR_KEYS: &[&str] = &["drawHUDtext", "showHUDGear"];
const SEP_KEYS: &[&str] = &["drawHUDtext", "showHUDSep"];
const GLOAD_KEYS: &[&str] = &["drawHUDtext", "showHUDGLoad"];
const MANEUVERBAR_KEYS: &[&str] = &["drawHUDtext", "showHUDManeuverBar"];

/// minihud 族 ctx 取用 (缺 ctx = 编排器未提供 → 工厂 Err 跳过该节点, 不 panic)
fn need_ctx<'a>(
    fctx: &FactoryCtx<'a>,
) -> Result<&'a crate::overlays::minihud::MinimalHudContext, String> {
    fctx.minihud_ctx
        .ok_or_else(|| "minihud 族组件需 minihud_ctx (页面编排器未提供)".to_string())
}

fn f_speed(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let ctx = need_ctx(fctx)?;
    Ok(Box::new(SpeedReadout(HUDTextRow::new(
        0,
        ctx.hud_font_size,
    ))))
}

fn f_aoa(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let ctx = need_ctx(fctx)?;
    Ok(Box::new(AoaGauge::new(
        ctx.hud_font_size,
        ctx.right_draw,
        ctx.line_width,
    )))
}

fn f_altitude(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let ctx = need_ctx(fctx)?;
    Ok(Box::new(AltitudeReadout(HUDTextRow::new(
        1,
        ctx.hud_font_size,
    ))))
}

fn f_energy(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let ctx = need_ctx(fctx)?;
    Ok(Box::new(EnergyReadout::new(
        ctx.hud_font_size,
        ctx.right_draw,
    )))
}

fn f_flaps(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    Ok(Box::new(MechPart::new(
        MechKind::Flaps,
        need_ctx(fctx)?.hud_font_size,
    )))
}

fn f_airbrake(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    Ok(Box::new(MechPart::new(
        MechKind::Airbrake,
        need_ctx(fctx)?.hud_font_size,
    )))
}

fn f_gear(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    Ok(Box::new(MechPart::new(
        MechKind::Gear,
        need_ctx(fctx)?.hud_font_size,
    )))
}

fn f_sep(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    Ok(Box::new(SepReadout(HUDTextRow::new(
        3,
        need_ctx(fctx)?.hud_font_size,
    ))))
}

fn f_gload(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let ctx = need_ctx(fctx)?;
    Ok(Box::new(GLoadReadout(HUDTextRow::new(
        4,
        ctx.hud_font_size,
    ))))
}

fn f_maneuverbar(
    _props: &serde_json::Value,
    fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    let ctx = need_ctx(fctx)?;
    Ok(Box::new(ManeuverBar::new(
        ctx.hud_font_size,
        ctx.right_draw,
        ctx.half_line,
        ctx.line_width,
        ctx.stroke_thick_w,
        ctx.stroke_thin_w,
    )))
}

fn f_flap(_props: &serde_json::Value, _fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    Ok(Box::new(FlapAngleBar::new()))
}

fn f_speed_bar(_props: &serde_json::Value, _fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    Ok(Box::new(SpeedRatioBar::new()))
}

fn f_throttle(
    _props: &serde_json::Value,
    _fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    Ok(Box::new(LinearGauge::new("ThrottleBar", 110, true)))
}

fn f_attitude(
    _props: &serde_json::Value,
    _fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    Ok(Box::new(AttitudeIndicatorGauge::new()))
}

fn f_compass(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    Ok(Box::new(CompassGauge::new(need_ctx(fctx)?.round_compass)))
}

fn f_crosshair(
    _props: &serde_json::Value,
    _fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    Ok(Box::new(CrosshairGauge::new()))
}

/// MiniHUD 族注册表 (顺序 = palette 展示序; 行原子件按行序排列)
pub(super) const REGISTRY_ENTRIES: &[WidgetMeta] = &[
    meta("core.minihud.speed", "速度读数", WidgetCategory::Text, SPEED_KEYS, &["ias"], f_speed),
    meta("core.minihud.aoa", "AoA 指示", WidgetCategory::Text, AOA_KEYS, &["aoa"], f_aoa),
    meta("core.minihud.altitude", "高度读数", WidgetCategory::Text, ALTITUDE_KEYS, &["altitude"], f_altitude),
    meta("core.minihud.energy", "能量读数", WidgetCategory::Text, ENERGY_KEYS, &["energy"], f_energy),
    meta("core.minihud.flaps", "襟翼/可变翼", WidgetCategory::Text, FLAPS_KEYS, &["flaps"], f_flaps),
    meta("core.minihud.airbrake", "减速板", WidgetCategory::Text, AIRBRAKE_KEYS, &["airbrake"], f_airbrake),
    meta("core.minihud.gear", "起落架", WidgetCategory::Text, GEAR_KEYS, &["gear"], f_gear),
    meta("core.minihud.sep", "SEP 读数", WidgetCategory::Text, SEP_KEYS, &["sep"], f_sep),
    meta("core.minihud.gload", "G 读数", WidgetCategory::Text, GLOAD_KEYS, &["ny"], f_gload),
    meta("core.minihud.maneuverbar", "机动刻度条", WidgetCategory::Text, MANEUVERBAR_KEYS, &["maneuver_index"], f_maneuverbar),
    meta("core.minihud.flapBar", "智能襟翼条", WidgetCategory::Gauge, &["drawHUDtext", "enableFlapAngleBar"], &["flaps"], f_flap),
    meta("core.minihud.speedBar", "速度条", WidgetCategory::Gauge, &["drawHUDtext", "showSpeedBar"], &["ias"], f_speed_bar),
    meta("core.minihud.throttleBar", "油门条", WidgetCategory::Gauge, &["drawHUDtext", "showSpeedBar"], &["throttle"], f_throttle),
    meta("core.gauge.attitude", "姿态指示器", WidgetCategory::Gauge, &["drawHUDtext", "showAttitudeGauge", "attitudeIndicatorInertialMode"], &["roll"], f_attitude),
    meta("core.gauge.compass", "罗盘", WidgetCategory::Gauge, &["drawHUDtext", "showAttitudeGauge", "attitudeIndicatorInertialMode"], &["compass"], f_compass),
    meta("core.decor.crosshair", "准星", WidgetCategory::Decor, &["displayCrosshair", "crosshairScale"], &[], f_crosshair),
];
