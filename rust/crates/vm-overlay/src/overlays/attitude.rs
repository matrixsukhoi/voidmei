//! gauge_attitude: 地平仪家族 C 类语义复刻 (最复杂件: 旋转/侧滑/pitch 刻度/双模式)
//!
//! - AttitudeIndicatorGauge — MiniHUD 地平仪: 牵引线 + 旋转 marks (下半圆弧 + 3 刻度)
//!   + pitch/侧滑双值文本; 随体/离体双模式仅翻转符号表。
//! - AttitudeOverlay — 独立地平仪窗: 橙色地面多边形 (±2 宽被窗口裁剪) + 4 条 pitch
//!   刻度 + 中线/下半圆 + 侧滑球十字 + 攻角极限线 + 航向指针对。
//!
//! Graphics2D 变换的复刻策略 (D7: 矢量基元走 tiny-skia):
//! - **旋转 marks** (IndicatorGauge): Java setTransform(rotate(θ, target)) 后连续光栅化。
//!   PixCanvas 整数基元吃不下连续旋转坐标, 故弧与刻度线均按 stroke 区域的精确几何
//!   轮廓折线单次 fill (arc_stroke_outline / line_stroke_outline, Minkowski 和):
//!   弧 = 外弧→端帽→内弧→端帽; 线段 = 矩形体+双半圆帽 (stadium)。
//!   单次合成避免半透明色 (alpha<255) 分段叠加加深伪影, 且保住 Java 连续变换的
//!   亚像素定位; 每遍内绘制序 (弧→3 线) 与 Java drawMarks 一致。
//! - **旋转多边形** (Overlay): Java 逐点 AffineTransform.transform 后 Point.setLocation
//!   取整 (floor(x+0.5)), 再 fillPolygon —— 端点先取整再连直边, 非 "连续旋转后填充",
//!   Rust 侧同序复刻 (java_round_i32), 像素级关键差异。
//! - **窗口裁剪** (Overlay): 画布光栅化天然裁剪到 [0,w)×[0,h), 画布取 w×h 即得
//!   等效 clip。
//!
//! 颜色 = 全局静态色直通 RGBA (与 gauges_bars 同源)。

use crate::render::font::LoadedFont;
use crate::render::palette::{aa, colors};
use vm_core::base::format::java_round;
use vm_core::base::format::java_round_f64;

use crate::overlays::spec_common::keyed_spec;
use crate::platform::host::{OverlaySpec, ReinitFn};
use crate::platform::reinit::ReinitParams;
use crate::render::canvas::{LineCapStyle, PixCanvas};
use crate::render::primitives::{arc_stroke_outline, line_stroke_outline, text_shaded_auto};
use std::cell::RefCell;
use std::rc::Rc;

/// AffineTransform.getRotateInstance(θ, ax, ay) 的点映射 (屏幕 y 向下, 正 θ = 视觉顺时针):
/// p' = anchor + R(θ)·(p − anchor), R = [[cos, −sin],[sin, cos]]
fn rotate_point(px: f64, py: f64, ax: f64, ay: f64, theta: f64) -> (f64, f64) {
    let (s, c) = theta.sin_cos();
    let (dx, dy) = (px - ax, py - ay);
    (ax + c * dx - s * dy, ay + s * dx + c * dy)
}

/// Java String.format("%3d", v): 右对齐宽 3 空格补左 (超宽原样)
fn fmt_d3(v: i32) -> String {
    let mut s = format!("{}", v);
    while s.chars().count() < 3 {
        s.insert(0, ' ');
    }
    s
}

/// Java String.format("%-4.1f", v): 左对齐宽 4, 1 位小数 HALF_UP。
/// 舍入按 floor(v*10 + 0.5) (Math.round 式); 与 Java Formatter 的精确十进制
/// HALF_UP 在 f64 乘法 ±1 ulp 边界值上可能差 1 个末位 (与 fmt_pct3 同类的已知容差,
/// 真实侧滑显示值不落在该边界)。
fn fmt_f41(v: f64) -> String {
    if v.is_nan() {
        return "NaN ".to_string();
    }
    if v.is_infinite() {
        return "Infinity".to_string();
    }
    let ri = java_round(v * 10.0); // 一位小数 ×10
    let mut s = if ri >= 10 {
        format!("{}.{}", ri / 10, ri % 10)
    } else {
        format!("0.{}", ri)
    };
    while s.chars().count() < 4 {
        s.push(' ');
    }
    s
}

// 阴影双遍文本本地副本 (text_shade) 与旋转 stroke 精确轮廓 (arc/line_stroke_outline)
// 已收敛 render::primitives (重构波13: text_shaded_auto 同式, 轮廓族迁居)。

// ---------------------------------------------------------------------------
// AttitudeIndicatorGauge (MiniHUD 组件)
// ---------------------------------------------------------------------------

/// MiniHUD 地平仪。
///
/// 双模式 (仅翻转符号表; **代码与注释矛盾处以代码为准** — Java 源注释声称
/// body 态 signSlip=+1 / earth 态 −1, 且 pitch/slip 的移动方向描述全部反写;
/// 代码实为下表, 本复刻忠实代码):
/// - 随体 body-fixed (默认): signPitch=−1, signSlip=−1, rollSign=+1
/// - 离体 earth-fixed: signPitch=+1, signSlip=+1, rollSign=−1
pub struct AttitudeIndicatorGauge {
    // 风格上下文 (setStyleContext 注入; font 仅参与 size 度量, 存字号)
    compass_diameter: i32,
    compass_radius: i32,
    compass_inner_mark_radius: i32,
    line_width: i32,
    half_line: i32,
    font_size: i32,
    // 状态
    pitch: f64,
    roll_deg: f64,
    aos_x: i32,
    s_attitude: String,
    round_horizon: i32,
    s_sideslip: String,
    round_slip: i32,
    pitch_valid: bool,
    inertial_mode: bool,
    // 脏检查 (W3 契约 — 组装层门控, 非 Java 迁移字段)
    dirty: bool,
}

impl AttitudeIndicatorGauge {
    /// 构造 (s_attitude/s_sideslip 空串, inertial_mode=false, 其余字段 0)
    pub fn new() -> Self {
        AttitudeIndicatorGauge {
            compass_diameter: 0,
            compass_radius: 0,
            compass_inner_mark_radius: 0,
            line_width: 0,
            half_line: 0,
            font_size: 0,
            pitch: 0.0,
            roll_deg: 0.0,
            aos_x: 0,
            s_attitude: String::new(),
            round_horizon: 0,
            s_sideslip: String::new(),
            round_slip: 0,
            pitch_valid: false,
            inertial_mode: false,
            dirty: true,
        }
    }

    /// 组件 id
    pub fn id(&self) -> &'static str {
        "gauge.attitude"
    }

    /// 首选尺寸 = compass_diameter × compass_diameter
    pub fn preferred_size(&self) -> (i32, i32) {
        (self.compass_diameter, self.compass_diameter)
    }

    /// setStyleContext (Font 参数折为 size —— 类内仅消费 getSize()/getFontMetrics
    /// 度量, draw 时实际字体经参数传入)。
    /// font_size (供 on_data_update 的 aos_x 换算) 与 draw 传入的 font
    /// (gap/『888』模板宽度) 分离 — 组装层须保证两者出自同一字号, 否则 aos_x
    /// 与文本布局口径分裂
    pub fn set_style_context(
        &mut self,
        compass_diameter: i32,
        compass_radius: i32,
        compass_inner_mark_radius: i32,
        line_width: i32,
        half_line: i32,
        font_size: i32,
    ) {
        self.compass_diameter = compass_diameter;
        self.compass_radius = compass_radius;
        self.compass_inner_mark_radius = compass_inner_mark_radius;
        self.line_width = line_width;
        self.half_line = half_line;
        self.font_size = font_size;
        self.dirty = true;
    }

    /// 随体/离体模式切换
    pub fn set_inertial_mode(&mut self, inertial: bool) {
        if self.inertial_mode != inertial {
            self.inertial_mode = inertial;
            self.dirty = true;
        }
    }

    /// legacy 直调通道 (不触 s_sideslip/round_slip)
    pub fn update(
        &mut self,
        pitch: f64,
        roll_deg: f64,
        aos_x: i32,
        s_attitude: &str,
        round_horizon: i32,
    ) -> bool {
        let changed = self.pitch != pitch
            || self.roll_deg != roll_deg
            || self.aos_x != aos_x
            || self.s_attitude != s_attitude
            || self.round_horizon != round_horizon;
        self.pitch = pitch;
        self.roll_deg = roll_deg;
        self.aos_x = aos_x;
        self.s_attitude.clear();
        self.s_attitude.push_str(s_attitude);
        self.round_horizon = round_horizon;
        self.dirty |= changed;
        changed
    }

    /// 数据更新: pitch/roll/slip 注入 + aos_x 换算 + 双值文本格式化。
    /// 返回是否变化 (脏检查)。
    pub fn on_data_update(&mut self, data: &vm_core::derived::hud_data::HUDData) -> bool {
        let slide_limit = 4 * self.font_size;
        // font_size=0 → 乘积 0 → aos_x=0, 无需分支
        // Java (int)(-slip * slideLimit / 30.0f) — double 链, (int) 窄化。
        // JLS 5.1.3: double→int 是饱和 (NaN→0, 超界→±MAX), 与 Rust as i32 语义一致
        //, 无需双转
        let aos_x = (-data.slip * slide_limit as f64 / 30.0) as i32;

        // Attitude 文本 — 仅 pitch_valid 时显示
        let (round_horizon, s_attitude) = if data.pitch_valid {
            // Java (int) Math.round(double) — long→int 强转是位截断,
            // Rust as i32 饱和 — 双转 (as u32) as i32 复刻取低 32 位
            let rh = (java_round(data.pitch) as u32) as i32;
            (rh, fmt_d3(rh.wrapping_abs())) // Math.abs(MIN_VALUE) 回绕保号 (§2.2)
        } else {
            (0, String::new())
        };

        // Sideslip 文本 — 恒显示, 1 位小数
        let slip_value = java_round(data.slip * 10.0) as f64 / 10.0;
        let round_slip = if slip_value >= 0.0 { 1 } else { -1 }; // 颜色判据, 保留符号
        let s_sideslip = fmt_f41(slip_value.abs());

        let changed = self.pitch != data.pitch
            || self.roll_deg != data.roll
            || self.aos_x != aos_x
            || self.s_attitude != s_attitude
            || self.round_horizon != round_horizon
            || self.s_sideslip != s_sideslip
            || self.round_slip != round_slip
            || self.pitch_valid != data.pitch_valid;
        self.pitch = data.pitch;
        self.roll_deg = data.roll;
        self.aos_x = aos_x;
        self.round_horizon = round_horizon;
        self.s_attitude = s_attitude;
        self.s_sideslip = s_sideslip;
        self.round_slip = round_slip;
        self.pitch_valid = data.pitch_valid;
        self.dirty |= changed;
        changed
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 目标点 (天地基准符号中心): (x,y) 组件左上 → center → pitch/侧滑偏移出
    /// target。双模式符号表 = 上文以代码为准的表。
    pub fn target_point(&self, x: i32, y: i32) -> (i32, i32) {
        let radius = self.compass_diameter / 2;
        let center_x = x + radius;
        let center_y = y + radius;
        let (sign_pitch, sign_slip): (i32, i32) = if self.inertial_mode {
            (1, 1) // earth: signPitch=+1, signSlip=+1
        } else {
            (-1, -1) // body: signPitch=−1, signSlip=−1
        };
        // Java signSlip * aosX * 3 / 2 — int 链 ((sign·aos)·3)/2 向零截断,
        // 乘法回绕 (aosX 病态饱和到 ±2^31 时 ×3 溢出, Java 静默回绕)
        let target_x = center_x + sign_slip.wrapping_mul(self.aos_x).wrapping_mul(3) / 2;
        // Java signPitch * (int)(pitch / 2) — double→int 窄化为饱和 (同上, 非 §2.2 截断)
        let target_y = center_y + sign_pitch * (self.pitch / 2.0) as i32;
        (target_x, target_y)
    }

    /// 滚转角 (度, 含模式符号): rollSign * toRadians(rollDeg)
    fn roll_theta(&self) -> f64 {
        let roll_sign = if self.inertial_mode { -1.0 } else { 1.0 };
        roll_sign * self.roll_deg.to_radians()
    }

    /// 绘制。font=None 跳过文本 (font==null 守卫)。
    pub fn draw(
        &mut self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        font: Option<&LoadedFont>,
        aa: bool,
    ) {
        let radius = self.compass_diameter / 2;
        let center_x = x + radius;
        let center_y = y + radius;
        let theta = self.roll_theta();
        let (target_x, target_y) = self.target_point(x, y);
        let lw = self.line_width as f32;

        // 1. 牵引线 (地面/牵引基准线): 粗 shade → 细 label,
        //    BasicStroke(lw+2 / lw, CAP_ROUND, JOIN_ROUND)
        cv.draw_line(
            center_x,
            center_y,
            target_x,
            target_y,
            lw + 2.0,
            colors().shade_shape,
            aa,
        );
        cv.draw_line(
            center_x,
            center_y,
            target_x,
            target_y,
            lw,
            colors().label,
            aa,
        );

        // 2. 旋转 marks: rotate(θ, target) 后 下半圆弧 + 3 刻度,
        //    粗 shade → 细 colorNum。端点/圆心/角度按同一旋转变换预计算。
        self.draw_marks(cv, target_x, target_y, theta, aa);

        // 3. 文本 (已恢复原 transform — 不随滚转旋转)
        if let Some(font) = font {
            let gap = font.size / 4;

            // Pitch 角 — 右侧
            let pitch_color = if self.round_horizon >= 0 {
                colors().num
            } else {
                colors().unit
            };
            text_shaded_auto(
                cv,
                font,
                target_x + gap,
                target_y - 1,
                &self.s_attitude,
                pitch_color,
                aa,
            );

            // Sideslip 角 — 左侧, "888" 模板宽锁定左缘
            if !self.s_sideslip.is_empty() {
                let template_width = font.measure("888");
                let slip_color = if self.round_slip >= 0 {
                    colors().num
                } else {
                    colors().unit
                };
                text_shaded_auto(
                    cv,
                    font,
                    target_x - gap - template_width,
                    target_y - 1,
                    &self.s_sideslip,
                    slip_color,
                    aa,
                );
            }
        }
        self.dirty = false;
    }

    /// 旋转 marks。粗 (lw+2) shade → 细 (lw) num, 双遍 CAP_ROUND。
    /// 中心参数是 target (drawMarks(g2d, targetX, targetY, ...) 语义)。
    fn draw_marks(&self, cv: &mut PixCanvas, target_x: i32, target_y: i32, theta: f64, aa: bool) {
        let hbs = self.half_line + 1;
        let cd = self.compass_diameter;
        let cr = self.compass_radius;
        let inner = self.compass_inner_mark_radius;
        let lw = self.line_width as f32;

        // 弧: drawArc(cx−cr+hbs, cy−cr+hbs, cd, cd, −180, 180)
        // → 盒中心 (cx−cr+hbs+cd/2, ...), 半径 cd/2, 下半圆 (render2d 基线:
        // drawArc(−180,180) 走 9点→6点→3点)。旋转下: 圆心绕 target 旋转、
        // 角度区间平移 −θ (u(φ)→u(φ−θ), 见模块头)。
        let box_x = (target_x - cr + hbs) as f64;
        let box_y = (target_y - cr + hbs) as f64;
        let arc_cx = box_x + cd as f64 / 2.0;
        let arc_cy = box_y + cd as f64 / 2.0;
        let arc_r = cd as f64 / 2.0;
        let (rc_x, rc_y) = rotate_point(arc_cx, arc_cy, target_x as f64, target_y as f64, theta);
        let theta_deg = theta.to_degrees();
        let a1 = -180.0 - theta_deg;

        // 3 刻度线端点 (int 坐标; cr/2 为 int 除)
        let ticks: [((i32, i32), (i32, i32)); 3] = [
            (
                (target_x + hbs, target_y - cr / 2 + hbs),
                (target_x + hbs, target_y - inner + hbs),
            ), // 顶部竖刻度
            (
                (target_x + cr + hbs, target_y + hbs),
                (target_x + inner + hbs, target_y + hbs),
            ), // 右横刻度
            (
                (target_x - cr + hbs, target_y + hbs),
                (target_x - inner + hbs, target_y + hbs),
            ), // 左横刻度
        ];

        for &(width, color) in &[(lw + 2.0, colors().shade_shape), (lw, colors().num)] {
            // 粗遍 / 细遍; 每遍内先弧后线 (drawMarks 序)
            // arc_r==0 (compassDiameter=0) 时 Java BasicStroke(CAP_ROUND) 对零尺寸弧
            // 仍画直径 lineWidth 的圆帽点 (弧两端点重合), Rust 此处整体跳过 — 仅退化布局
            // (preferredSize 0×0 组件不可见) 可达, 不复刻
            if arc_r > 0.0 {
                let outline = arc_stroke_outline(
                    rc_x as f32,
                    rc_y as f32,
                    arc_r as f32,
                    a1 as f32,
                    180.0,
                    width,
                );
                cv.fill_path(&outline, color, aa);
            }
            for &((x0, y0), (x1, y1)) in &ticks {
                // 端点连续旋转 (Java setTransform 语义), stadium 精确轮廓填充
                let (rx0, ry0) = rotate_point(
                    x0 as f64,
                    y0 as f64,
                    target_x as f64,
                    target_y as f64,
                    theta,
                );
                let (rx1, ry1) = rotate_point(
                    x1 as f64,
                    y1 as f64,
                    target_x as f64,
                    target_y as f64,
                    theta,
                );
                let outline = line_stroke_outline(rx0, ry0, rx1, ry1, width as f64);
                cv.fill_path(&outline, color, aa);
            }
        }
    }
}

impl Default for AttitudeIndicatorGauge {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// AttitudeOverlay (独立地平仪窗组件)
// ---------------------------------------------------------------------------

/// pitch 刻度对数 (tickLine), 2 对 = 4 条
pub const TICK_LINE: i32 = 2;
/// 俯仰/攻角满量程 ±30° (MaxAoA)
pub const MAX_AOA: i32 = 30;
/// 侧滑满量程 ±15° (MaxAoS)
pub const MAX_AOS: i32 = 15;
/// 构造期缺省尺寸 (reinit 前用)
pub const BASE_WIDTH: i32 = 100;
pub const BASE_HEIGHT: i32 = 200;
/// locater 常量: 中心参考弧径 / 侧滑球十字臂长
const CENTER_ROUND: i32 = 12;
const LOCATOR_SIZE: i32 = 6;
/// 攻角极限线关闭时的哨兵 y (−10, 画在窗口外被裁剪 → 不可见)
const AOA_LIMIT_OFF: i64 = -10;

/// 独立地平仪窗。C 类复刻只保留绘制语义核心: drawTick 的数据换算 + locater 的
/// 图层序; 窗口/拖动/边框阴影/渲染节流属组装层, 不在本组件。
/// 画布 = [0, x_width)×[0, x_height) 的 Pixmap, 裁剪由光栅化界天然给出。
/// Java 侧两级 paintComponent 都调 locater — 半透明层 (α220/42/100) 可能
/// 双重合成 (地面有效 α≈246 而非单遍 220), 除非 WebLaF opaque 默认压制其中一级。
/// 本复刻锚定**单遍** locater 语义 (像素对拍基准即单遍)。
pub struct AttitudeOverlay {
    /// 绘制区尺寸 (Java xWidth/xHeight, reinitConfig 已含 DPI 缩放的终值)
    pub x_width: i32,
    pub x_height: i32,
    /// showDirection (attitudeIndicatorDisplayDirection, 默认 false)
    pub show_direction: bool,
    /// showAoALimits (attitudeIndicatorDisplayAoALimits, 默认 true)
    pub show_aoa_limits: bool,
    // drawTick 计算缓存 (保留 long 语义)
    /// 侧滑球十字 x = round((−aos+15)·w/30)
    pub aos_x: i64,
    /// 侧滑球十字 y = round((aoa+30)·h/60)
    pub aoa_y: i64,
    /// 地平线平移量 = round((−pitch+30)·h/60)
    pub pitch_y: i64,
    /// 航向指针分量 (showDirection 时非零)
    pub compass_x: i64,
    pub compass_y: i64,
    /// 攻角极限线 y (哨兵 −10 = 不显示)
    pub aoa_limit_u: i64,
    pub aoa_limit_d: i64,
    /// 旋转+取整后的点集 (Java pT: pT[0..4] 地面多边形角点, pT[4..12] 刻度线端点对)
    pub p_t: [(i32, i32); (4 + TICK_LINE * 4) as usize],
    dirty: bool,
}

impl AttitudeOverlay {
    /// 字段初始化 (x_width=BASE_WIDTH, x_height=BASE_HEIGHT, show_direction=false,
    /// show_aoa_limits=true — 与 reinit 两分支的默认一致)
    pub fn new() -> Self {
        AttitudeOverlay {
            x_width: BASE_WIDTH,
            x_height: BASE_HEIGHT,
            show_direction: false,
            show_aoa_limits: true,
            aos_x: 0,
            aoa_y: 0,
            pitch_y: 0,
            compass_x: 0,
            compass_y: 0,
            aoa_limit_u: AOA_LIMIT_OFF,
            aoa_limit_d: AOA_LIMIT_OFF,
            p_t: [(0, 0); (4 + TICK_LINE * 4) as usize],
            dirty: true,
        }
    }

    /// reinit 的绘制相关子集: 尺寸为调用方完成 DPI 缩放后的终值
    /// (round(base·dpiScale))。
    pub fn reinit(
        &mut self,
        x_width: i32,
        x_height: i32,
        show_direction: bool,
        show_aoa_limits: bool,
    ) {
        self.x_width = x_width;
        self.x_height = x_height;
        self.show_direction = show_direction;
        self.show_aoa_limits = show_aoa_limits;
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 数据面回构造器初值 (D8: 单实例跨 preview/live 重建存活的补口 — live 会话
    /// 残留的姿态点集在 preview 重开前清除, 否则预览窗地平仪冻结在上次 live 姿态)。
    /// 几何保留 (x_width/x_height/开关 — reinit 闭包负责刷新)。
    pub fn reset_preview(&mut self) {
        self.aos_x = 0;
        self.aoa_y = 0;
        self.pitch_y = 0;
        self.compass_x = 0;
        self.compass_y = 0;
        self.aoa_limit_u = AOA_LIMIT_OFF;
        self.aoa_limit_d = AOA_LIMIT_OFF;
        self.p_t = [(0, 0); (4 + TICK_LINE * 4) as usize];
        self.dirty = true;
    }

    /// drawTick 语义: 遥测换算 + 地面多边形/刻度点集的平移旋转。
    /// `aoa_limits` = FM 的 (NoFlapsWing.AoACritHigh, AoACritLow);
    /// None = 无 FM —— 极限线取哨兵 −10 (画在窗口外)。
    /// show_aoa_limits=false 同走哨兵 (条件 = 有 FM 且开关开)。
    /// 恒置脏恒返回 true = 每次数据到即重绘的语义 (40ms 节流归组装层);
    /// 变化检测非本组件行为, 组装层按需自做。
    #[allow(clippy::too_many_arguments)] // 输入面对齐 drawTick 原型
    pub fn update_telemetry(
        &mut self,
        aoa: f64,
        aos: f64,
        pitch: f64,
        roll: f64,
        compass: f64,
        aoa_limits: Option<(f64, f64)>,
    ) -> bool {
        let w = self.x_width;
        let h = self.x_height;
        self.aoa_y = java_round((aoa + MAX_AOA as f64) * h as f64 / (2 * MAX_AOA) as f64);
        self.aos_x = java_round((-aos + MAX_AOS as f64) * w as f64 / (2 * MAX_AOS) as f64);
        self.pitch_y = java_round((-pitch + MAX_AOA as f64) * h as f64 / (2 * MAX_AOA) as f64);

        // (int) 是 double→int 饱和 (JLS 5.1.3), 故先 as i32 再拓宽, 非 as i64
        if self.show_direction {
            let rads = compass.to_radians();
            self.compass_x = (((w / 4) as f64 * rads.sin()) as i32) as i64;
            self.compass_y = (((w / 4) as f64 * rads.cos()) as i32) as i64;
        }

        match aoa_limits {
            Some((crit_high, crit_low)) if self.show_aoa_limits => {
                self.aoa_limit_u =
                    java_round((crit_high + MAX_AOA as f64) * h as f64 / (2 * MAX_AOA) as f64);
                self.aoa_limit_d =
                    java_round((crit_low + MAX_AOA as f64) * h as f64 / (2 * MAX_AOA) as f64);
            }
            _ => {
                self.aoa_limit_u = AOA_LIMIT_OFF;
                self.aoa_limit_d = AOA_LIMIT_OFF;
            }
        }

        // 地面多边形角点: ±2 宽 (越界部分被窗口裁剪), 地面厚
        // 180/MaxAoA·h = 6h; int 坐标
        let mut p_s = [(0i32, 0i32); (4 + TICK_LINE * 4) as usize];
        let ground_h = 180 / MAX_AOA * h; // int 除 180/30=6 (整)
        p_s[0] = (-2 * w, 0);
        p_s[1] = (2 * w, 0);
        p_s[2] = (2 * w, ground_h);
        p_s[3] = (-2 * w, ground_h);

        // pitch 刻度: start=−90, dTick=90/(tickLine+1)=30 →
        // ±30°/±60° 刻度对; y = round((角)/(2·MaxAoA)·h)
        let start = -90.0f64;
        let d_tick = (90 / (TICK_LINE + 1)) as f64; // int 除 90/3=30
        for i in 0..TICK_LINE {
            let a = start + d_tick * (i + 1) as f64;
            let y_up = java_round_f64(a / (2 * MAX_AOA) as f64 * h as f64);
            // 对称刻度: −start − dTick·(i+1)
            let y_dn = java_round_f64(-a / (2 * MAX_AOA) as f64 * h as f64);
            p_s[(4 + 4 * i) as usize] = (-w, y_up);
            p_s[(4 + 4 * i + 1) as usize] = (w, y_up);
            p_s[(4 + 4 * i + 2) as usize] = (-w, y_dn);
            p_s[(4 + 4 * i + 3) as usize] = (w, y_dn);
        }

        // 平移: x += w/2 (int 除), y += Pitch (long→int 窄化)
        for p in &mut p_s {
            p.0 += w / 2;
            // Java p.y += Pitch 复合赋值隐式 (int)(y+long) — 位截断,
            // Rust as i32 饱和, 双转 (as u32) as i32 复刻取低 32 位
            p.1 = ((p.1 as i64 + self.pitch_y) as u32) as i32;
        }

        // 旋转: 绕 pC=(w/2, h/2) 旋转 roll 度后取整 floor(x+0.5)。
        // pC 取 int 除 (奇尺寸圆心取整)
        let theta = roll.to_radians();
        for (i, p) in p_s.iter().enumerate() {
            let (rx, ry) = rotate_point(
                p.0 as f64,
                p.1 as f64,
                (w / 2) as f64,
                (h / 2) as f64,
                theta,
            );
            self.p_t[i] = (java_round_f64(rx), java_round_f64(ry));
        }

        self.dirty = true;
        true
    }

    /// locater: 图层序绘制。
    /// 画布必须为 x_width×x_height (裁剪语义), 由调用方保证 — 防呆断言
    /// (更大画布会让 ±2w 地面多边形画出窗口界, 背离窗口裁剪语义)。
    pub fn draw(&mut self, cv: &mut PixCanvas, aa: bool) {
        debug_assert!(
            cv.width() == self.x_width && cv.height() == self.x_height,
            "画布须为 {}×{}, 实为 {}×{}",
            self.x_width,
            self.x_height,
            cv.width(),
            cv.height()
        );
        let w = self.x_width;
        let h = self.x_height;
        // 调用点实参: x=(int)AoS, y=(int)AoA — long→int 位截断
        let x = (self.aos_x as u32) as i32;
        let y = (self.aoa_y as u32) as i32;
        let cr_half = CENTER_ROUND / 2; // 6

        // 1. 地面多边形 (最底层, colorUnit)
        //    PORT(精确定性): Java 侧先声明 transParentWhite=colorUnit, 后读
        //    attitudeIndicatorUseNumColor 覆盖为 colorNum — 但该字段全文件仅
        //    声明+赋值两处, 无任何读取者 (键被读、值写进死字段) → 无可观测
        //    行为, 不复刻 (本处恒 colorUnit 与 Java 可见行为一致)
        let poly: [(f32, f32); 4] = [
            (self.p_t[0].0 as f32, self.p_t[0].1 as f32),
            (self.p_t[1].0 as f32, self.p_t[1].1 as f32),
            (self.p_t[2].0 as f32, self.p_t[2].1 as f32),
            (self.p_t[3].0 as f32, self.p_t[3].1 as f32),
        ];
        cv.fill_path(&poly, colors().unit, aa);

        // 2. 边框 (BasicStroke(1) 裸 = CAP_SQUARE/JOIN_MITER, shade)
        for &(x0, y0, x1, y1) in &[
            (0, 0, 0, h),
            (0, 0, w, 0),
            (0, h - 1, w - 1, h - 1),
            (w - 1, 0, w - 1, h - 1),
        ] {
            cv.draw_line_cap(
                x0,
                y0,
                x1,
                y1,
                1.0,
                colors().shade_shape,
                LineCapStyle::Square,
                aa,
            );
        }

        // 3. pitch 刻度线 (仍 1px shade): 4 条 = 2·tickLine 对
        for i in 0..(2 * TICK_LINE) as usize {
            let (x0, y0) = self.p_t[4 + 2 * i];
            let (x1, y1) = self.p_t[4 + 2 * i + 1];
            cv.draw_line_cap(
                x0,
                y0,
                x1,
                y1,
                1.0,
                colors().shade_shape,
                LineCapStyle::Square,
                aa,
            );
        }

        // 4. 中心参考 (BasicStroke(3), colorNum)
        let mid_y = h / 2 - 1;
        for &(x0, x1) in &[
            (w / 2 - cr_half - w / 8 - 1, w / 2 - cr_half - 1), // 左内段
            (w / 2 + cr_half, w / 2 + cr_half + w / 8 - 1),     // 右内段
            (0, w / 8 - 1),                                     // 左外段
            (w - w / 8 + 1, w),                                 // 右外段
        ] {
            cv.draw_line_cap(
                x0,
                mid_y,
                x1,
                mid_y,
                3.0,
                colors().num,
                LineCapStyle::Square,
                aa,
            );
        }
        // 中心下半圆: drawArc(w/2−7, h/2−7, 12, 12, −180, 180) 的弧心 = 盒角+半径
        // = (w/2−1, h/2−1), r=6 (stroke_arc 收圆心而非盒角)
        cv.stroke_arc(
            w / 2 - 1,
            h / 2 - 1,
            CENTER_ROUND / 2,
            -180.0,
            0.0,
            3.0,
            colors().num,
            LineCapStyle::Square,
            aa,
        );

        // 5. 侧滑球十字 (BasicStroke(2), colorNum 承袭)
        let ls_half = LOCATOR_SIZE / 2; // 3
        cv.draw_line_cap(
            x - ls_half - 1,
            y - 1,
            x + ls_half - 1,
            y - 1,
            2.0,
            colors().num,
            LineCapStyle::Square,
            aa,
        );
        cv.draw_line_cap(
            x - 1,
            y - ls_half - 1,
            x - 1,
            y + ls_half - 1,
            2.0,
            colors().num,
            LineCapStyle::Square,
            aa,
        );

        // 6. 攻角极限线 (colorWarning, 仍 2px); 哨兵 −10 落在窗口外被裁
        // Java (int) AoALimitU/D — long→int 位截断
        let lu = (self.aoa_limit_u as u32) as i32;
        let ld = (self.aoa_limit_d as u32) as i32;
        cv.draw_line_cap(
            0,
            lu,
            w - 1,
            lu,
            2.0,
            colors().warning,
            LineCapStyle::Square,
            aa,
        );
        cv.draw_line_cap(
            0,
            ld,
            w - 1,
            ld,
            2.0,
            colors().warning,
            LineCapStyle::Square,
            aa,
        );

        // 7. 航向指针对: colorNum 正向 + warning 反向
        if self.show_direction {
            let (ccx, ccy) = (w / 2, h / 2);
            // Java (int)(width/2 ± compassX) — long 加法后 →int 位截断
            let px = ((ccx as i64 + self.compass_x) as u32) as i32;
            let py = ((ccy as i64 + self.compass_y) as u32) as i32;
            let mx = ((ccx as i64 - self.compass_x) as u32) as i32;
            let my = ((ccy as i64 - self.compass_y) as u32) as i32;
            cv.draw_line_cap(
                ccx,
                ccy,
                px,
                py,
                2.0,
                colors().num,
                LineCapStyle::Square,
                aa,
            );
            cv.draw_line_cap(
                ccx,
                ccy,
                mx,
                my,
                2.0,
                colors().warning,
                LineCapStyle::Square,
                aa,
            );
        }
        self.dirty = false;
    }
}

impl Default for AttitudeOverlay {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// OverlayHost 挂载 (注册键 enableAttitudeIndicator)
// ---------------------------------------------------------------------------

/// 地平仪共享句柄 (minihud_overlay_spec 先例: render 闭包与喂入方共享 state)
pub type AttitudeOverlayHandle = Rc<RefCell<AttitudeOverlay>>;

/// 参数仓 → reinitConfig 绘制面 (工厂初建与 reinit 闭包共用一份读取):
/// base 宽高 = attitudeIndicatorWidth/Height (cfg 缺省 150/300), 此处完成 DPI
/// 缩放 (round(base·dpiScale), §2.3 floor(x+0.5));
/// show_direction/show_aoa_limits = attitudeIndicatorDisplayDirection (false) /
/// ...DisplayAoALimits (true)
fn attitude_geom(p: &ReinitParams) -> (i32, i32, bool, bool) {
    let dpi = p.dpi_scale;
    (
        (p.attitude.width as f64 * dpi + 0.5).floor() as i32,
        (p.attitude.height as f64 * dpi + 0.5).floor() as i32,
        p.attitude.show_direction,
        p.attitude.show_aoa_limits,
    )
}

/// 地平仪 OverlaySpec + live 句柄。参数为 reinitConfig 的配置面,
/// 经 [`ReinitParams`] 仓读取 (换算见 [`attitude_geom`])。
/// PORT(边框不承载): Java totalWidth = xWidth+4+sw·2 的 sw 边距是 WebLaF 窗口装饰
/// (enableAttitudeIndicatorEdge, 默认 false), host 无边框层 — spec 尺寸 = 内容区
/// x_width×x_height (draw 的画布断言钉内容尺寸, 裁剪语义)。
/// 初始态 = 未飞形态 (AoA/AoS/Pitch 0, drawTick 未跑), 预览/游戏共用; live 由喂入方
/// update_telemetry 推进 (40ms 节流归组装层, Java onFlightData freqMili)。
/// PORT(WYSIWYG): reinit 闭包 = reinit_config 的绘制相关子集 (宽高/开关), 喂入
/// 节流 freqMili 由组装层随参数仓同步 (app_shell ReinitOverlays 处理点)
pub fn attitude_overlay_spec(
    params: &Rc<RefCell<ReinitParams>>,
) -> Result<(AttitudeOverlayHandle, OverlaySpec), String> {
    let (x_width, x_height, show_direction, show_aoa_limits) = attitude_geom(&params.borrow());
    let mut overlay = AttitudeOverlay::new();
    overlay.reinit(x_width, x_height, show_direction, show_aoa_limits);
    let handle: AttitudeOverlayHandle = Rc::new(RefCell::new(overlay));
    let render_handle = Rc::clone(&handle);
    // reinit 闭包: DPI 缩放后的新宽高 + 开关族 → state reinit + 新尺寸 (setBounds)
    let reinit_handle = Rc::clone(&handle);
    let reinit_params = Rc::clone(params);
    let reinit: ReinitFn = Box::new(move || {
        let (xw, xh, dir, aoa) = attitude_geom(&reinit_params.borrow());
        reinit_handle.borrow_mut().reinit(xw, xh, dir, aoa);
        Some((xw, xh))
    });
    Ok((
        handle,
        // keyed_spec 键 = configKey (注册开关名)
        keyed_spec(
            "enableAttitudeIndicator",
            x_width,
            x_height,
            Box::new(move |cv: &mut PixCanvas| {
                // aa = 运行时全局仓 (cfg AAEnable 可关 — Java 默认 true 仅是
                // 声明默认, 审查轮 1-A 曾误当生产不变式钉死 true)
                render_handle.borrow_mut().draw(cv, aa());
            }),
            Some(reinit),
        ),
    ))
}

#[cfg(test)]
mod tests;
