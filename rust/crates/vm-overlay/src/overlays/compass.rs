//! gauge_compass: CompassGauge C 类语义复刻
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
//! 坐标系双模式 (逻辑反转以对齐 AttitudeIndicatorGauge):
//! - inertial_mode=false (配置 OFF, 默认) = 离体: 指针随航向旋转, 北三角固定 12 点钟;
//! - inertial_mode=true  (配置 ON)        = 随体: 机标线固定 12 点钟, 北三角转 -compassRads。
//!
//! CompassDelta 平滑行为: Java 链路 Service.updateCompass (仪表罗盘优先/
//! 地图方向回退) → HUDData.heading → onDataUpdate, **无任何插值/
//! 平滑/低通**——每轮 ~10Hz 轮询值直接驱动指针。Rust 侧同样直通 (update 即时重算);
//! -65535 哨兵回退分支属数据层, 已落 vm-data::service_loop (update_compass), 不在本文件。

use crate::render::font::LoadedFont;
use crate::render::palette::colors;
use crate::render::primitives;

use crate::render::canvas::{LineCapStyle, PixCanvas};
use vm_core::base::format;

/// Java (int) double 强转语义: 向零截断, NaN→0, ±∞ 饱和到 MIN/MAX —
/// 与 Rust `as i32` (饱和转换) 逐例一致
#[inline]
fn trunc_i32(v: f64) -> i32 {
    v as i32
}

/// Java Math.toRadians(deg) + (float) 收窄:
/// OpenJDK 实现 = degrees / 180.0 * PI (f64), 存入 float 字段 compassRads
#[inline]
fn java_to_radians_f32(deg: f64) -> f32 {
    (deg / 180.0 * std::f64::consts::PI) as f32
}

/// Java String.format("%3.0f") = java_f0_exact + 宽 3
/// 右对齐组合 (±∞ 常量与 |v|≥2^63 完整十进制展开的语义注记见
/// vm_core::base::format::java_f0_exact, 重构波13 收割)
fn fmt_heading3(v: f64) -> String {
    format::pad_width(format::java_f0_exact(v), 3, false)
}

/// 指针末端偏移 compassDx/Dy。
/// (radius * 1.3f) 先做 f32 乘 (int×float 提升 float), 再提升 f64 乘 sin/cos;
/// 注意此处 1.3f 与 draw 内机标线的 double 1.3 精度不同
fn compass_dxy(radius: i32, compass_rads: f32) -> (i32, i32) {
    let outer = radius as f32 * 1.3f32;
    let rad = compass_rads as f64;
    (
        trunc_i32(outer as f64 * rad.sin()),
        trunc_i32(outer as f64 * rad.cos()),
    )
}

/// 离体模式指针内端 (0.618r 处)。0.618 是 double 字面量, 全程 f64
fn pointer_tip(cx: i32, cy: i32, r: i32, compass_rads: f32) -> (i32, i32) {
    let rad = compass_rads as f64;
    (
        cx + trunc_i32(0.618 * r as f64 * rad.sin()),
        cy - trunc_i32(0.618 * r as f64 * rad.cos()),
    )
}

/// 随体模式固定机标线两端:
/// tip = cy - (int)(0.618·r), end = cy - (int)(1.3·r) — 1.3 为 double 字面量
fn fixed_segment(cx: i32, cy: i32, r: i32) -> ((i32, i32), (i32, i32)) {
    (
        (cx, cy - trunc_i32(0.618 * r as f64)),
        (cx, cy - trunc_i32(1.3 * r as f64)),
    )
}

/// 北三角三顶点: tip 在圆外 (r+0.35r), 底边中点在圆周上,
/// 底边沿圆切向 (cosθ, sinθ) 展开半宽 0.30r。角 0 = 12 点钟 (北)。
/// (int)(radius*0.35)/(int)(radius*0.30) 的 f64 积截断值
/// 直接进入几何 (r=20 → 6/6 而非 7/6, 0.35/0.30 的 f64 表示略小于精确值);
/// radius + triangle_height 为 int 加法, 极端半径下 Java 静默回绕 / Rust debug
/// panic, 真实布局幅度不可达, 备查
fn north_triangle(cx: i32, cy: i32, radius: i32, angle_rads: f64) -> [(i32, i32); 3] {
    let triangle_height = trunc_i32(radius as f64 * 0.35);
    let triangle_half_base = trunc_i32(radius as f64 * 0.30);
    let tip_dist = (radius + triangle_height) as f64;
    let (s, c) = (angle_rads.sin(), angle_rads.cos());
    let tip = (cx + trunc_i32(tip_dist * s), cy - trunc_i32(tip_dist * c));
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

/// 北三角角: 离体恒 0 (北固定 12 点钟),
/// 随体 = -compassRads (float 取负后提升 double)
fn north_angle(inertial_mode: bool, compass_rads: f32) -> f64 {
    if inertial_mode {
        -(compass_rads as f64)
    } else {
        0.0
    }
}

/// 文本标签基线位置:
/// compass = (x+lw+3, y+HUDFontSize - (r-HUDFontSize)/2), loc = (x+lw+3, y+r+small/2+big)。
/// (r - HUDFontSize)/2 与 HUDFontSizeSmall/2 均为 int 除法 (向零截断);
/// 各 int 加减链极端参数下 Java 静默回绕, 真实布局幅度不可达, 备查
fn label_positions(
    x: i32,
    y: i32,
    r: i32,
    line_width: i32,
    hud_font_size: i32,
    hud_font_size_small: i32,
) -> ((i32, i32), (i32, i32)) {
    (
        (
            x + line_width + 3,
            y + hud_font_size - (r - hud_font_size) / 2,
        ),
        (
            x + line_width + 3,
            y + r + hud_font_size_small / 2 + hud_font_size,
        ),
    )
}

/// drawPolygon 1px 描边 (THIN_STROKE = BasicStroke(1),
/// 默认 CAP_SQUARE+JOIN_MITER)。PixCanvas 无任意折线 stroke 入口, 用闭合 3 边的
/// Square 帽线段近似 miter 转角 — 1px 线宽下转角差异 ≤~0.7px, 在 C 类对拍容差内
/// (render2d 头注的 Java 非 AA 光栅 ~0.5-1px 系统差同级)
fn stroke_polygon_thin(cv: &mut PixCanvas, pts: &[(i32, i32); 3], color: [u8; 4], aa: bool) {
    for i in 0..3 {
        let j = (i + 1) % 3;
        cv.draw_line_cap(
            pts[i].0,
            pts[i].1,
            pts[j].0,
            pts[j].1,
            1.0,
            color,
            LineCapStyle::Square,
            aa,
        );
    }
}

/// 罗盘 gauge。
/// Java 端无脏检查 (每帧直绘), Rust 侧按 vm-overlay 组件规范补 last_value 脏检查:
/// update/set_* 变化置脏, draw 清脏。
/// 脏检查域契约 (与 sibling 组件同模式): 仅覆盖 update/set_* 触达的状态;
/// draw 的 x/y/font_small/aa 入参与 pub visible 不置脏 — 按 is_dirty() 门控 draw
/// 的组装层必须在位置/字体/AA/可见性变化时无条件重绘, 否则漏帧。
pub struct CompassGauge {
    /// AbstractHUDComponent.visible (布局引擎消费, draw 本身不检查 — Java 同)
    pub visible: bool,
    radius: i32,
    // 风格上下文 (setStyleContext; int 默认 0 = 未注入前的 Java 隐式初值)
    line_width: i32,
    hud_font_size: i32,
    hud_font_size_small: i32,
    // 状态
    heading: f64,
    compass_rads: f32,
    compass_dx: i32,
    compass_dy: i32,
    line_compass: String,
    line_loc: String,
    /// 坐标系模式 (默认 false = 离体)
    inertial_mode: bool,
    dirty: bool,
}

impl CompassGauge {
    /// 构造
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

    /// HUDComponent.getId
    pub fn id(&self) -> &'static str {
        "gauge.compass"
    }

    /// getPreferredSize: 半径 ×2 圆, 无额外留白。
    /// radius*2 int 乘法极端半径下 Java 静默回绕 / Rust debug panic,
    /// 真实布局幅度不可达, 备查
    pub fn preferred_size(&self) -> (i32, i32) {
        (self.radius * 2, self.radius * 2)
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// setInertialMode; Rust 侧变化置脏 (视觉分支切换)
    pub fn set_inertial_mode(&mut self, inertial: bool) {
        if self.inertial_mode != inertial {
            self.inertial_mode = inertial;
            self.dirty = true;
        }
    }

    pub fn inertial_mode(&self) -> bool {
        self.inertial_mode
    }

    /// setStyleContext。Rust 侧变化置脏。
    /// Java 此处不重算 compassDx/Dy — 半径变化后到下一次 onDataUpdate 前,
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

    /// onDataUpdate(HUDData) 的两输入 (heading/mapGrid)。
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

    /// 北三角 (图层 1): colorUnit 填充 + colorShadeShape
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
        // fillPolygon (偶奇填充) colorUnit
        cv.fill_path(&fp, colors().unit, aa);
        // drawPolygon THIN_STROKE colorShadeShape
        stroke_polygon_thin(cv, &pts, colors().shade_shape, aa);
    }

    /// draw。font_small=None 跳过文本 (Java fontSmall==null 同)。
    /// aa 对齐 graphAASetting (生产恒 ON)
    pub fn draw(
        &mut self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        font_small: Option<&LoadedFont>,
        aa: bool,
    ) {
        let r = self.radius;
        let center_x = x + r;
        let center_y = y + r;
        // Java 的 stroke 状态残留不移植 — 线宽每帧由参数直取
        let out_w = (self.line_width + 2) as f32;
        let in_w = self.line_width as f32;

        // 图层 1 (底): 北三角先画
        Self::draw_north_triangle(
            cv,
            center_x,
            center_y,
            r,
            north_angle(self.inertial_mode, self.compass_rads),
            aa,
        );

        // 图层 2: 罗盘圆双层描边 (drawOval(x,y,2r,2r) 圆心 (x+r,y+r))
        cv.stroke_circle(center_x, center_y, r, out_w, colors().shade_shape, aa);
        cv.stroke_circle(center_x, center_y, r, in_w, colors().num, aa);

        // 图层 3 (顶): 航向指针 (离体) / 固定机标线 (随体), 均 shade 粗线 + num 细线
        if self.inertial_mode {
            let ((tx, ty), (ex, ey)) = fixed_segment(center_x, center_y, r);
            cv.draw_line(tx, ty, ex, ey, out_w, colors().shade_shape, aa);
            cv.draw_line(tx, ty, ex, ey, in_w, colors().num, aa);
        } else {
            let (tx, ty) = pointer_tip(center_x, center_y, r, self.compass_rads);
            let ex = center_x + self.compass_dx;
            let ey = center_y - self.compass_dy;
            cv.draw_line(tx, ty, ex, ey, out_w, colors().shade_shape, aa);
            cv.draw_line(tx, ty, ex, ey, in_w, colors().num, aa);
        }

        // 图层 4: 文本标签 (两模式同位)
        if let Some(f) = font_small {
            let ((cpx, cpy), (lpx, lpy)) = label_positions(
                x,
                y,
                r,
                self.line_width,
                self.hud_font_size,
                self.hud_font_size_small,
            );
            primitives::text_shaded(
                cv,
                f,
                cpx,
                cpy,
                &self.line_compass,
                colors().num,
                colors().shade_shape,
                aa,
            );
            primitives::text_shaded(
                cv,
                f,
                lpx,
                lpy,
                &self.line_loc,
                colors().num,
                colors().shade_shape,
                aa,
            );
        }
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests;
