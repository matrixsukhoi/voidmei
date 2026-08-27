//! renderers: ui/renderer 包的 C 类语义复刻 (基于 render2d::PixCanvas)
//!
//! | Java (src/ui/renderer/) | 本文件 | 语义要点 |
//! |---|---|---|
//! | OverlayRenderer.java:23/28 | [`OverlayRenderer`] trait | 多实现接口 → trait dyn (§1) |
//! | RenderContext.java | [`RenderContext`] | 几何公式复用 vm-core `RenderCtx`, 本层挂字体/调色板/AA |
//! | LinearGaugeRenderer.java | [`LinearGaugeRenderer`] | 竖/横条布局推进 (dx/dy), 组件走 gauges_bars |
//! | BOSStyleRenderer.java | [`BosStyleRenderer`] | 多列网格 + TextGauge 按 label 缓存 |
//! | TextOnlyRenderer.java | [`TextOnlyRenderer`] | 纯数值行, 白色无阴影 |
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
use crate::gauges_bars::LabeledLinearGauge;
use crate::global_colors::colors;
use crate::render2d::PixCanvas;
use vm_core::layout::RenderCtx;
use vm_core::ui_model::{DataField, GaugeField};

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
        // Java: fontSize = 24 + fontAdd → 反解 fontAdd (RenderCtx::new 内部再加回)
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
/// PORT 通道语义差 (当前不可达; C 批 EngineControlOverlay 移植时必须对齐):
/// Java 是"后写者胜" — String 通道 LinearGauge.update(int,String) 置组件
/// valueLen=0 使缓冲失效 (LinearGauge.java:40-44); 本函数是"length>0 则缓冲恒胜",
/// 且 vm-core GaugeField::update_gauge 不清 base.length → 同一字段混用两通道时
/// 陈旧缓冲会盖掉新值。生产路径每渲染器实例互斥 (telemetrySource 定死于 init,
/// 零 GC 与 Map 两套更新走同一通道); 对齐手段 = vm-core update_gauge 清 length
/// 或 overlay 保证单通道写入 (跨文件契约, 本批不越界修 — PORTING.md §6)。
fn gauge_value_text(gf: &GaugeField) -> &str {
    if gf.base.length > 0 {
        gf.base.buffer.as_str()
    } else {
        gf.base.current_value.as_str()
    }
}

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
// LinearGaugeRenderer (LinearGaugeRenderer.java:17)
// ---------------------------------------------------------------------------

/// 竖/横条渲染器。gauge 组件缓存: Java 中组件存活于 GaugeField.gauge 字段
/// (Swing 组件), Rust 侧由渲染器按 field key 持有并每帧 update 同步
/// (脏检查在 LabeledLinearGauge 内部)。
///
/// PORT 缓存失效契约: Java 的 LabeledLinearGauge 组件随 GaugeField 重建而消亡
/// (FieldOverlay.reinitConfig → initFields → fieldManager.clearAll, FieldOverlay
/// .java:121-146; renderer 实例本身存活, LinearGaugeRenderer.java 无缓存);
/// Rust 缓存跨字段重建存活, 首帧固化 (label, max_value) — LabeledLinearGauge::new
/// 的构造参数, update 不改写。调用方重建字段 (量程/标签变化, 如公英制切换) 后
/// **必须调用 [`LinearGaugeRenderer::clear`] 或重建渲染器**, 否则条形量程/标签陈旧错绘。
#[derive(Default)]
pub struct LinearGaugeRenderer {
    gauges: HashMap<String, LabeledLinearGauge>,
}

impl LinearGaugeRenderer {
    /// 组件缓存失效钩子: 等价 Java "组件随 GaugeField 重建" 的语义 — 清空后
    /// 下一帧按新字段的 (label, max_value) 重建组件。见结构体 PORT 契约注。
    pub fn clear(&mut self) {
        self.gauges.clear();
    }
}

impl OverlayRenderer for LinearGaugeRenderer {
    fn render(
        &mut self,
        canvas: &mut PixCanvas,
        fields: &[Field<'_>],
        ctx: &RenderContext,
        offset: &mut [i32; 2],
    ) {
        // Java :21-22 设置 graphAA/textAA hint → ctx.*_aa
        let fontsize = ctx.font_size(); // ctx.numFont.getSize()
        let x = offset[0];
        let y = offset[1];
        let mut dx = 0;
        let mut dy = fontsize >> 1;
        for f in fields {
            // Java :31 !visible || !(instanceof GaugeField) → continue
            let Field::Gauge(gf) = f else { continue };
            if !gf.base.visible {
                continue;
            }
            if gf.gauge.is_none() {
                continue; // Java :38-39 gauge==null 防御
            }
            let gauge = match self.gauges.get_mut(gf.base.key.as_str()) {
                Some(g) => g,
                None => {
                    // GaugeField.java:28: new LabeledLinearGauge(label, maxValue, !isHorizontal)
                    self.gauges.insert(
                        gf.base.key.clone(),
                        LabeledLinearGauge::new(&gf.base.label, gf.max_value, !gf.is_horizontal),
                    );
                    self.gauges.get_mut(gf.base.key.as_str()).expect("刚插入")
                }
            };
            // Java :42/:46 每帧原地赋 gauge.vertical = isHorizontal ? false : true —
            // 幂等状态同步, 等价按场推导
            gauge.gauge.vertical = !gf.is_horizontal;
            // Java 中 gauge 由 GaugeField.updateGauge 更新; Rust 数据态在
            // current_int_value/base, 每帧同步进缓存组件 (同值不置脏)。
            // PORT 值通道契约 (EngineControlOverlay C 批移植必须遵守): Java 双通道
            // 互斥 — (a) 零 GC 路径 (EngineControlOverlay.java:530-541) 只写
            // gf.buffer/length + gauge.update(intVal, buffer, length), 不写
            // GaugeField.currentValue/currentIntValue (恒 "---"/0); (b) String 路径
            // (:571/:579-585/:588-605) 绕过 GaugeField 直写 gauge 组件。本渲染器
            // 消费 gf.current_int_value + gauge_value_text(gf) → 移植时必须统一走
            // vm-core GaugeField::update_gauge (int/文本两通道同写), 不可只写
            // base.buffer/length — 否则条恒画 0 / 文本陈旧 (gauge_value_text 注)。
            gauge.gauge.update(gf.current_int_value, gauge_value_text(gf));
            // draw 的 aa 只传 graph_aa: gauge 内文本在 Java 受 textAA 管, 依赖
            // "graph_aa==text_aa 恒成立" 的生产不变式 (见 RenderContext PORT 注)
            if gf.is_horizontal {
                // Java :43: draw(x, y+dy, 4*fontsize, fontsize>>1, labelFont, labelFont)
                gauge.draw(canvas, x, y + dy, 4 * fontsize, fontsize >> 1, &ctx.label_font, ctx.graph_aa);
                dy += fontsize + (fontsize >> 2); // Java :44
            } else {
                gauge.draw(canvas, x + dx, y, 4 * fontsize, fontsize >> 1, &ctx.label_font, ctx.graph_aa);
                dx += (5 * fontsize) >> 1; // Java :48
            }
        }
    }

    fn calculate_preferred_size(&mut self, fields: &[Field<'_>], ctx: &RenderContext) -> (i32, i32) {
        let fontsize = ctx.font_size();
        let mut row_num = 0;
        // Java :57 columnNum 计数后从未参与公式 (仅循环结构保留), 保真照抄
        let mut _column_num = 0;
        for f in fields {
            let Field::Gauge(gf) = f else { continue };
            if !gf.base.visible {
                continue;
            }
            if gf.is_horizontal {
                row_num += 1;
            } else {
                _column_num += 1;
            }
        }
        let width = fontsize * 8; // Java :70
        // Java :71 `(fontsize*4 + (fontsize*9) >> 1)`: JLS §15.22 移位优先级低于
        // 加法 → (13*fontsize)>>1 (保真陷阱, 勿"顺手"加括号到别处)
        let height = ((fontsize * 4 + fontsize * 9) >> 1) + (row_num + 1) * (fontsize + (fontsize >> 2));
        (width, height)
    }
}

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
        offset[0] = ctx.font_size() >> 1; // Java :30 (覆盖进入值)
        offset[1] = ctx.font_size() >> 1; // Java :31
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
            gauge.update(&base.current_value); // Java :49
            // Java :52: field.unit != null && !equals(gauge.unit) → setUnit
            // PORT: Rust String 恒非 null, 条件退化为不等比较
            if base.unit != gauge.unit {
                gauge.set_unit(&base.unit);
            }
            let value = bos_value_text(base, gauge.value.as_str());
            let val_buffer = if base.length > 0 { Some(value) } else { None };
            gauge.draw(canvas, ctx, offset[0], offset[1], 1, val_buffer); // Java :61-62
            visible_index += 1;
            self.update_offset(visible_index, offset, ctx); // Java :65
        }
    }

    fn calculate_preferred_size(&mut self, fields: &[Field<'_>], ctx: &RenderContext) -> (i32, i32) {
        let visible_count = fields.iter().filter(|f| f.base().visible).count() as i32;
        (ctx.geom.total_width(), ctx.geom.total_height(visible_count)) // Java :86-87
    }
}

// ---------------------------------------------------------------------------
// TextOnlyRenderer (TextOnlyRenderer.java:17)
// ---------------------------------------------------------------------------

pub struct TextOnlyRenderer;

impl OverlayRenderer for TextOnlyRenderer {
    fn render(
        &mut self,
        canvas: &mut PixCanvas,
        fields: &[Field<'_>],
        ctx: &RenderContext,
        offset: &mut [i32; 2],
    ) {
        // Java :26-27: setFont(labelFont) + setColor(WHITE) — 无阴影直画
        let x = ctx.font_size() >> 1; // Java :29
        let mut y = ctx.font_size();  // Java :30 (留出 ascent)
        let line_height = (ctx.font_size() as f64 * 1.5) as i32; // Java :31 (int)(fs*1.5)
        for f in fields {
            let base = f.base();
            if !base.visible {
                continue;
            }
            canvas.draw_text(&ctx.label_font, x, y, &base.current_value, WHITE, ctx.text_aa);
            y += line_height;
        }
        offset[1] = y; // Java :44 (offset[0] 不动)
    }

    fn calculate_preferred_size(&mut self, fields: &[Field<'_>], ctx: &RenderContext) -> (i32, i32) {
        let char_width = ctx.font_size() / 2; // Java :52
        let line_height = (ctx.font_size() as f64 * 1.5) as i32;
        let mut visible_count = 0;
        let mut max_width = 0;
        for f in fields {
            let base = f.base();
            if base.visible {
                visible_count += 1;
                // Java :58 currentValue.length() = UTF-16 码元数; 值域为格式化
                // ASCII 数字串, chars().count() 等价 (§2.1)
                max_width = std::cmp::max(max_width, base.current_value.chars().count() as i32 * char_width);
            }
        }
        let height = visible_count * line_height + ctx.font_size(); // Java :64
        (max_width + ctx.font_size(), height) // Java :66
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONTS: &str = "../../../fonts";

    /// 几何断言走确定性非 AA 路径 (组件内核为 fillRect 像素精确复刻)
    fn ctx_n(column_num: i32) -> RenderContext {
        let mut ctx = RenderContext::load(std::path::Path::new(FONTS), 0, column_num).unwrap();
        ctx.graph_aa = false;
        ctx.text_aa = false;
        ctx
    }

    /// 读像素: to_premul_bgra (BGRA) → [r,g,b,a], 颜色分量为预乘值 (断言以 alpha 为主:
    /// 空=0, shade=42, colorNum 填充=240)
    fn px(buf: &[u8], w: i32, x: i32, y: i32) -> [u8; 4] {
        let i = ((y * w + x) * 4) as usize;
        [buf[i + 2], buf[i + 1], buf[i], buf[i + 3]]
    }

    fn alpha(buf: &[u8], w: i32, x: i32, y: i32) -> u8 {
        px(buf, w, x, y)[3]
    }

    fn data_field(label: &str, unit: &str, value: &str) -> DataField {
        let mut d = DataField::new("k", label, unit, "c", false, false);
        d.set_value(value);
        d
    }

    fn row_has_pixels(buf: &[u8], w: i32, y: i32) -> bool {
        (0..w).any(|x| alpha(buf, w, x, y) > 0)
    }

    // ---- RenderContext (RenderContext.java:68-81/:104-121) ----

    /// 字号派生: label/unit = Math.round(fontSize/2.0f); num_height 同源度量
    #[test]
    fn render_context_geometry_and_half_sizes() {
        let ctx = ctx_n(3);
        assert_eq!(ctx.font_size(), 24);
        assert_eq!(ctx.num_font.size, 24);
        assert_eq!(ctx.label_font.size, 12); // round(12.0)=12
        assert_eq!(ctx.unit_font.size, 12);
        assert_eq!(ctx.num_height(), ctx.num_font.metrics().height);
        assert_eq!(ctx.field_width(), 72); // 3*fontSize
        assert_eq!(ctx.geom.total_width(), 432); // 12 + 3.5*5*24
        // fontAdd=1 → fontSize=25 → half = Math.round(12.5) = 13 (§2.3 半舍入)
        let ctx25 = RenderContext::load(std::path::Path::new(FONTS), 1, 3).unwrap();
        assert_eq!(ctx25.font_size(), 25);
        assert_eq!(ctx25.label_font.size, 13);
        assert_eq!(ctx25.unit_font.size, 13);
    }

    // ---- LinearGaugeRenderer (LinearGaugeRenderer.java:20-74) ----

    /// 布局推进: 竖条 dx += (5*fs)>>1; 横条画于 (x, y+fs>>1);
    /// 首选尺寸 = (8*fs, (13*fs>>1) + (rowNum+1)*(fs + fs>>2)) — 移位优先级保真
    #[test]
    fn linear_gauge_renderer_layout_advancement() {
        let ctx = ctx_n(3);
        let mut g1 = GaugeField::new("rpm", "RPM", "R", 1, 3000, false);
        g1.update_gauge(0, "0");
        let mut g2 = GaugeField::new("thr", "THR", "%", 2, 110, false);
        g2.update_gauge(0, "0");
        let mut g3 = GaugeField::new("flt", "FLT", "%", 3, 100, true);
        g3.update_gauge(0, "0");
        let fields = [Field::Gauge(&g1), Field::Gauge(&g2), Field::Gauge(&g3)];
        let mut r: Box<dyn OverlayRenderer> = Box::new(LinearGaugeRenderer::default());
        let (w, h) = r.calculate_preferred_size(&fields, &ctx);
        // width=192; height=(24*4+24*9)>>1 + (1+1)*(24+6) = 156+60 = 216
        assert_eq!((w, h), (192, 216));
        let mut c = PixCanvas::new(w, h + 20).unwrap();
        let mut offset = [0, 0];
        r.render(&mut c, &fields, &ctx, &mut offset);
        let b = c.to_premul_bgra();
        let m = |t: &str| ctx.label_font.measure(t);
        // 竖条默认分支: 条左缘 = x + (labelW+valueW) + 2
        let tw1 = m("RPM") + m("0");
        let tw2 = m("THR") + m("0");
        assert_eq!(alpha(&b, w, tw1 + 2, 0), 42, "竖条1 条框左上");
        assert_eq!(alpha(&b, w, 60 + tw2 + 2, 0), 42, "竖条2 条框左上 (dx=(5*24)>>1=60)");
        // 横条: (x, y + fs>>1) = (0, 12), 边框盒 0..95 × 12..23; cur=0 → 分隔主线
        // 恰落列 0, 叠在边框上 (colorNum 叠 shade → SrcOver 加深, Java 同款叠色)
        assert!(
            (241..=245).contains(&(alpha(&b, w, 0, 12) as i32)),
            "横条框左上+分隔主线叠加"
        );
        assert_eq!(alpha(&b, w, 5, 12), 42, "横条框上边 (避开分隔线列)");
        assert_eq!(alpha(&b, w, 95, 12), 42, "横条框右上");
        assert_eq!(alpha(&b, w, 96, 12), 0, "横条盒外");
    }

    /// 横条接线 (经渲染器全链路): 填充宽 valW-2 / 分隔线主线+影线各 1 列,
    /// 高度 thickness+字号+2 (gauges_bars 内核几何的渲染器侧钉死)
    #[test]
    fn linear_gauge_renderer_horizontal_wiring() {
        let ctx = ctx_n(3);
        let mut gf = GaugeField::new("flt", "FLT", "%", 3, 100, true);
        gf.update_gauge(50, "50"); // pix = round(50*96/100) = 48
        let fields = [Field::Gauge(&gf)];
        let mut r = LinearGaugeRenderer::default();
        let mut c = PixCanvas::new(192, 60).unwrap();
        let mut offset = [0, 0];
        r.render(&mut c, &fields, &ctx, &mut offset);
        let b = c.to_premul_bgra();
        // (x, y+12)=(0,12), length=96, thickness=12:
        // 边框盒 0..95 × 12..23; 填充 (1,13,46,10) → 列 1..46 × 行 13..22
        assert_eq!(alpha(&b, 192, 1, 13), 240, "填充左上内");
        assert_eq!(alpha(&b, 192, 46, 22), 240, "填充右下内 (valW-2)");
        assert_eq!(alpha(&b, 192, 47, 17), 0, "填充右外");
        // 分隔线: 主线列 48, 影线列 49, 行 12..12+26
        assert_eq!(alpha(&b, 192, 48, 17), 240, "主线");
        assert_eq!(alpha(&b, 192, 49, 17), 42, "影线 (+1)");
        assert_eq!(alpha(&b, 192, 47, 17), 0, "主线左邻空");
        assert_eq!(alpha(&b, 192, 50, 17), 0, "影线右邻空");
        assert_eq!(alpha(&b, 192, 48, 38), 240, "主线末端行 (y+thickness+font+2)");
        assert_eq!(alpha(&b, 192, 48, 39), 0, "线外");
    }

    /// 首选尺寸计数: 仅可见 GaugeField; Data 字段与不可见字段不参与
    #[test]
    fn linear_gauge_renderer_preferred_counts() {
        let ctx = ctx_n(3);
        let mut vis = GaugeField::new("a", "A", "", 1, 10, false);
        vis.update_gauge(0, "0");
        let mut invis = GaugeField::new("b", "B", "", 1, 10, true);
        invis.update_gauge(0, "0");
        invis.base.visible = false;
        let data = data_field("L", "U", "123");
        let fields = [Field::Gauge(&vis), Field::Gauge(&invis), Field::Data(&data)];
        let mut r = LinearGaugeRenderer::default();
        // 1 竖 0 横 (invis 横向不可见被跳过): 156 + 1*30 = 186
        assert_eq!(r.calculate_preferred_size(&fields, &ctx), (192, 186));
        // 2 横: 156 + 3*30 = 246
        let h1 = GaugeField::new("c", "C", "", 1, 10, true);
        let h2 = GaugeField::new("d", "D", "", 1, 10, true);
        let fields2 = [Field::Gauge(&h1), Field::Gauge(&h2)];
        assert_eq!(r.calculate_preferred_size(&fields2, &ctx), (192, 246));
    }

    /// 组件缓存复用: 第二帧同 key 不新建组件, 值变化走 update 脏检查
    #[test]
    fn linear_gauge_renderer_cache_reuse() {
        let ctx = ctx_n(3);
        let mut gf = GaugeField::new("rpm", "RPM", "", 1, 100, true);
        gf.update_gauge(50, "50");
        let fields1 = [Field::Gauge(&gf)];
        let mut r = LinearGaugeRenderer::default();
        let mut c = PixCanvas::new(192, 216).unwrap();
        let mut off = [0, 0];
        r.render(&mut c, &fields1, &ctx, &mut off);
        assert_eq!(r.gauges.len(), 1);
        let comp = r.gauges.get("rpm").unwrap();
        assert!(!comp.gauge.is_dirty(), "draw 后清脏");
        assert_eq!(comp.gauge.cur_value, 50);
        // 值变化 → update 置脏, 仍复用同组件
        gf.update_gauge(60, "60");
        let fields2 = [Field::Gauge(&gf)];
        r.render(&mut c, &fields2, &ctx, &mut off);
        assert_eq!(r.gauges.len(), 1, "缓存复用");
        assert_eq!(r.gauges.get("rpm").unwrap().gauge.cur_value, 60, "值同步进组件");
    }

    // ---- BosStyleRenderer (BOSStyleRenderer.java:21-90) ----

    /// offset 网格推进 (columnNum 换行) + TextGauge 按 label 缓存 + 单位动态同步
    #[test]
    fn bos_style_offset_grid_and_cache() {
        let ctx = ctx_n(3);
        let f1 = data_field("A1", "Km/h", "100");
        let f2 = data_field("A2", "Km/h", "200");
        let f3 = data_field("A3", "Km/h", "300");
        let f4 = data_field("A4", "M", "400");
        let fields = [Field::Data(&f1), Field::Data(&f2), Field::Data(&f3), Field::Data(&f4)];
        let mut r = BosStyleRenderer::default();
        assert_eq!(r.cached_gauge_count(), 0);
        let dynr: &mut dyn OverlayRenderer = &mut r;
        let (w, h) = dynr.calculate_preferred_size(&fields, &ctx);
        assert_eq!(w, 432); // total_width
        assert_eq!(h, ctx.geom.total_height(4)); // 4 字段 3 列: addnum=1 → 3 行
        let mut c = PixCanvas::new(w, h).unwrap();
        let mut offset = [999, 999]; // 进入值应被覆盖 (Java :30-31)
        dynr.render(&mut c, &fields, &ctx, &mut offset);
        // 4 可见 → 4%3≠0: ox=12+120, oy=12+num_height
        assert_eq!(offset, [132, 12 + ctx.num_height()]);
        assert_eq!(r.cached_gauge_count(), 4, "按 label 缓存 4 个 TextGauge");
        // 单位动态同步 (Java :52-53): 同 label 换单位 → 缓存内更新, 不新建
        let f4b = data_field("A4", "Ft", "400");
        let fields2 = [Field::Data(&f4b)];
        let mut off2 = [0, 0];
        OverlayRenderer::render(&mut r, &mut c, &fields2, &ctx, &mut off2);
        assert_eq!(r.cached_unit("A4"), Some("Ft"));
        assert_eq!(r.cached_gauge_count(), 4, "缓存复用");
    }

    /// 内容包围盒: 不越左缘 (fontSize>>1), 第 4 字段落位第二行, 不超首选高
    #[test]
    fn bos_style_content_bounds() {
        let ctx = ctx_n(3);
        let f1 = data_field("A1", "Km/h", "100");
        let f2 = data_field("A2", "Km/h", "200");
        let f3 = data_field("A3", "Km/h", "300");
        let f4 = data_field("A4", "M", "400");
        let fields = [Field::Data(&f1), Field::Data(&f2), Field::Data(&f3), Field::Data(&f4)];
        let mut r = BosStyleRenderer::default();
        let (w, h) = r.calculate_preferred_size(&fields, &ctx);
        let mut c = PixCanvas::new(w, h).unwrap();
        let mut offset = [0, 0];
        r.render(&mut c, &fields, &ctx, &mut offset);
        let b = c.to_premul_bgra();
        let (mut min_x, mut min_y, mut max_y, mut any) = (i32::MAX, i32::MAX, i32::MIN, false);
        for y in 0..h {
            for x in 0..w {
                if alpha(&b, w, x, y) > 0 {
                    any = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }
        assert!(any, "有内容");
        assert!(min_x >= 12, "内容不越左缘 (offset 初值 fontSize>>1), min_x={min_x}");
        assert!(min_y < 15, "首行标签贴近顶部, min_y={min_y}");
        assert!(max_y >= 12 + ctx.num_height() - 11, "第 4 字段进入第二行, max_y={max_y}");
        assert!(max_y < h, "不超首选高");
    }

    // ---- TextOnlyRenderer (TextOnlyRenderer.java:20-67) ----

    /// 行布局: 基线 y=fontSize 起, 行高 (int)(fontSize*1.5); offset[0] 不动;
    /// 首选尺寸 = (maxLen*fs/2 + fs, n*行高 + fs)
    #[test]
    fn text_only_layout() {
        let ctx = ctx_n(1);
        let f1 = data_field("L1", "", "12345");
        let f2 = data_field("L2", "", "7"); // set_value 补宽 → "    7" (5 码元)
        let fields = [Field::Data(&f1), Field::Data(&f2)];
        let mut r = TextOnlyRenderer;
        // 两字段均 %5s 后 5 字符: maxW=5*12=60 → (84, 2*36+24=96)
        assert_eq!(r.calculate_preferred_size(&fields, &ctx), (84, 96));
        let mut c = PixCanvas::new(200, 120).unwrap();
        let mut offset = [77, 5];
        r.render(&mut c, &fields, &ctx, &mut offset);
        assert_eq!(offset, [77, 96], "offset[0] 不动; offset[1] = 24 + 2*36");
        let b = c.to_premul_bgra();
        assert!((14..=26).any(|y| row_has_pixels(&b, 200, y)), "第一行 (基线 24)");
        assert!((30..=45).all(|y| !row_has_pixels(&b, 200, y)), "行间空");
        assert!((50..=62).any(|y| row_has_pixels(&b, 200, y)), "第二行 (基线 60)");
        assert!((66..120).all(|y| !row_has_pixels(&b, 200, y)), "第二行之下空");
    }

    /// 不可见字段全局过滤 (渲染器共用 base.visible 通道)
    #[test]
    fn field_visibility_filtering() {
        let ctx = ctx_n(1);
        let f1 = data_field("L1", "", "100");
        let mut f2 = data_field("L2", "", "200");
        f2.visible = false;
        let fields = [Field::Data(&f1), Field::Data(&f2)];
        let mut r = TextOnlyRenderer;
        assert_eq!(r.calculate_preferred_size(&fields, &ctx), (84, 60)); // 1 行: 36+24
        let mut c = PixCanvas::new(200, 120).unwrap();
        let mut offset = [0, 0];
        r.render(&mut c, &fields, &ctx, &mut offset);
        assert_eq!(offset[1], 60, "只推进 1 行");
        let b = c.to_premul_bgra();
        assert!((14..=26).any(|y| row_has_pixels(&b, 200, y)), "第一行在");
        assert!((30..120).all(|y| !row_has_pixels(&b, 200, y)), "第二行不可见");
    }

    // ---- 数值文本双通道解析 ----

    /// buffer/length 通道优先于字符串值 (TextGauge.java:54 / LinearGauge.java:266)
    #[test]
    fn value_text_channel_resolution() {
        let mut d = data_field("L", "U", "88");
        d.buffer = "88".to_string();
        d.length = 2;
        assert_eq!(bos_value_text(&d, "  88"), "88");
        d.length = 0;
        assert_eq!(bos_value_text(&d, "  88"), "  88");
        let mut gf = GaugeField::new("rpm", "RPM", "", 1, 100, false);
        gf.update_gauge(50, "50");
        assert_eq!(gauge_value_text(&gf), "50");
        gf.base.buffer = "49".to_string();
        gf.base.length = 2;
        assert_eq!(gauge_value_text(&gf), "49");
    }

    /// AA 路径冒烟: 默认 graphAA/textAA 全开不 panic 且有输出
    #[test]
    fn aa_smoke_all_renderers() {
        let ctx = RenderContext::load(std::path::Path::new(FONTS), 0, 3).unwrap();
        let mut g = GaugeField::new("rpm", "RPM", "", 1, 100, true);
        g.update_gauge(50, "50");
        let d = data_field("IAS", "Km/h", "800");
        let fields = [Field::Gauge(&g), Field::Data(&d)];
        let mut c = PixCanvas::new(432, 200).unwrap();
        let mut off = [0, 0];
        LinearGaugeRenderer::default().render(&mut c, &fields, &ctx, &mut off);
        let mut off = [0, 0];
        BosStyleRenderer::default().render(&mut c, &fields, &ctx, &mut off);
        let mut off = [0, 0];
        TextOnlyRenderer.render(&mut c, &fields, &ctx, &mut off);
        assert!(c.to_premul_bgra().iter().any(|&v| v != 0), "AA 输出非空");
    }

    // ---- 缓存失效契约 / 防护 ----

    /// clear() 失效钩子: 清空后按新字段构造参数 (label, max_value) 重建组件 —
    /// 等价 Java "LabeledLinearGauge 组件随 GaugeField 重建而消亡"
    #[test]
    fn linear_gauge_renderer_clear_rebuilds_with_new_range() {
        let ctx = ctx_n(3);
        let mut gf = GaugeField::new("rpm", "RPM", "", 1, 100, true);
        gf.update_gauge(50, "50");
        let fields = [Field::Gauge(&gf)];
        let mut r = LinearGaugeRenderer::default();
        let mut c = PixCanvas::new(192, 216).unwrap();
        let mut off = [0, 0];
        r.render(&mut c, &fields, &ctx, &mut off);
        assert_eq!(r.gauges.get("rpm").unwrap().gauge.max_value, 100);
        // 模拟 FieldOverlay.reinitConfig → clearAll 后同 key 字段换量程 (100→200)
        let mut gf2 = GaugeField::new("rpm", "RPM", "", 1, 200, true);
        gf2.update_gauge(50, "50");
        let fields2 = [Field::Gauge(&gf2)];
        r.clear();
        assert!(r.gauges.is_empty(), "clear 后缓存空");
        let mut off = [0, 0];
        r.render(&mut c, &fields2, &ctx, &mut off);
        assert_eq!(r.gauges.get("rpm").unwrap().gauge.max_value, 200, "重建后取新量程");
    }

    /// 不 clear 直接复用 → 首帧量程固化 (钉死契约失败面: 调用方必须 clear,
    /// Java 中组件随字段重建天然刷新, Rust 缓存不会)
    #[test]
    fn linear_gauge_renderer_cache_freezes_range_without_clear() {
        let ctx = ctx_n(3);
        let mut gf = GaugeField::new("rpm", "RPM", "", 1, 100, true);
        gf.update_gauge(50, "50");
        let fields1 = [Field::Gauge(&gf)];
        let mut r = LinearGaugeRenderer::default();
        let mut c = PixCanvas::new(192, 216).unwrap();
        let mut off = [0, 0];
        r.render(&mut c, &fields1, &ctx, &mut off);
        let mut gf2 = GaugeField::new("rpm", "RPM", "", 1, 200, true);
        gf2.update_gauge(50, "50");
        let fields2 = [Field::Gauge(&gf2)];
        let mut off = [0, 0];
        r.render(&mut c, &fields2, &ctx, &mut off);
        assert_eq!(r.gauges.len(), 1, "同 key 复用");
        assert_eq!(r.gauges.get("rpm").unwrap().gauge.max_value, 100, "量程陈旧 (契约失败面)");
    }

    /// BOS clear(): 仅释放语义 (Java gaugeCache 本就跨字段重建存活, 无失效需求)
    #[test]
    fn bos_style_clear_releases_cache() {
        let ctx = ctx_n(3);
        let f = data_field("A1", "Km/h", "100");
        let fields = [Field::Data(&f)];
        let mut r = BosStyleRenderer::default();
        let mut c = PixCanvas::new(60, 40).unwrap();
        let mut off = [0, 0];
        r.render(&mut c, &fields, &ctx, &mut off);
        assert_eq!(r.cached_gauge_count(), 1);
        r.clear();
        assert_eq!(r.cached_gauge_count(), 0, "clear 后缓存空");
        let mut off = [0, 0];
        r.render(&mut c, &fields, &ctx, &mut off);
        assert_eq!(r.cached_gauge_count(), 1, "再渲染按需重建");
    }

    /// load() 字体校验: 损坏 ttf 返回 Err 而非 metrics() 处 panic
    /// (Java new Font 静默回退 dialog 字体; Rust 无回退族 → 显式 Err, POC 裁决)
    #[test]
    fn load_rejects_corrupt_font() {
        let dir = std::env::temp_dir().join("vm_overlay_renderers_font_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("sarasa-mono-sc-bold.ttf"), b"definitely not a ttf").unwrap();
        let result = RenderContext::load(&dir, 0, 3);
        let _ = std::fs::remove_dir_all(&dir);
        let err = match result {
            Ok(_) => panic!("损坏字体应返回 Err, 却构造成功"),
            Err(e) => e,
        };
        assert!(
            err.contains("损坏") || err.contains("无法解析"),
            "错误信息应指向字体损坏: {err}"
        );
    }

    /// columnNum==0 程序化误用: 显式 panic (Java 同位置 ArithmeticException, 行为对等)
    #[test]
    #[should_panic(expected = "columnNum==0")]
    fn bos_style_column_zero_panics_like_java() {
        let ctx = ctx_n(0);
        let f = data_field("A1", "Km/h", "100");
        let fields = [Field::Data(&f)];
        let mut r = BosStyleRenderer::default();
        let mut c = PixCanvas::new(10, 10).unwrap();
        let mut off = [0, 0];
        r.render(&mut c, &fields, &ctx, &mut off);
    }

    /// 默认态 palette() = Java 静态初始值 (对拍基线常量同源)。
    /// 动态性 (set 后跟随) 不以 set 操演断言: 并行渲染测试读同一仓, set 窗口
    /// 会互踩 (实测撞 linear_gauge 断言); palette() 方法体直调 colors() 无缓存,
    /// 动态性由结构保证 + global_colors.rs 的仓往返测试覆盖
    #[test]
    fn palette_defaults_to_java_static_values() {
        assert_eq!(ctx_n(1).palette(), APPLICATION_COLORS);
    }
}
