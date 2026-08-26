//! gauge_attitude: 地平仪家族 C 类语义复刻 (最复杂件: 旋转/侧滑/pitch 刻度/双模式)
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | AttitudeIndicatorGauge | ui/component/AttitudeIndicatorGauge.java | MiniHUD 地平仪: 牵引线 + 旋转 marks (下半圆弧 + 3 刻度) + pitch/侧滑双值文本; 随体/离体双模式仅翻转符号表 |
//! | AttitudeOverlay | ui/overlay/AttitudeOverlay.java | 独立地平仪窗: 橙色地面多边形 (±2 宽被窗口裁剪) + 4 条 pitch 刻度 + 中线/下半圆 + 侧滑球十字 + 攻角极限线 + 航向指针对 |
//!
//! Java Graphics2D 变换的复刻策略 (D7: 矢量基元走 tiny-skia):
//! - **旋转 marks** (IndicatorGauge): Java setTransform(rotate(θ, target)) 后连续光栅化。
//!   PixCanvas 整数基元吃不下连续旋转坐标, 故弧与刻度线均按 stroke 区域的精确几何
//!   轮廓折线单次 fill (arc_stroke_outline / line_stroke_outline, Minkowski 和):
//!   弧 = 外弧→端帽→内弧→端帽; 线段 = 矩形体+双半圆帽 (stadium)。
//!   单次合成避免半透明色 (alpha<255) 分段叠加加深伪影, 且保住 Java 连续变换的
//!   亚像素定位; 每遍内绘制序 (弧→3 线) 与 Java drawMarks 一致。
//! - **旋转多边形** (Overlay): Java 逐点 AffineTransform.transform 后 Point.setLocation
//!   取整 (floor(x+0.5)), 再 fillPolygon —— 端点先取整再连直边, 非 "连续旋转后填充",
//!   Rust 侧同序复刻 (java_round_i32), 像素级关键差异。
//! - **窗口裁剪** (Overlay): Swing 面板自动裁剪到 [0,w)×[0,h); Rust 侧 Pixmap 光栅化
//!   天然裁剪到画布界, 画布取 w×h 即得等效 clip。
//!
//! 颜色 = Application.java:106-111 静态色直通 RGBA (与 gauges_bars 同源)。

use crate::font::LoadedFont;
use crate::gauges_bars::{COLOR_LABEL, COLOR_NUM, COLOR_SHADE_SHAPE, COLOR_WARNING};
use crate::host::OverlaySpec;
use crate::render2d::{LineCapStyle, PixCanvas};
use std::cell::RefCell;
use std::rc::Rc;

/// Application.java:109 colorUnit = (166,166,166,220) — IndicatorGauge 文本负色 /
/// Overlay 地面多边形色 (gauges_bars 未导出, 本模块地平仪专用)
pub const COLOR_UNIT: [u8; 4] = [166, 166, 166, 220];

/// Java Math.round(double)→long: floor(x+0.5) (§2.3, Rust round 是半偶)
fn java_round_i64(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

/// Java Point.setLocation(double,double) 取整: (int)floor(x+0.5)
/// (AttitudeOverlay.rotatePointMatrix → AffineTransform.transform(Point[]) 的落点语义)
fn java_round_i32(x: f64) -> i32 {
    (x + 0.5).floor() as i32
}

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
        return "NaN ".to_string(); // Java Formatter: "NaN" 左对齐宽 4
    }
    if v.is_infinite() {
        return "Infinity".to_string(); // Java Formatter 同串 (超宽原样)
    }
    let ri = java_round_i64(v * 10.0); // 一位小数 ×10
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

/// 阴影双遍文本 (UIBaseElements.__drawStringShade drawFontShape=false 分支,
/// Application.java:143 默认 false): 影 (x+1,y+1) shade → 本色 (x,y)。
/// (gauges_bars::text_shaded 私有, 本模块同式)
fn text_shade(
    cv: &mut PixCanvas,
    font: &LoadedFont,
    x: i32,
    y: i32,
    s: &str,
    c: [u8; 4],
    aa: bool,
) {
    cv.draw_text(font, x + 1, y + 1, s, COLOR_SHADE_SHAPE, aa);
    cv.draw_text(font, x, y, s, c, aa);
}

/// 圆弧 stroke 的精确几何区域轮廓折线: 圆弧中心线 (cx,cy,r, a1→a1+sweep) ⊖ 半径
/// half=w/2 圆盘 = 外弧 (r+half) → 末端圆帽 (CAP_ROUND) → 内弧 (r−half) → 始端圆帽,
/// 单闭合折线。fill 一次完成 → 半透明色单次 SrcOver 合成, 无分段叠加伪影
/// (Java drawArc + BasicStroke(CAP_ROUND) 的 stroke 区域即此 Minkowski 和)。
/// 角度约定同 render2d::stroke_arc: point(φ) = (cx + r·cosφ, cy − r·sinφ),
/// 正 sweep = 视觉逆时针; 负 sweep 归一化为正 (区域与参数方向无关)。
fn arc_stroke_outline(cx: f32, cy: f32, r: f32, a1: f32, sweep: f32, w: f32) -> Vec<(f32, f32)> {
    let (a1, sweep) = if sweep < 0.0 {
        (a1 + sweep, -sweep)
    } else {
        (a1, sweep)
    };
    let a2 = a1 + sweep;
    let half = w / 2.0;
    let r_out = r + half;
    // PORT: r−half<0 (线宽≥2r) 时内弧塌到圆心, 退化扇形近似 Java stroke 的满盘;
    // 真实布局 r≥5、w≤6 不可达, 备查
    let r_in = (r - half).max(0.0);
    const STEP: f32 = 4.0; // 折线步进: 弦矢 ≈ r·(1−cos2°) ≈ 0.0006r, 亚像素
    let n = ((sweep / STEP).ceil() as i32).max(1) as usize;
    let pt = |radius: f32, ang: f32| -> (f32, f32) {
        let t = ang.to_radians();
        (cx + radius * t.cos(), cy - radius * t.sin())
    };
    // CAP_ROUND 端帽绕【弧端点】(非弧心) 的半圆: 帽点 = 弧端点 + half·u(ψ),
    // ψ 沿扇区外侧从径向外 u(a) 扫到径向内 u(a+180) (对下半圆扇区两帽均经上方)
    let cap_pt = |ex: f32, ey: f32, psi: f32| -> (f32, f32) {
        let t = psi.to_radians();
        (ex + half * t.cos(), ey - half * t.sin())
    };
    let mut pts = Vec::with_capacity(n * 4 + 4);
    for i in 0..=n {
        pts.push(pt(r_out, a1 + sweep * i as f32 / n as f32)); // 外弧 a1→a2
    }
    let (ex2, ey2) = pt(r, a2); // 末端的弧端点 (cx+r·cos a2, cy−r·sin a2)
    for i in 1..=n {
        pts.push(cap_pt(ex2, ey2, a2 + 180.0 * i as f32 / n as f32)); // 末端帽
    }
    for i in 1..=n {
        pts.push(pt(r_in, a2 - sweep * i as f32 / n as f32)); // 内弧 a2→a1
    }
    let (ex1, ey1) = pt(r, a1); // 始端的弧端点
    for i in 1..=n {
        pts.push(cap_pt(ex1, ey1, a1 + 180.0 + 180.0 * i as f32 / n as f32)); // 始端帽
    }
    pts
}

/// 线段 stroke 的精确几何区域轮廓折线 (stadium): 中心线段 ⊕ 半径 half=w/2 圆盘
/// = 矩形体 + 两端 CAP_ROUND 半圆帽, 单闭合折线一次 fill。
/// 端点保持 f64 (Java setTransform 旋转下线端是连续坐标, 走整数基元会丢亚像素
/// 定位与 AA 柔边, 故旋转刻度线不走 draw_line_cap 而用本精确轮廓)。
/// 零长度线段退化为圆点 (Java BasicStroke CAP_ROUND 零长线画点, 行为一致)。
fn line_stroke_outline(x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Vec<(f32, f32)> {
    let half = w / 2.0;
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = dx.hypot(dy);
    const N: usize = 16; // 半圆 16 段: 矢高 ≈ r·(1−cos5.6°) ≈ 0.005r, 亚像素
    let mut pts = Vec::with_capacity(N * 2 + 2);
    if len == 0.0 {
        // 圆点: 完整圆周折线
        for i in 0..N {
            let a = std::f64::consts::TAU * i as f64 / N as f64;
            pts.push(((x0 + half * a.cos()) as f32, (y0 + half * a.sin()) as f32));
        }
        return pts;
    }
    let (tx, ty) = (dx / len, dy / len); // 切向
    let (nx, ny) = (-ty * half, tx * half); // 法向 × half
    // 上侧边: P0+n → P1+n
    pts.push(((x0 + nx) as f32, (y0 + ny) as f32));
    pts.push(((x1 + nx) as f32, (y1 + ny) as f32));
    // P1 端帽: +n 绕过 +t 到 −n (φ: 0→π)
    for i in 1..=N {
        // sin_cos 返回 (sin, cos) — 帽点 = P1 + n·cosφ + t·half·sinφ
        let (s, c) = (std::f64::consts::PI * i as f64 / N as f64).sin_cos();
        pts.push((
            (x1 + nx * c + tx * half * s) as f32,
            (y1 + ny * c + ty * half * s) as f32,
        ));
    }
    // 下侧边: P1−n → P0−n
    pts.push(((x1 - nx) as f32, (y1 - ny) as f32));
    pts.push(((x0 - nx) as f32, (y0 - ny) as f32));
    // P0 端帽: −n 绕过 −t 到 +n (φ: 0→π, 方向取 −t)
    for i in 1..=N {
        let (s, c) = (std::f64::consts::PI * i as f64 / N as f64).sin_cos();
        pts.push((
            (x0 - nx * c - tx * half * s) as f32,
            (y0 - ny * c - ty * half * s) as f32,
        ));
    }
    pts
}

// ---------------------------------------------------------------------------
// AttitudeIndicatorGauge (MiniHUD 组件)
// ---------------------------------------------------------------------------

/// MiniHUD 地平仪 (AttitudeIndicatorGauge.java:16)。
///
/// 双模式 (Java:41-56, 仅翻转符号表; **代码与注释矛盾处以代码为准** —
/// Java:100-108 注释声称 body 态 signSlip=+1 / earth 态 −1, 且 pitch/slip 的
/// 移动方向描述全部反写; 代码 L112-122 实为下表, 本复刻忠实代码):
/// - 随体 body-fixed (默认): signPitch=−1, signSlip=−1, rollSign=+1
/// - 离体 earth-fixed: signPitch=+1, signSlip=+1, rollSign=−1
pub struct AttitudeIndicatorGauge {
    // 风格上下文 (Java:23-29 setStyleContext 注入; font 仅参与 size 度量, 存字号)
    compass_diameter: i32,
    compass_radius: i32,
    compass_inner_mark_radius: i32,
    line_width: i32,
    half_line: i32,
    font_size: i32,
    // 状态 (Java:31-42)
    pitch: f64,
    roll_deg: f64,
    aos_x: i32,
    s_attitude: String,
    round_horizon: i32,
    s_sideslip: String,
    round_slip: i32,
    pitch_valid: bool,
    inertial_mode: bool,
    // 脏检查 (W3 契约, Java 无此字段 — C 类组装层门控)
    dirty: bool,
}

impl AttitudeIndicatorGauge {
    /// Java:44-48 构造 (sAttitude/sSideslip 空串, inertialMode=false, 其余字段 0)
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

    /// Java:59-61 getId
    pub fn id(&self) -> &'static str {
        "gauge.attitude"
    }

    /// Java:63-66 getPreferredSize = compassDiameter × compassDiameter
    pub fn preferred_size(&self) -> (i32, i32) {
        (self.compass_diameter, self.compass_diameter)
    }

    /// Java:68-76 setStyleContext (Font 参数在 Rust 侧折为其 size —— 该对象在类内
    /// 仅消费 getSize() 与 getFontMetrics 度量, draw 时实际字体经参数传入)。
    /// PORT: Java 单一 font 字段同源; Rust 侧 font_size (供 on_data_update 的 aosX
    /// 换算) 与 draw 传入的 font (gap/『888』模板宽度) 分离 — 组装层须保证两者
    /// 出自同一字号, 否则 aosX 与文本布局口径分裂
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

    /// Java:54-56 setInertialMode
    pub fn set_inertial_mode(&mut self, inertial: bool) {
        if self.inertial_mode != inertial {
            self.inertial_mode = inertial;
            self.dirty = true;
        }
    }

    /// Java:78-84 update (legacy 直调通道, 不触 sSideslip/roundSlip)
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

    /// Java:192-224 onDataUpdate。pitch/roll/slip 注入 + aosX 换算 + 双值文本格式化。
    /// 返回是否变化 (脏检查)。
    pub fn on_data_update(&mut self, data: &vm_core::hud_data::HUDData) -> bool {
        let slide_limit = 4 * self.font_size; // Java:204 (font==null 时 Java 跳过;
        // font_size=0 → 乘积 0 → aos_x=0, 与 Java else 分支数值一致, 无需分支)
        // PORT: Java:205 (int)(-slip * slideLimit / 30.0f) — double 链, (int) 窄化。
        // JLS 5.1.3: double→int 是饱和 (NaN→0, 超界→±MAX), 与 Rust as i32 语义一致
        // (§2.2 的位截断规则只适用整数间窄化), 无需双转
        let aos_x = (-data.slip * slide_limit as f64 / 30.0) as i32;

        // Attitude 文本 — 仅 pitchValid 时显示 (Java:210-218)
        let (round_horizon, s_attitude) = if data.pitch_valid {
            // PORT: Java:213 (int) Math.round(double) — long→int 强转是位截断 (§2.2),
            // Rust as i32 饱和 — 双转 (as u32) as i32 复刻取低 32 位
            let rh = (java_round_i64(data.pitch) as u32) as i32;
            (rh, fmt_d3(rh.wrapping_abs())) // Math.abs(MIN_VALUE) 回绕保号 (§2.2)
        } else {
            (0, String::new())
        };

        // Sideslip 文本 — 恒显示, 1 位小数 (Java:220-223)
        let slip_value = java_round_i64(data.slip * 10.0) as f64 / 10.0;
        let round_slip = if slip_value >= 0.0 { 1 } else { -1 }; // 颜色判据, 保留符号 (Java:222)
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

    /// Java:91-125 目标点 (天地基准符号中心): (x,y) 组件左上 → center →
    /// pitch/侧滑偏移出 target。双模式符号表 = Java:112-122 代码值。
    pub fn target_point(&self, x: i32, y: i32) -> (i32, i32) {
        let radius = self.compass_diameter / 2; // Java:94 int 除
        let center_x = x + radius;
        let center_y = y + radius;
        let (sign_pitch, sign_slip): (i32, i32) = if self.inertial_mode {
            (1, 1) // earth: signPitch=+1, signSlip=+1 (Java:114-115 代码值)
        } else {
            (-1, -1) // body: signPitch=−1, signSlip=−1 (Java:119-120 代码值)
        };
        // PORT: Java:124 signSlip * aosX * 3 / 2 — int 链 ((sign·aos)·3)/2 向零截断,
        // 乘法回绕 (aosX 病态饱和到 ±2^31 时 ×3 溢出, Java 静默回绕)
        let target_x = center_x + sign_slip.wrapping_mul(self.aos_x).wrapping_mul(3) / 2;
        // PORT: Java:125 signPitch * (int)(pitch / 2) — double→int 窄化为饱和 (同上, 非 §2.2 截断)
        let target_y = center_y + sign_pitch * (self.pitch / 2.0) as i32;
        (target_x, target_y)
    }

    /// 滚转角 (度, 含模式符号): Java:98/116/121/138 rollSign * toRadians(rollDeg)
    fn roll_theta(&self) -> f64 {
        let roll_sign = if self.inertial_mode { -1.0 } else { 1.0 };
        roll_sign * self.roll_deg.to_radians()
    }

    /// Java:87-168 draw。font=None 跳过文本 (Java:154 font==null 守卫)。
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

        // 1. 牵引线 (地面/牵引基准线, Java:127-134): 粗 shade → 细 colorLabel
        //    BasicStroke(lw+2 / lw, CAP_ROUND, JOIN_ROUND)
        cv.draw_line(
            center_x, center_y, target_x, target_y,
            lw + 2.0, COLOR_SHADE_SHAPE, aa,
        );
        cv.draw_line(center_x, center_y, target_x, target_y, lw, COLOR_LABEL, aa);

        // 2. 旋转 marks (Java:136-151): rotate(θ, target) 后 下半圆弧 + 3 刻度,
        //    粗 shade → 细 colorNum。端点/圆心/角度按同一旋转变换预计算。
        self.draw_marks(cv, target_x, target_y, theta, aa);

        // 3. 文本 (Java:153-167, 已恢复原 transform — 不随滚转旋转)
        if let Some(font) = font {
            let gap = font.size / 4; // Java:155 gap = font.getSize()/4 (int 除)

            // Pitch 角 — 右侧 (Java:158-159)
            let pitch_color = if self.round_horizon >= 0 {
                COLOR_NUM
            } else {
                COLOR_UNIT
            };
            text_shade(cv, font, target_x + gap, target_y - 1, &self.s_attitude, pitch_color, aa);

            // Sideslip 角 — 左侧, "888" 模板宽锁定左缘 (Java:161-166)
            if !self.s_sideslip.is_empty() {
                let template_width = font.measure("888");
                let slip_color = if self.round_slip >= 0 {
                    COLOR_NUM
                } else {
                    COLOR_UNIT
                };
                text_shade(
                    cv, font, target_x - gap - template_width, target_y - 1,
                    &self.s_sideslip, slip_color, aa,
                );
            }
        }
        self.dirty = false;
    }

    /// Java:141-151+170-181: 旋转 marks。粗 (lw+2) shade → 细 (lw) colorNum,
    /// 双遍 CAP_ROUND。中心参数是 target (Java:144/149 drawMarks(g2d, targetX, targetY, ...))。
    fn draw_marks(&self, cv: &mut PixCanvas, target_x: i32, target_y: i32, theta: f64, aa: bool) {
        let hbs = self.half_line + 1; // Java:172 半线宽后仍差 1px 的经验修正
        let cd = self.compass_diameter;
        let cr = self.compass_radius;
        let inner = self.compass_inner_mark_radius;
        let lw = self.line_width as f32;

        // 弧 (Java:173-174): drawArc(cx−cr+hbs, cy−cr+hbs, cd, cd, −180, 180)
        // → 盒中心 (cx−cr+hbs+cd/2, ...), 半径 cd/2, 下半圆 (render2d oracle:
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

        // 3 刻度线端点 (Java:175-180, int 坐标; cr/2 为 int 除)
        let ticks: [((i32, i32), (i32, i32)); 3] = [
            ((target_x + hbs, target_y - cr / 2 + hbs), (target_x + hbs, target_y - inner + hbs)), // 顶部竖刻度
            ((target_x + cr + hbs, target_y + hbs), (target_x + inner + hbs, target_y + hbs)),     // 右横刻度
            ((target_x - cr + hbs, target_y + hbs), (target_x - inner + hbs, target_y + hbs)),    // 左横刻度
        ];

        for &(width, color) in &[(lw + 2.0, COLOR_SHADE_SHAPE), (lw, COLOR_NUM)] {
            // 粗遍 (Java:142-144) / 细遍 (Java:147-149); 每遍内先弧后线 (drawMarks 序)
            // PORT: arc_r==0 (compassDiameter=0) 时 Java BasicStroke(CAP_ROUND) 对零尺寸弧
            // 仍画直径 lineWidth 的圆帽点 (弧两端点重合), Rust 此处整体跳过 — 仅退化布局
            // (preferredSize 0×0 组件不可见) 可达, 不复刻
            if arc_r > 0.0 {
                let outline = arc_stroke_outline(
                    rc_x as f32, rc_y as f32, arc_r as f32, a1 as f32, 180.0, width,
                );
                cv.fill_path(&outline, color, aa);
            }
            for &((x0, y0), (x1, y1)) in &ticks {
                // 端点连续旋转 (Java setTransform 语义), stadium 精确轮廓填充
                let (rx0, ry0) = rotate_point(x0 as f64, y0 as f64, target_x as f64, target_y as f64, theta);
                let (rx1, ry1) = rotate_point(x1 as f64, y1 as f64, target_x as f64, target_y as f64, theta);
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

/// tickLine (AttitudeOverlay.java:88): pitch 刻度对数, 2 对 = 4 条
pub const TICK_LINE: i32 = 2;
/// MaxAoA (Java:89): 俯仰/攻角满量程 ±30°
pub const MAX_AOA: i32 = 30;
/// MaxAoS (Java:90): 侧滑满量程 ±15°
pub const MAX_AOS: i32 = 15;
/// BASE_WIDTH/HEIGHT (Java:92-93): 构造期缺省尺寸 (reinit 前用)
pub const BASE_WIDTH: i32 = 100;
pub const BASE_HEIGHT: i32 = 200;
/// locater 调用点常量 (AttitudeOverlay.java:211/331): 中心参考弧径 / 侧滑球十字臂长
const CENTER_ROUND: i32 = 12;
const LOCATOR_SIZE: i32 = 6;
/// 攻角极限线关闭时的哨兵 y (Java:404-405 = −10, 画在窗口外被裁剪 → 不可见)
const AOA_LIMIT_OFF: i64 = -10;

/// 独立地平仪 (AttitudeOverlay.java:26)。C 类复刻只保留绘制语义核心:
/// drawTick 的数据换算 (L375-448) + locater 的图层序 (L134-185);
/// 窗口/拖动/WebLaF 边框阴影/EDT 节流属组装层, 不在本组件。
/// 画布 = [0, x_width)×[0, x_height) 的 Pixmap, Swing 面板裁剪由光栅化界天然给出。
/// PORT: Java 侧 init() 的匿名 topPanel (:325-332) 与 initpanel() 塞入的匿名子面板
/// (:200-215) **两级 paintComponent 都调 locater** — 半透明层 (α220/42/100) 可能双重
/// 合成 (地面有效 α≈246 而非单遍 220), 除非 WebLaF opaque 默认压制其中一级。
/// 本复刻锚定**单遍** locater 语义; C 类像素对拍验收前需 Java 端截图 oracle 确认
/// 真实合成遍数, 若确系双遍则对拍容差须按双遍基准校准。
pub struct AttitudeOverlay {
    /// 绘制区尺寸 (Java xWidth/xHeight, reinitConfig 已含 DPI 缩放的终值)
    pub x_width: i32,
    pub x_height: i32,
    /// showDirection (attitudeIndicatorDisplayDirection, 默认 false)
    pub show_direction: bool,
    /// showAoALimits (attitudeIndicatorDisplayAoALimits, 默认 true)
    pub show_aoa_limits: bool,
    // drawTick 计算缓存 (Java:55-71 public long 字段, 保留 long 语义)
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
    /// Java:28-97 字段初始化 (xWidth=BASE_WIDTH, xHeight=BASE_HEIGHT,
    /// showDirection=false, showAoALimits=true — 与 reinit 两分支的默认一致)
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

    /// reinitConfig 的绘制相关子集 (Java:230-270): 尺寸为调用方完成 DPI 缩放后的
    /// 终值 (Java:237-238 round(base·dpiScale))。
    pub fn reinit(&mut self, x_width: i32, x_height: i32, show_direction: bool, show_aoa_limits: bool) {
        self.x_width = x_width;
        self.x_height = x_height;
        self.show_direction = show_direction;
        self.show_aoa_limits = show_aoa_limits;
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// drawTick (Java:375-448): 遥测换算 + 地面多边形/刻度点集的平移旋转。
    /// `aoa_limits` = FM 的 (NoFlapsWing.AoACritHigh, AoACritLow);
    /// None = 无 FM —— Java blkx==null 分支, 极限线取哨兵 −10 (画在窗口外)。
    /// show_aoa_limits=false 同走哨兵 (Java:398 条件 b != null && showAoALimits)。
    /// PORT: 恒置脏恒返回 true = Java drawTick 末尾无条件 root.repaint() 的语义
    /// (40ms 节流在 onFlightData 组装层); 变化检测非本组件行为, 组装层按需自做。
    #[allow(clippy::too_many_arguments)] // 对齐 Java drawTick 的输入面
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
        // Java:385-387 — double 链, Math.round → long
        self.aoa_y = java_round_i64((aoa + MAX_AOA as f64) * h as f64 / (2 * MAX_AOA) as f64);
        self.aos_x = java_round_i64((-aos + MAX_AOS as f64) * w as f64 / (2 * MAX_AOS) as f64);
        self.pitch_y = java_round_i64((-pitch + MAX_AOA as f64) * h as f64 / (2 * MAX_AOA) as f64);

        // Java:389-393 — compassX/Y = (int)(w/4 · sin/cos) 后 widen 到 long 字段,
        // (int) 是 double→int 饱和 (JLS 5.1.3), 故先 as i32 再拓宽, 非 as i64
        if self.show_direction {
            let rads = compass.to_radians();
            self.compass_x = (((w / 4) as f64 * rads.sin()) as i32) as i64;
            self.compass_y = (((w / 4) as f64 * rads.cos()) as i32) as i64;
        }

        // Java:397-406 攻角极限 (FM 有效且开关开才显示)
        match aoa_limits {
            Some((crit_high, crit_low)) if self.show_aoa_limits => {
                self.aoa_limit_u = java_round_i64((crit_high + MAX_AOA as f64) * h as f64 / (2 * MAX_AOA) as f64);
                self.aoa_limit_d = java_round_i64((crit_low + MAX_AOA as f64) * h as f64 / (2 * MAX_AOA) as f64);
            }
            _ => {
                self.aoa_limit_u = AOA_LIMIT_OFF;
                self.aoa_limit_d = AOA_LIMIT_OFF;
            }
        }

        // 地面多边形角点 (Java:408-418): ±2 宽 (越界部分被窗口裁剪), 地面厚
        // 180/MaxAoA·h = 6h; int 坐标
        let mut p_s = [(0i32, 0i32); (4 + TICK_LINE * 4) as usize];
        let ground_h = 180 / MAX_AOA * h; // int 除 180/30=6 (整)
        p_s[0] = (-2 * w, 0);
        p_s[1] = (2 * w, 0);
        p_s[2] = (2 * w, ground_h);
        p_s[3] = (-2 * w, ground_h);

        // pitch 刻度 (Java:420-435): start=−90, dTick=90/(tickLine+1)=30 →
        // ±30°/±60° 刻度对; y = round((角)/(2·MaxAoA)·h)
        let start = -90.0f64;
        let d_tick = (90 / (TICK_LINE + 1)) as f64; // int 除 90/3=30
        for i in 0..TICK_LINE {
            let a = start + d_tick * (i + 1) as f64;
            let y_up = java_round_i32(a / (2 * MAX_AOA) as f64 * h as f64);
            // 对称刻度 (Java:430-434): −start − dTick·(i+1)
            let y_dn = java_round_i32(-a / (2 * MAX_AOA) as f64 * h as f64);
            p_s[(4 + 4 * i) as usize] = (-w, y_up);
            p_s[(4 + 4 * i + 1) as usize] = (w, y_up);
            p_s[(4 + 4 * i + 2) as usize] = (-w, y_dn);
            p_s[(4 + 4 * i + 3) as usize] = (w, y_dn);
        }

        // 平移 (Java:437-440): x += w/2 (int 除), y += Pitch (long→int 窄化)
        for p in &mut p_s {
            p.0 += w / 2;
            // PORT: Java p.y += Pitch 复合赋值隐式 (int)(y+long) — 位截断 (§2.2),
            // Rust as i32 饱和, 双转 (as u32) as i32 复刻取低 32 位
            p.1 = ((p.1 as i64 + self.pitch_y) as u32) as i32;
        }

        // 旋转 (Java:442 rotatePointMatrix): 绕 pC=(w/2, h/2) 旋转 roll 度后
        // Point.setLocation 取整 floor(x+0.5)。
        // PORT: Java pC = new Point(xWidth/2, xHeight/2) 是 int 除 (奇尺寸圆心取整)
        let theta = roll.to_radians();
        for (i, p) in p_s.iter().enumerate() {
            let (rx, ry) = rotate_point(p.0 as f64, p.1 as f64, (w / 2) as f64, (h / 2) as f64, theta);
            self.p_t[i] = (java_round_i32(rx), java_round_i32(ry));
        }

        self.dirty = true;
        true
    }

    /// locater (Java:134-185, 调用点 Java:211/331): 图层序绘制。
    /// 画布必须为 x_width×x_height (裁剪语义), 由调用方保证 — 防呆断言
    /// (更大画布会让 ±2w 地面多边形画出窗口界, 背离 Swing 裁剪语义)。
    pub fn draw(&mut self, cv: &mut PixCanvas, aa: bool) {
        debug_assert!(
            cv.width() == self.x_width && cv.height() == self.x_height,
            "画布须为 {}×{}, 实为 {}×{}",
            self.x_width, self.x_height, cv.width(), cv.height()
        );
        let w = self.x_width;
        let h = self.x_height;
        // 调用点实参 (Java:211): x=(int)AoS, y=(int)AoA — long→int 位截断 (§2.2 双转)
        let x = (self.aos_x as u32) as i32;
        let y = (self.aoa_y as u32) as i32;
        let cr_half = CENTER_ROUND / 2; // 6

        // 1. 地面多边形 (最底层, colorUnit) (Java:137-139)
        //    PORT: locater 硬编码 Application.colorUnit — transParentWhite/
        //    attitudeIndicatorUseNumColor 字段在 locater 无消费点 (Java 死配置), 不复刻
        let poly: [(f32, f32); 4] = [
            (self.p_t[0].0 as f32, self.p_t[0].1 as f32),
            (self.p_t[1].0 as f32, self.p_t[1].1 as f32),
            (self.p_t[2].0 as f32, self.p_t[2].1 as f32),
            (self.p_t[3].0 as f32, self.p_t[3].1 as f32),
        ];
        cv.fill_path(&poly, COLOR_UNIT, aa);

        // 2. 边框 (BasicStroke(1) 裸 = CAP_SQUARE/JOIN_MITER, shade) (Java:141-147)
        for &(x0, y0, x1, y1) in &[
            (0, 0, 0, h),
            (0, 0, w, 0),
            (0, h - 1, w - 1, h - 1),
            (w - 1, 0, w - 1, h - 1),
        ] {
            cv.draw_line_cap(x0, y0, x1, y1, 1.0, COLOR_SHADE_SHAPE, LineCapStyle::Square, aa);
        }

        // 3. pitch 刻度线 (仍 1px shade): 4 条 = 2·tickLine 对 (Java:149-152)
        for i in 0..(2 * TICK_LINE) as usize {
            let (x0, y0) = self.p_t[4 + 2 * i];
            let (x1, y1) = self.p_t[4 + 2 * i + 1];
            cv.draw_line_cap(x0, y0, x1, y1, 1.0, COLOR_SHADE_SHAPE, LineCapStyle::Square, aa);
        }

        // 4. 中心参考 (BasicStroke(3), colorNum) (Java:154-166)
        let mid_y = h / 2 - 1;
        for &(x0, x1) in &[
            (w / 2 - cr_half - w / 8 - 1, w / 2 - cr_half - 1), // 左内段
            (w / 2 + cr_half, w / 2 + cr_half + w / 8 - 1),     // 右内段
            (0, w / 8 - 1),                                     // 左外段
            (w - w / 8 + 1, w),                                 // 右外段
        ] {
            cv.draw_line_cap(x0, mid_y, x1, mid_y, 3.0, COLOR_NUM, LineCapStyle::Square, aa);
        }
        // 中心下半圆: drawArc(w/2−7, h/2−7, 12, 12, −180, 180) 的弧心 = 盒角+半径
        // = (w/2−1, h/2−1), r=6 (stroke_arc 收圆心而非盒角)
        cv.stroke_arc(
            w / 2 - 1, h / 2 - 1, CENTER_ROUND / 2,
            -180.0, 0.0, 3.0, COLOR_NUM, LineCapStyle::Square, aa,
        );

        // 5. 侧滑球十字 (BasicStroke(2), colorNum 承袭) (Java:168-171)
        let ls_half = LOCATOR_SIZE / 2; // 3
        cv.draw_line_cap(x - ls_half - 1, y - 1, x + ls_half - 1, y - 1, 2.0, COLOR_NUM, LineCapStyle::Square, aa);
        cv.draw_line_cap(x - 1, y - ls_half - 1, x - 1, y + ls_half - 1, 2.0, COLOR_NUM, LineCapStyle::Square, aa);

        // 6. 攻角极限线 (colorWarning, 仍 2px) (Java:173-176); 哨兵 −10 落在窗口外被裁
        // PORT: Java (int) AoALimitU/D — long→int 位截断 (§2.2 双转)
        let lu = (self.aoa_limit_u as u32) as i32;
        let ld = (self.aoa_limit_d as u32) as i32;
        cv.draw_line_cap(0, lu, w - 1, lu, 2.0, COLOR_WARNING, LineCapStyle::Square, aa);
        cv.draw_line_cap(0, ld, w - 1, ld, 2.0, COLOR_WARNING, LineCapStyle::Square, aa);

        // 7. 航向指针对 (Java:178-184): colorNum 正向 + warning 反向
        if self.show_direction {
            let (ccx, ccy) = (w / 2, h / 2);
            // PORT: Java (int)(width/2 ± compassX) — long 加法后 →int 位截断 (§2.2 双转)
            let px = ((ccx as i64 + self.compass_x) as u32) as i32;
            let py = ((ccy as i64 + self.compass_y) as u32) as i32;
            let mx = ((ccx as i64 - self.compass_x) as u32) as i32;
            let my = ((ccy as i64 - self.compass_y) as u32) as i32;
            cv.draw_line_cap(ccx, ccy, px, py, 2.0, COLOR_NUM, LineCapStyle::Square, aa);
            cv.draw_line_cap(ccx, ccy, mx, my, 2.0, COLOR_WARNING, LineCapStyle::Square, aa);
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
// OverlayHost 挂载 (Java Controller.java:690 registerWithPreview("enableAttitudeIndicator"))
// ---------------------------------------------------------------------------

/// 地平仪共享句柄 (minihud_overlay_spec 先例: render 闭包与喂入方共享 state)
pub type AttitudeOverlayHandle = Rc<RefCell<AttitudeOverlay>>;

/// 地平仪 OverlaySpec + live 句柄。参数为 reinitConfig (:230-270) 的配置面:
/// `base_width`/`base_height` = attitudeIndicatorWidth/Height (cfg 缺省 150/300),
/// 工厂内完成 DPI 缩放 (Java :237-238 round(base·dpiScale), §2.3 floor(x+0.5));
/// `show_direction`/`show_aoa_limits` = attitudeIndicatorDisplayDirection (false) /
/// ...DisplayAoALimits (true)。
/// PORT(边框不承载): Java totalWidth = xWidth+4+sw·2 的 sw 边距是 WebLaF 窗口装饰
/// (enableAttitudeIndicatorEdge, 默认 false), host 无边框层 — spec 尺寸 = 内容区
/// x_width×x_height (draw 的画布断言钉内容尺寸, 裁剪语义)。
/// 初始态 = 未飞形态 (AoA/AoS/Pitch 0, drawTick 未跑), 预览/游戏共用; live 由喂入方
/// update_telemetry 推进 (40ms 节流归组装层, Java onFlightData freqMili)
pub fn attitude_overlay_spec(
    base_width: i32,
    base_height: i32,
    dpi_scale: f64,
    show_direction: bool,
    show_aoa_limits: bool,
) -> Result<(AttitudeOverlayHandle, OverlaySpec), String> {
    let x_width = (base_width as f64 * dpi_scale + 0.5).floor() as i32;
    let x_height = (base_height as f64 * dpi_scale + 0.5).floor() as i32;
    let mut overlay = AttitudeOverlay::new();
    overlay.reinit(x_width, x_height, show_direction, show_aoa_limits);
    let handle: AttitudeOverlayHandle = Rc::new(RefCell::new(overlay));
    let render_handle = Rc::clone(&handle);
    Ok((
        handle,
        OverlaySpec {
            // Java LinkedHashMap 键 = configKey (Controller.java:690)
            id: "enableAttitudeIndicator".to_string(),
            config_key: "enableAttitudeIndicator".to_string(),
            width: x_width,
            height: x_height,
            render: Box::new(move |cv: &mut PixCanvas| {
                // 生产 AA 恒开 (Application.java:102 graphAASetting 默认 ON)
                render_handle.borrow_mut().draw(cv, true);
            }),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &str = "../../../fonts/sarasa-mono-sc-bold.ttf";

    fn font() -> LoadedFont {
        LoadedFont::new(std::path::Path::new(FONT), 24).unwrap()
    }

    /// 读预乘 RGBA 像素 (与 gauges_bars/render2d 测试同约定)
    fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
        let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
        [d[0], d[1], d[2], d[3]]
    }

    fn a(c: &PixCanvas, x: i32, y: i32) -> u8 {
        px(c, x, y)[3]
    }

    fn hud(pitch: f64, roll: f64, slip: f64, valid: bool) -> vm_core::hud_data::HUDData {
        vm_core::hud_data::Builder {
            pitch,
            roll,
            slip,
            pitch_valid: valid,
            ..Default::default()
        }
        .build()
    }

    /// 样式上下文: cd=20, cr=10, inner=12, lw=2, half=1 (MinimalHUDContext 典型比例)
    fn gauge() -> AttitudeIndicatorGauge {
        let mut g = AttitudeIndicatorGauge::new();
        g.set_style_context(20, 10, 12, 2, 1, 24);
        g
    }

    // -----------------------------------------------------------------------
    // 格式化 (Java String.format 语义)
    // -----------------------------------------------------------------------

    /// %3d: 右对齐宽 3 空格补; 负号占宽; 超宽原样
    #[test]
    fn fmt_d3_padding() {
        assert_eq!(fmt_d3(0), "  0");
        assert_eq!(fmt_d3(5), "  5");
        assert_eq!(fmt_d3(-7), " -7");
        assert_eq!(fmt_d3(123), "123");
        assert_eq!(fmt_d3(1234), "1234");
    }

    /// %-4.1f: 左对齐宽 4, HALF_UP 1 位小数
    #[test]
    fn fmt_f41_rounding_and_padding() {
        assert_eq!(fmt_f41(0.0), "0.0 ");
        assert_eq!(fmt_f41(3.25), "3.3 ", "精确 .25 二进制值 HALF_UP 进位");
        assert_eq!(fmt_f41(2.5), "2.5 ");
        assert_eq!(fmt_f41(9.96), "10.0", "进位自然超宽");
        assert_eq!(fmt_f41(15.0), "15.0");
        assert_eq!(fmt_f41(100.3), "100.3");
        assert_eq!(fmt_f41(f64::NAN), "NaN ");
        assert_eq!(fmt_f41(f64::INFINITY), "Infinity");
    }

    // -----------------------------------------------------------------------
    // AttitudeIndicatorGauge — 双模式目标点与数据换算 (Java 公式手算)
    // -----------------------------------------------------------------------

    /// 双模式符号表 (Java:112-125 代码值):
    /// cd=20 → radius=10, center=(40,50) (x=30,y=40)。
    /// pitch=10 slip=5 → aosX=(int)(−5·96/30)=−16;
    /// body (−1,−1): target=(40+(−1)(−16)·3/2, 50−(int)(10/2)) = (64,45);
    /// earth (+1,+1): target=(40−24, 50+5) = (16,55)。
    #[test]
    fn indicator_gauge_dual_mode_target_points() {
        let mut g = gauge();
        assert!(g.on_data_update(&hud(10.0, 0.0, 5.0, true)));
        assert_eq!(g.aos_x, -16, "aosX = (int)(−slip·4·fontSize/30)");
        g.set_inertial_mode(false);
        assert_eq!(g.target_point(30, 40), (64, 45), "body: horizon 随 pitch 上移/侧滑右移");
        g.set_inertial_mode(true);
        assert_eq!(g.target_point(30, 40), (16, 55), "earth: 符号全翻");
        // roll 符号: body=+1, earth=−1 (roll_theta)
        let mut zb = gauge();
        zb.on_data_update(&hud(0.0, 90.0, 0.0, true));
        assert!((zb.roll_theta() - std::f64::consts::FRAC_PI_2).abs() < 1e-12, "body θ=+90°");
        zb.set_inertial_mode(true);
        assert!((zb.roll_theta() + std::f64::consts::FRAC_PI_2).abs() < 1e-12, "earth θ=−90°");
    }

    /// onDataUpdate 的文本族 (Java:210-223):
    /// - pitchValid=false → sAttitude 空 / roundHorizon=0
    /// - pitchValid=true → "%3d" of |round(pitch)|
    /// - slip → "%-4.1f" of |round(slip·10)/10|, roundSlip 仅保符号
    #[test]
    fn indicator_gauge_on_data_update_texts() {
        let mut g = gauge();
        g.on_data_update(&hud(7.6, 0.0, -2.44, true));
        assert_eq!(g.round_horizon, 8, "round(7.6)=8");
        assert_eq!(g.s_attitude, "  8");
        // slipValue = round(−24.4)/10 = −24/10 = −2.4 → 符号 −1, 文本 "2.4 "
        assert_eq!(g.round_slip, -1);
        assert_eq!(g.s_sideslip, "2.4 ");
        // pitch 无效
        g.on_data_update(&hud(7.6, 0.0, 0.0, false));
        assert_eq!(g.s_attitude, "");
        assert_eq!(g.round_horizon, 0);
        assert_eq!(g.s_sideslip, "0.0 ");
        // pitch 为负 → 文本色走 colorUnit 分支的 roundHorizon<0
        g.on_data_update(&hud(-3.0, 0.0, 0.0, true));
        assert_eq!(g.round_horizon, -3);
        assert_eq!(g.s_attitude, "  3");
    }

    // -----------------------------------------------------------------------
    // AttitudeIndicatorGauge — marks 几何 (像素断言, 手算坐标)
    // -----------------------------------------------------------------------

    /// roll=0 (无旋转): 弧盒 (32,42,20,20) → 圆心 (42,52) r=10, 下半圆;
    /// 细 w=2 带 r∈[9,11], 粗 w=4 带 r∈[8,12]; 顶部竖刻度 x=42 y∈[40,47];
    /// 左右横刻度 y=52 x∈[30,32]/[52,54]。
    #[test]
    fn indicator_gauge_marks_geometry_roll0() {
        let mut g = gauge();
        g.on_data_update(&hud(0.0, 0.0, 0.0, true));
        let mut cv = PixCanvas::new(200, 200).unwrap();
        g.draw(&mut cv, 30, 40, None, false);

        // 弧底细带内 (d=10.5): colorNum(240) 叠 shade(42) ≈ 242
        assert!((235..=250).contains(&a(&cv, 42, 62)), "弧底细带 d=10.5, a={}", a(&cv, 42, 62));
        // 外侧粗独占环 (d=11.5 ∈ [11,12]): 单层 shade 精确 42
        assert_eq!(a(&cv, 42, 63), 42, "外粗独占环 d=11.5");
        // 粗带外 (d=12.6): 透明
        assert_eq!(a(&cv, 42, 64), 0, "粗带外 d=12.6");
        // 上半圆无弧 (d=9.9 但角度 +135° 不在跨度): 顶部竖刻度在 x=42 不在 (49,45)
        assert_eq!(a(&cv, 49, 45), 0, "上半圆无弧");
        // 顶部竖刻度 (x=42, y∈[40,47] 带)
        assert!(a(&cv, 42, 43) > 200, "顶部竖刻度");
        // 左横刻度 (x∈[30,32]+圆帽, y=52)
        assert!(a(&cv, 30, 52) > 200, "左横刻度");
        // 右横刻度
        assert!(a(&cv, 53, 52) > 200, "右横刻度");
        // 牵引线零长 → 圆点: 粗 r=2 + 细 r=1 的 colorLabel(166) 叠 shade(42) ≈ 181
        let dot = a(&cv, 40, 50);
        assert!((170..=195).contains(&dot), "牵引线圆点 a={dot}");
        // 弧圆心处无图形 (下半圆弧是环带)
        assert_eq!(a(&cv, 42, 52), 0, "弧圆心透明");
    }

    /// 滚转旋转 (body roll=90, θ=+90° 视觉顺时针):
    /// 弧心 (42,52) 绕 target(40,50) 旋转 → (38,52), 角度跨度 [−180,0]−90 = [−270,−90]
    /// → 左半圆 (6点→9点→12点); 顶部竖刻度旋到右侧水平 (43,52)→(50,52);
    /// 右横刻度旋到竖直向下; 左横刻度旋到竖直向上。
    #[test]
    fn indicator_gauge_marks_rotation_body_roll90() {
        let mut g = gauge();
        g.on_data_update(&hud(0.0, 90.0, 0.0, true));
        let mut cv = PixCanvas::new(200, 200).unwrap();
        g.draw(&mut cv, 30, 40, None, false);

        // 左半圆 9 点 (d=9.5 from (38,52))
        assert!(a(&cv, 28, 52) > 200, "左半圆 9 点 (旋转后弧)");
        // 右下 45° (d=10.6) 不在跨度 → 无弧
        assert_eq!(a(&cv, 45, 59), 0, "右下象限无弧");
        // 顶部刻度旋到右侧水平 (43..50, 52)
        assert!(a(&cv, 46, 52) > 200, "顶刻度旋至右侧");
        // 右横刻度旋到 (38,62)-(38,64) 竖直向下
        assert!(a(&cv, 38, 63) > 200, "右刻度旋至向下");
        // 左横刻度旋到 (38,40)-(38,42) 竖直向上
        assert!(a(&cv, 38, 41) > 200, "左刻度旋至向上");
    }

    /// 惯性模式 roll=90 (θ=−90° 视觉逆时针): 弧心 → (42,48), 跨度 [−90,90]
    /// → 右半圆 — 与 body 模式镜像。
    #[test]
    fn indicator_gauge_marks_rotation_inertial_roll90() {
        let mut g = gauge();
        g.set_inertial_mode(true);
        g.on_data_update(&hud(0.0, 90.0, 0.0, true));
        let mut cv = PixCanvas::new(200, 200).unwrap();
        g.draw(&mut cv, 30, 40, None, false);

        // 右半圆 3 点 (d=10.6 from (42,48), φ=−45° 在跨度)
        assert!(a(&cv, 49, 55) > 200, "右半圆右下 45° (惯性模式)");
        // 左上 135° 不在跨度 → 无弧 (d=10.6)
        assert_eq!(a(&cv, 34, 40), 0, "左上象限无弧");
    }

    /// 文本带 (Java:153-166): pitch 文本在 target 右侧 gap 起, slip 文本以
    /// "888" 模板宽锁定左缘在 target 左侧; 基线 target_y−1。
    /// x=100,y=80 → center (110,90); slip=−3.4 → aosX=10 → target=(95,84)。
    #[test]
    fn indicator_gauge_text_zones() {
        let mut g = gauge();
        g.on_data_update(&hud(12.0, 0.0, -3.4, true));
        let f = font();
        let mut cv = PixCanvas::new(220, 220).unwrap();
        g.draw(&mut cv, 100, 80, Some(&f), false);

        let (tx, ty) = (95, 84);
        // marks 最远到达 target+cr+hbs+粗帽 ≈ 110 → x≥115 的非零像素必为 pitch 文本
        let mut right = false;
        for yy in (ty - 24 - 2)..(ty + 8) {
            for xx in 115..160 {
                if a(&cv, xx, yy) > 0 {
                    right = true;
                }
            }
        }
        assert!(right, "pitch 文本出现在 target 右侧带 (target_x={tx})");
        // marks 左侧最远 target−cr−hbs−粗帽 ≈ 80 → x≤78 的非零像素必为 slip 文本
        let mut left = false;
        for yy in (ty - 24 - 2)..(ty + 8) {
            for xx in 20..78 {
                if a(&cv, xx, yy) > 0 {
                    left = true;
                }
            }
        }
        assert!(left, "slip 文本出现在 target 左侧带");
    }

    /// 脏检查 (W3 契约): 同值不脏, 变化置脏, draw 清脏, 模式切换置脏
    #[test]
    fn indicator_gauge_dirty_checking() {
        let mut g = gauge();
        assert!(g.on_data_update(&hud(1.0, 2.0, 3.0, true)));
        assert!(!g.on_data_update(&hud(1.0, 2.0, 3.0, true)), "同值不脏");
        assert!(g.is_dirty());
        let mut cv = PixCanvas::new(80, 80).unwrap();
        g.draw(&mut cv, 10, 10, None, false);
        assert!(!g.is_dirty(), "draw 后清脏");
        g.set_inertial_mode(true);
        assert!(g.is_dirty(), "模式切换置脏");
    }

    // -----------------------------------------------------------------------
    // AttitudeOverlay — 点集几何 (Java drawTick 公式手算)
    // -----------------------------------------------------------------------

    /// w=150 h=300 pitch=0 roll=0: Pitch=150 → 地面多边形角点
    /// (±225/±375, 150/1950); 刻度 4 条 y∈{−150,450,0,300} (±60°/±30°对)。
    #[test]
    fn overlay_polygon_and_ticks_roll0_pitch0() {
        let mut o = AttitudeOverlay::new();
        o.reinit(150, 300, false, false);
        o.update_telemetry(0.0, 0.0, 0.0, 0.0, 0.0, None);
        assert_eq!(o.pitch_y, 150, "Pitch = round((−0+30)·300/60)");
        assert_eq!(o.p_t[0], (-225, 150), "多边形左上 (−2w+w/2, 0+Pitch)");
        assert_eq!(o.p_t[1], (375, 150), "多边形右上");
        assert_eq!(o.p_t[2], (375, 1950), "多边形右下 (6h)");
        assert_eq!(o.p_t[3], (-225, 1950), "多边形左下");
        // 刻度对: i=0 → ±60° (y=∓300+150), i=1 → ±30° (y=∓150+150)
        assert_eq!(o.p_t[4], (-75, -150), "60° 刻度左端");
        assert_eq!(o.p_t[5], (225, -150), "60° 刻度右端");
        assert_eq!(o.p_t[6], (-75, 450), "−60° 刻度左端");
        assert_eq!(o.p_t[7], (225, 450));
        assert_eq!(o.p_t[8], (-75, 0), "30° 刻度左端");
        assert_eq!(o.p_t[9], (225, 0));
        assert_eq!(o.p_t[10], (-75, 300), "−30° 刻度左端");
        assert_eq!(o.p_t[11], (225, 300));
    }

    /// roll=90 (绕 (75,150) 视觉顺时针): 手算旋转+floor(x+0.5) 取整端点 —
    /// 地平线竖直化 (75,−150)-(75,450), 地面在左 (x≤75)。
    #[test]
    fn overlay_polygon_roll90_endpoints() {
        let mut o = AttitudeOverlay::new();
        o.reinit(150, 300, false, false);
        o.update_telemetry(0.0, 0.0, 0.0, 90.0, 0.0, None);
        assert_eq!(o.p_t[0], (75, -150), "地平线左端点旋至正上方");
        assert_eq!(o.p_t[1], (75, 450), "地平线右端点旋至正下方");
        assert_eq!(o.p_t[2], (-1725, 450), "深地面角点");
        assert_eq!(o.p_t[3], (-1725, -150));
        // 60° 刻度线旋成竖直 x=375
        assert_eq!(o.p_t[4], (375, 0));
        assert_eq!(o.p_t[5], (375, 300));
    }

    /// pitch=+10: Pitch=round(20·5)=100 → 地平线上移到 y=100 (天地分界随俯仰);
    /// aoa/aos 映射与航向分量。
    #[test]
    fn overlay_pitch_offset_and_mappings() {
        let mut o = AttitudeOverlay::new();
        o.reinit(150, 300, true, true);
        o.update_telemetry(5.0, -3.0, 10.0, 0.0, 180.0, Some((18.0, -4.0)));
        assert_eq!(o.pitch_y, 100, "Pitch = round((−10+30)·300/60) = 100");
        assert_eq!(o.p_t[0], (-225, 100), "地平线随 pitch 上移");
        assert_eq!(o.p_t[1], (375, 100));
        assert_eq!(o.aoa_y, 175, "AoA = round((5+30)·300/60)");
        assert_eq!(o.aos_x, 90, "AoS = round((3+15)·150/30)");
        // compass=180°: sin(π)≈1.2e−16 → compassX=0; cos(π)=−1 → compassY=−37 (w/4=37)
        assert_eq!(o.compass_x, 0);
        assert_eq!(o.compass_y, -37);
        // 攻角极限: U=round(48·5)=240, D=round(26·5)=130
        assert_eq!(o.aoa_limit_u, 240);
        assert_eq!(o.aoa_limit_d, 130);

        // 关闭开关 → 哨兵 −10 (画在窗口外)
        o.reinit(150, 300, true, false);
        o.update_telemetry(5.0, -3.0, 10.0, 0.0, 180.0, Some((18.0, -4.0)));
        assert_eq!(o.aoa_limit_u, AOA_LIMIT_OFF);
        assert_eq!(o.aoa_limit_d, AOA_LIMIT_OFF);
        // 无 FM 同哨兵
        o.reinit(150, 300, true, true);
        o.update_telemetry(5.0, -3.0, 10.0, 0.0, 180.0, None);
        assert_eq!(o.aoa_limit_u, AOA_LIMIT_OFF);
    }

    // -----------------------------------------------------------------------
    // AttitudeOverlay — 像素级图层 (locater 绘制序)
    // -----------------------------------------------------------------------

    /// 图层与裁剪: 地面 colorUnit 下半、上半透明; 中线行; 侧滑球十字;
    /// 攻角极限线 (开/关); 航向指针对; 窗口裁剪天然成立。
    #[test]
    fn overlay_draw_layers_pixels() {
        let mut o = AttitudeOverlay::new();
        o.reinit(150, 300, true, true);
        o.update_telemetry(5.0, 0.0, 0.0, 0.0, 0.0, Some((18.0, -4.0)));
        let mut cv = PixCanvas::new(150, 300).unwrap();
        o.draw(&mut cv, false);

        // 地面: (75,200) colorUnit a=220 预乘 RGB≈143
        let g = px(&cv, 75, 200);
        assert_eq!(g[3], 220, "地面 alpha=colorUnit 220");
        assert!((g[0] as i32 - 143).abs() <= 1 && g[0] == g[1] && g[1] == g[2], "预乘灰 {g:?}");
        // 天空: (75,100) 透明 (刻度不在该行, 中线在 149)
        assert_eq!(a(&cv, 75, 100), 0, "地平线上方透明");
        // 左外中线段 (0..18, 149) colorNum 240
        assert!((230..=255).contains(&a(&cv, 5, 148)), "中线行 a={}", a(&cv, 5, 148));
        // 侧滑球十字 (AoS=75, AoA=175): colorNum(240) 叠地面(220) ≈ 247
        assert!(a(&cv, 75, 174) > 240, "十字横臂叠地面");
        // 攻角极限 U=240 (warning 100 叠地面 220 ≈ 234), D=130 (叠天空 = 100)
        let u = a(&cv, 120, 240) as i32;
        assert!((225..=245).contains(&u), "上限线叠地面 a={u}");
        let d = a(&cv, 120, 130) as i32;
        assert!((90..=110).contains(&d), "下限线叠天空 a=100");
        // 航向指针对 (compass=0 → 竖直): 正向 colorNum 叠地面 / 反向 warning 叠天空
        assert!(a(&cv, 75, 180) > 240, "航向正向臂");
        let m = a(&cv, 75, 120) as i32;
        assert!((90..=110).contains(&m), "航向反向臂");
        // 地面延伸被窗口裁剪: 近右缘仍纯地面 (x=149 列有边框 shade 叠加, 避开)
        assert_eq!(a(&cv, 140, 200), 220, "多边形 ±2w 宽被画布裁剪");
        // 边框存在 (shade 弱 alpha; 取天空段避开地平线行的地面叠色)
        assert_eq!(a(&cv, 0, 140), 42, "左边框 shade");
        // pitch 刻度行 (60° 对旋平后 y=0) 与 30° 对 (y=300 界外被裁, y=0 可见)
        assert!(a(&cv, 120, 0) > 0, "30° 刻度行 y=0 (60° 对在 y=−150 已出界)");
    }

    /// 极限线关闭 (哨兵 −10): 窗口内无 warning 横线
    #[test]
    fn overlay_aoa_limits_off_invisible() {
        let mut o = AttitudeOverlay::new();
        o.reinit(150, 300, false, false);
        o.update_telemetry(0.0, 0.0, 0.0, 0.0, 0.0, Some((18.0, -4.0)));
        let mut cv = PixCanvas::new(150, 300).unwrap();
        o.draw(&mut cv, false);
        assert_eq!(a(&cv, 120, 130), 0, "哨兵 −10 → 极限线不可见");
        assert_eq!(a(&cv, 120, 5), 0, "窗口上部无其他图形");
    }

    // -----------------------------------------------------------------------
    // Blocker 回归: CAP_ROUND 端帽绕弧端点 (非弧心) — 孔内/弧心上方无杂散填充
    // -----------------------------------------------------------------------

    /// roll=0: 弧心 (42,52) r=10, 粗带 [8,12] 细带 [9,11]。
    /// 孔内点 (35,52)/(49,52): 距弧心 <8 (带内缘内)、距弧端帽 (32,52)/(52,52) >2、
    /// 距牵引圆点 (40,50) r=2 与三刻度线均 >2 —— 修复前端帽绕弧心的自相交轮廓
    /// 在孔内产出杂散填充 (Java 该处为空), 修复后应为 0。
    #[test]
    fn indicator_gauge_arc_caps_hole_interior_clean() {
        let mut g = gauge();
        g.on_data_update(&hud(0.0, 0.0, 0.0, true));
        let mut cv = PixCanvas::new(200, 200).unwrap();
        g.draw(&mut cv, 30, 40, None, false);
        assert_eq!(a(&cv, 35, 52), 0, "孔内左 (距弧心 6.5 < 内缘 8)");
        assert_eq!(a(&cv, 49, 52), 0, "孔内右 (距弧心 7.5 < 内缘 8, 距弧端帽 2.55>2)");
    }

    /// roll=45: 弧心旋至 (40,52.83), 孔内深处的像素距弧带 (≥8)/牵引圆点 (r=2)/
    /// 三条旋转刻度线 (粗帽 r=2) 均有余量 —— 修复前孔内出现 19 个杂散像素
    /// (alpha 42~243, 距弧心最近 0.84px), 修复后全空。
    #[test]
    fn indicator_gauge_arc_caps_hole_interior_clean_rolled() {
        let mut g = gauge();
        g.on_data_update(&hud(0.0, 45.0, 0.0, true));
        let mut cv = PixCanvas::new(200, 200).unwrap();
        g.draw(&mut cv, 30, 40, None, false);
        for (x, y) in [(40, 53), (39, 52), (41, 54), (42, 52)] {
            assert_eq!(a(&cv, x, y), 0, "滚转 45° 弧孔内 ({x},{y})");
        }
    }

    // 注: 弧端帽的【外伸】无法用独立像素断言 —— Java 几何里弧两端点 (roll=0 时
    // (32,52)/(52,52)) 与左右横刻度端点重合, 同为 CAP_ROUND 同宽的刻度 stadium 帽
    // 与弧帽同圆心同半径, 弧帽永远被刻度帽完全覆盖 (实测 roll=90 时 (38,43) =
    // 弧帽+刻度帽两遍 shade=77, 与 Java 合成一致)。端帽修复的可见差异仅在孔内
    // 杂散清除, 由上面两个 hole_interior 测试钉死。

    /// Blocker 回归: 中心下半圆弧心 = drawArc 盒角+半径 = (w/2−1, h/2−1) r=6
    /// (stroke_arc 收圆心入参, 非盒角)。w=150 h=300 pitch=0 roll=0:
    /// - 弧底 (74,155): 距正确圆心 6.5 ∈ 描边带 [4.5,7.5], colorNum 叠地面 ≈253
    /// - (62,143): 距错误圆心 (68,143) 5.5 曾为弧体, 距正确圆心 13.2 带外 → 空
    /// - (79,149): 弧体桥接中线右内段 (方帽起 x=81) 前的 2px —— 修复前错位弧
    ///   留出 Java 没有的断口
    #[test]
    fn overlay_center_arc_true_center() {
        let mut o = AttitudeOverlay::new();
        o.reinit(150, 300, false, false);
        o.update_telemetry(0.0, 0.0, 0.0, 0.0, 0.0, None);
        let mut cv = PixCanvas::new(150, 300).unwrap();
        o.draw(&mut cv, false);
        assert!(a(&cv, 74, 155) > 230, "弧底在正确圆心 (74,149) 下方, a={}", a(&cv, 74, 155));
        assert!(a(&cv, 74, 154) > 230, "弧体径向 5.5 带内");
        assert_eq!(a(&cv, 62, 143), 0, "旧错误圆心 (68,143) 的弧体带内点已无弧");
        assert!(a(&cv, 79, 149) > 230, "弧端桥接中线右内段, a={}", a(&cv, 79, 149));
    }

    /// live 工厂: DPI 缩放尺寸 (150%→round(150·1.5)=225/round(300·1.5)=450) +
    /// 句柄喂入后 render 闭包画到新值 (共享 state 生效) + 注册键
    #[test]
    fn attitude_overlay_spec_dpi_and_shared_state() {
        let (h, mut spec) = attitude_overlay_spec(150, 300, 1.5, false, true).unwrap();
        assert_eq!((spec.width, spec.height), (225, 450));
        assert_eq!((spec.id.as_str(), spec.config_key.as_str()),
            ("enableAttitudeIndicator", "enableAttitudeIndicator"));
        // 喂入: aoa=10 → AoA = round((10+30)·450/60) = 300
        h.borrow_mut().update_telemetry(10.0, 0.0, 0.0, 0.0, 0.0, None);
        assert_eq!(h.borrow().aoa_y, 300);
        let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
        (spec.render)(&mut cv);
        // 侧滑球十字在 y=300 (colorNum 线体, BasicStroke(2) 行 299..300)
        assert!(a(&cv, 110, 299) > 0 || a(&cv, 110, 300) > 0, "十字随 aoa 喂入下移");
    }
}
