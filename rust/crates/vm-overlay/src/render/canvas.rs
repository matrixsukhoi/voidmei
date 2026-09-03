//! render2d: C 类组件统一绘制层 (D7 裁决落地: 矢量基元走 tiny-skia)
//!
//! PixCanvas = tiny_skia::Pixmap 封装的预乘 RGBA 主画布, 复刻 Java Graphics2D
//! 语义 (颜色/几何/AA 分支对齐各组件 paintComponent 的视觉行为, 非代码结构):
//!
//! | Rust 基元 | Java 对应 | 语义要点 |
//! |---|---|---|
//! | fill_rect | Graphics.fillRect | 上左 (x,y) 精确 w×h 像素, 无描边 +1 扩展 |
//! | draw_line | Graphics.drawLine + BasicStroke(w, CAP_ROUND, JOIN_ROUND) | 端点圆帽外伸 w/2 |
//! | draw_line_cap | GraphicsUtil.createPreciseStroke (CAP_BUTT+JOIN_MITER) | 刻度线精确端点 |
//! | fill_circle | Graphics.fillOval | 圆特例: 内切于 fillOval(cx-r,cy-r,2r,2r) |
//! | stroke_circle | Graphics.drawOval | 同上描边 (CompassGauge 双层描圆) |
//! | stroke_arc | Graphics.drawArc | 0°=3 点钟, 正角=视觉逆时针 (Java oracle 实测) |
//! | fill_path | Graphics.fillPolygon | 偶奇填充规则, 自动闭合 |
//! | draw_text | Graphics.drawString | font.rs swash 光栅化/字形缓存共用 |
//!
//! AA 开关对齐 Application.graphAASetting / textAASetting:
//! aa=false 走 tiny-skia 非 AA 扫描线光栅化 (硬边), 文本二值化阈值 128 与
//! font.rs glyph() 一致。
//!
//! render.rs 的 FlightInfoOverlay 仍走 Canvas 直通管线 (POC 对拍基线勿动),
//! 本层为并行新增, 后续 C 类组件批统一使用 PixCanvas。
//!
//! C 类像素对拍容差 (Java oracle 固有系统差, 勿逐字节断言): Java fillOval/drawOval
//! 非 AA 光栅相对几何圆有 ~0.5-1px 内缩; 预乘取整路径存在 ±1 LSB 色差
//! (tiny-skia 取整式 vs 截断式镜像)。

use crate::render::font::{CachedGlyph, LoadedFont};

/// 线端点样式: Java BasicStroke.CAP_ROUND / CAP_BUTT / CAP_SQUARE
/// (GraphicsUtil.createRoundedStroke / createPreciseStroke / 裸 new BasicStroke(w) 三族)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineCapStyle {
    /// 圆帽: 端点外伸 width/2 半圆 (HUD 仪表线主样式, CompassGauge/AttitudeIndicatorGauge)
    Round,
    /// 平帽: 精确端点无外伸 (MarkedGauge marker/GaugeBarStyle 刻度线)
    Butt,
    /// 方帽: 端点沿切向外伸 width/2 的平头 (Java 裸 new BasicStroke(w) 的默认 CAP_SQUARE,
    /// AttitudeOverlay 边框/地平线线/FlapAngleBar tickStroke/CompassGauge THIN_STROKE)
    Square,
}

/// 直通 RGBA [u8;4] → tiny-skia Color (直通 f32, 混合管线内部预乘)
fn ts_color(color: [u8; 4]) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(color[0], color[1], color[2], color[3])
}

/// 纯色画刷; blend_mode 默认 SourceOver = Java2D 默认复合
fn solid_paint(color: [u8; 4], aa: bool) -> tiny_skia::Paint<'static> {
    let mut p = tiny_skia::Paint::default();
    p.set_color(ts_color(color));
    // PORT: aa=false ↔ Java ANTIALIAS_OFF (graphAASetting), tiny-skia 走非 AA 扫描线
    p.anti_alias = aa;
    p
}

/// BasicStroke 复刻: Java 端 CAP_ROUND 恒配 JOIN_ROUND (createRoundedStroke),
/// CAP_BUTT/CAP_SQUARE 恒配 JOIN_MITER (createPreciseStroke / 裸 BasicStroke 默认)
fn stroke_of(width: f32, cap: LineCapStyle) -> tiny_skia::Stroke {
    tiny_skia::Stroke {
        // PORT: Java BasicStroke(0)=hairline 最细 1px 线, 负宽构造即抛异常;
        // tiny-skia width<=0 渲染为空 — 统一钳到 1.0 对齐 hairline 语义
        width: if width <= 0.0 { 1.0 } else { width },
        // Java BasicStroke 默认 miterlimit 10.0 (GraphicsUtil 两族均不覆盖该参)
        miter_limit: 10.0,
        line_cap: match cap {
            LineCapStyle::Round => tiny_skia::LineCap::Round,
            LineCapStyle::Butt => tiny_skia::LineCap::Butt,
            LineCapStyle::Square => tiny_skia::LineCap::Square,
        },
        line_join: match cap {
            LineCapStyle::Round => tiny_skia::LineJoin::Round,
            LineCapStyle::Butt | LineCapStyle::Square => tiny_skia::LineJoin::Miter,
        },
        dash: None,
    }
}

/// 预乘存储的直通重构: p = trunc(s*a/255) 的逆近似 (round(p*255/a))
fn un_premul(p: u8, a: u8) -> u8 {
    if a == 0 {
        0
    } else {
        ((p as u32 * 255 + a as u32 / 2) / a as u32) as u8
    }
}

/// 形状绘制后整体重构直通伴随缓冲 (逐像素 un_premul)
fn rebuild_straight(premul: &[u8], straight: &mut [u8]) {
    for (s, d) in premul.chunks_exact(4).zip(straight.chunks_exact_mut(4)) {
        let a = s[3];
        d[0] = un_premul(s[0], a);
        d[1] = un_premul(s[1], a);
        d[2] = un_premul(s[2], a);
        d[3] = a;
    }
}

/// 预乘 RGBA 主画布 (C 类组件的统一绘制目标)
pub struct PixCanvas {
    pm: tiny_skia::Pixmap,
    /// 直通域伴随缓冲 (文本合成用): Java2D TYPE_INT_ARGB 直通域的精确复刻,
    /// 预乘 u8 无法无损往返 (字形 AA 边缘互相叠加时会丢 LSB), 故文本路径
    /// 在直通域合成后按 window.rs 截断式镜像写预乘;
    /// 形状绘制使其失效, 下次文本合成前由预乘存储整体重构
    straight: Vec<u8>,
    straight_valid: bool,
}

fn pixmap_of(width: i32, height: i32) -> Option<tiny_skia::Pixmap> {
    if width < 0 || height < 0 {
        return None;
    }
    tiny_skia::Pixmap::new(width as u32, height as u32)
}

impl PixCanvas {
    pub fn new(width: i32, height: i32) -> Result<Self, String> {
        let pm = pixmap_of(width, height).ok_or_else(|| format!("非法画布尺寸 {width}x{height}"))?;
        let n = pm.data().len();
        Ok(PixCanvas {
            pm,
            straight: vec![0; n],
            straight_valid: true,
        })
    }

    /// 清空并按需重建 (每帧复用: 同尺寸原地清零不重分配, 语义同 render_fields_fixed 的 buf.fill(0))
    /// 返回 false = 非法尺寸, 状态不变
    pub fn clear(&mut self, width: i32, height: i32) -> bool {
        if self.pm.width() as i32 == width && self.pm.height() as i32 == height {
            self.pm.fill(tiny_skia::Color::TRANSPARENT);
            self.straight.fill(0);
            self.straight_valid = true;
            return true;
        }
        match pixmap_of(width, height) {
            Some(pm) => {
                let n = pm.data().len();
                self.pm = pm;
                self.straight = vec![0; n];
                self.straight_valid = true;
                true
            }
            None => false,
        }
    }

    pub fn width(&self) -> i32 {
        self.pm.width() as i32
    }

    pub fn height(&self) -> i32 {
        self.pm.height() as i32
    }

    /// 预乘 RGBA → 预乘 BGRA (UpdateLayeredWindow / XRender ARGB32 呈现格式)。
    /// 存储已是预乘, 此处仅做字节序转换。文本路径与 font::Canvas 直通参照路径
    /// 逐字节等价 — 由双路文本对拍测试钉死;
    /// 形状路径存在 tiny-skia 预乘取整 vs 截断镜像的 ±1 LSB 系统差 (对拍需容差)。
    pub fn to_premul_bgra(&self) -> Vec<u8> {
        let src = self.pm.data();
        let mut out = vec![0u8; src.len()];
        for (i, p) in src.chunks_exact(4).enumerate() {
            let o = i * 4;
            out[o] = p[2];
            out[o + 1] = p[1];
            out[o + 2] = p[0];
            out[o + 3] = p[3];
        }
        out
    }

    /// 只读访问底层 Pixmap (后续组件批的 draw_pixmap/纹理叠加入口)
    pub fn pixmap(&self) -> &tiny_skia::Pixmap {
        &self.pm
    }

    /// 整帧直通 RGBA 以 SrcOver 合入 (POC font::Canvas → PixCanvas 桥,
    /// FlightInfo 专径渲染栈产出直通帧)。逐像素式与 font.rs Canvas.blit_glyph
    /// 同源: 直通域 SrcOver 合成 + 截断式镜像预乘 — host 预览灰底 (fill_rect
    /// 先行) 得以保留, live 全透明底上等效整帧替换。
    /// 尺寸不符返回 false 不动状态 (调用方自查)
    pub fn composite_straight_frame(&mut self, rgba_direct: &[u8]) -> bool {
        if rgba_direct.len() != self.straight.len() {
            return false;
        }
        if !self.straight_valid {
            // 形状动过预乘存储 (如 fill_rect 铺灰底): 先重构直通域再合成
            rebuild_straight(self.pm.data(), &mut self.straight);
            self.straight_valid = true;
        }
        let pm = self.pm.data_mut();
        let st = &mut self.straight;
        for ((s, d), m) in rgba_direct
            .chunks_exact(4)
            .zip(st.chunks_exact_mut(4))
            .zip(pm.chunks_exact_mut(4))
        {
            let sa = s[3] as u32;
            if sa == 0 {
                continue; // 零 alpha 源 SrcOver = 目标不变 (Java 快路径)
            }
            let fa = sa as f32 / 255.0;
            let fda = d[3] as f32 / 255.0;
            let out_a = fa + fda * (1.0 - fa);
            if out_a <= 0.0 {
                continue;
            }
            let out_a_u8 = (out_a * 255.0 + 0.5) as u8;
            for c in 0..3 {
                let out_c = (s[c] as f32 * fa + d[c] as f32 * fda * (1.0 - fa)) / out_a;
                d[c] = out_c.min(255.0).round() as u8;
            }
            d[3] = out_a_u8;
            // 镜像写预乘 (截断式同 window.rs to_premul_bgra)
            for c in 0..3 {
                m[c] = (d[c] as u32 * out_a_u8 as u32 / 255) as u8;
            }
            m[3] = out_a_u8;
        }
        true
    }

    /// Java Graphics.fillRect: 整数坐标下 AA 与非 AA 输出一致 (精确覆盖), 故无 aa 参数;
    /// 负宽高静默不绘制 (Java 同)
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: [u8; 4]) {
        let rect = match tiny_skia::Rect::from_xywh(x as f32, y as f32, w as f32, h as f32) {
            Some(r) => r,
            None => return, // PORT: Java 负宽高 fillRect 不绘制
        };
        let paint = solid_paint(color, false);
        self.pm
            .fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
        self.straight_valid = false;
    }

    /// Java Graphics.drawLine + BasicStroke(width, CAP_ROUND, JOIN_ROUND)
    /// (LinearGauge / CompassGauge / AttitudeIndicatorGauge 主样式)
    #[allow(clippy::too_many_arguments)] // 签名对齐 Java drawLine(x0,y0,x1,y1)+线型三元组
    pub fn draw_line(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        width: f32,
        color: [u8; 4],
        aa: bool,
    ) {
        self.draw_line_cap(x0, y0, x1, y1, width, color, LineCapStyle::Round, aa);
    }

    /// 平帽变体: GraphicsUtil.createPreciseStroke (CAP_BUTT+JOIN_MITER),
    /// 端点无外伸, 用于必须精确对齐组件边界的线 (MarkedGauge tickStroke)
    #[allow(clippy::too_many_arguments)] // 同 draw_line, 额外 cap 参数对齐 Java 线型族
    pub fn draw_line_cap(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        width: f32,
        color: [u8; 4],
        cap: LineCapStyle,
        aa: bool,
    ) {
        let mut pb = tiny_skia::PathBuilder::new();
        pb.move_to(x0 as f32, y0 as f32);
        pb.line_to(x1 as f32, y1 as f32);
        if let Some(path) = pb.finish() {
            let paint = solid_paint(color, aa);
            let stroke = stroke_of(width, cap);
            self.pm
                .stroke_path(&path, &paint, &stroke, tiny_skia::Transform::identity(), None);
            self.straight_valid = false;
        }
    }

    /// Java Graphics.fillOval 的圆特例: 圆心 (cx,cy) 半径 r 实心圆
    /// (= fillOval(cx-r, cy-r, 2r, 2r), Java 椭圆内切于外接盒)
    pub fn fill_circle(&mut self, cx: i32, cy: i32, r: i32, color: [u8; 4], aa: bool) {
        if r <= 0 {
            return; // PORT: Java 零尺寸椭圆不绘制
        }
        let mut pb = tiny_skia::PathBuilder::new();
        pb.push_circle(cx as f32, cy as f32, r as f32);
        if let Some(path) = pb.finish() {
            let paint = solid_paint(color, aa);
            self.pm.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
            self.straight_valid = false;
        }
    }

    /// Java Graphics.drawOval 的圆特例: 描边圆 (CompassGauge 双层描圆)
    pub fn stroke_circle(&mut self, cx: i32, cy: i32, r: i32, width: f32, color: [u8; 4], aa: bool) {
        if r <= 0 {
            return;
        }
        let mut pb = tiny_skia::PathBuilder::new();
        pb.push_circle(cx as f32, cy as f32, r as f32);
        if let Some(path) = pb.finish() {
            let paint = solid_paint(color, aa);
            let stroke = stroke_of(width, LineCapStyle::Round);
            self.pm
                .stroke_path(&path, &paint, &stroke, tiny_skia::Transform::identity(), None);
            self.straight_valid = false;
        }
    }

    /// Java Graphics.drawArc(cx-r, cy-r, 2r, 2r, start, end-start) 的圆特例。
    /// 角度约定 (Java 8 oracle 实测): 0°=3 点钟, 正角=**视觉逆时针**
    /// (drawArc(0,90) 走 3点→12点右上象限), 即 point(θ)=(cx+r·cosθ, cy−r·sinθ);
    /// 从 start_angle 扫到 end_angle, 负 sweep = 顺时针反向弧 (同 Java 负 arcAngle),
    /// 零 sweep 不绘制 (Java drawArc(start,0) oracle 实测 0 像素), |sweep|≥360 整圆。
    /// cap 对齐调用方线型: AttitudeIndicatorGauge.drawMarks 的
    /// drawArc(...,-180,180) 下半圆 = stroke_arc(cx,cy,r,-180,0,Round);
    /// AttitudeOverlay.locater 的同弧用裸 BasicStroke(3) → Square
    #[allow(clippy::too_many_arguments)] // 对齐 Java drawArc(x,y,w,h,start,extent)+线型三元组
    pub fn stroke_arc(
        &mut self,
        cx: i32,
        cy: i32,
        r: i32,
        start_angle: f32,
        end_angle: f32,
        width: f32,
        color: [u8; 4],
        cap: LineCapStyle,
        aa: bool,
    ) {
        if r <= 0 {
            return;
        }
        // PORT: Java drawArc 角度是 int 入参, NaN/±inf 在 Java 调用点 (int) 强转即归 0;
        // Rust f32 直收需消毒 — 非有限角度不绘制 (防 -inf 死循环 / NaN 进光栅化 UB)
        if !start_angle.is_finite() || !end_angle.is_finite() {
            return;
        }
        let sweep = end_angle - start_angle;
        if !sweep.is_finite() {
            return; // 有限入参相减溢出 ±inf, 同上消毒
        }
        if sweep == 0.0 {
            return; // PORT: Java 零 extent 不绘制
        }
        let mut pb = tiny_skia::PathBuilder::new();
        if sweep.abs() >= 360.0 {
            pb.push_circle(cx as f32, cy as f32, r as f32); // |extent|≥360 整圆 = drawOval
        } else {
            append_arc(&mut pb, cx as f32, cy as f32, r as f32, start_angle, sweep);
        }
        if let Some(path) = pb.finish() {
            let paint = solid_paint(color, aa);
            let stroke = stroke_of(width, cap);
            self.pm
                .stroke_path(&path, &paint, &stroke, tiny_skia::Transform::identity(), None);
            self.straight_valid = false;
        }
    }

    /// Java Graphics.fillPolygon(xs, ys, n): 偶奇填充规则 + 自动闭合 (不足 3 点不绘制)
    pub fn fill_path(&mut self, points: &[(f32, f32)], color: [u8; 4], aa: bool) {
        if points.len() < 3 {
            return;
        }
        // PORT: Java fillPolygon 坐标是 int[], NaN 在调用点 (int) 强转已归 0;
        // Rust f32 直收需消毒 — 含非有限坐标的多边形整体不绘制 (避免 tiny-skia 静默 UB)
        if points.iter().any(|&(x, y)| !x.is_finite() || !y.is_finite()) {
            return;
        }
        let mut pb = tiny_skia::PathBuilder::new();
        pb.move_to(points[0].0, points[0].1);
        for &(x, y) in &points[1..] {
            pb.line_to(x, y);
        }
        pb.close();
        if let Some(path) = pb.finish() {
            let paint = solid_paint(color, aa);
            self.pm.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::EvenOdd,
                tiny_skia::Transform::identity(),
                None,
            );
            self.straight_valid = false;
        }
    }

    /// Java Graphics.drawString 文本桥: (x, baseline) 为基线原点,
    /// pen 推进 (Σround(advance)) 与 font.rs Canvas::draw_text 逐句对齐,
    /// 共用 swash 光栅化与字形缓存 (gamma 校正/无 AA 二值化同源)
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
                self.blit_glyph_premul(&g, pen_x, baseline, color);
            }
            pen_x += adv;
        }
    }

    /// glyph alpha 遮罩 → 画布。
    /// 合成数学与 font.rs Canvas.blit_glyph (直通域) 逐式一致: 在直通伴随缓冲上
    /// 做 SrcOver (Java2D TYPE_INT_ARGB 精确复刻, 字形 AA 边缘叠加无 LSB 损失),
    /// 再按 window.rs to_premul_bgra 相同的截断式 (s*a/255) 镜像写预乘存储 —
    /// 保证两路文本输出逐字节一致。
    fn blit_glyph_premul(&mut self, g: &CachedGlyph, pen_x: i32, baseline: i32, color: [u8; 4]) {
        let (w, h) = (self.width(), self.height());
        if !self.straight_valid {
            // 形状动过预乘存储: 整体重构直通域 (近似还原, 仅影响形状上的文本)
            rebuild_straight(self.pm.data(), &mut self.straight);
            self.straight_valid = true;
        }
        let gx = pen_x + g.x;
        // swash placement.top 是字形顶相对基线的高度 (正值向上), 同 Canvas.blit_glyph
        let gy = baseline - g.y;
        let pm_data = self.pm.data_mut();
        let st = &mut self.straight;
        for row in 0..g.h {
            let py = gy + row;
            if py < 0 || py >= h {
                continue;
            }
            for col in 0..g.w {
                let px = gx + col;
                if px < 0 || px >= w {
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
                let idx = ((py * w + px) * 4) as usize;
                let s = &mut st[idx..idx + 4];
                // SrcOver, Java2D 同款 (font.rs Canvas.blit_glyph 同式), 直通域:
                let fa = a as f32 / 255.0;
                let fda = s[3] as f32 / 255.0;
                let out_a = fa + fda * (1.0 - fa);
                if out_a <= 0.0 {
                    continue;
                }
                let out_a_u8 = (out_a * 255.0 + 0.5) as u8;
                for c in 0..3 {
                    let sc = color[c] as f32;
                    let dc = s[c] as f32;
                    // 直通 SrcOver: out_c = (sc*fa + dc*fda*(1-fa)) / out_a
                    let out_c = (sc * fa + dc * fda * (1.0 - fa)) / out_a;
                    s[c] = out_c.min(255.0).round() as u8;
                }
                s[3] = out_a_u8;
                // 镜像写预乘 (截断式同 window.rs to_premul_bgra)
                let d = &mut pm_data[idx..idx + 4];
                for c in 0..3 {
                    d[c] = (s[c] as u32 * out_a_u8 as u32 / 255) as u8;
                }
                d[3] = out_a_u8;
            }
        }
    }

    /// 调试/像素对拍导出 (tiny-skia 自带去预乘 PNG 编码)
    pub fn save_png(&self, path: &std::path::Path) -> Result<(), String> {
        let bytes = self
            .pm
            .encode_png()
            .map_err(|e| format!("PNG 编码失败: {e}"))?;
        std::fs::write(path, bytes).map_err(|e| format!("写 {} 失败: {}", path.display(), e))
    }
}

/// 圆弧 → ≤90° 三次贝塞尔段 (SVG kappa 近似: K = 4/3·tan(Δθ/4))
/// Java drawArc 角度语义 (oracle 实测): point(θ) = (cx + r·cosθ, cy − r·sinθ)
/// — y 分量取负 (角度按数学逆时针解释, 与屏幕 y 向下无关), 正 sweep 视觉逆时针;
/// sweep 可负 = 反向 (顺时针) 弧, kappa 随 seg 变号自动反向
fn append_arc(pb: &mut tiny_skia::PathBuilder, cx: f32, cy: f32, r: f32, start_deg: f32, sweep_deg: f32) {
    let total = sweep_deg.to_radians();
    let n = ((total.abs() / std::f32::consts::FRAC_PI_2).ceil() as i32).max(1);
    let seg = total / n as f32;
    let kappa = (4.0 / 3.0) * (seg / 4.0).tan();
    let point = |t: f32| (cx + r * t.cos(), cy - r * t.sin());
    let mut th = start_deg.to_radians();
    let (mut x0, mut y0) = point(th);
    pb.move_to(x0, y0);
    for _ in 0..n {
        let (t0, t1) = (th, th + seg);
        th = t1;
        let (x1, y1) = point(t1);
        // 切向 T(θ) = d/dθ(cosθ, −sinθ) = (−sinθ, −cosθ)
        let (c1x, c1y) = (x0 - kappa * r * t0.sin(), y0 - kappa * r * t0.cos());
        let (c2x, c2y) = (x1 + kappa * r * t1.sin(), y1 + kappa * r * t1.cos());
        pb.cubic_to(c1x, c1y, c2x, c2y, x1, y1);
        x0 = x1;
        y0 = y1;
    }
}

#[cfg(test)]
mod tests;
