//! rows: HUD 文本行组件族 C 类语义复刻 (MiniHUD 左侧数据列的 5 行)
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | HUDTextRow | ui/component/row/HUDTextRow.java | 主文本行: 基线 = y+ascent, 警告色/常态色, 模板锁宽 |
//! | HUDAkbRow | ui/component/row/HUDAkbRow.java | 速度行: 左主文字 + 右 AoA 横条(drawHRect)与 α 小字 |
//! | HUDEnergyRow | ui/component/row/HUDEnergyRow.java | 高度行: 左主文字 + 右能量小字 (同基线) |
//! | HUDFlapsRow | ui/component/row/HUDFlapsRow.java | 襟翼/起落架状态行 (纯数据映射, 无自绘) |
//! | HUDManeuverRow | ui/component/row/HUDManeuverRow.java | G 行: 左主文字 + 右机动指数条(thick 影线/thin 主线)与刻度 |
//!
//! 绘制目标 = render2d::PixCanvas; Java extends HUDTextRow 统一映射为组合
//! (`base: HUDTextRow` 字段, PORTING.md §1 禁止造继承); 颜色/坐标公式逐项对照
//! Java paint 逻辑 (关键处 // PORT: 注明行号)。
//!
//! // PORT: Java HUDRow 接口 (HUDRow.java) 的 getPreferredSize 默认 (200, getHeight)
//! 由 preferred_size 实现覆盖, 不单独建 trait —— Rust 侧该接口无第二实现需求。
//! // PORT: HUDMechanizationRow.java (Row 2 的三段拆分变体) 不在本批任务清单,
//! 未移植; 其视觉 = 三段模板占位推进的文字行, 后续批次处理。

use crate::font::LoadedFont;
use crate::gauges_bars::{COLOR_NUM, COLOR_SHADE_SHAPE, COLOR_WARNING};
use crate::render2d::PixCanvas;

/// Java Color.YELLOW (HUDAkbRow.java:30-31 构造默认)
const COLOR_YELLOW: [u8; 4] = [255, 255, 0, 255];

/// 阴影双遍文本 (UIBaseElements.__drawStringShade drawFontShape=false 分支,
/// Application.java:143 恒 false): 影 (x+1,y+1) colorShadeShape → 本色 (x,y)。
/// 镜像 gauges_bars::text_shaded (同一 Java 出处, 模块私有故本地复刻)。
fn text_shaded(
    cv: &mut PixCanvas,
    font: &LoadedFont,
    x: i32,
    y: i32,
    s: &str,
    c: [u8; 4],
    aa: bool,
) {
    cv.draw_text(font, x + 1, y + 1, s, COLOR_SHADE_SHAPE, aa);
    cv.draw_text(font, x, y, s, c, aa);
}

/// Java Graphics.drawRect(x,y,w,h) + BasicStroke(1): 覆盖 x..x+w × y..y+h
/// (含端点) 的 1px 环。负宽/负高整体不绘制, 零宽/零高退化 1px 线
/// (Java 8 oracle, 镜像 gauges_bars::ring 同一语义)。
fn ring(cv: &mut PixCanvas, x: i32, y: i32, w: i32, h: i32, color: [u8; 4]) {
    if w < 0 || h < 0 {
        return; // PORT: Java drawRect 负宽/负高不绘制 (oracle 0 像素)
    }
    if w == 0 || h == 0 {
        if w == 0 && h > 0 {
            cv.fill_rect(x, y, 1, h + 1, color); // 零宽退化竖线 行 y..y+h
        } else if h == 0 && w > 0 {
            cv.fill_rect(x, y, w + 1, 1, color); // 零高退化横线 列 x..x+w
        }
        return; // 双零无输出
    }
    cv.fill_rect(x, y, w + 1, 1, color); // 上边
    cv.fill_rect(x, y + h, w + 1, 1, color); // 下边
    if h > 1 {
        cv.fill_rect(x, y + 1, 1, h - 1, color); // 左边
        cv.fill_rect(x + w, y + 1, 1, h - 1, color); // 右边
    }
}

/// UIBaseElements.drawHRect (UIBaseElements.java:97-112): shade 1px 外框环 +
/// 内缩 1px 填充条。width<0 时框/条翻转到起点右侧 (Java 原样分支)。
/// borderwidth 调用点恒 1 (HUDAkbRow.java:91), 参数保留对齐 Java 签名。
fn draw_h_rect(
    cv: &mut PixCanvas,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    borderwidth: i32,
    c: [u8; 4],
) {
    if width >= 0 {
        // PORT: UIBaseElements.java:102-105 drawRect(x,y,width-1,height-1) 环 +
        // fillRect(x+bw, y+bw, width-2*bw, height-2*bw) 内芯
        ring(cv, x, y, width - 1, height - 1, COLOR_SHADE_SHAPE);
        cv.fill_rect(
            x + borderwidth,
            y + borderwidth,
            width - 2 * borderwidth,
            height - 2 * borderwidth,
            c,
        );
    } else {
        // PORT: UIBaseElements.java:106-109 负宽分支: 环自 x+width 起, 填充同步翻转
        ring(cv, x + width, y, -width - 1, height - 1, COLOR_SHADE_SHAPE);
        cv.fill_rect(
            x + borderwidth + width,
            y + borderwidth,
            -width - 2 * borderwidth,
            height - 2 * borderwidth,
            c,
        );
    }
}

// ---------------------------------------------------------------------------
// HUDTextRow (族基类 → 组合基座)
// ---------------------------------------------------------------------------

/// 简单文本行 (HUDTextRow.java:10)。上左角 (x,y) 入参, 内部换算基线。
pub struct HUDTextRow {
    /// 行号 (HUDTextRow.java:14, 调试/getId 用)
    pub index: i32,
    /// 主文字 (Java protected text, 构造置 "")
    pub text: String,
    /// 行高 (Java protected height)
    pub height: i32,
    /// 模板文字锁宽 (Java templateText, null=未设)
    pub template: Option<String>,
    /// 警告态 → colorWarning, 否则 colorNum (HUDTextRow.java:46-50)
    pub is_warning: bool,
    /// 组件可见性 (AbstractHUDComponent.visible, 布局引擎门控 — draw 本身不检查,
    /// 对齐 ModernHUDLayoutEngine.java:160 的调用侧检查)
    pub visible: bool,
}

impl HUDTextRow {
    /// Java:18-23 构造 (font 在 Rust 侧为 draw 参数, 不入结构体)
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

    /// Java:31-33 getId
    pub fn id(&self) -> String {
        format!("row.{}", self.index)
    }

    /// Java:54-56 getHeight
    pub fn get_height(&self) -> i32 {
        self.height
    }

    /// Java:25-28 setStyle (仅 height; font 为 draw 参数)
    pub fn set_style(&mut self, height: i32) {
        self.height = height;
    }

    /// Java:35-38 update(text, isWarning)。返回内容是否变化 (组装侧脏检查用,
    /// Java 返回 void —— 行为等价, 附加元数据)。
    pub fn update(&mut self, text: &str, is_warning: bool) -> bool {
        let changed = self.text != text || self.is_warning != is_warning;
        self.text.clear();
        self.text.push_str(text);
        self.is_warning = is_warning;
        changed
    }

    /// Java:61-63 setTemplate (null 语义由 None 承载)
    pub fn set_template(&mut self, template: Option<&str>) {
        self.template = template.map(|s| s.to_string());
    }

    /// Java:41-51 draw: Top-Left y → Baseline y 换算后阴影双遍文本。
    /// 警告 → colorWarning, 常态 → colorNum。
    pub fn draw(&self, cv: &mut PixCanvas, x: i32, y: i32, font: &LoadedFont, aa: bool) {
        // PORT: Java:42-44 ascent = getFontMetrics(font).getAscent(); baseY = y + ascent
        let ascent = font.metrics().ascent;
        let base_y = y + ascent;
        let c = if self.is_warning {
            COLOR_WARNING
        } else {
            COLOR_NUM
        };
        text_shaded(cv, font, x, base_y, &self.text, c, aa);
    }

    /// Java:66-83 getPreferredSize: 模板优先测量 (布局防抖), 空文本宽 0。
    /// 返回 (w, h) 对应 java.awt.Dimension。
    pub fn preferred_size(&self, font: &LoadedFont) -> (i32, i32) {
        // PORT: Java:67 w=200 起始, 但非空测量路径必覆盖 (getStringWidth 空串=0)
        let text_to_measure: &str = match &self.template {
            // Java:69 templateText != null && !isEmpty() ? templateText : text
            Some(t) if !t.is_empty() => t,
            _ => &self.text,
        };
        // Java:71 textToMeasure != null (恒真, 构造置 "") → 无条件测量
        let w = font.measure(text_to_measure);
        (w, self.height)
    }
}

// ---------------------------------------------------------------------------
// HUDAkbRow (速度 + AoA 指示)
// ---------------------------------------------------------------------------

/// Row 0: 速度文字 + 攻角横条 (HUDAkbRow.java:9)。
pub struct HUDAkbRow {
    /// Java extends HUDTextRow → 组合基座
    pub base: HUDTextRow,
    /// AoA 读数文字 (小字号)
    pub aoa_text: String,
    /// AoA 条有效长度像素 (aoaRatio × aoaLength, 钳到 rightDraw)
    pub aoa_y: i32,
    /// 右侧绘制基准 X 偏移 (α 文字左缘 = x + rightDraw)
    pub right_draw: i32,
    pub line_width: i32,
    /// Java:34 aoaLength 默认 100 (setStyle 注入生产值)
    pub aoa_length: i32,
    /// α 文字色 (Java aoaColor, 构造默认 YELLOW)
    pub aoa_color: [u8; 4],
    /// AoA 条填充色 (Java aoaBarColor, 构造默认 YELLOW)
    pub aoa_bar_color: [u8; 4],
    /// AoA 文字模板 (宽度估算用)
    pub aoa_template: Option<String>,
    /// 组件级可见性开关: 速度文字 (HUDAkbRow.java:20)
    pub show_speed: bool,
    /// 组件级可见性开关: 攻角指示器 (HUDAkbRow.java:22)
    pub show_aoa: bool,
}

impl HUDAkbRow {
    /// Java:24-32 构造 (fonts 为 draw 参数)
    pub fn new(index: i32, height: i32, right_draw: i32, line_width: i32) -> Self {
        HUDAkbRow {
            base: HUDTextRow::new(index, height),
            aoa_text: String::new(),
            aoa_y: 0,
            right_draw,
            line_width,
            aoa_length: 100, // Java:34 默认
            aoa_color: COLOR_YELLOW,
            aoa_bar_color: COLOR_YELLOW,
            aoa_template: None,
            show_speed: true,
            show_aoa: true,
        }
    }

    /// Java:47-53 setStyle (font/height 走 base, 此处为 AoA 专属几何)
    pub fn set_style(&mut self, right_draw: i32, line_width: i32, aoa_length: i32) {
        self.right_draw = right_draw;
        self.line_width = line_width;
        self.aoa_length = aoa_length;
    }

    /// Java:38-40 可见性开关
    pub fn set_show_speed(&mut self, v: bool) {
        self.show_speed = v;
    }
    pub fn set_show_aoa(&mut self, v: bool) {
        self.show_aoa = v;
    }

    /// Java:42-45 setTemplate(main, aoa)
    pub fn set_template(&mut self, main: Option<&str>, aoa: Option<&str>) {
        self.base.set_template(main);
        self.aoa_template = aoa.map(|s| s.to_string());
    }

    /// Java:56-73 onDataUpdate 的条长计算段 (69-72):
    /// aoaY = (int)(aoaRatio * aoaLength), 钳到 rightDraw。
    /// // PORT: Java double→int 强转 (JLS 5.1.3) = NaN→0 + 超范围饱和到
    /// MIN/MAX, 与 Rust as i32 语义完全一致 — 两语言无差异 (§2.2 的截断/
    /// 回绕差异仅适用于 long→int 整数窄化, 不适用本处浮点转换)
    pub fn set_aoa_from_ratio(&mut self, aoa_ratio: f64) {
        self.aoa_y = (aoa_ratio * self.aoa_length as f64) as i32;
        if self.aoa_y > self.right_draw {
            self.aoa_y = self.right_draw;
        }
    }

    /// Java:75-81 手动 update (预览模式路径; 游戏模式数据映射见 set_aoa_from_ratio)
    #[allow(clippy::too_many_arguments)] // 对齐 Java update(text,isWarning,aoaText,aoaY,aoaColor,aoaBarColor)
    pub fn update(
        &mut self,
        text: &str,
        is_warning: bool,
        aoa_text: &str,
        aoa_y: i32,
        aoa_color: [u8; 4],
        aoa_bar_color: [u8; 4],
    ) -> bool {
        // 先判后写 (基座 update 内部同理)
        let changed = self.base.text != text
            || self.base.is_warning != is_warning
            || self.aoa_text != aoa_text
            || self.aoa_y != aoa_y
            || self.aoa_color != aoa_color
            || self.aoa_bar_color != aoa_bar_color;
        self.base.update(text, is_warning);
        self.aoa_text.clear();
        self.aoa_text.push_str(aoa_text);
        self.aoa_y = aoa_y;
        self.aoa_color = aoa_color;
        self.aoa_bar_color = aoa_bar_color;
        changed
    }

    /// Java:84-99 draw。图层序: AoA 条+文字先, 速度主文字后 (重叠时主文字在上)。
    pub fn draw(
        &self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        font: &LoadedFont,
        small_font: &LoadedFont,
        aa: bool,
    ) {
        // PORT: Java:85-87 ascent 取主字体; liney = baseY + 1
        let ascent = font.metrics().ascent;
        let base_y = y + ascent;
        let liney = base_y + 1;

        if self.show_aoa {
            // PORT: Java:91 drawHRect(x + (rightDraw - aoaY), liney, aoaY, lineWidth+3, 1, aoaBarColor)
            draw_h_rect(
                cv,
                x + (self.right_draw - self.aoa_y),
                liney,
                self.aoa_y,
                self.line_width + 3,
                1,
                self.aoa_bar_color,
            );
            // PORT: Java:92 α 文字基线 liney - 1, 小字号
            text_shaded(
                cv,
                small_font,
                x + self.right_draw,
                liney - 1,
                &self.aoa_text,
                self.aoa_color,
                aa,
            );
        }

        if self.show_speed {
            self.base.draw(cv, x, y, font, aa);
        }
    }

    /// Java:102-112 getPreferredSize: 主文字宽与 rightDraw+α宽取大
    /// (隐藏组件保留占位, 布局稳定)。
    pub fn preferred_size(&self, font: &LoadedFont, small_font: &LoadedFont) -> (i32, i32) {
        let mut w = self.base.preferred_size(font).0;
        // PORT: Java:106 aoaTemplate != null ? aoaTemplate : aoaText (无空串检查)
        let measure_aoa: &str = self.aoa_template.as_deref().unwrap_or(&self.aoa_text);
        let extra_w = self.right_draw + small_font.measure(measure_aoa);
        if extra_w > w {
            w = extra_w;
        }
        (w, self.base.height)
    }
}

// ---------------------------------------------------------------------------
// HUDEnergyRow (高度 + 能量)
// ---------------------------------------------------------------------------

/// Row 1: 高度文字 + 右侧能量读数 (HUDEnergyRow.java:8)。
pub struct HUDEnergyRow {
    pub base: HUDTextRow,
    /// 能量读数 (小字号, colorNum)
    pub energy_text: String,
    /// 能量文字左缘 = x + rightDraw
    pub right_draw: i32,
    pub energy_template: Option<String>,
    /// 组件级可见性开关: 高度文字 (HUDEnergyRow.java:15)
    pub show_altitude: bool,
    /// 组件级可见性开关: 能量读数 (HUDEnergyRow.java:17)
    pub show_energy: bool,
}

impl HUDEnergyRow {
    /// Java:19-24 构造
    pub fn new(index: i32, height: i32, right_draw: i32) -> Self {
        HUDEnergyRow {
            base: HUDTextRow::new(index, height),
            energy_text: String::new(),
            right_draw,
            energy_template: None,
            show_altitude: true,
            show_energy: true,
        }
    }

    /// Java:37-41 setStyle
    pub fn set_style(&mut self, right_draw: i32) {
        self.right_draw = right_draw;
    }

    /// Java:29-30 可见性开关
    pub fn set_show_altitude(&mut self, v: bool) {
        self.show_altitude = v;
    }
    pub fn set_show_energy(&mut self, v: bool) {
        self.show_energy = v;
    }

    /// Java:32-35 setTemplate(main, energy)
    pub fn set_template(&mut self, main: Option<&str>, energy: Option<&str>) {
        self.base.set_template(main);
        self.energy_template = energy.map(|s| s.to_string());
    }

    /// Java:56-59 update (预览/手动路径; 游戏模式 = altStr/warnAltitude/energyStr 映射)
    pub fn update(&mut self, text: &str, is_warning: bool, energy_text: &str) -> bool {
        // 先判后写, 全字段参与 (与 HUDAkbRow::update 同口径): 能量逐帧变化而
        // 高度文字稳定, 漏比 energy_text 会让按返回值门控重绘的组装侧冻结能量读数
        let changed = self.base.text != text
            || self.base.is_warning != is_warning
            || self.energy_text != energy_text;
        self.base.update(text, is_warning);
        self.energy_text.clear();
        self.energy_text.push_str(energy_text);
        changed
    }

    /// Java:62-75 draw。图层序: 能量读数先, 高度主文字后。
    /// 能量色恒 colorNum (Java:55 注释: 已统一, 不再传色)。
    pub fn draw(
        &self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        font: &LoadedFont,
        small_font: &LoadedFont,
        aa: bool,
    ) {
        // PORT: Java:63-64 ascent 取主字体, 能量文字与主文字同基线 baseY
        let ascent = font.metrics().ascent;
        let base_y = y + ascent;

        if self.show_energy {
            // PORT: Java:68 __drawStringShade(x + rightDraw, baseY, 1, energyText, smallFont, colorNum)
            text_shaded(
                cv,
                small_font,
                x + self.right_draw,
                base_y,
                &self.energy_text,
                COLOR_NUM,
                aa,
            );
        }

        if self.show_altitude {
            self.base.draw(cv, x, y, font, aa);
        }
    }

    /// Java:78-88 getPreferredSize: 主文字宽与 rightDraw+能量宽取大。
    pub fn preferred_size(&self, font: &LoadedFont, small_font: &LoadedFont) -> (i32, i32) {
        let mut w = self.base.preferred_size(font).0;
        // PORT: Java:82 energyTemplate != null ? energyTemplate : energyText
        let measure_en: &str = self.energy_template.as_deref().unwrap_or(&self.energy_text);
        let extra_w = self.right_draw + small_font.measure(measure_en);
        if extra_w > w {
            w = extra_w;
        }
        (w, self.base.height)
    }
}

// ---------------------------------------------------------------------------
// HUDFlapsRow (襟翼/起落架状态行)
// ---------------------------------------------------------------------------

/// Row 2: 襟翼/减速板/起落架状态行 (HUDFlapsRow.java:8)。
/// 纯数据映射组件 — 无自绘, 全部视觉 = 基类文本行
/// (onDataUpdate: mechanizationStr + warnConfiguration → update)。
pub struct HUDFlapsRow {
    pub base: HUDTextRow,
}

impl HUDFlapsRow {
    /// Java:10-13 构造
    pub fn new(index: i32, height: i32) -> Self {
        HUDFlapsRow {
            base: HUDTextRow::new(index, height),
        }
    }

    /// Java:15-19 onDataUpdate → 基类 update
    pub fn update(&mut self, mechanization_str: &str, warn_configuration: bool) -> bool {
        self.base.update(mechanization_str, warn_configuration)
    }

    /// 基类 draw 透传
    pub fn draw(&self, cv: &mut PixCanvas, x: i32, y: i32, font: &LoadedFont, aa: bool) {
        self.base.draw(cv, x, y, font, aa);
    }

    /// 基类 preferred_size 透传
    pub fn preferred_size(&self, font: &LoadedFont) -> (i32, i32) {
        self.base.preferred_size(font)
    }
}

// ---------------------------------------------------------------------------
// HUDManeuverRow (G 值 + 机动指数条)
// ---------------------------------------------------------------------------

/// Row 4: G 力文字 + 机动指数条 (HUDManeuverRow.java:9)。
/// Java 的 strokeThick/strokeThin (BasicStroke, CAP_ROUND+JOIN_ROUND,
/// MinimalHUDContext.java:147-148 造: 宽 halfLine+2 / halfLine) 在 Rust 侧
/// 仅宽度可变 → 存 f32 宽度, 线型由 PixCanvas::draw_line (Round) 固定。
pub struct HUDManeuverRow {
    pub base: HUDTextRow,
    pub right_draw: i32,
    pub half_line: i32,
    pub line_width: i32,
    /// 机动指数 0..0.5+ (刻度点亮阈值 0.1~0.4)
    pub maneuver_index: f64,
    /// 当前值条长 (右端固定 x+rightDraw, 向左延展)
    pub maneuver_index_len: i32,
    pub maneuver_index_len10: i32,
    pub maneuver_index_len20: i32,
    pub maneuver_index_len30: i32,
    pub maneuver_index_len40: i32,
    pub maneuver_index_len50: i32,
    /// strokeThick 宽 (影线)
    pub stroke_thick_w: f32,
    /// strokeThin 宽 (主线)
    pub stroke_thin_w: f32,
    /// 组件级可见性开关: G 力文字 (HUDManeuverRow.java:17)
    pub show_g_load: bool,
    /// 组件级可见性开关: 机动条 (HUDManeuverRow.java:19)
    pub show_maneuver_bar: bool,
}

impl HUDManeuverRow {
    /// Java:34-42 构造 (strokes 以宽度入参, cap/join 恒 ROUND)
    #[allow(clippy::too_many_arguments)] // 对齐 Java 构造 8 参
    pub fn new(
        index: i32,
        height: i32,
        right_draw: i32,
        half_line: i32,
        line_width: i32,
        stroke_thick_w: f32,
        stroke_thin_w: f32,
    ) -> Self {
        HUDManeuverRow {
            base: HUDTextRow::new(index, height),
            right_draw,
            half_line,
            line_width,
            maneuver_index: 0.0,
            maneuver_index_len: 0,
            maneuver_index_len10: 0,
            maneuver_index_len20: 0,
            maneuver_index_len30: 0,
            maneuver_index_len40: 0,
            maneuver_index_len50: 0,
            stroke_thick_w,
            stroke_thin_w,
            show_g_load: true,
            show_maneuver_bar: true,
        }
    }

    /// Java:44-52 setStyle
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
        self.base.set_style(height);
        self.right_draw = right_draw;
        self.half_line = half_line;
        self.line_width = line_width;
        self.stroke_thick_w = stroke_thick_w;
        self.stroke_thin_w = stroke_thin_w;
    }

    /// Java:55-56 可见性开关
    pub fn set_show_g_load(&mut self, v: bool) {
        self.show_g_load = v;
    }
    pub fn set_show_maneuver_bar(&mut self, v: bool) {
        self.show_maneuver_bar = v;
    }

    /// Java:58-68 update (len 族 = 各阈值刻度到右端距离)
    #[allow(clippy::too_many_arguments)] // 对齐 Java update 9 参
    pub fn update(
        &mut self,
        text: &str,
        is_warning: bool,
        maneuver_index: f64,
        len: i32,
        len10: i32,
        len20: i32,
        len30: i32,
        len40: i32,
        len50: i32,
    ) -> bool {
        // 先判后写, 全字段参与 (与 HUDAkbRow::update 同口径): G 文字低频变化而
        // 机动条/刻度逐帧变化, 漏比 index 与 len 族会让按返回值门控重绘的
        // 组装侧几乎永不重绘条与刻度
        let changed = self.base.text != text
            || self.base.is_warning != is_warning
            || self.maneuver_index != maneuver_index
            || self.maneuver_index_len != len
            || self.maneuver_index_len10 != len10
            || self.maneuver_index_len20 != len20
            || self.maneuver_index_len30 != len30
            || self.maneuver_index_len40 != len40
            || self.maneuver_index_len50 != len50;
        self.base.update(text, is_warning);
        self.maneuver_index = maneuver_index;
        self.maneuver_index_len = len;
        self.maneuver_index_len10 = len10;
        self.maneuver_index_len20 = len20;
        self.maneuver_index_len30 = len30;
        self.maneuver_index_len40 = len40;
        self.maneuver_index_len50 = len50;
        changed
    }

    /// Java:117-120 drawLineMark: 列 x+rightDraw-len, 行
    /// baseY+halfLine .. baseY+halfLine+2*lineWidth 的 1px 竖刻度。
    /// // PORT: Java 未 setColor/setStroke — 承袭 g2d 遗留状态。生产调用链
    /// 前置 super.draw → __drawStringShade 尾部 setColor(主文字色) +
    /// setStroke(BasicStroke(1,ROUND,ROUND)) (UIBaseElements.java:23,38-43),
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
        // PORT: Java:118-119 y+halfLine+lineWidth+lineWidth → y+halfLine-lineWidth+lineWidth
        // (后者 -lineWidth+lineWidth 相消 = halfLine), 端点含 1px 线 = 精确像素盒
        // (Java drawLine 端点序无关, fillRect 盒需取 top = min)
        let ya = base_y + half_line;
        let yb = base_y + half_line + line_width + line_width;
        cv.fill_rect(x + right_draw - len, ya, 1, yb - ya + 1, color);
    }

    /// Java:71-115 draw。图层序: G 文字先, 刻度线, 最后 thick 影线 + thin 主线。
    pub fn draw(&self, cv: &mut PixCanvas, x: i32, y: i32, font: &LoadedFont, aa: bool) {
        // PORT: Java:72-75 G 主文字
        if self.show_g_load {
            self.base.draw(cv, x, y, font, aa);
        }
        // PORT: Java:77-80 机动条开关关闭即返回
        if !self.show_maneuver_bar {
            return;
        }

        // PORT: Java:83-85 基线换算 (刻度/条线相对 Baseline 定位)
        let ascent = font.metrics().ascent;
        let base_y = y + ascent;

        // 刻度颜色 = 主文字色 (见 draw_line_mark 的 PORT 注)
        let mark_color = if self.base.is_warning {
            COLOR_WARNING
        } else {
            COLOR_NUM
        };

        // PORT: Java:87-102 len10 恒画; 0.1~0.4 阈值逐级点亮
        Self::draw_line_mark(
            cv,
            x,
            base_y,
            self.right_draw,
            self.half_line,
            self.line_width,
            self.maneuver_index_len10,
            mark_color,
        );
        if self.maneuver_index >= 0.1 {
            Self::draw_line_mark(
                cv,
                x,
                base_y,
                self.right_draw,
                self.half_line,
                self.line_width,
                self.maneuver_index_len20,
                mark_color,
            );
        }
        if self.maneuver_index >= 0.2 {
            Self::draw_line_mark(
                cv,
                x,
                base_y,
                self.right_draw,
                self.half_line,
                self.line_width,
                self.maneuver_index_len30,
                mark_color,
            );
        }
        if self.maneuver_index >= 0.3 {
            Self::draw_line_mark(
                cv,
                x,
                base_y,
                self.right_draw,
                self.half_line,
                self.line_width,
                self.maneuver_index_len40,
                mark_color,
            );
        }
        if self.maneuver_index >= 0.4 {
            Self::draw_line_mark(
                cv,
                x,
                base_y,
                self.right_draw,
                self.half_line,
                self.line_width,
                self.maneuver_index_len50,
                mark_color,
            );
        }

        // PORT: Java:104-114 条线: newX = x+rightDraw, newY = baseY+halfLine,
        // y = newY+lineWidth; thick(shade) 先画, thin(colorNum) 后画 (双层描边)
        let new_x = x + self.right_draw;
        let line_y = base_y + self.half_line + self.line_width;
        cv.draw_line(
            new_x,
            line_y,
            new_x - self.maneuver_index_len,
            line_y,
            self.stroke_thick_w,
            COLOR_SHADE_SHAPE,
            aa,
        );
        cv.draw_line(
            new_x,
            line_y,
            new_x - self.maneuver_index_len,
            line_y,
            self.stroke_thin_w,
            COLOR_NUM,
            aa,
        );
    }

    /// Java:123-128 getPreferredSize: 主文字宽与 rightDraw+5 取大。
    pub fn preferred_size(&self, font: &LoadedFont) -> (i32, i32) {
        let w = self.base.preferred_size(font).0;
        let w = if self.right_draw + 5 > w {
            self.right_draw + 5
        } else {
            w
        };
        (w, self.base.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &str = "../../../fonts/sarasa-mono-sc-bold.ttf";

    fn main_font() -> LoadedFont {
        LoadedFont::new(std::path::Path::new(FONT), 24).unwrap()
    }

    /// MinimalHUDContext.java:152 hudFontSizeSmall = 0.75 × 主字号
    fn small_font() -> LoadedFont {
        LoadedFont::new(std::path::Path::new(FONT), 18).unwrap()
    }

    fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
        let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
        [d[0], d[1], d[2], d[3]]
    }

    fn a(c: &PixCanvas, x: i32, y: i32) -> u8 {
        px(c, x, y)[3]
    }

    /// 区域内是否存在 alpha 达阈值的像素 (文本笔画的稳健判据)
    fn any_alpha_above(c: &PixCanvas, x0: i32, y0: i32, x1: i32, y1: i32, thr: u8) -> bool {
        for y in y0..y1 {
            for x in x0..x1 {
                if a(c, x, y) > thr {
                    return true;
                }
            }
        }
        false
    }

    /// Java2D SrcOver 直通域合成后的 alpha (双层叠色期望值, tiny-skia ±2 LSB)
    fn src_over_a(fg: u8, bg: u8) -> u8 {
        let fa = fg as f32 / 255.0;
        let fda = bg as f32 / 255.0;
        ((fa + fda * (1.0 - fa)) * 255.0 + 0.5) as u8
    }

    fn assert_a_close(actual: u8, expected: u8, what: &str) {
        assert!(
            (actual as i32 - expected as i32).abs() <= 2,
            "{what}: alpha {actual} 期望 ~{expected}"
        );
    }

    /// HUDTextRow: 警告/常态双色 + 基线平移不变性 (draw 输出仅依赖 (x,y) 相对几何)。
    #[test]
    fn text_row_colors_and_translation() {
        let f = main_font();
        let mut row = HUDTextRow::new(2, 30);
        assert_eq!(row.id(), "row.2");
        assert_eq!(row.get_height(), 30);

        // 常态 colorNum (a=240)
        assert!(row.update("875", false));
        let mut cv = PixCanvas::new(120, 60).unwrap();
        row.draw(&mut cv, 10, 10, &f, false);
        assert!(any_alpha_above(&cv, 5, 5, 60, 45, 200), "常态笔画存在");
        assert_eq!(a(&cv, 10, 9), 0, "行顶 y-1 之上无笔画 (小字号上探之外)");

        // 警告 colorWarning (a=100)
        assert!(row.update("875", true));
        let mut cvw = PixCanvas::new(120, 60).unwrap();
        row.draw(&mut cvw, 10, 10, &f, false);
        assert!(any_alpha_above(&cvw, 5, 5, 60, 45, 80), "警告笔画存在");
        assert!(!any_alpha_above(&cvw, 5, 5, 60, 45, 150), "警告色无 240 级像素");

        // 平移不变性: y+10 的输出 = y 输出整体下移 10 行
        let mut cv2 = PixCanvas::new(120, 70).unwrap();
        row.update("875", false);
        row.draw(&mut cv2, 10, 20, &f, false);
        for y in 0..60 {
            for x in 5..60 {
                let p0 = px(&cv, x, y);
                let p1 = px(&cv2, x, y + 10);
                assert_eq!(p0, p1, "平移像素 ({x},{y})");
            }
        }
    }

    /// HUDTextRow.getPreferredSize (HUDTextRow.java:66-83): 模板优先 / 空文本宽 0。
    #[test]
    fn text_row_template_width() {
        let f = main_font();
        let mut row = HUDTextRow::new(0, 30);
        // 无模板空文本: Java getStringWidth("")=0 → w=0 (非默认 200)
        assert_eq!(row.preferred_size(&f), (0, 30));
        row.update("1", false);
        assert_eq!(row.preferred_size(&f), (f.measure("1"), 30));
        row.set_template(Some("88888"));
        assert_eq!(row.preferred_size(&f), (f.measure("88888"), 30));
        assert!(f.measure("88888") > f.measure("1"), "等宽字体前提");
        // 空模板视为未设 (Java:69 !templateText.isEmpty() 条件)
        row.set_template(Some(""));
        assert_eq!(row.preferred_size(&f), (f.measure("1"), 30));
        assert!(!row.update("1", false), "同值 update 无变化");
    }

    /// HUDAkbRow: AoA 条几何 (drawHRect 1px 环 + 内芯) + α 文字右置 + 主文字左置。
    /// rightDraw=60, aoaY=30, lineWidth=2 → 条 (x+30, liney) 宽 30 高 5。
    #[test]
    fn akb_row_bar_and_text_geometry() {
        let f = main_font();
        let sf = small_font();
        let mut row = HUDAkbRow::new(0, 30, 60, 2);
        row.update("500", false, "12", 30, COLOR_YELLOW, COLOR_YELLOW);

        let mut cv = PixCanvas::new(140, 60).unwrap();
        let (x, y) = (10, 5);
        row.draw(&mut cv, x, y, &f, &sf, false);

        let ascent = f.metrics().ascent;
        let liney = y + ascent + 1; // Java:87 liney = baseY + 1
        // 环 (shade): 上边行 liney / 下边行 liney+4, 列 x+30..x+59
        assert_eq!(a(&cv, x + 30, liney), 42, "条环上边 shade");
        assert_eq!(a(&cv, x + 59, liney), 42, "条环上边右端");
        assert_eq!(a(&cv, x + 45, liney + 4), 42, "条环下边 shade");
        // 内芯 (aoaBarColor=不透明黄): 列 x+31..x+58, 行 liney+1..liney+3
        assert_eq!(px(&cv, x + 31, liney + 1), COLOR_YELLOW, "条内芯左上");
        assert_eq!(px(&cv, x + 58, liney + 3), COLOR_YELLOW, "条内芯右下");
        assert_eq!(a(&cv, x + 29, liney + 1), 0, "条左侧无");
        assert_eq!(a(&cv, x + 60, liney + 2), 0, "条右侧无 (α 文字区行不重叠)");
        // α 文字: 基线 liney-1, 左缘 x+60 (数字无降部, 不触条区行)
        assert!(
            any_alpha_above(&cv, x + 60, liney - 20, x + 110, liney, 100),
            "α 文字在 x+rightDraw 右侧"
        );
        // 主文字: colorNum, 位于条左侧区域
        assert!(
            any_alpha_above(&cv, x, y, x + 29, y + 28, 200),
            "速度主文字在左侧"
        );
    }

    /// HUDAkbRow onDataUpdate 条长计算 (Java:69-72): 截断 + rightDraw 钳制。
    #[test]
    fn akb_row_aoa_ratio_clamp() {
        let mut row = HUDAkbRow::new(0, 30, 60, 2);
        row.set_style(60, 2, 100);
        row.set_aoa_from_ratio(0.255);
        assert_eq!(row.aoa_y, 25, "(int)(0.255*100) 截断");
        row.set_aoa_from_ratio(2.0);
        assert_eq!(row.aoa_y, 60, "钳到 rightDraw");
        row.set_aoa_from_ratio(-0.2);
        assert_eq!(row.aoa_y, -20, "负值不钳 (Java 仅上限钳制)");
    }

    /// HUDAkbRow 负宽分支 (UIBaseElements.java:106-109): aoaY<0 时条翻转到
    /// x+rightDraw 右侧 (环自 x+rightDraw 起, 内芯 +1)。
    #[test]
    fn akb_row_negative_aoa_bar_flips_right() {
        let f = main_font();
        let sf = small_font();
        let mut row = HUDAkbRow::new(0, 30, 40, 2);
        row.update("500", false, "", -10, COLOR_YELLOW, COLOR_YELLOW);
        let mut cv = PixCanvas::new(120, 60).unwrap();
        let (x, y) = (10, 5);
        row.draw(&mut cv, x, y, &f, &sf, false);
        let liney = y + f.metrics().ascent + 1;
        // 环: drawRect(x+50-10, liney, 9, 4) → 列 x+40..x+49
        assert_eq!(a(&cv, x + 40, liney), 42, "负宽环左边");
        assert_eq!(a(&cv, x + 49, liney), 42, "负宽环右边");
        // 内芯: fillRect(x+50+1-10, liney+1, 8, 3) → 列 x+41..x+48
        assert_eq!(px(&cv, x + 41, liney + 1), COLOR_YELLOW, "负宽内芯");
        assert_eq!(a(&cv, x + 39, liney + 1), 0, "负宽条左侧无");
    }

    /// HUDAkbRow 组件级开关 (Java:38-40): 双关全闭无输出, 单开互不影响占位。
    #[test]
    fn akb_row_visibility_gates() {
        let f = main_font();
        let sf = small_font();
        let (x, y) = (10, 5);
        let liney = y + f.metrics().ascent + 1;

        // 仅 AoA: 左侧主文字区无笔画, 条存在
        let mut row = HUDAkbRow::new(0, 30, 60, 2);
        row.update("500", false, "12", 30, COLOR_YELLOW, COLOR_YELLOW);
        row.set_show_speed(false);
        let mut cv = PixCanvas::new(140, 60).unwrap();
        row.draw(&mut cv, x, y, &f, &sf, false);
        assert!(!any_alpha_above(&cv, x, y, x + 29, y + 28, 30), "主文字隐藏");
        assert_eq!(px(&cv, x + 31, liney + 1), COLOR_YELLOW, "条仍在");

        // 仅速度: 条与 α 文字均无
        let mut row2 = HUDAkbRow::new(0, 30, 60, 2);
        row2.update("500", false, "12", 30, COLOR_YELLOW, COLOR_YELLOW);
        row2.set_show_aoa(false);
        let mut cv2 = PixCanvas::new(140, 60).unwrap();
        row2.draw(&mut cv2, x, y, &f, &sf, false);
        assert!(
            any_alpha_above(&cv2, x, y, x + 29, y + 28, 200),
            "主文字仍在"
        );
        assert!(
            cv2.pixmap().data()[((liney * cv2.width() + x + 45) * 4) as usize + 3] == 0,
            "条位置无"
        );
        assert!(
            !any_alpha_above(&cv2, x + 60, 0, 140, 60, 30),
            "α 文字区无输出"
        );
    }

    /// HUDAkbRow/HUDEnergyRow.getPreferredSize: 模板 + rightDraw 占位取大,
    /// 隐藏开关不缩宽 (布局稳定, Java:102-112 / 78-88)。
    #[test]
    fn akb_energy_preferred_size_uses_templates() {
        let f = main_font();
        let sf = small_font();
        let mut akb = HUDAkbRow::new(0, 30, 60, 2);
        akb.update("888", false, "9", 30, COLOR_YELLOW, COLOR_YELLOW);
        akb.set_show_aoa(false); // 隐藏仍占位
        akb.set_template(Some("8888"), Some("88888"));
        let (w, h) = akb.preferred_size(&f, &sf);
        assert_eq!(w, (f.measure("8888")).max(60 + sf.measure("88888")));
        assert_eq!(h, 30);

        let mut en = HUDEnergyRow::new(1, 30, 50);
        en.update("8888", false, "9.9");
        en.set_show_energy(false);
        en.set_template(Some("88888"), Some("88.8"));
        let (w, _) = en.preferred_size(&f, &sf);
        assert_eq!(w, (f.measure("88888")).max(50 + sf.measure("88.8")));
        // 能量模板为 None 时回退实测文本 (Java:82)
        en.set_template(Some("88888"), None);
        let (w, _) = en.preferred_size(&f, &sf);
        assert_eq!(w, (f.measure("88888")).max(50 + sf.measure("9.9")));
    }

    /// HUDEnergyRow: 能量小字右置同基线 (Java:62-75), 双开关独立。
    #[test]
    fn energy_row_side_text_and_gates() {
        let f = main_font();
        let sf = small_font();
        let (x, y) = (10, 5);
        let base_y = y + f.metrics().ascent;

        let mut row = HUDEnergyRow::new(1, 30, 50);
        row.update("88", false, "12.3");
        let mut cv = PixCanvas::new(140, 60).unwrap();
        row.draw(&mut cv, x, y, &f, &sf, false);
        assert!(any_alpha_above(&cv, x, y, x + 40, y + 28, 200), "高度主文字");
        assert!(
            any_alpha_above(&cv, x + 50, base_y - 20, x + 110, base_y + 4, 200),
            "能量小字在 x+rightDraw 右侧"
        );

        // 仅高度: 能量区无 (主文字 "88" 墨迹 ≤ x+27, 不入 x+50 起的右区)
        let mut row2 = HUDEnergyRow::new(1, 30, 50);
        row2.update("88", false, "12.3");
        row2.set_show_energy(false);
        let mut cv2 = PixCanvas::new(140, 60).unwrap();
        row2.draw(&mut cv2, x, y, &f, &sf, false);
        assert!(!any_alpha_above(&cv2, x + 45, 0, 140, 60, 30), "能量隐藏");

        // 仅能量: 主文字区无
        let mut row3 = HUDEnergyRow::new(1, 30, 50);
        row3.update("88", false, "12.3");
        row3.set_show_altitude(false);
        let mut cv3 = PixCanvas::new(140, 60).unwrap();
        row3.draw(&mut cv3, x, y, &f, &sf, false);
        assert!(!any_alpha_above(&cv3, x, 0, x + 45, 60, 30), "高度隐藏");
        assert!(
            any_alpha_above(&cv3, x + 50, 0, 140, 60, 200),
            "能量仍在"
        );
    }

    /// HUDFlapsRow: 纯委托 (Java:15-19 mechanizationStr/warnConfiguration 映射)。
    #[test]
    fn flaps_row_delegates_to_text_row() {
        let f = main_font();
        let mut row = HUDFlapsRow::new(2, 30);
        assert_eq!(row.base.id(), "row.2");
        assert!(row.update("F100 BRK GEA", true));
        assert!(!row.update("F100 BRK GEA", true));
        let mut cv = PixCanvas::new(160, 60).unwrap();
        row.draw(&mut cv, 10, 5, &f, false);
        assert!(
            any_alpha_above(&cv, 5, 5, 155, 45, 80),
            "警告态笔画存在 (colorWarning)"
        );
        assert!(!any_alpha_above(&cv, 5, 5, 155, 45, 150), "无常态色像素");
        row.update("F100 BRK GEA", false);
        let mut cv2 = PixCanvas::new(160, 60).unwrap();
        row.draw(&mut cv2, 10, 5, &f, false);
        assert!(
            any_alpha_above(&cv2, 5, 5, 155, 45, 200),
            "常态笔画存在"
        );
    }

    /// HUDManeuverRow 刻度几何: len10 恒画, 0.1~0.4 阈值逐级点亮 (Java:87-102);
    /// 列 = x+rightDraw-len, 行 = baseY+halfLine .. +halfLine+2*lineWidth (1px)。
    #[test]
    fn maneuver_row_tick_thresholds() {
        let f = main_font();
        let (x, y) = (10, 5);
        let (right_draw, half_line, line_width) = (60, 2, 2);
        let base_y = y + f.metrics().ascent;

        let mut row = HUDManeuverRow::new(4, 30, right_draw, half_line, line_width, 4.0, 2.0);
        // showGLoad=false: 排除主文字, 刻度列纯净 (色取主文字色规范语义)
        row.set_show_g_load(false);
        row.update("2.0", false, 0.35, 5, 10, 20, 30, 40, 50);
        let mut cv = PixCanvas::new(100, 60).unwrap();
        row.draw(&mut cv, x, y, &f, false);

        let tick_top = base_y + half_line;
        let tick_bot = base_y + half_line + 2 * line_width;
        // len10 (恒画), len20 (0.35>=0.1), len30 (>=0.2), len40 (>=0.3) 点亮
        for len in [10, 20, 30, 40] {
            let col = x + right_draw - len;
            assert_eq!(a(&cv, col, tick_top), 240, "刻度 len={len} 顶行");
            assert_eq!(a(&cv, col, tick_bot), 240, "刻度 len={len} 底行");
            assert_eq!(a(&cv, col - 1, tick_top + 2), 0, "刻度 len={len} 左邻");
        }
        // len50 (0.35<0.4) 不点亮
        assert_eq!(a(&cv, x + right_draw - 50, tick_top + 2), 0, "len50 未点亮");
        // 刻度行范围外无 (竖刻度 1px 精确盒)
        assert_eq!(a(&cv, x + right_draw - 10, tick_top - 1), 0, "刻度上方无");
        assert_eq!(a(&cv, x + right_draw - 10, tick_bot + 1), 0, "刻度下方无");

        // 阈值边界: index=0.4 → len50 点亮 (>= 含等)
        let mut row2 = HUDManeuverRow::new(4, 30, right_draw, half_line, line_width, 4.0, 2.0);
        row2.set_show_g_load(false);
        row2.update("2.0", false, 0.4, 5, 10, 20, 30, 40, 50);
        let mut cv2 = PixCanvas::new(100, 60).unwrap();
        row2.draw(&mut cv2, x, y, &f, false);
        assert_eq!(a(&cv2, x + right_draw - 50, tick_top + 2), 240, "0.4 含等点亮");
    }

    /// HUDManeuverRow 条线双层描边 (Java:104-114): thick shade 下层 + thin colorNum
    /// 上层, y = baseY+halfLine+lineWidth; 行覆盖 = thick 半径外扩。
    /// halfLine=2/lineWidth=2 → thin(2) 行 baseY+3..4, thick(4) 行 baseY+2..5。
    #[test]
    fn maneuver_row_bar_double_stroke_layers() {
        let f = main_font();
        let (x, y) = (10, 5);
        let (right_draw, half_line, line_width) = (60, 2, 2);
        let base_y = y + f.metrics().ascent;
        let line_y = base_y + half_line + line_width; // newY + lineWidth

        let mut row = HUDManeuverRow::new(4, 30, right_draw, half_line, line_width, 4.0, 2.0);
        row.set_show_g_load(false); // 排除文字, 条区纯净
        row.update("2.0", false, 0.35, 30, 10, 20, 30, 40, 50);
        let mut cv = PixCanvas::new(100, 60).unwrap();
        row.draw(&mut cv, x, y, &f, false);

        // 条横跨 x+30..x+60 (len=30), 采样列 x+58 (条体内, 非刻度列)
        let col = x + 58;
        // thin(宽2, 圆帽) 行 line_y-1..line_y = baseY+3..4: thin 叠 thick
        assert_a_close(a(&cv, col, line_y), src_over_a(240, 42), "主线行 (thin over thick)");
        assert_a_close(a(&cv, col, line_y - 1), src_over_a(240, 42), "主线行上");
        // thick(宽4) 独占行 baseY+2 / baseY+5 (band 边界为整, 像素中心 .5 无歧义)
        assert_eq!(a(&cv, col, line_y - 2), 42, "影线单独行上 (thick only)");
        assert_eq!(a(&cv, col, line_y + 1), 42, "影线单独行下 (thick only)");
        // thick band 外
        assert_eq!(a(&cv, col, line_y - 3), 0, "条上方 2px");
        assert_eq!(a(&cv, col, line_y + 2), 0, "条下方 2px");
        // 条长: 左端 x+30 内侧 (x+32), 条外 (x+26)
        assert_a_close(a(&cv, x + 32, line_y), src_over_a(240, 42), "条左端内侧");
        assert_eq!(a(&cv, x + 26, line_y), 0, "条长之外");
    }

    /// HUDManeuverRow 开关与 preferred_size (Java:123-128):
    /// max(主文字宽, rightDraw+5); 机动条关闭仅剩文字。
    #[test]
    fn maneuver_row_gates_and_preferred_size() {
        let f = main_font();
        let (x, y) = (10, 5);
        let base_y = y + f.metrics().ascent;
        let line_y = base_y + 2 + 2;

        let mut row = HUDManeuverRow::new(4, 30, 60, 2, 2, 4.0, 2.0);
        row.update("2.0", false, 0.35, 30, 10, 20, 30, 40, 50);
        let (w, h) = row.preferred_size(&f);
        assert_eq!(w, (f.measure("2.0")).max(60 + 5));
        assert_eq!(h, 30);

        // 机动条关: 条行无输出, 文字仍在
        let mut cv = PixCanvas::new(100, 60).unwrap();
        row.set_show_maneuver_bar(false);
        row.draw(&mut cv, x, y, &f, false);
        assert_eq!(a(&cv, x + 58, line_y), 0, "条关闭无条线");
        assert_eq!(a(&cv, x + 50, base_y + 4), 0, "条关闭无刻度");
        assert!(any_alpha_above(&cv, x, y, x + 40, y + 28, 200), "G 文字仍在");

        // G 文字关: 仅条 (index=0.25 → len10/20/30 刻度点亮, 列 ≥ x+30 不入左区)
        let mut row2 = HUDManeuverRow::new(4, 30, 60, 2, 2, 4.0, 2.0);
        row2.update("2.0", false, 0.25, 30, 10, 20, 30, 40, 50);
        row2.set_show_g_load(false);
        let mut cv2 = PixCanvas::new(100, 60).unwrap();
        row2.draw(&mut cv2, x, y, &f, false);
        assert!(!any_alpha_above(&cv2, x, y, x + 25, y + 28, 30), "G 文字隐藏");
        assert_a_close(a(&cv2, x + 58, line_y), src_over_a(240, 42), "条仍在");
    }

    /// 脏检查契约回归: update 返回值必须覆盖组件全部可变字段 (Java 原方法
    /// 返回 void, bool 为 Rust 附加的组装侧重绘门控元数据)。HUDEnergyRow 的
    /// energy_text 与 HUDManeuverRow 的 index/len 族均逐帧变化而 base 文字
    /// 稳定, 漏比任一字段即冻结对应读数/条刻度。
    #[test]
    fn update_changed_covers_all_fields() {
        // HUDEnergyRow
        let mut en = HUDEnergyRow::new(1, 30, 50);
        en.update("1000", false, "E100");
        assert!(!en.update("1000", false, "E100"), "全同值无变化");
        assert!(en.update("1000", false, "E200"), "仅能量变化须报 changed");
        assert!(!en.update("1000", false, "E200"), "重复同能量无变化");
        assert!(en.update("1001", false, "E200"), "仅 base 文字变化仍报 changed");
        assert!(en.update("1001", true, "E200"), "仅警告态变化仍报 changed");

        // HUDManeuverRow
        let mut mn = HUDManeuverRow::new(4, 30, 60, 2, 2, 4.0, 2.0);
        mn.update("2.0", false, 0.1, 5, 10, 20, 30, 40, 50);
        assert!(
            !mn.update("2.0", false, 0.1, 5, 10, 20, 30, 40, 50),
            "全同值无变化"
        );
        assert!(mn.update("2.0", false, 0.2, 5, 10, 20, 30, 40, 50), "仅 index 变化");
        assert!(mn.update("2.0", false, 0.2, 6, 10, 20, 30, 40, 50), "仅 len 变化");
        assert!(mn.update("2.0", false, 0.2, 6, 11, 20, 30, 40, 50), "仅 len10 变化");
        assert!(mn.update("2.0", false, 0.2, 6, 11, 21, 30, 40, 50), "仅 len20 变化");
        assert!(mn.update("2.0", false, 0.2, 6, 11, 21, 31, 40, 50), "仅 len30 变化");
        assert!(mn.update("2.0", false, 0.2, 6, 11, 21, 31, 41, 50), "仅 len40 变化");
        assert!(mn.update("2.0", false, 0.2, 6, 11, 21, 31, 41, 51), "仅 len50 变化");
        assert!(mn.update("2.1", false, 0.2, 6, 11, 21, 31, 41, 51), "仅文字变化");
        assert!(mn.update("2.1", true, 0.2, 6, 11, 21, 31, 41, 51), "仅警告态变化");
    }
}
