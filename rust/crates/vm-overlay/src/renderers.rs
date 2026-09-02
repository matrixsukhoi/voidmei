//! renderers: ui/renderer 包的 C 类语义复刻 (基于 render2d::PixCanvas)
//!
//! | Java (src/ui/renderer/) | 本文件 | 语义要点 |
//! |---|---|---|
//! | OverlayRenderer.java:23/28 | [`OverlayRenderer`] trait | 多实现接口 → trait dyn (§1) |
//! | RenderContext.java | [`RenderContext`] | 几何公式复用 vm-core `RenderCtx`, 本层挂字体/调色板/AA |
//! | BOSStyleRenderer.java | [`BosStyleRenderer`] | 多列网格 + TextGauge 按 label 缓存 |
//!
//! 绘制委托目标分工: 条形 gauge 族 = `crate::gauges_bars` (W 批已落地,
//! LabeledLinearGauge 即 GaugeField.java:28 的唯一实例化类型);
//! TextGauge (ui/component/TextGauge.java) 是 BOS 专属组件, 随本文件落地。
//!
//! 保真对象 = 像素级视觉输出与状态行为 (非代码结构):
//! - LinearGaugeRenderer 的 gauge 组件在 Java 存活于 GaugeField 字段,
//!   Rust 侧由渲染器按 field key 缓存 + 每帧 update 同步 (脏检查在组件内)。
//! - BOS 的 TextGauge 按 label 缓存 (Java gaugeCache, BOSStyleRenderer.java:18)。

use vm_core::configuration_service::GlobalColors;
use std::collections::HashMap;
use std::rc::Rc;

use crate::font::LoadedFont;
use crate::global_colors::colors;
use crate::render2d::PixCanvas;
use crate::layout::RenderCtx;
use crate::ui_model::{DataField, GaugeField};

// ---------------------------------------------------------------------------
// 调色板 (Application.java:108-111 静态默认色, TextGauge 直接引用)
// ---------------------------------------------------------------------------

/// Application.colorNum / colorLabel / colorUnit / colorShadeShape 的运行时快照
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderPalette {
    pub num: [u8; 4],
    pub label: [u8; 4],
    pub unit: [u8; 4],
    pub shade: [u8; 4],
}

/// Application.java:108-111 默认值 (Color(r,g,b,a) 直读 RGBA)。
/// PORT: num/label/shade 与 gauges_bars 的同源色收敛单源 (colors().num/colors().label/
/// colors().shade_shape), 消除双份维护漂移; colorUnit 仅本文件消费, 保留唯一一份。
pub const APPLICATION_COLORS: RenderPalette = RenderPalette {
    num: GlobalColors::JAVA_DEFAULT.num,           // colorNum
    label: GlobalColors::JAVA_DEFAULT.label,       // colorLabel
    unit: GlobalColors::JAVA_DEFAULT.unit,         // colorUnit
    shade: GlobalColors::JAVA_DEFAULT.shade_shape, // colorShadeShape
};

/// java.awt.Color.WHITE (TextOnlyRenderer.java:27 默认字色)
pub const WHITE: [u8; 4] = [255, 255, 255, 255];

// ---------------------------------------------------------------------------
// RenderContext (RenderContext.java)
// ---------------------------------------------------------------------------

/// 渲染配置上下文: 字体三元组 + 几何 (vm-core RenderCtx) + 调色板 + AA 开关。
/// Java 端字体/AA 是 Application 全局态, 此处收敛为显式字段。
///
/// PORT AA 不变式: graph_aa 与 text_aa 生产上恒相等 — Java 单配置 AAEnable 同开同关
/// 两个 hint (ConfigurationService.java:159-164), LinearGaugeRenderer.java:21-22 也是
/// 成对设置。LinearGaugeRenderer 渲染路径只把 graph_aa 传入 gauge.draw (gauge 内的
/// label/value 文本在 Java 受 KEY_TEXT_ANTIALIASING 管), 依赖该不变式无视觉差;
/// 若未来两 AA 拆成独立配置, 必须给 gauge.draw 增设 text_aa 参数, 否则 gauge 内文本失真。
pub struct RenderContext {
    /// num = BOLD(fontSize) (Java RenderContext.java:72)
    pub num_font: Rc<LoadedFont>,
    /// label = BOLD(round(fontSize/2)) (Java :73)
    pub label_font: Rc<LoadedFont>,
    /// unit = PLAIN(round(fontSize/2)) (Java :74)
    pub unit_font: Rc<LoadedFont>,
    /// 几何公式 (fontSize/columnNum/numHeight 及派生), RenderContext.java:104-121
    pub geom: RenderCtx,
    /// Application.graphAASetting (Application.java:102 默认 ANTIALIAS_ON)
    pub graph_aa: bool,
    /// Application.textAASetting (Application.java:101 默认 GASP ≈ 开)
    pub text_aa: bool,
}

/// Java Math.round(float) = floor(x + 0.5) (§2.3)
fn java_round_f(x: f32) -> i32 {
    (x + 0.5).floor() as i32
}

impl RenderContext {
    /// 对应 RenderContext.create (RenderContext.java:68-81)。
    /// 契约: num_font.size 即 fontSize (Java fontSize == numFont.getSize());
    /// numHeight 取 Toolkit.getFontMetrics(numFont).getHeight() 的 Rust 同源
    /// 度量 (font.rs: Java FontMetrics 语义 ascent+descent+leading)。
    /// 传入字体必须已通过 load() 校验 (或等价校验) — metrics() 对无法解析的
    /// 字体内部 expect 会 panic (font.rs 缺陷, 本层 load() 补齐防护)。
    pub fn new(
        num_font: Rc<LoadedFont>,
        label_font: Rc<LoadedFont>,
        unit_font: Rc<LoadedFont>,
        column_num: i32,
    ) -> Self {
        let num_height = num_font.metrics().height;
        let font_add = num_font.size - 24;
        RenderContext {
            num_font,
            label_font,
            unit_font,
            geom: RenderCtx::new(font_add, column_num, num_height),
            graph_aa: true,
            text_aa: true,
        }
    }

    /// Java 静态配色 (colorNum 族) — 每帧动态读全局仓: Java TextGauge.
    /// drawTextShaded 直读 Application 静态, cfg 五键 (fontNum 等) 运行时可变
    /// (WYSIWYG); 构造期字段快照会冻结启动值 (人工验收: 动力信息文本曾冻结
    /// Java 静态初始值荧光绿, 即此因), 故为方法非字段。
    /// 对拍工具路径 (需恒定基线) 用 [`APPLICATION_COLORS`] 常量
    pub fn palette(&self) -> RenderPalette {
        RenderPalette {
            num: colors().num,
            label: colors().label,
            unit: colors().unit,
            shade: colors().shade_shape,
        }
    }

    /// fromConfig/create 的字体装载通道 (POC 先例: render.rs FontTriple::load —
    /// Java 按族名创建 Font, Rust 按 sarasa 文件装载, num/label 用 bold, unit 用 regular)。
    /// fontSize = 24 + font_add; label/unit = Math.round(fontSize / 2.0f) (Java :71-74)。
    ///
    /// 字体校验: LoadedFont::new 只做 fs::read 不校验 Face (font.rs 缺陷) — 此处
    /// 用 glyph 探针补齐本构造路径的防护: 损坏/无法解析的文件返回 Err, 而非在
    /// metrics() 处 panic。Java new Font 对坏字体静默回退 dialog 字体; Rust 无
    /// 内置回退族, 降级为显式 Err (POC 裁决: 字体随应用分发, 损坏属安装事故)。
    pub fn load(fonts_dir: &std::path::Path, font_add: i32, column_num: i32) -> Result<Self, String> {
        let font_size = 24 + font_add;
        let half = java_round_f(font_size as f32 / 2.0);
        let bold = fonts_dir.join("sarasa-mono-sc-bold.ttf");
        let regular = fonts_dir.join("sarasa-mono-sc-regular.ttf");
        // glyph 是 font.rs 唯一非 panic 的解析通道 (Face 解析失败 → None),
        // 装载即逐个校验, 损坏文件直接报该文件而非下游 panic/误报
        let load_font = |path: &std::path::Path, size: i32| -> Result<Rc<LoadedFont>, String> {
            let f = Rc::new(LoadedFont::new(path, size)?);
            if f.glyph('A', false).is_none() {
                return Err(format!("字体文件损坏或无法解析: {}", path.display()));
            }
            Ok(f)
        };
        let num_font = load_font(&bold, font_size)?;
        let label_font = load_font(&bold, half)?;
        let unit_font = load_font(&regular, half)?;
        Ok(RenderContext::new(num_font, label_font, unit_font, column_num))
    }

    pub fn font_size(&self) -> i32 {
        self.geom.font_size
    }

    pub fn column_num(&self) -> i32 {
        self.geom.column_num
    }

    pub fn num_height(&self) -> i32 {
        self.geom.num_height
    }

    /// getFieldWidth (RenderContext.java:104-106)
    pub fn field_width(&self) -> i32 {
        3 * self.geom.font_size
    }
}

// ---------------------------------------------------------------------------
// Field 判别通道 (Java List<DataField> + instanceof GaugeField)
// ---------------------------------------------------------------------------

/// gauge_field.rs 预告的 enum 判别方案: 替代 Java `field instanceof GaugeField`
/// (LinearGaugeRenderer.java:31/60), 继承字段经 `base` 通道访问。
pub enum Field<'a> {
    Data(&'a DataField),
    Gauge(&'a GaugeField),
}

impl<'a> Field<'a> {
    /// Java 多态读取公共字段 (GaugeField extends DataField)
    pub fn base(&self) -> &'a DataField {
        match self {
            Field::Data(d) => d,
            Field::Gauge(g) => &g.base,
        }
    }
}

// ---------------------------------------------------------------------------
// OverlayRenderer (OverlayRenderer.java:13)
// ---------------------------------------------------------------------------

pub trait OverlayRenderer {
    /// Java render(g2d, fields, ctx, int[] offset) (OverlayRenderer.java:23)。
    /// offset = [x, y] 双向通道: 进入为初值, 渲染推进后写回 (Java 原地改数组)。
    fn render(
        &mut self,
        canvas: &mut PixCanvas,
        fields: &[Field<'_>],
        ctx: &RenderContext,
        offset: &mut [i32; 2],
    );

    /// Java calculatePreferredSize(fields, ctx) → Dimension (OverlayRenderer.java:28),
    /// 返回 (width, height)。
    fn calculate_preferred_size(&mut self, fields: &[Field<'_>], ctx: &RenderContext) -> (i32, i32);
}

// ---------------------------------------------------------------------------
// TextGauge (ui/component/TextGauge.java)
// ---------------------------------------------------------------------------

/// 数值+标签+单位三元文本 (TextGauge.java:14)。
pub struct TextGauge {
    pub label: String,
    pub unit: String,
    pub value: String,
}

impl TextGauge {
    /// TextGauge.java:24-29 (value 初值 "")
    pub fn new(label: &str, unit: &str) -> Self {
        TextGauge {
            label: label.to_string(),
            unit: unit.to_string(),
            value: String::new(),
        }
    }

    /// TextGauge.java:30-32
    pub fn update(&mut self, value: &str) {
        self.value.clear();
        self.value.push_str(value);
    }

    /// TextGauge.java:34-36
    pub fn set_unit(&mut self, unit: &str) {
        self.unit.clear();
        self.unit.push_str(unit);
    }

    /// TextGauge.java:43-65。val_buffer = Some(缓冲) 对应 Java (valBuffer, valLen)
    /// 双参通道, None 走 value 字符串通道 (valLen==0)。
    pub fn draw(
        &self,
        canvas: &mut PixCanvas,
        ctx: &RenderContext,
        x: i32,
        y: i32,
        shade_width: i32,
        val_buffer: Option<&str>,
    ) {
        // PORT: Java 在此构建 BasicStroke(shadeWidth, ROUND, ROUND) 但从未
        // g2d.setStroke — 仅作 stroke!=null 旗标决定画不画阴影, 首帧后恒非 null
        // → 恒画 (+1,+1) 阴影。shade_width 参数保留接口位, 视觉无作用。
        let _ = shade_width;
        let lwidth = (13 * ctx.num_font.size) >> 2; // TextGauge.java:50
        let center_y = (y + y + ctx.label_font.size + ctx.unit_font.size) >> 1; // :51
        let num_padding = std::cmp::max(4, ctx.num_font.size / 4); // :52

        // 数值右对齐: charsWidth/stringWidth = Σround(advance) = measure (font.rs)
        let value = val_buffer.unwrap_or(self.value.as_str());
        let val_width = ctx.num_font.measure(value);
        draw_text_shaded(
            canvas,
            &ctx.num_font,
            x + lwidth - val_width - num_padding,
            center_y,
            value,
            ctx.palette().num,
            ctx.palette().shade,
            ctx.text_aa,
        );
        // 标签 (基线 y) — TextGauge.java:63
        draw_text_shaded(
            canvas,
            &ctx.label_font,
            x + lwidth,
            y,
            &self.label,
            ctx.palette().label,
            ctx.palette().shade,
            ctx.text_aa,
        );
        // 单位 (基线 y + labelFontSize) — TextGauge.java:64
        draw_text_shaded(
            canvas,
            &ctx.unit_font,
            x + lwidth,
            y + ctx.label_font.size,
            &self.unit,
            ctx.palette().unit,
            ctx.palette().shade,
            ctx.text_aa,
        );
    }
}

/// BOSStyleRenderer 侧的数值文本解析 (TextGauge.java:54 valBuffer!=null && valLen>0)
fn bos_value_text<'a>(base: &'a DataField, gauge_value: &'a str) -> &'a str {
    if base.length > 0 {
        base.buffer.as_str()
    } else {
        gauge_value
    }
}

/// GaugeField 侧数值文本解析: Java gauge.valueLen>0 ? valueBuffer : displayValue
/// (vm-core 数据态映射: base.buffer/length ↔ gauge.valueBuffer/valueLen,
/// base.current_value ↔ gauge.displayValue, 见 gauge_field.rs PORT 注)
///

/// Java TextGauge.drawTextShaded / LinearGauge.drawTextShaded:
/// (+1,+1) 阴影先画, 本色后画 (TextGauge.java:85-93)
#[allow(clippy::too_many_arguments)] // 对齐 Java drawTextShaded(g2d,x,y,s,f,c)+显式 shade/aa
fn draw_text_shaded(
    canvas: &mut PixCanvas,
    font: &LoadedFont,
    x: i32,
    y: i32,
    text: &str,
    color: [u8; 4],
    shade: [u8; 4],
    aa: bool,
) {
    canvas.draw_text(font, x + 1, y + 1, text, shade, aa);
    canvas.draw_text(font, x, y, text, color, aa);
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// BOSStyleRenderer (BOSStyleRenderer.java:15)
// ---------------------------------------------------------------------------

/// BOS 风格: 多列网格 (数值右对齐 + 标签 + 单位), TextGauge 按 label 缓存。
#[derive(Default)]
pub struct BosStyleRenderer {
    /// Java :18 gaugeCache (保持 stroke 缓存 → Rust 即组件复用)
    gauge_cache: HashMap<String, TextGauge>,
}

impl BosStyleRenderer {
    /// updateOffset (BOSStyleRenderer.java:69-76)
    fn update_offset(&self, visible_index: i32, offset: &mut [i32; 2], ctx: &RenderContext) {
        // PORT: columnNum==0 时 Java % 直接抛 ArithmeticException — Rust i32 取余
        // 同样 panic (行为对等), 显式 assert 换取可诊断信息。ui_layout.cfg 列数
        // 滑条 :min 1 使配置路径不可达, 仅程序化误用触达。
        // (calculate_preferred_size 经 vm-core RenderCtx::total_height 的同型除零
        // 属 vm-core 侧, 本文件不越界修 — PORTING.md §6。)
        assert!(
            ctx.column_num() > 0,
            "BOSStyleRenderer: columnNum==0 (Java 同位置 ArithmeticException)"
        );
        if visible_index % ctx.column_num() == 0 {
            // Math.round(1 * numHeight) = numHeight (整数恒等)
            offset[1] += ctx.num_height();
            offset[0] = ctx.font_size() >> 1;
        } else {
            // Math.round(5 * fontSize) = 5*fontSize (整数恒等)
            offset[0] += 5 * ctx.font_size();
        }
    }

    /// 测试/诊断: 缓存内 TextGauge 数量
    pub fn cached_gauge_count(&self) -> usize {
        self.gauge_cache.len()
    }

    /// 测试/诊断: 缓存内指定 label 的当前 unit
    pub fn cached_unit(&self, label: &str) -> Option<&str> {
        self.gauge_cache.get(label).map(|g| g.unit.as_str())
    }

    /// 缓存释放钩子。注意与 LinearGaugeRenderer::clear 的失效语义不同: Java 的
    /// gaugeCache 本就存活于渲染器实例 (FieldOverlay.java:66 构造时创建,
    /// reinitConfig 不重建), TextGauge 的 value/unit 每帧全量同步 → 跨字段重建
    /// 无陈旧值问题; 本方法仅供调用方销毁/复用渲染器时显式释放。
    pub fn clear(&mut self) {
        self.gauge_cache.clear();
    }
}

impl OverlayRenderer for BosStyleRenderer {
    fn render(
        &mut self,
        canvas: &mut PixCanvas,
        fields: &[Field<'_>],
        ctx: &RenderContext,
        offset: &mut [i32; 2],
    ) {
        // Java :22-28: setPaintMode + AA/速度 hint → 无几何作用, AA 走 ctx
        offset[0] = ctx.font_size() >> 1;
        offset[1] = ctx.font_size() >> 1;
        let mut visible_index = 0;
        for f in fields {
            let base = f.base();
            if !base.visible {
                continue;
            }
            // Java :42-46 gaugeCache 按 label get-or-create (稳态零分配: 先查后插)
            let gauge = match self.gauge_cache.get_mut(base.label.as_str()) {
                Some(g) => g,
                None => {
                    self.gauge_cache
                        .insert(base.label.clone(), TextGauge::new(&base.label, &base.unit));
                    self.gauge_cache.get_mut(base.label.as_str()).expect("刚插入")
                }
            };
            gauge.update(&base.current_value);
            // Java :52: field.unit != null && !equals(gauge.unit) → setUnit
            // PORT: Rust String 恒非 null, 条件退化为不等比较
            if base.unit != gauge.unit {
                gauge.set_unit(&base.unit);
            }
            let value = bos_value_text(base, gauge.value.as_str());
            let val_buffer = if base.length > 0 { Some(value) } else { None };
            gauge.draw(canvas, ctx, offset[0], offset[1], 1, val_buffer);
            visible_index += 1;
            self.update_offset(visible_index, offset, ctx);
        }
    }

    fn calculate_preferred_size(&mut self, fields: &[Field<'_>], ctx: &RenderContext) -> (i32, i32) {
        let visible_count = fields.iter().filter(|f| f.base().visible).count() as i32;
        (ctx.geom.total_width(), ctx.geom.total_height(visible_count))
    }
}
