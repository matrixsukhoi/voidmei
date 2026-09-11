//! rows: HUD 文本行组件族 (MiniHUD 左侧数据列) — 行族原子化产物。
//!
//! - HUDTextRow — 主文本行基座: 基线 = y+ascent, 警告色/常态色, 模板锁宽。
//! - SpeedReadout/AltitudeReadout/GLoadReadout/SepReadout — 行主读数
//!   (HUDTextRow 包装; 数据槽/模板槽由 widgets::minihud 的 HudWidget impl 区分)。
//! - AoaGauge — 速度行辅件: AoA 横条(drawHRect) + α 小字 (条右端 x+rightDraw)。
//! - EnergyReadout — 高度行辅件: 右置能量小字 (左缘 x+rightDraw, 同基线)。
//! - MechPart — 机械化行单段 (襟翼/减速板/起落架, MechKind 分数据槽);
//!   段间距由页面布局 pos 表达 (原行内 curX 模板宽推进的换算:
//!   等宽字体 advance = 0.5em → 段推进 = (模板字符数+尾随空格) × 0.5 行高)。
//! - ManeuverBar — G 行辅件: 机动指数刻度 + thick 影线/thin 主线双层条
//!   (右端固定 x+rightDraw, 向左延展)。
//!
//! 绘制目标 = render2d::PixCanvas; Java extends HUDTextRow 统一映射为组合
//! (newtype 包装/独立结构, 禁止造继承); 颜色/坐标公式逐项对照
//! Java paint 逻辑 (关键处 // 标注)。
//!
//! // Java HUDRow 接口 (HUDRow.java) 的 getPreferredSize 默认 (200, getHeight)
//! 由 preferred_size 实现覆盖, 不单独建 trait —— Rust 侧该接口无第二实现需求。

use crate::render::palette::colors;
use crate::render::primitives::{self, draw_h_rect};

use crate::render::font::LoadedFont;

use crate::render::canvas::PixCanvas;

/// Java Color.YELLOW (构造默认)
const COLOR_YELLOW: [u8; 4] = [255, 255, 0, 255];

// ---------------------------------------------------------------------------
// HUDTextRow (族基类 → 组合基座)
// ---------------------------------------------------------------------------

/// 简单文本行。上左角 (x,y) 入参, 内部换算基线。
pub struct HUDTextRow {
    /// 行号 (调试/getId 用)
    pub index: i32,
    /// 主文字 (Java protected text, 构造置 "")
    pub text: String,
    /// 行高 (Java protected height)
    pub height: i32,
    /// 模板文字锁宽 (Java templateText, null=未设)
    pub template: Option<String>,
    /// 警告态 → colorWarning, 否则 colorNum
    pub is_warning: bool,
    /// 组件可见性 (AbstractHUDComponent.visible, 布局引擎门控 — draw 本身不检查,
    /// 对齐 ModernHUDLayoutEngine 的调用侧检查)
    pub visible: bool,
}

impl HUDTextRow {
    /// 构造 (font 在 Rust 侧为 draw 参数, 不入结构体)
    pub fn new(index: i32, height: i32) -> Self {
        HUDTextRow {
            index,
            text: String::new(),
            height,
            template: None,
            is_warning: false,
            visible: true,
        }
    }

    /// getId
    pub fn id(&self) -> String {
        format!("row.{}", self.index)
    }

    /// setStyle (仅 height; font 为 draw 参数)
    pub fn set_style(&mut self, height: i32) {
        self.height = height;
    }

    /// update(text, isWarning)。返回内容是否变化 (组装侧脏检查用,
    /// Java 返回 void —— 行为等价, 附加元数据)。
    pub fn update(&mut self, text: &str, is_warning: bool) -> bool {
        let changed = self.text != text || self.is_warning != is_warning;
        self.text.clear();
        self.text.push_str(text);
        self.is_warning = is_warning;
        changed
    }

    /// setTemplate (null 语义由 None 承载)
    pub fn set_template(&mut self, template: Option<&str>) {
        self.template = template.map(|s| s.to_string());
    }

    /// draw: Top-Left y → Baseline y 换算后阴影双遍文本。
    /// 警告 → colorWarning, 常态 → colorNum。
    pub fn draw(&self, cv: &mut PixCanvas, x: i32, y: i32, font: &LoadedFont, aa: bool) {
        // ascent = getFontMetrics(font).getAscent(); baseY = y + ascent
        let ascent = font.metrics().ascent;
        let base_y = y + ascent;
        let c = if self.is_warning {
            colors().warning
        } else {
            colors().num
        };
        primitives::text_shaded_auto(cv, font, x, base_y, &self.text, c, aa);
    }

    /// getPreferredSize: 模板优先测量 (布局防抖), 空文本宽 0。
    /// 返回 (w, h) 对应 java.awt.Dimension。
    pub fn preferred_size(&self, font: &LoadedFont) -> (i32, i32) {
        // w=200 起始, 但非空测量路径必覆盖 (getStringWidth 空串=0)
        let text_to_measure: &str = match &self.template {
            Some(t) if !t.is_empty() => t,
            _ => &self.text,
        };
        let w = font.measure(text_to_measure);
        (w, self.height)
    }
}

// ---------------------------------------------------------------------------
// 行主读数 newtype (同结构异数据面 — HudWidget impl 按类型分发)
// ---------------------------------------------------------------------------

/// 速度读数 (原 Row0 速度主文字段)。
pub struct SpeedReadout(pub HUDTextRow);

/// 高度读数 (原 Row1 高度主文字段)。
pub struct AltitudeReadout(pub HUDTextRow);

/// SEP 读数 (原 Row3 整行)。
pub struct SepReadout(pub HUDTextRow);

/// G 读数 (原 Row4 G 主文字段)。
pub struct GLoadReadout(pub HUDTextRow);

// ---------------------------------------------------------------------------
// AoaGauge (速度行辅件: AoA 横条 + α 小字)
// ---------------------------------------------------------------------------

/// AoA 指示器: 横条右端锚 x+rightDraw (向左延展), α 小字续于条右端。
pub struct AoaGauge {
    /// AoA 读数文字 (小字号)
    pub aoa_text: String,
    /// AoA 条有效长度像素 (aoaRatio × aoaLength, 钳到 rightDraw)
    pub aoa_y: i32,
    /// 右侧绘制基准 X 偏移 (条右端/α 文字左缘 = x + rightDraw)
    pub right_draw: i32,
    pub line_width: i32,
    /// aoaLength 默认 100 (setStyle 注入生产值)
    pub aoa_length: i32,
    /// α 文字色 (Java aoaColor, 构造默认 YELLOW)
    pub aoa_color: [u8; 4],
    /// AoA 条填充色 (Java aoaBarColor, 构造默认 YELLOW)
    pub aoa_bar_color: [u8; 4],
    /// α 文字模板 (宽度估算用)
    pub aoa_template: Option<String>,
    /// 行高 (布局盒高)
    pub height: i32,
}

impl AoaGauge {
    /// 构造 (fonts 为 draw 参数)
    pub fn new(height: i32, right_draw: i32, line_width: i32) -> Self {
        AoaGauge {
            aoa_text: String::new(),
            aoa_y: 0,
            right_draw,
            line_width,
            aoa_length: 100,
            aoa_color: COLOR_YELLOW,
            aoa_bar_color: COLOR_YELLOW,
            aoa_template: None,
            height,
        }
    }

    /// setStyle (AoA 专属几何)
    pub fn set_style(&mut self, right_draw: i32, line_width: i32, aoa_length: i32) {
        self.right_draw = right_draw;
        self.line_width = line_width;
        self.aoa_length = aoa_length;
    }

    /// setTemplate 的 aoa 槽
    pub fn set_template(&mut self, aoa: Option<&str>) {
        self.aoa_template = aoa.map(|s| s.to_string());
    }

    /// 条长计算 (Java onDataUpdate 69-72):
    /// aoaY = (int)(aoaRatio * aoaLength), 钳到 rightDraw。
    /// // Java double→int 强转 (JLS 5.1.3) = NaN→0 + 超范围饱和到
    /// MIN/MAX, 与 Rust as i32 语义完全一致 — 两语言无差异 (§2.2 的截断/
    /// 回绕差异仅适用于 long→int 整数窄化, 不适用本处浮点转换)
    pub fn set_aoa_from_ratio(&mut self, aoa_ratio: f64) {
        self.aoa_y = (aoa_ratio * self.aoa_length as f64) as i32;
        if self.aoa_y > self.right_draw {
            self.aoa_y = self.right_draw;
        }
    }

    /// 手动 update (预览模式路径; 游戏模式数据映射见 set_aoa_from_ratio)
    pub fn update(
        &mut self,
        aoa_text: &str,
        aoa_y: i32,
        aoa_color: [u8; 4],
        aoa_bar_color: [u8; 4],
    ) -> bool {
        // 先判后写 (脏检查全字段参与)
        let changed = self.aoa_text != aoa_text
            || self.aoa_y != aoa_y
            || self.aoa_color != aoa_color
            || self.aoa_bar_color != aoa_bar_color;
        self.aoa_text.clear();
        self.aoa_text.push_str(aoa_text);
        self.aoa_y = aoa_y;
        self.aoa_color = aoa_color;
        self.aoa_bar_color = aoa_bar_color;
        changed
    }

    /// draw (ascent 取主字体; liney = baseY + 1)。
    pub fn draw(
        &self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        font: &LoadedFont,
        small_font: &LoadedFont,
        aa: bool,
    ) {
        let ascent = font.metrics().ascent;
        let liney = y + ascent + 1;
        // drawHRect(x + (rightDraw - aoaY), liney, aoaY, lineWidth+3, 1, aoaBarColor)
        draw_h_rect(
            cv,
            x + (self.right_draw - self.aoa_y),
            liney,
            self.aoa_y,
            self.line_width + 3,
            1,
            self.aoa_bar_color,
        );
        // α 文字基线 liney - 1, 小字号
        primitives::text_shaded_auto(
            cv,
            small_font,
            x + self.right_draw,
            liney - 1,
            &self.aoa_text,
            self.aoa_color,
            aa,
        );
    }

    /// getPreferredSize: rightDraw + α 宽 (rightDraw 恒占位, 布局稳定)。
    pub fn preferred_size(&self, small_font: &LoadedFont) -> (i32, i32) {
        // aoaTemplate != null ? aoaTemplate : aoaText (无空串检查)
        let measure_aoa: &str = self.aoa_template.as_deref().unwrap_or(&self.aoa_text);
        (self.right_draw + small_font.measure(measure_aoa), self.height)
    }
}

// ---------------------------------------------------------------------------
// EnergyReadout (高度行辅件: 右置能量小字)
// ---------------------------------------------------------------------------

/// 能量读数: 小字号, 左缘 = x + rightDraw, 与行主文字同基线, 色恒 colorNum。
pub struct EnergyReadout {
    /// 能量读数串
    pub energy_text: String,
    /// 文字左缘 = x + rightDraw
    pub right_draw: i32,
    pub energy_template: Option<String>,
    /// 行高 (布局盒高)
    pub height: i32,
}

impl EnergyReadout {
    /// 构造
    pub fn new(height: i32, right_draw: i32) -> Self {
        EnergyReadout {
            energy_text: String::new(),
            right_draw,
            energy_template: None,
            height,
        }
    }

    /// setStyle
    pub fn set_style(&mut self, right_draw: i32) {
        self.right_draw = right_draw;
    }

    /// setTemplate (energy 槽)
    pub fn set_template(&mut self, energy: Option<&str>) {
        self.energy_template = energy.map(|s| s.to_string());
    }

    /// update (返回是否变化 — 能量逐帧变化, 脏检查参与)
    pub fn update(&mut self, energy_text: &str) -> bool {
        let changed = self.energy_text != energy_text;
        self.energy_text.clear();
        self.energy_text.push_str(energy_text);
        changed
    }

    /// draw (ascent 取主字体, 能量文字与主文字同基线 baseY)。
    /// 能量色恒 colorNum (注释: 已统一, 不再传色)。
    pub fn draw(
        &self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        font: &LoadedFont,
        small_font: &LoadedFont,
        aa: bool,
    ) {
        let ascent = font.metrics().ascent;
        let base_y = y + ascent;
        // __drawStringShade(x + rightDraw, baseY, 1, energyText, smallFont, colorNum)
        primitives::text_shaded_auto(
            cv,
            small_font,
            x + self.right_draw,
            base_y,
            &self.energy_text,
            colors().num,
            aa,
        );
    }

    /// getPreferredSize: rightDraw + 能量宽 (rightDraw 恒占位)。
    pub fn preferred_size(&self, small_font: &LoadedFont) -> (i32, i32) {
        // energyTemplate != null ? energyTemplate : energyText
        let measure_en: &str = self.energy_template.as_deref().unwrap_or(&self.energy_text);
        (self.right_draw + small_font.measure(measure_en), self.height)
    }
}

// ---------------------------------------------------------------------------
// MechPart (机械化行单段: 襟翼/减速板/起落架)
// ---------------------------------------------------------------------------

/// 机械化段种类 (数据槽/默认模板区分; 三段同色源 = warnConfiguration)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MechKind {
    /// 襟翼/可变翼段 (0..4; 空模板回退 "F100")
    Flaps,
    /// 减速板段 (4..7)
    Airbrake,
    /// 起落架段 (7..10)
    Gear,
}

/// 机械化行单段: 数据非空才绘制, 模板恒锁宽 (空数据不缩布局)。
/// 段间距由页面布局 pos 表达 (见模块头换算注)。
pub struct MechPart {
    pub kind: MechKind,
    /// 段数据串 (空 = 不绘制)
    pub text: String,
    /// 段模板 (宽度估算; 默认 W100/BRK/GEA 对应 Java 前代)
    pub template: String,
    /// 警告态 → colorWarning, 否则 colorNum
    pub is_warning: bool,
    /// 行高 (布局盒高)
    pub height: i32,
}

impl MechPart {
    /// 构造 (font 为 draw/preferred 参数, 不入结构体)
    pub fn new(kind: MechKind, height: i32) -> Self {
        let template = match kind {
            MechKind::Flaps => "W100",
            MechKind::Airbrake => "BRK",
            MechKind::Gear => "GEA",
        };
        MechPart {
            kind,
            text: String::new(),
            template: template.to_string(),
            is_warning: false,
            height,
        }
    }

    /// setStyle (仅 height)
    pub fn set_style(&mut self, height: i32) {
        self.height = height;
    }

    /// setTemplate 单段槽 (襟翼空段回退 "F100", Java:77)
    pub fn set_template(&mut self, seg: &str) {
        self.template = seg.to_string();
        if self.kind == MechKind::Flaps && self.template.is_empty() {
            self.template = "F100".to_string();
        }
    }

    /// update (先判后写, text/is_warning 全参与)
    pub fn update(&mut self, text: &str, is_warning: bool) -> bool {
        let changed = self.text != text || self.is_warning != is_warning;
        self.text.clear();
        self.text.push_str(text);
        self.is_warning = is_warning;
        changed
    }

    /// draw: 数据非空才绘制文字 (同基线 baseY, 主字体)。
    pub fn draw(&self, cv: &mut PixCanvas, x: i32, y: i32, font: &LoadedFont, aa: bool) {
        if self.text.is_empty() {
            return;
        }
        // ascent = getFontMetrics(font).getAscent(); baseY = y + ascent
        let base_y = y + font.metrics().ascent;
        // isWarning ? colorWarning : colorNum
        let c = if self.is_warning {
            colors().warning
        } else {
            colors().num
        };
        primitives::text_shaded_auto(cv, font, x, base_y, &self.text, c, aa);
    }

    /// getPreferredSize: 模板宽 (段间距归布局 pos, 不含尾随空格)。
    pub fn preferred_size(&self, font: &LoadedFont) -> (i32, i32) {
        let w = if self.template.is_empty() {
            0
        } else {
            font.measure(&self.template)
        };
        (w, self.height)
    }
}

/// / 75-80 共用的三段切分: 0..4 / 4..7 / 7..10 各自 trim。
/// // Java substring + length()>=10 按 UTF-16 码元; 输入域为
/// HUDCalculator 的 mechanization 格式串 (纯 ASCII: F/W 前缀+数字+空格+BRK/GEA),
/// 字节索引与 UTF-16 索引等价。Java trim() 删两端 <=U+0020, Rust trim()
/// 删 Unicode 空白 — ASCII 域内等价。
pub(crate) fn split_trim3(text: &str) -> Option<(String, String, String)> {
    let b = text.as_bytes();
    if b.len() < 10 {
        return None;
    }
    let seg = |r: std::ops::Range<usize>| -> String {
        let bytes = &b[r];
        // ASCII 域论证下 from_utf8 恒成功; debug_assert 让域漂移 (切分点落在
        // 非 ASCII 字节) 在测试期响亮失败, release 静默回退空段保运行 (Java
        // substring 会切出乱码文本而非空串 — 域内不可达, 保真不受影响)
        debug_assert!(
            std::str::from_utf8(bytes).is_ok(),
            "mechanization 切分点落在非 ASCII 域: {text:?}"
        );
        std::str::from_utf8(bytes).unwrap_or("").trim().to_string()
    };
    Some((seg(0..4), seg(4..7), seg(7..10)))
}

// ---------------------------------------------------------------------------
// ManeuverBar (G 行辅件: 机动指数刻度条)
// ---------------------------------------------------------------------------

/// 机动条满量程 (Java lenN = N/0.5 × rightDraw 系列公式的 0.5)
pub const MANEUVER_FULL_SCALE: f64 = 0.5;

/// 刻度档位表 0.1~0.5 (阈值本就是数据; 原 len10..len50 五连平行字段的来源)
pub const MANEUVER_TICK_STEPS: [f64; 5] = [0.1, 0.2, 0.3, 0.4, 0.5];

/// 机动指数刻度尺: 各档刻度到条右端的距离 (F10 收敛 len10..len50 平行字段)。
/// 档 i 距离 = round(档位/满量程 × rightDraw), 由 minihud 的 legacy 链计算注入。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TickScale {
    /// 各档距离, 序 = [`MANEUVER_TICK_STEPS`]
    pub ticks: [i32; 5],
}

impl TickScale {
    /// 点亮档位的距离迭代: 档 0 恒亮 (len10 恒画), 档 i 在
    /// index ≥ 前一档档位时亮 (的 0.1~0.4 逐级阈值)
    pub fn lit_lens(&self, index: f64) -> impl Iterator<Item = i32> + '_ {
        self.ticks
            .iter()
            .enumerate()
            .filter(move |&(i, _)| i == 0 || index >= MANEUVER_TICK_STEPS[i - 1])
            .map(|(_, &len)| len)
    }
}

/// 机动指数刻度条: 各档 1px 竖刻度 + 当前值条线 (thick 影线/thin 主线双层描边),
/// 右端固定 x+rightDraw 向左延展。
/// Java 的 strokeThick/strokeThin (BasicStroke, CAP_ROUND+JOIN_ROUND,
/// MinimalHUDContext 造: 宽 halfLine+2 / halfLine) 在 Rust 侧
/// 仅宽度可变 → 存 f32 宽度, 线型由 PixCanvas::draw_line (Round) 固定。
pub struct ManeuverBar {
    pub right_draw: i32,
    pub half_line: i32,
    pub line_width: i32,
    /// 机动指数 0..0.5+ (刻度点亮阈值 0.1~0.4)
    pub maneuver_index: f64,
    /// 当前值条长 (右端固定 x+rightDraw, 向左延展)
    pub maneuver_index_len: i32,
    /// 各档刻度距离 (0.1~0.5 档位表见 [`MANEUVER_TICK_STEPS`])
    pub tick_scale: TickScale,
    /// strokeThick 宽 (影线)
    pub stroke_thick_w: f32,
    /// strokeThin 宽 (主线)
    pub stroke_thin_w: f32,
    /// 刻度色态 (warning → colorWarning; live 恒 false, 原行 G 文字警告位)
    pub is_warning: bool,
    /// 行高 (布局盒高)
    pub height: i32,
}

impl ManeuverBar {
    /// 构造 (strokes 以宽度入参, cap/join 恒 ROUND)
    #[allow(clippy::too_many_arguments)] // 对齐 Java 构造 8 参
    pub fn new(
        height: i32,
        right_draw: i32,
        half_line: i32,
        line_width: i32,
        stroke_thick_w: f32,
        stroke_thin_w: f32,
    ) -> Self {
        ManeuverBar {
            right_draw,
            half_line,
            line_width,
            maneuver_index: 0.0,
            maneuver_index_len: 0,
            tick_scale: TickScale::default(),
            stroke_thick_w,
            stroke_thin_w,
            is_warning: false,
            height,
        }
    }

    /// setStyle
    #[allow(clippy::too_many_arguments)] // 对齐 Java setStyle 8 参
    pub fn set_style(
        &mut self,
        height: i32,
        right_draw: i32,
        half_line: i32,
        line_width: i32,
        stroke_thick_w: f32,
        stroke_thin_w: f32,
    ) {
        self.height = height;
        self.right_draw = right_draw;
        self.half_line = half_line;
        self.line_width = line_width;
        self.stroke_thick_w = stroke_thick_w;
        self.stroke_thin_w = stroke_thin_w;
    }

    /// update (len = 当前条长, tick_scale = 各阈值刻度到右端距离)。
    /// 先判后写, 全字段参与: 条/刻度逐帧变化, 漏比任一即冻结
    pub fn update(&mut self, maneuver_index: f64, len: i32, tick_scale: TickScale) -> bool {
        let changed = self.maneuver_index != maneuver_index
            || self.maneuver_index_len != len
            || self.tick_scale != tick_scale;
        self.maneuver_index = maneuver_index;
        self.maneuver_index_len = len;
        self.tick_scale = tick_scale;
        changed
    }

    /// drawLineMark: 列 x+rightDraw-len, 行
    /// baseY+halfLine .. baseY+halfLine+2*lineWidth 的 1px 竖刻度。
    /// // Java 未 setColor/setStroke — 承袭 g2d 遗留状态。生产调用链
    /// 前置 super.draw → __drawStringShade 尾部 setColor(主文字色) +
    /// setStroke(BasicStroke(1,ROUND,ROUND)),
    /// 故刻度 = 主文字色 1px 线; showGLoad=false 时 Java 承袭更早组件状态
    /// (未钉死), Rust 统一取主文字色为规范语义。
    #[allow(clippy::too_many_arguments)] // 对齐 Java drawLineMark(g,x,y,len) + 展开的行内几何参数
    fn draw_line_mark(
        cv: &mut PixCanvas,
        x: i32,
        base_y: i32,
        right_draw: i32,
        half_line: i32,
        line_width: i32,
        len: i32,
        color: [u8; 4],
    ) {
        // y+halfLine+lineWidth+lineWidth → y+halfLine-lineWidth+lineWidth
        // (后者 -lineWidth+lineWidth 相消 = halfLine), 端点含 1px 线 = 精确像素盒
        // (Java drawLine 端点序无关, fillRect 盒需取 top = min)
        let ya = base_y + half_line;
        let yb = base_y + half_line + line_width + line_width;
        cv.fill_rect(x + right_draw - len, ya, 1, yb - ya + 1, color);
    }

    /// draw。图层序: 刻度线, 最后 thick 影线 + thin 主线。
    pub fn draw(&self, cv: &mut PixCanvas, x: i32, y: i32, font: &LoadedFont, aa: bool) {
        // 基线换算 (刻度/条线相对 Baseline 定位)
        let ascent = font.metrics().ascent;
        let base_y = y + ascent;

        // 刻度颜色 = 主文字色 (见 draw_line_mark 的 PORT 注)
        let mark_color = if self.is_warning {
            colors().warning
        } else {
            colors().num
        };

        // 刻度档 0 恒画 (len10), 其余 index ≥ 前档阈值逐级点亮
        // (点亮判定与档位表收敛在 TickScale::lit_lens, 绘制序 = 档位升序)
        for len in self.tick_scale.lit_lens(self.maneuver_index) {
            Self::draw_line_mark(
                cv,
                x,
                base_y,
                self.right_draw,
                self.half_line,
                self.line_width,
                len,
                mark_color,
            );
        }

        // 条线: newX = x+rightDraw, newY = baseY+halfLine,
        // y = newY+lineWidth; thick(shade) 先画, thin(colorNum) 后画 (双层描边)
        let new_x = x + self.right_draw;
        let line_y = base_y + self.half_line + self.line_width;
        cv.draw_line(
            new_x,
            line_y,
            new_x - self.maneuver_index_len,
            line_y,
            self.stroke_thick_w,
            colors().shade_shape,
            aa,
        );
        cv.draw_line(
            new_x,
            line_y,
            new_x - self.maneuver_index_len,
            line_y,
            self.stroke_thin_w,
            colors().num,
            aa,
        );
    }

    /// getPreferredSize: rightDraw+5 (条右端占位, 布局稳定)。
    pub fn preferred_size(&self) -> (i32, i32) {
        (self.right_draw + 5, self.height)
    }
}

