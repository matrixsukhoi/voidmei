//! 字段字体三档 (字段网格时代的多行绘制面已随 fields.grid 原子化退役,
//! 单行绘制在 widgets::data_field — 直画 PixCanvas)。

use std::rc::Rc;

use crate::layout::RenderCtx;
use crate::render::font::LoadedFont;

pub struct FontTriple {
    pub num: Rc<LoadedFont>,
    pub label: Rc<LoadedFont>,
    pub unit: Rc<LoadedFont>,
}

impl FontTriple {
    /// num=BOLD(fontSize), label=BOLD(round(fontSize/2)), unit=PLAIN(round(fontSize/2))
    /// (线程本地缓存版 — data.field 组件工厂每页 N 次, 字体文件/glyph 缓存只首次)
    pub fn load(fonts_dir: &std::path::Path, ctx: &RenderCtx) -> Result<Self, String> {
        let bold = fonts_dir.join("sarasa-mono-sc-bold.ttf");
        let regular = fonts_dir.join("sarasa-mono-sc-regular.ttf");
        Ok(FontTriple {
            num: LoadedFont::new_cached(&bold, ctx.font_size)?,
            label: LoadedFont::new_cached(&bold, ctx.label_font_size)?,
            unit: LoadedFont::new_cached(&regular, ctx.unit_font_size)?,
        })
    }
}
