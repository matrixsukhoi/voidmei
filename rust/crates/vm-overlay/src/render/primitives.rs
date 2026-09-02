//! 公共像素基元 (重构波3 建, 波13 收割扩容): 多组件复制/同族的 Java Graphics2D
//! 语义基元收敛单源。原分散于 gauges_bars/rows/minihud/overlay_gauges/renderers/
//! gauge_compass/overlay_control_surfaces 的逐字级副本 (各副本 oracle 对拍等价)
//! 统一于此; 波13 再收 draw_h_rect (rows/gear_flaps 双副本)、butt_line 族
//! (原 gauges_bars::hline_butt2 为其 w=2 水平特化, 逐像素等价已并入)、
//! vline_square2 与旋转 stroke 精确轮廓族 (attitude)。
//! Java 取整族 (java_round_*) 收敛在 vm_core::base::format。

use crate::render::font::LoadedFont;
use crate::render::palette::colors;
use crate::render::canvas::PixCanvas;

/// Java Graphics.drawRect(x,y,w,h) + BasicStroke(1): 覆盖 x..x+w × y..y+h
/// (含端点) 的 1px 环。负宽或负高整体不绘制 (Java 8 oracle 实测 0 像素 —
/// "负尺寸时 4 条 drawLine 反向仍可见"的假设已证伪); 零宽/零高退化 1px 线
/// (drawRect 的 4 条边线中零长度段无输出, 剩两段共线: oracle drawRect(50,10,0,20)
/// = 列 50 行 10..30 的 1px 竖线; 双零则 4 段全零长度, 无输出)。
pub(crate) fn ring1px(cv: &mut PixCanvas, x: i32, y: i32, w: i32, h: i32, color: [u8; 4]) {
    if w < 0 || h < 0 {
        return; // PORT: Java drawRect 负宽/负高不绘制 (镜像归一化假设已证伪)
    }
    if w == 0 || h == 0 {
        if w == 0 && h > 0 {
            cv.fill_rect(x, y, 1, h + 1, color); // 零宽退化竖线 行 y..y+h
        } else if h == 0 && w > 0 {
            cv.fill_rect(x, y, w + 1, 1, color); // 零高退化横线 列 x..x+w
        }
        return; // 双零无输出
    }
    cv.fill_rect(x, y, w + 1, 1, color); // 上边
    cv.fill_rect(x, y + h, w + 1, 1, color); // 下边
    if h > 1 {
        cv.fill_rect(x, y + 1, 1, h - 1, color); // 左边
        cv.fill_rect(x + w, y + 1, 1, h - 1, color); // 右边
    }
}

/// BasicStroke(1, CAP_ROUND, JOIN_ROUND) 竖线: 列恰为 x, 行 y0..y1 端点含
/// (宽 1 圆帽半径 0.5 不外伸 — Java drawLine 整数端点像素级语义)。
/// aa 分支与 false 输出一致 (1px stroke 规整后无柔边, 见 gauges_bars 文档),
/// 故无 aa 参。
pub(crate) fn vline_1px(cv: &mut PixCanvas, x: i32, y0: i32, y1: i32, color: [u8; 4]) {
    let (ya, yb) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    cv.fill_rect(x, ya, 1, yb - ya + 1, color);
}

/// UIBaseElements.drawHRect (UIBaseElements.java:97-112): shade 1px 外框环 +
/// 内缩 borderwidth 填充条。width<0 时框/条翻转到起点右侧 (Java 原样分支)。
/// borderwidth 调用点恒 1 (HUDAkbRow.java:91 / drawVBarTextNum), 参数保留
/// 对齐 Java 签名。(重构波13: rows/gear_flaps 双副本合一)
#[allow(clippy::too_many_arguments)] // 对齐 Java drawHRect(g2d,x,y,width,height,borderwidth,c)
pub(crate) fn draw_h_rect(
    cv: &mut PixCanvas,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    borderwidth: i32,
    c: [u8; 4],
) {
    if width >= 0 {
        // PORT: UIBaseElements.java:102-105 drawRect(x,y,width-1,height-1) 环 +
        // fillRect(x+bw, y+bw, width-2*bw, height-2*bw) 内芯
        ring1px(cv, x, y, width - 1, height - 1, colors().shade_shape);
        cv.fill_rect(
            x + borderwidth,
            y + borderwidth,
            width - 2 * borderwidth,
            height - 2 * borderwidth,
            c,
        );
    } else {
        // PORT: UIBaseElements.java:106-109 负宽分支: 环自 x+width 起, 填充同步翻转
        ring1px(cv, x + width, y, -width - 1, height - 1, colors().shade_shape);
        cv.fill_rect(
            x + borderwidth + width,
            y + borderwidth,
            -width - 2 * borderwidth,
            height - 2 * borderwidth,
            c,
        );
    }
}

/// BasicStroke(w, CAP_BUTT, JOIN_MITER) 轴对齐线 (GraphicsUtil.createPreciseStroke,
/// MarkedGauge 的 tickStroke/borderStroke 族)。Java 调用点全部轴对齐 (竖/横)。
/// aa=false (ANTIALIAS_OFF, 中心规则): 像素中心落在覆盖盒 [xa,xb]×[y±w/2] 内才
/// 点亮 (w=2 水平线 = 行 y-1..y × 列 xa..xb-1, 右端列不点亮; 1px 默认 stroke 的
/// Bresenham 端点含像素不适用于宽>1 的 strokedShape 路径, oracle:
/// drawLine(5,15,25,15) 覆盖列 5..24 共 20 列)。
/// aa=true (生产 graphAA 恒 ON): STROKE_NORMALIZE 规整到像素中心后按分离覆盖
/// 模型 (cov_x × cov_y) 缩放 alpha (w=2 = 行 y 全值/行 y±1 半值/端点列半覆盖/
/// 四角 1/4, oracle: 21 列×3 行=63 非零像素, 角点 a=64)。
/// (重构波13: 原 gauges_bars::hline_butt2 为本形态的 w=2 水平特化, 逐像素等价
/// 已并入, 调用方直接传 w=2)
#[allow(clippy::too_many_arguments)] // 对齐 Java drawLine(x0,y0,x1,y1)+线宽/色/AA 三元组
pub(crate) fn butt_line(
    cv: &mut PixCanvas,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    w: i32,
    color: [u8; 4],
    aa: bool,
) {
    // PORT: Java BasicStroke(0)=hairline 1px; w<=0 钳到 1 (render2d::stroke_of 同款)
    let w = if w <= 0 { 1 } else { w };
    let half = w as f32 / 2.0;
    let vert = x0 == x1;
    // 沿线方向 (u) 与横截方向 (v) 的整数端点
    let (ua, ub, v) = if vert {
        let (ya, yb) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        (ya, yb, x0)
    } else {
        let (xa, xb) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        (xa, xb, y0)
    };
    if ub <= ua {
        return; // PORT: 零长度 CAP_BUTT 线不绘制 (Java strokedShape 零长度段无输出)
    }
    let put = |cv: &mut PixCanvas, u: i32, vpos: i32, c: [u8; 4]| {
        if vert {
            cv.fill_rect(vpos, u, 1, 1, c);
        } else {
            cv.fill_rect(u, vpos, 1, 1, c);
        }
    };
    if !aa {
        // 中心规则: 沿线像素中心 u+0.5 ∈ [ua, ub] → u ∈ [ua, ub-1];
        // 横截像素中心 v+0.5 ∈ [v-half, v+half] → 逐行判定 (整数边界用含等号)
        let u_hi = ub - 1;
        for u in ua..=u_hi {
            for vv in (v - w - 1)..=(v + w + 1) {
                let c = vv as f32 + 0.5;
                if c >= v as f32 - half && c <= v as f32 + half {
                    put(cv, u, vv, color);
                }
            }
        }
        return;
    }
    // AA: 规整后覆盖盒 沿线 [ua+0.5, ub+0.5] × 横截 [v-half+0.5, v+half+0.5]
    let (alo, ahi) = (ua as f32 + 0.5, ub as f32 + 0.5);
    let (clo, chi) = (v as f32 - half + 0.5, v as f32 + half + 0.5);
    for u in (ua - 1)..=(ub + 1) {
        let cu = coverage(u, alo, ahi);
        if cu <= 0.0 {
            continue;
        }
        for vv in (v - w - 1)..=(v + w + 1) {
            let cvv = coverage(vv, clo, chi);
            if cvv <= 0.0 {
                continue;
            }
            put(cv, u, vv, cov_color(color, cu * cvv));
        }
    }
}

/// 裸 BasicStroke(2) (默认 CAP_SQUARE) 竖线 (FlapAngleBar.java:110-125 tick)。
/// aa=false (中心规则): 覆盖盒 [tx-1,tx+1]×[ya-1,yb+1] → 列 tx-1..tx, 行
/// ya-1..yb (方帽两端各外伸 1px 的几何经中心规则光栅后上端外伸、下端不外伸,
/// oracle: drawLine(40,5,40,25) 覆盖列 39..40、行 4..25 共 22 行)。
/// aa=true: 覆盖盒 [tx-0.5,tx+1.5]×[ya-0.5,yb+1.5] → 3 列柔边: 列 tx 全值,
/// 列 tx±1 半值, 端行 ya-1/yb+1 半覆盖, 角点 1/4 (oracle: 行 4..26 端行半透明)。
/// (方帽 ≠ CAP_BUTT 的 [`butt_line`], 独立基元; 重构波13 自 gauges_bars 迁入)
pub(crate) fn vline_square2(cv: &mut PixCanvas, tx: i32, y0: i32, y1: i32, color: [u8; 4], aa: bool) {
    let (ya, yb) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    if !aa {
        cv.fill_rect(tx - 1, ya - 1, 2, yb - ya + 2, color);
        return;
    }
    let body = yb - ya + 1;
    let soft = cov_color(color, 0.5);
    let corner = cov_color(color, 0.25);
    if body > 0 {
        cv.fill_rect(tx, ya, 1, body, color);
        cv.fill_rect(tx - 1, ya, 1, body, soft);
        cv.fill_rect(tx + 1, ya, 1, body, soft);
    }
    for ry in [ya - 1, yb + 1] {
        cv.fill_rect(tx, ry, 1, 1, soft);
        cv.fill_rect(tx - 1, ry, 1, 1, corner);
        cv.fill_rect(tx + 1, ry, 1, 1, corner);
    }
}

/// 阴影双遍文本 — 显式双色版 (LinearGauge/TextGauge.drawTextShaded,
/// UIBaseElements.__drawStringShade drawFontShape=false 分支):
/// 影 (x+1,y+1) shade → 本色 (x,y)
#[allow(clippy::too_many_arguments)] // 对齐 Java drawTextShaded(g2d,x,y,s,f,c) + 显式 shade/aa
pub(crate) fn text_shaded(
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

/// 阴影双遍文本 — 全局 shade 版 (__drawStringShade 的 char[] fallback 形态):
/// 影色恒取 global_colors 的 colorShadeShape (UIBaseElements.java:46-55;
/// shadeWidth 只作用于 setStroke 对文本无效果, 不复刻)
pub(crate) fn text_shaded_auto(
    cv: &mut PixCanvas,
    font: &LoadedFont,
    x: i32,
    y: i32,
    s: &str,
    c: [u8; 4],
    aa: bool,
) {
    cv.draw_text(font, x + 1, y + 1, s, colors().shade_shape, aa);
    cv.draw_text(font, x, y, s, c, aa);
}

/// AA 柔边像素的覆盖率缩放: Java AA 管线合成式 = SrcOver(源 alpha × 覆盖率),
/// 不透明色 oracle 值 cov=0.5 → a=128、cov=0.25 → a=64, 即 round(a·cov)
pub(crate) fn cov_color(color: [u8; 4], cov: f32) -> [u8; 4] {
    [color[0], color[1], color[2], ((color[3] as f32) * cov + 0.5) as u8]
}

/// 像素区间 [p, p+1) 与覆盖盒 [lo, hi] 的重叠覆盖率
pub(crate) fn coverage(p: i32, lo: f32, hi: f32) -> f32 {
    let a = (p as f32).max(lo);
    let b = ((p + 1) as f32).min(hi);
    (b - a).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// 旋转 stroke 精确轮廓族 (重构波13 自 attitude 迁入; D7 矢量基元走 tiny-skia)
// ---------------------------------------------------------------------------

/// 圆弧 stroke 的精确几何区域轮廓折线: 圆弧中心线 (cx,cy,r, a1→a1+sweep) ⊖ 半径
/// half=w/2 圆盘 = 外弧 (r+half) → 末端圆帽 (CAP_ROUND) → 内弧 (r−half) → 始端圆帽,
/// 单闭合折线。fill 一次完成 → 半透明色单次 SrcOver 合成, 无分段叠加伪影
/// (Java drawArc + BasicStroke(CAP_ROUND) 的 stroke 区域即此 Minkowski 和)。
/// 角度约定同 render2d::stroke_arc: point(φ) = (cx + r·cosφ, cy − r·sinφ),
/// 正 sweep = 视觉逆时针; 负 sweep 归一化为正 (区域与参数方向无关)。
pub(crate) fn arc_stroke_outline(
    cx: f32,
    cy: f32,
    r: f32,
    a1: f32,
    sweep: f32,
    w: f32,
) -> Vec<(f32, f32)> {
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
pub(crate) fn line_stroke_outline(x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Vec<(f32, f32)> {
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
