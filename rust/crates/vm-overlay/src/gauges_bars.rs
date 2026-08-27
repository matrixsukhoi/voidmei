//! gauges_bars: 条形 gauge 家族 C 类语义复刻
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | LinearGauge | ui/component/LinearGauge.java | 竖/横条 + 随值移动的刻度线与文本, 零 GC buffer→String 缓存 |
//! | LabeledLinearGauge | ui/component/LabeledLinearGauge.java | 前置标签 (label+value 同基线), 横向刻度线修正版 |
//! | SpeedRatioBar | ui/component/SpeedRatioBar.java | 速度比双色条 + 失速红区/马赫红线/锁舵刻度 |
//! | FlapAngleBar | ui/component/FlapAngleBar.java | 襟翼三色分区条 + 固定角度刻度 |
//!
//! 绘制目标 = render2d::PixCanvas; 颜色/坐标公式逐项对照 Java paint 逻辑
//! (关键处 // PORT: 注明 Java 行号)。Java char[] 零 GC buffer 模式在 Rust 侧
//! 统一为可复用 String 缓存 (update 仅在内容变化时重写)。
//!
//! 轴对齐整数端点线不走 tiny-skia 矢量光栅化, 按 Java 8 oracle 实测像素盒直填
//! (期望值逐像素钉死在 line_primitive_pixel_boxes):
//! - AA OFF (ANTIALIAS_OFF): 像素中心规则 — 像素中心落在 stroke 覆盖盒内才点亮。
//!   CAP_BUTT 宽 2 水平线 drawLine(xa,y,xb,y) = 行 y-1..y × 列 xa..xb-1
//!   (右端列不点亮, oracle: 20 列/40px); CAP_SQUARE 宽 2 竖线 = 列 tx-1..tx ×
//!   行 y0-1..y1 (方帽几何 [y0-1,y1+1] 光栅后下端行不点亮, oracle: 22 行);
//!   1px 线 = 恰该列/行, 端点含。
//! - AA ON (生产 graphAASetting 恒 ON, Application.java:102, 无运行时关闭路径):
//!   整数端点经 STROKE_NORMALIZE 规整到像素中心 (+0.5,+0.5 偏移), 宽 2 线呈
//!   3 行/列柔边 — 不透明色 a=128/255/128、端点列/行半覆盖、角点 1/4 (a=64,
//!   oracle: 21 列×3 行=63 非零像素) — 用覆盖率缩放 alpha 的解析盒复刻
//!   (cov_color, 等价 Java AA 的 SrcOver×coverage 合成)。
//!   1px 线规整后覆盖盒边界恰为整数像素边界 ([x,x+1]), AA 开关输出一致。
//! - drawRect 环: 负宽/负高整体不绘制 (oracle 0 像素); 零宽/零高退化 1px 线。

use crate::global_colors::colors;
use crate::font::LoadedFont;
use crate::render2d::PixCanvas;


/// Java Math.round(float): floor(x+0.5) (PORTING.md §2.3, Rust round 是半偶)
fn java_round_f32(x: f32) -> i32 {
    (x + 0.5).floor() as i32
}

/// Java (int) Math.round(double) 复合 (SpeedRatioBar.java:143)
fn java_round_f64_to_i32(x: f64) -> i32 {
    (x + 0.5).floor() as i32
}

/// Java Graphics.drawRect(x,y,w,h) + BasicStroke(1): 覆盖 x..x+w × y..y+h
/// (含端点) 的 1px 环。负宽或负高整体不绘制 (Java 8 oracle 实测 0 像素 —
/// "负尺寸时 4 条 drawLine 反向仍可见"的假设已证伪); 零宽/零高退化 1px 线
/// (drawRect 的 4 条边线中零长度段无输出, 剩两段共线: oracle drawRect(50,10,0,20)
/// = 列 50 行 10..30 的 1px 竖线; 双零则 4 段全零长度, 无输出)。
fn ring(cv: &mut PixCanvas, x: i32, y: i32, w: i32, h: i32, color: [u8; 4]) {
    if w < 0 || h < 0 {
        return; // PORT: Java drawRect 负宽/负高不绘制 (镜像归一化假设已证伪)
    }
    if w == 0 || h == 0 {
        if w == 0 && h > 0 {
            vline_1px(cv, x, y, y + h, color, false);
        } else if h == 0 && w > 0 {
            cv.fill_rect(x, y, w + 1, 1, color);
        }
        return;
    }
    let bw = w + 1;
    let bh = h + 1;
    cv.fill_rect(x, y, bw, 1, color); // 上边
    cv.fill_rect(x, y + h, bw, 1, color); // 下边
    if bh > 2 {
        cv.fill_rect(x, y + 1, 1, bh - 2, color); // 左边
        cv.fill_rect(x + w, y + 1, 1, bh - 2, color); // 右边
    }
}

/// BasicStroke(1, CAP_ROUND, JOIN_ROUND) 竖线: 列恰为 x, 行 y0..y1 端点含
/// (宽 1 圆帽半径 0.5 不外伸 — Java drawLine 整数端点像素级语义)。
/// aa=true 输出与 false 一致: 1px stroke 经 STROKE_NORMALIZE 规整后覆盖盒
/// 边界 ([x,x+1]×[y0,y1]) 恰为整数像素边界, 无柔边 (宽 ≥2 的线才有半像素
/// 柔边, 见 hline_butt2/vline_square2 的 AA ON 分支)。
fn vline_1px(cv: &mut PixCanvas, x: i32, y0: i32, y1: i32, color: [u8; 4], _aa: bool) {
    let (ya, yb) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    cv.fill_rect(x, ya, 1, yb - ya + 1, color);
}

/// LinearGauge.java:230-244 私有 drawRect 助手: shade 环 + fill 内芯。
/// flip_logic=true 为横向 gauge 的竖直刻度 (LinearGauge.java:176 调用):
/// drawRect(x+w, y, -w-1, h-1) 负宽 → 整体不绘制, fillRect 负宽同样不绘制 —
/// 该分隔线在 Java 中【完全不可见】(oracle 实测), 横向 LinearGauge 只剩
/// 条 + 条下方文本。
#[allow(clippy::too_many_arguments)] // 签名对齐 Java 私有 drawRect(g2d,x,y,w,h,shade,fill,flip)
fn gauge_rect(
    cv: &mut PixCanvas,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    shade: [u8; 4],
    fill: [u8; 4],
    flip_logic: bool,
) {
    if !flip_logic {
        // PORT: LinearGauge.java:235-237 drawRect(x,y,w-1,h-1)=w×h 环 + fillRect(x+1,y+1,w-2,h-2)
        ring(cv, x, y, w - 1, h - 1, shade);
        cv.fill_rect(x + 1, y + 1, w - 2, h - 2, fill);
    } else {
        // PORT: LinearGauge.java:240-242
        ring(cv, x + w, y, -w - 1, h - 1, shade);
        cv.fill_rect(x + 1 + w, y + 1, -w - 2, h - 2, fill); // 负高 → 不绘制
    }
}

/// 阴影双遍文本 (LinearGauge.drawTextShaded / UIBaseElements.__drawStringShade
/// drawFontShape=false 分支, Application.java:143): 影 (x+1,y+1) shade → 本色 (x,y)
#[allow(clippy::too_many_arguments)] // 对齐 Java drawTextShaded(g2d,x,y,s,f,c) + 显式 shade/aa
fn text_shaded(
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

/// AA 柔边像素的覆盖率缩放: Java AA 管线合成式 = SrcOver(源 alpha × 覆盖率),
/// 不透明色 oracle 值 cov=0.5 → a=128、cov=0.25 → a=64, 即 round(a·cov)
fn cov_color(color: [u8; 4], cov: f32) -> [u8; 4] {
    [color[0], color[1], color[2], ((color[3] as f32) * cov + 0.5) as u8]
}

/// BasicStroke(2, CAP_BUTT, JOIN_MITER) 水平线 (GraphicsUtil.createPreciseStroke(2))。
/// aa=false (ANTIALIAS_OFF, 中心规则): 覆盖盒 [xa,xb]×[y-1,y+1] → 列 xa..xb-1
/// (右端列不点亮; 1px 默认 stroke 的 Bresenham 端点含像素不适用于宽>1 的
/// strokedShape 路径, oracle: drawLine(5,15,25,15) 覆盖列 5..24 共 20 列),
/// 行 y-1..y。
/// aa=true (生产 graphAA 恒 ON): STROKE_NORMALIZE 规整到像素中心后覆盖盒
/// [xa+0.5,xb+0.5]×[y-0.5,y+1.5] → 3 行柔边: 行 y 全值, 行 y±1 半值,
/// 端点列 xa/xb 半覆盖, 四角 1/4 (oracle: 63 非零像素, 角点 a=64)。
fn hline_butt2(cv: &mut PixCanvas, x0: i32, x1: i32, y: i32, color: [u8; 4], aa: bool) {
    let (xa, xb) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    if xb <= xa {
        return; // 零长度 CAP_BUTT 线不绘制 (Java strokedShape 零长度段无输出)
    }
    if !aa {
        cv.fill_rect(xa, y - 1, xb - xa, 2, color);
        return;
    }
    let mid = xb - xa - 1; // 整列覆盖段 (端点列半覆盖)
    if mid > 0 {
        let soft = cov_color(color, 0.5);
        cv.fill_rect(xa + 1, y, mid, 1, color);
        cv.fill_rect(xa + 1, y - 1, mid, 1, soft);
        cv.fill_rect(xa + 1, y + 1, mid, 1, soft);
    }
    let soft = cov_color(color, 0.5);
    let corner = cov_color(color, 0.25);
    for cx in [xa, xb] {
        cv.fill_rect(cx, y, 1, 1, soft);
        cv.fill_rect(cx, y - 1, 1, 1, corner);
        cv.fill_rect(cx, y + 1, 1, 1, corner);
    }
}

/// 裸 BasicStroke(2) (默认 CAP_SQUARE) 竖线 (FlapAngleBar.java:110-125 tick)。
/// aa=false (中心规则): 覆盖盒 [tx-1,tx+1]×[ya-1,yb+1] → 列 tx-1..tx, 行
/// ya-1..yb (方帽两端各外伸 1px 的几何经中心规则光栅后上端外伸、下端不外伸,
/// oracle: drawLine(40,5,40,25) 覆盖列 39..40、行 4..25 共 22 行)。
/// aa=true: 覆盖盒 [tx-0.5,tx+1.5]×[ya-0.5,yb+1.5] → 3 列柔边: 列 tx 全值,
/// 列 tx±1 半值, 端行 ya-1/yb+1 半覆盖, 角点 1/4 (oracle: 行 4..26 端行半透明)。
fn vline_square2(cv: &mut PixCanvas, tx: i32, y0: i32, y1: i32, color: [u8; 4], aa: bool) {
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

// ---------------------------------------------------------------------------
// LinearGauge
// ---------------------------------------------------------------------------

/// 通用条形 gauge (LinearGauge.java:10)。
/// Java public 字段直译为 pub; value_color=None → 用 colors().num (Java:127)。
pub struct LinearGauge {
    pub label: String,
    pub max_value: i32,
    pub cur_value: i32,
    pub display_value: String,
    pub vertical: bool,
    /// true = 刻度在竖条右侧 (LinearGauge.java:17)
    pub tick_on_right: bool,
    /// 值色覆盖 (Java valueColor, MiniHUD ThrottleBar 注入 HUDData.throttleColor)
    pub value_color: Option<[u8; 4]>,
    // 风格上下文缓存 (Java:84-89 setStyleContext)
    length_cache: i32,
    thickness_cache: i32,
    // 脏检查: 值/风格变化置位, draw 后清零
    dirty: bool,
}

impl LinearGauge {
    /// Java:28 构造 (tickOnRight=false 默认)
    pub fn new(label: &str, max_value: i32, vertical: bool) -> Self {
        Self::with_tick_on_right(label, max_value, vertical, false)
    }

    /// Java:32 构造
    pub fn with_tick_on_right(
        label: &str,
        max_value: i32,
        vertical: bool,
        tick_on_right: bool,
    ) -> Self {
        LinearGauge {
            label: label.to_string(),
            max_value,
            cur_value: 0,
            display_value: String::new(),
            vertical,
            tick_on_right,
            value_color: None,
            length_cache: 100, // Java:79-80 默认
            thickness_cache: 10,
            dirty: true,
        }
    }

    pub fn set_style_context(&mut self, length: i32, thickness: i32) {
        self.length_cache = length;
        self.thickness_cache = thickness;
        self.dirty = true;
    }

    /// Java:40 update(value, displayValue)。返回值是否变化 (脏检查)。
    /// 契约: 仅覆盖 value/displayValue — Java onDataUpdate 同帧还会注入
    /// valueColor/displayValue 等字段 (MiniHUD ThrottleBar, Java:94-103), Rust 侧
    /// 那些字段走下方 set_* 置脏 setter; 直改 pub 字段不置脏, 按 is_dirty() 门控
    /// draw 的调用方必须用 setter (或无条件 draw, renderers.rs 现状)。
    pub fn update(&mut self, value: i32, display_value: &str) -> bool {
        let changed = value != self.cur_value || display_value != self.display_value;
        self.cur_value = value;
        self.display_value.clear();
        self.display_value.push_str(display_value);
        self.dirty |= changed;
        changed
    }

    /// 值色注入 (Java valueColor 覆盖, WEP 色等): 变化置脏
    pub fn set_value_color(&mut self, c: Option<[u8; 4]>) {
        if self.value_color != c {
            self.value_color = c;
            self.dirty = true;
        }
    }

    /// 横竖向切换 (Java GaugeField 每帧赋 gauge.vertical): 变化置脏
    pub fn set_vertical(&mut self, v: bool) {
        if self.vertical != v {
            self.vertical = v;
            self.dirty = true;
        }
    }

    /// 标签/量程/刻度侧切换: 变化置脏 (同 W3 契约)
    pub fn set_label(&mut self, l: &str) {
        if self.label != l {
            self.label = l.to_string();
            self.dirty = true;
        }
    }

    pub fn set_max_value(&mut self, m: i32) {
        if self.max_value != m {
            self.max_value = m;
            self.dirty = true;
        }
    }

    pub fn set_tick_on_right(&mut self, t: bool) {
        if self.tick_on_right != t {
            self.tick_on_right = t;
            self.dirty = true;
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Java:124 pixVal = Math.round((float)curValue * length / maxValue), max<=0 → 0
    fn pix_value(&self, length: i32) -> i32 {
        if self.max_value > 0 {
            // PORT: Java (float) 提升链保持 f32 (§2.12)
            java_round_f32(self.cur_value as f32 * length as f32 / self.max_value as f32)
        } else {
            0
        }
    }

    /// Java:189-228 drawBar: shade 1px 环 + colorNum 填充 (竖条自底向上)
    #[allow(clippy::too_many_arguments)] // 对齐 Java drawBar(g2d,x,y,w,h,val,shade,fill,isVert)
    fn draw_bar(
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        val: i32,
        shade: [u8; 4],
        fill: [u8; 4],
        is_vert: bool,
    ) {
        // PORT: Java:196/222 drawRect(x,y,w-1,h-1) — 自 (x,y) 向下/右生长
        ring(cv, x, y, w - 1, h - 1, shade);
        if is_vert {
            let val_h = if val > h { h } else { val };
            if val_h >= 0 {
                // PORT: Java:217 fillRect(x+1, y+h-1-valH, w-2, valH) — 底部对齐内芯
                cv.fill_rect(x + 1, y + h - 1 - val_h, w - 2, val_h, fill);
            }
        } else {
            let val_w = if val > w { w } else { val };
            if val_w >= 0 {
                // PORT: Java:226 fillRect(x+1, y+1, valW-2, h-2) — valW<2 时负宽不绘制
                cv.fill_rect(x + 1, y + 1, val_w - 2, h - 2, fill);
            }
        }
    }

    /// Java:106-108 draw(g2d, x, y) — 用缓存风格。fontLabel 未参与绘制 (Java 同)。
    pub fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, font_num: &LoadedFont, aa: bool) {
        let (len, th) = (self.length_cache, self.thickness_cache);
        self.draw_sized(cv, x, y, len, th, font_num, aa);
    }

    /// 完整参数版 (Java:114 draw(g2d, x, y, length, thickness, ...))
    #[allow(clippy::too_many_arguments)] // 对齐 Java draw(g2d,x,y,length,thickness,fontLabel,fontNum)
    pub fn draw_sized(
        &mut self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        length: i32,
        thickness: i32,
        font_num: &LoadedFont,
        aa: bool,
    ) {
        let pix_val = self.pix_value(length);
        // PORT: Java:127-128 值色覆盖 / shade 恒为 colorShadeShape; 条填充恒 colorNum
        let c = self.value_color.unwrap_or(colors().num);
        let shade = colors().shade_shape;

        if self.vertical {
            // (x,y) = 组合区域左上 (Java:133)
            let text_width = font_num.measure(&self.display_value);
            let label_spacing = 2; // Java:135

            // PORT: Java:139 分隔线 y = 底 - 1 - pixVal
            // (极端 curValue 下 i32 加减 Java 静默回绕 / Rust panic §2.2, 不可达, 备查)
            let sep_y = y + length - 1 - pix_val;

            if self.tick_on_right {
                // PORT: Java:141-154 条在左, 刻度(分隔线+文本)在右
                Self::draw_bar(cv, x, y, thickness, length, pix_val, shade, colors().num, true);
                let total_width = thickness + label_spacing + text_width;
                gauge_rect(cv, x, sep_y, total_width, 3, shade, c, false);
                text_shaded(
                    cv, font_num, x + thickness + label_spacing, sep_y - 1,
                    &self.display_value, c, shade, aa,
                );
            } else {
                // PORT: Java:155-169 刻度(文本+分隔线)在左, 条在右 (默认)
                let bar_x = x + text_width + label_spacing;
                Self::draw_bar(cv, bar_x, y, thickness, length, pix_val, shade, colors().num, true);
                let total_width = text_width + label_spacing + thickness;
                gauge_rect(cv, x, sep_y, total_width, 3, shade, c, false);
                text_shaded(cv, font_num, x, sep_y - 1, &self.display_value, c, shade, aa);
            }
        } else {
            // PORT: Java:170-180 横条 + 竖直分隔线(flip 环在条上方) + 条下方文本
            Self::draw_bar(cv, x, y, length, thickness, pix_val, shade, colors().num, false);
            gauge_rect(cv, x + pix_val - 2, y, 3, -thickness - font_num.size, shade, c, true);
            text_shaded(
                cv, font_num, x + pix_val, y + thickness + font_num.size,
                &self.display_value, c, shade, aa,
            );
        }
        self.dirty = false;
    }
}

// ---------------------------------------------------------------------------
// LabeledLinearGauge (组合 + 委托, PORTING.md §1 extends 映射)
// ---------------------------------------------------------------------------

/// 带前置标签的 LinearGauge (LabeledLinearGauge.java:14)。
/// 标签与数值同基线相连: [label][value] (Java:38-45)。
pub struct LabeledLinearGauge {
    pub gauge: LinearGauge,
}

impl LabeledLinearGauge {
    /// Java:16
    pub fn new(label: &str, max_value: i32, vertical: bool) -> Self {
        LabeledLinearGauge {
            gauge: LinearGauge::new(label, max_value, vertical),
        }
    }

    /// label+value 合成宽度 (Java:32-34 getValueWidth 覆写)
    fn value_width(&self, font_num: &LoadedFont) -> i32 {
        font_num.measure(&self.gauge.label) + font_num.measure(&self.gauge.display_value)
    }

    /// 标签+数值双段阴影文本 (Java:38-45 drawValueText 覆写)
    #[allow(clippy::too_many_arguments)] // 对齐 Java 覆写签名 drawValueText(g2d,x,y,f,c)
    fn draw_value_text(
        &self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        font_num: &LoadedFont,
        c: [u8; 4],
        shade: [u8; 4],
        aa: bool,
    ) {
        text_shaded(cv, font_num, x, y, &self.gauge.label, c, shade, aa);
        let label_w = font_num.measure(&self.gauge.label);
        text_shaded(cv, font_num, x + label_w, y, &self.gauge.display_value, c, shade, aa);
    }

    /// Java:48-86 draw 覆写 (竖向走基类逻辑 + 前置标签宽度; 横向自绘修正分隔线)。
    /// Java:50 的 new Font("Sarasa Mono SC",...) 由调用方直接传等宽字体实现。
    #[allow(clippy::too_many_arguments)] // 对齐 Java draw(g2d,x,y,length,thickness,fontLabel,fontValue)
    pub fn draw(
        &mut self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        length: i32,
        thickness: i32,
        font_num: &LoadedFont,
        aa: bool,
    ) {
        let g = &self.gauge;
        if g.vertical {
            // PORT: Java:53-55 基类逻辑, 文本宽度换 label+value 合成 (条与分隔线右移)
            let pix_val = g.pix_value(length);
            let c = g.value_color.unwrap_or(colors().num);
            let shade = colors().shade_shape;
            let text_width = self.value_width(font_num);
            let label_spacing = 2;
            let sep_y = y + length - 1 - pix_val;
            if g.tick_on_right {
                LinearGauge::draw_bar(cv, x, y, thickness, length, pix_val, shade, colors().num, true);
                let total_width = thickness + label_spacing + text_width;
                gauge_rect(cv, x, sep_y, total_width, 3, shade, c, false);
                self.draw_value_text(
                    cv, x + thickness + label_spacing, sep_y - 1, font_num, c, shade, aa,
                );
            } else {
                let bar_x = x + text_width + label_spacing;
                LinearGauge::draw_bar(cv, bar_x, y, thickness, length, pix_val, shade, colors().num, true);
                let total_width = text_width + label_spacing + thickness;
                gauge_rect(cv, x, sep_y, total_width, 3, shade, c, false);
                self.draw_value_text(cv, x, sep_y - 1, font_num, c, shade, aa);
            }
        } else {
            // PORT: Java:57-85 横向修正版
            let pix_val = g.pix_value(length);
            let c = colors().num; // Java:64 恒 colorNum (忽略 valueColor)
            let shade_shadow = colors().shade_shape;

            // 1. 条背景+边框 (Java:68; drawBarFixed 横向分支与基类横向 drawBar 一致)
            LinearGauge::draw_bar(cv, x, y, length, thickness, pix_val, shade_shadow, colors().num, false);

            // 2. 竖直分隔线: 条顶延伸到文本底 (Java:72-80)
            //    sepHeight = thickness + fontSize + 2; 影线 x+pixVal+1 / 主线 x+pixVal
            //    PORT: Java 此处未 setStroke, 线宽承袭调用链遗留 stroke — 唯一消费链
            //    GaugeField→EngineControlOverlay/LinearGaugeRenderer 的绘制序中前置
            //    gauge 恒 set 1px 族 stroke, 首个 gauge 前为 Graphics2D 默认
            //    BasicStroke(1); oracle 实测两种遗留 stroke 下 1px 竖线输出一致
            //    (列 x, 行 y0..y1 端点含硬边)。Rust 组装层若绘制顺序变化需回访此处
            let sep_height = thickness + font_num.size + 2;
            vline_1px(cv, x + pix_val + 1, y, y + sep_height, shade_shadow, aa);
            vline_1px(cv, x + pix_val, y, y + sep_height, c, aa);

            // 3. label+value 合成文本, 条下方 (Java:84)
            self.draw_value_text(
                cv, x + pix_val, y + thickness + font_num.size, font_num, c, shade_shadow, aa,
            );
        }
        self.gauge.dirty = false;
    }
}

// ---------------------------------------------------------------------------
// SpeedRatioBar
// ---------------------------------------------------------------------------

/// 速度比竖条 (SpeedRatioBar.java:23)。0=底 1=顶, 全高代表 VNE。
pub struct SpeedRatioBar {
    width: i32,
    height: i32,
    // 数据态 (Java:39-43)
    speed_ratio: f64,
    stall_ratio: f64,
    unit_mach_ratio: f64,
    aileron_lock_ratio: f64,
    rudder_lock_ratio: f64,
    dirty: bool,
}

/// Java:183-189 clamp(v) = 0..1 (NaN 穿透: 两比较均 false 返回原值, 与 Java 一致)
#[allow(clippy::manual_clamp)] // 显式分支复刻 Java clamp 私有方法 (NaN 语义钉死)
fn clamp01(v: f64) -> f64 {
    if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

impl SpeedRatioBar {
    pub fn new() -> Self {
        SpeedRatioBar {
            width: 10, // Java:26-27 默认
            height: 100,
            speed_ratio: 0.0,
            stall_ratio: 0.0,
            unit_mach_ratio: 0.0,
            aileron_lock_ratio: 0.0,
            rudder_lock_ratio: 0.0,
            dirty: true,
        }
    }

    /// Java:63 setStyleContext (tick_font 由 draw 参数传入, Java 允许 null)
    pub fn set_style_context(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
        self.dirty = true;
    }

    /// Java:70-78 onDataUpdate 五比值注入。返回是否变化 (脏检查)。
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        speed_ratio: f64,
        stall_ratio: f64,
        unit_mach_ratio: f64,
        aileron_lock_ratio: f64,
        rudder_lock_ratio: f64,
    ) -> bool {
        let changed = self.speed_ratio != speed_ratio
            || self.stall_ratio != stall_ratio
            || self.unit_mach_ratio != unit_mach_ratio
            || self.aileron_lock_ratio != aileron_lock_ratio
            || self.rudder_lock_ratio != rudder_lock_ratio;
        self.speed_ratio = speed_ratio;
        self.stall_ratio = stall_ratio;
        self.unit_mach_ratio = unit_mach_ratio;
        self.aileron_lock_ratio = aileron_lock_ratio;
        self.rudder_lock_ratio = rudder_lock_ratio;
        self.dirty |= changed;
        changed
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Java:96/105/174 lockY = y + height - Math.round((float)(height * ratio))
    /// PORT: 极端 |ratio| 下 java_round_f32 饱和到 i32::MAX/MIN 后的加减在
    /// Java 静默回绕 (§2.2), Rust debug 会 panic — 真实遥测幅度不可达, 记录备查
    fn ratio_y(&self, y: i32, ratio: f64) -> i32 {
        // PORT: Java double 乘法后 (float) 强转, 再 Math.round(float)
        y + self.height - java_round_f32((self.height as f64 * ratio) as f32)
    }

    /// Java:81-181 draw。绘制顺序即图层序 (锁舵刻度先画被条盖住右半, 红区最后)。
    pub fn draw(
        &mut self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        tick_font: Option<&LoadedFont>,
        aa: bool,
    ) {
        let w = self.width;
        let h = self.height;

        // 1. 副翼锁舵刻度 (左) (Java:95-101)
        if self.aileron_lock_ratio > 0.0 && self.aileron_lock_ratio < 1.0 {
            let lock_y = self.ratio_y(y, self.aileron_lock_ratio);
            hline_butt2(cv, x - 4, x + w / 2, lock_y, colors().num, aa);
        }

        // 2. 方向舵锁舵刻度 (右) (Java:104-110)
        if self.rudder_lock_ratio > 0.0 && self.rudder_lock_ratio < 1.0 {
            let lock_y = self.ratio_y(y, self.rudder_lock_ratio);
            hline_butt2(cv, x + w / 2, x + w + 4, lock_y, colors().num, aa);
        }

        // 3. 背景 colorNum 全条 = 剩余范围 (Java:113-114)
        cv.fill_rect(x, y, w, h, colors().num);

        // 4. shade 填充 = 当前速度比 (底→值) (Java:117-121)
        let green_h = java_round_f32((h as f64 * clamp01(self.speed_ratio)) as f32);
        if green_h > 0 {
            cv.fill_rect(x, y + h - green_h, w, green_h, colors().shade_shape);
        }

        // 4.5 速度比刻度 (左, 跨文本区到条右缘) + 右对齐数值 (Java:123-159)
        {
            let tick_y = self.ratio_y(y, clamp01(self.speed_ratio));
            let template_width = tick_font.map(|f| f.measure("888")).unwrap_or(0);
            let tick_extend = 4;
            let label_spacing = 2;
            let tick_start_x = x - tick_extend - label_spacing - template_width;
            let tick_width = template_width + label_spacing + tick_extend + w;
            hline_butt2(cv, tick_start_x, tick_start_x + tick_width - 1, tick_y, colors().num, aa);

            if let Some(f) = tick_font {
                // PORT: Java:143 (int) Math.round(speedRatio * 100)
                let display_value = java_round_f64_to_i32(self.speed_ratio * 100.0);
                let value_str = display_value.to_string();
                let actual_text_width = f.measure(&value_str);
                let text_right_edge = x - tick_extend - label_spacing;
                let text_x = text_right_edge - actual_text_width;
                let text_y = tick_y - 3; // 刻度上方 (Java:150)
                text_shaded(cv, f, text_x, text_y, &value_str, colors().num, colors().shade_shape, aa);
            }
        }

        // 5. 红区 (失速): 右侧半宽覆盖条 (Java:161-170)
        let stall_h = java_round_f32((h as f64 * clamp01(self.stall_ratio)) as f32);
        if stall_h > 0 {
            let mut stall_w = w / 2;
            if stall_w < 2 {
                stall_w = 2; // Java:166-167 最小宽保护
            }
            cv.fill_rect(x + w - stall_w, y + h - stall_h, stall_w, stall_h, colors().warning);
        }

        // 6. 马赫单位红线 (Java:172-178)
        if self.unit_mach_ratio > 0.0 && self.unit_mach_ratio < 1.0 {
            let mach_y = self.ratio_y(y, self.unit_mach_ratio);
            hline_butt2(cv, x, x + w, mach_y, colors().warning, aa);
        }
        // 无边框 (Java:180, 对齐 FlapAngleBar 风格)
        self.dirty = false;
    }
}

impl Default for SpeedRatioBar {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// FlapAngleBar
// ---------------------------------------------------------------------------

/// 襟翼角度三色条 (FlapAngleBar.java:15)。刻度固定 {20,33,60,100}, 满刻度 125。
pub struct FlapAngleBar {
    total_width: i32,
    bar_height: i32,
    current_angle: f64,
    max_safe_angle: f64,
    display_text: String,
    dirty: bool,
}

/// 刻度位置 (Java:32)
const TICK_POSITIONS: [i32; 4] = [20, 33, 60, 100];
/// 满刻度 (Java:33)
const MAX_SCALE: i32 = 125;

/// Java String.format("%3.0f") 语义: HALF_UP 舍入 0 位小数 (按精确十进制值,
/// 不能用 v+0.5 — f64 中 0.49999999999999994+0.5 会进到 1.0, Java oracle 实测
/// 输出 "  0"), 负号对舍到零的结果保留 (oracle: -0.0 → "-0"), 右对齐宽 3 空格补,
/// NaN → "NaN" (Java Formatter 输出)
fn fmt_pct3(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    // 负号判定含 -0.0 (Java Formatter 对负零输出 "-0", v < 0.0 对 -0.0 为 false)
    let neg = v < 0.0 || (v == 0.0 && v.is_sign_negative());
    // HALF_UP (远离零): 小数部分与 0.5 的比较取 v-floor(v) 的精确 f64 值
    let m = v.abs();
    let f = m.floor();
    let r = if m - f >= 0.5 { f + 1.0 } else { f };
    let mut s = format!("{}", r as i64);
    if neg {
        s.insert(0, '-');
    }
    while s.len() < 3 {
        s.insert(0, ' ');
    }
    s
}

impl FlapAngleBar {
    pub fn new() -> Self {
        FlapAngleBar {
            total_width: 0,
            bar_height: 0,
            current_angle: 0.0,
            max_safe_angle: 100.0,      // Java:37
            display_text: "  0/100".to_string(), // Java:38
            dirty: true,
        }
    }

    /// Java:53 setStyleContext
    pub fn set_style_context(&mut self, total_width: i32, bar_height: i32) {
        self.total_width = total_width;
        self.bar_height = bar_height;
        self.dirty = true;
    }

    /// Java:60-67 onDataUpdate: 角度对 + "%3.0f/%3.0f" 显示文本
    pub fn update(&mut self, current_angle: f64, max_safe_angle: f64) -> bool {
        let changed = current_angle != self.current_angle || max_safe_angle != self.max_safe_angle;
        self.current_angle = current_angle;
        self.max_safe_angle = max_safe_angle;
        self.display_text = format!("{}/{}", fmt_pct3(current_angle), fmt_pct3(max_safe_angle));
        self.dirty |= changed;
        changed
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn display_text(&self) -> &str {
        &self.display_text
    }

    /// Java:70-144 draw。font=None 直接返回 (Java:71-72)。
    pub fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, font: Option<&LoadedFont>, aa: bool) {
        let font = match font {
            Some(f) => f,
            None => return, // PORT: Java:71-72 font==null 不绘制
        };
        let total_width = self.total_width;
        let bar_height = self.bar_height;

        // 1. 居中文本 (ascent 基线) (Java:75-82)
        let ascent = font.metrics().ascent;
        let text_y = y + ascent;
        let str_width = font.measure(&self.display_text);
        // PORT: Java:81 int 除法向零截断 (strWidth 超宽时整体左移)
        let str_x = x + (total_width - str_width) / 2;
        text_shaded(cv, font, str_x, text_y, &self.display_text, colors().num, colors().shade_shape, aa);

        // 条位于文本下方 (字号近似行高) (Java:84-85)
        let bar_y = y + font.size + 2;

        // 三区宽度 (Java:87-102): used=已用(shade), margin=安全裕度(colorNum), overspeed=超速(warning)
        // PORT: Java:92/96 (int)(double) 截断
        let mut used_width = (self.current_angle * total_width as f64 / MAX_SCALE as f64) as i32;
        let margin_width = if self.current_angle <= self.max_safe_angle {
            // 正常: 有安全裕度
            (self.max_safe_angle * total_width as f64 / MAX_SCALE as f64) as i32 - used_width
        } else {
            // 超限: 无裕度, 红色为右侧剩余
            0
        };
        let mut overspeed_width = total_width - used_width - margin_width;

        // 边界保护 (Java:104-107)
        used_width = used_width.max(0);
        let margin_width = margin_width.max(0);
        overspeed_width = overspeed_width.max(0);

        // 刻度: colorLabel, 裸 BasicStroke(2) 方帽 (Java:109-125)
        // PORT: Java:120-121 tx = x + tick*totalWidth/MAX_SCALE 是 int 算术 (整除);
        // 100 刻度全高, 其余 1/4 高; 线自 barY-ext-2 到 barY (先于分区绘制, 与条重叠段被覆盖)
        // PORT: tick*total_width 在极端 total_width 下 Java 静默回绕 / Rust panic (§2.2),
        // 真实布局幅度不可达, 记录备查
        for &tick in &TICK_POSITIONS {
            let tx = x + tick * total_width / MAX_SCALE;
            let ext = if tick == 100 { bar_height } else { bar_height / 4 };
            vline_square2(cv, tx, bar_y - ext - 2, bar_y, colors().label, aa);
        }

        // 已用区 (0→current, shade) (Java:127-131)
        if used_width > 0 {
            cv.fill_rect(x, bar_y, used_width, bar_height, colors().shade_shape);
        }
        // 安全裕度区 (current→maxSafe, colorNum) (Java:133-137)
        if margin_width > 0 {
            cv.fill_rect(x + used_width, bar_y, margin_width, bar_height, colors().num);
        }
        // 超速区 (右侧剩余, warning) (Java:139-143)
        if overspeed_width > 0 {
            cv.fill_rect(x + used_width + margin_width, bar_y, overspeed_width, bar_height, colors().warning);
        }
        self.dirty = false;
    }
}

impl Default for FlapAngleBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &str = "../../../fonts/sarasa-mono-sc-bold.ttf";

    fn font() -> LoadedFont {
        LoadedFont::new(std::path::Path::new(FONT), 24).unwrap()
    }

    /// 读预乘 RGBA 像素 (与 render2d 测试同约定; 断言走 alpha 通道 —
    /// 同族色 RGB 相同仅 alpha 不同: num=240 label=166 warning=100 shade=42)
    fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
        let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
        [d[0], d[1], d[2], d[3]]
    }

    fn a(c: &PixCanvas, x: i32, y: i32) -> u8 {
        px(c, x, y)[3]
    }

    /// Java2D SrcOver 直通域合成后的 alpha (两层叠色的期望值)。
    /// 半透明层叠在 Java 同样加深 alpha (shade 叠 shade 等), 此处按同式计算;
    /// tiny-skia 预乘取整路径存在 ±1-2 LSB 系统差 (render2d 头注), 用容差比较。
    fn src_over_a(fg: u8, bg: u8) -> u8 {
        let fa = fg as f32 / 255.0;
        let fda = bg as f32 / 255.0;
        ((fa + fda * (1.0 - fa)) * 255.0 + 0.5) as u8
    }

    fn assert_a_close(actual: u8, expected: u8, what: &str) {
        assert!(
            (actual as i32 - expected as i32).abs() <= 2,
            "{what}: alpha {actual} 期望 ~{expected} (SrcOver 叠色 ±2 LSB)"
        );
    }

    /// 竖向 LinearGauge (默认 tick 在左): 条右置、填充自底向上、随值分隔线。
    /// pixVal = round(55*120/110) = 60; 填充行 = y+h-1-60 .. y+h-2 (valH 行)。
    #[test]
    fn linear_gauge_vertical_fill_and_separator() {
        let f = font();
        let mut g = LinearGauge::new("T", 110, true);
        g.set_style_context(120, 8);
        assert!(g.update(55, "55"));

        let mut cv = PixCanvas::new(140, 160).unwrap();
        g.draw(&mut cv, 20, 10, &f, false);

        let text_w = f.measure("55");
        let bar_x = 20 + text_w + 2; // Java:157 barX = x + textWidth + labelSpacing
        // 填充: 列 bar_x+1..bar_x+6 (w-2), 行 69..128; 129 为底边框行 (shade)
        assert_eq!(a(&cv, bar_x + 1, 72), 240, "填充体 (分隔线覆盖区之下)");
        assert_eq!(a(&cv, bar_x + 1, 128), 240, "填充底");
        assert_eq!(a(&cv, bar_x + 1, 129), 42, "底边框行 shade");
        assert_eq!(a(&cv, bar_x + 1, 68), 0, "填充上方透明");
        // 边框环: 列 bar_x / bar_x+7
        assert_eq!(a(&cv, bar_x, 10), 42, "左上边框 shade");
        assert_eq!(a(&cv, bar_x + 7, 129), 42, "右下边框 shade");
        // 分隔线 3px 环 + 1px 值色内芯: sepY = y+length-1-pixVal = 69
        let sep_y = 10 + 120 - 1 - 60;
        let total_w = text_w + 2 + 8;
        assert_eq!(a(&cv, 20, sep_y), 42, "分隔线环上边 shade");
        // 环右下角叠在条右边框列上 (shade 叠 shade → SrcOver 加深, Java 同)
        assert_a_close(a(&cv, 20 + total_w - 1, sep_y + 2), src_over_a(42, 42), "分隔线环右下");
        assert_eq!(a(&cv, 21, sep_y + 1), 240, "分隔线内芯值色");
        assert_eq!(a(&cv, 45, sep_y - 1), 0, "分隔线上方 (文本影与条间隙列)");
        assert_eq!(a(&cv, 45, sep_y + 3), 0, "分隔线下方");
    }

    /// 竖向越界钳制: curValue > maxValue → valH 钳到 h, 填充自条上方 1px 起
    /// (Java:202 valH = min(val,h) 后 fillRect 上探 y+h-1-valH, 行为如此)。
    #[test]
    fn linear_gauge_vertical_clamp_over_range() {
        let f = font();
        let mut g = LinearGauge::new("T", 100, true);
        g.set_style_context(80, 8);
        g.update(250, "250");
        let mut cv = PixCanvas::new(140, 120).unwrap();
        g.draw(&mut cv, 20, 10, &f, false);
        let bar_x = 20 + f.measure("250") + 2;
        // pixVal=200 → valH 钳 80 → 填充行 9..88 (9 = y+h-1-80, 高出条顶 1px)
        assert_eq!(a(&cv, bar_x + 1, 9), 240, "填充顶上探 1px (Java 行为)");
        assert_eq!(a(&cv, bar_x + 1, 11), 240, "条内填充");
        assert_eq!(a(&cv, bar_x + 1, 8), 0, "上探之外透明");
    }

    /// 横向 LinearGauge: 填充列截断 (valW-2)、flip 分隔线【不渲染】(Java
    /// drawRect 负宽 oracle 0 像素)、文本在条下方 — 只剩条 + 条下文本。
    #[test]
    fn linear_gauge_horizontal_separator_invisible() {
        let f = font();
        let mut g = LinearGauge::new("T", 100, false);
        g.set_style_context(100, 6);
        g.update(50, "50");
        let mut cv = PixCanvas::new(140, 100).unwrap();
        g.draw(&mut cv, 10, 20, &f, false);

        // pixVal=50 → 填充列 11..58 (valW-2=48), 行 21..24 (h-2)
        assert_eq!(a(&cv, 11, 22), 240, "填充左端");
        assert_eq!(a(&cv, 58, 24), 240, "填充右端");
        assert_eq!(a(&cv, 60, 22), 0, "填充右外 (valW-2 截断)");
        // flip 分隔线 (LinearGauge.java:176): drawRect(x+pixVal-2, y, 3, -thickness-fontSize)
        // = drawRect(58, 20, -4, ·) 负宽 → Java 整体不绘制 (oracle 实测 0 像素)。
        // 条顶边框行 y=20 只有 shade 单层, 无 shade 叠 shade 加深, 分隔线区域
        // (列 57..61) 与条内无异。
        assert_eq!(a(&cv, 57, 20), 42, "条顶边框单层 shade (无分隔线叠色)");
        assert_eq!(a(&cv, 61, 20), 42, "条顶边框单层 shade");
        assert_eq!(a(&cv, 59, 20), 42, "条顶边框单层 shade");
        assert_eq!(a(&cv, 59, 21), 0, "分隔线位置无向上延伸 (Java 不可见)");
        assert_eq!(a(&cv, 50, 21), 240, "条顶内填充体");
        // 文本基线 y+thickness+fontSize = 50 → 笔画像素存在 (alpha 240)
        assert!(
            cv.pixmap().data().chunks_exact(4).any(|p| p[3] == 240),
            "文本笔画存在"
        );
    }

    /// LabeledLinearGauge 横向: 主线列恰 x+pixVal / 影线列 x+pixVal+1,
    /// 自条顶 y 延伸到 y+sepHeight (thickness+fontSize+2), 1px 精确列。
    #[test]
    fn labeled_linear_gauge_horizontal_separator_lines() {
        let f = font();
        let mut g = LabeledLinearGauge::new("RPM", 100, false);
        g.gauge.update(40, "88");
        let mut cv = PixCanvas::new(160, 100).unwrap();
        g.draw(&mut cv, 10, 20, 100, 6, &f, false);

        let pix_val = 40;
        let sep_height = 6 + 24 + 2;
        // 条顶行 (20) 是条边框 shade: 主线 colorNum 叠 shade / 影线 shade 叠 shade
        assert_a_close(a(&cv, 10 + pix_val, 20), src_over_a(240, 42), "主线条顶");
        assert_eq!(a(&cv, 10 + pix_val, 20 + sep_height), 240, "主线尾端");
        assert_a_close(a(&cv, 10 + pix_val + 1, 20), src_over_a(42, 42), "影线条顶");
        assert_eq!(a(&cv, 10 + pix_val - 1, 22), 0, "主线左侧");
        assert_eq!(a(&cv, 10 + pix_val + 2, 22), 0, "影线右侧");
        // 填充: valW=40 → 列 11..48
        assert_eq!(a(&cv, 11, 22), 240, "填充左端");
        assert_eq!(a(&cv, 48, 24), 240, "填充右端");
        assert_eq!(a(&cv, 49, 22), 0, "填充右外");
    }

    /// LabeledLinearGauge 竖向: 标签参与总宽 → 条相对无标签版右移 labelW。
    #[test]
    fn labeled_linear_gauge_vertical_label_offsets_bar() {
        let f = font();
        let mut plain = LinearGauge::new("", 100, true);
        plain.set_style_context(100, 8);
        plain.update(50, "88");
        let mut cv_plain = PixCanvas::new(200, 140).unwrap();
        plain.draw(&mut cv_plain, 20, 10, &f, false);

        let mut lab = LabeledLinearGauge::new("油", 100, true);
        lab.gauge.set_style_context(100, 8);
        lab.gauge.update(50, "88");
        let mut cv_lab = PixCanvas::new(200, 140).unwrap();
        lab.draw(&mut cv_lab, 20, 10, 100, 8, &f, false);

        // 填充行 = y+h-1-50 .. y+h-2 = 59..108, 行 100 在内
        let plain_bar = 20 + f.measure("88") + 2;
        assert_eq!(a(&cv_plain, plain_bar + 1, 100), 240, "plain 填充存在");
        let label_w = f.measure("油");
        assert_eq!(a(&cv_lab, plain_bar + label_w + 1, 100), 240, "labeled 填充右移 labelW");
        assert_eq!(a(&cv_lab, plain_bar + 1, 100), 0, "原位置无条 (行 100 在文本区下方)");
    }

    /// SpeedRatioBar 分区/边界分支:
    /// - 背景 colorNum 全条, 速度比 shade 自底向上
    /// - 失速红区右半宽; 锁舵/马赫线在对应行 (butt2 行 y-1..y), r<=0 或 >=1 不画
    #[test]
    fn speed_ratio_bar_zones_and_boundary_branches() {
        let f = font();
        let mut b = SpeedRatioBar::new();
        b.set_style_context(10, 100);

        // 分支 1: 常态 speed=0.5 stall=0.25 mach=0.8 ail=0.3 rud=0.6
        // (x=45 使速度刻度左端 x-6-templateW("888")=3 落在画布内)
        b.update(0.5, 0.25, 0.8, 0.3, 0.6);
        let mut cv = PixCanvas::new(100, 130).unwrap();
        let (x, y) = (45, 10);
        b.draw(&mut cv, x, y, Some(&f), false);

        // 背景 colorNum: 顶部未被覆盖
        assert_eq!(a(&cv, x, y), 240, "顶部背景 colorNum");
        // shade 速度区 = shade 叠在 colorNum 背景上 (greenH=50 → 行 60..109; 行 60 被刻度覆盖 → 查 61)
        assert_a_close(a(&cv, x, y + 51), src_over_a(42, 240), "shade 区顶 (刻度行下)");
        assert_a_close(a(&cv, x, y + 99), src_over_a(42, 240), "shade 区底");
        // 红区: stallH=25, stallW=5 → 列 x+5..x+9, 行 y+75..y+99 (warning 叠 shade 栈)
        assert_a_close(a(&cv, x + 9, y + 99), src_over_a(100, src_over_a(42, 240)), "红区右下");
        assert_a_close(a(&cv, x + 4, y + 99), src_over_a(42, 240), "红区左邻仍是 shade");
        // 红区上方 (行 84) 仍在 shade 速度区内 (shade 行 60..109, 红区行 85..109)
        assert_a_close(a(&cv, x + 9, y + 74), src_over_a(42, 240), "红区上方仍是 shade 区");
        // 马赫线: machY = y+100-80 = y+20, 行 y+19..y+20, 列 x..x+10 (warning 叠背景)
        assert_a_close(a(&cv, x + 5, y + 20), src_over_a(100, 240), "马赫线行");
        assert_eq!(a(&cv, x + 5, y + 21), 240, "马赫线下方无");
        // 副翼刻度: lockY = y+100-30, 列 x-4..x+w/2-1 (butt 右端列不点亮), 行 lockY-1..lockY
        assert_eq!(a(&cv, x - 4, y + 70), 240, "副翼刻度左端 colorNum");
        assert_eq!(a(&cv, x - 5, y + 70), 0, "刻度左外");
        // 方向舵刻度: lockY = y+100-60 = y+40, 列 x+5..x+w+3 (右端列 x+w+4 不点亮)
        assert_eq!(a(&cv, x + 13, y + 40), 240, "方向舵刻度右端");
        assert_eq!(a(&cv, x + 14, y + 40), 0, "方向舵刻度右外");
        // 速度刻度: tickY = y+50, 左起 x-6-templateW("888"), 右端 x+8 (butt 右端列不点亮)
        let tw = f.measure("888");
        assert_eq!(a(&cv, x - 6 - tw, y + 50), 240, "速度刻度左端");
        // 右端在条上: tick(colorNum) 叠 shade 栈 (行 60 = shade 区顶)
        assert_a_close(
            a(&cv, x + 8, y + 50),
            src_over_a(240, src_over_a(42, 240)),
            "速度刻度右端 (条右缘-1)",
        );
        // x+9 (条右缘列) 无刻度像素: 仅 shade 叠背景栈
        assert_a_close(a(&cv, x + 9, y + 50), src_over_a(42, 240), "条右缘列无刻度");

        // 分支 2: 全零/越界开关 — 无 shade/红区/刻度, 纯背景
        let mut b2 = SpeedRatioBar::new();
        b2.set_style_context(10, 100);
        b2.update(0.0, 0.0, 1.0, 0.0, 1.5);
        let mut cv2 = PixCanvas::new(100, 130).unwrap();
        b2.draw(&mut cv2, x, y, None, false);
        // 速度刻度恒画 (行 y+99..y+100), 取避开刻度行的采样
        for yy in [y, y + 50, y + 98] {
            assert_eq!(a(&cv2, x + 5, yy), 240, "纯背景行 {yy}");
        }
        assert_eq!(a(&cv2, x - 4, y + 70), 0, "ail=0 无左刻度");
        assert_eq!(a(&cv2, x + 14, y + 40), 0, "rud=1.5 无右刻度");
        assert_eq!(a(&cv2, x + 5, y + 20), 240, "mach=1 无红线 (背景)");

        // 分支 3: clamp 越界: speed/stall > 1 → 满条 shade + 满高红区 (刻度行 y 除外)
        let mut b3 = SpeedRatioBar::new();
        b3.set_style_context(10, 100);
        b3.update(1.5, 1.5, 0.0, 0.0, 0.0);
        let mut cv3 = PixCanvas::new(100, 130).unwrap();
        b3.draw(&mut cv3, x, y, None, false);
        assert_a_close(a(&cv3, x, y + 5), src_over_a(42, 240), "speed>1 满条 shade");
        assert_a_close(a(&cv3, x + 9, y + 5), src_over_a(100, src_over_a(42, 240)), "stall>1 满高红区");
        assert_a_close(a(&cv3, x + 4, y + 5), src_over_a(42, 240), "红区左半 shade");
    }

    /// SpeedRatioBar 数值文本右对齐到刻度左缘, 基线 tickY-3 (Java:141-157)。
    /// 扫描速度刻度行 (y+62) 以上、条左侧区域: 只有 "47"+阴影, 右缘不越 tick 缘。
    #[test]
    fn speed_ratio_bar_value_text_right_aligned() {
        let f = font();
        let mut b = SpeedRatioBar::new();
        b.set_style_context(10, 100);
        b.update(0.47, 0.0, 0.0, 0.0, 0.0);
        let mut cv = PixCanvas::new(80, 130).unwrap();
        let (x, y) = (40, 10);
        b.draw(&mut cv, x, y, Some(&f), false);
        // displayValue = round(47)=47, 右缘 = x-6, 基线 = tickY-3 = y+100-47-3 = y+50
        let text_x = x - 6 - f.measure("47");
        assert!(text_x >= 0, "测试几何: 文本起点在画布内");
        let (mut min_col, mut max_col, mut count) = (i32::MAX, i32::MIN, 0);
        for yy in 0..(y + 50) {
            for xx in text_x..x {
                if a(&cv, xx, yy) > 0 {
                    min_col = min_col.min(xx);
                    max_col = max_col.max(xx);
                    count += 1;
                }
            }
        }
        assert!(count > 0, "存在文本像素");
        assert!(
            (text_x..=text_x + 2).contains(&min_col),
            "文本左缘 ≈ 右对齐起点 ({min_col} vs {text_x})"
        );
        assert!(max_col <= x - 5, "文本右缘含阴影不越 tick+1 缘 ({max_col})");
    }

    /// FlapAngleBar 正常分支: 三区宽度划分 (used=截断值, margin, overspeed)。
    /// total=250/current=25/maxSafe=65 选位使边界列避开固定刻度 (纯色可等值断言):
    /// used=50 (列 20..69), margin=80 (70..149), overspeed=120 (150..269);
    /// 刻度列 t=20+2·tick ∈ {59..60, 85..86, 139..140, 219..220} 均不在边界。
    #[test]
    fn flap_angle_bar_normal_split() {
        let f = font();
        let mut b = FlapAngleBar::new();
        b.set_style_context(250, 8);
        assert!(b.update(25.0, 65.0));
        assert_eq!(b.display_text(), " 25/ 65", "%3.0f 格式");
        let mut cv = PixCanvas::new(300, 80).unwrap();
        let (x, y) = (20, 5);
        b.draw(&mut cv, x, y, Some(&f), false);

        let bar_y = y + f.size + 2;
        assert_eq!(a(&cv, x + 30, bar_y), 42, "used 区 shade (列 50, 避开刻度 59..60)");
        assert_eq!(a(&cv, x + 49, bar_y), 42, "used 末列 (=50-1)");
        assert_eq!(a(&cv, x + 50, bar_y), 240, "margin 首列 colorNum");
        assert_eq!(a(&cv, x + 129, bar_y), 240, "margin 末列");
        assert_eq!(a(&cv, x + 130, bar_y), 100, "overspeed 首列 warning");
        assert_eq!(a(&cv, x + 249, bar_y), 100, "最右仍是 overspeed");
    }

    /// FlapAngleBar 刻度几何: 整除定位 tx、100 全高/其余 1/4、方帽上伸 1px
    /// (AA OFF 中心规则: 行 y0-1..y1, 下端行 y1+1 不点亮)、列 tx-1..tx。
    /// 宽 400 使文本居中后不与所测刻度列重叠 (alpha 才是纯 label 166)。
    #[test]
    fn flap_angle_bar_tick_geometry() {
        let f = font();
        let mut b = FlapAngleBar::new();
        b.set_style_context(400, 8);
        b.update(0.0, 100.0);
        let mut cv = PixCanvas::new(460, 80).unwrap();
        let (x, y) = (20, 5);
        b.draw(&mut cv, x, y, Some(&f), false);
        let bar_y = y + f.size + 2;
        // tx = x + tick*400/125 (int 除): 20→84? 20*400/125=64 → 84; 33→125+20=125... 计算见断言
        let t33 = x + 33 * 400 / 125; // 20 + 105 = 125
        let t100 = x + 100 * 400 / 125; // 20 + 320 = 340
        // 1/4 高刻度 (ext=8/4=2): 行 barY-2-3 .. barY = barY-5..barY (下端行不外伸)
        assert_eq!(a(&cv, t33, bar_y - 5), 166, "1/4 刻度顶部");
        assert_eq!(a(&cv, t33 - 1, bar_y - 5), 166, "刻度左列 (tx-1)");
        assert_eq!(a(&cv, t33, bar_y - 6), 0, "1/4 刻度上方无");
        assert_eq!(a(&cv, t33 - 2, bar_y - 5), 0, "刻度左列外");
        // 100 刻度全高 (ext=8): 行 barY-8-3 .. barY
        assert_eq!(a(&cv, t100, bar_y - 11), 166, "100 刻度方帽上伸 1px");
        assert_eq!(a(&cv, t100, bar_y - 12), 0, "100 刻度上方无");
    }

    /// FlapAngleBar 超限分支: current > maxSafe → margin=0, 红色紧接 used;
    /// 负角/NaN 边界保护不产生负宽 ((int)NaN=0 与 Java 一致)。
    /// used=160 → 列 20..179; 刻度 t100 列 179..180 恰压边界 → 断言取清洁列。
    #[test]
    fn flap_angle_bar_overspeed_and_guard() {
        let f = font();
        let mut b = FlapAngleBar::new();
        b.set_style_context(200, 8);
        b.update(100.0, 50.0);
        let mut cv = PixCanvas::new(240, 80).unwrap();
        let (x, y) = (20, 5);
        b.draw(&mut cv, x, y, Some(&f), false);
        let bar_y = y + f.size + 2;
        // used = 160; margin = 0; overspeed = 40 (首列 180 在刻度上 → 查 181)
        assert_eq!(a(&cv, x + 138, bar_y), 42, "used 区内清洁列");
        assert_eq!(a(&cv, x + 161, bar_y), 100, "超限红色 (清洁列)");
        assert_eq!(a(&cv, x + 199, bar_y), 100, "红色到最右");

        // 负角: used=max(0,-48)=0 → 全条 margin+overspeed
        let mut b2 = FlapAngleBar::new();
        b2.set_style_context(200, 8);
        b2.update(-30.0, 60.0);
        let mut cv2 = PixCanvas::new(240, 80).unwrap();
        b2.draw(&mut cv2, x, y, Some(&f), false);
        assert_eq!(a(&cv2, x, bar_y), 240, "used=0 → margin 起点");

        // NaN 角: 文本 "NaN/NaN"; Java NaN<=NaN 恒 false → 走超限分支, 全条红
        // ((int)NaN=0 → used=0, margin=0, overspeed=total)
        let mut b3 = FlapAngleBar::new();
        b3.set_style_context(200, 8);
        b3.update(f64::NAN, f64::NAN);
        let mut cv3 = PixCanvas::new(240, 80).unwrap();
        b3.draw(&mut cv3, x, y, Some(&f), false);
        assert_eq!(b3.display_text(), "NaN/NaN");
        assert_eq!(a(&cv3, x, bar_y), 100, "NaN → 超限分支全红 (Java NaN 比较恒 false)");
    }

    /// 线基元几何 (期望值 = Java 8 oracle 实测像素盒):
    /// - butt2 AA OFF: drawLine(10,·,20,·) 覆盖列 10..19 — 右端列不点亮
    ///   (宽>1 的 strokedShape 中心规则光栅, 端点含像素仅 1px Bresenham 快速路径)
    /// - square2 AA OFF: drawLine(·,10,·,25) 覆盖列 tx-1..tx / 行 9..25 —
    ///   方帽下端行不点亮
    /// - AA ON: 宽 2 线 3 行/列柔边, oracle 不透明色 a=128/255/128 角点 a=64,
    ///   此处按 cov_color 同式 (colors().num a=240 → 120/60; LABEL 166 → 83/42)
    #[test]
    fn line_primitive_pixel_boxes() {
        // butt2 AA OFF: 列 10..19, 行 14..15
        let mut cv = PixCanvas::new(40, 40).unwrap();
        hline_butt2(&mut cv, 10, 20, 15, colors().num, false);
        assert_eq!(a(&cv, 10, 14), 240, "butt2 左端列");
        assert_eq!(a(&cv, 19, 15), 240, "butt2 右端列 (oracle: 端点列不点亮)");
        assert_eq!(a(&cv, 20, 15), 0, "butt2 右端外");
        assert_eq!(a(&cv, 10, 13), 0, "butt2 上行外");
        assert_eq!(a(&cv, 10, 16), 0, "butt2 下行外");
        assert_eq!(a(&cv, 9, 15), 0, "butt2 左外");

        // butt2 AA ON: 覆盖盒 [10.5,20.5]×[14.5,16.5] → 3 行柔边 + 端点列半覆盖
        // (oracle: 21 列×3 行 = 63 非零像素)
        let mut cvs = PixCanvas::new(40, 40).unwrap();
        hline_butt2(&mut cvs, 10, 20, 15, colors().num, true);
        assert_eq!(a(&cvs, 15, 15), 240, "AA 中行全值");
        assert_eq!(a(&cvs, 15, 14), 120, "AA 上柔边行 a=round(240·0.5)");
        assert_eq!(a(&cvs, 15, 16), 120, "AA 下柔边行");
        assert_eq!(a(&cvs, 15, 13), 0, "AA 柔边外");
        assert_eq!(a(&cvs, 10, 15), 120, "AA 左端点列半覆盖");
        assert_eq!(a(&cvs, 20, 15), 120, "AA 右端点列 (AA ON 才点亮)");
        assert_eq!(a(&cvs, 10, 14), 60, "AA 角点 1/4 覆盖 (oracle a=64 同式)");
        assert_eq!(a(&cvs, 20, 16), 60, "AA 右下角点");
        assert_eq!(a(&cvs, 9, 15), 0, "AA 左外");
        assert_eq!(a(&cvs, 21, 15), 0, "AA 右外");

        // square2 AA OFF: 列 29..30, 行 9..25 (方帽上端外伸 1 行、下端行不点亮)
        let mut cv2 = PixCanvas::new(40, 40).unwrap();
        vline_square2(&mut cv2, 30, 10, 25, colors().label, false);
        assert_eq!(a(&cv2, 29, 9), 166, "square2 左列方帽上伸");
        assert_eq!(a(&cv2, 30, 25), 166, "square2 下端行 y1 (oracle: y1+1 行不点亮)");
        assert_eq!(a(&cv2, 30, 26), 0, "square2 下端行外");
        assert_eq!(a(&cv2, 30, 8), 0, "square2 上外");
        assert_eq!(a(&cv2, 28, 15), 0, "square2 左外");
        assert_eq!(a(&cv2, 31, 15), 0, "square2 右外");

        // square2 AA ON: 覆盖盒 [29.5,31.5]×[9.5,26.5] → 3 列柔边, 端行半透明
        // (oracle: 行 4..26 端行半透明, 列 a=128/255/128)
        let mut cv2s = PixCanvas::new(40, 40).unwrap();
        vline_square2(&mut cv2s, 30, 10, 25, colors().label, true);
        assert_eq!(a(&cv2s, 30, 15), 166, "AA 中列全值");
        assert_eq!(a(&cv2s, 29, 15), 83, "AA 左柔边列 a=round(166·0.5)");
        assert_eq!(a(&cv2s, 31, 15), 83, "AA 右柔边列");
        assert_eq!(a(&cv2s, 30, 9), 83, "AA 上端行半透明");
        assert_eq!(a(&cv2s, 30, 26), 83, "AA 下端行 (AA ON 才点亮)");
        assert_eq!(a(&cv2s, 29, 9), 42, "AA 角点 1/4 覆盖");
        assert_eq!(a(&cv2s, 30, 8), 0, "AA 上外");
        assert_eq!(a(&cv2s, 32, 15), 0, "AA 右外");

        // 1px: 列恰 x, 行 y0..y1 (AA 开关输出一致)
        let mut cv3 = PixCanvas::new(40, 40).unwrap();
        vline_1px(&mut cv3, 12, 5, 15, colors().warning, false);
        vline_1px(&mut cv3, 14, 5, 15, colors().warning, true);
        assert_eq!(a(&cv3, 12, 5), 100, "1px 线顶");
        assert_eq!(a(&cv3, 12, 15), 100, "1px 线底");
        assert_eq!(a(&cv3, 11, 10), 0, "1px 线左外");
        assert_eq!(a(&cv3, 13, 10), 0, "1px 线右外");
        assert_eq!(a(&cv3, 14, 10), 100, "1px AA ON 同盒");
        assert_eq!(a(&cv3, 15, 10), 0, "1px AA ON 右外");
    }

    /// drawRect 负/零尺寸语义 (Java 8 oracle): 负宽/负高整体不绘制;
    /// 零宽退化 1px 竖线 (列 x, 行 y..y+h)、零高退化 1px 横线、双零无输出
    #[test]
    fn ring_negative_and_degenerate() {
        let mut cv = PixCanvas::new(40, 40).unwrap();
        ring(&mut cv, 10, 10, -4, 20, colors().num);
        ring(&mut cv, 10, 10, 20, -4, colors().num);
        ring(&mut cv, 10, 10, -4, -9, colors().num);
        assert!(cv.pixmap().data().iter().all(|&b| b == 0), "负宽/负高 0 像素");

        // 零宽: oracle drawRect(50,10,0,20) = 列 50 行 10..30 的 1px 竖线
        let mut cv2 = PixCanvas::new(40, 40).unwrap();
        ring(&mut cv2, 20, 5, 0, 15, colors().num);
        assert_eq!(a(&cv2, 20, 5), 240, "零宽退化竖线顶");
        assert_eq!(a(&cv2, 20, 20), 240, "零宽退化竖线底 (行 y..y+h)");
        assert_eq!(a(&cv2, 20, 21), 0, "竖线底外");
        assert_eq!(a(&cv2, 19, 10), 0, "竖线左外");

        // 零高: 1px 横线 列 x..x+w
        let mut cv3 = PixCanvas::new(40, 40).unwrap();
        ring(&mut cv3, 5, 20, 15, 0, colors().num);
        assert_eq!(a(&cv3, 5, 20), 240, "零高退化横线左端");
        assert_eq!(a(&cv3, 20, 20), 240, "零高退化横线右端");
        assert_eq!(a(&cv3, 21, 20), 0, "横线右外");
        assert_eq!(a(&cv3, 10, 19), 0, "横线上外");

        // 双零: drawRect 的 4 条边线全为零长度段, 无输出
        let mut cv4 = PixCanvas::new(40, 40).unwrap();
        ring(&mut cv4, 10, 10, 0, 0, colors().num);
        assert!(cv4.pixmap().data().iter().all(|&b| b == 0), "双零无输出");
    }

    /// fmt_pct3 边界: HALF_UP 舍入、宽度补齐、负数 (含 -0.0 保号)、
    /// 0.5 的 f64 前驱 (精确十进制舍入)、NaN
    #[test]
    fn fmt_pct3_rounding_and_padding() {
        assert_eq!(fmt_pct3(0.0), "  0");
        assert_eq!(fmt_pct3(99.5), "100", "HALF_UP 进位且自然超宽");
        assert_eq!(fmt_pct3(0.4), "  0");
        assert_eq!(fmt_pct3(0.5), "  1", ".5 进位");
        assert_eq!(fmt_pct3(-0.5), " -1", "负域远离零 (Java Formatter HALF_UP)");
        assert_eq!(fmt_pct3(-2.4), " -2");
        // oracle: String.format("%3.0f", -0.0) 数值部分 "-0" (负零保号), 宽 3 补成 " -0"
        assert_eq!(fmt_pct3(-0.0), " -0", "负零保号 (Java oracle, 宽 3 含符号)");
        assert_eq!(fmt_pct3(-0.4), " -0", "负值舍到零保负号");
        // oracle: 0.5 的 f64 前驱按精确十进制 HALF_UP 舍到 0
        // (v+0.5 在 f64 中进到 1.0 的舍入路径已修正)
        assert_eq!(fmt_pct3(0.49999999999999994), "  0");
        assert_eq!(fmt_pct3(f64::NAN), "NaN");
    }

    /// 脏检查: 同值重复 update 返回 false, 变化置脏, draw 清脏
    #[test]
    fn dirty_checking_semantics() {
        let mut g = LinearGauge::new("T", 100, true);
        assert!(g.update(50, "50"));
        assert!(!g.update(50, "50"), "同值不脏");
        assert!(g.is_dirty());
        let f = font();
        let mut cv = PixCanvas::new(100, 140).unwrap();
        g.draw(&mut cv, 5, 5, &f, false);
        assert!(!g.is_dirty(), "draw 后清脏");
        assert!(g.update(60, "60"));

        let mut b = FlapAngleBar::new();
        assert!(b.update(10.0, 100.0));
        assert!(!b.update(10.0, 100.0));

        let mut s = SpeedRatioBar::new();
        assert!(s.update(0.5, 0.1, 0.1, 0.1, 0.1));
        assert!(!s.update(0.5, 0.1, 0.1, 0.1, 0.1));
    }

    /// W3 契约: 风格/值色 setter 置脏 (同值不置脏) — 按 is_dirty() 门控 draw
    /// 的组装层 (MiniHUD ThrottleBar 的 valueColor 注入) 必须经 setter 改字段
    #[test]
    fn linear_gauge_style_setters_mark_dirty() {
        let f = font();
        let mut g = LinearGauge::new("T", 100, true);
        g.set_style_context(40, 6);
        let mut cv = PixCanvas::new(120, 140).unwrap();
        g.update(50, "50");
        g.draw(&mut cv, 5, 5, &f, false);
        assert!(!g.is_dirty(), "draw 后清脏");

        g.set_value_color(Some(colors().warning));
        assert!(g.is_dirty(), "值色注入置脏");
        g.set_vertical(false);
        g.set_tick_on_right(true);
        g.set_max_value(200);
        g.set_label("U");
        g.draw(&mut cv, 5, 5, &f, false);
        assert!(!g.is_dirty(), "再次 draw 清脏");

        g.set_vertical(false);
        g.set_value_color(Some(colors().warning));
        assert!(!g.is_dirty(), "同值 setter 不置脏");
    }
}
