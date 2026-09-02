//! 渲染底座域 (波10 分域; 三胞胎命名消解):
//! canvas (原 render2d, tiny-skia PixCanvas 画布) / fields (原 render, 字段网格
//! GDI 形态) / renderers (OverlayRenderer trait 族) / font (swash/ttf 光栅)
//! / palette (原 global_colors, 五色+AA 全局仓) / primitives (基元收敛层)。

pub mod canvas;
pub mod fields;
pub mod font;
pub mod palette;
pub mod primitives;
pub mod renderers;
