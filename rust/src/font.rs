//! 字体加载与文本度量/绘制
//! 度量来源 ttf-parser (hhea, 与 Java FontMetrics 同源), 光栅化 swash (Format::Alpha)
//! 语义对齐 Java: charWidth=round(advance), charsWidth=Σround; getHeight=ascent+descent+leading

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::Format;
use ttf_parser::Face;

/// Java FontMetrics.getHeight() 语义: round(各分量) 求和
#[derive(Debug, Clone, Copy)]
pub struct FontMetricsCal {
    pub ascent: i32,
    pub descent: i32,
    pub leading: i32,
    pub height: i32,
}

struct CachedGlyph {
    /// 相对基线原点的包围盒 (x 向右, y 向下)
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    /// 灰度 alpha 遮罩
    alpha: Vec<u8>,
}

/// 单个字号的一份字体 (对应 Java 一个 Font 实例)
pub struct LoadedFont {
    data: Arc<Vec<u8>>,
    pub size: i32,
    /// coverage 幂变换指数: 对齐 Java2D 文本 AA 的 gamma 校正 (实验校准)
    pub gamma: RefCell<f32>,
    scale_ctx: RefCell<ScaleContext>,
    glyph_cache: RefCell<HashMap<(u32, bool, u32), Rc<CachedGlyph>>>,
    adv_cache: RefCell<HashMap<char, i32>>,
}

fn round_f(x: f32) -> i32 {
    // Java Math.round(float): floor(x+0.5)
    (x + 0.5).floor() as i32
}

impl LoadedFont {
    /// 从 ttf/otf 数据创建指定字号的字体
    pub fn new(path: &std::path::Path, size: i32) -> Result<Self, String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("读取字体 {} 失败: {}", path.display(), e))?;
        let data = Arc::new(data);
        Ok(LoadedFont {
            data,
            size,
            gamma: RefCell::new(1.0),
            scale_ctx: RefCell::new(ScaleContext::new()),
            glyph_cache: RefCell::new(HashMap::new()),
            adv_cache: RefCell::new(HashMap::new()),
        })
    }

    fn face(&self) -> Result<Face<'_>, String> {
        Face::parse(&self.data, 0).map_err(|e| format!("解析字体失败: {:?}", e))
    }

    /// Java FontMetrics 语义的度量 (hhea 的 descent 为负值, Java 取正距离)
    pub fn metrics(&self) -> FontMetricsCal {
        let face = self.face().expect("字体已校验");
        let upem = face.units_per_em() as f32;
        let asc = face.ascender() as f32 * self.size as f32 / upem;
        let desc = face.descender() as f32 * self.size as f32 / upem;
        let lead = face.line_gap() as f32 * self.size as f32 / upem;
        let ascent = round_f(asc);
        let descent = round_f(-desc);
        let leading = round_f(lead);
        FontMetricsCal {
            ascent,
            descent,
            leading,
            height: ascent + descent + leading,
        }
    }

    /// 单字符 advance (Java charWidth: round 后 int)
    pub fn char_width(&self, ch: char) -> i32 {
        if let Some(&w) = self.adv_cache.borrow().get(&ch) {
            return w;
        }
        let face = self.face().expect("字体已校验");
        let gid = face.glyph_index(ch).unwrap_or(ttf_parser::GlyphId(0));
        let upem = face.units_per_em() as f32;
        let adv_units = face
            .glyph_hor_advance(gid)
            .unwrap_or(0) as f32;
        let w = round_f(adv_units * self.size as f32 / upem);
        self.adv_cache.borrow_mut().insert(ch, w);
        w
    }

    /// Java charsWidth: 逐字符 round 后累加
    pub fn measure(&self, text: &str) -> i32 {
        text.chars().map(|ch| self.char_width(ch)).sum()
    }

    /// 光栅化 glyph 并缓存 (aa=false 时 1-bit 二值化, 对齐 Java TEXT_ANTIALIAS_OFF)
    fn glyph(&self, ch: char, aa: bool) -> Option<Rc<CachedGlyph>> {
        let gamma = *self.gamma.borrow();
        let key = (ch as u32, aa, gamma.to_bits());
        if let Some(g) = self.glyph_cache.borrow().get(&key) {
            return Some(Rc::clone(g));
        }
        let face = self.face().ok()?;
        let gid = face.glyph_index(ch).unwrap_or(ttf_parser::GlyphId(0));
        // swash FontRef 直接引用字体数据
        let font = swash::FontRef::from_index(&self.data, 0)?;
        let mut ctx = self.scale_ctx.borrow_mut();
        let mut scaler = ctx.builder(font).size(self.size as f32).hint(true).build();
        let image = Render::new(&[Source::Outline])
            .format(Format::Alpha)
            .render(&mut scaler, gid.0)?;
        let p = image.placement;
        let mut alpha = image.data.to_vec();
        // gamma 幂变换: 对齐 Java2D 文本 AA 的覆盖率校正 (实验校准, 默认 1.0)
        if gamma != 1.0 {
            for a in alpha.iter_mut() {
                *a = (255.0 * (*a as f32 / 255.0).powf(gamma) + 0.5) as u8;
            }
        }
        if !aa {
            // 无 AA: 覆盖率二值化 (Java 无 AA 文本是 1-bit coverage)
            for a in alpha.iter_mut() {
                *a = if *a >= 128 { 255 } else { 0 };
            }
        }
        let g = Rc::new(CachedGlyph {
            x: p.left,
            y: p.top,
            w: p.width as i32,
            h: p.height as i32,
            alpha,
        });
        self.glyph_cache.borrow_mut().insert(key, Rc::clone(&g));
        Some(g)
    }
}

/// 绘制目标画布 (直通 RGBA, 导出/比对用; 窗口呈现前转预乘)
pub struct Canvas {
    pub width: i32,
    pub height: i32,
    /// RGBA 直通, 行主序, length = w*h*4
    pub buf: Vec<u8>,
}

impl Canvas {
    pub fn new(width: i32, height: i32) -> Self {
        Canvas {
            width,
            height,
            buf: vec![0; (width * height * 4) as usize],
        }
    }

    /// 整幅铺色 (预览灰底; 直接覆盖, 非 SrcOver)
    pub fn fill(&mut self, color: [u8; 4]) {
        for px in self.buf.chunks_exact_mut(4) {
            px.copy_from_slice(&color);
        }
    }

    /// 以 (x, baseline) 为基线原点绘制文本, SrcOver 合成 (同 Java2D TYPE_INT_ARGB)
    pub fn draw_text(
        &mut self,
        font: &LoadedFont,
        x: i32,
        baseline: i32,
        text: &str,
        color: [u8; 4],
        aa: bool,
    ) {
        let mut pen_x = x;
        for ch in text.chars() {
            let adv = font.char_width(ch);
            if let Some(g) = font.glyph(ch, aa) {
                self.blit_glyph(&g, pen_x, baseline, color);
            }
            pen_x += adv;
        }
    }

    fn blit_glyph(&mut self, g: &CachedGlyph, pen_x: i32, baseline: i32, color: [u8; 4]) {
        let gx = pen_x + g.x;
        // swash 0.2 的 placement.top 是字形顶相对基线的高度(正值向上), 与常见
        // "offset 向下正" 约定相反 (glyph_placement_debug 测试实测确认)
        let gy = baseline - g.y;
        for row in 0..g.h {
            let py = gy + row;
            if py < 0 || py >= self.height {
                continue;
            }
            for col in 0..g.w {
                let px = gx + col;
                if px < 0 || px >= self.width {
                    continue;
                }
                let cov = g.alpha[(row * g.w + col) as usize];
                if cov == 0 {
                    continue;
                }
                // 覆盖率 × 字色 alpha = 该像素有效 src alpha
                let a = (color[3] as u32 * cov as u32 + 127) / 255;
                if a == 0 {
                    continue;
                }
                let idx = ((py * self.width + px) * 4) as usize;
                // SrcOver, Java2D 同款: 预乘中间精度, 存直通 (颜色除以合成后 alpha)
                let fa = a as f32 / 255.0;
                let fda = self.buf[idx + 3] as f32 / 255.0;
                let out_a = fa + fda * (1.0 - fa);
                if out_a <= 0.0 {
                    continue;
                }
                for c in 0..3 {
                    let sc = color[c] as f32;
                    let dc = self.buf[idx + c] as f32;
                    // 直通 SrcOver: out_c = (sc*fa + dc*fda*(1-fa)) / out_a
                    let out_c = (sc * fa + dc * fda * (1.0 - fa)) / out_a;
                    self.buf[idx + c] = out_c.min(255.0).round() as u8;
                }
                self.buf[idx + 3] = (out_a * 255.0 + 0.5) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sarasa 字体度量快照: 校准 numHeight 的依据 (Java FontMetrics 实测 24/6/1 @24px)
    #[test]
    fn sarasa_metrics_snapshot() {
        let f = LoadedFont::new(std::path::Path::new("../fonts/sarasa-mono-sc-bold.ttf"), 24).unwrap();
        let m = f.metrics();
        println!("rust metrics @24px: {:?}", m);
        // dump 原始表值用于校准分析
        let data = std::fs::read("../fonts/sarasa-mono-sc-bold.ttf").unwrap();
        let face = Face::parse(&data, 0).unwrap();
        println!("upem={}", face.units_per_em());
        println!(
            "hhea: ascender={} descender={} line_gap={}",
            face.ascender(),
            face.descender(),
            face.line_gap()
        );
        if let Some(os2) = face.tables().os2 {
            println!(
                "os2 typo: asc={} desc={} linegap={}",
                os2.typographic_ascender(), os2.typographic_descender(), os2.typographic_line_gap()
            );
            println!(
                "os2 win:  asc={} desc={}",
                os2.windows_ascender(), os2.windows_descender()
            );
        }
        assert!(m.height > 0);
    }
}

#[cfg(test)]
mod debug_tests {
    use super::*;

    /// 调试: 打印字形 placement 与 ASCII 渲染, 排查基线定位
    #[test]
    fn glyph_placement_debug() {
        let f = LoadedFont::new(std::path::Path::new("../fonts/sarasa-mono-sc-bold.ttf"), 24).unwrap();
        let m = f.metrics();
        println!("metrics @24px: {:?}", m);
        for ch in ['5', '表'] {
            let g = f.glyph(ch, true).unwrap();
            println!("char {:?}: left={} top={} w={} h={}", ch, g.x, g.y, g.w, g.h);
        }
        // ASCII art: '5' 画在基线=24 的 48x48 画布 (draw_text 走修正后的 blit)
        let mut c = Canvas::new(48, 48);
        c.draw_text(&f, 0, 24, "5", [255, 255, 255, 255], true);
        for y in 0..48 {
            let mut row = String::new();
            for x in 0..48 {
                let a = c.buf[(y * 48 + x) * 4 + 3];
                row.push(if a > 200 { '#' } else if a > 80 { '+' } else if a > 10 { '.' } else { ' ' });
            }
            println!("{:02}|{}", y, row);
        }
    }
}
