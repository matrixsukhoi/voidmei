//! 公共像素基元 (重构波3): 多组件复制的 Java Graphics2D 语义基元收敛单源。
//! 原分散于 gauges_bars/rows/minihud/overlay_gauges/renderers/gauge_compass/
//! overlay_control_surfaces 的逐字级副本 (各副本 oracle 对拍等价) 统一于此;
//! 组件特有基元 (butt_line/hline_butt2/vline_square2 等) 仍留各自模块。
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
