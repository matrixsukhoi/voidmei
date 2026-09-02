//! MiniHUD 组件装配层 (HUDComponent.java + AbstractHUDComponent.java):
//! - MiniHudComponentInner: 组件清单的异构装箱 (Java 各组件类按具名槽位装箱)
//! - MiniHudComponent: 内件 + visible + 字体共享 + 风格缓存 (组装 seam)
//! - CompCell: 共享句柄 (Rc<RefCell>), overlay 具名字段与布局节点图双持
//! - MiniHudOverlay 的装配方法: 组件创建/风格注入/模板推送/布局引擎组 parts
//!
//! 映射裁决 (模块头): Java `List<HUDComponent> components` (initComponentsLayout
//! 添加序) 与布局引擎节点图**共享同一批组件对象** → [`CompCell`](Rc<RefCell>) 双持:
//! overlay 具名字段 (风格/模板/可见性写入口) + engine 节点负载 (渲染读出口),
//! Java 引用共享语义落地。

use std::cell::RefCell;
use std::rc::Rc;

use vm_core::base::format::{java_f, pad_width};
use vm_core::config::config_api::HUDSettings;
use vm_core::derived::hud_data::HUDData;

use crate::layout::hud_layout_node::{Dimension, HasPreferredSize};
use crate::layout::minihud_layout::{
    build_mihud_layout, HasVisibility, MiniHudLayoutConfig, MiniHudParts,
};
use crate::overlays::attitude::AttitudeIndicatorGauge;
use crate::overlays::bars::{FlapAngleBar, LinearGauge, SpeedRatioBar};
use crate::overlays::compass::CompassGauge;
use crate::overlays::crosshair::CrosshairGauge;
use crate::overlays::rows::{HUDAkbRow, HUDEnergyRow, HUDMechanizationRow, HUDManeuverRow, HUDTextRow};
use crate::render::canvas::PixCanvas;
use crate::render::palette::colors;

use super::{fmt_d, MiniHudOverlay};
use super::ctx::MiniHudFonts;

// ---------------------------------------------------------------------------
// HUDComponent 装配 seam (HUDComponent.java + AbstractHUDComponent.java)
// ---------------------------------------------------------------------------

/// MiniHUD 组件清单的异构装箱 (Java 各组件类; Rust 组件已译于
/// rows/gauges_bars/gauge_* 模块, 此处按 MiniHUDOverlay 的具名槽位装箱)。
/// Java Row2 = HUDMechanizationRow (MiniHUDOverlay.java:561; 三段拆分:
/// 襟翼/减速板/起落架独立开关 + 模板占位推进, rows.rs 同译)。
pub enum MiniHudComponentInner {
    /// hudRows[0]: HUDAkbRow (速度 + AoA)
    Row0(HUDAkbRow),
    /// hudRows[1]: HUDEnergyRow (高度 + 能量)
    Row1(HUDEnergyRow),
    /// hudRows[2]: HUDMechanizationRow (襟翼/可变翼 + 减速板 + 起落架三段)
    Row2(HUDMechanizationRow),
    /// hudRows[3]: HUDTextRow (SEP)
    Row3(HUDTextRow),
    /// hudRows[4]: HUDManeuverRow (G + 机动条)
    Row4(HUDManeuverRow),
    /// flapAngleBar
    FlapBar(FlapAngleBar),
    /// speedRatioBar
    SpeedRatioBar(SpeedRatioBar),
    /// throttleBar (LinearGauge "ThrottleBar")
    ThrottleBar(LinearGauge),
    /// attitudeIndicatorGauge
    Attitude(AttitudeIndicatorGauge),
    /// compassGauge
    Compass(CompassGauge),
    /// crosshairGauge
    Crosshair(CrosshairGauge),
}

/// 组装层组件 = 内件 + AbstractHUDComponent.visible + 字体共享 + 风格缓存。
/// 风格缓存: Java 组件的 width/height/totalWidth/lengthCache 等字段参与
/// getPreferredSize, Rust 移植未暴露 → 组装层在 set_style 时镜像 (值同源同步,
/// 只读回放, 不构成第二真相)。
pub struct MiniHudComponent {
    pub inner: MiniHudComponentInner,
    /// AbstractHUDComponent.visible (布局引擎 render/getContentBounds 门控)
    visible: bool,
    fonts: Rc<MiniHudFonts>,
    /// FlapAngleBar total_width (Java: totalWidth; preferred 用)
    flap_total_width: i32,
    /// FlapAngleBar bar_height (Java: barHeight)
    flap_bar_height: i32,
    /// SpeedRatioBar width/height (Java 字段; Rust 组件私有)
    speed_w: i32,
    speed_h: i32,
    /// Throttle LinearGauge lengthCache/thicknessCache (setStyleContext 注入)
    throttle_length: i32,
    throttle_thickness: i32,
}

impl MiniHudComponent {
    fn new(inner: MiniHudComponentInner, fonts: Rc<MiniHudFonts>) -> Self {
        MiniHudComponent {
            inner,
            visible: true, // AbstractHUDComponent.visible 初始 true
            fonts,
            flap_total_width: 0,
            flap_bar_height: 0,
            speed_w: 10,   // SpeedRatioBar::new 缺省 (Java:26-27)
            speed_h: 100,
            throttle_length: 100, // LinearGauge::new 缺省 (Java:79-80)
            throttle_thickness: 10,
        }
    }

    /// AbstractHUDComponent.setVisible
    pub fn set_visible(&mut self, v: bool) {
        self.visible = v;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Java setStyle 链的 Font 形参对应物: reinit 重建 ctx 后字体档整体换新
    /// (Java 各组件 setStyle(..., ctx.drawFont, ...) 传入新 Font 对象)
    pub fn set_fonts(&mut self, fonts: Rc<MiniHudFonts>) {
        self.fonts = fonts;
    }

    /// HUDComponent.getPreferredSize (各 Java 组件实现的组装层聚合;
    /// 字体经 self.fonts 达成无参签名 — solve() 调用约束)
    pub fn preferred_size(&self) -> Dimension {
        match &self.inner {
            // HUDAkbRow.java:102-112 (rows.rs preferred_size 同译)
            MiniHudComponentInner::Row0(r) => {
                let (w, h) = r.preferred_size(&self.fonts.draw, &self.fonts.small);
                Dimension::new(w, h)
            }
            // HUDEnergyRow.java:78-88
            MiniHudComponentInner::Row1(r) => {
                let (w, h) = r.preferred_size(&self.fonts.draw, &self.fonts.small);
                Dimension::new(w, h)
            }
            // HUDMechanizationRow.java:115-131 (三段模板占位宽之和)
            MiniHudComponentInner::Row2(r) => {
                let (w, h) = r.preferred_size(&self.fonts.draw);
                Dimension::new(w, h)
            }
            MiniHudComponentInner::Row3(r) => {
                let (w, h) = r.preferred_size(&self.fonts.draw);
                Dimension::new(w, h)
            }
            // HUDManeuverRow.java:123-128
            MiniHudComponentInner::Row4(r) => {
                let (w, h) = r.preferred_size(&self.fonts.draw);
                Dimension::new(w, h)
            }
            // FlapAngleBar.java:47-51: w = totalWidth>0 ? totalWidth : 200;
            // h = (font!=null ? font.size : 12) + barHeight + 5 (font = drawFontSmall)
            MiniHudComponentInner::FlapBar(_) => Dimension::new(
                if self.flap_total_width > 0 { self.flap_total_width } else { 200 },
                self.fonts.small.size + self.flap_bar_height + 5,
            ),
            // SpeedRatioBar.java:54-56
            MiniHudComponentInner::SpeedRatioBar(_) => {
                Dimension::new(self.speed_w, self.speed_h)
            }
            // LinearGauge.java:61-76 (vertical): textMetric = fontNum.size*2 + thickness;
            // height = lengthCache (fontNum = drawFontSSmall)
            MiniHudComponentInner::ThrottleBar(_) => Dimension::new(
                self.fonts.s_small.size * 2 + self.throttle_thickness,
                self.throttle_length,
            ),
            // AttitudeIndicatorGauge.java:63-66
            MiniHudComponentInner::Attitude(a) => {
                let (w, h) = a.preferred_size();
                Dimension::new(w, h)
            }
            // CompassGauge.java:58-60
            MiniHudComponentInner::Compass(c) => {
                let (w, h) = c.preferred_size();
                Dimension::new(w, h)
            }
            // CrosshairGauge.java:38-44 (软件分支)
            MiniHudComponentInner::Crosshair(c) => {
                let (w, h) = c.preferred_size();
                Dimension::new(w, h)
            }
        }
    }

    /// HUDComponent.onDataUpdate(HUDData) — 各 Java 组件覆写的分发
    pub fn on_data_update(&mut self, data: &HUDData) {
        match &mut self.inner {
            // HUDAkbRow.java:56-73: super.update(speedStr, warnVne) + aoa 族 +
            // aoaY = (int)(aoaRatio*aoaLength) 钳 rightDraw (rows.rs set_aoa_from_ratio)
            MiniHudComponentInner::Row0(r) => {
                r.base.update(&data.speed_str, data.warn_vne);
                r.aoa_text.clear();
                r.aoa_text.push_str(&data.aoa_str);
                r.aoa_color = data.aoa_color;
                r.aoa_bar_color = data.aoa_bar_color;
                r.set_aoa_from_ratio(data.aoa_ratio);
            }
            // HUDEnergyRow.java:44-50
            MiniHudComponentInner::Row1(r) => {
                r.update(&data.alt_str, data.warn_altitude, &data.energy_str);
            }
            // HUDMechanizationRow.java:63-70: 三段串直取 + isWarning 直写
            MiniHudComponentInner::Row2(r) => {
                r.on_data_update(data);
            }
            // Row3/Row4 无 onDataUpdate 覆写 (default 空) — 数据走 updateLegacyComponents 桥
            MiniHudComponentInner::Row3(_) | MiniHudComponentInner::Row4(_) => {}
            // FlapAngleBar.java:60-67
            MiniHudComponentInner::FlapBar(f) => {
                f.update(data.flaps, data.flap_allow_angle);
            }
            // SpeedRatioBar.java:70-78
            MiniHudComponentInner::SpeedRatioBar(s) => {
                s.update(
                    data.speed_bar_speed_ratio,
                    data.speed_bar_stall_ratio,
                    data.speed_bar_unit_mach_limit_ratio,
                    data.speed_bar_aileron_lock_ratio,
                    data.speed_bar_rudder_lock_ratio,
                );
            }
            // CompassGauge.java:83-99 (heading/mapGrid 两输入)
            MiniHudComponentInner::Compass(c) => {
                c.update(data.heading, &data.map_grid);
            }
            // AttitudeIndicatorGauge.java:192-224
            MiniHudComponentInner::Attitude(a) => {
                a.on_data_update(data);
            }
            // LinearGauge.java:91-103 (label=="ThrottleBar" 分支)
            MiniHudComponentInner::ThrottleBar(t) => {
                t.update(data.throttle, &fmt_d(data.throttle, 3));
                t.set_value_color(Some(data.throttle_color));
            }
            // CrosshairGauge 无 onDataUpdate 覆写
            MiniHudComponentInner::Crosshair(_) => {}
        }
    }

    /// HUDComponent.draw(g2d, x, y) — aa 对齐 paintComponent 的 graphAASetting
    /// (生产恒 ON; false 供对拍)。字体 = 各 Java 组件构造/setStyle 注入的同三档。
    pub fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, aa: bool) {
        let f = self.fonts.clone(); // Rc 引用计数, 零堆分配 (零分配纪律)
        match &mut self.inner {
            MiniHudComponentInner::Row0(r) => r.draw(cv, x, y, &f.draw, &f.small, aa),
            MiniHudComponentInner::Row1(r) => r.draw(cv, x, y, &f.draw, &f.small, aa),
            MiniHudComponentInner::Row2(r) => r.draw(cv, x, y, &f.draw, aa),
            MiniHudComponentInner::Row3(r) => r.draw(cv, x, y, &f.draw, aa),
            MiniHudComponentInner::Row4(r) => r.draw(cv, x, y, &f.draw, aa),
            // FlapAngleBar: font=drawFontSmall (applyStyleToComponents L615)
            MiniHudComponentInner::FlapBar(b) => b.draw(cv, x, y, Some(&f.small), aa),
            // SpeedRatioBar: tickFont=drawFontSSmall (applyStyleToComponents L601)
            MiniHudComponentInner::SpeedRatioBar(s) => s.draw(cv, x, y, Some(&f.s_small), aa),
            // LinearGauge: fontNum=drawFontSSmall (applyStyleToComponents L645)
            MiniHudComponentInner::ThrottleBar(t) => t.draw(cv, x, y, &f.s_small, aa),
            // Attitude: font=drawFontSmall (applyStyleToComponents L624)
            MiniHudComponentInner::Attitude(a) => a.draw(cv, x, y, Some(&f.small), aa),
            // Compass: fontSmall=drawFontSmall (applyStyleToComponents L618)
            MiniHudComponentInner::Compass(c) => c.draw(cv, x, y, Some(&f.small), aa),
            MiniHudComponentInner::Crosshair(c) => c.draw(cv, x, y, aa),
        }
    }
}

/// 组件共享句柄 (Java `components` 列表与布局节点图共享同一批对象 → Rc<RefCell>)。
/// newtype 承载 vm-core 的 [`HasPreferredSize`] 与本 crate 的 [`HasVisibility`]
/// (孤儿规则禁直impl Rc)。
pub struct CompCell(pub(super) Rc<RefCell<MiniHudComponent>>);

impl Clone for CompCell {
    fn clone(&self) -> Self {
        CompCell(Rc::clone(&self.0))
    }
}

impl CompCell {
    fn new(inner: MiniHudComponentInner, fonts: Rc<MiniHudFonts>) -> Self {
        CompCell(Rc::new(RefCell::new(MiniHudComponent::new(inner, fonts))))
    }

    /// 枚举分发借出口: 把 inner 交给闭包, 调用侧以 `if let MiniHudComponentInner::X(r) = inner`
    /// 守卫变体 (不匹配即空转, 同原 borrow_mut+if let 样板)。
    /// **闭包内不得再借其它 CompCell** — RefCell 独占借用跨槽嵌套会 panic
    /// (现有调用已守此纪律)
    pub(super) fn map_inner<R>(&self, f: impl FnOnce(&mut MiniHudComponentInner) -> R) -> R {
        f(&mut self.0.borrow_mut().inner)
    }

    /// 组件整体借出口: inner 之外还要改组装层镜像字段 (speed_w/flap_total_width/
    /// throttle_length 等风格缓存) 的混合写面专用, 其余场景用 [`Self::map_inner`]
    fn map_comp<R>(&self, f: impl FnOnce(&mut MiniHudComponent) -> R) -> R {
        f(&mut self.0.borrow_mut())
    }

    /// AbstractHUDComponent.setVisible 的句柄侧便捷口 (组装层/测试)
    pub fn set_visible(&self, v: bool) {
        self.0.borrow_mut().set_visible(v);
    }

    pub fn is_visible(&self) -> bool {
        self.0.borrow().is_visible()
    }

    pub fn set_fonts(&self, fonts: Rc<MiniHudFonts>) {
        self.0.borrow_mut().set_fonts(fonts);
    }
}

impl HasPreferredSize for CompCell {
    fn preferred_size(&self) -> Dimension {
        // PORT: 与节点图的 RefCell 相互独立 (组件内省不回指节点图, 审查 B3 约束)
        self.0.borrow().preferred_size()
    }
}

impl HasVisibility for CompCell {
    fn is_visible(&self) -> bool {
        self.0.borrow().is_visible()
    }
}

// ---------------------------------------------------------------------------
// MiniHudOverlay 的组件装配段 (init/components 风格/模板/布局引擎组 parts)
// ---------------------------------------------------------------------------

/// 六个具名仪表组件的构造集 (见 [`MiniHudOverlay::named_gauge_cells`])
pub(super) struct NamedGaugeCells {
    pub(super) flap_angle_bar: CompCell,
    pub(super) speed_ratio_bar: CompCell,
    pub(super) compass_gauge: CompCell,
    pub(super) attitude_indicator_gauge: CompCell,
    pub(super) crosshair_gauge: CompCell,
    pub(super) throttle_bar: CompCell,
}

impl MiniHudOverlay {
    /// 六个具名仪表组件的构造集 — init 占位 (Java null 字段的 Rust 可查替身) 与
    /// init_components_layout 正式建身共用一份 (C27 双份构造收敛; 构造参数同源
    /// ctx.roundCompass, 行组件仅正式建身 — 占位期 hud_rows 为空)
    pub(super) fn named_gauge_cells(fonts: &Rc<MiniHudFonts>, round_compass: i32) -> NamedGaugeCells {
        let cell = |inner: MiniHudComponentInner| CompCell::new(inner, Rc::clone(fonts));
        NamedGaugeCells {
            flap_angle_bar: cell(MiniHudComponentInner::FlapBar(FlapAngleBar::new())),
            speed_ratio_bar: cell(MiniHudComponentInner::SpeedRatioBar(SpeedRatioBar::new())),
            compass_gauge: cell(MiniHudComponentInner::Compass(CompassGauge::new(round_compass))),
            attitude_indicator_gauge: cell(MiniHudComponentInner::Attitude(
                AttitudeIndicatorGauge::new(),
            )),
            crosshair_gauge: cell(MiniHudComponentInner::Crosshair(CrosshairGauge::new())),
            throttle_bar: cell(MiniHudComponentInner::ThrottleBar(LinearGauge::new(
                "ThrottleBar", 110, true,
            ))),
        }
    }

    /// Java refreshTemplates() (L161-208)
    pub(super) fn refresh_templates<S: HUDSettings>(&mut self, settings: &S) {
        let spd_pre = if settings.is_speed_label_disabled() { "" } else { "SPD" };
        let alt_pre = if settings.is_altitude_label_disabled() { "" } else { "ALT" };
        let sep_pre = if settings.is_sep_label_disabled() { "" } else { "SEP" };

        if settings.draw_hud_mach() {
            // "M%5.2f" (0.85) — M 前缀在宽度域外
            self.lines[0] = format!("M{}", pad_width(java_f(0.85, 2), 5, false));
        } else {
            self.lines[0] = format!("{spd_pre}{}", pad_width("360".to_string(), 5, false));
        }
        // Format must match HUDCalculator: radar = "R%5.0f" (R + 5 digits),
        // barometric = "%6.0f" (6 digits)
        self.lines[1] = if settings.always_show_radar_altitude() {
            // "R%5s" ("1024") — R 前缀 + 5 宽右对齐
            format!("{alt_pre}R{}", pad_width("1024".to_string(), 5, false))
        } else {
            format!("{alt_pre}{}", pad_width("1024".to_string(), 6, false))
        };
        // "↑%-4s"("30") — ↑ 是格式串字面量 (前缀, 不占 %-4s 宽度域)
        self.lines[3] = format!("{sep_pre}↑{}", pad_width("30".to_string(), 4, true));
        self.lines[4] = format!("G{}", pad_width("2.0".to_string(), 5, false));
        if settings.enable_flap_angle_bar() {
            self.lines[2] = pad_width(String::new(), 4, false); // "%4s"%""
        } else {
            self.lines[2] = format!("F{}", pad_width("100".to_string(), 3, false));
        }
        self.lines[2].push_str("BRK");
        self.lines[2].push_str("GEAR");
        self.throttley = 100;
        self.aoa_y = 10;
        self.throttle_color = colors().shade_shape; // Application.colorShadeShape
        self.aoa_color = colors().num;              // Application.colorNum
        self.aoa_bar_color = colors().num;
        self.line_aoa = format!("α{}", pad_width(java_f(20.0, 0), 3, false));
        self.rel_energy = "E114514".to_string();

        // Push new templates to existing components immediately
        if self.hud_rows.len() >= 5 {
            self.set_row_templates();
        }
    }

    /// refreshTemplates 尾部的模板推送 (L201-207; 行句柄借用拆出)
    pub(super) fn set_row_templates(&mut self) {
        let (l0, laoa, l1, lrel, l2, l3, l4) = (
            self.lines[0].clone(),
            self.line_aoa.clone(),
            self.lines[1].clone(),
            self.rel_energy.clone(),
            self.lines[2].clone(),
            self.lines[3].clone(),
            self.lines[4].clone(),
        );
        self.hud_rows[0].map_inner(|inner| {
            if let MiniHudComponentInner::Row0(r) = inner {
                r.set_template(Some(&l0), Some(&laoa));
            }
        });
        self.hud_rows[1].map_inner(|inner| {
            if let MiniHudComponentInner::Row1(r) = inner {
                r.set_template(Some(&l1), Some(&lrel));
            }
        });
        self.hud_rows[2].map_inner(|inner| {
            if let MiniHudComponentInner::Row2(r) = inner {
                // PORT: Java MiniHUDOverlay.java:204 强转 HUDTextRow, 但 setTemplate 非
                // final 且被 HUDMechanizationRow 同签名覆写 → 虚分派走覆写 (super + 三段
                // 模板重解析)。须调完整 set_template 而非仅基座, 否则模板变化时三段
                // 占位宽滞留旧值 (Java 会重解析)
                r.set_template(Some(&l2));
            }
        });
        self.hud_rows[3].map_inner(|inner| {
            if let MiniHudComponentInner::Row3(r) = inner {
                r.set_template(Some(&l3));
            }
        });
        self.hud_rows[4].map_inner(|inner| {
            if let MiniHudComponentInner::Row4(r) = inner {
                r.base.set_template(Some(&l4));
            }
        });
    }

    /// Java initComponentsLayout() (L524-589)
    pub(super) fn init_components_layout<S: HUDSettings>(&mut self, settings: &S) {
        self.components.clear(); // Ensure list is clean on re-init

        let fonts = Rc::clone(&self.fonts);
        let cell = |inner: MiniHudComponentInner| CompCell::new(inner, Rc::clone(&fonts));

        // 0.~3. 六具名仪表 (构造集与 init 占位共用一份; 入表序 = Java 添加序:
        // warningOverlay 已由 WarningBlinkHost 组合持有, Java:528)
        let ng = Self::named_gauge_cells(&fonts, self.ctx.round_compass);
        self.flap_angle_bar = ng.flap_angle_bar;
        self.components.push(self.flap_angle_bar.clone());

        // New SpeedRatioBar
        self.speed_ratio_bar = ng.speed_ratio_bar;
        self.components.push(self.speed_ratio_bar.clone());

        // 1. Compass — 构造注入 ctx.roundCompass (Java:537)
        self.compass_gauge = ng.compass_gauge;
        self.components.push(self.compass_gauge.clone());

        // 2. Attitude
        self.attitude_indicator_gauge = ng.attitude_indicator_gauge;
        self.components.push(self.attitude_indicator_gauge.clone());

        // 3. Crosshair — 无条件入 components (节点是否建由 cfg 决定, Java:545-546)
        self.crosshair_gauge = ng.crosshair_gauge;
        self.components.push(self.crosshair_gauge.clone());

        // 4. Rows (L549-578) — 构造第三参 height = ctx.hudFontSize (Java 各行构造)
        let h = self.ctx.hud_font_size;
        let mut row0 = HUDAkbRow::new(0, h, self.ctx.right_draw, self.ctx.line_width);
        row0.set_template(Some(&self.lines[0]), Some(&self.line_aoa));
        let mut row1 = HUDEnergyRow::new(1, h, self.ctx.right_draw);
        row1.set_template(Some(&self.lines[1]), Some(&self.rel_energy));
        let mut row2 = HUDMechanizationRow::new(2, h);
        // 使用旧格式模板，内部自动解析 (Java 注释原文; rows.rs set_template 三段切分)
        row2.set_template(Some(&self.lines[2]));
        let mut row3 = HUDTextRow::new(3, h);
        row3.set_template(Some(&self.lines[3]));
        let mut row4 = HUDManeuverRow::new(
            4,
            h,
            self.ctx.right_draw,
            self.ctx.half_line,
            self.ctx.line_width,
            self.ctx.stroke_thick_w,
            self.ctx.stroke_thin_w,
        );
        row4.base.set_template(Some(&self.lines[4]));

        self.hud_rows = vec![
            cell(MiniHudComponentInner::Row0(row0)),
            cell(MiniHudComponentInner::Row1(row1)),
            cell(MiniHudComponentInner::Row2(row2)),
            cell(MiniHudComponentInner::Row3(row3)),
            cell(MiniHudComponentInner::Row4(row4)),
        ];
        for row in &self.hud_rows {
            self.components.push(row.clone());
        }

        // 5. Bars — throttleBar (Java:581: new LinearGauge("ThrottleBar", 110, true, false))
        self.throttle_bar = ng.throttle_bar;
        self.components.push(self.throttle_bar.clone());

        // Ensure everything is styled and updated before layout & sizing (Java 注释)
        self.apply_style_to_components(settings);
        // PORT: 同 init 尾部 — Java 读 service 字段, 此处 None (throttle 闪 0,
        // 下一放行 on_flight_data 修复, ≤1 帧)
        self.update_components(settings, None);

        self.init_modern_layout(settings);
    }

    /// Java applyStyleToComponents() (L591-647)
    pub(super) fn apply_style_to_components<S: HUDSettings>(&mut self, settings: &S) {
        if self.components.is_empty() {
            // 恒在, 以 components 清单空近似同一守卫 (占位件随后被整体替换)
            return;
        }
        // 字体档换新 (Java 各 setStyle 的 Font 形参; reinit 重建 ctx 后生效)
        let fonts = Rc::clone(&self.fonts);
        for c in &self.components {
            c.set_fonts(Rc::clone(&fonts));
        }
        self.style_gauges(settings);
        // Synchronize styles for Rows
        if self.hud_rows.len() >= 5 {
            self.style_rows();
        }
        self.style_throttle_bar();
    }

    /// applyStyleToComponents 仪表段 (Java 段序: speed → crosshair → flap →
    /// compass → attitude)
    fn style_gauges<S: HUDSettings>(&mut self, settings: &S) {
        let ctx = &self.ctx;
        self.speed_ratio_bar.map_comp(|c| {
            if let MiniHudComponentInner::SpeedRatioBar(s) = &mut c.inner {
                // Width: similar to throttle bar or slightly thinner?
                let mut w = (ctx.hud_font_size as f64 * 0.25) as i32;
                let h = (ctx.hud_font_size as f64 * 5.5) as i32;
                if w < 6 {
                    w = 6;
                }
                s.set_style_context(w, h);
                c.speed_w = w;
                c.speed_h = h;
            }
        });
        self.crosshair_gauge.map_inner(|inner| {
            if let MiniHudComponentInner::Crosshair(g) = inner {
                // PORT: Java useTextureCrosshair 纹理分支 (L605-607) 不迁移 —
                // gauge_crosshair.rs 裁决, 软件路径即唯一视觉语义
                g.set_style_context(settings.get_crosshair_scale());
            }
        });
        self.flap_angle_bar.map_comp(|c| {
            if let MiniHudComponentInner::FlapBar(b) = &mut c.inner {
                // Dynamic width
                let responsive_width = (ctx.hud_font_size as f64 * 6.0) as i32;
                b.set_style_context(responsive_width, ctx.line_width + 2);
                c.flap_total_width = responsive_width;
                c.flap_bar_height = ctx.line_width + 2;
            }
        });
        self.compass_gauge.map_inner(|inner| {
            if let MiniHudComponentInner::Compass(g) = inner {
                g.set_style_context(
                    ctx.round_compass,
                    ctx.line_width,
                    ctx.hud_font_size,
                    ctx.hud_font_size_small,
                );
                g.set_inertial_mode(settings.is_attitude_indicator_inertial_mode());
            }
        });
        self.attitude_indicator_gauge.map_inner(|inner| {
            if let MiniHudComponentInner::Attitude(g) = inner {
                g.set_style_context(
                    ctx.compass_diameter,
                    ctx.compass_radius,
                    ctx.compass_inner_mark_radius,
                    ctx.line_width,
                    ctx.half_line,
                    ctx.fonts.small.size, // drawFontSmall 折为其 size (gauge_attitude 口径)
                );
                g.set_inertial_mode(settings.is_attitude_indicator_inertial_mode());
            }
        });
    }

    /// applyStyleToComponents 行段: 5 行的风格注入 (调用方保证 hud_rows.len()>=5)
    fn style_rows(&mut self) {
        let ctx = &self.ctx;
        self.hud_rows[0].map_inner(|inner| {
            if let MiniHudComponentInner::Row0(r) = inner {
                // PORT: (int) ctx.aoaLength — double→int 截断 (JLS 5.1.3)
                r.set_style(ctx.right_draw, ctx.line_width, ctx.aoa_length as i32);
            }
        });
        self.hud_rows[1].map_inner(|inner| {
            if let MiniHudComponentInner::Row1(r) = inner {
                r.set_style(ctx.right_draw);
            }
        });
        self.hud_rows[2].map_inner(|inner| {
            if let MiniHudComponentInner::Row2(r) = inner {
                r.base.set_style(ctx.hud_font_size);
            }
        });
        self.hud_rows[3].map_inner(|inner| {
            if let MiniHudComponentInner::Row3(r) = inner {
                r.set_style(ctx.hud_font_size);
            }
        });
        self.hud_rows[4].map_inner(|inner| {
            if let MiniHudComponentInner::Row4(r) = inner {
                r.set_style(
                    ctx.hud_font_size,
                    ctx.right_draw,
                    ctx.half_line,
                    ctx.line_width,
                    ctx.stroke_thick_w,
                    ctx.stroke_thin_w,
                );
            }
        });
    }

    /// applyStyleToComponents 尾段: throttleBar 的响应式高度
    fn style_throttle_bar(&mut self) {
        let ctx = &self.ctx;
        self.throttle_bar.map_comp(|c| {
            if let MiniHudComponentInner::ThrottleBar(t) = &mut c.inner {
                // Re-calc explicit height for ThrottleBar if needed or use existing
                // throttley_max
                // Standardizing to relative size: 4.8 lines high (closer to legacy 4.75)
                let responsive_height = (ctx.hud_font_size as f64 * 4.8) as i32;
                t.set_style_context(responsive_height, ctx.bar_width);
                c.throttle_length = responsive_height;
                c.throttle_thickness = ctx.bar_width;
            }
        });
    }

    /// Java initModernLayout() (L652-763) — 树构建委托
    /// minihud_layout::build_mihud_layout (spec 表快照), 此处组 parts。
    pub(super) fn init_modern_layout<S: HUDSettings>(&mut self, settings: &S) {
        let cfg = MiniHudLayoutConfig {
            // Java L654: hudSettings.isDisplayCrosshair()
            // (= getBool("displayCrosshair", false), ConfigurationService 兜底)
            display_crosshair: settings.is_display_crosshair(),
            // Java L668: hudSettings.getBool("enableLayoutDebug", false)
            enable_layout_debug: settings.get_bool("enableLayoutDebug", false),
        };
        let parts = MiniHudParts {
            rows: self.hud_rows.clone(),
            flap_angle_bar: self.flap_angle_bar.clone(),
            speed_ratio_bar: self.speed_ratio_bar.clone(),
            throttle_bar: self.throttle_bar.clone(),
            attitude_indicator_gauge: self.attitude_indicator_gauge.clone(),
            compass_gauge: self.compass_gauge.clone(),
            // Java 组件恒建但节点仅 displayCrosshair 才建 (build 内裁剪);
            // overlay 侧 handle 恒持 (components 分发序完整)
            crosshair_gauge: Some(self.crosshair_gauge.clone()),
        };
        self.layout = build_mihud_layout(
            &cfg,
            parts,
            self.ctx.width,
            self.ctx.height,
            // Use lineHeight from font size for responsive scaling
            self.ctx.hud_font_size as f64,
        );
    }
}
