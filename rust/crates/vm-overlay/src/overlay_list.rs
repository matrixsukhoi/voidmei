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

    /// lastData 清空 — PORT: Java `protected List<String> lastData` 的子类可达面
    /// (组合形态下私有字段经本方法开放)。closeAll 后 preview 重开的"新实例
    /// lastData=null 空面板"语义 (FMUnpackedData reset_preview 消费): 不清则预览
    /// 窗渲染上次 live 行
    pub fn clear_last_data(&mut self) {
        self.last_data = None;
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
mod tests;
