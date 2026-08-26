//! overlay_list: 列表型 overlay 基座 C 类语义复刻 (依赖 render2d::PixCanvas)
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | [`ZebraList`] | ui/overlay/ZebraListRenderer.java | 斑马纹行渲染: 表头琥珀 #503C00 / 偶 #191919 / 奇 #282828, margin 2,6,2,6 |
//! | [`BaseListOverlay`] | ui/overlay/BaseOverlay.java | 轮询-脏检查-高度自适应状态机 (200ms 默认) |
//!
//! BaseOverlay.run() (BaseOverlay.java:229-261) 的 while(doit)+sleep 线程循环在
//! Rust 侧由窗口层驱动 (每 refresh_interval_ms 调一次 [`BaseListOverlay::tick`]),
//! 本文件保留其**单轮语义**: 可见门控 → dataSupplier 取数 → equals 脏检查 →
//! 高度自适应, 返回是否需要重绘 (调用方再 render 到 PixCanvas)。
//!
//! 行几何模型 (WebLabel + VerticalFlowLayout(0,0), BaseOverlay.java:108-109):
//! - 行高 = WebLabel preferred 高 = margin 上 2 + 下 2 + FontMetrics.getHeight()
//! - 行背景条被纵向流布局拉伸至 dataPanel 全宽 (斑马条满宽), 文本左缩进 6
//! - 文本基线 = 行顶 + 2 + ascent (preferred 高度下 CENTER 对齐零余量,
//!   BasicLabelUI paint 的 textY + fm.getAscent())
//!
//! 合成模型 (Java 8 + WebLaF oracle 实测, dep/weblaf-complete-1.29.jar 离屏 paint):
//! dataPanel (WebPanel opaque) 的背景 fillRect 在 WebPanelUI 链上被画**两遍**
//! (ComponentUI.update 一次 + LafUtils.fillVisibleBackground 一次), 行 label 背景
//! 再叠第三层 — 全部 SrcOver 叠加, 非"不透明组件整块替换"。alpha=180 时 oracle:
//! 间隙 (panel²)=0xE9141414, 行 (label over panel²) alpha=249, 表头 0xF93E3005 /
//! 偶 0xF9181818 / 奇 0xF9222222。draw() 按 [`java2d_src_over`] 预合成最终单色直铺。

use crate::font::LoadedFont;
use crate::render2d::PixCanvas;

// ---------------------------------------------------------------------------
// 调色板与边距 (ZebraListRenderer.java / BaseOverlay.java 直读)
// ---------------------------------------------------------------------------

/// ZebraListRenderer.java:33 表头底色 new Color(80, 60, 0, alpha) — 深琥珀 #503C00
const HEADER_RGB: [u8; 3] = [80, 60, 0];
/// ZebraListRenderer.java:37 偶数行 new Color(25, 25, 25, alpha) — #191919
const ZEBRA_EVEN_RGB: [u8; 3] = [25, 25, 25];
/// ZebraListRenderer.java:39 奇数行 new Color(40, 40, 40, alpha) — #282828
const ZEBRA_ODD_RGB: [u8; 3] = [40, 40, 40];
/// BaseOverlay.java:110 dataPanel 底色 new Color(20, 20, 20, alpha) — #141414,
/// 行未覆盖区域 (行数不足窗口高 / ±2px 高度容差带 / 空列表) 的兜底色
const PANEL_BG_RGB: [u8; 3] = [20, 20, 20];
/// ZebraListRenderer.java:27 label.setForeground(Color.WHITE) — 行文本恒白且不透明
const TEXT_COLOR: [u8; 4] = [255, 255, 255, 255];

/// ZebraListRenderer.java:29 label.setMargin(2, 6, 2, 6) 上下边距 (行高组成部分)
pub const MARGIN_TOP: i32 = 2;
/// ZebraListRenderer.java:29 左边距 (文本 x 缩进)
pub const MARGIN_LEFT: i32 = 6;
/// ZebraListRenderer.java:29 下边距 (行高组成部分)
pub const MARGIN_BOTTOM: i32 = 2;
// PORT: margin 右分量 6 只影响 WebLabel preferred 宽 (行被拉伸至全宽后不可见),
// 不单独立常量; 文本超宽由画布边界裁剪 (Java 组件 clip 等效)

/// 直通 RGBA 组装 (java.awt.Color(r, g, b, a) 字节序)
fn rgba(rgb: [u8; 3], alpha: u8) -> [u8; 4] {
    [rgb[0], rgb[1], rgb[2], alpha]
}

/// Java Math.round(float) = floor(x + 0.5) (PORTING.md §2.3)
fn java_round_f(x: f32) -> i32 {
    (x + 0.5).floor() as i32
}

/// Java2D AlphaComposite.SrcOver 的 8bit 整数路径 (TYPE_INT_ARGB 直通存储):
/// 载入预乘 round(c·a/255) → 合成 o = s + round(d·(255−sa)/255) → 直通存储
/// round(o·255/oa)。WebLaF 双遍背景的 oracle 值由此式逐值复现 (间隙 0xE9141414 /
/// 表头 0xF93E3005 / 偶 0xF9181818 / 奇 0xF9222222, 含可区分色探针
/// panel(200,100,50,180)²+label(25,25,25,180)=(249,74,46,32))
fn java2d_src_over(s: [u8; 4], d: [u8; 4]) -> [u8; 4] {
    let sa = s[3] as u32;
    if sa == 0 {
        return d; // 零 alpha 源 SrcOver = 目标不变 (Java 快路径)
    }
    let da = d[3] as u32;
    let inv = 255 - sa;
    let oa = sa + (da * inv + 127) / 255;
    if oa == 0 {
        return [0, 0, 0, 0];
    }
    let mut out = [0u8; 4];
    for c in 0..3 {
        let sp = (s[c] as u32 * sa + 127) / 255; // 直通 → 预乘 (Java 载入宏)
        let dp = (d[c] as u32 * da + 127) / 255;
        let op = sp + (dp * inv + 127) / 255; // 预乘域 SrcOver
        out[c] = ((op * 255 + oa / 2) / oa) as u8; // 预乘 → 直通 (Java 存储宏)
    }
    out[3] = oa as u8;
    out
}

// ---------------------------------------------------------------------------
// ZebraList (ZebraListRenderer.java)
// ---------------------------------------------------------------------------

/// 斑马纹列表渲染器 (ZebraListRenderer.java:13)。
/// 组件 = struct + draw; 表头判定可插拔 (headerMatcher 谓词)。
pub struct ZebraList {
    /// 表头判定谓词 (ZebraListRenderer.java:16 默认 contains 匹配)
    header_matcher: Box<dyn Fn(&str) -> bool>,
}

/// 默认表头判定 (ZebraListRenderer.java:16): contains("fm器件") || contains("FM文件")
fn default_header_matcher(line: &str) -> bool {
    line.contains("fm器件") || line.contains("FM文件")
}

impl ZebraList {
    /// 默认渲染器 (BaseOverlay.java:49 构造时 new ZebraListRenderer())
    pub fn new() -> Self {
        ZebraList {
            header_matcher: Box::new(default_header_matcher),
        }
    }

    /// setHeaderMatcher (ZebraListRenderer.java:53-58)。
    /// PORT: Java 的 null 检查由 Rust 类型系统免除 (Box<dyn Fn> 无 null 态)
    pub fn set_header_matcher(&mut self, matcher: Box<dyn Fn(&str) -> bool>) {
        self.header_matcher = matcher;
    }

    /// isHeader (ZebraListRenderer.java:48-51)
    pub fn is_header(&self, line: &str) -> bool {
        (self.header_matcher)(line)
    }

    /// 单行底色: 表头不消耗斑马索引 (ZebraListRenderer.java:31-41 —
    /// rowIndex 仅在 else 分支自增, 表头行的出现不打断偶奇交替)
    pub fn row_background(&self, line: &str, zebra_index: i32, alpha: u8) -> [u8; 4] {
        if self.is_header(line) {
            rgba(HEADER_RGB, alpha)
        } else if zebra_index % 2 == 0 {
            rgba(ZEBRA_EVEN_RGB, alpha)
        } else {
            rgba(ZEBRA_ODD_RGB, alpha)
        }
    }

    /// 行高 = MARGIN_TOP + FontMetrics.getHeight() + MARGIN_BOTTOM。
    /// WebLabel preferred 高 = margin + 文本高, 文本高取 fm.getHeight()
    /// (BasicLabelUI.getPreferredSize → SwingUtilities.layoutCompoundLabel)
    pub fn row_height(font: &LoadedFont) -> i32 {
        MARGIN_TOP + font.metrics().height + MARGIN_BOTTOM
    }

    /// 列表 preferred 高 = 行数 × 行高 (dataPanel 的 VerticalFlowLayout(0,0)
    /// vgap=0, BaseOverlay.java:109; adjustPosition 的 getPreferredSize 来源)
    pub fn preferred_height(lines: &[String], font: &LoadedFont) -> i32 {
        lines.len() as i32 * Self::row_height(font)
    }

    /// render() 等效 (ZebraListRenderer.java:19-46): 逐行画 (满宽背景条 + 白字),
    /// 行未覆盖的下方余量铺 dataPanel 底色; 部分可见的末行行条裁到 panel_h
    /// (Java 窗口边界硬裁剪), panel_h 之外的行整体不画。
    /// alpha 与 Java render 的第 4 参一致 (来自 BaseOverlay.alpha, 默认 180)。
    ///
    /// PORT: 合成栈为三层 SrcOver (见模块头注 oracle 实测) — dataPanel 底色连画
    /// 两遍 (透明窗起点), 行 label 背景叠第三层。此处预合成最终单色直铺, 与
    /// Java 内部预乘值逐位一致 (免 tiny-skia 多层叠的 ±1 LSB 漂移);
    /// alpha=255 时合成退化为恒等 (直铺原色)。
    #[allow(clippy::too_many_arguments)] // 签名对齐 Java render(data, panel, font, alpha) + 显式几何
    pub fn draw(
        &mut self,
        cv: &mut PixCanvas,
        x: i32,
        y: i32,
        w: i32,
        panel_h: i32,
        lines: &[String],
        font: &LoadedFont,
        alpha: u8,
        aa: bool,
    ) {
        let row_h = Self::row_height(font);
        let rows_h = lines.len() as i32 * row_h;
        // Java 栈: panel 底色在透明窗上连画两遍 (WebPanelUI 双 fillRect) → 间隙色;
        // 行 = SrcOver(label 底色 over 双叠 panel)
        let panel = rgba(PANEL_BG_RGB, alpha);
        let panel2 = java2d_src_over(panel, java2d_src_over(panel, [0, 0, 0, 0]));
        if panel_h > rows_h {
            cv.fill_rect(x, y + rows_h, w, panel_h - rows_h, panel2);
        }
        // 第一遍: 全部行条 (形状先行, 文本阶段的直通域重构只发生一次)
        let mut zebra_index = 0i32;
        for (i, line) in lines.iter().enumerate() {
            let ry = y + i as i32 * row_h;
            if ry >= y + panel_h {
                break; // 高度被 clamp 到 logicalHeight-40 时的行裁剪
            }
            let bg = self.row_background(line, zebra_index, alpha);
            if !self.is_header(line) {
                zebra_index += 1; // PORT: :41 仅非表头行自增 rowIndex
            }
            // 满宽斑马条 (VerticalFlowLayout 拉伸子件至容器宽)
            cv.fill_rect(x, ry, w, row_h.min(y + panel_h - ry), java2d_src_over(bg, panel2));
        }
        // 第二遍: 全部文本 — 左缩进 6, 基线 = 行顶 + 2 + ascent (头注行几何模型)
        let ascent = font.metrics().ascent;
        for (i, line) in lines.iter().enumerate() {
            let ry = y + i as i32 * row_h;
            if ry >= y + panel_h {
                break;
            }
            cv.draw_text(font, x + MARGIN_LEFT, ry + MARGIN_TOP + ascent, line, TEXT_COLOR, aa);
        }
    }
}

impl Default for ZebraList {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// BaseListOverlay (BaseOverlay.java 状态机部分)
// ---------------------------------------------------------------------------

/// 列表型 overlay 基座 (BaseOverlay.java)。Java extends DraggableOverlay 的
/// 窗口/拖拽部分归平台层, 本结构只承载 run() 循环的状态语义:
/// 脏检查 (lastData)、高度自适应 (height)、可见门控 (visible_now/should_exit)。
pub struct BaseListOverlay {
    /// stop flag (BaseOverlay.java:25 volatile doit; stop() 置 false)
    pub doit: bool,
    /// 行背景 alpha (BaseOverlay.java:33 默认 180; setAlpha 可改)
    pub alpha: u8,
    /// 预览模式 (BaseOverlay.java:35, initPreview 置 true — 免 isVisibleNow 门控)
    pub is_preview: bool,
    /// 窗口宽 (init 公式, BaseOverlay.java:94)
    pub width: i32,
    /// 当前窗口高 (adjustPosition 维护, :282 setSize 目标值)
    pub height: i32,
    /// init 时的 fontSize 字段 (BaseOverlay.java:93, 派生量留档)
    pub font_size: i32,
    /// Application.logicalHeight 快照 (高度 clamp 上限来源, :275)
    pub logical_height: i32,
    /// getRefreshInterval() 默认 200ms (BaseOverlay.java:221-223)
    pub refresh_interval_ms: u64,
    /// isVisibleNow() 默认 true (BaseOverlay.java:225-227; 子类可覆盖为开关态)
    pub visible_now: bool,
    /// shouldExit() 默认 false (BaseOverlay.java:217-219)
    pub should_exit: bool,
    /// run() 的 setVisible 目标态 (不可见分支置 false, 可见分支置 true)
    pub window_visible: bool,
    /// setup_font 后的行高 (ZebraList::row_height 快照)
    row_height: i32,
    /// lastData (BaseOverlay.java:37, 脏检查基准; null = 尚无数据)
    last_data: Option<Vec<String>>,
    /// pluggable renderer (BaseOverlay.java:42, 默认 ZebraListRenderer)
    pub zebra: ZebraList,
}

impl BaseListOverlay {
    /// init 的几何段 (BaseOverlay.java:88-95):
    /// scaleFactor = (float)(logicalHeight / 1440.0 * dpiScale);
    /// fontSize = round(16 * scaleFactor); width = round(defaultFontsize * 36 * scaleFactor);
    /// height = defaultFontsize * 72 (初始字段值, 首次数据到达即被 adjustPosition 接管)。
    pub fn new(logical_height: i32, dpi_scale: f64, default_fontsize: i32) -> Self {
        // PORT: Java :92 双精度算完 (float) 强转 — f64 运算后 as f32
        let scale = ((logical_height as f64 / 1440.0) * dpi_scale) as f32;
        BaseListOverlay {
            doit: true,
            alpha: 180,
            is_preview: false,
            width: java_round_f((default_fontsize * 36) as f32 * scale),
            height: default_fontsize * 72,
            font_size: java_round_f(16.0 * scale),
            logical_height,
            refresh_interval_ms: 200,
            visible_now: true,
            should_exit: false,
            window_visible: false,
            row_height: 0, // setup_font 前无字体度量 (Java setupFont 先于线程启动)
            last_data: None,
            zebra: ZebraList::new(),
        }
    }

    /// setupFont (BaseOverlay.java:169-184) 的度量段:
    /// Java 建 displayFont = new Font(name, PLAIN, 14 + fontSizeAdd), 行高由
    /// WebLabel 按该字体 FontMetrics 推导; Rust 侧调用方创建等价 LoadedFont 传入。
    pub fn setup_font(&mut self, font: &LoadedFont) {
        self.row_height = ZebraList::row_height(font);
    }

    /// setAlpha (BaseOverlay.java:77-79)
    pub fn set_alpha(&mut self, alpha: u8) {
        self.alpha = alpha;
    }

    /// setHeaderMatcher 委托 renderer (BaseOverlay.java:71-75)
    pub fn set_header_matcher(&mut self, matcher: Box<dyn Fn(&str) -> bool>) {
        self.zebra.set_header_matcher(matcher);
    }

    /// stop (BaseOverlay.java:286-288)
    pub fn stop(&mut self) {
        self.doit = false;
    }

    /// run() 的单轮语义 (BaseOverlay.java:230-257, 去 sleep/线程):
    /// 可见 (isPreview || isVisibleNow) → supplier 取数 → equals 脏检查 →
    /// 变化时 adjustPosition 并返回 true (需重绘); 不可见 → 窗口隐藏且不取数。
    pub fn tick<F>(&mut self, supplier: F) -> bool
    where
        F: FnOnce() -> Option<Vec<String>>,
    {
        // PORT: :231-232 while(doit) 循环条件 + shouldExit() break
        if !self.doit || self.should_exit {
            return false;
        }
        if self.is_preview || self.visible_now {
            let mut dirty = false;
            // PORT: :236-237 currentData != null && !currentData.equals(lastData);
            // null 数据不更新 lastData (Java 同 — null 时不进 if)
            if let Some(cur) = supplier() {
                if self.last_data.as_ref() != Some(&cur) {
                    self.last_data = Some(cur);
                    dirty = true;
                }
            }
            // PORT: :242-247 仅不可见时 setVisible(true) — 重复调用会触发 DWM
            // 全量合成导致 DX12 游戏卡顿 (Issue #54), 故置位而非无条件调用
            self.window_visible = true;
            if dirty {
                self.adjust_position(); // :239 updateUI → :267 adjustPosition
            }
            dirty
        } else {
            self.window_visible = false; // PORT: :249 else 分支 setVisible(false)
            false
        }
    }

    /// adjustPosition (BaseOverlay.java:272-284):
    /// preferred = dataPanel preferred 高 (行数 × 行高); 超过 logicalHeight-40 钳制;
    /// 与当前高差 >2px 才 setSize (宽恒定, 位置由 OverlaySettings 管理)。
    pub fn adjust_position(&mut self) {
        let mut preferred =
            self.last_data
                .as_ref()
                .map_or(0, |d| d.len() as i32 * self.row_height);
        let max_h = self.logical_height - 40; // :275 maxHeight
        if preferred > max_h {
            preferred = max_h;
        }
        if (self.height - preferred).abs() > 2 {
            // PORT: :282 setSize(width, preferredHeight) — 仅高度
            self.height = preferred;
        }
    }

    /// updateUI 的渲染段 (BaseOverlay.java:263-269, EDT 上的 renderer.render):
    /// 把 lastData 画到窗口画布 (左上原点, width × height), 空数据只铺 panel 底色。
    pub fn render(&mut self, cv: &mut PixCanvas, font: &LoadedFont, aa: bool) {
        let lines = self.last_data.as_deref().unwrap_or(&[]);
        self.zebra
            .draw(cv, 0, 0, self.width, self.height, lines, font, self.alpha, aa);
    }
}

// ---------------------------------------------------------------------------
// 测试: 几何 (行高/斑马色/边距) + 状态机 (脏检查/门控/高度自适应)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// WebLabel 是 Font.PLAIN → regular 字重 (renderers.rs unit 字体同源)
    const FONT: &str = "../../../fonts/sarasa-mono-sc-regular.ttf";

    fn font(size: i32) -> LoadedFont {
        LoadedFont::new(std::path::Path::new(FONT), size).unwrap()
    }

    fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
        let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
        [d[0], d[1], d[2], d[3]]
    }

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// 表头不消耗斑马索引: 偶/奇交替跨表头延续 (ZebraListRenderer.java:31-41)
    #[test]
    fn zebra_row_colors_header_skips_index() {
        let z = ZebraList::new();
        // 默认表头判定 contains (ZebraListRenderer.java:16)
        assert!(z.is_header("------fm器件: 机翼"));
        assert!(z.is_header("FM文件: spitfire"));
        assert!(!z.is_header("速度 600"));
        assert_eq!(z.row_background("FM文件: x", 0, 255), [80, 60, 0, 255], "表头 #503C00");
        assert_eq!(z.row_background("a", 0, 255), [25, 25, 25, 255], "偶行 #191919");
        assert_eq!(z.row_background("b", 1, 255), [40, 40, 40, 255], "奇行 #282828");
        // 表头行出现后斑马索引不自增: 表头前 idx0(偶), 表头(不增), 后续 idx1(奇)
        assert_eq!(
            z.row_background("------fm器件", 1, 255),
            [80, 60, 0, 255],
            "表头行无视斑马索引"
        );
        assert_eq!(z.row_background("c", 2, 255), [25, 25, 25, 255], "idx2 回偶");
        // alpha 透传 (BaseOverlay.alpha, 默认 180)
        assert_eq!(z.row_background("a", 0, 180), [25, 25, 25, 180]);
    }

    /// 表头谓词可插拔: FMUnpackedDataOverlay.java:87 的 startsWith 覆盖语义
    /// (默认 contains 匹不中的 "fm器件x" 前缀式, 自定义谓词可命中)
    #[test]
    fn header_matcher_override() {
        let mut ov = BaseListOverlay::new(1440, 1.0, 12);
        // contains 默认: "FM文件" 命中
        assert!(ov.zebra.is_header("FM文件: x"));
        // 换 startsWith 谓词 (BaseOverlay.setHeaderMatcher 委托 renderer)
        ov.set_header_matcher(Box::new(|l| l.starts_with("FM文件") || l.starts_with("------fm器件")));
        assert!(ov.zebra.is_header("------fm器件: 机翼"));
        assert!(!ov.zebra.is_header("prefix FM文件")); // startsWith 不命中
    }

    /// 行高 = 2 + fm.getHeight() + 2; preferred 高 = 行数 × 行高 (vgap=0)
    #[test]
    fn row_height_and_preferred_height() {
        let f = font(14); // WebLabel 默认 PLAIN 14+add (BaseOverlay.setupFont:182)
        let m = f.metrics().height;
        assert!(m > 0);
        assert_eq!(ZebraList::row_height(&f), m + 4);
        let f16 = font(16);
        assert_eq!(ZebraList::row_height(&f16), f16.metrics().height + 4);
        let ls = lines(&["a", "b", "c"]);
        assert_eq!(ZebraList::preferred_height(&ls, &f), 3 * (m + 4));
        assert_eq!(ZebraList::preferred_height(&[], &f), 0, "空列表 preferred 0");
    }

    /// 像素: 表头/偶/奇满宽条 + 左缩进 6 处白字 + 底色兜底 (alpha=255 免预乘歧义)
    #[test]
    fn draw_zebra_rows_pixels() {
        let f = font(14);
        let mut z = ZebraList::new();
        let ls = lines(&["FM文件: spitfire", "速度 600", "高度 5000"]);
        let row_h = ZebraList::row_height(&f);
        let w = 120;
        let h = ZebraList::preferred_height(&ls, &f);
        let mut cv = PixCanvas::new(w, h).unwrap();
        z.draw(&mut cv, 0, 0, w, h, &ls, &f, 255, false);

        // 满宽条: 行两端 (含左 margin 区 x=0 与右缘 x=w-1) 均为行底色
        for (y, color, what) in [
            (0, [80, 60, 0, 255], "表头条"),
            (row_h, [25, 25, 25, 255], "偶行条"),
            (2 * row_h, [40, 40, 40, 255], "奇行条"),
        ] {
            let y2 = y + row_h - 1;
            assert_eq!(px(&cv, 0, y), color, "{what} 行顶左缘");
            assert_eq!(px(&cv, w - 1, y), color, "{what} 行顶右缘 (满宽拉伸)");
            assert_eq!(px(&cv, w - 1, y2), color, "{what} 行底右缘");
        }
        // 相邻行色不同 (斑马分界): 行0底 vs 行1顶
        assert_ne!(px(&cv, 0, row_h - 1), px(&cv, 0, row_h));
        // 行底恰为画布底 (preferred 高 = 行数×行高)
        assert_eq!(px(&cv, 5, h - 1), [40, 40, 40, 255], "末行底缘");
        // 白字存在: 表头行文本区 (x≥6, 基线 y=MARGIN_TOP+ascent) 有不透明白像素
        let baseline = MARGIN_TOP + f.metrics().ascent;
        let text_zone = (MARGIN_LEFT..w)
            .map(|x| (x, baseline))
            .find(|&(x, y)| px(&cv, x, y) == TEXT_COLOR);
        assert!(text_zone.is_some(), "基线上有白色字形像素 (x={:?})", text_zone);
        // 左 margin 列 x<MARGIN_LEFT 在无字形负 bearing 侵入时为纯底色:
        // x=0..2 距文本起笔 4px+, 全列均为表头色
        for x in 0..MARGIN_LEFT {
            assert_eq!(px(&cv, x, 0), [80, 60, 0, 255], "左边距列 {x}");
        }
    }

    /// panel 底色兜底 (#141414): 行数不足窗口高时下方露底;
    /// 高度 clamp 时超出行被裁剪 (BaseOverlay.java:275-277 + 窗口裁剪)
    #[test]
    fn draw_panel_bg_and_clipped_rows() {
        let f = font(14);
        let mut z = ZebraList::new();
        let ls = lines(&["a", "b", "c"]);
        let row_h = ZebraList::row_height(&f);
        let w = 100;

        // 兜底: panel_h 比 preferred 高 3px (±2px 容差带 + 余量), 露底为 #141414
        let preferred = ZebraList::preferred_height(&ls, &f);
        let mut cv = PixCanvas::new(w, preferred + 3).unwrap();
        z.draw(&mut cv, 0, 0, w, preferred + 3, &ls, &f, 255, false);
        assert_eq!(px(&cv, 0, preferred), [20, 20, 20, 255], "行下方露 panel 底色");
        assert_eq!(px(&cv, w - 1, preferred + 2), [20, 20, 20, 255]);
        // 空列表 = 纯 panel 底色 (初始无数据窗口)
        let mut cv0 = PixCanvas::new(w, 10).unwrap();
        z.draw(&mut cv0, 0, 0, w, 10, &[], &f, 255, false);
        assert!(cv0
            .pixmap()
            .data()
            .chunks_exact(4)
            .all(|p| p[0] == 20 && p[1] == 20 && p[2] == 20 && p[3] == 255));

        // 裁剪: panel_h 截断末行 (只画前 2 行 + 4px), 截断处之下露底色
        let cut = 2 * row_h + 4;
        let mut cv2 = PixCanvas::new(w, cut).unwrap();
        z.draw(&mut cv2, 0, 0, w, cut, &ls, &f, 255, false);
        assert_eq!(px(&cv2, 0, 2 * row_h), [25, 25, 25, 255], "第 3 行顶部可见 (idx2 偶色)");
        assert_eq!(px(&cv2, 0, cut - 1), [25, 25, 25, 255]);
        // 画布外不可见; panel_h 之外的行整体不画 (此处无第 3 行完整区)
        let mut cv3 = PixCanvas::new(w, 2 * row_h).unwrap();
        z.draw(&mut cv3, 0, 0, w, 2 * row_h, &ls, &f, 255, false);
        assert_eq!(px(&cv3, 0, 2 * row_h - 1), [40, 40, 40, 255], "末行=第 2 行 (奇色)");
    }

    /// 默认 alpha=180 的三层合成 (Java 8 + WebLaF oracle): 间隙 = panel², 行 =
    /// label over panel² — 直通值间隙 0xE9141414 / 表头 0xF93E3005 / 偶 0xF9181818 /
    /// 奇 0xF9222222; PixCanvas 预乘存储 = round(直通×a/255) 与 Java 内部预乘逐位一致
    #[test]
    fn draw_default_alpha_premultiplied() {
        let f = font(14);
        let mut z = ZebraList::new();
        let row_h = ZebraList::row_height(&f);
        let ls = lines(&["FM文件: x", "a", "b"]); // 表头/偶/奇
        let rows_h = 3 * row_h;
        let mut cv = PixCanvas::new(60, rows_h + 5).unwrap();
        z.draw(&mut cv, 0, 0, 60, rows_h + 5, &ls, &f, 180, false);
        // 直通域 oracle 复现 (合成模型自检, 与像素断言分层定位错误)
        let panel = rgba(PANEL_BG_RGB, 180);
        let panel2 = java2d_src_over(panel, java2d_src_over(panel, [0, 0, 0, 0]));
        assert_eq!(panel2, [20, 20, 20, 233], "间隙 = panel² = 0xE9141414");
        assert_eq!(java2d_src_over(rgba(HEADER_RGB, 180), panel2), [62, 48, 5, 249]);
        assert_eq!(java2d_src_over(rgba(ZEBRA_EVEN_RGB, 180), panel2), [24, 24, 24, 249]);
        assert_eq!(java2d_src_over(rgba(ZEBRA_ODD_RGB, 180), panel2), [34, 34, 34, 249]);
        // 预乘: 62·249/255=60.5→61, 48·249/255=46.9→47, 5·249/255=4.9→5;
        // 24·249/255=23.4→23; 34·249/255=33.2→33; 20·233/255=18.3→18
        assert_eq!(px(&cv, 0, 0), [61, 47, 5, 249], "表头 = oracle 0xF93E3005 预乘");
        assert_eq!(px(&cv, 0, row_h), [23, 23, 23, 249], "偶行 = oracle 0xF9181818 预乘");
        assert_eq!(px(&cv, 0, 2 * row_h), [33, 33, 33, 249], "奇行 = oracle 0xF9222222 预乘");
        assert_eq!(px(&cv, 0, rows_h), [18, 18, 18, 233], "余量带 = oracle 0xE9141414 预乘");
        assert_eq!(px(&cv, 59, rows_h + 4), [18, 18, 18, 233], "余量带满宽");
    }

    /// 脏检查生命周期 (BaseOverlay.run:236-241): 首帧必脏 → 同数据不脏 →
    /// 变更脏 → null 不脏且保留基准 → 回到旧数据同基准仍不脏
    #[test]
    fn tick_dirty_check_lifecycle() {
        let f = font(14);
        let mut ov = BaseListOverlay::new(1440, 1.0, 12);
        ov.setup_font(&f);

        assert!(ov.tick(|| Some(lines(&["a", "b"]))), "首帧 (lastData=null → 必更新)");
        assert_eq!(ov.height, 2 * ZebraList::row_height(&f), "高度自适应到 preferred");
        assert!(!ov.tick(|| Some(lines(&["a", "b"]))), "同数据 equals → 不更新");
        assert!(ov.tick(|| Some(lines(&["a", "c"]))), "内容变化 → 更新");
        assert!(!ov.tick(|| None), "null 数据 → 不更新 (Java :237 null 检查)");
        assert!(!ov.tick(|| Some(lines(&["a", "c"]))), "null 未污染基准, 仍与 lastData 同");
        assert!(ov.tick(|| Some(lines(&["a"]))), "行数变化 → 更新, 高度随之降");
        assert_eq!(ov.height, ZebraList::row_height(&f));
    }

    /// 门控语义 (BaseOverlay.run:231-249): 隐藏不取数不显示; preview 绕过门控;
    /// shouldExit/doit 短路; 重现后同数据不重绘但窗口恢复显示
    #[test]
    fn tick_visibility_and_exit_gates() {
        let f = font(14);
        let mut ov = BaseListOverlay::new(1440, 1.0, 12);
        ov.setup_font(&f);
        assert!(ov.tick(|| Some(lines(&["x"]))));
        assert!(ov.window_visible, "可见分支置 window_visible");

        // 游戏模式隐藏 (isVisibleNow=false): supplier 不被调用, 窗口隐藏
        ov.visible_now = false;
        let mut called = false;
        assert!(!ov.tick(|| {
            called = true;
            Some(lines(&["y"]))
        }));
        assert!(!called, "隐藏分支不调 dataSupplier (Java :236 在可见分支内)");
        assert!(!ov.window_visible, "setVisible(false)");

        // 重现: 同数据不重绘, 但窗口恢复显示 (Java :245-247 守卫置 true)
        ov.visible_now = true;
        assert!(!ov.tick(|| Some(lines(&["x"]))), "lastData 未变 → 不重绘");
        assert!(ov.window_visible);

        // preview 模式绕过 isVisibleNow (Java :235 isPreview ||)
        ov.visible_now = false;
        ov.is_preview = true;
        assert!(ov.tick(|| Some(lines(&["z"]))), "preview 隐藏态仍取数且变更脏");
        assert!(ov.window_visible);
        ov.is_preview = false;

        // shouldExit: 短路且不取数 (Java :232-233 break)
        ov.should_exit = true;
        let mut called2 = false;
        assert!(!ov.tick(|| {
            called2 = true;
            Some(lines(&["w"]))
        }));
        assert!(!called2, "shouldExit 后不再取数");

        // stop(): doit=false 同 while 退出 (BaseOverlay.java:286-288)
        ov.should_exit = false;
        ov.stop();
        assert!(!ov.tick(|| Some(lines(&["v"]))));
        assert!(!ov.doit);
    }

    /// 高度自适应 (BaseOverlay.adjustPosition:272-284): clamp 到 logicalHeight-40;
    /// ±2px 容差不调整, >2px 才 setSize
    #[test]
    fn height_adaptation_clamp_and_tolerance() {
        let f = font(14);
        let row_h = ZebraList::row_height(&f);
        let mut ov = BaseListOverlay::new(1000, 1.0, 12);
        ov.setup_font(&f);
        assert_eq!(ov.height, 12 * 72, "初始 height 字段 = defaultFontsize*72 (:95)");

        // clamp: 行数超逻辑屏高 (1000-40=960)
        let n = (960 / row_h + 10) as usize;
        let many: Vec<String> = (0..n).map(|i| format!("行{i}")).collect();
        let preferred = n as i32 * row_h;
        assert!(preferred > 960, "测试前提: preferred 超上限");
        ov.tick(move || Some(many.clone()));
        assert_eq!(ov.height, 960, "钳制到 logicalHeight-40 (:275-277)");

        // 容差: 差 ≤2px 不调整 (|P+2 - P| = 2)
        let small = lines(&["a", "b"]);
        ov.tick(move || Some(small.clone()));
        let p2 = 2 * row_h;
        assert_eq!(ov.height, p2, "差 >2 → 调整到 preferred");
        ov.height = p2 + 2;
        ov.tick(|| Some(lines(&["a", "c"])));
        assert_eq!(ov.height, p2 + 2, "差 2px ≤ 容差 → 不动 (:279)");
        ov.height = p2 + 3;
        ov.tick(|| Some(lines(&["a", "d"])));
        assert_eq!(ov.height, p2, "差 3px > 容差 → 调整");
    }

    /// init 几何公式 (BaseOverlay.java:88-95) 与默认字段值
    #[test]
    fn init_geometry_scaling() {
        // 1440p / 100%: scale=1.0 → fontSize 16, width 432, height 864
        let ov = BaseListOverlay::new(1440, 1.0, 12);
        assert_eq!((ov.font_size, ov.width, ov.height), (16, 432, 864));
        assert_eq!(ov.alpha, 180, "默认 alpha (:33)");
        assert_eq!(ov.refresh_interval_ms, 200, "默认 200ms (:222)");
        assert!(ov.visible_now && !ov.should_exit && ov.doit && !ov.is_preview);
        assert!(ov.last_data.is_none(), "初始 lastData = null (:37)");

        // 1080p / 100%: scale=0.75 → fontSize round(12)=12, width round(324)=324
        let ov = BaseListOverlay::new(1080, 1.0, 12);
        assert_eq!((ov.font_size, ov.width), (12, 324));

        // 1440p / 150% DPI: scale=1.5 → fontSize 24, width round(648)=648
        let ov = BaseListOverlay::new(1440, 1.5, 12);
        assert_eq!((ov.font_size, ov.width), (24, 648));

        // setAlpha
        let mut ov = BaseListOverlay::new(1440, 1.0, 12);
        ov.set_alpha(255);
        assert_eq!(ov.alpha, 255);
    }

    /// render: 窗口画布上按 lastData 出斑马条 (三层预合成); 无数据时纯 panel² 底色
    #[test]
    fn render_to_canvas() {
        let f = font(14);
        let mut ov = BaseListOverlay::new(1440, 1.0, 12);
        ov.setup_font(&f);
        ov.width = 80;
        ov.tick(|| Some(lines(&["FM文件: x", "a"])));
        let h = ov.height;
        let mut cv = PixCanvas::new(80, h).unwrap();
        ov.render(&mut cv, &f, false);
        let row_h = ZebraList::row_height(&f);
        // 预合成直铺: 表头 oracle 0xF93E3005=(249,62,48,5) → 预乘 61/47/5;
        // 偶行 0xF9181818=(249,24,24,24) → 预乘 23 (render2d 头注预乘语义)
        assert_eq!(px(&cv, 0, 0), [61, 47, 5, 249], "表头条 (三层合成, alpha=249)");
        assert_eq!(px(&cv, 0, row_h), [23, 23, 23, 249], "数据条 = 偶行预乘");

        // 无数据: 只铺 panel² 底色 (初始窗口, 间隙色 0xE9141414)
        let mut ov0 = BaseListOverlay::new(1440, 1.0, 12);
        ov0.width = 40;
        ov0.height = 12;
        let mut cv0 = PixCanvas::new(40, 12).unwrap();
        ov0.render(&mut cv0, &f, false);
        assert!(cv0.pixmap().data().chunks_exact(4).all(|p| p[3] == 233));
    }
}
