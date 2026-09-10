//! GearFlaps 绘制原语 (UIBaseElements.drawVBar 族复刻)。
//!
//! 原黑盒 GearFlapsState 已随面板拆解退役 (flapbar/warn 两原子
//! 组件独立摆位, 见 widgets::gear_flaps_atom); 本文件承载跨组件共享的
//! 竖条+数值绘制面。

use crate::render::canvas::PixCanvas;
use crate::render::font::LoadedFont;
use crate::render::palette::colors;
use crate::render::primitives::{draw_h_rect, ring1px, text_shaded_auto};

/// UIBaseElements.drawVBar (UIBaseElements): 竖条 (底对齐, shade 环 +
/// c 内芯); val_height<0 分支为条自 y 向下生长 (GearFlaps 值域 0..100 不可达, 保真保留)
#[allow(clippy::too_many_arguments)] // 对齐 Java drawVBar(g2d,x,y,width,height,val_height,borderwidth,c)
pub(crate) fn draw_v_bar(cv: &mut PixCanvas, x: i32, y: i32, w: i32, h: i32, val_h: i32, bw: i32, c: [u8; 4]) {
    if val_h >= 0 {
        ring1px(cv, x, y - h, w - 1, h - 1, colors().shade_shape);
        cv.fill_rect(x + bw, y + bw - val_h, w - 2 * bw, val_h - 2 * bw, c);
    } else {
        ring1px(cv, x, y, w - 1, -h - 1, colors().shade_shape); // 负高 → 不绘制
        cv.fill_rect(x + bw, y + bw, w - 2 * bw, -val_h - 2 * bw, c);
    }
}

/// UIBaseElements.drawVBarTextNum (UIBaseElements): 竖条 + 随值指针横线 +
/// 数值文本。lbl 形参在 Java 中传入后未绘制 (drawVBarText 的标签绘制已注释), 保真保留
#[allow(clippy::too_many_arguments)] // 对齐 Java drawVBarTextNum(g2d,x,y,width,height,val_height,borderwidth,c,lbl,num,lblFont,numFont)
pub(crate) fn draw_v_bar_text_num(
    cv: &mut PixCanvas,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    val_h: i32,
    bw: i32,
    c: [u8; 4],
    _lbl: &str,
    num: &str,
    _lbl_font: &LoadedFont,
    num_font: &LoadedFont,
    aa: bool,
) {
    let val_h = if val_h > h { h } else { val_h };
    draw_v_bar(cv, x, y, w, h, val_h, bw, c);
    // 指针横线 (drawHRect): colorLabel, 总宽 = width + 3*numFontSize
    draw_h_rect(
        cv,
        x,
        y - val_h - 1,
        w + 3 * num_font.size,
        3,
        1,
        colors().label,
    );
    // 数值文本: shade (+1,+1) + 本色 colorLabel (基线 y-val_height-2)
    text_shaded_auto(cv, num_font, x + w, y - val_h - 2, num, colors().label, aa);
}
