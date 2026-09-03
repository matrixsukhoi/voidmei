//! 公共像素基元 + MarkedGauge 仪表组件族 (ui/component/gauge/{MarkedGauge,
//! GaugeBarStyle, GaugeMarker, MarkerType}.java 的内容复刻)。
//! 重构波2 自 overlays_field1.rs 拆出; 基元已随波3/波13 收敛 primitives.rs
//! (butt_line 波13 迁出)。

use crate::render::primitives::butt_line;
use crate::render::primitives::ring1px;
use crate::render::primitives::text_shaded_auto;
use crate::render::primitives::vline_1px;
use std::rc::Rc;
use vm_core::base::format::java_round_f32;

use crate::render::canvas::PixCanvas;
use crate::render::font::LoadedFont;
use crate::render::palette::colors;

// ---------------------------------------------------------------------------
// MarkerType (ui/component/gauge/MarkerType.java)
// ---------------------------------------------------------------------------

/// 标记类型枚举 (MarkerType.java: 每个值的 javadoc 语义见各变体文档)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerType {
    /// 跨条全宽线 (增压器 optimal 档指示 / 马赫红线)
    LineFull,
    /// 部分宽度刻度 (side 控制位置 -1=左 0=中 1=右, widthRatio 控制长度)
    LinePartial,
    /// 填充区间 (widthRatio 控制宽, side 定位)
    Zone,
    /// 带文字标签的刻度
    TickLabeled,
}

// ---------------------------------------------------------------------------
// GaugeMarker (ui/component/gauge/GaugeMarker.java)
// ---------------------------------------------------------------------------

/// 不可变标记规格 (Java final 类 + Builder; Rust pub 字段 + Default 承接 Builder 缺省)。
#[derive(Debug, Clone, PartialEq)]
pub struct GaugeMarker {
    /// 唯一 id (动态更新通道 updateMarkerRatio 的键)
    pub id: String,
    pub marker_type: MarkerType,
    /// 位置比率 0..1; 越界隐藏 (isVisible)
    pub ratio: f64,
    pub color: [u8; 4],
    /// TICK_LABELED 的文字
    pub label: String,
    /// ZONE / LINE_PARTIAL 的宽度比率 (相对条宽)
    pub width_ratio: f32,
    /// 定位侧: -1 左 / 0 中 / 1 右
    pub side: i32,
}

impl Default for GaugeMarker {
    /// Java Builder 缺省: type=LINE_FULL, ratio=-1 (隐藏), color=RED,
    /// widthRatio=0.5, side=0
    fn default() -> Self {
        GaugeMarker {
            id: String::new(),
            marker_type: MarkerType::LineFull,
            ratio: -1.0,
            color: [255, 0, 0, 255], // java.awt.Color.RED
            label: String::new(),
            width_ratio: 0.5,
            side: 0,
        }
    }
}

impl GaugeMarker {
    /// withRatio: |new-old|<0.0001 返回自身 (零分配);
    /// Rust 返回 bool 表示是否变化, 拷贝语义由调用方原位赋值等价实现
    pub fn with_ratio_changed(&self, new_ratio: f64) -> bool {
        (new_ratio - self.ratio).abs() >= 0.0001
    }

    /// isVisible: ratio ∈ [0,1] 才绘制
    pub fn is_visible(&self) -> bool {
        self.ratio >= 0.0 && self.ratio <= 1.0
    }
}

// ---------------------------------------------------------------------------
// GaugeBarStyle (ui/component/gauge/GaugeBarStyle.java)
// ---------------------------------------------------------------------------

/// 条样式配置 (Java final + Builder 预缓存 Stroke; Rust 直接持有宽度参数,
/// borderStroke=createPreciseStroke(1), tickStroke=createPreciseStroke(strokeWidth))
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaugeBarStyle {
    /// 当前值填充色 (fillColor)
    pub fill_color: [u8; 4],
    /// 空段背景色 (backgroundColor)
    pub background_color: [u8; 4],
    /// 边框色 (showBorder 时绘制)
    pub border_color: [u8; 4],
    pub show_border: bool,
    /// 竖条 (自底向上填充)
    pub vertical: bool,
    /// tick/border 线宽 (px)
    pub stroke_width: i32,
}

impl Default for GaugeBarStyle {
    /// Java Builder 缺省: fillColor=CYAN, backgroundColor=DARK_GRAY,
    /// borderColor=GRAY, showBorder=false, vertical=true, strokeWidth=2
    fn default() -> Self {
        GaugeBarStyle {
            fill_color: [0, 255, 255, 255],      // Color.CYAN
            background_color: [64, 64, 64, 255], // Color.DARK_GRAY
            border_color: [128, 128, 128, 255],  // Color.GRAY
            show_border: false,
            vertical: true,
            stroke_width: 2,
        }
    }
}

// ---------------------------------------------------------------------------
// MarkedGauge (ui/component/gauge/MarkedGauge.java)
// ---------------------------------------------------------------------------

/// 带可插拔标记的条形仪表 (组合模式, 不继承 LinearGauge — Java 同)。
/// 双值通道: update_buffer (零 GC buffer, value_len>0) / update_display (字符串,
/// value_len=0); drawValueText/getValueWidth 按 value_len 优选 buffer。
pub struct MarkedGauge {
    /// setStyleContext 的宽度缓存 (仅 2 参 draw()/preferredSize 用, 显式 draw 不读)
    pub width: i32,
    pub height: i32,
    pub bar_style: GaugeBarStyle,
    /// TICK_LABELED 标签字体 (Java tickFont 字段, setStyleContext 注入, null 则标签
    /// 不画 — MarkedGauge 的 null 守卫)
    pub tick_font: Option<Rc<LoadedFont>>,
    /// 标记按加入顺序绘制
    pub markers: Vec<GaugeMarker>,
    pub current_value: f64,
    pub max_value: f64,
    pub label: String,
    /// 零 GC 缓冲 (Java char[32]; Rust String 无 stale tail, length=字符数)
    pub value_buffer: String,
    pub value_len: i32,
    pub display_value: String,
}

impl Default for MarkedGauge {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkedGauge {
    /// Java 构造器: width=10, height=100, 默认样式 (fill=colorNum, bg=colorShadeShape, vertical)
    pub fn new() -> Self {
        MarkedGauge {
            width: 10,
            height: 100,
            bar_style: GaugeBarStyle {
                fill_color: colors().num,
                background_color: colors().shade_shape,
                vertical: true,
                ..GaugeBarStyle::default()
            },
            tick_font: None,
            markers: Vec::new(),
            current_value: 0.0,
            max_value: 1.0,
            label: String::new(),
            value_buffer: String::new(),
            value_len: 0,
            display_value: String::new(),
        }
    }

    /// setStyleContext
    pub fn set_style_context(
        &mut self,
        width: i32,
        height: i32,
        tick_font: Option<Rc<LoadedFont>>,
        style: GaugeBarStyle,
    ) {
        self.width = width;
        self.height = height;
        self.tick_font = tick_font;
        self.bar_style = style;
    }

    /// setBarStyle
    pub fn set_bar_style(&mut self, style: GaugeBarStyle) {
        self.bar_style = style;
    }

    /// setMaxValue — double 量程 (0 到 maxValue)
    pub fn set_max_value(&mut self, max_value: f64) {
        self.max_value = max_value;
    }

    /// addMarker
    pub fn add_marker(&mut self, marker: GaugeMarker) {
        self.markers.push(marker);
    }

    /// updateMarkerRatio: 按 id 原位更新; 未命中无操作;
    /// |Δ|<0.0001 不写 (Java copy-on-write 零分配语义的等价物)
    pub fn update_marker_ratio(&mut self, marker_id: &str, new_ratio: f64) {
        for m in &mut self.markers {
            if m.id == marker_id {
                if m.with_ratio_changed(new_ratio) {
                    m.ratio = new_ratio;
                }
                return;
            }
        }
    }

    /// update(int, char[], int) 零 GC 通道。
    /// len 上限 32 (Java arraycopy 前 clamp)。
    /// Java 调用方显式传 len (可短于 buf); Rust 无 stale tail, len 即 buf 全长 —
    /// 本文件唯一调用点 (EngineControl) 两语义等价, 通用 API 语义略窄, 仅记录
    pub fn update_buffer(&mut self, value: i32, buf: &str) {
        self.current_value = value as f64;
        let s: String = buf.chars().take(32).collect();
        self.value_len = s.chars().count() as i32;
        self.value_buffer = s;
    }

    /// update(int, String) 字符串通道: 置 valueLen=0
    pub fn update_display(&mut self, value: i32, display_value: &str) {
        self.current_value = value as f64;
        self.display_value.clear();
        self.display_value.push_str(display_value);
        self.value_len = 0;
    }

    /// 填充像素数: pixVal = Math.round((float)(currentValue*length/maxValue)),
    /// clamp 到 [0, length]; maxValue<=0 → 0 (MarkedGauge 两分支共用)
    pub(crate) fn pix_value(&self, length: i32) -> i32 {
        if self.max_value > 0.0 {
            // Java 先 double 乘除再 (float) 强转再 round — f64 算完 as f32
            let v = java_round_f32(((self.current_value * length as f64) / self.max_value) as f32);
            v.clamp(0, length)
        } else {
            0
        }
    }

    /// clamp: [0,1] 钳制 (NaN 穿透, 与 Java if 链一致)
    fn clamp01(v: f64) -> f64 {
        v.clamp(0.0, 1.0)
    }

    /// draw(g2d, x, y, length, thickness, fontLabel, fontValue) 显式尺寸版
    ///。Java fontLabel 形参在竖/横两分支均未
    /// 参与绘制 (文本一律 fontValue), Rust 单字体参数; 2 参 draw()/getPreferredSize
    /// 为 Swing 尺寸协议 (D7 弃译清单), 不迁。
    #[allow(clippy::too_many_arguments)] // 对齐 Java draw(g2d,x,y,length,thickness,fontLabel,fontValue)
    pub fn draw(
        &mut self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        length: i32,
        thickness: i32,
        font_value: &LoadedFont,
        aa: bool,
    ) {
        if self.bar_style.vertical {
            self.draw_vertical(cv, x, y, length, thickness, font_value, aa);
        } else {
            self.draw_horizontal(cv, x, y, length, thickness, font_value, aa);
        }
    }

    /// drawVertical
    #[allow(clippy::too_many_arguments)] // 对齐 Java drawVertical(g2d,x,y,length,thickness,fontLabel,fontValue)
    fn draw_vertical(
        &mut self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        length: i32,
        thickness: i32,
        font_value: &LoadedFont,
        aa: bool,
    ) {
        let pix_val = self.pix_value(length);
        let text_width = self.value_width(font_value);
        let label_spacing = 2;
        let bar_x = x + text_width + label_spacing; // 文本左, 条右
        let style = self.bar_style;

        // 1. LINE_PARTIAL 标记垫底 (被条盖住)
        for m in &self.markers {
            if m.is_visible() && m.marker_type == MarkerType::LinePartial {
                Self::draw_marker_vertical(
                    cv,
                    bar_x,
                    y,
                    length,
                    thickness,
                    m,
                    style,
                    self.tick_font.as_deref(),
                    aa,
                );
            }
        }
        // 2. 背景条
        cv.fill_rect(bar_x, y, thickness, length, style.background_color);
        // 3. 填充段 (自底向上)
        if pix_val > 0 {
            cv.fill_rect(
                bar_x,
                y + length - pix_val,
                thickness,
                pix_val,
                style.fill_color,
            );
        }
        // 4/5. ZONE 与 LINE_FULL 标记 (叠在条上)
        for m in &self.markers {
            if m.is_visible() && m.marker_type == MarkerType::Zone {
                Self::draw_marker_vertical(
                    cv,
                    bar_x,
                    y,
                    length,
                    thickness,
                    m,
                    style,
                    self.tick_font.as_deref(),
                    aa,
                );
            }
        }
        for m in &self.markers {
            if m.is_visible() && m.marker_type == MarkerType::LineFull {
                Self::draw_marker_vertical(
                    cv,
                    bar_x,
                    y,
                    length,
                    thickness,
                    m,
                    style,
                    self.tick_font.as_deref(),
                    aa,
                );
            }
        }
        // 6. 分隔线 + 数值文本 (随值移动); Java 此处显式 setStroke(borderStroke)
        let sep_y = y + length - 1 - pix_val;
        let text_color = style.fill_color;
        let total_width = text_width + label_spacing + thickness;
        Self::draw_separator(cv, x, sep_y, total_width, text_color);
        self.draw_value_text(cv, x, sep_y - 1, font_value, text_color, aa);
        // 7. 边框
        if style.show_border {
            ring1px(cv, bar_x, y, thickness - 1, length - 1, style.border_color);
        }
    }

    /// drawHorizontal
    #[allow(clippy::too_many_arguments)] // 对齐 Java drawHorizontal(g2d,x,y,length,thickness,fontLabel,fontValue)
    fn draw_horizontal(
        &mut self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        length: i32,
        thickness: i32,
        font_value: &LoadedFont,
        aa: bool,
    ) {
        let pix_val = self.pix_value(length);
        let style = self.bar_style;

        // 1. LINE_PARTIAL 标记垫底
        for m in &self.markers {
            if m.is_visible() && m.marker_type == MarkerType::LinePartial {
                Self::draw_marker_horizontal(
                    cv,
                    x,
                    y,
                    length,
                    thickness,
                    m,
                    style,
                    self.tick_font.as_deref(),
                    aa,
                );
            }
        }
        // 2. 背景条
        cv.fill_rect(x, y, length, thickness, style.background_color);
        // 3. 填充段 (自左向右)
        if pix_val > 0 {
            cv.fill_rect(x, y, pix_val, thickness, style.fill_color);
        }
        // 4/5. ZONE 与 LINE_FULL 标记
        let mut tick_stroke_set = false; // 画过标记 → g2d stroke 已被置为 tickStroke
        for m in &self.markers {
            if m.is_visible() && m.marker_type == MarkerType::Zone {
                Self::draw_marker_horizontal(
                    cv,
                    x,
                    y,
                    length,
                    thickness,
                    m,
                    style,
                    self.tick_font.as_deref(),
                    aa,
                );
                tick_stroke_set = true;
            }
        }
        for m in &self.markers {
            if m.is_visible() && m.marker_type == MarkerType::LineFull {
                Self::draw_marker_horizontal(
                    cv,
                    x,
                    y,
                    length,
                    thickness,
                    m,
                    style,
                    self.tick_font.as_deref(),
                    aa,
                );
                tick_stroke_set = true;
            }
        }
        // 6. 分隔线: 影线 x+pixVal+1 / 主线 x+pixVal, 行 y..y+sepHeight
        let sep_height = thickness + font_value.size + 2;
        let text_color = style.fill_color;
        // Java 此处未 setStroke — 分隔线线宽承袭: 本帧画过标记则 tickStroke
        // (createPreciseStroke(strokeWidth), CAP_BUTT); 否则承袭进入时遗留 stroke
        // (调用链前置组件的 1px 族, 见 gauges_bars LabeledLinearGauge 横向同款注)。
        // 按此确定性复刻两条路径
        if tick_stroke_set {
            butt_line(
                cv,
                x + pix_val + 1,
                y,
                x + pix_val + 1,
                y + sep_height,
                style.stroke_width,
                colors().shade_shape,
                aa,
            );
            butt_line(
                cv,
                x + pix_val,
                y,
                x + pix_val,
                y + sep_height,
                style.stroke_width,
                text_color,
                aa,
            );
        } else {
            vline_1px(cv, x + pix_val + 1, y, y + sep_height, colors().shade_shape);
            vline_1px(cv, x + pix_val, y, y + sep_height, text_color);
        }
        // 7. 数值文本 (条下方)
        self.draw_value_text(
            cv,
            x + pix_val,
            y + thickness + font_value.size,
            font_value,
            text_color,
            aa,
        );
        // 8. 边框 (borderStroke 显式 set)
        if style.show_border {
            ring1px(cv, x, y, length - 1, thickness - 1, style.border_color);
        }
    }

    /// drawMarkerVertical: Y 位置 bottom=0 top=1
    #[allow(clippy::too_many_arguments)] // 对齐 Java drawMarkerVertical(g2d,barX,barY,length,thickness,m) + tickFont
    fn draw_marker_vertical(
        cv: &mut PixCanvas,
        bar_x: i32,
        bar_y: i32,
        length: i32,
        thickness: i32,
        m: &GaugeMarker,
        style: GaugeBarStyle,
        tick_font: Option<&LoadedFont>,
        aa: bool,
    ) {
        // Java (int)(length * clamp(ratio)) 截断向零
        let marker_y = bar_y + length - (length as f64 * Self::clamp01(m.ratio)) as i32;
        match m.marker_type {
            MarkerType::LineFull => {
                butt_line(
                    cv,
                    bar_x,
                    marker_y,
                    bar_x + thickness,
                    marker_y,
                    style.stroke_width,
                    m.color,
                    aa,
                );
            }
            MarkerType::LinePartial => {
                let line_width = (thickness as f32 * m.width_ratio) as i32;
                if m.side < 0 {
                    // 左侧伸入条内
                    butt_line(
                        cv,
                        bar_x - 4,
                        marker_y,
                        bar_x + line_width,
                        marker_y,
                        style.stroke_width,
                        m.color,
                        aa,
                    );
                } else if m.side > 0 {
                    // 右侧伸出条外
                    butt_line(
                        cv,
                        bar_x + thickness - line_width,
                        marker_y,
                        bar_x + thickness + 4,
                        marker_y,
                        style.stroke_width,
                        m.color,
                        aa,
                    );
                } else {
                    let start = bar_x + (thickness - line_width) / 2;
                    butt_line(
                        cv,
                        start,
                        marker_y,
                        start + line_width,
                        marker_y,
                        style.stroke_width,
                        m.color,
                        aa,
                    );
                }
            }
            MarkerType::Zone => {
                let zone_width = (thickness as f32 * m.width_ratio) as i32;
                let zone_height = (length as f64 * Self::clamp01(m.ratio)) as i32;
                let mut zone_x = bar_x;
                if m.side > 0 {
                    zone_x = bar_x + thickness - zone_width;
                } else if m.side == 0 {
                    zone_x = bar_x + (thickness - zone_width) / 2;
                }
                cv.fill_rect(
                    zone_x,
                    bar_y + length - zone_height,
                    zone_width,
                    zone_height,
                    m.color,
                );
            }
            MarkerType::TickLabeled => {
                butt_line(
                    cv,
                    bar_x,
                    marker_y,
                    bar_x + thickness,
                    marker_y,
                    style.stroke_width,
                    m.color,
                    aa,
                );
                if let Some(f) = tick_font {
                    if !m.label.is_empty() {
                        text_shaded_auto(
                            cv,
                            f,
                            bar_x + thickness + 4,
                            marker_y + 4,
                            &m.label,
                            m.color,
                            aa,
                        );
                    }
                }
            }
        }
    }

    /// drawMarkerHorizontal: X 位置 left=0 right=1
    #[allow(clippy::too_many_arguments)] // 对齐 Java drawMarkerHorizontal(g2d,barX,barY,length,thickness,m) + tickFont
    fn draw_marker_horizontal(
        cv: &mut PixCanvas,
        bar_x: i32,
        bar_y: i32,
        length: i32,
        thickness: i32,
        m: &GaugeMarker,
        style: GaugeBarStyle,
        tick_font: Option<&LoadedFont>,
        aa: bool,
    ) {
        let marker_x = bar_x + (length as f64 * Self::clamp01(m.ratio)) as i32;
        match m.marker_type {
            MarkerType::LineFull => {
                butt_line(
                    cv,
                    marker_x,
                    bar_y,
                    marker_x,
                    bar_y + thickness,
                    style.stroke_width,
                    m.color,
                    aa,
                );
            }
            MarkerType::LinePartial => {
                let line_height = (thickness as f32 * m.width_ratio) as i32;
                if m.side < 0 {
                    // 上侧
                    butt_line(
                        cv,
                        marker_x,
                        bar_y - 4,
                        marker_x,
                        bar_y + line_height,
                        style.stroke_width,
                        m.color,
                        aa,
                    );
                } else if m.side > 0 {
                    // 下侧
                    butt_line(
                        cv,
                        marker_x,
                        bar_y + thickness - line_height,
                        marker_x,
                        bar_y + thickness + 4,
                        style.stroke_width,
                        m.color,
                        aa,
                    );
                } else {
                    let start = bar_y + (thickness - line_height) / 2;
                    butt_line(
                        cv,
                        marker_x,
                        start,
                        marker_x,
                        start + line_height,
                        style.stroke_width,
                        m.color,
                        aa,
                    );
                }
            }
            MarkerType::Zone => {
                let zone_height = (thickness as f32 * m.width_ratio) as i32;
                let zone_width = (length as f64 * Self::clamp01(m.ratio)) as i32;
                let mut zone_y = bar_y;
                if m.side > 0 {
                    zone_y = bar_y + thickness - zone_height;
                } else if m.side == 0 {
                    zone_y = bar_y + (thickness - zone_height) / 2;
                }
                cv.fill_rect(bar_x, zone_y, zone_width, zone_height, m.color);
            }
            MarkerType::TickLabeled => {
                butt_line(
                    cv,
                    marker_x,
                    bar_y,
                    marker_x,
                    bar_y + thickness,
                    style.stroke_width,
                    m.color,
                    aa,
                );
                if let Some(f) = tick_font {
                    if !m.label.is_empty() {
                        text_shaded_auto(
                            cv,
                            f,
                            marker_x + 4,
                            bar_y + thickness + f.size,
                            &m.label,
                            m.color,
                            aa,
                        );
                    }
                }
            }
        }
    }

    /// drawSeparator: shade 环 (w×3) + fill 1px 内芯
    fn draw_separator(cv: &mut PixCanvas, x: i32, y: i32, width: i32, c: [u8; 4]) {
        ring1px(cv, x, y, width - 1, 3 - 1, colors().shade_shape);
        cv.fill_rect(x + 1, y + 1, width - 2, 3 - 2, c);
    }

    /// getValueWidth: label + 当前值通道文本宽度;
    /// 字体 None → 30 (Java f==null); label 空 → 0
    fn value_width(&self, font: &LoadedFont) -> i32 {
        let label_w = if self.label.is_empty() {
            0
        } else {
            font.measure(&self.label)
        };
        let value_w = if self.value_len > 0 {
            font.measure(&self.value_buffer)
        } else {
            font.measure(&self.display_value)
        };
        label_w + value_w
    }

    /// drawValueText: label (可选) + 值, 双遍阴影
    fn draw_value_text(
        &self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        font: &LoadedFont,
        c: [u8; 4],
        aa: bool,
    ) {
        let mut label_w = 0;
        if !self.label.is_empty() {
            text_shaded_auto(cv, font, x, y, &self.label, c, aa);
            label_w = font.measure(&self.label);
        }
        if self.value_len > 0 {
            text_shaded_auto(cv, font, x + label_w, y, &self.value_buffer, c, aa);
        } else if !self.display_value.is_empty() {
            text_shaded_auto(cv, font, x + label_w, y, &self.display_value, c, aa);
        }
    }
}
