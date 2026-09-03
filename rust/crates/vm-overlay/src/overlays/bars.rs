//! gauges_bars: 条形 gauge 家族 C 类语义复刻
//!
//! - LinearGauge — 竖/横条 + 随值移动的刻度线与文本, 零 GC buffer→String 缓存。
//! - LabeledLinearGauge — 前置标签 (label+value 同基线), 横向刻度线修正版。
//! - SpeedRatioBar — 速度比双色条 + 失速红区/马赫红线/锁舵刻度。
//! - FlapAngleBar — 襟翼三色分区条 + 固定角度刻度。
//!
//! 绘制目标 = render2d::PixCanvas; 颜色/坐标公式逐项对照 Java paint 逻辑
//! (关键处 // 标注)。Java char[] 零 GC buffer 模式在 Rust 侧
//! 统一为可复用 String 缓存 (update 仅在内容变化时重写)。
//!
//! 轴对齐整数端点线不走 tiny-skia 矢量光栅化, 按 历史基线像素盒直填
//! (期望值逐像素钉死在 line_primitive_pixel_boxes; 基元与完整 基线 注记在
//! render::primitives, 重构波13 自本文件收敛):
//! - AA OFF (ANTIALIAS_OFF): 像素中心规则 — 像素中心落在 stroke 覆盖盒内才点亮。
//! - AA ON (生产 graphAASetting 恒 ON, 无运行时关闭路径):
//!   整数端点经 STROKE_NORMALIZE 规整到像素中心, 宽 2 线呈 3 行/列柔边,
//!   用覆盖率缩放 alpha 的解析盒复刻 (cov_color, 等价 Java AA 的 SrcOver×coverage
//!   合成); 1px 线 AA 开关输出一致。
//! - drawRect 环: 负宽/负高整体不绘制 (基线 0 像素); 零宽/零高退化 1px 线。

use crate::render::canvas::PixCanvas;
use crate::render::font::LoadedFont;
use crate::render::palette::colors;
use crate::render::primitives::{self, butt_line, vline_1px, vline_square2};
use vm_core::base::format;
use vm_core::base::format::java_round_f32;

/// LinearGauge 私有 drawRect 助手: shade 环 + fill 内芯。
/// flip_logic=true 为横向 gauge 的竖直刻度:
/// drawRect(x+w, y, -w-1, h-1) 负宽 → 整体不绘制, fillRect 负宽同样不绘制 —
/// 该分隔线在 Java 中【完全不可见】(历史基线), 横向 LinearGauge 只剩
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
        // drawRect(x,y,w-1,h-1)=w×h 环 + fillRect(x+1,y+1,w-2,h-2)
        primitives::ring1px(cv, x, y, w - 1, h - 1, shade);
        cv.fill_rect(x + 1, y + 1, w - 2, h - 2, fill);
    } else {
        // flip 分支的 drawRect 负宽环
        primitives::ring1px(cv, x + w, y, -w - 1, h - 1, shade);
        cv.fill_rect(x + 1 + w, y + 1, -w - 2, h - 2, fill); // 负高 → 不绘制
    }
}

// hline_butt2/vline_square2 已收敛 primitives.rs (波13): 水平 w=2 线直接
// butt_line(w=2) — 逐像素等价 (基线 见 primitives 文档); 方帽竖线为独立基元
// vline_square2。

// ---------------------------------------------------------------------------
// LinearGauge
// ---------------------------------------------------------------------------

/// 通用条形 gauge。
/// Java public 字段直译为 pub; value_color=None → 用 colors().num。
pub struct LinearGauge {
    pub label: String,
    pub max_value: i32,
    pub cur_value: i32,
    pub display_value: String,
    pub vertical: bool,
    /// true = 刻度在竖条右侧
    pub tick_on_right: bool,
    /// 值色覆盖 (Java valueColor, MiniHUD ThrottleBar 注入 HUDData.throttleColor)
    pub value_color: Option<[u8; 4]>,
    // 风格上下文缓存 (setStyleContext)
    length_cache: i32,
    thickness_cache: i32,
    // 脏检查: 值/风格变化置位, draw 后清零
    dirty: bool,
}

impl LinearGauge {
    /// 构造 (tickOnRight=false 默认)
    pub fn new(label: &str, max_value: i32, vertical: bool) -> Self {
        Self::with_tick_on_right(label, max_value, vertical, false)
    }

    /// 构造
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
            length_cache: 100,
            thickness_cache: 10,
            dirty: true,
        }
    }

    pub fn set_style_context(&mut self, length: i32, thickness: i32) {
        self.length_cache = length;
        self.thickness_cache = thickness;
        self.dirty = true;
    }

    /// 风格缓存读取口 (组装层 preferred_size 直读; Java lengthCache 字段)
    pub fn length_cache(&self) -> i32 {
        self.length_cache
    }

    /// 风格缓存读取口 (同上; Java thicknessCache 字段)
    pub fn thickness_cache(&self) -> i32 {
        self.thickness_cache
    }

    /// update(value, displayValue)。返回值是否变化 (脏检查)。
    /// 契约: 仅覆盖 value/displayValue — Java onDataUpdate 同帧还会注入
    /// valueColor/displayValue 等字段 (MiniHUD ThrottleBar), Rust 侧
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

    /// pixVal = Math.round((float)curValue * length / maxValue), max<=0 → 0
    fn pix_value(&self, length: i32) -> i32 {
        if self.max_value > 0 {
            // Java (float) 提升链保持 f32
            java_round_f32(self.cur_value as f32 * length as f32 / self.max_value as f32)
        } else {
            0
        }
    }

    /// drawBar: shade 1px 环 + colorNum 填充 (竖条自底向上)
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
        // drawRect(x,y,w-1,h-1) — 自 (x,y) 向下/右生长
        primitives::ring1px(cv, x, y, w - 1, h - 1, shade);
        if is_vert {
            let val_h = if val > h { h } else { val };
            if val_h >= 0 {
                // fillRect(x+1, y+h-1-valH, w-2, valH) — 底部对齐内芯
                cv.fill_rect(x + 1, y + h - 1 - val_h, w - 2, val_h, fill);
            }
        } else {
            let val_w = if val > w { w } else { val };
            if val_w >= 0 {
                // fillRect(x+1, y+1, valW-2, h-2) — valW<2 时负宽不绘制
                cv.fill_rect(x + 1, y + 1, val_w - 2, h - 2, fill);
            }
        }
    }

    /// draw(g2d, x, y) — 用缓存风格。fontLabel 未参与绘制 (Java 同)。
    pub fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, font_num: &LoadedFont, aa: bool) {
        let (len, th) = (self.length_cache, self.thickness_cache);
        self.draw_sized(cv, x, y, len, th, font_num, aa);
    }

    /// 完整参数版 (draw(g2d, x, y, length, thickness, ...))
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
        // 值色覆盖 / shade 恒为 colorShadeShape; 条填充恒 colorNum
        let c = self.value_color.unwrap_or(colors().num);
        let shade = colors().shade_shape;

        if self.vertical {
            // (x,y) = 组合区域左上
            let text_width = font_num.measure(&self.display_value);
            let label_spacing = 2;

            // 分隔线 y = 底 - 1 - pixVal
            // (极端 curValue 下 i32 加减 Java 静默回绕 / Rust panic §2.2, 不可达, 备查)
            let sep_y = y + length - 1 - pix_val;

            if self.tick_on_right {
                // 条在左, 刻度(分隔线+文本)在右
                Self::draw_bar(
                    cv,
                    x,
                    y,
                    thickness,
                    length,
                    pix_val,
                    shade,
                    colors().num,
                    true,
                );
                let total_width = thickness + label_spacing + text_width;
                gauge_rect(cv, x, sep_y, total_width, 3, shade, c, false);
                primitives::text_shaded(
                    cv,
                    font_num,
                    x + thickness + label_spacing,
                    sep_y - 1,
                    &self.display_value,
                    c,
                    shade,
                    aa,
                );
            } else {
                // 刻度(文本+分隔线)在左, 条在右 (默认)
                let bar_x = x + text_width + label_spacing;
                Self::draw_bar(
                    cv,
                    bar_x,
                    y,
                    thickness,
                    length,
                    pix_val,
                    shade,
                    colors().num,
                    true,
                );
                let total_width = text_width + label_spacing + thickness;
                gauge_rect(cv, x, sep_y, total_width, 3, shade, c, false);
                primitives::text_shaded(
                    cv,
                    font_num,
                    x,
                    sep_y - 1,
                    &self.display_value,
                    c,
                    shade,
                    aa,
                );
            }
        } else {
            // 横条 + 竖直分隔线(flip 环在条上方) + 条下方文本
            Self::draw_bar(
                cv,
                x,
                y,
                length,
                thickness,
                pix_val,
                shade,
                colors().num,
                false,
            );
            gauge_rect(
                cv,
                x + pix_val - 2,
                y,
                3,
                -thickness - font_num.size,
                shade,
                c,
                true,
            );
            primitives::text_shaded(
                cv,
                font_num,
                x + pix_val,
                y + thickness + font_num.size,
                &self.display_value,
                c,
                shade,
                aa,
            );
        }
        self.dirty = false;
    }
}

// ---------------------------------------------------------------------------
// LabeledLinearGauge (组合 + 委托, extends 映射)
// ---------------------------------------------------------------------------

/// 带前置标签的 LinearGauge。
/// 标签与数值同基线相连: [label][value]。
pub struct LabeledLinearGauge {
    pub gauge: LinearGauge,
}

impl LabeledLinearGauge {
    pub fn new(label: &str, max_value: i32, vertical: bool) -> Self {
        LabeledLinearGauge {
            gauge: LinearGauge::new(label, max_value, vertical),
        }
    }

    /// label+value 合成宽度 (getValueWidth 覆写)
    fn value_width(&self, font_num: &LoadedFont) -> i32 {
        font_num.measure(&self.gauge.label) + font_num.measure(&self.gauge.display_value)
    }

    /// 标签+数值双段阴影文本 (drawValueText 覆写)
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
        primitives::text_shaded(cv, font_num, x, y, &self.gauge.label, c, shade, aa);
        let label_w = font_num.measure(&self.gauge.label);
        primitives::text_shaded(
            cv,
            font_num,
            x + label_w,
            y,
            &self.gauge.display_value,
            c,
            shade,
            aa,
        );
    }

    /// draw 覆写 (竖向走基类逻辑 + 前置标签宽度; 横向自绘修正分隔线)。
    /// 的 new Font("Sarasa Mono SC",...) 由调用方直接传等宽字体实现。
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
            // 基类逻辑, 文本宽度换 label+value 合成 (条与分隔线右移)
            let pix_val = g.pix_value(length);
            let c = g.value_color.unwrap_or(colors().num);
            let shade = colors().shade_shape;
            let text_width = self.value_width(font_num);
            let label_spacing = 2;
            let sep_y = y + length - 1 - pix_val;
            if g.tick_on_right {
                LinearGauge::draw_bar(
                    cv,
                    x,
                    y,
                    thickness,
                    length,
                    pix_val,
                    shade,
                    colors().num,
                    true,
                );
                let total_width = thickness + label_spacing + text_width;
                gauge_rect(cv, x, sep_y, total_width, 3, shade, c, false);
                self.draw_value_text(
                    cv,
                    x + thickness + label_spacing,
                    sep_y - 1,
                    font_num,
                    c,
                    shade,
                    aa,
                );
            } else {
                let bar_x = x + text_width + label_spacing;
                LinearGauge::draw_bar(
                    cv,
                    bar_x,
                    y,
                    thickness,
                    length,
                    pix_val,
                    shade,
                    colors().num,
                    true,
                );
                let total_width = text_width + label_spacing + thickness;
                gauge_rect(cv, x, sep_y, total_width, 3, shade, c, false);
                self.draw_value_text(cv, x, sep_y - 1, font_num, c, shade, aa);
            }
        } else {
            // 横向修正版
            let pix_val = g.pix_value(length);
            let c = colors().num;
            let shade_shadow = colors().shade_shape;

            // 1. 条背景+边框 (drawBarFixed 横向分支与基类横向 drawBar 一致)
            LinearGauge::draw_bar(
                cv,
                x,
                y,
                length,
                thickness,
                pix_val,
                shade_shadow,
                colors().num,
                false,
            );

            // 2. 竖直分隔线: 条顶延伸到文本底
            //    sepHeight = thickness + fontSize + 2; 影线 x+pixVal+1 / 主线 x+pixVal
            //    Java 此处未 setStroke, 线宽承袭调用链遗留 stroke — 唯一消费链
            //    GaugeField→EngineControlOverlay/LinearGaugeRenderer 的绘制序中前置
            //    gauge 恒 set 1px 族 stroke, 首个 gauge 前为 Graphics2D 默认
            //    BasicStroke(1); 历史基线两种遗留 stroke 下 1px 竖线输出一致
            //    (列 x, 行 y0..y1 端点含硬边)。Rust 组装层若绘制顺序变化需回访此处
            let sep_height = thickness + font_num.size + 2;
            vline_1px(cv, x + pix_val + 1, y, y + sep_height, shade_shadow);
            vline_1px(cv, x + pix_val, y, y + sep_height, c);

            // 3. label+value 合成文本, 条下方
            self.draw_value_text(
                cv,
                x + pix_val,
                y + thickness + font_num.size,
                font_num,
                c,
                shade_shadow,
                aa,
            );
        }
        self.gauge.dirty = false;
    }
}

// ---------------------------------------------------------------------------
// SpeedRatioBar
// ---------------------------------------------------------------------------

/// 速度比竖条。0=底 1=顶, 全高代表 VNE。
pub struct SpeedRatioBar {
    width: i32,
    height: i32,
    // 数据态
    speed_ratio: f64,
    stall_ratio: f64,
    unit_mach_ratio: f64,
    aileron_lock_ratio: f64,
    rudder_lock_ratio: f64,
    dirty: bool,
}

/// clamp(v) = 0..1 (NaN 穿透: 两比较均 false 返回原值, 与 Java 一致)
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
            width: 10,
            height: 100,
            speed_ratio: 0.0,
            stall_ratio: 0.0,
            unit_mach_ratio: 0.0,
            aileron_lock_ratio: 0.0,
            rudder_lock_ratio: 0.0,
            dirty: true,
        }
    }

    /// setStyleContext (tick_font 由 draw 参数传入, Java 允许 null)
    pub fn set_style_context(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
        self.dirty = true;
    }

    /// width/height 读取口 (组装层 preferred_size 直读; Java 字段)
    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    /// onDataUpdate 五比值注入。返回是否变化 (脏检查)。
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

    /// lockY = y + height - Math.round((float)(height * ratio))
    /// 极端 |ratio| 下 java_round_f32 饱和到 i32::MAX/MIN 后的加减在
    /// Java 静默回绕, Rust debug 会 panic — 真实遥测幅度不可达, 记录备查
    fn ratio_y(&self, y: i32, ratio: f64) -> i32 {
        // Java double 乘法后 (float) 强转, 再 Math.round(float)
        y + self.height - java_round_f32((self.height as f64 * ratio) as f32)
    }

    /// draw。绘制顺序即图层序 (锁舵刻度先画被条盖住右半, 红区最后)。
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

        // 1. 副翼锁舵刻度 (左)
        if self.aileron_lock_ratio > 0.0 && self.aileron_lock_ratio < 1.0 {
            let lock_y = self.ratio_y(y, self.aileron_lock_ratio);
            butt_line(cv, x - 4, lock_y, x + w / 2, lock_y, 2, colors().num, aa);
        }

        // 2. 方向舵锁舵刻度 (右)
        if self.rudder_lock_ratio > 0.0 && self.rudder_lock_ratio < 1.0 {
            let lock_y = self.ratio_y(y, self.rudder_lock_ratio);
            butt_line(
                cv,
                x + w / 2,
                lock_y,
                x + w + 4,
                lock_y,
                2,
                colors().num,
                aa,
            );
        }

        // 3. 背景 colorNum 全条 = 剩余范围
        cv.fill_rect(x, y, w, h, colors().num);

        // 4. shade 填充 = 当前速度比 (底→值)
        let green_h = java_round_f32((h as f64 * clamp01(self.speed_ratio)) as f32);
        if green_h > 0 {
            cv.fill_rect(x, y + h - green_h, w, green_h, colors().shade_shape);
        }

        // 4.5 速度比刻度 (左, 跨文本区到条右缘) + 右对齐数值
        {
            let tick_y = self.ratio_y(y, clamp01(self.speed_ratio));
            let template_width = tick_font.map(|f| f.measure("888")).unwrap_or(0);
            let tick_extend = 4;
            let label_spacing = 2;
            let tick_start_x = x - tick_extend - label_spacing - template_width;
            let tick_width = template_width + label_spacing + tick_extend + w;
            butt_line(
                cv,
                tick_start_x,
                tick_y,
                tick_start_x + tick_width - 1,
                tick_y,
                2,
                colors().num,
                aa,
            );

            if let Some(f) = tick_font {
                // (int) Math.round(speedRatio * 100)
                let display_value = format::java_round_f64(self.speed_ratio * 100.0);
                let value_str = display_value.to_string();
                let actual_text_width = f.measure(&value_str);
                let text_right_edge = x - tick_extend - label_spacing;
                let text_x = text_right_edge - actual_text_width;
                let text_y = tick_y - 3; // 刻度上方
                primitives::text_shaded(
                    cv,
                    f,
                    text_x,
                    text_y,
                    &value_str,
                    colors().num,
                    colors().shade_shape,
                    aa,
                );
            }
        }

        // 5. 红区 (失速): 右侧半宽覆盖条
        let stall_h = java_round_f32((h as f64 * clamp01(self.stall_ratio)) as f32);
        if stall_h > 0 {
            let mut stall_w = w / 2;
            if stall_w < 2 {
                stall_w = 2;
            }
            cv.fill_rect(
                x + w - stall_w,
                y + h - stall_h,
                stall_w,
                stall_h,
                colors().warning,
            );
        }

        // 6. 马赫单位红线
        if self.unit_mach_ratio > 0.0 && self.unit_mach_ratio < 1.0 {
            let mach_y = self.ratio_y(y, self.unit_mach_ratio);
            butt_line(cv, x, mach_y, x + w, mach_y, 2, colors().warning, aa);
        }
        // 无边框 (对齐 FlapAngleBar 风格)
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

/// 襟翼角度三色条。刻度固定 {20,33,60,100}, 满刻度 125。
pub struct FlapAngleBar {
    total_width: i32,
    bar_height: i32,
    current_angle: f64,
    max_safe_angle: f64,
    display_text: String,
    dirty: bool,
}

/// 刻度位置
const TICK_POSITIONS: [i32; 4] = [20, 33, 60, 100];
/// 满刻度
const MAX_SCALE: i32 = 125;

/// Java String.format("%3.0f") = java_f0_exact + 宽 3 右对齐组合
/// (整数化算法本体在 vm_core::base::format, 重构波13 收割)
fn fmt_pct3(v: f64) -> String {
    format::pad_width(format::java_f0_exact(v), 3, false)
}

impl FlapAngleBar {
    pub fn new() -> Self {
        FlapAngleBar {
            total_width: 0,
            bar_height: 0,
            current_angle: 0.0,
            max_safe_angle: 100.0,
            display_text: "  0/100".to_string(),
            dirty: true,
        }
    }

    /// setStyleContext
    pub fn set_style_context(&mut self, total_width: i32, bar_height: i32) {
        self.total_width = total_width;
        self.bar_height = bar_height;
        self.dirty = true;
    }

    /// totalWidth/barHeight 读取口 (组装层 preferred_size 直读; Java 字段)
    pub fn total_width(&self) -> i32 {
        self.total_width
    }

    pub fn bar_height(&self) -> i32 {
        self.bar_height
    }

    /// onDataUpdate: 角度对 + "%3.0f/%3.0f" 显示文本
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

    /// draw。font=None 直接返回。
    pub fn draw(
        &mut self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        font: Option<&LoadedFont>,
        aa: bool,
    ) {
        let font = match font {
            Some(f) => f,
            None => return, // PORT: font==null 不绘制
        };
        let total_width = self.total_width;
        let bar_height = self.bar_height;

        // 1. 居中文本 (ascent 基线)
        let ascent = font.metrics().ascent;
        let text_y = y + ascent;
        let str_width = font.measure(&self.display_text);
        // int 除法向零截断 (strWidth 超宽时整体左移)
        let str_x = x + (total_width - str_width) / 2;
        primitives::text_shaded(
            cv,
            font,
            str_x,
            text_y,
            &self.display_text,
            colors().num,
            colors().shade_shape,
            aa,
        );

        // 条位于文本下方 (字号近似行高)
        let bar_y = y + font.size + 2;

        // 三区宽度: used=已用(shade), margin=安全裕度(colorNum), overspeed=超速(warning)
        // (int)(double) 截断
        let mut used_width = (self.current_angle * total_width as f64 / MAX_SCALE as f64) as i32;
        let margin_width = if self.current_angle <= self.max_safe_angle {
            // 正常: 有安全裕度
            (self.max_safe_angle * total_width as f64 / MAX_SCALE as f64) as i32 - used_width
        } else {
            // 超限: 无裕度, 红色为右侧剩余
            0
        };
        let mut overspeed_width = total_width - used_width - margin_width;

        // 边界保护
        used_width = used_width.max(0);
        let margin_width = margin_width.max(0);
        overspeed_width = overspeed_width.max(0);

        // 刻度: colorLabel, 裸 BasicStroke(2) 方帽
        // tx = x + tick*totalWidth/MAX_SCALE 是 int 算术 (整除);
        // 100 刻度全高, 其余 1/4 高; 线自 barY-ext-2 到 barY (先于分区绘制, 与条重叠段被覆盖)
        // tick*total_width 在极端 total_width 下 Java 静默回绕 / Rust panic,
        // 真实布局幅度不可达, 记录备查
        for &tick in &TICK_POSITIONS {
            let tx = x + tick * total_width / MAX_SCALE;
            let ext = if tick == 100 {
                bar_height
            } else {
                bar_height / 4
            };
            vline_square2(cv, tx, bar_y - ext - 2, bar_y, colors().label, aa);
        }

        // 已用区 (0→current, shade)
        if used_width > 0 {
            cv.fill_rect(x, bar_y, used_width, bar_height, colors().shade_shape);
        }
        // 安全裕度区 (current→maxSafe, colorNum)
        if margin_width > 0 {
            cv.fill_rect(
                x + used_width,
                bar_y,
                margin_width,
                bar_height,
                colors().num,
            );
        }
        // 超速区 (右侧剩余, warning)
        if overspeed_width > 0 {
            cv.fill_rect(
                x + used_width + margin_width,
                bar_y,
                overspeed_width,
                bar_height,
                colors().warning,
            );
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
mod tests;
