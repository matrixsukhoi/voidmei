//! minihud: MiniHUDOverlay 主体 C 类语义复刻 (src/ui/overlay/MiniHUDOverlay.java)
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | [`MinimalHudContext`] | src/ui/overlay/MinimalHUDContext.java | 不可变配置快照: 全部派生量 (字号/线宽/罗盘直径/rightDraw) 从 crossScale×dpiScale 级联; 字体 = 三份 BOLD 字号档 |
//! | [`MiniHudComponent`]+[`CompCell`] | ui/component/HUDComponent.java + AbstractHUDComponent.java | 组件接口的组装层 seam: getPreferredSize/isVisible/setVisible/onDataUpdate; 异构组件装箱为枚举 |
//! | [`MiniHudOverlay`] | src/ui/overlay/MiniHUDOverlay.java | 编排器: 组件创建 → 风格/模板注入 → DAG 布局 (minihud_layout::build_mihud_layout) → 渲染循环 (doLayout+render+drawBlinkX) |
//! | [`minihud_overlay_spec`] | Controller.java:671 registerWithPreview("crosshairSwitch") | OverlayHost 挂载: render 闭包持共享句柄, 数据侧经 [`MiniHudHandle`] 外部喂入 |
//!
//! 渲染循环 (Java paintComponent L241-256):
//! `engine.do_layout()` (惰性拓扑 + 锚点求解) → `engine.render(cb)` (可见节点按拓扑序
//! 逐个 `component.draw(g,x,y)`, debug 开启时紧跟 1px 调试框) → `draw_blink_x`
//! (致命警告 X, 压在 HUD 内容之上)。
//!
//! 零分配纪律 (手册 §11.4): draw 路径不 new — 字体/颜色经 [`MiniHudFonts`] Rc 共享,
//! 组件句柄 [`CompCell`] 克隆仅是引用计数; Java 侧对应 "严禁在 draw() 循环中 new
//! Color/Font" (缓存复用)。
//!
//! 映射裁决:
//! - Java `List<HUDComponent> components` (initComponentsLayout 添加序) 与布局引擎
//!   节点图**共享同一批组件对象** → [`CompCell`](Rc<RefCell>) 双持: overlay 具名字段
//!   (风格/模板/可见性写入口) + engine 节点负载 (渲染读出口), Java 引用共享语义落地。
//! - `Math.round` 双语义 (§2.3): Math.round(float)→int 与 Math.round(double)→long→
//!   (int) 窄化 (§2.2 双转) 分别落 [`java_round_f32`]/[`java_round_long_narrowed`]。
//! - `String.format` 的 %N.Mf / %Ns / %Nd → vm_core::base::format 收敛点
//!   (`java_f`/`pad_width`, 重构波13 收割本地副本)。
//! - Application 静态色 (colorNum/colorShadeShape) → gauges_bars 常量 (同源)。
//! - Application.dpiScale (LIFETIMES §1.2 Env 只读) → 参数注入 (调用方持 Env)。
//! - Font(family, BOLD, size) 的家族名 → Rust 按字体文件路径加载 (font.rs 只吃
//!   文件); MonoNumFont 的 cfg 缺省 "Sarasa Mono SC" 映射到随包
//!   sarasa-mono-sc-bold.ttf, 由调用方解析路径。
//! - crosshairImageScaled 纹理链 (MinimalHUDContext.java:161-178) 不迁移 —
//!   gauge_crosshair.rs 头部裁决: 软件矢量路径是唯一视觉语义。
//! - Java 死字段 (hudCheckMili/realSpdPitch/firstDraw/throttley/throttleColor/
//!   inAction/disableAttitude) 保真保留 (§2.10 + hud_layout_node ignoreBounds
//!   先例: write-only 状态不删), 各带 PORT 注。

use crate::render::primitives;
use vm_core::base::format::{java_f, java_round_f32, pad_width};
use crate::render::palette::{aa, colors};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use vm_core::fm::data::FmData;
use vm_core::config::config_api::HUDSettings;
use vm_core::base::event::event_payload::EventPayload;
use vm_core::derived::hud_calculator::{self, HudColors};
use vm_core::telemetry::parser::{Indicators, State};
use vm_core::derived::hud_data::HUDData;
use crate::layout::hud_layout_node::{Dimension, HasPreferredSize, HUDLayoutNodeExt};
use vm_core::formula::registry::FormulaView;

use crate::render::font::LoadedFont;
use crate::overlays::attitude::AttitudeIndicatorGauge;
use crate::overlays::compass::CompassGauge;
use crate::overlays::crosshair::CrosshairGauge;
use crate::overlays::bars::{FlapAngleBar, LinearGauge, SpeedRatioBar};
use crate::platform::host::{OverlaySpec, ReinitFn};
use crate::platform::reinit::ReinitParams;
use crate::layout::minihud_layout::{build_mihud_layout, AutoSizingPlan, BuiltMiniHudLayout, HasVisibility, MiniHudLayoutConfig, MiniHudParts, ModernHUDLayoutEngine};
use crate::render::canvas::PixCanvas;
use crate::overlays::rows::{HUDAkbRow, HUDEnergyRow, HUDMechanizationRow, HUDManeuverRow, HUDTextRow};
use crate::overlays::warning::WarningBlinkHost;

// ---------------------------------------------------------------------------
// Java Math / printf 复刻 (§2.3/§2.2; 取整族收敛 vm_core::base::format)
// ---------------------------------------------------------------------------

/// Java `(int) Math.round(double)`: round 返回 long, (int) 窄化取低 32 位
/// (§2.2 双转; 值域内与饱和无差, 防御性对齐 Java 溢出行为)
fn java_round_long_narrowed(x: f64) -> i32 {
    let l = (x + 0.5).floor() as i64;
    (l as u32) as i32
}

/// Java `String.format("%Nd", v)` = pad_width(十进制) 组合: 右对齐补空格
/// (v 为 i32, 无舍入; 算法本体在 vm_core::base::format::pad_width)
fn fmt_d(v: i32, width: usize) -> String {
    pad_width(v.to_string(), width, false)
}

// ---------------------------------------------------------------------------
// MinimalHUDContext (src/ui/overlay/MinimalHUDContext.java 一比一)
// ---------------------------------------------------------------------------

/// ctx 的三份字体 (Java drawFont/drawFontSmall/drawFontSSmall; BOLD 由字体文件承载)
#[derive(Clone)]
pub struct MiniHudFonts {
    /// drawFont: BOLD hudFontSize
    pub draw: Rc<LoadedFont>,
    /// drawFontSmall: BOLD hudFontSizeSmall
    pub small: Rc<LoadedFont>,
    /// drawFontSSmall: BOLD hudFontSize/2
    pub s_small: Rc<LoadedFont>,
}

/// Immutable context object holding pre-calculated metrics and resources for
/// MinimalHUD. Generated from HUDSettings. (Java javadoc 原文)
///
/// PORT: BasicStroke 字段按 rows.rs HUDManeuverRow 口径折为 f32 宽度 —
/// CAP_ROUND+JOIN_ROUND 由 PixCanvas::draw_line 线型族固定承载。
#[derive(Clone)]
pub struct MinimalHudContext {
    // --- Layout Dimensions ---
    pub width: i32,
    pub height: i32,
    pub hud_font_size: i32,
    pub hud_font_size_small: i32,
    pub window_x: i32,
    pub window_y: i32,

    // --- Component Metrics ---
    pub cross_scale: i32,
    pub cross_x: i32,
    pub cross_y: i32,
    pub round_compass: i32,
    pub right_draw: i32,
    pub compass_diameter: i32,
    pub compass_radius: i32,
    pub compass_inner_mark_radius: i32,
    pub aoa_length: f64,

    // --- Styling ---
    pub line_width: i32,
    pub bar_width: i32,
    pub half_line: i32,
    /// drawFont/drawFontSmall/drawFontSSmall (Java Font 字段; Rust 句柄共享)
    pub fonts: MiniHudFonts,
    /// strokeThick = BasicStroke(halfLine+2, CAP_ROUND, JOIN_ROUND) 的宽度
    pub stroke_thick_w: f32,
    /// strokeThin = BasicStroke(halfLine, ...) 的宽度
    pub stroke_thin_w: f32,

    // --- Resources ---
    // PORT: crosshairImageScaled (纹理准星双线性缩放缓存) 不迁移 —
    // gauge_crosshair.rs 头注裁决: 软件矢量路径为唯一视觉语义, 配置项裁剪。
}

impl MinimalHudContext {
    /// Factory method to create a context from settings.
    /// Contains all the detailed calculation logic previously in
    /// MinimalHUD.reinitConfig. (Java javadoc 原文)
    ///
    /// DPI Scaling: All pixel dimensions are multiplied by dpiScale
    /// (参数注入, LIFETIMES Env; Java 读 Application.dpiScale)。
    ///
    /// PORT: Java `new Font(...)` 恒成功 (家族缺失时 AWT 兜底字体); Rust
    /// LoadedFont::new 读文件可失败 → Result。font_path 由调用方从 settings 的
    /// 字体族名解析 (cfg 缺省 "Sarasa Mono SC" → 随包 sarasa-mono-sc-bold.ttf)。
    pub fn create<S: HUDSettings>(
        settings: &S,
        dpi_scale: f64,
        font_path: &Path,
    ) -> Result<Self, String> {
        // 1. Basic Metrics - apply DPI scaling
        // PORT: (int) Math.round(double) = round→long→int 窄化 (§2.2 双转)
        let base_cross_scale = settings.get_crosshair_scale();
        let cross_scale = java_round_long_narrowed(base_cross_scale as f64 * dpi_scale);

        let f_add = settings.get_font_size_add();
        // Font size is derived from crossScale (already scaled)
        let mut hud_font_size = cross_scale / 4 + java_round_long_narrowed(f_add as f64 * dpi_scale);
        // Ensure minimum size to prevent crash
        if hud_font_size < 8 {
            hud_font_size = 8;
        }

        let bar_width = hud_font_size / 4;
        let mut line_width = if hud_font_size / 10 == 0 { 1 } else { hud_font_size / 10 };

        // 2. Window Dimensions - derived from scaled crossScale
        // PORT: (int)(double) 强转 = JLS 5.1.3 截断+饱和, 与 Rust as i32 一致
        let width = if !settings.is_display_crosshair() {
            (cross_scale as f64 * 2.25) as i32 - hud_font_size
        } else {
            (cross_scale as f64 * 2.25) as i32
        };
        // PORT: 两个独立 (int) 强转后再相加 (Java 原样, 不合并为一个表达式)
        let height = (cross_scale as f64 * 1.5) as i32 + (hud_font_size as f64 * 3.5) as i32;

        let window_x = settings.get_window_x(width);
        let window_y = settings.get_window_y(height);

        let cross_x = width / 2;
        let cross_y = height / 2;

        // 3. Component Details - derived from scaled hudFontSize
        if line_width == 0 {
            line_width = 1;
        }

        // PORT: Math.round(hudFontSize * 0.8f) — int*float 提升 float,
        // Math.round(float)→int (非 double 版, §2.3 双语义)
        let round_compass = java_round_f32(hud_font_size as f32 * 0.8f32);

        // Dynamic rightDraw calculation (WYSWYG Overlap Fix)
        // Standard value (~5 chars): 3.5f * fontSize
        // Labeled value (~9 chars): 5.5f * fontSize
        let multiplier = if !settings.is_speed_label_disabled()
            || !settings.is_altitude_label_disabled()
            || !settings.is_sep_label_disabled()
        {
            5.5f32
        } else {
            3.5f32
        };
        // PORT: (int)(int * float) — float 乘法后截断 (保 f32 链, §2.12)
        let right_draw = (hud_font_size as f32 * multiplier) as i32;

        let compass_diameter = java_round_long_narrowed(2.0 * hud_font_size as f64 * 0.618);
        let compass_radius = java_round_long_narrowed(compass_diameter as f64 / 2.0);
        let compass_inner_mark_radius = java_round_long_narrowed(0.618 * compass_diameter as f64);

        // Adjusted for dynamic rightDraw
        let aoa_length = right_draw as f64 - hud_font_size as f64 / 1.5;

        // 4. Strokes & Fonts - scaled stroke widths for crisp lines (Java 注释)
        let half_line = if line_width / 2 == 0 {
            1
        } else {
            java_round_f32(line_width as f32 / 2.0f32)
        };
        let stroke_thick_w = (half_line + 2) as f32;
        let stroke_thin_w = half_line as f32;

        let n_font = settings.get_num_font();
        let draw = Rc::new(LoadedFont::new(font_path, hud_font_size)?);
        // PORT: (int)(hudFontSize * 0.75f) — float 链截断
        let hud_font_size_small = (hud_font_size as f32 * 0.75f32) as i32;
        let small = Rc::new(LoadedFont::new(font_path, hud_font_size_small)?);
        // PORT: new Font(nFont, BOLD, hudFontSize / 2) — int 除法在造 Font 之前
        let s_small = Rc::new(LoadedFont::new(font_path, hud_font_size / 2)?);
        let _ = n_font; // 家族名已由 font_path 承载 (模块头映射裁决)

        // 5. Resource Loading (IO) — 纹理准星链不迁移 (模块头 PORT 注)

        vm_core::base::logger::info(
            "MinimalHUD",
            &format!(
                "MinimalHUD Config: Width={}, Height={}, CrossWidth={}",
                width, height, cross_scale
            ),
        );

        Ok(MinimalHudContext {
            width,
            height,
            hud_font_size,
            hud_font_size_small,
            window_x,
            window_y,
            cross_scale,
            cross_x,
            cross_y,
            round_compass,
            right_draw,
            compass_diameter,
            compass_radius,
            compass_inner_mark_radius,
            aoa_length,
            line_width,
            bar_width,
            half_line,
            fonts: MiniHudFonts { draw, small, s_small },
            stroke_thick_w,
            stroke_thin_w,
        })
    }
}

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
pub struct CompCell(Rc<RefCell<MiniHudComponent>>);

impl Clone for CompCell {
    fn clone(&self) -> Self {
        CompCell(Rc::clone(&self.0))
    }
}

impl CompCell {
    fn new(inner: MiniHudComponentInner, fonts: Rc<MiniHudFonts>) -> Self {
        CompCell(Rc::new(RefCell::new(MiniHudComponent::new(inner, fonts))))
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
// MiniHUDOverlay (src/ui/overlay/MiniHUDOverlay.java 主体)
// ---------------------------------------------------------------------------

/// overlay 的共享数据句柄 (host render 闭包与数据喂入方各持一份;
/// 单线程 RefCell — host 是主循环单线程独占, 上层 Controller 须同线程喂入)
pub type MiniHudHandle = Rc<RefCell<MiniHudOverlay>>;

/// MinimalHUD overlay for displaying compact flight information.
/// Being migrated to event-driven architecture. (Java 类 javadoc 原文)
pub struct MiniHudOverlay {
    ctx: MinimalHudContext,
    /// 字体快照 (与 ctx.fonts 同源; reinit_config 重建 ctx 时同步换)
    fonts: Rc<MiniHudFonts>,
    /// 字体文件路径 (reinit_config 重建字体用)
    font_path: PathBuf,
    /// Application.dpiScale 参数注入 (LIFETIMES Env 只读快照)
    dpi_scale: f64,
    /// Java service 字段的在场语义 (null = 预览模式; 遥测数据经参数喂入)
    service_present: bool,

    // Reactive Components List (initComponentsLayout 添加序 = onDataUpdate 分发序)
    components: Vec<CompCell>,

    // 具名组件句柄 (Java 字段; 与布局节点图共享同一对象)
    crosshair_gauge: CompCell,
    flap_angle_bar: CompCell,
    compass_gauge: CompCell,
    attitude_indicator_gauge: CompCell,
    speed_ratio_bar: CompCell,
    /// hudRows (5 行; 行链 spec 按 rows.len() 截断)
    hud_rows: Vec<CompCell>,
    throttle_bar: CompCell,

    // 0. Aux Overlays — warningOverlay 组合于 WarningBlinkHost (drawBlinkX 链)
    warning: WarningBlinkHost,

    // --- Modern Layout Engine Integration ---
    layout: BuiltMiniHudLayout<CompCell>,

    // Java 遗留/只写字段 (模块头映射裁决; §2.10 保真保留)
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
    #[allow(dead_code)] // PORT: Java MiniHUDOverlay.java:286 同名死字段
    real_spd_pitch: f64,
    /// Java private boolean firstDraw (reinitConfig 写 true; 无读者)
    first_draw: bool,
    /// Java public long hudCheckMili (死字段 — 全库无读写, 声明保真保留)
    #[allow(dead_code)] // PORT: Java MiniHUDOverlay.java:283 同名死字段
    hud_check_mili: i64,
    /// update_legacy_components 更新, update_components 预览路径读
    maneuver_index: f64,
    maneuver_index_len: i32,
    maneuver_index_len10: i32,
    maneuver_index_len20: i32,
    maneuver_index_len30: i32,
    maneuver_index_len40: i32,
    maneuver_index_len50: i32,

    // Java public boolean warnRH / warnVne (updateFromEvent 写; 外层消费)
    pub warn_rh: bool,
    pub warn_vne: bool,

    // Throttling for refresh rate (Java:412-415)
    refresh_interval: i64,
    last_refresh_time: i64,
}

impl MiniHudOverlay {
    /// Java init(Controller c, Service s, HUDSettings settings) (L217-281)。
    /// `service_loop_interval_ms` = controller.serviceLoopIntervalMs (blinkTicks/
    /// refreshInterval 同源); `service_present` = (s != null); Rust 侧 service /
    /// controller 不入结构 — 遥测经 [`on_flight_data`] 参数喂入 (单线程 host 模型,
    /// 模块头映射裁决)。
    pub fn init<S: HUDSettings>(
        service_present: bool,
        service_loop_interval_ms: i64,
        settings: &S,
        dpi_scale: f64,
        font_path: &Path,
    ) -> Result<Self, String> {
        vm_core::base::logger::info("MinimalHUD", "init called");
        let ctx = MinimalHudContext::create(settings, dpi_scale, font_path)?;
        let fonts = Rc::new(ctx.fonts.clone());
        // Java initComponentsLayout 之前各组件字段为 null → 首轮 reinitConfig 的
        // applyStyle/updateComponents 对组件全空转 (initModernLayout 空表早退)。
        // Rust 无 null: 占位组件即刻可查 (空引擎不渲染), initComponentsLayout
        // 建齐真身后整体替换 — 调用序列与 Java 逐行对应。
        let placeholder = |inner: MiniHudComponentInner| CompCell::new(inner, Rc::clone(&fonts));
        let empty_engine = ModernHUDLayoutEngine::new(ctx.width, ctx.height);
        let mut overlay = MiniHudOverlay {
            crosshair_gauge: placeholder(MiniHudComponentInner::Crosshair(CrosshairGauge::new())),
            flap_angle_bar: placeholder(MiniHudComponentInner::FlapBar(FlapAngleBar::new())),
            compass_gauge: placeholder(MiniHudComponentInner::Compass(CompassGauge::new(
                ctx.round_compass,
            ))),
            attitude_indicator_gauge: placeholder(MiniHudComponentInner::Attitude(
                AttitudeIndicatorGauge::new(),
            )),
            speed_ratio_bar: placeholder(MiniHudComponentInner::SpeedRatioBar(
                SpeedRatioBar::new(),
            )),
            throttle_bar: placeholder(MiniHudComponentInner::ThrottleBar(LinearGauge::new(
                "ThrottleBar", 110, true,
            ))),
            fonts,
            font_path: font_path.to_path_buf(),
            dpi_scale,
            service_present,
            components: Vec::new(),
            hud_rows: Vec::new(),
            warning: WarningBlinkHost::new(service_loop_interval_ms),
            layout: BuiltMiniHudLayout { engine: empty_engine, sizing: None },
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
            maneuver_index_len10: 0,
            maneuver_index_len20: 0,
            maneuver_index_len30: 0,
            maneuver_index_len40: 0,
            maneuver_index_len50: 0,
            warn_rh: false,
            warn_vne: false,
            refresh_interval: service_loop_interval_ms,
            last_refresh_time: 0,
            ctx,
        };

        overlay.reinit_config(settings)?;

        if overlay.aoa_y > overlay.ctx.right_draw {
            overlay.aoa_y = overlay.ctx.right_draw;
        }
        overlay.aoa_color = colors().num;
        overlay.aoa_bar_color = colors().num;

        overlay.init_components_layout(settings);

        // PORT: Java 读 service 字段 — 游戏模式 S1.start() 先于 overlay 激活
        // (Controller.java:633-641), sState 可能已轮询出值, throttle 分支可吃到
        // 真值; 组装层此阶段无遥测口可传 → None, throttle 闪 0, 由下一放行的
        // on_flight_data (≤refreshInterval) 覆盖, 影响 ≤1 帧
        overlay.update_components(settings, None);

        Ok(overlay)
    }

    /// Java reinitConfig() (L127-159) — ctx 快照重建 + 模板 + 风格 + 布局引擎重建。
    /// PORT: setBounds (L143-146) 的窗口几何副作用归 OverlayHost (spec 尺寸取
    /// applyAutoSizing 计划); Java 先 setBounds 再被 applyAutoSizing 的
    /// window.setSize 覆盖, 净效果 = 内容包围盒 + 2×LAYOUT_PADDING。
    pub fn reinit_config<S: HUDSettings>(&mut self, settings: &S) -> Result<(), String> {
        vm_core::base::logger::info("MinimalHUD", "reinitConfig called");

        // Create Immutable Context
        self.ctx = MinimalHudContext::create(settings, self.dpi_scale, &self.font_path)?;
        self.fonts = Rc::new(self.ctx.fonts.clone());

        // 1. Refresh mock data and templates (WYSIWYG support)
        self.refresh_templates(settings);

        // Apply dimensions (Initial guess, will be refined by dynamic layout)
        // (Java 注释原文; setBounds → 宿主, 见方法头 PORT 注)

        // 2. Sync Component State (Style & Visibility) BEFORE Layout
        // This ensures getContentBounds() sees the correct visible components
        //
        self.apply_style_to_components(settings);
        // PORT: Java 此处 updateComponents() 读 service 字段 — 游戏模式 WYSIWYG
        // reinit 时 sState 可非 null (throttle 吃真值); Rust 恒传 None → 油门条
        // 闪 0, 下一放行 on_flight_data (≤refreshInterval) 修复, 影响 ≤1 帧
        self.update_components(settings, None);

        // 3. Setup Layout Engine & Dynamic Sizing
        self.init_modern_layout(settings);

        self.first_draw = true;
        // repaint() → 宿主 render_tick 标脏 (host 脏检查逐字节, 无需显式)
        Ok(())
    }

    /// Java refreshTemplates() (L161-208)
    fn refresh_templates<S: HUDSettings>(&mut self, settings: &S) {
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
    fn set_row_templates(&mut self) {
        let (l0, laoa, l1, lrel, l2, l3, l4) = (
            self.lines[0].clone(),
            self.line_aoa.clone(),
            self.lines[1].clone(),
            self.rel_energy.clone(),
            self.lines[2].clone(),
            self.lines[3].clone(),
            self.lines[4].clone(),
        );
        if let MiniHudComponentInner::Row0(r) = &mut self.hud_rows[0].0.borrow_mut().inner {
            r.set_template(Some(&l0), Some(&laoa));
        }
        if let MiniHudComponentInner::Row1(r) = &mut self.hud_rows[1].0.borrow_mut().inner {
            r.set_template(Some(&l1), Some(&lrel));
        }
        if let MiniHudComponentInner::Row2(r) = &mut self.hud_rows[2].0.borrow_mut().inner {
            // PORT: Java MiniHUDOverlay.java:204 强转 HUDTextRow, 但 setTemplate 非
            // final 且被 HUDMechanizationRow 同签名覆写 → 虚分派走覆写 (super + 三段
            // 模板重解析)。须调完整 set_template 而非仅基座, 否则模板变化时三段
            // 占位宽滞留旧值 (Java 会重解析)
            r.set_template(Some(&l2));
        }
        if let MiniHudComponentInner::Row3(r) = &mut self.hud_rows[3].0.borrow_mut().inner {
            r.set_template(Some(&l3));
        }
        if let MiniHudComponentInner::Row4(r) = &mut self.hud_rows[4].0.borrow_mut().inner {
            r.base.set_template(Some(&l4));
        }
    }

    /// Java initComponentsLayout() (L524-589)
    fn init_components_layout<S: HUDSettings>(&mut self, settings: &S) {
        self.components.clear(); // Ensure list is clean on re-init

        let fonts = Rc::clone(&self.fonts);
        let cell = |inner: MiniHudComponentInner| CompCell::new(inner, Rc::clone(&fonts));

        // 0. Aux Overlays — warningOverlay 已由 WarningBlinkHost 组合持有 (Java:528)
        self.flap_angle_bar = cell(MiniHudComponentInner::FlapBar(FlapAngleBar::new()));
        self.components.push(self.flap_angle_bar.clone());

        // New SpeedRatioBar
        self.speed_ratio_bar = cell(MiniHudComponentInner::SpeedRatioBar(SpeedRatioBar::new()));
        self.components.push(self.speed_ratio_bar.clone());

        // 1. Compass — 构造注入 ctx.roundCompass (Java:537)
        self.compass_gauge = cell(MiniHudComponentInner::Compass(CompassGauge::new(
            self.ctx.round_compass,
        )));
        self.components.push(self.compass_gauge.clone());

        // 2. Attitude
        self.attitude_indicator_gauge =
            cell(MiniHudComponentInner::Attitude(AttitudeIndicatorGauge::new()));
        self.components.push(self.attitude_indicator_gauge.clone());

        // 3. Crosshair — 无条件入 components (节点是否建由 cfg 决定, Java:545-546)
        self.crosshair_gauge = cell(MiniHudComponentInner::Crosshair(CrosshairGauge::new()));
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
        self.throttle_bar =
            cell(MiniHudComponentInner::ThrottleBar(LinearGauge::new("ThrottleBar", 110, true)));
        self.components.push(self.throttle_bar.clone());

        // Ensure everything is styled and updated before layout & sizing (Java 注释)
        self.apply_style_to_components(settings);
        // PORT: 同 init 尾部 — Java 读 service 字段, 此处 None (throttle 闪 0,
        // 下一放行 on_flight_data 修复, ≤1 帧)
        self.update_components(settings, None);

        self.init_modern_layout(settings);
    }

    /// Java applyStyleToComponents() (L591-647)
    fn apply_style_to_components<S: HUDSettings>(&mut self, settings: &S) {
        if self.components.is_empty() {
            // 恒在, 以 components 清单空近似同一守卫 (占位件随后被整体替换)
            return;
        }
        // 字体档换新 (Java 各 setStyle 的 Font 形参; reinit 重建 ctx 后生效)
        let fonts = Rc::clone(&self.fonts);
        for c in &self.components {
            c.set_fonts(Rc::clone(&fonts));
        }

        let ctx = &self.ctx;
        {
            let mut c = self.speed_ratio_bar.0.borrow_mut();
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
        }
        {
            let mut c = self.crosshair_gauge.0.borrow_mut();
            if let MiniHudComponentInner::Crosshair(g) = &mut c.inner {
                // PORT: Java useTextureCrosshair 纹理分支 (L605-607) 不迁移 —
                // gauge_crosshair.rs 裁决, 软件路径即唯一视觉语义
                g.set_style_context(settings.get_crosshair_scale());
            }
        }
        {
            let mut c = self.flap_angle_bar.0.borrow_mut();
            if let MiniHudComponentInner::FlapBar(b) = &mut c.inner {
                // Dynamic width
                let responsive_width = (ctx.hud_font_size as f64 * 6.0) as i32;
                b.set_style_context(responsive_width, ctx.line_width + 2);
                c.flap_total_width = responsive_width;
                c.flap_bar_height = ctx.line_width + 2;
            }
        }
        {
            let mut c = self.compass_gauge.0.borrow_mut();
            if let MiniHudComponentInner::Compass(g) = &mut c.inner {
                g.set_style_context(
                    ctx.round_compass,
                    ctx.line_width,
                    ctx.hud_font_size,
                    ctx.hud_font_size_small,
                );
                g.set_inertial_mode(settings.is_attitude_indicator_inertial_mode());
            }
        }
        {
            let mut c = self.attitude_indicator_gauge.0.borrow_mut();
            if let MiniHudComponentInner::Attitude(g) = &mut c.inner {
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
        }
        // Synchronize styles for Rows
        if self.hud_rows.len() >= 5 {
            {
                let mut c = self.hud_rows[0].0.borrow_mut();
                if let MiniHudComponentInner::Row0(r) = &mut c.inner {
                    // PORT: (int) ctx.aoaLength — double→int 截断 (JLS 5.1.3)
                    r.set_style(ctx.right_draw, ctx.line_width, ctx.aoa_length as i32);
                }
            }
            {
                let mut c = self.hud_rows[1].0.borrow_mut();
                if let MiniHudComponentInner::Row1(r) = &mut c.inner {
                    r.set_style(ctx.right_draw);
                }
            }
            {
                let mut c = self.hud_rows[2].0.borrow_mut();
                if let MiniHudComponentInner::Row2(r) = &mut c.inner {
                    r.base.set_style(ctx.hud_font_size);
                }
            }
            {
                let mut c = self.hud_rows[3].0.borrow_mut();
                if let MiniHudComponentInner::Row3(r) = &mut c.inner {
                    r.set_style(ctx.hud_font_size);
                }
            }
            {
                let mut c = self.hud_rows[4].0.borrow_mut();
                if let MiniHudComponentInner::Row4(r) = &mut c.inner {
                    r.set_style(
                        ctx.hud_font_size,
                        ctx.right_draw,
                        ctx.half_line,
                        ctx.line_width,
                        ctx.stroke_thick_w,
                        ctx.stroke_thin_w,
                    );
                }
            }
        }
        {
            let mut c = self.throttle_bar.0.borrow_mut();
            if let MiniHudComponentInner::ThrottleBar(t) = &mut c.inner {
                // Re-calc explicit height for ThrottleBar if needed or use existing
                // throttley_max
                // Standardizing to relative size: 4.8 lines high (closer to legacy 4.75)
                let responsive_height = (ctx.hud_font_size as f64 * 4.8) as i32;
                t.set_style_context(responsive_height, ctx.bar_width);
                c.throttle_length = responsive_height;
                c.throttle_thickness = ctx.bar_width;
            }
        }
    }

    /// Java updateComponents() (L309-402)。
    /// `service` = Java service 字段处的遥测读取口 (throttle 分支);
    /// 行 0/1 预览串分支按 Java 语义读 **service 字段在场性** (init 决定),
    /// 不随本参数摆动 (WYSIWYG 游戏内 reinit 亦不推预览串)。
    fn update_components<S: HUDSettings>(
        &mut self,
        settings: &S,
        service: Option<&dyn FormulaView>,
    ) {
        let text_visible = settings.draw_hud_text();

        let enable_flap_bar = settings.enable_flap_angle_bar();
        self.flap_angle_bar.set_visible(text_visible && enable_flap_bar);
        let show_attitude = settings.show_attitude_gauge();
        self.compass_gauge.set_visible(text_visible && !show_attitude);
        self.attitude_indicator_gauge
            .set_visible(text_visible && show_attitude && !self.disable_attitude);
        // Dynamic position based on current Width/CrossX —
        // Position handled by ModernHUDLayoutEngine (Java 注释原文; ctx 空块不复刻)
        self.crosshair_gauge.set_visible(settings.is_display_crosshair());
        let show_speed = settings.show_speed_bar();
        self.throttle_bar.set_visible(text_visible && !show_speed);
        self.speed_ratio_bar.set_visible(text_visible && show_speed);
        let master = settings.draw_hud_text();

        // 组件级独立可见性控制
        if self.hud_rows.len() >= 5 {
            // Row 0: Speed + AoA — 两个独立组件
            let row0_speed = master && settings.show_hud_speed();
            let row0_aoa = master && settings.show_hud_aoa();
            self.hud_rows[0].set_visible(row0_speed || row0_aoa);
            {
                let mut c = self.hud_rows[0].0.borrow_mut();
                if let MiniHudComponentInner::Row0(r) = &mut c.inner {
                    r.set_show_speed(row0_speed);
                    r.set_show_aoa(row0_aoa);
                }
            }

            // Row 1: Altitude + Energy — 两个独立组件
            let row1_alt = master && settings.show_hud_altitude();
            let row1_energy = master && settings.show_hud_energy();
            self.hud_rows[1].set_visible(row1_alt || row1_energy);
            {
                let mut c = self.hud_rows[1].0.borrow_mut();
                if let MiniHudComponentInner::Row1(r) = &mut c.inner {
                    r.set_show_altitude(row1_alt);
                    r.set_show_energy(row1_energy);
                }
            }

            // Row 2: 襟翼/可变翼 + 减速板 + 起落架 — 三个独立组件
            let row2_flaps = master && settings.show_hud_flaps();
            let row2_brk = master && settings.show_hud_airbrake();
            let row2_gear = master && settings.show_hud_gear();
            self.hud_rows[2].set_visible(row2_flaps || row2_brk || row2_gear);
            {
                let mut c = self.hud_rows[2].0.borrow_mut();
                if let MiniHudComponentInner::Row2(r) = &mut c.inner {
                    r.set_show_flaps(row2_flaps);
                    r.set_show_airbrake(row2_brk);
                    r.set_show_gear(row2_gear);
                }
            }

            // Row 3: 单组件（爬升率）
            self.hud_rows[3].set_visible(master && settings.show_hud_sep());

            // Row 4: G-force + ManeuverBar — 两个独立组件
            let row4_g_load = master && settings.show_hud_g_load();
            let row4_bar = master && settings.show_hud_maneuver_bar();
            self.hud_rows[4].set_visible(row4_g_load || row4_bar);
            {
                let mut c = self.hud_rows[4].0.borrow_mut();
                if let MiniHudComponentInner::Row4(r) = &mut c.inner {
                    r.set_show_g_load(row4_g_load);
                    r.set_show_maneuver_bar(row4_bar);
                }
            }
        }

        if self.hud_rows.len() >= 5 {
            // Row 0, 1: Only update in preview mode (service == null)
            // In game mode, they are updated via onDataUpdate() from FlightDataEvent
            // (Java 注释原文; service==null 即 init 的 service_present=false)
            if !self.service_present {
                let (l0, laoa, aoa_y, a_col, ab_col) = (
                    self.lines[0].clone(),
                    self.line_aoa.clone(),
                    self.aoa_y,
                    self.aoa_color,
                    self.aoa_bar_color,
                );
                {
                    let mut c = self.hud_rows[0].0.borrow_mut();
                    if let MiniHudComponentInner::Row0(r) = &mut c.inner {
                        r.update(&l0, false, &laoa, aoa_y, a_col, ab_col);
                    }
                }
                let (l1, lrel) = (self.lines[1].clone(), self.rel_energy.clone());
                {
                    let mut c = self.hud_rows[1].0.borrow_mut();
                    if let MiniHudComponentInner::Row1(r) = &mut c.inner {
                        // 能量颜色已统一使用 Application.colorNum，不再需要传入颜色参数
                        //
                        r.update(&l1, false, &lrel);
                    }
                }
            }

            // Row 2: Standard (Flaps/Gear)
            let l2 = self.lines[2].clone();
            {
                let mut c = self.hud_rows[2].0.borrow_mut();
                if let MiniHudComponentInner::Row2(r) = &mut c.inner {
                    r.update(&l2, self.in_action);
                }
            }
            // Row 3: Standard (SEP)
            let l3 = self.lines[3].clone();
            {
                let mut c = self.hud_rows[3].0.borrow_mut();
                if let MiniHudComponentInner::Row3(r) = &mut c.inner {
                    r.update(&l3, false);
                }
            }
            // Row 4: Maneuver (G)
            let l4 = self.lines[4].clone();
            let (mi, l, l10, l20, l30, l40, l50) = (
                self.maneuver_index,
                self.maneuver_index_len,
                self.maneuver_index_len10,
                self.maneuver_index_len20,
                self.maneuver_index_len30,
                self.maneuver_index_len40,
                self.maneuver_index_len50,
            );
            {
                let mut c = self.hud_rows[4].0.borrow_mut();
                if let MiniHudComponentInner::Row4(r) = &mut c.inner {
                    r.update(&l4, false, mi, l, l10, l20, l30, l40, l50);
                }
            }
        }

        {
            let mut throttle_value = 0;
            // PORT: Java `service != null && service.sState != null` — sState 空判
            // 折入 TelemetrySource 实现域 (Service 批次); getThrottle 返回 double
            // 而 Java 读 int 字段 sState.throttle → as i32 (JLS 5.1.3 同义)
            if let Some(s) = service {
                throttle_value = s.var_value("throttle").unwrap_or(0.0) as i32;
            }
            let mut c = self.throttle_bar.0.borrow_mut();
            if let MiniHudComponentInner::ThrottleBar(t) = &mut c.inner {
                t.update(throttle_value, &fmt_d(throttle_value, 3));
            }
        }
    }

    /// Java initModernLayout() (L652-763) — 树构建委托
    /// minihud_layout::build_mihud_layout (spec 表快照), 此处组 parts。
    fn init_modern_layout<S: HUDSettings>(&mut self, settings: &S) {
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

    // --- Event-Driven Update ---

    /// Java onFlightData(FlightDataEvent) (L418-431)。
    /// 返回 false = 节流跳过 (Java return); true = 已进入 updateFromEvent。
    /// `now_ms` = System.currentTimeMillis (宿主时钟注入, 可测)。
    /// W-B 事件瘦身后直参: State/Indicators/payload 由调用方从共享 guard 借引用传入。
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
        // Throttling prevents EDT task accumulation when events arrive faster
        // than processing
        if now_ms - self.last_refresh_time < self.refresh_interval {
            return false; // Skip this update, too soon
        }
        self.last_refresh_time = now_ms;

        self.update_from_event(state, indic, payload, service, fmdata, settings, colors);
        // root.repaint() → 宿主 render_tick (脏检查逐字节, 无需显式标脏)
        true
    }

    /// Java updateFromEvent(FlightDataEvent) (L433-468)
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
        // (Java 的 FMManager.current().blkx 快照语义由调用方以 blkx=None 表达 —
        // 非 READY 句柄降级)
        let data = hud_calculator::calculate(state, indic, payload, service, fmdata, settings, colors);

        // Dispatch to Reactive Components
        for comp in &self.components {
            comp.0.borrow_mut().on_data_update(&data);
        }

        // Update Legacy Components (Bridge) & Global State
        self.warn_vne = data.warn_vne;
        self.warn_rh = data.warn_altitude;
        // blinkX = event.getPayload().fatalWarn (Java:458)
        self.warning.set_blink_x(payload.fatal_warn);

        if self.hud_rows.len() >= 5 {
            // Let's call a legacy bridge method explicitly
            self.update_legacy_components(&data);
        }

        {
            let mut c = self.throttle_bar.0.borrow_mut();
            if let MiniHudComponentInner::ThrottleBar(t) = &mut c.inner {
                t.update(data.throttle, &fmt_d(data.throttle, 3));
            }
        }
    }

    /// Java updateLegacyComponents(HUDData) (L470-496)
    // PORT(allow eq_op): lenN = N/0.5 系列统一公式在 N=0.5 时字面为 0.5/0.5
    // (Java HUDManeuverRow 调用点原样), 保真保留字面结构
    #[allow(clippy::eq_op)]
    fn update_legacy_components(&mut self, data: &HUDData) {
        // Row 0, 1, 2 are refactored (Akb, Energy, Mechanization). They use
        // onDataUpdate.
        // Row 3: SEP
        {
            let sep = data.sep_str.clone();
            let mut c = self.hud_rows[3].0.borrow_mut();
            if let MiniHudComponentInner::Row3(r) = &mut c.inner {
                r.update(&sep, false);
            }
        }
        // Row 4: Maneuver
        // ManeuverRow update signature is complex.
        {
            let (ms, mi) = (data.maneuver_state_str.clone(), data.maneuver_index);
            let (l, l10, l20, l30, l40, l50) = (
                self.maneuver_index_len,
                self.maneuver_index_len10,
                self.maneuver_index_len20,
                self.maneuver_index_len30,
                self.maneuver_index_len40,
                self.maneuver_index_len50,
            );
            let mut c = self.hud_rows[4].0.borrow_mut();
            if let MiniHudComponentInner::Row4(r) = &mut c.inner {
                r.update(&ms, false, mi, l, l10, l20, l30, l40, l50);
            }
        }
        // Note: maneuverIndexLen variables are member fields of MinimalHUD
        // calculated in legacy loop.
        let right_draw = self.ctx.right_draw;
        // PORT: (int) Math.round(double) — round→long→(int) 窄化 (§2.2 双转);
        // 求值序 (index / 0.5) * rightDraw 与 Java 左结合一致
        self.maneuver_index_len =
            java_round_long_narrowed(data.maneuver_index / 0.5 * right_draw as f64);
        self.maneuver_index_len10 = java_round_long_narrowed(0.1 / 0.5 * right_draw as f64);
        self.maneuver_index_len20 = java_round_long_narrowed(0.2 / 0.5 * right_draw as f64);
        self.maneuver_index_len30 = java_round_long_narrowed(0.3 / 0.5 * right_draw as f64);
        self.maneuver_index_len40 = java_round_long_narrowed(0.4 / 0.5 * right_draw as f64);
        self.maneuver_index_len50 = java_round_long_narrowed(0.5 / 0.5 * right_draw as f64);
    }

    /// Java paintComponent 主体 (L241-256): doLayout + render + drawBlinkX。
    /// aa = graphAASetting (生产恒 ON; false 供对拍)。
    pub fn draw(&mut self, cv: &mut PixCanvas, aa: bool) {
        // (render2d 口径)
        {
            self.layout.engine.do_layout();
            let engine = &self.layout.engine;
            engine.render(|node, x, y, dbg| {
                // dbg=None: component.draw(g, x, y); Some(color): drawDebug 的
                // 1px 线框 (ModernHUDLayoutEngine.java:187-189 drawRect(x,y,w,h))
                match dbg {
                    None => {
                        let comp = node.borrow().component.0.clone();
                        comp.borrow_mut().draw(cv, x, y, aa);
                    }
                    Some(color) => {
                        let r = node.get_pixel_rect();
                        primitives::ring1px(cv, x, y, r.width, r.height, color);
                    }
                }
            });
        }
        // drawBlinkX(g2d) — X 只盖 ctx.width × ctx.height (crosshair 双宽窗口同,
        // warning_overlay.rs 头注保真)
        let (w, h) = (self.ctx.width, self.ctx.height);
        self.warning.draw_blink_x(cv, w, h, aa);
    }

    /// 自动尺寸计划 (initModernLayout 尾部 applyAutoSizing 的窗口尺寸来源;
    /// None = Java components 空裸 return 分支, 宿主保持初始尺寸)
    pub fn sizing(&self) -> Option<AutoSizingPlan> {
        self.layout.sizing
    }

    pub fn ctx(&self) -> &MinimalHudContext {
        &self.ctx
    }
}

// ---------------------------------------------------------------------------
// OverlayHost 挂载 (Controller.java:671 registerWithPreview("crosshairSwitch"))
// ---------------------------------------------------------------------------

/// MiniHUD 的 OverlayHost 注册件: 返回 (共享句柄, spec)。
/// render 闭包持句柄克隆画帧; 数据侧 (Controller/Service 批次) 持同一句柄调
/// [`MiniHudOverlay::on_flight_data`] — host 现仅 render 通道 (overlays_field1
/// 备案), 数据钩子以共享句柄承载, 不扩 host 接口。
///
/// spec 尺寸 = applyAutoSizing 计划 (Java: setBounds 初值被 applyAutoSizing 的
/// window.setSize 覆盖, 净效果 = 内容包围盒 + 2×LAYOUT_PADDING)。
/// PORT(WYSIWYG 收口, 原"创建时快照冻结"备案): reinit 闭包随 [`ReinitParams`] 仓
/// 走 reinit_config (ctx/模板/风格/布局引擎全量重建, Java L127-159), 新 sizing()
/// 计划经返回值交 host resize_entry — 对位 Java reinitConfig→applyAutoSizing 的
/// window.setSize 副作用, 窗口不再冻结在创建尺寸。
/// `service_loop_interval_ms` / `service_present` 语义见 [`MiniHudOverlay::init`]。
pub fn minihud_overlay_spec<S: HUDSettings>(
    service_present: bool,
    service_loop_interval_ms: i64,
    settings: &S,
    dpi_scale: f64,
    font_path: &Path,
    params: &Rc<RefCell<ReinitParams>>,
) -> Result<(MiniHudHandle, OverlaySpec), String> {
    let overlay = MiniHudOverlay::init(
        service_present,
        service_loop_interval_ms,
        settings,
        dpi_scale,
        font_path,
    )?;
    let (w, h) = match overlay.sizing() {
        Some(p) => (p.new_width, p.new_height),
        // Java 空 components 裸 return: 窗口保持 setBounds 初值 (5 行恒在, 不可达)
        None => (overlay.ctx().width, overlay.ctx().height),
    };
    let handle: MiniHudHandle = Rc::new(RefCell::new(overlay));
    let render_handle = Rc::clone(&handle);
    // reinit 闭包: reinit_config(最新 hud 快照) → 新 sizing 计划 (setBounds 面);
    // 重建失败 (字体文件缺失等) 留痕并保持旧尺寸
    let reinit_handle = Rc::clone(&handle);
    let reinit_params = Rc::clone(params);
    let reinit: ReinitFn = Box::new(move || {
        let hud = reinit_params.borrow().hud.clone();
        let mut o = reinit_handle.borrow_mut();
        if let Err(e) = o.reinit_config(&hud) {
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
        OverlaySpec {
            // Java LinkedHashMap 键 = configKey (Controller.java:671)
            id: "crosshairSwitch".to_string(),
            config_key: "crosshairSwitch".to_string(),
            width: w,
            height: h,
            render: Box::new(move |cv: &mut PixCanvas| {
                // aa = 运行时仓 (cfg AAEnable 可关)
                render_handle.borrow_mut().draw(cv, aa());
            }),
            reinit: Some(reinit),
        },
    ))
}

#[cfg(test)]
mod tests;
