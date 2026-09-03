//! FlightInfo 直通渲染栈的字段网格绘制 (font::Canvas GDI 形态):
//! value(阴影+本体) → label(阴影+本体) → unit(阴影+本体)
//! (BOS 形态的 PixCanvas 路径在 renderers.rs; render_fields 整帧自建画布变体
//! 已随波12 死代码清扫删除, 生产仅剩 fixed 复用缓冲形态)

use crate::layout::RenderCtx;
use crate::render::font::{Canvas, LoadedFont};

/// 运行时颜色 (对应 Application.colorNum 等, 当前 ui_layout.cfg 默认值)
pub struct RenderColors {
    pub num: [u8; 4],
    pub label: [u8; 4],
    pub unit: [u8; 4],
    pub shade: [u8; 4],
}

/// 当前 ui_layout.cfg 默认配色: fontNum/fontLabel/fontUnit/fontShade (#RRGGBBAA)
pub const DEFAULT_COLORS: RenderColors = RenderColors {
    num: [0xFF, 0xFF, 0xFF, 0xFF],
    label: [0xFF, 0xFF, 0xFF, 0xFF],
    unit: [0xE8, 0x93, 0x32, 0xFF],
    shade: [0x00, 0x00, 0x00, 0xFF],
};

pub struct FontTriple {
    pub num: LoadedFont,
    pub label: LoadedFont,
    pub unit: LoadedFont,
}

impl FontTriple {
    /// num=BOLD(fontSize), label=BOLD(round(fontSize/2)), unit=PLAIN(round(fontSize/2))
    pub fn load(fonts_dir: &std::path::Path, ctx: &RenderCtx) -> Result<Self, String> {
        let bold = fonts_dir.join("sarasa-mono-sc-bold.ttf");
        let regular = fonts_dir.join("sarasa-mono-sc-regular.ttf");
        Ok(FontTriple {
            num: LoadedFont::new(&bold, ctx.font_size)?,
            label: LoadedFont::new(&bold, ctx.label_font_size)?,
            unit: LoadedFont::new(&regular, ctx.unit_font_size)?,
        })
    }
}

/// 单个可见字段 (value 为已格式化字符串)
pub struct FieldText<'a> {
    pub label: &'a str,
    pub unit: &'a str,
    pub value: &'a str,
}

/// 渲染到固定尺寸画布 (live 模式复用缓冲; 先清零, 空白行透明)
pub fn render_fields_fixed(
    canvas: &mut Canvas,
    fields: &[FieldText<'_>],
    ctx: &RenderCtx,
    fonts: &FontTriple,
    colors: &RenderColors,
    aa: bool,
) {
    canvas.buf.fill(0);
    draw_fields(canvas, fields, ctx, fonts, colors, aa);
}

/// 渲染到既有画布 (不清零, 预览模式铺灰底后调用)
pub fn draw_fields(
    canvas: &mut Canvas,
    fields: &[FieldText<'_>],
    ctx: &RenderCtx,
    fonts: &FontTriple,
    colors: &RenderColors,
    aa: bool,
) {
    let (mut ox, mut oy) = ctx.start_offset();
    let lwidth = ctx.lwidth();
    let mut visible_index = 0i32;

    for f in fields {
        // --- TextGauge.draw: 数值 (右对齐, 基线 centerY) ---
        let vbase = ctx.value_baseline(oy);
        let vw = fonts.num.measure(f.value);
        let vx = ox + lwidth - vw - ctx.num_padding();
        draw_shaded(
            canvas,
            &fonts.num,
            vx,
            vbase,
            f.value,
            colors.num,
            colors.shade,
            aa,
        );

        // --- 标签 (基线 y) ---
        draw_shaded(
            canvas,
            &fonts.label,
            ox + lwidth,
            oy,
            f.label,
            colors.label,
            colors.shade,
            aa,
        );

        // --- 单位 (基线 y + labelFontSize) ---
        let ubase = ctx.unit_baseline(oy);
        draw_shaded(
            canvas,
            &fonts.unit,
            ox + lwidth,
            ubase,
            f.unit,
            colors.unit,
            colors.shade,
            aa,
        );

        // --- 列推进 (Java updateOffset: 每画满 columnNum 个换行) ---
        visible_index += 1;
        if visible_index % ctx.column_num == 0 {
            oy += ctx.advance_y();
            ox = ctx.font_size >> 1;
        } else {
            ox += ctx.advance_x();
        }
    }
}

/// Java TextGauge.drawTextShaded: (+1,+1) 阴影先画, 本体后画
// PORT(allow too_many_arguments): 签名对齐 Java drawTextShaded 原参列表
#[allow(clippy::too_many_arguments)]
fn draw_shaded(
    canvas: &mut Canvas,
    font: &LoadedFont,
    x: i32,
    baseline: i32,
    text: &str,
    color: [u8; 4],
    shade: [u8; 4],
    aa: bool,
) {
    canvas.draw_text(font, x + 1, baseline + 1, text, shade, aa);
    canvas.draw_text(font, x, baseline, text, color, aa);
}
