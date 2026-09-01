//! rows: HUD 文本行组件族 C 类语义复刻 (MiniHUD 左侧数据列的 5 行)
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | HUDTextRow | ui/component/row/HUDTextRow.java | 主文本行: 基线 = y+ascent, 警告色/常态色, 模板锁宽 |
//! | HUDAkbRow | ui/component/row/HUDAkbRow.java | 速度行: 左主文字 + 右 AoA 横条(drawHRect)与 α 小字 |
//! | HUDEnergyRow | ui/component/row/HUDEnergyRow.java | 高度行: 左主文字 + 右能量小字 (同基线) |
//! | HUDFlapsRow | ui/component/row/HUDFlapsRow.java | 襟翼/起落架状态行 (纯数据映射, 无自绘; Java 前代组件, 生产 Row2 已被 HUDMechanizationRow 取代, 保真保留) |
//! | HUDMechanizationRow | ui/component/row/HUDMechanizationRow.java | Row 2 生产组件: 襟翼/减速板/起落架三段拆分, 模板占位推进 curX, 独立三开关 |
//! | HUDManeuverRow | ui/component/row/HUDManeuverRow.java | G 行: 左主文字 + 右机动指数条(thick 影线/thin 主线)与刻度 |
//!
//! 绘制目标 = render2d::PixCanvas; Java extends HUDTextRow 统一映射为组合
//! (`base: HUDTextRow` 字段, PORTING.md §1 禁止造继承); 颜色/坐标公式逐项对照
//! Java paint 逻辑 (关键处 // PORT: 注明行号)。
//!
//! // PORT: Java HUDRow 接口 (HUDRow.java) 的 getPreferredSize 默认 (200, getHeight)
//! 由 preferred_size 实现覆盖, 不单独建 trait —— Rust 侧该接口无第二实现需求。

use crate::global_colors::colors;
use vm_core::hud_data::HUDData;

use crate::font::LoadedFont;

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
    cv.draw_text(font, x + 1, y + 1, s, colors().shade_shape, aa);
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
        ring(cv, x, y, width - 1, height - 1, colors().shade_shape);
        cv.fill_rect(
            x + borderwidth,
            y + borderwidth,
            width - 2 * borderwidth,
            height - 2 * borderwidth,
            c,
        );
    } else {
        // PORT: UIBaseElements.java:106-109 负宽分支: 环自 x+width 起, 填充同步翻转
        ring(cv, x + width, y, -width - 1, height - 1, colors().shade_shape);
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
            colors().warning
        } else {
            colors().num
        };
        text_shaded(cv, font, x, base_y, &self.text, c, aa);
    }

    /// Java:66-83 getPreferredSize: 模板优先测量 (布局防抖), 空文本宽 0。
    /// 返回 (w, h) 对应 java.awt.Dimension。
    pub fn preferred_size(&self, font: &LoadedFont) -> (i32, i32) {
        // PORT: Java:67 w=200 起始, 但非空测量路径必覆盖 (getStringWidth 空串=0)
        let text_to_measure: &str = match &self.template {
            Some(t) if !t.is_empty() => t,
            _ => &self.text,
        };
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
            aoa_length: 100,
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
                colors().num,
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

/// 已改用 HUDMechanizationRow — 本类保真保留, 见下方同文件邻居)。
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
// HUDMechanizationRow (襟翼/减速板/起落架三段拆分行)
// ---------------------------------------------------------------------------

/// Row 2 组件级拆分：襟翼/可变翼 + 减速板 + 起落架 (HUDMechanizationRow.java:12)。
/// 三个子组件各有一个独立的可见性开关 (Java javadoc 原文)。
pub struct HUDMechanizationRow {
    /// Java extends HUDTextRow → 组合基座。draw 全覆写 (base.text 不参与渲染),
    /// base.is_warning 参与三段取色; setStyle/模板锁宽复用基座。
    pub base: HUDTextRow,
    /// 组件级可见性开关：襟翼/可变翼 (Java:15)
    pub show_flaps: bool,
    /// 组件级可见性开关：减速板 (Java:17)
    pub show_airbrake: bool,
    /// 组件级可见性开关：起落架 (Java:19)
    pub show_gear: bool,
    /// 三段数据串 (Java:21-23 构造置 "")
    pub flaps_wing_str: String,
    pub airbrake_str: String,
    pub gear_str: String,
    /// 各子组件模板字符串（用于宽度估算）(Java 注释原文; 默认 W100/BRK/GEA)
    pub flaps_template: String,
    pub airbrake_template: String,
    pub gear_template: String,
}

/// Java:52-60 / 75-80 共用的三段切分: 0..4 / 4..7 / 7..10 各自 trim。
/// // PORT: Java substring + length()>=10 按 UTF-16 码元; 输入域为
/// HUDCalculator 的 mechanization 格式串 (纯 ASCII: F/W 前缀+数字+空格+BRK/GEA),
/// 字节索引与 UTF-16 索引等价 (§2.1)。Java trim() 删两端 <=U+0020, Rust trim()
/// 删 Unicode 空白 — ASCII 域内等价。
fn split_trim3(text: &str) -> Option<(String, String, String)> {
    let b = text.as_bytes();
    if b.len() < 10 {
        return None;
    }
    let seg = |r: std::ops::Range<usize>| -> String {
        let bytes = &b[r];
        // PORT: ASCII 域论证下 from_utf8 恒成功; debug_assert 让域漂移 (切分点落在
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

/// Java getStringWidth(template + " ", font) (HUDCalculator.java:337-345, 内部
/// FontMetrics.stringWidth — javadoc 明示串 advance 不必等于各字符 advance 之和):
/// 拼接串宽按 模板宽 + 空格 advance 拆分。Rust 侧 font.measure 逐字符求和
/// (font.rs charsWidth 口径), 拆分在 Rust 内部严格恒等; Java 侧等价性非规范
/// 保证, 依据 = JDK8 无 layout 属性字体 stringWidth 的逐字符累加实现语义,
/// 经 Java 8 oracle 实测 (1.8.0_342, 6 字号 × 6 段串 ALL-EQUAL) + 555×270
/// 整帧对拍右缘 dx=0 背书 (换字体/字号理论可差 1px)。免 draw 路径堆分配
/// (Java 原码每帧拼新串 — Rust 以拆分复刻); 空模板段宽 0 (Java isEmpty 分支)。
fn seg_width(font: &LoadedFont, template: &str) -> i32 {
    if template.is_empty() {
        0
    } else {
        font.measure(template) + font.char_width(' ')
    }
}

impl HUDMechanizationRow {
    /// Java:30-32 构造 (font 为 draw/preferred 参数, 不入结构体)
    pub fn new(index: i32, height: i32) -> Self {
        HUDMechanizationRow {
            base: HUDTextRow::new(index, height),
            show_flaps: true,
            show_airbrake: true,
            show_gear: true,
            flaps_wing_str: String::new(),
            airbrake_str: String::new(),
            gear_str: String::new(),
            flaps_template: "W100".to_string(),
            airbrake_template: "BRK".to_string(),
            gear_template: "GEA".to_string(),
        }
    }

    /// Java:34-37 可见性开关
    pub fn set_show_flaps(&mut self, v: bool) {
        self.show_flaps = v;
    }
    pub fn set_show_airbrake(&mut self, v: bool) {
        self.show_airbrake = v;
    }
    pub fn set_show_gear(&mut self, v: bool) {
        self.show_gear = v;
    }

    /// Java:40-45 updateParts (游戏模式数据入口)。
    /// super.update("", isWarning) 清空主文字（不使用）。
    pub fn update_parts(
        &mut self,
        flaps_wing_str: &str,
        airbrake_str: &str,
        gear_str: &str,
        is_warning: bool,
    ) -> bool {
        // 先判后写, 全字段参与 (update_changed_covers_all_fields 契约):
        // 三段串逐帧变化而 isWarning 低频, 漏比任一即冻结对应段
        let changed = !self.base.text.is_empty()
            || self.base.is_warning != is_warning
            || self.flaps_wing_str != flaps_wing_str
            || self.airbrake_str != airbrake_str
            || self.gear_str != gear_str;
        self.base.update("", is_warning);
        self.flaps_wing_str.clear();
        self.flaps_wing_str.push_str(flaps_wing_str);
        self.airbrake_str.clear();
        self.airbrake_str.push_str(airbrake_str);
        self.gear_str.clear();
        self.gear_str.push_str(gear_str);
        changed
    }

    /// Java:48-61 update(text, isWarning) 预览模式更新（兼容旧接口）。
    /// 从合并字符串解析回子组件（预览用，格式: "F100BRKGEA" 或 "    BRKGEA"）
    ///。
    pub fn update(&mut self, text: &str, is_warning: bool) -> bool {
        let parts = split_trim3(text);
        let (fw, ab, g) = match &parts {
            Some((a, b, c)) => (a.as_str(), b.as_str(), c.as_str()),
            None => ("", "", ""),
        };
        let changed = self.base.text != text
            || self.base.is_warning != is_warning
            || self.flaps_wing_str != fw
            || self.airbrake_str != ab
            || self.gear_str != g;
        self.base.update(text, is_warning);
        self.flaps_wing_str.clear();
        self.flaps_wing_str.push_str(fw);
        self.airbrake_str.clear();
        self.airbrake_str.push_str(ab);
        self.gear_str.clear();
        self.gear_str.push_str(g);
        changed
    }

    /// Java:63-70 onDataUpdate: 直接写三段串 + isWarning (不走 update ——
    /// base.text 保持不动, 不参与渲染)。
    pub fn on_data_update(&mut self, data: &HUDData) -> bool {
        let changed = self.flaps_wing_str != data.flaps_wing_str
            || self.airbrake_str != data.airbrake_str
            || self.gear_str != data.gear_str
            || self.base.is_warning != data.warn_configuration;
        self.flaps_wing_str.clear();
        self.flaps_wing_str.push_str(&data.flaps_wing_str);
        self.airbrake_str.clear();
        self.airbrake_str.push_str(&data.airbrake_str);
        self.gear_str.clear();
        self.gear_str.push_str(&data.gear_str);
        self.base.is_warning = data.warn_configuration;
        changed
    }

    /// Java:72-81 setTemplate（预览模式），格式同旧 mechanizationStr
    ///。空襟翼段回退 "F100" (Java:77)。
    pub fn set_template(&mut self, template: Option<&str>) {
        self.base.set_template(template);
        if let Some((fw, ab, g)) = template.and_then(split_trim3) {
            self.flaps_template = fw;
            if self.flaps_template.is_empty() {
                self.flaps_template = "F100".to_string();
            }
            self.airbrake_template = ab;
            self.gear_template = g;
        }
    }

    /// Java:83-113 draw。三段沿 curX 依次推进: 模板非空段恒占位 (模板宽 + 尾随
    /// 空格), 数据非空且开关开才绘制文字; 段序 = 图层序, 同基线 baseY, 主字体。
    pub fn draw(&self, cv: &mut PixCanvas, x: i32, y: i32, font: &LoadedFont, aa: bool) {
        // PORT: Java:85-86 ascent = getFontMetrics(font).getAscent(); baseY = y + ascent
        let base_y = y + font.metrics().ascent;
        // PORT: Java:95/104/111 isWarning ? colorWarning : colorNum (三段同色)
        let c = if self.base.is_warning {
            colors().warning
        } else {
            colors().num
        };

        let mut cur_x = x;

        // 襟翼/可变翼：始终占位推进 curX，隐藏时仅不绘制文字
        let flaps_width = seg_width(font, &self.flaps_template);
        if self.show_flaps && !self.flaps_wing_str.is_empty() {
            text_shaded(cv, font, cur_x, base_y, &self.flaps_wing_str, c, aa);
        }
        cur_x += flaps_width;

        // 减速板：始终占位推进 curX，隐藏时仅不绘制文字
        let brk_width = seg_width(font, &self.airbrake_template);
        if self.show_airbrake && !self.airbrake_str.is_empty() {
            text_shaded(cv, font, cur_x, base_y, &self.airbrake_str, c, aa);
        }
        cur_x += brk_width;

        // 起落架：始终占位推进 curX，隐藏时仅不绘制文字 (Java 注释原文;
        // 末段, 其后无推进消费)
        if self.show_gear && !self.gear_str.is_empty() {
            text_shaded(cv, font, cur_x, base_y, &self.gear_str, c, aa);
        }
    }

    /// Java:115-131 getPreferredSize: 三段模板宽之和 (襟翼/减速板含尾随空格,
    /// 起落架无 — Java:128 原样); 隐藏段保留占位符。
    pub fn preferred_size(&self, font: &LoadedFont) -> (i32, i32) {
        let mut w = 0;
        // 始终使用模板估算完整宽度，隐藏的组件保留占位符，保持布局稳定
        w += seg_width(font, &self.flaps_template);
        w += seg_width(font, &self.airbrake_template);
        if !self.gear_template.is_empty() {
            w += font.measure(&self.gear_template);
        }
        (w, self.base.height)
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
            colors().warning
        } else {
            colors().num
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
mod tests;
