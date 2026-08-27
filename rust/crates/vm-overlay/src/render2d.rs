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
//! AA 开关对齐 Application.graphAASetting / textAASetting (Application.java:101-102):
//! aa=false 走 tiny-skia 非 AA 扫描线光栅化 (硬边), 文本二值化阈值 128 与
//! font.rs glyph() 一致。
//!
//! render.rs 的 FlightInfoOverlay 仍走 Canvas 直通管线 (POC 对拍基线勿动),
//! 本层为并行新增, 后续 C 类组件批统一使用 PixCanvas。
//!
//! C 类像素对拍容差 (Java oracle 固有系统差, 勿逐字节断言): Java fillOval/drawOval
//! 非 AA 光栅相对几何圆有 ~0.5-1px 内缩; 预乘取整路径存在 ±1 LSB 色差
//! (tiny-skia 取整式 vs 截断式镜像)。

use crate::font::{CachedGlyph, LoadedFont};

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
    /// 存储已是预乘, 此处仅做字节序转换。文本路径与 window.rs 的
    /// to_premul_bgra(直通 Canvas) 逐字节等价 — 由双路文本对拍测试钉死;
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
    /// (LinearGauge.java:117 / CompassGauge.java:106 / AttitudeIndicatorGauge.java:185 主样式)
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

    /// Java Graphics.drawOval 的圆特例: 描边圆 (CompassGauge.java:128-132 双层描圆)
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
mod tests {
    use super::*;
    use crate::font::Canvas;
    use crate::window::to_premul_bgra;

    const FONT: &str = "../../../fonts/sarasa-mono-sc-bold.ttf";

    /// 读预乘 RGBA 像素
    fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
        let i = ((y * c.width() + x) * 4) as usize;
        let d = &c.pm.data()[i..i + 4];
        [d[0], d[1], d[2], d[3]]
    }

    /// fillRect 角点精确值: 不透明色预乘=直通, 恰好覆盖 w×h (无 drawRect 的 +1 扩展)
    #[test]
    fn fill_rect_corner_pixels_exact() {
        let mut c = PixCanvas::new(64, 64).unwrap();
        c.fill_rect(10, 12, 20, 30, [0xAB, 0xCD, 0xEF, 0xFF]);
        for (x, y) in [(10, 12), (29, 12), (10, 41), (29, 41)] {
            assert_eq!(px(&c, x, y), [0xAB, 0xCD, 0xEF, 0xFF], "内角点 ({x},{y})");
        }
        for (x, y) in [(9, 12), (30, 12), (10, 11), (10, 42), (9, 41), (30, 41)] {
            assert_eq!(px(&c, x, y), [0, 0, 0, 0], "紧邻外侧 ({x},{y})");
        }
    }

    /// 半透明 fillRect 的预乘存储 + 负宽高不绘制
    #[test]
    fn fill_rect_partial_alpha_and_negative() {
        let mut c = PixCanvas::new(32, 32).unwrap();
        c.fill_rect(5, 5, 10, 10, [200, 100, 50, 128]);
        // 预乘 ≈ c*a/255 (tiny-skia 取整式, 与截断式在此值一致): 100/50/25, a=128
        assert_eq!(px(&c, 10, 10), [100, 50, 25, 128]);
        // PORT: Java 负宽高 fillRect 静默不绘制
        c.fill_rect(0, 0, -5, 5, [255, 255, 255, 255]);
        c.fill_rect(0, 0, 5, -5, [255, 255, 255, 255]);
        assert_eq!(px(&c, 1, 1), [0, 0, 0, 0]);
    }

    /// 无 AA 线: 中线行/列必覆盖, 远离线体必空, 圆帽端点外伸 w/2 有界
    /// (只断言两种像素中心采样约定下都成立的行/列, 避免过钉边界)
    #[test]
    fn draw_line_bw_endpoints_and_caps() {
        let col = [0xFF, 0xFF, 0xFF, 0xFF];
        // 水平线 y=10, x∈[5,25], 宽 3 → stroke 纵向 [8.5,11.5]
        let mut c = PixCanvas::new(48, 48).unwrap();
        c.draw_line(5, 10, 25, 10, 3.0, col, false);
        for x in [5, 15, 25] {
            assert_eq!(px(&c, x, 10), col, "中线 ({x},10)");
            assert_eq!(px(&c, x, 9), col, "上邻行 ({x},9)");
        }
        assert_eq!(px(&c, 15, 7), [0, 0, 0, 0], "线体上方 2px");
        assert_eq!(px(&c, 15, 13), [0, 0, 0, 0], "线体下方 2px");
        // 圆帽: 端点 (5,10) 左侧 (4,10) 距帽心 0.707 < 1.5 必覆盖 (CAP_ROUND vs BUTT 的判据);
        // 右端 (25,10) 本体覆盖, 再外 2px 必空 ((26,10) 距帽心 1.58 > 1.5 属帽边界不钉)
        assert_eq!(px(&c, 4, 10), col, "左圆帽外伸");
        assert_eq!(px(&c, 25, 10), col, "右端点");
        assert_eq!(px(&c, 3, 10), [0, 0, 0, 0], "左帽外");
        assert_eq!(px(&c, 27, 10), [0, 0, 0, 0], "右帽外");
        // 竖线 x=10 宽 3 → stroke 横向 [8.5,11.5], 列 9/10 必覆盖
        let mut c2 = PixCanvas::new(48, 48).unwrap();
        c2.draw_line(10, 5, 10, 40, 3.0, col, false);
        for y in [5, 20, 40] {
            assert_eq!(px(&c2, 9, y), col, "左邻列 (9,{y})");
            assert_eq!(px(&c2, 10, y), col, "中线 (10,{y})");
        }
        assert_eq!(px(&c2, 7, 20), [0, 0, 0, 0], "线体左侧 2px");
        assert_eq!(px(&c2, 12, 20), [0, 0, 0, 0], "线体右侧 2px");
    }

    /// fillCircle 几何: 圆心/内部覆盖, 外部与外接盒角透明
    #[test]
    fn fill_circle_geometry() {
        let mut c = PixCanvas::new(48, 48).unwrap();
        let col = [0x30, 0x60, 0x90, 0xFF];
        c.fill_circle(24, 24, 15, col, false);
        assert_eq!(px(&c, 24, 24), col, "圆心");
        assert_eq!(px(&c, 24, 10), col, "内部 (距心 13.5)");
        assert_eq!(px(&c, 38, 24), col, "内部 (距心 14.5)");
        assert_eq!(px(&c, 24, 8), [0, 0, 0, 0], "外部 (距心 15.5)");
        assert_eq!(px(&c, 10, 10), [0, 0, 0, 0], "外接盒角 (距心 20.5)");
    }

    /// strokeArc 下半圆 (Java oracle: drawArc(-180,180) 弧体在下半 —
    /// drawArc(0,0,40,40,-180,180) downPixels=171 顶部空, 正角=视觉逆时针,
    /// -180→0 途经 -90°=6 点): 弧底/两端覆盖, 上半无弧
    #[test]
    fn stroke_arc_lower_semicircle() {
        let mut c = PixCanvas::new(64, 64).unwrap();
        let col = [0xFF, 0xFF, 0xFF, 0xFF];
        c.stroke_arc(32, 32, 20, -180.0, 0.0, 3.0, col, LineCapStyle::Round, false);
        // 断言取径向距离 ≥1px 离内外描边界 [18.5,21.5] 的稳态点 (贝塞尔压平误差 ~0.05px)
        assert_eq!(px(&c, 32, 51), col, "弧底 (径向 19.5)");
        assert_eq!(px(&c, 12, 32), col, "弧左端 (径向 19.5)");
        assert_eq!(px(&c, 52, 32), col, "弧右端 (径向 19.5)");
        assert_eq!(px(&c, 9, 32), [0, 0, 0, 0], "左端外 (径向 22.5)");
        assert_eq!(px(&c, 32, 12), [0, 0, 0, 0], "上半圆位置无弧");
        assert_eq!(px(&c, 32, 32), [0, 0, 0, 0], "圆心无弧");
    }

    /// Java drawArc 方向 oracle: drawArc(0,90) 覆盖 12点→3点右上象限,
    /// drawArc(0,-90) 走 3点→6点右下短弧 (负 extent = 顺时针反向)
    #[test]
    fn stroke_arc_direction_quadrants() {
        let col = [0xFF, 0xFF, 0xFF, 0xFF];
        let mut c = PixCanvas::new(64, 64).unwrap();
        c.stroke_arc(32, 32, 20, 0.0, 90.0, 3.0, col, LineCapStyle::Round, false);
        assert_eq!(px(&c, 46, 18), col, "正 sweep 45° 中点 = 右上象限");
        assert_eq!(px(&c, 32, 12), col, "12 点端点");
        assert_eq!(px(&c, 18, 46), [0, 0, 0, 0], "左下象限无弧");
        assert_eq!(px(&c, 46, 46), [0, 0, 0, 0], "右下象限无弧 (正 sweep 不经 6 点)");
        let mut c2 = PixCanvas::new(64, 64).unwrap();
        c2.stroke_arc(32, 32, 20, 0.0, -90.0, 3.0, col, LineCapStyle::Round, false);
        assert_eq!(px(&c2, 46, 46), col, "负 sweep -45° 中点 = 右下象限 (顺时针)");
        assert_eq!(px(&c2, 32, 51), col, "6 点端点");
        assert_eq!(px(&c2, 46, 18), [0, 0, 0, 0], "右上象限无弧");
        assert_eq!(px(&c2, 32, 12), [0, 0, 0, 0], "12 点无弧");
    }

    /// sweep 边界语义: 零 extent 不绘制 (Java drawArc(start,0) oracle 0 像素);
    /// NaN/±inf 入参消毒不绘制不挂死 (Java int 入参无此态);
    /// |sweep|≥360 (含负向) = 整圆
    #[test]
    fn stroke_arc_zero_nonfinite_and_full_circle() {
        let mut c = PixCanvas::new(64, 64).unwrap();
        let col = [0xFF, 0xFF, 0xFF, 0xFF];
        c.stroke_arc(32, 32, 20, 45.0, 45.0, 3.0, col, LineCapStyle::Round, false);
        c.stroke_arc(32, 32, 20, f32::NAN, 90.0, 3.0, col, LineCapStyle::Round, false);
        c.stroke_arc(32, 32, 20, 0.0, f32::NAN, 3.0, col, LineCapStyle::Round, false);
        c.stroke_arc(32, 32, 20, f32::NEG_INFINITY, f32::INFINITY, 3.0, col, LineCapStyle::Round, false);
        assert!(c.pm.data().iter().all(|&b| b == 0), "零/非有限角度均无输出");
        let mut c2 = PixCanvas::new(64, 64).unwrap();
        c2.stroke_arc(32, 32, 20, 90.0, -270.0, 3.0, col, LineCapStyle::Round, false);
        assert_eq!(px(&c2, 32, 12), col, "负向 360 整圆: 12 点");
        assert_eq!(px(&c2, 32, 51), col, "6 点");
        assert_eq!(px(&c2, 12, 32), col, "9 点");
        assert_eq!(px(&c2, 52, 32), col, "3 点");
    }

    /// CAP_SQUARE (Java 裸 new BasicStroke(w) 默认): 端点沿切向外伸 width/2 平头,
    /// 与 Butt (无外伸) / Round (端点半圆) 的判别像素
    #[test]
    fn draw_line_cap_square_vs_butt_round() {
        let col = [0xFF, 0xFF, 0xFF, 0xFF];
        // 水平线 (10,10)-(20,10) 宽 4: 方帽外伸 2px → 覆盖域 x∈[8,22), y∈[8,12)
        let mut c = PixCanvas::new(32, 32).unwrap();
        c.draw_line_cap(10, 10, 20, 10, 4.0, col, LineCapStyle::Square, false);
        assert_eq!(px(&c, 9, 10), col, "左端方帽外伸");
        assert_eq!(px(&c, 21, 10), col, "右端方帽外伸");
        assert_eq!(px(&c, 8, 8), col, "方帽直角 (圆帽此处距帽心 2.12>2 为空)");
        assert_eq!(px(&c, 15, 8), col, "本体上缘行");
        assert_eq!(px(&c, 15, 12), [0, 0, 0, 0], "本体下缘外");
        assert_eq!(px(&c, 7, 10), [0, 0, 0, 0], "方帽外");
        let mut b = PixCanvas::new(32, 32).unwrap();
        b.draw_line_cap(10, 10, 20, 10, 4.0, col, LineCapStyle::Butt, false);
        assert_eq!(px(&b, 9, 10), [0, 0, 0, 0], "Butt 端点无外伸");
        assert_eq!(px(&b, 15, 10), col, "Butt 本体中线");
        let mut r = PixCanvas::new(32, 32).unwrap();
        r.draw_line_cap(10, 10, 20, 10, 4.0, col, LineCapStyle::Round, false);
        assert_eq!(px(&r, 8, 8), [0, 0, 0, 0], "圆帽无直角");
        assert_eq!(px(&r, 9, 10), col, "圆帽外伸 (距帽心 0.71<2)");
    }

    /// fillPath 三角形: 内部覆盖, 斜边外/上边外透明
    #[test]
    fn fill_path_triangle() {
        let mut c = PixCanvas::new(64, 64).unwrap();
        let col = [0xEE, 0x00, 0x77, 0xFF];
        c.fill_path(&[(10.0, 10.0), (50.0, 10.0), (10.0, 50.0)], col, false);
        assert_eq!(px(&c, 11, 11), col, "内部近直角顶");
        assert_eq!(px(&c, 20, 20), col, "斜边内侧 (中心和 41<60)");
        assert_eq!(px(&c, 40, 40), [0, 0, 0, 0], "斜边外侧 (中心和 81>60)");
        assert_eq!(px(&c, 49, 12), [0, 0, 0, 0], "上边外");
    }

    /// 双路对拍: PixCanvas 文本 vs Canvas 文本 (经 window.rs 预乘转换) 逐字节一致。
    /// 覆盖 aa 开/关 × 不透明/半透明色 × ASCII/CJK, 同字体同坐标。
    #[test]
    fn text_blit_matches_canvas() {
        let font = LoadedFont::new(std::path::Path::new(FONT), 24).unwrap();
        for &aa in &[true, false] {
            for &color in &[
                [255u8, 255, 255, 255],
                [0x12, 0x34, 0x56, 0xFF],
                [0x80, 0x60, 0x40, 0xC0],
            ] {
                let mut cv = Canvas::new(220, 120);
                let mut pc = PixCanvas::new(220, 120).unwrap();
                let texts = ["5", "M5 88", "表速度 1234", "0.-+", "W"];
                for (i, t) in texts.iter().enumerate() {
                    let (x, y) = (4, 26 + i as i32 * 18);
                    cv.draw_text(&font, x, y, t, color, aa);
                    pc.draw_text(&font, x, y, t, color, aa);
                }
                assert_eq!(
                    to_premul_bgra(&cv),
                    pc.to_premul_bgra(),
                    "aa={aa} color={color:?}"
                );
            }
        }
    }

    /// drawTextShade 双遍流 (黑影 +1,+1 → 本色): 直通伴随缓冲保证第二遍
    /// 合成域与 Canvas 一致, 两路逐字节相同 (LinearGauge/MarkedGauge
    /// drawTextShaded 的生产绘制顺序)
    #[test]
    fn text_shaded_two_pass_matches_canvas() {
        let font = LoadedFont::new(std::path::Path::new(FONT), 24).unwrap();
        let shade = [0x00, 0x00, 0x00, 0xFF]; // Application.colorShadeShape
        let color = [0xFF, 0xFF, 0xFF, 0xFF]; // Application.colorNum
        for &aa in &[true, false] {
            let mut cv = Canvas::new(160, 60);
            let mut pc = PixCanvas::new(160, 60).unwrap();
            let t = "875 表";
            cv.draw_text(&font, 11, 31, t, shade, aa);
            cv.draw_text(&font, 10, 30, t, color, aa);
            pc.draw_text(&font, 11, 31, t, shade, aa);
            pc.draw_text(&font, 10, 30, t, color, aa);
            assert_eq!(to_premul_bgra(&cv), pc.to_premul_bgra(), "aa={aa}");
        }
    }

    /// 形状上叠文本: 直通伴随缓冲失效 → 重构路径 (形状像素 un_premul 还原后合成)
    #[test]
    fn text_over_shape_rebuild() {
        let mut c = PixCanvas::new(120, 60).unwrap();
        c.fill_rect(0, 0, 120, 60, [80, 80, 80, 255]); // 不透明灰底
        let font = LoadedFont::new(std::path::Path::new(FONT), 24).unwrap();
        c.draw_text(&font, 10, 40, "7", [255, 255, 255, 255], true);
        // 不透明底上 fa=1 的笔画核心像素应为纯白 (预乘=直通)
        assert!(
            c.pm.data().chunks_exact(4).any(|p| p[0] == 255 && p[1] == 255 && p[2] == 255 && p[3] == 255),
            "文本笔画核心像素存在"
        );
        // 底色仍在 (字形外区域未被动过)
        assert_eq!(px(&c, 5, 5), [80, 80, 80, 255]);
    }

    /// clear: 尺寸重建 + 原地清零 + 非法尺寸拒改
    #[test]
    fn clear_resets_and_resizes() {
        let mut c = PixCanvas::new(20, 10).unwrap();
        c.fill_rect(0, 0, 20, 10, [255, 255, 255, 255]);
        c.clear(30, 8);
        assert_eq!((c.width(), c.height()), (30, 8));
        assert!(c.pm.data().iter().all(|&b| b == 0), "重建后清零");
        c.fill_rect(0, 0, 30, 8, [10, 20, 30, 40]);
        assert!(c.clear(30, 8), "同尺寸原地清零");
        assert!(c.pm.data().iter().all(|&b| b == 0));
        assert!(!c.clear(-1, 5), "非法尺寸拒绝");
        assert_eq!(c.width(), 30, "状态不变");
    }
}
