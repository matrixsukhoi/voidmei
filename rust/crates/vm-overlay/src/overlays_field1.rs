//! overlays_field1: FieldOverlay 系三 overlay 的内容复刻 (渲染函数级, 挂 OverlayHost 渲染闭包)
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | [`PowerInfoState`] | ui/overlay/PowerInfoOverlay.java | BOS 字段网格: 常量表快照 (ui_layout.cfg "动力信息" 段) + FieldOverlay.onFlightData 50ms 节流 + 零 GC 更新路径 + BosStyleRenderer 绘制 |
//! | [`EngineControlState`] | ui/overlay/EngineControlOverlay.java | LabeledLinearGauge 条形仪表 (竖条 throttle/pitch/power + 横条 mixture/radiator/compressor/fuel), COMPRESSOR 走 MarkedGauge 画 optimal 档标记; onFlightData 节流间隔配置驱动 (loadRefreshInterval) |
//! | [`GearFlapsState`] | ui/overlay/GearFlapsOverlay.java | 襟翼竖条 (UIBaseElements.drawVBarTextNum) + 起落架/减速板状态告警文本; onFlightData 100ms 节流 |
//! | [`MarkedGauge`] 族 | ui/component/gauge/{MarkedGauge,GaugeBarStyle,GaugeMarker,MarkerType}.java | 条 + 可插拔标记系统 (LINE_FULL/LINE_PARTIAL/ZONE/TICK_LABELED) |
//!
//! 三者均为 "数据 struct + 内容绘制 fn" 形态: 上层把 state 与画布闭包捕获进
//! [`crate::host::OverlaySpec`] 的 render (FnMut(&mut PixCanvas)) 即挂入 OverlayHost;
//! 文件尾的 `*_preview_spec` 工厂给出预览模式 (静态数据) 的现成闭包。
//!
//! 字段定义快照模式沿用 vm-core `fields.rs` (POC 先例): ui_layout.cfg 对应 panel 段
//! 的 (item :type data ...) 逐行转常量表, 不运行时解析 cfg。
//!
//! 视觉语义逐项对照 Java paintComponent/drawTick/drawGauges; Java char[] 零 GC buffer
//! 统一为 String (gauges_bars 先例, 无 stale tail)。

use crate::global_colors::{aa, colors};
use std::cell::RefCell;
use std::rc::Rc;

use crate::font::LoadedFont;
use crate::gauges_bars::LabeledLinearGauge;
use crate::host::{OverlaySpec, ReinitFn};
use crate::reinit::ReinitParams;
use crate::render2d::PixCanvas;
use crate::renderers::{BosStyleRenderer, Field, OverlayRenderer, RenderContext};
use vm_core::event::EventPayload;
use vm_core::format;
use vm_core::lang::Lang;
// EngineControlOverlay.java:50 DEFAULT_REFRESH_INTERVAL 的既有移植 (单一来源, 勿重复定义)
use vm_core::ui_constants::ENGINE_DEFAULT_REFRESH_MS;
use vm_core::ui_model::{DataField, TelemetrySource};

// ---------------------------------------------------------------------------
// 公共像素基元 (Java Graphics2D 语义; 与 gauges_bars 同源规则的局部实现)
// ---------------------------------------------------------------------------

/// Java Math.round(double) = floor(x+0.5) (PORTING.md §2.3)
fn java_round_f64(x: f64) -> i32 {
    (x + 0.5).floor() as i32
}

/// Java Math.round(float) = floor(x+0.5)
fn java_round_f32(x: f32) -> i32 {
    (x + 0.5).floor() as i32
}

/// Java Graphics.drawRect(x,y,w,h) + BasicStroke(1): 覆盖 x..x+w × y..y+h (含端点)
/// 的 1px 环。负宽/负高整体不绘制, 零宽/零高退化 1px 线 — 语义与 gauges_bars::ring
/// 一致 (crate 内私有, 此处局部复刻供本文件组件使用)
fn ring1px(cv: &mut PixCanvas, x: i32, y: i32, w: i32, h: i32, color: [u8; 4]) {
    if w < 0 || h < 0 {
        return; // PORT: Java drawRect 负宽/负高不绘制
    }
    if w == 0 || h == 0 {
        if w == 0 && h > 0 {
            cv.fill_rect(x, y, 1, h + 1, color);
        } else if h == 0 && w > 0 {
            cv.fill_rect(x, y, w + 1, 1, color);
        }
        return;
    }
    cv.fill_rect(x, y, w + 1, 1, color); // 上边
    cv.fill_rect(x, y + h, w + 1, 1, color); // 下边
    if h > 1 {
        cv.fill_rect(x, y + 1, 1, h - 1, color); // 左边
        cv.fill_rect(x + w, y + 1, 1, h - 1, color); // 右边
    }
}

/// BasicStroke(1) 竖线: 列恰为 x, 行 y0..y1 端点含 (gauges_bars::vline_1px 同款)
fn vline_1px(cv: &mut PixCanvas, x: i32, y0: i32, y1: i32, color: [u8; 4]) {
    let (ya, yb) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    cv.fill_rect(x, ya, 1, yb - ya + 1, color);
}

/// AA 柔边像素的覆盖率缩放 (Java AA 管线 = SrcOver(源 alpha × 覆盖率),
/// gauges_bars::cov_color 同式)
fn cov_color(color: [u8; 4], cov: f32) -> [u8; 4] {
    [color[0], color[1], color[2], ((color[3] as f32) * cov + 0.5) as u8]
}

/// 像素区间 [p, p+1) 与覆盖盒 [lo, hi] 的重叠覆盖率
fn coverage(p: i32, lo: f32, hi: f32) -> f32 {
    let a = (p as f32).max(lo);
    let b = ((p + 1) as f32).min(hi);
    (b - a).clamp(0.0, 1.0)
}

/// BasicStroke(w, CAP_BUTT, JOIN_MITER) 轴对齐线 (GraphicsUtil.createPreciseStroke,
/// MarkedGauge 的 tickStroke/borderStroke 族)。Java 调用点全部轴对齐 (竖/横)。
/// aa=false: 中心规则 — 像素中心落在覆盖盒 [xa,xb]×[y±w/2] 内才点亮 (w=2 水平线 =
/// 行 y-1..y × 列 xa..xb-1, 与 gauges_bars::hline_butt2 文档一致);
/// aa=true: STROKE_NORMALIZE 规整到像素中心后按分离覆盖模型 (cov_x × cov_y)
/// 缩放 alpha (w=2 = 行 y 全值/行 y±1 半值/端点列半覆盖/四角 1/4, 同 hline_butt2)。
#[allow(clippy::too_many_arguments)] // 对齐 Java drawLine(x0,y0,x1,y1)+线宽/色/AA 三元组
fn butt_line(
    cv: &mut PixCanvas,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    w: i32,
    color: [u8; 4],
    aa: bool,
) {
    // PORT: Java BasicStroke(0)=hairline 1px; w<=0 钳到 1 (render2d::stroke_of 同款)
    let w = if w <= 0 { 1 } else { w };
    let half = w as f32 / 2.0;
    let vert = x0 == x1;
    // 沿线方向 (u) 与横截方向 (v) 的整数端点
    let (ua, ub, v) = if vert {
        let (ya, yb) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        (ya, yb, x0)
    } else {
        let (xa, xb) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        (xa, xb, y0)
    };
    if ub <= ua {
        return; // PORT: 零长度 CAP_BUTT 线不绘制 (Java strokedShape 零长度段无输出)
    }
    let put = |cv: &mut PixCanvas, u: i32, vpos: i32, c: [u8; 4]| {
        if vert {
            cv.fill_rect(vpos, u, 1, 1, c);
        } else {
            cv.fill_rect(u, vpos, 1, 1, c);
        }
    };
    if !aa {
        // 中心规则: 沿线像素中心 u+0.5 ∈ [ua, ub] → u ∈ [ua, ub-1];
        // 横截像素中心 v+0.5 ∈ [v-half, v+half] → 逐行判定 (整数边界用含等号)
        let u_hi = ub - 1;
        for u in ua..=u_hi {
            for vv in (v - w - 1)..=(v + w + 1) {
                let c = vv as f32 + 0.5;
                if c >= v as f32 - half && c <= v as f32 + half {
                    put(cv, u, vv, color);
                }
            }
        }
        return;
    }
    // AA: 规整后覆盖盒 沿线 [ua+0.5, ub+0.5] × 横截 [v-half+0.5, v+half+0.5]
    let (alo, ahi) = (ua as f32 + 0.5, ub as f32 + 0.5);
    let (clo, chi) = (v as f32 - half + 0.5, v as f32 + half + 0.5);
    for u in (ua - 1)..=(ub + 1) {
        let cu = coverage(u, alo, ahi);
        if cu <= 0.0 {
            continue;
        }
        for vv in (v - w - 1)..=(v + w + 1) {
            let cvv = coverage(vv, clo, chi);
            if cvv <= 0.0 {
                continue;
            }
            put(cv, u, vv, cov_color(color, cu * cvv));
        }
    }
}

/// 阴影双遍文本 (UIBaseElements.__drawStringShade / MarkedGauge.drawTextShaded):
/// 影 (x+1,y+1) colorShadeShape → 本色 (x,y)
fn text_shaded(
    cv: &mut PixCanvas,
    font: &LoadedFont,
    x: i32,
    y: i32,
    s: &str,
    c: [u8; 4],
    aa: bool,
) {
    cv.draw_text(font, x + 1, y + 1, s, colors().shade_shape, aa);
    cv.draw_text(font, x, y, s, c, aa);
}

// ---------------------------------------------------------------------------
// VisExpr: :visible-when / :na-when 表达式的常量表快照形态
// ---------------------------------------------------------------------------

/// ui_layout.cfg "动力信息" 段实际用到的表达式算子闭包 (求值语义与
/// vm_core::visibility_expression 的 VisibilityExpressionEvaluator 逐算子一致:
/// = / != 带 0.0001 容差; 方法调用走 TelemetrySource)。
/// 完整 SExp 求值器持 `&dyn TelemetrySource` 借用无法进常量表, 故按 fields.rs
/// (POC) 先例把本 panel 用到的表达式树快照为枚举。
#[derive(Debug, Clone, PartialEq)]
pub enum VisExpr {
    /// (isJetEngine)
    IsJetEngine,
    /// (isPistonEngine)
    IsPistonEngine,
    /// (hasWep)
    HasWep,
    /// (hasBooster)
    HasBooster,
    /// (> value N)
    Gt(f64),
    /// (>= value N)
    Gte(f64),
    /// (< value N)
    Lt(f64),
    /// (<= value N)
    Lte(f64),
    /// (= value N) — |value-N| < 0.0001 视为相等 (Java 求值器容差)
    Eq(f64),
    /// (!= value N) — |value-N| >= 0.0001 (Java 求值器容差边界)
    NotEq(f64),
    /// (not e) — 子树为 const 提升的静态引用 (常量表可构造; Box::new 非 const)
    Not(&'static VisExpr),
    /// (and a b)
    And(&'static VisExpr, &'static VisExpr),
}

impl VisExpr {
    /// 求值; value 为字段当前值 (对应 Java evaluator.evaluate(value))
    pub fn eval(&self, s: &dyn TelemetrySource, value: f64) -> bool {
        match self {
            VisExpr::IsJetEngine => s.is_jet_engine(),
            VisExpr::IsPistonEngine => s.is_piston_engine(),
            VisExpr::HasWep => s.has_wep(),
            VisExpr::HasBooster => s.has_booster(),
            VisExpr::Gt(n) => value > *n,
            VisExpr::Gte(n) => value >= *n,
            VisExpr::Lt(n) => value < *n,
            VisExpr::Lte(n) => value <= *n,
            VisExpr::Eq(n) => (value - n).abs() < 0.0001,
            VisExpr::NotEq(n) => (value - n).abs() >= 0.0001,
            VisExpr::Not(e) => !e.eval(s, value),
            VisExpr::And(a, b) => a.eval(s, value) && b.eval(s, value),
        }
    }
}

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
    /// withRatio (GaugeMarker.java:67-73): |new-old|<0.0001 返回自身 (零分配);
    /// Rust 返回 bool 表示是否变化, 拷贝语义由调用方原位赋值等价实现
    pub fn with_ratio_changed(&self, new_ratio: f64) -> bool {
        (new_ratio - self.ratio).abs() >= 0.0001
    }

    /// isVisible (GaugeMarker.java:80): ratio ∈ [0,1] 才绘制
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
            fill_color: [0, 255, 255, 255], // Color.CYAN
            background_color: [64, 64, 64, 255], // Color.DARK_GRAY
            border_color: [128, 128, 128, 255], // Color.GRAY
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
    /// 不画 — MarkedGauge.java:323/376 的 null 守卫)
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

    /// setStyleContext (MarkedGauge.java:75-81)
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

    /// setBarStyle (MarkedGauge.java:83-85)
    pub fn set_bar_style(&mut self, style: GaugeBarStyle) {
        self.bar_style = style;
    }

    /// setMaxValue (MarkedGauge.java:87-89) — double 量程 (0 到 maxValue)
    pub fn set_max_value(&mut self, max_value: f64) {
        self.max_value = max_value;
    }

    /// addMarker (MarkedGauge.java:98-100)
    pub fn add_marker(&mut self, marker: GaugeMarker) {
        self.markers.push(marker);
    }

    /// updateMarkerRatio (MarkedGauge.java:106-121): 按 id 原位更新; 未命中无操作;
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

    /// update(int, char[], int) 零 GC 通道 (MarkedGauge.java:124-131)。
    /// len 上限 32 (Java arraycopy 前 clamp)。
    /// PORT: Java 调用方显式传 len (可短于 buf); Rust 无 stale tail, len 即 buf 全长 —
    /// 本文件唯一调用点 (EngineControl) 两语义等价, 通用 API 语义略窄, 仅记录
    pub fn update_buffer(&mut self, value: i32, buf: &str) {
        self.current_value = value as f64;
        let s: String = buf.chars().take(32).collect();
        self.value_len = s.chars().count() as i32;
        self.value_buffer = s;
    }

    /// update(int, String) 字符串通道 (MarkedGauge.java:138-143): 置 valueLen=0
    pub fn update_display(&mut self, value: i32, display_value: &str) {
        self.current_value = value as f64;
        self.display_value.clear();
        self.display_value.push_str(display_value);
        self.value_len = 0;
    }

    /// 填充像素数: pixVal = Math.round((float)(currentValue*length/maxValue)),
    /// clamp 到 [0, length]; maxValue<=0 → 0 (MarkedGauge 两分支共用)
    fn pix_value(&self, length: i32) -> i32 {
        if self.max_value > 0.0 {
            // PORT: Java 先 double 乘除再 (float) 强转再 round — f64 算完 as f32 (§2.12)
            let v = java_round_f32(
                ((self.current_value * length as f64) / self.max_value) as f32,
            );
            v.clamp(0, length)
        } else {
            0
        }
    }

    /// clamp (MarkedGauge.java:341-345): [0,1] 钳制 (NaN 穿透, 与 Java if 链一致)
    fn clamp01(v: f64) -> f64 {
        v.clamp(0.0, 1.0)
    }

    /// draw(g2d, x, y, length, thickness, fontLabel, fontValue) 显式尺寸版
    /// (MarkedGauge.java:186-197)。PORT: Java fontLabel 形参在竖/横两分支均未
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

    /// drawVertical (MarkedGauge.java:204-265)
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
        let label_spacing = 2; // Java :222
        let bar_x = x + text_width + label_spacing; // 文本左, 条右
        let style = self.bar_style;

        // 1. LINE_PARTIAL 标记垫底 (被条盖住)
        for m in &self.markers {
            if m.is_visible() && m.marker_type == MarkerType::LinePartial {
                Self::draw_marker_vertical(cv, bar_x, y, length, thickness, m, style,
                    self.tick_font.as_deref(), aa);
            }
        }
        // 2. 背景条
        cv.fill_rect(bar_x, y, thickness, length, style.background_color);
        // 3. 填充段 (自底向上)
        if pix_val > 0 {
            cv.fill_rect(bar_x, y + length - pix_val, thickness, pix_val, style.fill_color);
        }
        // 4/5. ZONE 与 LINE_FULL 标记 (叠在条上)
        for m in &self.markers {
            if m.is_visible() && m.marker_type == MarkerType::Zone {
                Self::draw_marker_vertical(cv, bar_x, y, length, thickness, m, style,
                    self.tick_font.as_deref(), aa);
            }
        }
        for m in &self.markers {
            if m.is_visible() && m.marker_type == MarkerType::LineFull {
                Self::draw_marker_vertical(cv, bar_x, y, length, thickness, m, style,
                    self.tick_font.as_deref(), aa);
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

    /// drawHorizontal (MarkedGauge.java:267-329)
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
                Self::draw_marker_horizontal(cv, x, y, length, thickness, m, style,
                    self.tick_font.as_deref(), aa);
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
                Self::draw_marker_horizontal(cv, x, y, length, thickness, m, style,
                    self.tick_font.as_deref(), aa);
                tick_stroke_set = true;
            }
        }
        for m in &self.markers {
            if m.is_visible() && m.marker_type == MarkerType::LineFull {
                Self::draw_marker_horizontal(cv, x, y, length, thickness, m, style,
                    self.tick_font.as_deref(), aa);
                tick_stroke_set = true;
            }
        }
        // 6. 分隔线: 影线 x+pixVal+1 / 主线 x+pixVal, 行 y..y+sepHeight
        let sep_height = thickness + font_value.size + 2;
        let text_color = style.fill_color;
        // PORT: Java 此处未 setStroke — 分隔线线宽承袭: 本帧画过标记则 tickStroke
        // (createPreciseStroke(strokeWidth), CAP_BUTT); 否则承袭进入时遗留 stroke
        // (调用链前置组件的 1px 族, 见 gauges_bars LabeledLinearGauge 横向同款注)。
        // 按此确定性复刻两条路径
        if tick_stroke_set {
            butt_line(cv, x + pix_val + 1, y, x + pix_val + 1, y + sep_height,
                style.stroke_width, colors().shade_shape, aa);
            butt_line(cv, x + pix_val, y, x + pix_val, y + sep_height,
                style.stroke_width, text_color, aa);
        } else {
            vline_1px(cv, x + pix_val + 1, y, y + sep_height, colors().shade_shape);
            vline_1px(cv, x + pix_val, y, y + sep_height, text_color);
        }
        // 7. 数值文本 (条下方)
        self.draw_value_text(cv, x + pix_val, y + thickness + font_value.size,
            font_value, text_color, aa);
        // 8. 边框 (borderStroke 显式 set)
        if style.show_border {
            ring1px(cv, x, y, length - 1, thickness - 1, style.border_color);
        }
    }

    /// drawMarkerVertical (MarkedGauge.java:276-330): Y 位置 bottom=0 top=1
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
        // PORT: Java (int)(length * clamp(ratio)) 截断向零
        let marker_y = bar_y + length - (length as f64 * Self::clamp01(m.ratio)) as i32;
        match m.marker_type {
            MarkerType::LineFull => {
                butt_line(cv, bar_x, marker_y, bar_x + thickness, marker_y,
                    style.stroke_width, m.color, aa);
            }
            MarkerType::LinePartial => {
                let line_width = (thickness as f32 * m.width_ratio) as i32;
                if m.side < 0 {
                    // 左侧伸入条内
                    butt_line(cv, bar_x - 4, marker_y, bar_x + line_width, marker_y,
                        style.stroke_width, m.color, aa);
                } else if m.side > 0 {
                    // 右侧伸出条外
                    butt_line(cv, bar_x + thickness - line_width, marker_y,
                        bar_x + thickness + 4, marker_y, style.stroke_width, m.color, aa);
                } else {
                    let start = bar_x + (thickness - line_width) / 2;
                    butt_line(cv, start, marker_y, start + line_width, marker_y,
                        style.stroke_width, m.color, aa);
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
                cv.fill_rect(zone_x, bar_y + length - zone_height, zone_width, zone_height, m.color);
            }
            MarkerType::TickLabeled => {
                butt_line(cv, bar_x, marker_y, bar_x + thickness, marker_y,
                    style.stroke_width, m.color, aa);
                // Java: m.label != null && tickFont != null 才画 (条右侧)
                if let Some(f) = tick_font {
                    if !m.label.is_empty() {
                        text_shaded(cv, f, bar_x + thickness + 4, marker_y + 4,
                            &m.label, m.color, aa);
                    }
                }
            }
        }
    }

    /// drawMarkerHorizontal (MarkedGauge.java:332-381): X 位置 left=0 right=1
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
                butt_line(cv, marker_x, bar_y, marker_x, bar_y + thickness,
                    style.stroke_width, m.color, aa);
            }
            MarkerType::LinePartial => {
                let line_height = (thickness as f32 * m.width_ratio) as i32;
                if m.side < 0 {
                    // 上侧
                    butt_line(cv, marker_x, bar_y - 4, marker_x, bar_y + line_height,
                        style.stroke_width, m.color, aa);
                } else if m.side > 0 {
                    // 下侧
                    butt_line(cv, marker_x, bar_y + thickness - line_height,
                        marker_x, bar_y + thickness + 4, style.stroke_width, m.color, aa);
                } else {
                    let start = bar_y + (thickness - line_height) / 2;
                    butt_line(cv, marker_x, start, marker_x, start + line_height,
                        style.stroke_width, m.color, aa);
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
                butt_line(cv, marker_x, bar_y, marker_x, bar_y + thickness,
                    style.stroke_width, m.color, aa);
                // Java: markerX+4, barY+thickness+tickFont.getSize()
                if let Some(f) = tick_font {
                    if !m.label.is_empty() {
                        text_shaded(cv, f, marker_x + 4,
                            bar_y + thickness + f.size, &m.label, m.color, aa);
                    }
                }
            }
        }
    }

    /// drawSeparator (MarkedGauge.java:383-390): shade 环 (w×3) + fill 1px 内芯
    fn draw_separator(cv: &mut PixCanvas, x: i32, y: i32, width: i32, c: [u8; 4]) {
        ring1px(cv, x, y, width - 1, 3 - 1, colors().shade_shape);
        cv.fill_rect(x + 1, y + 1, width - 2, 3 - 2, c);
    }

    /// getValueWidth (MarkedGauge.java:392-402): label + 当前值通道文本宽度;
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

    /// drawValueText (MarkedGauge.java:404-420): label (可选) + 值, 双遍阴影
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
            text_shaded(cv, font, x, y, &self.label, c, aa);
            label_w = font.measure(&self.label);
        }
        if self.value_len > 0 {
            text_shaded(cv, font, x + label_w, y, &self.value_buffer, c, aa);
        } else if !self.display_value.is_empty() {
            text_shaded(cv, font, x + label_w, y, &self.display_value, c, aa);
        }
    }
}

// ---------------------------------------------------------------------------
// PowerInfoOverlay (ui/overlay/PowerInfoOverlay.java) — 动力信息 BOS 字段网格
// ---------------------------------------------------------------------------

/// 字段取值来源 (ui_layout.cfg :target 的 getter 名映射; 仅 "getFuelTimeMili * 0.001"
/// 一条带算术, 快照为专用变体)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSource {
    HorsePower,
    Thrust,
    Rpm,
    Pitch,
    PropEfficiency,
    EffHp,
    ManifoldPressureDisplay,
    PowerPercent,
    MassFuel,
    TotalWeight,
    /// "getFuelTimeMili * 0.001" (cfg 原样表达式)
    FuelTimeMiliMul001,
    WepKg,
    WepTime,
    BoosterFuelKg,
    BoosterFuelPercent,
    WaterTemp,
    OilTemp,
    HeatTolerance,
    EngineResponse,
}

impl PowerSource {
    /// cfg :target 原名 (ReflectBinder 反射键)
    pub fn getter(self) -> &'static str {
        match self {
            PowerSource::HorsePower => "getHorsePower",
            PowerSource::Thrust => "getThrust",
            PowerSource::Rpm => "getRPM",
            PowerSource::Pitch => "getPitch",
            PowerSource::PropEfficiency => "getPropEfficiency",
            PowerSource::EffHp => "getEffHp",
            PowerSource::ManifoldPressureDisplay => "getManifoldPressureDisplay",
            PowerSource::PowerPercent => "getPowerPercent",
            PowerSource::MassFuel => "getMassFuel",
            PowerSource::TotalWeight => "getTotalWeight",
            PowerSource::FuelTimeMiliMul001 => "getFuelTimeMili",
            PowerSource::WepKg => "getWepKg",
            PowerSource::WepTime => "getWepTime",
            PowerSource::BoosterFuelKg => "getBoosterFuelKg",
            PowerSource::BoosterFuelPercent => "getBoosterFuelPercent",
            PowerSource::WaterTemp => "getWaterTemp",
            PowerSource::OilTemp => "getOilTemp",
            PowerSource::HeatTolerance => "getHeatTolerance",
            PowerSource::EngineResponse => "getEngineResponse",
        }
    }

    /// ReflectBinder.resolveDouble(s, property) 的 match 注册表等价物 (禁反射, POC 先例)
    fn get(self, s: &dyn TelemetrySource) -> f64 {
        match self {
            PowerSource::HorsePower => s.get_horse_power(),
            PowerSource::Thrust => s.get_thrust(),
            PowerSource::Rpm => s.get_rpm(),
            PowerSource::Pitch => s.get_pitch(),
            PowerSource::PropEfficiency => s.get_prop_efficiency(),
            PowerSource::EffHp => s.get_eff_hp(),
            PowerSource::ManifoldPressureDisplay => s.get_manifold_pressure_display(),
            PowerSource::PowerPercent => s.get_power_percent(),
            PowerSource::MassFuel => s.get_mass_fuel(),
            PowerSource::TotalWeight => s.get_total_weight(),
            // PORT: cfg 表达式 "getFuelTimeMili * 0.001" (int 毫秒 → 秒)
            PowerSource::FuelTimeMiliMul001 => s.get_fuel_time_mili() as f64 * 0.001,
            PowerSource::WepKg => s.get_wep_kg(),
            PowerSource::WepTime => s.get_wep_time(),
            PowerSource::BoosterFuelKg => s.get_booster_fuel_kg(),
            PowerSource::BoosterFuelPercent => s.get_booster_fuel_percent(),
            PowerSource::WaterTemp => s.get_water_temp(),
            PowerSource::OilTemp => s.get_oil_temp(),
            PowerSource::HeatTolerance => s.get_heat_tolerance(),
            PowerSource::EngineResponse => s.get_engine_response(),
        }
    }
}

/// :unit-source / :precision-source 动态通道 (cfg 全表仅进气压一条使用)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynSource {
    /// "getManifoldPressureDisplayUnit" / "...Precision"
    ManifoldDisplay,
}

impl DynSource {
    /// 动态单位 (Java unitSupplier)
    fn unit(self, s: &dyn TelemetrySource) -> String {
        match self {
            DynSource::ManifoldDisplay => s.get_manifold_pressure_display_unit(),
        }
    }

    /// 动态精度 (Java precisionSupplier)
    fn precision(self, s: &dyn TelemetrySource) -> i32 {
        match self {
            DynSource::ManifoldDisplay => s.get_manifold_pressure_display_precision(),
        }
    }
}

/// :format 自定义格式 (cfg 仅 "TIME_MM_SS")
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerFormat {
    Plain,
    /// TIME_MM_SS → FastNumberFormatter.formatTime
    TimeMmSs,
}

/// 单个数据行定义 (ui_layout.cfg panel "动力信息" 性能数据组的 (item :type data) 行)
#[derive(Debug, Clone, PartialEq)]
pub struct PowerFieldDef {
    /// :target-name (全角/双空格对齐原样保留)
    pub label: &'static str,
    pub unit: &'static str,
    /// :preview-value 原样字符串 (preview 恒可见, 不经格式化)
    pub preview_value: &'static str,
    pub source: PowerSource,
    /// :precision (缺省 0)
    pub precision: u8,
    pub format: PowerFormat,
    pub visible_when: Option<VisExpr>,
    pub na_when: Option<VisExpr>,
    /// :unit-source
    pub unit_source: Option<DynSource>,
    /// :precision-source
    pub precision_source: Option<DynSource>,
}

/// ui_layout.cfg L151-170 "性能数据" 组逐行快照 (19 项, 顺序一致)
pub const POWER_FIELD_DEFS: &[PowerFieldDef] = &[
    PowerFieldDef {
        label: "功  率", unit: "Hp", preview_value: "1200",
        source: PowerSource::HorsePower, precision: 0, format: PowerFormat::Plain,
        visible_when: Some(VisExpr::Not(&VisExpr::IsJetEngine)),
        na_when: Some(VisExpr::Lte(0.0)),
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "推  力", unit: "Kgf", preview_value: "1000",
        source: PowerSource::Thrust, precision: 0, format: PowerFormat::Plain,
        visible_when: None, na_when: None,
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "转  速", unit: "Rpm", preview_value: "2400",
        source: PowerSource::Rpm, precision: 0, format: PowerFormat::Plain,
        visible_when: None, na_when: None,
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "桨距角", unit: "Deg", preview_value: "55",
        source: PowerSource::Pitch, precision: 1, format: PowerFormat::Plain,
        visible_when: Some(VisExpr::Not(&VisExpr::IsJetEngine)),
        na_when: Some(VisExpr::Eq(-65535.0)),
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "桨效率", unit: "%", preview_value: "85",
        source: PowerSource::PropEfficiency, precision: 1, format: PowerFormat::Plain,
        visible_when: Some(VisExpr::Not(&VisExpr::IsJetEngine)),
        na_when: Some(VisExpr::Lte(0.0)),
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "实功率", unit: "Hp", preview_value: "1100",
        source: PowerSource::EffHp, precision: 0, format: PowerFormat::Plain,
        visible_when: Some(VisExpr::Not(&VisExpr::IsJetEngine)),
        na_when: Some(VisExpr::Lte(0.0)),
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "进气压", unit: "Ata", preview_value: "1.2",
        source: PowerSource::ManifoldPressureDisplay, precision: 2, format: PowerFormat::Plain,
        visible_when: Some(VisExpr::And(&VisExpr::IsPistonEngine, &VisExpr::NotEq(1.0))),
        na_when: None,
        unit_source: Some(DynSource::ManifoldDisplay),
        precision_source: Some(DynSource::ManifoldDisplay),
    },
    PowerFieldDef {
        label: "动力量", unit: "%", preview_value: "95",
        source: PowerSource::PowerPercent, precision: 0, format: PowerFormat::Plain,
        visible_when: None, na_when: None,
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "燃油量", unit: "Kg", preview_value: "500",
        source: PowerSource::MassFuel, precision: 0, format: PowerFormat::Plain,
        visible_when: None, na_when: None,
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "总  重", unit: "Kg", preview_value: "3500",
        source: PowerSource::TotalWeight, precision: 0, format: PowerFormat::Plain,
        visible_when: Some(VisExpr::Gt(0.0)), na_when: None,
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "燃油时", unit: "s", preview_value: "45",
        source: PowerSource::FuelTimeMiliMul001, precision: 0, format: PowerFormat::TimeMmSs,
        visible_when: None, na_when: None,
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "加力量", unit: "Kg", preview_value: "50",
        source: PowerSource::WepKg, precision: 0, format: PowerFormat::Plain,
        visible_when: Some(VisExpr::HasWep), na_when: None,
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "加力时", unit: "s", preview_value: "300",
        source: PowerSource::WepTime, precision: 0, format: PowerFormat::TimeMmSs,
        visible_when: Some(VisExpr::And(&VisExpr::HasWep, &VisExpr::Gt(0.0))),
        na_when: None,
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "助推燃料", unit: "Kg", preview_value: "850",
        source: PowerSource::BoosterFuelKg, precision: 1, format: PowerFormat::Plain,
        visible_when: Some(VisExpr::HasBooster), na_when: None,
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "助推余量", unit: "%", preview_value: "100",
        source: PowerSource::BoosterFuelPercent, precision: 0, format: PowerFormat::Plain,
        visible_when: Some(VisExpr::HasBooster), na_when: None,
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "温  度", unit: "C", preview_value: "90",
        source: PowerSource::WaterTemp, precision: 0, format: PowerFormat::Plain,
        visible_when: None,
        na_when: Some(VisExpr::Lte(-65535.0)),
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "油  温", unit: "C", preview_value: "80",
        source: PowerSource::OilTemp, precision: 0, format: PowerFormat::Plain,
        visible_when: None, na_when: None,
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "耐热时", unit: "S", preview_value: "60",
        source: PowerSource::HeatTolerance, precision: 0, format: PowerFormat::Plain,
        visible_when: None,
        na_when: Some(VisExpr::Gt(90000.0)),
        unit_source: None, precision_source: None,
    },
    PowerFieldDef {
        label: "响应速", unit: "%/s", preview_value: "10",
        source: PowerSource::EngineResponse, precision: 0, format: PowerFormat::Plain,
        visible_when: None, na_when: None,
        unit_source: None, precision_source: None,
    },
];

/// Throttling to prevent EDT task accumulation (FieldOverlay.java:37-38
/// REFRESH_INTERVAL_MS)
pub const FIELD_OVERLAY_REFRESH_INTERVAL_MS: i64 = 50;

/// 动力信息面板状态 (Java PowerInfoOverlay 的 fieldManager + bindDynamicFields 产物)。
/// 预览 = 构造后不调 update (FieldOverlay.initPreview 不订阅事件, 字段保持 previewValue)。
pub struct PowerInfoState {
    /// 节流基准 (FieldOverlay.java:39 lastRefreshTime, System.currentTimeMillis 毫秒)
    pub last_refresh_time: i64,
    /// DataField 承接 (visible/buffer/length/precision/unit 与 BosStyleRenderer 的
    /// Field::Data 通道天然对接)
    fields: Vec<DataField>,
}

impl Default for PowerInfoState {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerInfoState {
    /// initFields (FieldOverlay.java:145-155) + DefaultFieldManager.addField:
    /// currentValue = previewValue 原样 (不经 %5s), hideWhenNA=true (EngineInfoConfig
    /// populateFromGroup 固定传 true), hideWhenZero=false (cfg 无 :hide-when-zero)
    pub fn new() -> Self {
        let fields = POWER_FIELD_DEFS
            .iter()
            .map(|def| {
                let mut f = DataField::new(
                    def.source.getter(),
                    def.label,
                    def.unit,
                    def.source.getter(), // configKey = property (EngineInfoConfig.populateFromGroup)
                    true,
                    false,
                );
                f.current_value = def.preview_value.to_string();
                f.precision = def.precision as i32;
                f
            })
            .collect();
        PowerInfoState {
            last_refresh_time: 0, // Java :39 隐式 0 初始化 (§2.10)
            fields,
        }
    }

    pub fn fields(&self) -> &[DataField] {
        &self.fields
    }

    /// 数据面回 previewValue 静态 (Java closeAll = 实例销毁 + refreshPreview
    /// 工厂新建 initPreview 实例的 initFields 段; D8 host 单条目跨重建存活的
    /// 补口 — live 会话残留的 buffer/length 在 preview 重开前清除, 否则预览窗
    /// 显示上次 live 数值而非 previewValue)。reinit 闭包只重建 RenderContext
    /// (字体/列度量), 不动数据面, 故此处显式重置。
    pub fn reset_preview(&mut self) {
        *self = Self::new();
    }

    /// FieldOverlay.onFlightData (FieldOverlay.java:166-217) 的单事件语义:
    /// 50ms 节流闩 → (invokeLater lambda 内) 零 GC 路径 (:178-217): 取值 →
    /// visible-when → 动态精度 → 动态单位 → 可见时格式化 (na-when → "-",
    /// TIME_MM_SS → formatTime, 其余 format(val, precision))。
    /// PORT: System.currentTimeMillis 由调用方注入 now_ms (field2 先例, 便于测试);
    /// 返回值 = 是否执行了更新 (false = 节流跳过, Java 原方法 void, 宿主可据此省重绘)
    pub fn update(&mut self, now_ms: i64, s: &dyn TelemetrySource) -> bool {
        // Throttling prevents EDT task accumulation
        if now_ms - self.last_refresh_time < FIELD_OVERLAY_REFRESH_INTERVAL_MS {
            return false; // Skip this update, too soon
        }
        self.last_refresh_time = now_ms;
        for (def, field) in POWER_FIELD_DEFS.iter().zip(self.fields.iter_mut()) {
            // 1. 取值 (visibilitySupplier 求值需要)
            let val = def.source.get(s);
            // 2. 可见性: 无 :visible-when 恒可见 (PowerInfoOverlay.java:147)
            field.visible = def.visible_when.as_ref().is_none_or(|e| e.eval(s, val));
            // 3. 动态精度 (仅变化时写)
            if let Some(ds) = def.precision_source {
                let new_precision = ds.precision(s);
                if new_precision != field.precision {
                    field.precision = new_precision;
                }
            }
            // 4. 动态单位 (仅变化时写)
            if let Some(ds) = def.unit_source {
                let new_unit = ds.unit(s);
                if new_unit != field.unit {
                    field.set_unit(&new_unit);
                }
            }
            // 5. 可见才格式化
            if field.visible {
                if let Some(e) = def.na_when.as_ref() {
                    if e.eval(s, val) {
                        // NA 条件满足, 显示 "-"
                        field.buffer.clear();
                        field.buffer.push('-');
                        field.length = 1;
                        continue;
                    }
                }
                match def.format {
                    PowerFormat::TimeMmSs => {
                        field.buffer = format::format_time(val);
                    }
                    PowerFormat::Plain => {
                        field.buffer = format::format(val, field.precision as u8);
                    }
                }
                // 缓冲内容为 ASCII 数字域, 字符数 = UTF-16 码元数 (§2.1)
                field.length = field.buffer.chars().count() as i32;
            }
        }
        true
    }

    /// 首选尺寸 = BosStyleRenderer.calculatePreferredSize (只读 ctx + 可见计数,
    /// 无渲染器状态参与 — BOSStyleRenderer.java:86-87)
    pub fn preferred_size(&self, ctx: &RenderContext) -> (i32, i32) {
        let visible = self.fields.iter().filter(|f| f.visible).count() as i32;
        (ctx.geom.total_width(), ctx.geom.total_height(visible))
    }

    /// 内容绘制 (FieldOverlay.paintComponent → renderer.render; PowerInfo 的
    /// createRenderer = BOSStyleRenderer)
    pub fn draw(&self, cv: &mut PixCanvas, ctx: &RenderContext, renderer: &mut BosStyleRenderer) {
        // PORT: Java BosStyleRenderer 直接迭代 fieldManager 列表零分配; Rust render
        // 契约收 `&[Field]` 且 Field 借用 DataField — 缓冲无法与 state 同域复用
        // (state 内自引用 / 渲染闭包内不变性, 均编译期否决), 故每帧 collect 19 项
        // (20Hz 下一笔小分配)。零分配化需 render 契约改迭代器/Rc 化 — 留惯用化 pass
        let fields: Vec<Field> = self.fields.iter().map(Field::Data).collect();
        let mut offset = [0, 0];
        OverlayRenderer::render(renderer, cv, &fields, ctx, &mut offset);
    }
}

// ---------------------------------------------------------------------------
// EngineControlOverlay (ui/overlay/EngineControlOverlay.java) — 引擎控制条形仪表
// ---------------------------------------------------------------------------

/// EngineControlOverlay.java:54-56 GaugeType 枚举 (ordinal 即 gaugeType 字段值)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeType {
    Throttle,
    Pitch,
    Power,
    Mixture,
    Radiator,
    Compressor,
    Fuel,
}

impl GaugeType {
    /// Java GaugeType.values()[gf.gaugeType] 反查
    pub fn from_ordinal(o: i32) -> GaugeType {
        match o {
            0 => GaugeType::Throttle,
            1 => GaugeType::Pitch,
            2 => GaugeType::Power,
            3 => GaugeType::Mixture,
            4 => GaugeType::Radiator,
            5 => GaugeType::Compressor,
            _ => GaugeType::Fuel,
        }
    }

    pub fn ordinal(self) -> i32 {
        match self {
            GaugeType::Throttle => 0,
            GaugeType::Pitch => 1,
            GaugeType::Power => 2,
            GaugeType::Mixture => 3,
            GaugeType::Radiator => 4,
            GaugeType::Compressor => 5,
            GaugeType::Fuel => 6,
        }
    }
}

/// Lang 标签访问器 (cfg 无 lang 快照, EngineControl 标签全部来自 Lang 静态字段)
fn lbl_throttle(l: &Lang) -> &'static str {
    l.e_throttle
}
fn lbl_proppitch(l: &Lang) -> &'static str {
    l.e_proppitch
}
fn lbl_power_percent(l: &Lang) -> &'static str {
    l.e_power_percent
}
fn lbl_mixture(l: &Lang) -> &'static str {
    l.e_mixture
}
fn lbl_radiator(l: &Lang) -> &'static str {
    l.e_radiator
}
fn lbl_compressor(l: &Lang) -> &'static str {
    l.e_compressor
}
fn lbl_fuel_per(l: &Lang) -> &'static str {
    l.e_fuel_per
}

/// 单个仪表定义 (EngineControlOverlay.initGaugeFields 的 addGaugeIfEnabled 参数快照,
/// ui_layout.cfg "引擎控制"→"发动机元素" 组的 switch-inv :target 即 disableKey)。
/// PORT: 无 PartialEq — label 是 fn 指针, 地址比较无意义 (rustc 同款告警)
#[derive(Debug, Clone, Copy)]
pub struct EngineGaugeDef {
    /// 开关键 ("true" 时该仪表不建)
    pub disable_key: &'static str,
    /// 字段键 (GaugeField key)
    pub key: &'static str,
    /// Lang 标签访问器
    pub label: fn(&Lang) -> &'static str,
    pub unit: &'static str,
    pub gauge_type: GaugeType,
    pub max_value: i32,
    pub is_horizontal: bool,
}

/// initGaugeFields (EngineControlOverlay.java:224-244) 的 7 条定义, 顺序原样
/// 7 仪表 disable 键 (ENGINE_GAUGE_DEFS 顺序; Java initGaugeFields 读
/// ui_layout.cfg:185-191 发动机元素组 switch-inv, 审查轮 1-B: 曾 never-wired
/// 恒显全部 7 条 — vm-app 经 OverlayInputs 按此表序传 [bool; 7])
pub const ENGINE_DISABLE_KEYS: [&str; 7] = [
    "disableEngineInfoThrottle",
    "disableEngineInfoPitch",
    "disableEngineInfoPower",
    "disableEngineInfoMixture",
    "disableEngineInfoRadiator",
    "disableEngineInfoCompressor",
    "disableEngineInfoLFuel",
];

pub const ENGINE_GAUGE_DEFS: &[EngineGaugeDef] = &[
    EngineGaugeDef {
        disable_key: "disableEngineInfoThrottle", key: "throttle",
        label: lbl_throttle, unit: "%",
        gauge_type: GaugeType::Throttle, max_value: 110, is_horizontal: false,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoPitch", key: "pitch",
        label: lbl_proppitch, unit: "%",
        gauge_type: GaugeType::Pitch, max_value: 100, is_horizontal: false,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoPower", key: "power",
        label: lbl_power_percent, unit: "%",
        gauge_type: GaugeType::Power, max_value: 100, is_horizontal: false,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoMixture", key: "mixture",
        label: lbl_mixture, unit: "%",
        gauge_type: GaugeType::Mixture, max_value: 120, is_horizontal: true,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoRadiator", key: "radiator",
        label: lbl_radiator, unit: "%",
        gauge_type: GaugeType::Radiator, max_value: 100, is_horizontal: true,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoCompressor", key: "compressor",
        label: lbl_compressor, unit: "",
        gauge_type: GaugeType::Compressor, max_value: 1, is_horizontal: true,
    },
    EngineGaugeDef {
        disable_key: "disableEngineInfoLFuel", key: "fuel",
        label: lbl_fuel_per, unit: "%",
        gauge_type: GaugeType::Fuel, max_value: 100, is_horizontal: true,
    },
];

/// EngineControlOverlay.java:47-49 常量
const BASE_FONT_SIZE: i32 = 24;
const WIDTH_MULTIPLIER: i32 = 8;
/// serviceLoopIntervalMs * 2 (EngineControlOverlay.java:51 ENGINE_REFRESH_MULTIPLIER);
/// 默认间隔 100ms 复用 vm_core::ui_constants::ENGINE_DEFAULT_REFRESH_MS (:50, 单一来源)
pub const ENGINE_REFRESH_MULTIPLIER: f64 = 2.0;

/// 单个仪表的运行态 (Java GaugeField + 其 Swing 组件; Rust 组件直接拥有)
pub struct EngineGauge {
    pub key: String,
    pub gauge_type: GaugeType,
    pub max_value: i32,
    pub is_horizontal: bool,
    /// 动态可见性 (PITCH/MIXTURE 无数据、COMPRESSOR 0 档时隐藏)
    pub visible: bool,
    /// Java GaugeField.gauge (LabeledLinearGauge 组件)
    pub gauge: LabeledLinearGauge,
    /// Java GaugeField.markedGauge (COMPRESSOR 专用)
    pub marked_gauge: Option<MarkedGauge>,
}

/// isJetHiddenGauge (EngineControlOverlay.java:356-361)
fn is_jet_hidden_gauge(t: GaugeType) -> bool {
    matches!(
        t,
        GaugeType::Pitch | GaugeType::Radiator | GaugeType::Compressor | GaugeType::Mixture
    )
}

/// 引擎控制面板状态: 布局 (fontsize/width/height) + 仪表表 + 喷气机门控状态机。
/// 数据更新走 [`EngineControlState::update`] (onFlightData 路径), 绘制走
/// [`EngineControlState::draw`] (paintComponent → drawGauges)。
pub struct EngineControlState {
    /// loadFontConfig: round((24+fontadd)*dpiScale)
    pub font_size: i32,
    /// calculateLayout: fontsize*WIDTH_MULTIPLIER
    pub width: i32,
    /// calculateLayout: (移位优先级陷阱见 new 内注释)
    pub height: i32,
    /// 横向仪表数 (calculateLayout 的高度公式项; Java 包内可见, reinitConfig 复算)
    pub row_num: i32,
    /// 节流间隔 ms (EngineControlOverlay.java:61 refreshInterval, loadRefreshInterval
    /// 配置驱动: dataPollIntervalMs×2 → legacy "Interval"×2, 双键空保持默认 100)
    pub refresh_interval: i64,
    /// 节流基准 (:62 lastRefreshTime, System.currentTimeMillis 毫秒)
    pub last_refresh_time: i64,
    gauges: Vec<EngineGauge>,
    is_jet: bool,
    /// 引擎类型检测一次性闩锁 (updateStateFromPayload)
    jet_label_updated: bool,
    /// 增压器量程一次性写入闩锁
    compressor_max_value_set: bool,
}

impl EngineControlState {
    /// init → reinitConfig 链: loadFontConfig + loadRefreshInterval + initGaugeFields +
    /// calculateLayout + 末尾 updateGaugesPreview (:179-188)。
    /// cfg_true = `"true".equals(getConfigSafe(key))` 的配置探测 (POC 未接配置层,
    /// 恒 false 即全启用; 接配置层时传入 Boolean.parseBoolean 语义);
    /// cfg_str = getConfigSafe 的字符串读取 (loadRefreshInterval 专用, POC 传空串
    /// 读取器即恒默认间隔)
    pub fn new(
        lang: &Lang,
        font_add: i32,
        dpi_scale: f64,
        cfg_true: &dyn Fn(&str) -> bool,
        cfg_str: &dyn Fn(&str) -> String,
    ) -> Self {
        // loadFontConfig (EngineControlOverlay.java:191-200); label 字体由 draw 的
        // 调用方持有 (Java fontLabel 字段)
        let font_size = java_round_f64((BASE_FONT_SIZE as f64 + font_add as f64) * dpi_scale);
        // initGaugeFields (:224-244)
        let mut gauges = Vec::new();
        let mut row_num = 0;
        for def in ENGINE_GAUGE_DEFS {
            // addGaugeIfEnabled: !"true".equals(getConfigSafe(disableKey)) 才建
            if cfg_true(def.disable_key) {
                continue;
            }
            let label = (def.label)(lang);
            let gauge = EngineGauge {
                key: def.key.to_string(),
                gauge_type: def.gauge_type,
                max_value: def.max_value,
                is_horizontal: def.is_horizontal,
                visible: true,
                // GaugeField 构造: new LabeledLinearGauge(label, maxValue, !isHorizontal)
                gauge: LabeledLinearGauge::new(label, def.max_value, !def.is_horizontal),
                marked_gauge: None,
            };
            let mut gauge = gauge;
            // COMPRESSOR 用 MarkedGauge 画 optimal 档指示 (addGaugeIfEnabled 内)
            if def.gauge_type == GaugeType::Compressor {
                let style = GaugeBarStyle {
                    fill_color: colors().num,
                    background_color: [0, 0, 0, 0], // 透明背景
                    border_color: colors().shade_shape,
                    show_border: true,
                    vertical: !def.is_horizontal, // COMPRESSOR 横条 → vertical=false
                    stroke_width: 2,
                };
                let mut mg = MarkedGauge::new();
                mg.label = label.to_string();
                mg.set_max_value(def.max_value as f64);
                mg.set_bar_style(style);
                // optimal 档标记 (初始 ratio=-1 隐藏, colorWarning)
                mg.add_marker(GaugeMarker {
                    id: "optimal".to_string(),
                    marker_type: MarkerType::LineFull,
                    ratio: -1.0,
                    color: colors().warning,
                    ..GaugeMarker::default()
                });
                gauge.marked_gauge = Some(mg);
            }
            gauges.push(gauge);
            if def.is_horizontal {
                row_num += 1;
            } else {
                // columnNum 计数后从未参与公式 (Java 同, 仅循环结构保留)
            }
        }
        // calculateLayout (:214-222)
        let width = font_size * WIDTH_MULTIPLIER;
        // PORT: Java `(fontsize * 4 + (fontsize * 9) >> 1)` — JLS 移位优先级低于加法
        // → (13*fontsize)>>1 (LinearGaugeRenderer.java:71 同款陷阱, 勿加括号)
        let height =
            ((font_size * 4 + font_size * 9) >> 1) + (row_num + 1) * (font_size + (font_size >> 2));
        let mut st = EngineControlState {
            font_size,
            width,
            height,
            row_num,
            refresh_interval: ENGINE_DEFAULT_REFRESH_MS, // Java :61 字段初始 DEFAULT_REFRESH_INTERVAL
            last_refresh_time: 0, // Java :62 隐式 0 初始化 (§2.10)
            gauges,
            is_jet: false,
            jet_label_updated: false,
            compressor_max_value_set: false,
        };
        // reinitConfig 链内 loadRefreshInterval (:179/:202-212)
        st.load_refresh_interval(cfg_str);
        // reinitConfig 末尾 updateGaugesPreview (:187): 游戏模式与预览共用此初值 —
        // 全仪表 maxValue/2 且可见, 首个有效事件 (引擎检测 ~5s) 前显示半量程条
        st.update_preview();
        st
    }

    /// loadRefreshInterval (EngineControlOverlay.java:202-212): 先取
    /// dataPollIntervalMs, 空则回退 legacy "Interval"; 两键皆空保持现值 (默认 100)。
    /// reinit 时宿主可再次调用以随配置更新间隔
    pub fn load_refresh_interval(&mut self, cfg_str: &dyn Fn(&str) -> String) {
        // Try new config key first, fallback to legacy key for backward compatibility
        let mut interval_val = cfg_str("dataPollIntervalMs");
        if interval_val.is_empty() {
            interval_val = cfg_str("Interval"); // Legacy key fallback
        }
        if !interval_val.is_empty() {
            // parseLongSafe (:301-309): null/空/解析异常 → defaultVal (§2.15)
            let service_loop_interval_ms =
                interval_val.parse::<i64>().unwrap_or(ENGINE_DEFAULT_REFRESH_MS);
            // PORT: Java (long)(long * double) 经 f64 再截断, 保持同路径
            self.refresh_interval = (service_loop_interval_ms as f64 * ENGINE_REFRESH_MULTIPLIER) as i64;
        }
    }

    pub fn gauges(&self) -> &[EngineGauge] {
        &self.gauges
    }

    pub fn gauge_by_key(&self, key: &str) -> Option<&EngineGauge> {
        self.gauges.iter().find(|g| g.key == key)
    }

    pub fn is_jet(&self) -> bool {
        self.is_jet
    }

    /// updateGaugesPreview (EngineControlOverlay.java:588-606): val=maxValue/2,
    /// COMPRESSOR 显示 1 基档号, 标记示例 ratio 0.5, 全部可见
    pub fn update_preview(&mut self) {
        for g in &mut self.gauges {
            let val = g.max_value / 2;
            let is_compressor = g.gauge_type == GaugeType::Compressor;
            let display_text = (if is_compressor { val + 1 } else { val }).to_string();
            g.gauge.gauge.update(val, &display_text);
            g.visible = true;
            if let Some(mg) = g.marked_gauge.as_mut() {
                mg.update_display(val, &display_text);
                // 预览示例 optimal 标记
                mg.update_marker_ratio("optimal", 0.5);
            }
        }
    }

    /// onFlightData (EngineControlOverlay.java:371-381) 的单事件语义: 节流闩
    /// (间隔 refreshInterval, 配置驱动) → (invokeLater lambda 内) updateResult
    /// (:383-397) = updateStateFromPayload + updateGaugesZeroGC。
    /// compressor_stages = FMManager.current().compressorStages 的档位数快照
    /// (None = 句柄非 READY / 无增压器 → Java null)。
    /// PORT: updateResult 的 legacy Map<String,String> 分支 (:391-395 →
    /// updateGaugeByType/updateGaugesFromData :547-586) 弃译 — 生产不可达
    /// (Service 恒实现 TelemetrySource, telemetrySource != null 恒真)。
    /// PORT: System.currentTimeMillis 由调用方注入 now_ms (field2 先例); 返回
    /// false = 节流跳过 (Java 原方法 void, 宿主可据此省重绘)
    pub fn update(
        &mut self,
        now_ms: i64,
        s: &dyn TelemetrySource,
        payload: &EventPayload,
        compressor_stages: Option<i32>,
    ) -> bool {
        // Throttle updates
        if now_ms - self.last_refresh_time < self.refresh_interval {
            return false;
        }
        self.last_refresh_time = now_ms;
        self.update_state_from_payload(payload, compressor_stages);
        self.update_gauges_zero_gc(s);
        true
    }

    /// updateStateFromPayload (EngineControlOverlay.java:409-439)
    fn update_state_from_payload(
        &mut self,
        payload: &EventPayload,
        compressor_stages: Option<i32>,
    ) {
        // 引擎类型只判一次 (检测完成约 5 秒)
        if !self.jet_label_updated && payload.engine_check_done {
            self.is_jet = payload.is_jet;
            self.jet_label_updated = true;
            // 增压器量程写 FM 档位数 (一次性); Java controller!=null 恒真 (init/initPreview
            // 均传入), POC 无此判
            if !self.compressor_max_value_set {
                if let Some(stages) = compressor_stages {
                    if stages > 1 {
                        for g in &mut self.gauges {
                            if g.gauge_type == GaugeType::Compressor {
                                g.gauge.gauge.max_value = stages - 1;
                                if let Some(mg) = g.marked_gauge.as_mut() {
                                    mg.set_max_value((stages - 1) as f64);
                                }
                                break;
                            }
                        }
                    }
                }
                self.compressor_max_value_set = true;
            }
        }
        // optimal 档标记 (每帧更新)
        self.update_optimal_compressor_marker(payload, compressor_stages);
    }

    /// updateOptimalCompressorMarker (EngineControlOverlay.java:445-468)
    fn update_optimal_compressor_marker(
        &mut self,
        payload: &EventPayload,
        compressor_stages: Option<i32>,
    ) {
        let optimal_stage = payload.optimal_compressor_stage;
        for g in &mut self.gauges {
            if g.gauge_type != GaugeType::Compressor {
                continue;
            }
            // Java 循环条件: markedGauge!=null 才处理并 break; null 则继续扫描后续仪表
            let Some(mg) = g.marked_gauge.as_mut() else { continue };
            match compressor_stages {
                Some(stages) if optimal_stage >= 0 && stages > 1 => {
                    // 档 0 = ratio 0, 档 n-1 = ratio 1
                    let ratio = optimal_stage as f64 / (stages - 1) as f64;
                    mg.update_marker_ratio("optimal", ratio);
                }
                // 无有效数据时隐藏标记
                _ => mg.update_marker_ratio("optimal", -1.0),
            }
            break;
        }
    }

    /// updateGaugesZeroGC (EngineControlOverlay.java:470-545)
    fn update_gauges_zero_gc(&mut self, s: &dyn TelemetrySource) {
        for g in &mut self.gauges {
            // 隐藏字段短路; COMPRESSOR/MIXTURE/PITCH 持续评估 (数据可能回归)
            if !g.visible
                && g.gauge_type != GaugeType::Compressor
                && g.gauge_type != GaugeType::Mixture
                // PITCH 需持续评估: 无桨距机型(自动桨)与手动桨机型间切换时恢复显示
                && g.gauge_type != GaugeType::Pitch
            {
                continue;
            }
            // 喷气机隐藏仪表跳过
            if self.is_jet && is_jet_hidden_gauge(g.gauge_type) {
                continue;
            }
            let mut val;
            let mut has_val = true;
            match g.gauge_type {
                GaugeType::Throttle => {
                    val = s.get_throttle(); // sState.throttle is 0-110
                }
                GaugeType::Pitch => {
                    // 无桨距数据(自动桨机型, 归一化后为-1) → 整条隐藏, 后续竖条自动补位
                    val = s.get_rpm_throttle();
                    g.visible = val >= 0.0;
                    if !g.visible {
                        has_val = false;
                    }
                }
                GaugeType::Power => {
                    val = s.get_power_percent();
                }
                GaugeType::Mixture => {
                    val = s.get_unknown_mixture(); // sState.mixture
                    g.visible = val >= 0.0;
                    if !g.visible {
                        has_val = false;
                    }
                }
                GaugeType::Radiator => {
                    val = s.get_radiator();
                }
                GaugeType::Compressor => {
                    val = s.get_compressor_stage();
                    let stage = val as i32; // Java (int) 截断
                    g.visible = stage > 0;
                    if stage > 0 {
                        // 显示 1 基档号, 条用 0 基值
                        val = (stage - 1) as f64;
                    } else {
                        has_val = false;
                    }
                }
                GaugeType::Fuel => {
                    val = s.get_fuel_percent();
                }
            }
            if has_val {
                // PORT: Java (int) val 截断向零; 值域 0..120, as i32 语义一致
                let int_val = val as i32;
                let text = if g.gauge_type == GaugeType::Compressor {
                    format::format((int_val + 1) as f64, 0)
                } else {
                    format::format(val, 0)
                };
                g.gauge.gauge.update(int_val, &text);
                if let Some(mg) = g.marked_gauge.as_mut() {
                    mg.update_buffer(int_val, &text);
                }
            }
        }
    }

    /// paintComponent → drawGauges (EngineControlOverlay.java:138-143/313-354):
    /// 起点 x=fontsize>>1, y=(fs*4)+((fs*6)>>1); 竖条画在 y-(4*fs), 横条画在 y+dy
    pub fn draw(&mut self, cv: &mut PixCanvas, font_label: &LoadedFont, aa: bool) {
        let fs = self.font_size;
        // paintComponent (EngineControlOverlay.java:143)
        let x = fs >> 1;
        let y = (fs * 4) + ((fs * 6) >> 1);
        let is_jet = self.is_jet;
        let mut dx = 0;
        let mut dy = fs >> 1;
        for g in &mut self.gauges {
            // 喷气机隐藏仪表跳过
            if is_jet && is_jet_hidden_gauge(g.gauge_type) {
                continue;
            }
            if !g.visible {
                continue;
            }
            // MarkedGauge 优先 (COMPRESSOR), 其余 LinearGauge
            if let Some(mg) = g.marked_gauge.as_mut() {
                if g.is_horizontal {
                    mg.draw(cv, x, y + dy, 4 * fs, fs >> 1, font_label, aa);
                    dy += fs + (fs >> 2);
                } else {
                    mg.draw(cv, x + dx, y - 4 * fs, 4 * fs, fs >> 1, font_label, aa);
                    dx += (5 * fs) >> 1;
                }
            } else {
                // Java 每帧原地赋 gauge.vertical = isHorizontal ? false : true
                g.gauge.gauge.vertical = !g.is_horizontal;
                if g.is_horizontal {
                    g.gauge.draw(cv, x, y + dy, 4 * fs, fs >> 1, font_label, aa);
                    dy += fs + (fs >> 2);
                } else {
                    // LinearGauge 逻辑自底向上改为自顶向下后, Y 需上移 (4*fontsize) 保持视觉位置
                    g.gauge.draw(cv, x + dx, y - 4 * fs, 4 * fs, fs >> 1, font_label, aa);
                    dx += (5 * fs) >> 1;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// GearFlapsOverlay (ui/overlay/GearFlapsOverlay.java) — 起落架/襟翼状态条
// ---------------------------------------------------------------------------

/// UIBaseElements.drawVBar (UIBaseElements.java:112-130): 竖条 (底对齐, shade 环 +
/// c 内芯); val_height<0 分支为条自 y 向下生长 (GearFlaps 值域 0..100 不可达, 保真保留)
#[allow(clippy::too_many_arguments)] // 对齐 Java drawVBar(g2d,x,y,width,height,val_height,borderwidth,c)
fn draw_v_bar(
    cv: &mut PixCanvas,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    val_h: i32,
    bw: i32,
    c: [u8; 4],
) {
    if val_h >= 0 {
        ring1px(cv, x, y - h, w - 1, h - 1, colors().shade_shape);
        cv.fill_rect(x + bw, y + bw - val_h, w - 2 * bw, val_h - 2 * bw, c);
    } else {
        ring1px(cv, x, y, w - 1, -h - 1, colors().shade_shape); // 负高 → 不绘制
        cv.fill_rect(x + bw, y + bw, w - 2 * bw, -val_h - 2 * bw, c);
    }
}

/// UIBaseElements.drawHRect (UIBaseElements.java:96-111): 横向条 (shade 环 + c 内芯)
#[allow(clippy::too_many_arguments)] // 对齐 Java drawHRect(g2d,x,y,width,height,borderwidth,c)
fn draw_h_rect(cv: &mut PixCanvas, x: i32, y: i32, w: i32, h: i32, bw: i32, c: [u8; 4]) {
    if w >= 0 {
        ring1px(cv, x, y, w - 1, h - 1, colors().shade_shape);
        cv.fill_rect(x + bw, y + bw, w - 2 * bw, h - 2 * bw, c);
    } else {
        ring1px(cv, x + w, y, -w - 1, h - 1, colors().shade_shape);
        cv.fill_rect(x + bw + w, y + bw, -w - 2 * bw, h - 2 * bw, c);
    }
}

/// UIBaseElements.drawVBarTextNum (UIBaseElements.java:144-154): 竖条 + 随值指针横线 +
/// 数值文本。lbl 形参在 Java 中传入后未绘制 (drawVBarText 的标签绘制已注释), 保真保留
#[allow(clippy::too_many_arguments)] // 对齐 Java drawVBarTextNum(g2d,x,y,width,height,val_height,borderwidth,c,lbl,num,lblFont,numFont)
fn draw_v_bar_text_num(
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
    draw_h_rect(cv, x, y - val_h - 1, w + 3 * num_font.size, 3, 1, colors().label);
    // 数值文本: shade (+1,+1) + 本色 colorLabel (基线 y-val_height-2)
    text_shaded(cv, num_font, x + w, y - val_h - 2, num, colors().label, aa);
}

/// Throttling to prevent EDT task accumulation (gear/flaps are low-frequency data)
/// (GearFlapsOverlay.java:28-29 REFRESH_INTERVAL_MS)
pub const GEAR_FLAPS_REFRESH_INTERVAL_MS: i64 = 100;

/// 起落襟翼面板状态: 几何 (reinitConfig) + 动态数据 (drawTick) + 绘制 (paintComponent)
pub struct GearFlapsState {
    /// 节流基准 (GearFlapsOverlay.java:30 lastRefreshTime, System.currentTimeMillis 毫秒)
    pub last_refresh_time: i64,
    pub font_size: i32,
    pub bar_width: i32,
    pub bar_height: i32,
    /// 内容区宽 (2*fontSize)
    pub width: i32,
    /// 内容区高 (5*fontSize)
    pub height: i32,
    /// 窗口总宽 (width + 4*fontSize + sw*2)
    pub total_width: i32,
    /// 窗口总高 (height + sw*2)
    pub total_height: i32,
    /// 襟翼填充像素高
    pub flap_pix: i32,
    /// 襟翼百分比文本 (Java "%3d")
    pub flap_text: String,
    /// 状态告警文本 (起落架/减速板)
    pub warn_text: String,
    pub warn_color: [u8; 4],
}

impl GearFlapsState {
    /// reinitConfig 几何段 (GearFlapsOverlay.java:95-142)。
    /// show_edge = enablegearAndFlapsEdge 开关 (sw=10)
    pub fn new(font_add: i32, dpi_scale: f64, show_edge: bool) -> Self {
        // fontSize = round((24 + fontadd) * dpiScale)
        let font_size = java_round_f64((24.0 + font_add as f64) * dpi_scale);
        let bar_width = font_size >> 1;
        let bar_height = 4 * font_size;
        let width = 2 * font_size;
        let height = 5 * font_size;
        // 初始 (预览) 襟翼 50%
        let flap_pix = bar_height * 50 / 100;
        let flap_text = format!("{:>3}", 50); // Java String.format("%3d", 50)
        let sw = if show_edge { 10 } else { 0 };
        GearFlapsState {
            last_refresh_time: 0, // Java :30 隐式 0 初始化 (§2.10)
            font_size,
            bar_width,
            bar_height,
            width,
            height,
            total_width: width + 4 * font_size + sw * 2,
            total_height: height + sw * 2,
            flap_pix,
            flap_text,
            warn_text: String::new(),
            warn_color: colors().num,
        }
    }

    /// onFlightData → drawTick (GearFlapsOverlay.java:199-256) 的单事件语义:
    /// 100ms 节流闩 → (invokeLater lambda 内) drawTick (:220-256): 起落架/减速板
    /// 状态文本 + 襟翼像素/文本。
    /// PORT: System.currentTimeMillis 由调用方注入 now_ms (field2 先例); 返回
    /// false = 节流跳过 (Java 原方法 void, 宿主可据此省重绘)
    pub fn update_tick(&mut self, now_ms: i64, lang: &Lang, s: &dyn TelemetrySource) -> bool {
        // Throttling prevents EDT task accumulation
        if now_ms - self.last_refresh_time < GEAR_FLAPS_REFRESH_INTERVAL_MS {
            return false; // Skip this update, too soon
        }
        self.last_refresh_time = now_ms;
        // Java (int) 强转截断; 值域 0..100
        let gear = s.get_gear() as i32;
        let mut flaps = s.get_flaps() as i32;
        let airbrake = s.get_airbrake() as i32;

        if gear >= 0 {
            if gear == 0 {
                self.warn_text.clear();
                self.warn_color = colors().num;
            } else if gear == 100 {
                self.warn_text = lang.g_gear.to_string();
                self.warn_color = colors().num;
            } else {
                self.warn_text = lang.g_gear_down.to_string();
                self.warn_color = colors().warning;
            }
            if airbrake > 0 {
                self.warn_text.push(' ');
                self.warn_text.push_str(lang.g_brake);
                self.warn_color = colors().warning;
            }
        }
        // gear < 0 (无数据): 保留上次告警状态 (Java 同)

        if flaps >= 0 {
            self.flap_pix = flaps * self.bar_height / 100;
        } else {
            self.flap_pix = 0;
            flaps = 0;
        }
        self.flap_text = format!("{:>3}", flaps); // Java String.format("%3d", flaps)
        true
    }

    /// paintComponent (GearFlapsOverlay.java:158-187)
    pub fn draw(
        &self,
        cv: &mut PixCanvas,
        font_num: &LoadedFont,
        font_label: &LoadedFont,
        aa: bool,
    ) {
        let fs = self.font_size;
        let mut dy = fs >> 1;
        // 已经有指示条, 不需要文字了. 暂时注释掉, 不删除.
        // (Java 注释掉的 drawLabelBOSType 调用原位保留于此)
        dy += self.bar_height;
        // 条画在 (0, dy), 数值 "F"+flapText
        let num = format!("F{}", self.flap_text);
        draw_v_bar_text_num(
            cv, 0, dy, self.bar_width, self.bar_height, self.flap_pix, 1, colors().num,
            "", &num, font_num, font_label, aa,
        );
        // 告警文本: (width, baseline=fontSize), fontLabel, 无阴影
        // PORT: Java 判 warnText != null (恒真, 空串绘制无输出), 空串等价无绘制
        cv.draw_text(font_label, self.width, fs, &self.warn_text, self.warn_color, aa);
    }
}

// ---------------------------------------------------------------------------
// OverlayHost 渲染闭包挂接工厂 (预览模式; Java initPreview = 静态数据 + 不订阅事件)
// ---------------------------------------------------------------------------
// PORT: 预览工厂把 state move 进 render 闭包 (此后外部无更新句柄), 仅覆盖 preview
// 语义。live 模式 (Java FlightDataBus → onFlightData → invokeLater → EDT) 的接线
// 选型留待 Controller/事件层批次: Rc<RefCell<State>> 由闭包与更新方共享, 或宿主侧
// 增设数据 tick 钩子 — 两者均需扩展 host.rs OverlaySpec (现仅 render 通道), 本批次
// 不动架构; 节流闩已内置于 update*/update_tick (now_ms 入参), 接线方按事件流注入即可。

/// 动力信息预览 OverlaySpec (Java initPreview: previewValue 静态显示, 恒可见)。
/// 字体目录内需有 sarasa-mono-sc-{bold,regular}.ttf (render.rs 同源)
pub fn power_info_preview_spec(
    fonts_dir: &std::path::Path,
    font_add: i32,
    column_num: i32,
) -> Result<OverlaySpec, String> {
    let ctx = RenderContext::load(fonts_dir, font_add, column_num)?;
    let state = PowerInfoState::new();
    let (w, h) = state.preferred_size(&ctx);
    let mut renderer = BosStyleRenderer::default();
    Ok(OverlaySpec {
        // Java Controller 注册键: config("engineInfoSwitch")
        id: "engineInfoSwitch".to_string(),
        config_key: "engineInfoSwitch".to_string(),
        width: w,
        height: h,
        render: Box::new(move |cv: &mut PixCanvas| state.draw(cv, &ctx, &mut renderer)),
        reinit: None, // 预览专径 (POC 冒烟); WYSIWYG reinit 面在 live 工厂
    })
}

/// 引擎控制预览 OverlaySpec (updateGaugesPreview 的半量程静态值)
pub fn engine_control_preview_spec(
    fonts_dir: &std::path::Path,
    lang: &Lang,
    font_add: i32,
    dpi_scale: f64,
) -> Result<OverlaySpec, String> {
    let mut state = EngineControlState::new(lang, font_add, dpi_scale, &|_| false, &|_| String::new());
    // Java initPreview (:171-172): init 链已含 updateGaugesPreview, 此处原样二次调用
    state.update_preview();
    // fontLabel = BOLD(round(fontSize/2.0f)) (loadFontConfig)
    let half = java_round_f32(state.font_size as f32 / 2.0);
    let font_label = Rc::new(LoadedFont::new(
        &fonts_dir.join("sarasa-mono-sc-bold.ttf"),
        half,
    )?);
    Ok(OverlaySpec {
        id: "enableEngineControl".to_string(),
        config_key: "enableEngineControl".to_string(),
        width: state.width,
        height: state.height,
        render: Box::new(move |cv: &mut PixCanvas| {
            // 生产 AA 恒开 (Application.java:102 graphAASetting 默认 ON);
            // 接配置层后随 graphAASetting (host GLOBAL_CONFIG_KEYS 的 AA 开关键族) 回收
            state.draw(cv, &font_label, true);
        }),
        reinit: None,
    })
}

/// 起落襟翼预览 OverlaySpec (初始襟翼 50%, 无告警)
pub fn gear_flaps_preview_spec(
    fonts_dir: &std::path::Path,
    font_add: i32,
    dpi_scale: f64,
    show_edge: bool,
) -> Result<OverlaySpec, String> {
    let state = GearFlapsState::new(font_add, dpi_scale, show_edge);
    let bold = fonts_dir.join("sarasa-mono-sc-bold.ttf");
    // fontNum = BOLD(fontSize); fontLabel = BOLD(round(fontSize/2.0f)) (reinitConfig)
    let font_num = Rc::new(LoadedFont::new(&bold, state.font_size)?);
    let font_label = Rc::new(LoadedFont::new(
        &bold,
        java_round_f32(state.font_size as f32 / 2.0),
    )?);
    Ok(OverlaySpec {
        id: "enablegearAndFlaps".to_string(),
        config_key: "enablegearAndFlaps".to_string(),
        width: state.total_width,
        height: state.total_height,
        render: Box::new(move |cv: &mut PixCanvas| {
            // 生产 AA 恒开 (Application.java:102 graphAASetting 默认 ON);
            // 接配置层后随 graphAASetting (host GLOBAL_CONFIG_KEYS 的 AA 开关键族) 回收
            state.draw(cv, &font_num, &font_label, true);
        }),
        reinit: None,
    })
}

// ---------------------------------------------------------------------------
// live 喂数形态工厂 (minihud_overlay_spec 先例: render 闭包与喂入方共享句柄)
// ---------------------------------------------------------------------------
// Java 各 overlay init(S) 时自订 FlightDataBus (LIFETIMES §2.1), preview 实例
// (initPreview) 不订阅保持 previewValue 静态。Rust host 单条目跨 open/refresh_preview
// 存活 (D8), 两形态共用一份 state — live 喂入由 win32 线程持句柄执行, preview 期
// 喂入门控见 app_shell 的 feed_overlays_live (overlay_ctx_preview 标志)。

/// 动力信息共享句柄 (render 闭包 + 喂入方各持克隆)
pub type PowerInfoHandle = Rc<RefCell<PowerInfoState>>;

/// 动力信息 OverlaySpec + live 句柄 (Java Controller.java:662 注册键 engineInfoSwitch)。
/// 初始态 = previewValue (PowerInfoState::new), 游戏模式由喂入方 update 推进。
/// PORT(WYSIWYG): 字号/列数随 [`ReinitParams`] 仓 — render 闭包经共享 ctx 单元
/// 读取, reinit 闭包重建 RenderContext (Java reinitConfig 的 super 段: 字体 +
/// 列布局重载) 并返回新 preferred_size (setBounds 副作用)
pub fn power_info_overlay_spec(
    fonts_dir: &std::path::Path,
    params: &Rc<RefCell<ReinitParams>>,
) -> Result<(PowerInfoHandle, OverlaySpec), String> {
    let (font_add, column_num) = {
        let p = params.borrow();
        (p.font_add_power, p.power_columns)
    };
    let ctx = Rc::new(RefCell::new(RenderContext::load(fonts_dir, font_add, column_num)?));
    let state = PowerInfoState::new();
    let (w, h) = state.preferred_size(&ctx.borrow());
    let handle: PowerInfoHandle = Rc::new(RefCell::new(state));
    let render_handle = Rc::clone(&handle);
    let mut renderer = BosStyleRenderer::default();
    // reinit 闭包: 重建 ctx (字体/列度量) → 新 preferred_size (Java setBounds)
    let reinit_handle = Rc::clone(&handle);
    let reinit_ctx = Rc::clone(&ctx);
    let reinit_fonts = fonts_dir.to_path_buf();
    let reinit_params = Rc::clone(params);
    let reinit: ReinitFn = Box::new(move || {
        let (fa, col) = {
            let p = reinit_params.borrow();
            (p.font_add_power, p.power_columns)
        };
        let new_ctx = match RenderContext::load(&reinit_fonts, fa, col) {
            Ok(c) => c,
            Err(e) => {
                // 字体重载失败: 保持旧 ctx (Java 字体族随包分发, 此路径不可达;
                // 显式留痕不静默)
                vm_core::logger::error("PowerInfo", &format!("reinit 字体重载失败: {}", e));
                return None;
            }
        };
        *reinit_ctx.borrow_mut() = new_ctx;
        Some(reinit_handle.borrow().preferred_size(&reinit_ctx.borrow()))
    });
    Ok((
        handle,
        OverlaySpec {
            id: "engineInfoSwitch".to_string(),
            config_key: "engineInfoSwitch".to_string(),
            width: w,
            height: h,
            render: Box::new(move |cv: &mut PixCanvas| {
                render_handle.borrow().draw(cv, &ctx.borrow(), &mut renderer);
            }),
            reinit: Some(reinit),
        },
    ))
}

/// 引擎控制共享句柄
pub type EngineControlHandle = Rc<RefCell<EngineControlState>>;

/// 引擎控制 OverlaySpec + live 句柄 (Java Controller.java:654 注册键 enableEngineControl)。
/// `lang` 以 Rc 共享 (reinit 闭包重建 state 需要标签源; Lang !Clone)。
/// PORT(WYSIWYG): 字号/7 仪表 disable/轮询间隔随 [`ReinitParams`] 仓 — reinit
/// 闭包整体重建 EngineControlState + fontLabel (Java reinitConfig: loadFontConfig +
/// loadRefreshInterval + initGaugeFields + calculateLayout + updateGaugesPreview),
/// 返回新 (width, height) (Java setLocation 尺寸面)
pub fn engine_control_overlay_spec(
    fonts_dir: &std::path::Path,
    lang: Rc<Lang>,
    params: &Rc<RefCell<ReinitParams>>,
) -> Result<(EngineControlHandle, OverlaySpec), String> {
    let (font_add, dpi_scale, interval_ms, disables) = {
        let p = params.borrow();
        (p.font_add_engine, p.dpi_scale, p.service_loop_interval_ms, p.engine_disables)
    };
    let interval_str = interval_ms.to_string();
    // init 链 (game 实例): initGaugeFields + calculateLayout + updateGaugesPreview
    // (半量程初值, 首个有效事件前的显示态; initPreview 的二次调用是 preview 专属)
    // cfg_true 按键名查 disables 表 (Java "true".equals(getConfigSafe(key));
    // 曾恒 false — 7 个 disable 开关从未生效, 启动首帧即与 Java 不一致)
    let state = build_engine_state(&lang, font_add, dpi_scale, &interval_str, &disables);
    // fontLabel = BOLD(round(fontSize/2.0f)) (loadFontConfig)
    let half = java_round_f32(state.font_size as f32 / 2.0);
    let bold_path = fonts_dir.join("sarasa-mono-sc-bold.ttf");
    let font_label = Rc::new(RefCell::new(Rc::new(LoadedFont::new(&bold_path, half)?)));
    let (w, h) = (state.width, state.height);
    let handle: EngineControlHandle = Rc::new(RefCell::new(state));
    let render_handle = Rc::clone(&handle);
    let render_font = Rc::clone(&font_label);
    // reinit 闭包: 状态整体重建 (Java initGaugeFields 全量重排) + fontLabel 重载
    let reinit_handle = Rc::clone(&handle);
    let reinit_font = Rc::clone(&font_label);
    let reinit_lang = Rc::clone(&lang);
    let reinit_params = Rc::clone(params);
    let reinit_bold = bold_path;
    let reinit: ReinitFn = Box::new(move || {
        let (fa, dpi, iv, dis) = {
            let p = reinit_params.borrow();
            (p.font_add_engine, p.dpi_scale, p.service_loop_interval_ms, p.engine_disables)
        };
        let new_state = build_engine_state(&reinit_lang, fa, dpi, &iv.to_string(), &dis);
        let half = java_round_f32(new_state.font_size as f32 / 2.0);
        let new_font = match LoadedFont::new(&reinit_bold, half) {
            Ok(f) => Rc::new(f),
            Err(e) => {
                vm_core::logger::error("EngineControl", &format!("reinit 字体重载失败: {}", e));
                return None;
            }
        };
        let (w, h) = (new_state.width, new_state.height);
        *reinit_handle.borrow_mut() = new_state;
        *reinit_font.borrow_mut() = new_font;
        Some((w, h))
    });
    Ok((
        handle,
        OverlaySpec {
            id: "enableEngineControl".to_string(),
            config_key: "enableEngineControl".to_string(),
            width: w,
            height: h,
            render: Box::new(move |cv: &mut PixCanvas| {
                // aa = 运行时仓 (cfg AAEnable 可关 — 审查轮 1-A 第 7 处钉死点)
                render_handle.borrow_mut().draw(cv, &render_font.borrow(), aa());
            }),
            reinit: Some(reinit),
        },
    ))
}

/// EngineControlState::new 的 interval/disables 参数打包 (工厂初建与 reinit 共用)
fn build_engine_state(
    lang: &Lang,
    font_add: i32,
    dpi_scale: f64,
    interval_str: &str,
    disables: &[bool; 7],
) -> EngineControlState {
    EngineControlState::new(
        lang,
        font_add,
        dpi_scale,
        &|key: &str| ENGINE_DISABLE_KEYS
            .iter()
            .position(|k| *k == key)
            .map(|i| disables[i])
            .unwrap_or(false),
        &|_| interval_str.to_string(),
    )
}

/// 起落襟翼共享句柄
pub type GearFlapsHandle = Rc<RefCell<GearFlapsState>>;

/// 起落襟翼 OverlaySpec + live 句柄 (Java Controller.java:709 注册键 enablegearAndFlaps)。
/// 初始态 = 襟翼 50% 无告警 (new 的预览初值), 游戏模式由喂入方 update_tick 推进。
/// PORT(WYSIWYG): 字号/边缘开关随 [`ReinitParams`] 仓 — reinit 闭包重建几何 +
/// 双字体 (Java reinitConfig :95-142), 返回新 (total_width, total_height)
pub fn gear_flaps_overlay_spec(
    fonts_dir: &std::path::Path,
    params: &Rc<RefCell<ReinitParams>>,
) -> Result<(GearFlapsHandle, OverlaySpec), String> {
    let (font_add, dpi_scale, show_edge) = {
        let p = params.borrow();
        (p.font_add_gear, p.dpi_scale, p.gear_show_edge)
    };
    let state = GearFlapsState::new(font_add, dpi_scale, show_edge);
    let bold = fonts_dir.join("sarasa-mono-sc-bold.ttf");
    // fontNum = BOLD(fontSize); fontLabel = BOLD(round(fontSize/2.0f)) (reinitConfig)
    let font_num = Rc::new(RefCell::new(Rc::new(LoadedFont::new(
        &bold,
        state.font_size,
    )?)));
    let font_label = Rc::new(RefCell::new(Rc::new(LoadedFont::new(
        &bold,
        java_round_f32(state.font_size as f32 / 2.0),
    )?)));
    let (w, h) = (state.total_width, state.total_height);
    let handle: GearFlapsHandle = Rc::new(RefCell::new(state));
    let render_handle = Rc::clone(&handle);
    // reinit 闭包: 几何 + 双字体重建 (Java reinitConfig 同段; flap 50%/warn 清空
    // 的预览复位语义原样保留)
    let reinit_handle = Rc::clone(&handle);
    let (reinit_num, reinit_label) = (Rc::clone(&font_num), Rc::clone(&font_label));
    let reinit_params = Rc::clone(params);
    let reinit_bold = bold;
    let reinit: ReinitFn = Box::new(move || {
        let (fa, dpi, edge) = {
            let p = reinit_params.borrow();
            (p.font_add_gear, p.dpi_scale, p.gear_show_edge)
        };
        let new_state = GearFlapsState::new(fa, dpi, edge);
        let (num, label) = match (
            LoadedFont::new(&reinit_bold, new_state.font_size),
            LoadedFont::new(&reinit_bold, java_round_f32(new_state.font_size as f32 / 2.0)),
        ) {
            (Ok(n), Ok(l)) => (Rc::new(n), Rc::new(l)),
            (r, _) => {
                if let Err(e) = r {
                    vm_core::logger::error("GearFlaps", &format!("reinit 字体重载失败: {}", e));
                }
                return None;
            }
        };
        let (w, h) = (new_state.total_width, new_state.total_height);
        *reinit_handle.borrow_mut() = new_state;
        *reinit_num.borrow_mut() = num;
        *reinit_label.borrow_mut() = label;
        Some((w, h))
    });
    Ok((
        handle,
        OverlaySpec {
            id: "enablegearAndFlaps".to_string(),
            config_key: "enablegearAndFlaps".to_string(),
            width: w,
            height: h,
            render: Box::new(move |cv: &mut PixCanvas| {
                // 生产 AA 恒开 (Application.java:102 graphAASetting 默认 ON)
                let (num, label) = (font_num.borrow(), font_label.borrow());
                render_handle.borrow().draw(cv, &num, &label, true);
            }),
            reinit: Some(reinit),
        },
    ))
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
