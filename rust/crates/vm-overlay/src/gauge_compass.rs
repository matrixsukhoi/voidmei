//! gauge_compass: CompassGauge C 类语义复刻 (src/ui/component/CompassGauge.java)
//!
//! 罗盘组件的视觉语义 (保真对象 = 像素输出与状态行为, 非代码结构):
//!
//! | 图层序 | 内容 | 颜色 | 线型 |
//! |---|---|---|---|
//! | 1 底 | 北三角 (fillPolygon + 1px 描边) | colorUnit 填 / colorShadeShape 边 | THIN_STROKE=BasicStroke(1) |
//! | 2 中 | 罗盘圆环双层描边 (drawOval) | colorShadeShape (宽 lw+2) / colorNum (宽 lw) | CAP_ROUND+JOIN_ROUND |
//! | 3 上 | 航向指针或固定机标线 (drawLine) | 同上双层 | 同上 |
//! | 4 文本 | 航向读数 + 地图网格 (drawStringShade) | colorNum + 阴影 (x+1,y+1) colorShadeShape | — |
//!
//! 坐标系双模式 (CompassGauge.java:34-37, 逻辑反转以对齐 AttitudeIndicatorGauge):
//! - inertial_mode=false (配置 OFF, 默认) = 离体: 指针随航向旋转, 北三角固定 12 点钟;
//! - inertial_mode=true  (配置 ON)        = 随体: 机标线固定 12 点钟, 北三角转 -compassRads。
//!
//! CompassDelta 平滑行为: Java 链路 Service.updateCompass (L1101-1113, 仪表罗盘优先/
//! 地图方向回退) → HUDData.heading (HUDCalculator L36) → onDataUpdate, **无任何插值/
//! 平滑/低通**——每轮 ~10Hz 轮询值直接驱动指针。Rust 侧同样直通 (update 即时重算);
//! -65535 哨兵回退分支属数据层, 已落 vm-data::service_loop (update_compass), 不在本文件。

use crate::font::LoadedFont;
use crate::gauges_bars::{COLOR_NUM, COLOR_SHADE_SHAPE};
use crate::render2d::{LineCapStyle, PixCanvas};

/// Application.java:109 colorUnit — 北三角填充色 (RGBA 直通)
pub const COLOR_UNIT: [u8; 4] = [166, 166, 166, 220];

/// Java (int) double 强转语义: 向零截断, NaN→0, ±∞ 饱和到 MIN/MAX —
/// 与 Rust `as i32` (饱和转换) 逐例一致 (PORTING §2.2 的 long 位截断差异不适用于浮点)
#[inline]
fn trunc_i32(v: f64) -> i32 {
    v as i32
}

/// Java Math.toRadians(deg) + (float) 收窄 (CompassGauge.java:88):
/// OpenJDK 实现 = degrees / 180.0 * PI (f64), 存入 float 字段 compassRads
#[inline]
fn java_to_radians_f32(deg: f64) -> f32 {
    (deg / 180.0 * std::f64::consts::PI) as f32
}

/// Java String.format("%3.0f") 语义 (CompassGauge.java:95):
/// HALF_UP 舍入 0 位小数 (按精确十进制值), 负零保号, 右对齐宽 3 空格补;
/// NaN→"NaN", ±∞→"Infinity"/"-Infinity", |v|≥2^63→完整十进制 (畸形遥测可达,
/// as i64 饱和串 9223372036854775807 是错误输出)。
/// (与 gauges_bars::fmt_pct3 同式, 该函数私有不可复用故复制, FlapAngleBar 已钉死边界 oracle)
fn fmt_heading3(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    // PORT: Formatter 对 ±∞ 输出常量 "Infinity"/"-Infinity" (org.json "1e999"→inf 可达)
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    let neg = v < 0.0 || (v == 0.0 && v.is_sign_negative());
    let m = v.abs();
    let f = m.floor();
    let r = if m - f >= 0.5 { f + 1.0 } else { f };
    // PORT: r ≥ 2^63 超 i64 域, 按完整十进制展开 (此域 ULP≥2048, m 必为整值, {:.0} 无舍入分歧)
    let mut s = if r >= 9_223_372_036_854_775_808.0 {
        format!("{:.0}", r)
    } else {
        format!("{}", r as i64)
    };
    if neg {
        s.insert(0, '-');
    }
    while s.len() < 3 {
        s.insert(0, ' ');
    }
    s
}

/// 指针末端偏移 compassDx/Dy (CompassGauge.java:92-93)。
/// PORT: (radius * 1.3f) 先做 f32 乘 (int×float 提升 float), 再提升 f64 乘 sin/cos;
/// 注意此处 1.3f 与 draw 内机标线的 double 1.3 (Java:139) 精度不同
fn compass_dxy(radius: i32, compass_rads: f32) -> (i32, i32) {
    let outer = radius as f32 * 1.3f32;
    let rad = compass_rads as f64;
    (trunc_i32(outer as f64 * rad.sin()), trunc_i32(outer as f64 * rad.cos()))
}

/// 离体模式指针内端 (0.618r 处, CompassGauge.java:151-152)。0.618 是 double 字面量, 全程 f64
fn pointer_tip(cx: i32, cy: i32, r: i32, compass_rads: f32) -> (i32, i32) {
    let rad = compass_rads as f64;
    (
        cx + trunc_i32(0.618 * r as f64 * rad.sin()),
        cy - trunc_i32(0.618 * r as f64 * rad.cos()),
    )
}

/// 随体模式固定机标线两端 (CompassGauge.java:138-139):
/// tip = cy - (int)(0.618·r), end = cy - (int)(1.3·r) — 1.3 为 double 字面量
fn fixed_segment(cx: i32, cy: i32, r: i32) -> ((i32, i32), (i32, i32)) {
    (
        (cx, cy - trunc_i32(0.618 * r as f64)),
        (cx, cy - trunc_i32(1.3 * r as f64)),
    )
}

/// 北三角三顶点 (CompassGauge.java:185-213): tip 在圆外 (r+0.35r), 底边中点在圆周上,
/// 底边沿圆切向 (cosθ, sinθ) 展开半宽 0.30r。角 0 = 12 点钟 (北)。
/// PORT: Java 187-188 (int)(radius*0.35)/(int)(radius*0.30) 的 f64 积截断值
/// 直接进入几何 (r=20 → 6/6 而非 7/6, 0.35/0.30 的 f64 表示略小于精确值);
/// radius + triangle_height 为 int 加法, 极端半径下 Java 静默回绕 / Rust debug
/// panic (§2.2), 真实布局幅度不可达, 备查
fn north_triangle(cx: i32, cy: i32, radius: i32, angle_rads: f64) -> [(i32, i32); 3] {
    let triangle_height = trunc_i32(radius as f64 * 0.35);
    let triangle_half_base = trunc_i32(radius as f64 * 0.30);
    let tip_dist = (radius + triangle_height) as f64;
    let (s, c) = (angle_rads.sin(), angle_rads.cos());
    let tip = (
        cx + trunc_i32(tip_dist * s),
        cy - trunc_i32(tip_dist * c),
    );
    let base = (
        cx + trunc_i32(radius as f64 * s),
        cy - trunc_i32(radius as f64 * c),
    );
    let hb = triangle_half_base as f64;
    [
        tip,
        (base.0 + trunc_i32(hb * c), base.1 + trunc_i32(hb * s)),
        (base.0 - trunc_i32(hb * c), base.1 - trunc_i32(hb * s)),
    ]
}

/// 北三角角 (CompassGauge.java:117-123): 离体恒 0 (北固定 12 点钟),
/// 随体 = -compassRads (float 取负后提升 double)
fn north_angle(inertial_mode: bool, compass_rads: f32) -> f64 {
    if inertial_mode {
        -(compass_rads as f64)
    } else {
        0.0
    }
}

/// 文本标签基线位置 (CompassGauge.java:167-171):
/// compass = (x+lw+3, y+HUDFontSize - (r-HUDFontSize)/2), loc = (x+lw+3, y+r+small/2+big)。
/// PORT: (r - HUDFontSize)/2 与 HUDFontSizeSmall/2 均为 int 除法 (向零截断);
/// 各 int 加减链极端参数下 Java 静默回绕 (§2.2), 真实布局幅度不可达, 备查
fn label_positions(
    x: i32,
    y: i32,
    r: i32,
    line_width: i32,
    hud_font_size: i32,
    hud_font_size_small: i32,
) -> ((i32, i32), (i32, i32)) {
    (
        (x + line_width + 3, y + hud_font_size - (r - hud_font_size) / 2),
        (
            x + line_width + 3,
            y + r + hud_font_size_small / 2 + hud_font_size,
        ),
    )
}

/// drawStringShade 双遍文本 (UIBaseElements.java:57-59 → __drawStringShade
/// drawFontShape=false 分支, Application.java:143): 影 (x+1,y+1) shade → 本色 (x,y)
#[allow(clippy::too_many_arguments)] // 对齐 Java drawStringShade(g2d,x,y,shadeWidth,s,f)+显式双色
fn draw_string_shade(
    cv: &mut PixCanvas,
    font: &LoadedFont,
    x: i32,
    y: i32,
    s: &str,
    c: [u8; 4],
    shade: [u8; 4],
    aa: bool,
) {
    cv.draw_text(font, x + 1, y + 1, s, shade, aa);
    cv.draw_text(font, x, y, s, c, aa);
}

/// drawPolygon 1px 描边 (CompassGauge.java:219-222, THIN_STROKE = BasicStroke(1),
/// 默认 CAP_SQUARE+JOIN_MITER)。PixCanvas 无任意折线 stroke 入口, 用闭合 3 边的
/// Square 帽线段近似 miter 转角 — 1px 线宽下转角差异 ≤~0.7px, 在 C 类对拍容差内
/// (render2d 头注的 Java 非 AA 光栅 ~0.5-1px 系统差同级)
fn stroke_polygon_thin(cv: &mut PixCanvas, pts: &[(i32, i32); 3], color: [u8; 4], aa: bool) {
    for i in 0..3 {
        let j = (i + 1) % 3;
        cv.draw_line_cap(
            pts[i].0, pts[i].1, pts[j].0, pts[j].1, 1.0, color, LineCapStyle::Square, aa,
        );
    }
}

/// 罗盘 gauge (CompassGauge.java:14)。
/// Java 端无脏检查 (每帧直绘), Rust 侧按 vm-overlay 组件规范补 last_value 脏检查:
/// update/set_* 变化置脏, draw 清脏。
/// 脏检查域契约 (与 sibling 组件同模式): 仅覆盖 update/set_* 触达的状态;
/// draw 的 x/y/font_small/aa 入参与 pub visible 不置脏 — 按 is_dirty() 门控 draw
/// 的组装层必须在位置/字体/AA/可见性变化时无条件重绘, 否则漏帧。
pub struct CompassGauge {
    /// AbstractHUDComponent.visible (布局引擎消费, draw 本身不检查 — Java 同)
    pub visible: bool,
    radius: i32,
    // 风格上下文 (Java:22-24 setStyleContext; int 默认 0 = 未注入前的 Java 隐式初值)
    line_width: i32,
    hud_font_size: i32,
    hud_font_size_small: i32,
    // 状态 (Java:28-32)
    heading: f64,
    compass_rads: f32,
    compass_dx: i32,
    compass_dy: i32,
    line_compass: String,
    line_loc: String,
    /// 坐标系模式 (Java:37 默认 false = 离体)
    inertial_mode: bool,
    dirty: bool,
}

impl CompassGauge {
    /// Java:46-50 构造
    pub fn new(radius: i32) -> Self {
        CompassGauge {
            visible: true,
            radius,
            line_width: 0,
            hud_font_size: 0,
            hud_font_size_small: 0,
            heading: 0.0,
            compass_rads: 0.0,
            compass_dx: 0,
            compass_dy: 0,
            line_compass: String::new(),
            line_loc: String::new(),
            inertial_mode: false,
            dirty: true,
        }
    }

    /// HUDComponent.getId (Java:53)
    pub fn id(&self) -> &'static str {
        "gauge.compass"
    }

    /// getPreferredSize (Java:58-60): 半径 ×2 圆, 无额外留白。
    /// PORT: radius*2 int 乘法极端半径下 Java 静默回绕 / Rust debug panic (§2.2),
    /// 真实布局幅度不可达, 备查
    pub fn preferred_size(&self) -> (i32, i32) {
        (self.radius * 2, self.radius * 2)
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Java:78-80 setInertialMode; Rust 侧变化置脏 (视觉分支切换)
    pub fn set_inertial_mode(&mut self, inertial: bool) {
        if self.inertial_mode != inertial {
            self.inertial_mode = inertial;
            self.dirty = true;
        }
    }

    pub fn inertial_mode(&self) -> bool {
        self.inertial_mode
    }

    /// Java:63-70 setStyleContext。Rust 侧变化置脏。
    /// PORT: Java 此处不重算 compassDx/Dy — 半径变化后到下一次 onDataUpdate 前,
    /// 指针末端仍用旧半径的偏移 (tip 用新 r), 该一帧错位是 Java 原生行为, 保真保留
    pub fn set_style_context(
        &mut self,
        radius: i32,
        line_width: i32,
        hud_font_size: i32,
        hud_font_size_small: i32,
    ) {
        let changed = self.radius != radius
            || self.line_width != line_width
            || self.hud_font_size != hud_font_size
            || self.hud_font_size_small != hud_font_size_small;
        self.radius = radius;
        self.line_width = line_width;
        self.hud_font_size = hud_font_size;
        self.hud_font_size_small = hud_font_size_small;
        self.dirty |= changed;
    }

    /// Java:83-99 onDataUpdate(HUDData) 的两输入 (heading/mapGrid)。
    /// 返回是否变化 (脏检查); 派生量即时重算 (无平滑, 见模块头注)
    pub fn update(&mut self, heading: f64, map_grid: &str) -> bool {
        let changed = self.heading != heading || self.line_loc != map_grid;
        self.heading = heading;
        self.compass_rads = java_to_radians_f32(heading);
        let (dx, dy) = compass_dxy(self.radius, self.compass_rads);
        self.compass_dx = dx;
        self.compass_dy = dy;
        self.line_compass = fmt_heading3(heading);
        self.line_loc.clear();
        self.line_loc.push_str(map_grid);
        self.dirty |= changed;
        changed
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn line_compass(&self) -> &str {
        &self.line_compass
    }

    pub fn line_loc(&self) -> &str {
        &self.line_loc
    }

    /// 北三角 (Java:117-123 图层 1 + 185-223 绘制): colorUnit 填充 + colorShadeShape
    /// 1px 描边; tip 朝外, 底边坐在圆周上 (圆环图层随后压住底边与描边一部分)
    fn draw_north_triangle(
        cv: &mut PixCanvas,
        cx: i32,
        cy: i32,
        r: i32,
        angle_rads: f64,
        aa: bool,
    ) {
        let pts = north_triangle(cx, cy, r, angle_rads);
        let mut fp = [(0f32, 0f32); 3];
        for i in 0..3 {
            fp[i] = (pts[i].0 as f32, pts[i].1 as f32);
        }
        // PORT: Java:216-217 fillPolygon (偶奇填充) colorUnit
        cv.fill_path(&fp, COLOR_UNIT, aa);
        // PORT: Java:219-222 drawPolygon THIN_STROKE colorShadeShape
        stroke_polygon_thin(cv, &pts, COLOR_SHADE_SHAPE, aa);
    }

    /// Java:102-173 draw。font_small=None 跳过文本 (Java fontSmall==null 同)。
    /// aa 对齐 graphAASetting (生产恒 ON, MiniHUDOverlay.paintComponent L244)
    pub fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, font_small: Option<&LoadedFont>, aa: bool) {
        let r = self.radius;
        let center_x = x + r;
        let center_y = y + r;
        // Java:105-109 的 BasicStroke 缓存 (cachedStrokeWidth) 是零 GC 优化, 无视觉
        // 影响, 不移植 — 线宽每帧由参数直取
        let out_w = (self.line_width + 2) as f32;
        let in_w = self.line_width as f32;

        // 图层 1 (底): 北三角先画 (Java:115-123)
        Self::draw_north_triangle(cv, center_x, center_y, r, north_angle(self.inertial_mode, self.compass_rads), aa);

        // 图层 2: 罗盘圆双层描边 (Java:125-132, drawOval(x,y,2r,2r) 圆心 (x+r,y+r))
        cv.stroke_circle(center_x, center_y, r, out_w, COLOR_SHADE_SHAPE, aa);
        cv.stroke_circle(center_x, center_y, r, in_w, COLOR_NUM, aa);

        // 图层 3 (顶): 航向指针 (离体) / 固定机标线 (随体), 均 shade 粗线 + num 细线 (Java:135-163)
        if self.inertial_mode {
            let ((tx, ty), (ex, ey)) = fixed_segment(center_x, center_y, r);
            cv.draw_line(tx, ty, ex, ey, out_w, COLOR_SHADE_SHAPE, aa);
            cv.draw_line(tx, ty, ex, ey, in_w, COLOR_NUM, aa);
        } else {
            let (tx, ty) = pointer_tip(center_x, center_y, r, self.compass_rads);
            let ex = center_x + self.compass_dx;
            let ey = center_y - self.compass_dy;
            cv.draw_line(tx, ty, ex, ey, out_w, COLOR_SHADE_SHAPE, aa);
            cv.draw_line(tx, ty, ex, ey, in_w, COLOR_NUM, aa);
        }

        // 图层 4: 文本标签 (两模式同位, Java:165-172)
        if let Some(f) = font_small {
            let ((cpx, cpy), (lpx, lpy)) = label_positions(
                x,
                y,
                r,
                self.line_width,
                self.hud_font_size,
                self.hud_font_size_small,
            );
            draw_string_shade(cv, f, cpx, cpy, &self.line_compass, COLOR_NUM, COLOR_SHADE_SHAPE, aa);
            draw_string_shade(cv, f, lpx, lpy, &self.line_loc, COLOR_NUM, COLOR_SHADE_SHAPE, aa);
        }
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 北三角基数象限几何 (CompassGauge.java:185-213):
    /// r=25 → height=(int)8.75=8, halfBase=(int)7.499…=7, tipDist=33;
    /// 角 0: tip 正上 (60,27), 底边坐圆顶 (60,35) 切向展开 ±7;
    /// 角 π/2: tip 正右 (93,60), 底边 (85,60) 竖直展开 ±7
    #[test]
    fn north_triangle_geometry_cardinal() {
        let pts = north_triangle(60, 60, 25, 0.0);
        assert_eq!(pts[0], (60, 27), "tip 在圆外 33px 处 (北)");
        assert_eq!(pts[1], (67, 35), "corner1 切向 +7 (圆顶)");
        assert_eq!(pts[2], (53, 35), "corner2 切向 -7");

        let pts = north_triangle(60, 60, 25, std::f64::consts::FRAC_PI_2);
        assert_eq!(pts[0], (93, 60), "tip 正右 (东)");
        assert_eq!(pts[1], (85, 67), "corner1 竖直向下");
        assert_eq!(pts[2], (85, 53), "corner2 竖直向上");
        // cos(π/2)=6.12e-17 → (int)(33·6.12e-17)=0, 三角形纯轴向
        assert_eq!(pts[0].1, 60);
    }

    /// 三角尺寸的 (int) 截断 (Java:187-188): f64 字面量乘积的舍入必须逐值核对 —
    /// r=20: 20×0.35 的精确积恰在 7.0 半 ulp 平局点, IEEE 取偶舍到 7.0 (Java 同)
    /// → height=7; 20×0.30 舍到 6.0 → halfBase=6; tipDist=27。
    /// r=25: 25×0.35 → 8.749…→8, 25×0.30 → 7.499…→7
    #[test]
    fn north_triangle_size_truncation() {
        let pts = north_triangle(0, 0, 20, 0.0);
        assert_eq!(pts[0], (0, -27), "r=20 tipDist = 20+7 = 27 (height 舍入为 7)");
        assert_eq!(pts[1], (6, -20), "halfBase 6");
        assert_eq!(pts[2], (-6, -20));

        let pts = north_triangle(0, 0, 25, 0.0);
        assert_eq!(pts[0], (0, -33), "r=25 tipDist=33 (height 8)");
        assert_eq!(pts[1], (7, -25), "halfBase 7");
    }

    /// Java (int) 向零截断 (非 floor): 微小负 sin 分量截为 0 而非 -1
    /// (tip.x 与 corner 的切向 y 分量均含 (int)负微小量 = 0)
    #[test]
    fn int_cast_truncates_toward_zero() {
        let pts = north_triangle(100, 100, 20, -1e-9);
        assert_eq!(pts[0].0, 100, "tip.x: (int)(27·sin(-1e-9)) = (int)(-2.7e-8) = 0");
        assert_eq!(pts[0].1, 73, "tip.y: 100 - 27");
        assert_eq!(pts[1].1, 80, "corner1.y: baseY + (int)(6·sin(-1e-9)) = 80 + 0");
        assert_eq!(pts[2].0, 94, "corner2.x: base - 切向半宽 6");
    }

    /// 指针几何 (update 派生 + Java:151-154), r=25:
    /// 0°: tip (60,45)/end (60,28); 90°: tip (75,60)/end (92,60); 180°: tip (60,75)/end (60,92)。
    /// 90°/180° 验证 f32 化 compassRads 后 sin 误差 (≤1e-7) 不翻越整型边界,
    /// 以及 (1.3f·25)=32.499…→32 / (0.618·25)=15.45→15 / 微小量向零截 0
    #[test]
    fn update_pointer_geometry_cardinals() {
        let mut g = CompassGauge::new(25);
        g.set_style_context(25, 3, 24, 12);

        g.update(0.0, "C4");
        assert_eq!((g.compass_dx, g.compass_dy), (0, 32));
        assert_eq!(pointer_tip(60, 60, 25, g.compass_rads), (60, 45));
        assert_eq!((60 + g.compass_dx, 60 - g.compass_dy), (60, 28), "指针朝北");

        g.update(90.0, "C4");
        assert_eq!((g.compass_dx, g.compass_dy), (32, 0), "sin(1.5707964f32)≈1 → 32, cos≈4.4e-8 → 0");
        assert_eq!(pointer_tip(60, 60, 25, g.compass_rads), (75, 60), "指针朝东");

        g.update(180.0, "C4");
        assert_eq!((g.compass_dx, g.compass_dy), (0, -32), "sin(πf32)=-8.7e-8 → 0, cos→-32");
        assert_eq!(pointer_tip(60, 60, 25, g.compass_rads), (60, 75), "指针朝南");
        assert_eq!((60 + g.compass_dx, 60 - g.compass_dy), (60, 92));
    }

    /// 随体模式固定机标线 (Java:138-139): (int)(0.618·25)=15, (int)(1.3·25)=32
    /// (此处 1.3 是 double 字面量, 32.5 截 32)
    #[test]
    fn fixed_segment_geometry() {
        let ((tx, ty), (ex, ey)) = fixed_segment(60, 60, 25);
        assert_eq!((tx, ty), (60, 45), "tip = cy - 15");
        assert_eq!((ex, ey), (60, 28), "end = cy - 32");
        // 长度与离体 0° 指针一致 (15→32 同区间)
        let mut g = CompassGauge::new(25);
        g.update(0.0, "");
        assert_eq!((tx, ty), pointer_tip(60, 60, 25, g.compass_rads));
    }

    /// 文本基线位置 (Java:167-171) 含 int 除法向零截断:
    /// r=25, big=24 → (r-big)/2 = 0; r=37 → 13/2 = 6
    #[test]
    fn label_positions_int_division() {
        let (compass, loc) = label_positions(30, 10, 25, 2, 24, 12);
        assert_eq!(compass, (35, 34), "y = 10+24-(25-24)/2 = 34");
        assert_eq!(loc, (35, 65), "y = 10+25+12/2+24 = 65");

        let (compass, _) = label_positions(30, 10, 37, 2, 24, 12);
        assert_eq!(compass, (35, 28), "(37-24)/2 = 6 (向零截断)");
    }

    /// %3.0f 航向格式 (Java:95): HALF_UP / 宽 3 右对齐 / 负零保号 / NaN
    #[test]
    fn fmt_heading3_rounding() {
        assert_eq!(fmt_heading3(5.0), "  5");
        assert_eq!(fmt_heading3(359.6), "360", "HALF_UP 进位自然超宽");
        assert_eq!(fmt_heading3(0.5), "  1");
        assert_eq!(fmt_heading3(-0.4), " -0", "负值舍到零保负号");
        assert_eq!(fmt_heading3(-0.0), " -0");
        assert_eq!(fmt_heading3(f64::NAN), "NaN");
        assert_eq!(fmt_heading3(0.49999999999999994), "  0", "精确十进制舍入");
    }

    /// %3.0f 非有限与超 i64 域值 (畸形遥测 org.json "1e999"→inf / "1e19" 路径):
    /// Formatter 输出 "Infinity"/"-Infinity"/完整十进制, 不得出现 as i64 饱和串。
    /// 1e19/2^63 均为整值 double, 精确十进制展开无舍入分歧
    #[test]
    fn fmt_heading3_infinite_and_huge() {
        assert_eq!(fmt_heading3(f64::INFINITY), "Infinity");
        assert_eq!(fmt_heading3(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(fmt_heading3(1e19), "10000000000000000000");
        assert_eq!(fmt_heading3(-1e19), "-10000000000000000000");
        assert_eq!(fmt_heading3(9_223_372_036_854_775_808.0), "9223372036854775808");
    }

    /// 双模式语义 (Java:117-123 / 34-37): 离体北三角角恒 0, 随体 = -compassRads
    #[test]
    fn mode_semantics_north_angle() {
        let mut g = CompassGauge::new(25);
        g.update(90.0, "");
        assert_eq!(north_angle(false, g.compass_rads), 0.0, "离体: 北固定 12 点钟");
        assert!(
            (north_angle(true, g.compass_rads) + g.compass_rads as f64).abs() < 1e-12,
            "随体: 北三角转 -compassRads"
        );
        assert!(!g.inertial_mode(), "默认离体 (Java:37)");
        g.set_inertial_mode(true);
        assert!(g.inertial_mode());
        assert!(g.is_dirty(), "模式切换置脏");
        assert_eq!(g.id(), "gauge.compass");
        assert_eq!(g.preferred_size(), (50, 50), "preferred = 2r×2r (Java:58-60)");
    }

    /// NaN 航向 (地图方向无效时 0/0 → NaN): Java (int)NaN=0 → dx/dy 归 0,
    /// 指针退化为 tip(0.618r 方向亦 NaN→0)=(cx,cy) 到 (cx,cy) 的零长度线;
    /// 文本 "NaN"
    #[test]
    fn nan_heading_semantics() {
        let mut g = CompassGauge::new(25);
        assert!(g.update(f64::NAN, ""));
        assert_eq!((g.compass_dx, g.compass_dy), (0, 0), "(int)NaN = 0");
        assert!(g.compass_rads.is_nan());
        assert_eq!(g.line_compass(), "NaN");
        assert_eq!(pointer_tip(60, 60, 25, g.compass_rads), (60, 60));
    }

    /// 脏检查: 同值不脏, 变化置脏, draw 清脏; set_style_context 同值不置脏
    #[test]
    fn dirty_checking_semantics() {
        let mut g = CompassGauge::new(25);
        assert!(g.update(123.4, "B2"));
        assert!(!g.update(123.4, "B2"), "同航向同网格不脏");
        assert!(g.is_dirty());
        let mut cv = PixCanvas::new(120, 120).unwrap();
        g.draw(&mut cv, 10, 10, None, true);
        assert!(!g.is_dirty(), "draw 后清脏");
        g.update(124.0, "B2");
        assert!(g.is_dirty(), "航向变化置脏");
        g.draw(&mut cv, 10, 10, None, true);
        g.set_style_context(25, 3, 24, 12);
        assert!(g.is_dirty(), "风格变化置脏");
        g.draw(&mut cv, 10, 10, None, true);
        g.set_style_context(25, 3, 24, 12);
        assert!(!g.is_dirty(), "同值风格不置脏");
    }

    /// 渲染冒烟: 图层序像素采样 (heading=180 → 指针在下半, 不污染北三角采样)。
    /// 预乘存储期望: colorUnit [166·220/255≈143]³;
    /// 圆环/指针处 shade 底层在下 (双层 stroke 同心), alpha = SrcOver(240,42)≈242
    #[test]
    fn render_smoke_layer_order() {
        /// Java2D SrcOver 直通域合成后的 alpha (同 gauges_bars 测试式)
        fn src_over_a(fg: u8, bg: u8) -> u8 {
            let fa = fg as f32 / 255.0;
            let fda = bg as f32 / 255.0;
            ((fa + fda * (1.0 - fa)) * 255.0 + 0.5) as u8
        }
        let mut g = CompassGauge::new(25);
        g.set_style_context(25, 3, 24, 12);
        g.update(180.0, "C4");
        let mut cv = PixCanvas::new(120, 120).unwrap();
        g.draw(&mut cv, 10, 10, None, false);
        // 圆心 (35,35): 北三角内部 (35,6) — 距心 28.5 > 外环外缘 27.5, 纯 colorUnit 填充
        let d = |x: i32, y: i32| {
            let i = ((y * cv.width() + x) * 4) as usize;
            cv.pixmap().data()[i..i + 4].to_vec()
        };
        let tri = d(35, 6);
        for (got, want) in tri.iter().zip([143u8, 143, 143, 220]) {
            assert!(
                (i32::from(*got) - i32::from(want)).abs() <= 2,
                "北三角填充 ≈{:?} (期望 ~{want})",
                tri
            );
        }
        // 圆环右点 (60,35): 距心 25.5 ∈ num 环 [23.5,26.5] 内部, shade 外环垫底
        let ring_alpha = src_over_a(240, 42);
        let ring = d(60, 35);
        for (got, want) in ring.iter().zip([25u8, 240, 120, ring_alpha]) {
            assert!(
                (i32::from(*got) - i32::from(want)).abs() <= 2,
                "圆环 num 层 ≈{:?} (期望 ~{want}, num 叠 shade)",
                ring
            );
        }
        // 指针列 (35,55): 180° 指针 (35,50)-(35,67), shade 宽 5 垫底 + num 宽 3 在上
        let ptr = d(35, 55);
        for (got, want) in ptr.iter().zip([25u8, 240, 120, ring_alpha]) {
            assert!(
                (i32::from(*got) - i32::from(want)).abs() <= 2,
                "指针 num 层 ≈{:?} (期望 ~{want}, num 叠 shade)",
                ptr
            );
        }
        // 图层序: (35,9) 在北三角内部 (距心 25.5, 同时被圆环双层覆盖) —
        // 绿色 num 环压在灰色三角上 → g 通道远大于 r 通道, 且 alpha 高于纯三角 220
        let over = d(35, 9);
        assert!(
            i32::from(over[1]) - i32::from(over[0]) > 100,
            "圆环 (绿) 盖住三角 (灰): {:?}",
            over
        );
        assert!(over[3] > 220, "叠层后 alpha 高于纯三角 220: {:?}", over);
    }
}
