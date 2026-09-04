//! MiniHUD 组件族注册 (W2: 原 MiniHudComponentInner 枚举分发的 trait 化)。
//!
//! impl 体 = 原 comp.rs 的各枚举臂 + mod.rs update_component/row_visibility
//! 的组件自治段 (组件从 settings 自算可见性 — "配置驱动可见性属于组件")。

use vm_core::base::format::pad_width;


use crate::layout::hud_layout_node::Dimension;
use crate::overlays::attitude::AttitudeIndicatorGauge;
use crate::overlays::bars::{FlapAngleBar, LinearGauge, SpeedRatioBar};
use crate::overlays::compass::CompassGauge;
use crate::overlays::crosshair::CrosshairGauge;
use crate::overlays::rows::{
    HUDAkbRow, HUDEnergyRow, HUDManeuverRow, HUDMechanizationRow, HUDTextRow,
};
use crate::render::canvas::PixCanvas;

use super::env::{FactoryCtx, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::registry::{HudWidget, PropSchema, WidgetCategory, WidgetMeta};
use super::registry::PageFonts;

// =====================================================================
// 行族
// =====================================================================

impl HudWidget for HUDAkbRow {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        let ctx = env.minihud_ctx.expect("minihud 族组件需 minihud_ctx");
        self.set_style(ctx.right_draw, ctx.line_width, ctx.aoa_length as i32);
        // 原 update_row_visibility 的 Row0 段 (master = drawHudText)
        let master = env.settings.draw_hud_text;
        self.set_show_speed(master && env.settings.show_hud_speed);
        self.set_show_aoa(master && env.settings.show_hud_aoa);
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        self.set_template(Some(&t.lines[0]), Some(&t.line_aoa));
        // preview 值推送 (原 update_row_values 的 service 缺席分支)
        self.update(
            &t.lines[0],
            false,
            &t.line_aoa,
            t.aoa_y,
            t.aoa_color,
            t.aoa_bar_color,
        );
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        let data = env.data;
        self.base.update(&data.speed_str, data.warn_vne);
        self.aoa_text.clear();
        self.aoa_text.push_str(&data.aoa_str);
        self.aoa_color = data.aoa_color;
        self.aoa_bar_color = data.aoa_bar_color;
        self.set_aoa_from_ratio(data.aoa_ratio);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        HUDAkbRow::draw(self, cv, x, y, &fonts.draw, &fonts.small, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        let (w, h) = HUDAkbRow::preferred_size(self, &fonts.draw, &fonts.small);
        Dimension::new(w, h)
    }
}

impl HudWidget for HUDEnergyRow {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        self.set_style(env.minihud_ctx.expect("minihud 族组件需 minihud_ctx").right_draw);
        let master = env.settings.draw_hud_text;
        self.set_show_altitude(master && env.settings.show_hud_altitude);
        self.set_show_energy(master && env.settings.show_hud_energy);
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        self.set_template(Some(&t.lines[1]), Some(&t.rel_energy));
        self.update(&t.lines[1], false, &t.rel_energy);
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        let data = env.data;
        self.update(&data.alt_str, data.warn_altitude, &data.energy_str);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        HUDEnergyRow::draw(self, cv, x, y, &fonts.draw, &fonts.small, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        let (w, h) = HUDEnergyRow::preferred_size(self, &fonts.draw, &fonts.small);
        Dimension::new(w, h)
    }
}

impl HudWidget for HUDMechanizationRow {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        self.base.set_style(env.minihud_ctx.expect("minihud ctx").hud_font_size);
        let master = env.settings.draw_hud_text;
        self.set_show_flaps(master && env.settings.show_hud_flaps);
        self.set_show_airbrake(master && env.settings.show_hud_airbrake);
        self.set_show_gear(master && env.settings.show_hud_gear);
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        // 完整 set_template (内部三段重解析, 原注: 虚分派语义)
        self.set_template(Some(&t.lines[2]));
        self.update(&t.lines[2], t.in_action);
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        HUDMechanizationRow::on_data_update(self, env.data);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        HUDMechanizationRow::draw(self, cv, x, y, &fonts.draw, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        let (w, h) = HUDMechanizationRow::preferred_size(self, &fonts.draw);
        Dimension::new(w, h)
    }
}

impl HudWidget for HUDTextRow {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, env: &StyleEnv) {
        self.set_style(env.minihud_ctx.expect("minihud ctx").hud_font_size);
        // Row3 无行内细粒度开关 (可见性 = master, WidgetBox 层)
        let _ = env.settings.draw_hud_text;
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        self.set_template(Some(&t.lines[3]));
        self.update(&t.lines[3], false);
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // live: SEP (原 update_legacy_components 桥)
        self.update(&env.data.sep_str, false);
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        HUDTextRow::draw(self, cv, x, y, &fonts.draw, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        let (w, h) = HUDTextRow::preferred_size(self, &fonts.draw);
        Dimension::new(w, h)
    }
}

impl HudWidget for HUDManeuverRow {
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
        let master = env.settings.draw_hud_text;
        self.set_show_g_load(master && env.settings.show_hud_g_load);
        self.set_show_maneuver_bar(master && env.settings.show_hud_maneuver_bar);
    }

    fn push_templates(&mut self, t: &MiniHudTemplates) {
        self.base.set_template(Some(&t.lines[4]));
        self.update(&t.lines[4], false, t.maneuver.0, t.maneuver.1, t.maneuver.2);
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // live: G + 机动条 (原 update_legacy_components 桥; maneuver 长度/刻度
        // 属编排器会话量, 经 env 透传)
        let data = env.data;
        self.update(
            &data.maneuver_state_str,
            false,
            data.maneuver_index,
            env.maneuver_len,
            env.maneuver_ticks,
        );
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool) {
        HUDManeuverRow::draw(self, cv, x, y, &fonts.draw, aa)
    }

    fn preferred_size(&self, fonts: &PageFonts) -> Dimension {
        let (w, h) = HUDManeuverRow::preferred_size(self, &fonts.draw);
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
        factory,
    }
}

/// 行组件可见性键集 (update_row_visibility 消费)
const ROW0_KEYS: &[&str] = &["drawHUDtext", "showHUDSpeed", "showHUDAoA"];
const ROW1_KEYS: &[&str] = &["drawHUDtext", "showHUDAltitude", "showHUDEnergy"];
const ROW2_KEYS: &[&str] = &["drawHUDtext", "showHUDFlaps", "showHUDAirbrake", "showHUDGear"];
const ROW3_KEYS: &[&str] = &["drawHUDtext", "showHUDSep"];
const ROW4_KEYS: &[&str] = &["drawHUDtext", "showHUDGLoad", "showHUDManeuverBar"];

/// minihud 族 ctx 取用 (缺 ctx = 编排器未提供 → 工厂 Err 跳过该节点, 不 panic)
fn need_ctx<'a>(
    fctx: &FactoryCtx<'a>,
) -> Result<&'a crate::overlays::minihud::MinimalHudContext, String> {
    fctx.minihud_ctx
        .ok_or_else(|| "minihud 族组件需 minihud_ctx (页面编排器未提供)".to_string())
}

fn f_row0(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let ctx = need_ctx(fctx)?;
    Ok(Box::new(HUDAkbRow::new(
        0,
        ctx.hud_font_size,
        ctx.right_draw,
        ctx.line_width,
    )))
}

fn f_row1(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let ctx = need_ctx(fctx)?;
    Ok(Box::new(HUDEnergyRow::new(1, ctx.hud_font_size, ctx.right_draw)))
}

fn f_row2(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    Ok(Box::new(HUDMechanizationRow::new(2, need_ctx(fctx)?.hud_font_size)))
}

fn f_row3(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    Ok(Box::new(HUDTextRow::new(3, need_ctx(fctx)?.hud_font_size)))
}

fn f_row4(_props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let ctx = need_ctx(fctx)?;
    Ok(Box::new(HUDManeuverRow::new(
        4,
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

fn f_speed(_props: &serde_json::Value, _fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
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

/// MiniHUD 族注册表 (W2 首批; 顺序 = palette 展示序)
pub(super) const REGISTRY_ENTRIES: &[WidgetMeta] = &[
    meta("core.minihud.row0", "速度/AoA 行", WidgetCategory::Text, ROW0_KEYS, &["ias", "aoa"], f_row0),
    meta("core.minihud.row1", "高度/能量 行", WidgetCategory::Text, ROW1_KEYS, &["altitude", "vy"], f_row1),
    meta("core.minihud.row2", "襟翼/减速板/起落架", WidgetCategory::Text, ROW2_KEYS, &["flaps", "airbrake", "gear"], f_row2),
    meta("core.minihud.row3", "SEP 行", WidgetCategory::Text, ROW3_KEYS, &["sep"], f_row3),
    meta("core.minihud.row4", "过载/机动条", WidgetCategory::Text, ROW4_KEYS, &["ny", "maneuver_index"], f_row4),
    meta("core.minihud.flapBar", "智能襟翼条", WidgetCategory::Gauge, &["drawHUDtext", "enableFlapAngleBar"], &["flaps"], f_flap),
    meta("core.minihud.speedBar", "速度条", WidgetCategory::Gauge, &["drawHUDtext", "showSpeedBar"], &["ias"], f_speed),
    meta("core.minihud.throttleBar", "油门条", WidgetCategory::Gauge, &["drawHUDtext", "showSpeedBar"], &["throttle"], f_throttle),
    meta("core.gauge.attitude", "姿态指示器", WidgetCategory::Gauge, &["drawHUDtext", "showAttitudeGauge", "attitudeIndicatorInertialMode"], &["roll"], f_attitude),
    meta("core.gauge.compass", "罗盘", WidgetCategory::Gauge, &["drawHUDtext", "showAttitudeGauge", "attitudeIndicatorInertialMode"], &["compass"], f_compass),
    meta("core.decor.crosshair", "准星", WidgetCategory::Decor, &["displayCrosshair", "crosshairScale"], &[], f_crosshair),
];
