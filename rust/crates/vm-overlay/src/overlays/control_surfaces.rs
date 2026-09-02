//! ControlSurfacesOverlay (ui/overlay/ControlSurfacesOverlay.java) — 操纵面十字指示。
//! 重构波2 自 overlays_field2.rs 拆出 (前半)。
//!
//! 副翼/升降舵/方向舵/可变翼位置: 边框+十字游标 (locater) + 4 行 BOS 标签 +
//! 底部方向舵横条; 50ms 节流。窗口/拖动/FlightDataBus 注册归组装层
//! (LIFETIMES §2.1 注销链), 本文件承载 paintComponent 的绘制序与
//! onFlightData 的数据换算。

use crate::render::primitives;
use std::cell::RefCell;
use std::rc::Rc;

use crate::render::font::LoadedFont;
use crate::render::palette::{aa, colors};

use crate::platform::host::{OverlaySpec, ReinitFn};
use crate::platform::reinit::ReinitParams;
use crate::render::canvas::{LineCapStyle, PixCanvas};
use vm_core::base::format as fast_number_format;
use vm_core::lang::Lang;

// ---------------------------------------------------------------------------
// UIBaseElements 绘制族 (ControlSurfaces 消费面, UIBaseElements.java)
// ---------------------------------------------------------------------------

/// Java Graphics.drawRect(x, y, w-1, h-1) + BasicStroke(1) 的 1px 周界:
/// 单遍描边路径覆盖 [x, x+w)×[y, y+h) 边缘一圈, 每像素恰好一次 (半透明色
/// 不重叠加深)。以四条互不重叠 fill_rect 精确复现 (fill 整数坐标无 AA 歧义)。
fn draw_rect_perimeter(cv: &mut PixCanvas, x: i32, y: i32, w: i32, h: i32, color: [u8; 4]) {
    // PORT: 调用域 (drawHBar/drawVRect) w/h 恒 > 0; Java 负尺寸 drawRect
    // 朝反方向画, 本组件不可达, 不复刻
    if w <= 0 || h <= 0 {
        return;
    }
    cv.fill_rect(x, y, w, 1, color); // 顶边
    if h > 1 {
        cv.fill_rect(x, y + h - 1, w, 1, color); // 底边
    }
    if h > 2 {
        cv.fill_rect(x, y + 1, 1, h - 2, color); // 左边
        if w > 1 {
            cv.fill_rect(x + w - 1, y + 1, 1, h - 2, color); // 右边
        }
    }
}

/// __drawLabelBOSType 的 char[] 版 (UIBaseElements.java:260-273):
/// 数字 (fontNum, colorNum) 基线 y = (2·y_offset + labelSize + unitSize) >> 1;
/// 标签名 (fontLabel, colorLabel) 在 (x + lwidth, y_offset);
/// 单位名 (fontUnit, colorUnit) 在 (x + lwidth, y_offset + labelSize);
/// lwidth = (lwwidth · numSize) >> 2。
#[allow(clippy::too_many_arguments)] // 签名对齐 Java __drawLabelBOSType(g2d, x, y, shade, num, label, unit, buf, len, lbl, unit, lwwidth)
fn draw_label_bos_type(
    cv: &mut PixCanvas,
    num: &LoadedFont,
    label: &LoadedFont,
    unit: &LoadedFont,
    x_offset: i32,
    y_offset: i32,
    s_num: &str,
    s_label: &str,
    s_unit: &str,
    lwwidth: i32,
    aa: bool,
) {
    // 数字
    let lwidth = (lwwidth * num.size) >> 2;
    // y偏移式加下底边再减去自己字体大小的一半
    let num_y = (y_offset + y_offset + label.size + unit.size) >> 1;
    primitives::text_shaded_auto(cv, num, x_offset, num_y, s_num, colors().num, aa);
    // 标签名
    primitives::text_shaded_auto(cv, label, x_offset + lwidth, y_offset, s_label, colors().label, aa);
    // 单位名
    primitives::text_shaded_auto(cv, unit, x_offset + lwidth, y_offset + label.size, s_unit, colors().unit, aa);
}

/// drawHBar (UIBaseElements.java:168-185) 的 val_width ≥ 0 分支 (调用域恒非负):
/// 外边框 drawRect(x, y, w-1, h-1) 阴影色 + 内部条 fillRect(x+b, y+b,
/// val-2b, h-2b) 填充色; 负宽 fillRect 不绘制 (PixCanvas 同)。
#[allow(clippy::too_many_arguments)] // 对齐 Java drawHBar(g2d, x, y, w, h, val, border, c)
fn draw_h_bar(
    cv: &mut PixCanvas,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    val_width: i32,
    borderwidth: i32,
    c: [u8; 4],
) {
    // 外边框 (BasicStroke(borderwidth=1, CAP_ROUND, JOIN_ROUND) 的 1px 周界等效)
    draw_rect_perimeter(cv, x, y, width, height, colors().shade_shape);
    // 内部条
    cv.fill_rect(
        x + borderwidth,
        y + borderwidth,
        val_width - 2 * borderwidth,
        height - 2 * borderwidth,
        c,
    );
}

/// drawVRect (UIBaseElements.java:80-95) 的 height < 0 分支 (drawHBarTextNum 的
/// 游标线专用): 外边框从 (x,y) 向下展开 w × -h, 内部条缩 borderwidth。
#[allow(clippy::too_many_arguments)] // 对齐 Java drawVRect(g2d, x, y, w, h, border, c)
fn draw_v_rect_negative(
    cv: &mut PixCanvas,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    borderwidth: i32,
    c: [u8; 4],
) {
    draw_rect_perimeter(cv, x, y, width, height, colors().shade_shape);
    cv.fill_rect(
        x + borderwidth,
        y + borderwidth,
        width - 2 * borderwidth,
        height - 2 * borderwidth,
        c,
    );
}

/// drawHBarTextNum 的 char[] 版 (UIBaseElements.java:208-218): 横条 +
/// 值游标竖线 (drawVRect, colorLabel) + 值数字 (__drawStringShade, colorLabel)。
/// numFont 尺寸取 label 字体 (调用点 lblFont/numFont 均传 fontLabel)。
/// PORT: lbl 实参传入但 drawHBarText 内的标签绘制在 Java 源已注释
/// (UIBaseElements.java:191-193), 本复刻同忽略。
#[allow(clippy::too_many_arguments)] // 对齐 Java drawHBarTextNum(g2d, x, y, w, h, val, border, c, lbl, num, len, lblFont, numFont)
fn draw_h_bar_text_num(
    cv: &mut PixCanvas,
    lbl_font: &LoadedFont,
    num_font: &LoadedFont,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    val_width: i32,
    borderwidth: i32,
    c: [u8; 4],
    num: &str,
    aa: bool,
) {
    let val_width = if val_width > width { width } else { val_width };
    draw_h_bar(cv, x, y, width, height, val_width, borderwidth, c);
    // 直线 (游标): drawVRect(x + val_width - 2, y, 3, -(height + numFont.getSize()), 1, colorLabel)
    let marker_h = height + num_font.size;
    draw_v_rect_negative(
        cv,
        x + val_width - 2,
        y,
        3,
        marker_h,
        borderwidth,
        colors().label,
    );
    // 数字
    primitives::text_shaded_auto(
        cv,
        lbl_font,
        x + val_width,
        y + height + num_font.size,
        num,
        colors().label,
        aa,
    );
}

// ---------------------------------------------------------------------------
// ControlSurfacesOverlay (ui/overlay/ControlSurfacesOverlay.java)
// ---------------------------------------------------------------------------

/// Throttling to prevent EDT task accumulation (Java:29)
pub const REFRESH_INTERVAL_MS: i64 = 50;

/// ControlSurfaces 的三字体组 (Java init 字段 fontNum/fontLabel/fontUnit;
/// LoadedFont.size 即 Java Font.getSize())。
pub struct CsFonts<'a> {
    /// fontNum = NumFont BOLD fontSize
    pub num: &'a LoadedFont,
    /// fontLabel = FontName BOLD round(fontSize/2)
    pub label: &'a LoadedFont,
    /// fontUnit = NumFont PLAIN round(fontSize/2)
    pub unit: &'a LoadedFont,
}

/// 操纵面位置指示 overlay (ControlSurfacesOverlay.java:27)。C 类复刻保留
/// paintComponent 的绘制序 (:116-149) 与 onFlightData 的数据换算 (:280-312);
/// 窗口/拖动/FlightDataBus 注册归组装层。画布 = 内容区 (twidth × theight),
/// WebLaF setShadeWidth(sw) 的边距由窗口层布局 (本组件不画)。
pub struct ControlSurfacesOverlay {
    /// 节流基准 (Java:31 lastRefreshTime, System.currentTimeMillis 毫秒)
    pub last_refresh_time: i64,
    /// 是否游戏模式 (Java :289 xs != null — preview 时为 false, 数据不更新)
    pub has_service: bool,
    // ---- init 时的 Lang 标签快照 (Java :96-103) ----
    s_elevator_label: String,
    s_elevator_unit: String,
    s_aileron_label: String,
    s_aileron_unit: String,
    s_rudder_label: String,
    s_rudder_unit: String,
    s_wing_sweep_label: String,
    s_wing_sweep_unit: String,
    // ---- Zero-GC Buffers 的 Rust 等价 (Java char[8] + len → String) ----
    pub(crate) elevator_num: String,
    pub(crate) aileron_num: String,
    pub(crate) rudder_num: String,
    pub(crate) wing_sweep_num: String,
    // ---- 几何 (reinitConfig 派生) ----
    pub lx: i32,
    pub ly: i32,
    pub font_size: i32,
    /// fontLabel/fontUnit 的字号 = Math.round(fontSize / 2.0f)
    pub label_font_size: i32,
    /// 十字区边长 width = fontSize * 6 (Java 字段 width, height == width)
    pub width: i32,
    pub height: i32,
    pub locate_size: i32,
    pub stroke_size: i32,
    pub px: i32,
    pub py: i32,
    pub rudder_val_pix: i32,
    /// twidth = (int)(width + 4·fontSize) — 内容区宽 (画布宽)
    pub content_width: i32,
    /// theight = (int)(height + 1.5·fontSize) — 内容区高 (画布高)
    pub content_height: i32,
    /// sw = enableAxisEdge ? 10 : 0 (WebLaF shade width, 窗口层边距)
    pub shade_width: i32,
    /// totalWidth = twidth + sw·2 (窗口 setBounds 宽)
    pub total_width: i32,
    pub total_height: i32,
}

impl Default for ControlSurfacesOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl ControlSurfacesOverlay {
    /// 构造器 (Java :33-36, setTitle("舵面值") 归窗口层)。字段按 Java 隐式
    /// 初始化 (§2.10): 数值 0 / 引用空态 → 空串。
    pub fn new() -> Self {
        ControlSurfacesOverlay {
            last_refresh_time: 0,
            has_service: false,
            s_elevator_label: String::new(),
            s_elevator_unit: String::new(),
            s_aileron_label: String::new(),
            s_aileron_unit: String::new(),
            s_rudder_label: String::new(),
            s_rudder_unit: String::new(),
            s_wing_sweep_label: String::new(),
            s_wing_sweep_unit: String::new(),
            elevator_num: String::new(),
            aileron_num: String::new(),
            rudder_num: String::new(),
            wing_sweep_num: String::new(),
            lx: 0,
            ly: 0,
            font_size: 0,
            label_font_size: 0,
            width: 0,
            height: 0,
            locate_size: 0,
            stroke_size: 0,
            px: 0,
            py: 0,
            rudder_val_pix: 0,
            content_width: 0,
            content_height: 0,
            shade_width: 0,
            total_width: 0,
            total_height: 0,
        }
    }

    /// init 的数据面 (Java :80-160, 窗口操作除外):
    /// reinitConfig → 初值 50 → Lang 标签 → px/py/locateSize/strokeSize →
    /// live 模式标记 (Java s != null 分支; 真实翻转在组装层, 见 has_service)。
    /// * `win_x`/`win_y` — overlaySettings.getWindowX/Y(total) 的结果 (调用方取)。
    pub fn init(
        &mut self,
        font_add: i32,
        dpi_scale: f64,
        enable_axis_edge: bool,
        win_x: i32,
        win_y: i32,
        live: bool,
    ) {
        self.has_service = live;
        self.reinit_config(font_add, dpi_scale, enable_axis_edge, win_x, win_y);

        // Initial Values (50) (Java :91-94)
        self.elevator_num = fast_number_format::format(50.0, 0);
        self.aileron_num = fast_number_format::format(50.0, 0);
        self.rudder_num = fast_number_format::format(50.0, 0);
        self.wing_sweep_num = fast_number_format::format(50.0, 0);

        let lang = Lang::init_lang();
        self.s_elevator_label = lang.v_elevator.to_string();
        self.s_elevator_unit = "%".to_string();
        self.s_aileron_label = lang.v_aileron.to_string();
        self.s_aileron_unit = "%".to_string();
        self.s_rudder_label = lang.v_rudder.to_string();
        self.s_rudder_unit = "%".to_string();
        self.s_wing_sweep_label = lang.v_vario_w.to_string();
        self.s_wing_sweep_unit = "%".to_string();

        self.px = self.width / 2;
        self.py = self.width / 2;
        self.locate_size = self.width / 30;
        self.stroke_size = self.width / 60;
    }

    /// initPreview (Java :162-168): init(null, settings) + 预览样式 (窗口层)。
    pub fn init_preview(
        &mut self,
        font_add: i32,
        dpi_scale: f64,
        enable_axis_edge: bool,
        win_x: i32,
        win_y: i32,
    ) {
        self.init(font_add, dpi_scale, enable_axis_edge, win_x, win_y, false);
    }

    /// reinitConfig (Java :225-271) 的派生量:
    /// fontSize = round((24 + fontadd) · dpiScale); width = fontSize·6;
    /// rudderValPix = 150·width/200; twidth/theight; sw; total; px/py/locate。
    /// PORT: Java :50 的 `static private int fontadd` 为伪单例 (LIFETIMES §1.3
    /// 已判存疑), 此处按参数传入 (实例字段化); repaint() 归组装层。
    /// PORT: strokeSize 只在 init (:111) 赋值, reinitConfig 不刷新 — fontadd
    /// 变更后 Java 保留旧 strokeSize 的行为原样保留 (调用方需重 init 才更新)。
    pub fn reinit_config(
        &mut self,
        font_add: i32,
        dpi_scale: f64,
        enable_axis_edge: bool,
        win_x: i32,
        win_y: i32,
    ) {
        // Apply DPI scaling to font size for crisp rendering on high-DPI displays
        // Math.round(double) = floor(x + 0.5) (§2.3)
        self.font_size = ((24.0 + font_add as f64) * dpi_scale + 0.5).floor() as i32;
        // Math.round(fontSize / 2.0f) = floor(x + 0.5) 的 float 路径 (§2.3)
        self.label_font_size = (self.font_size as f32 / 2.0 + 0.5).floor() as i32;

        self.width = self.font_size * 6;
        self.height = self.width;
        self.rudder_val_pix = (50 + 100) * self.width / 200;

        // (int)(width + 4·fontSize) — int+int 的 (int) 强转为空操作;
        // (int)(height + 1.5·fontSize) — double 和截断向零
        self.content_width = self.width + 4 * self.font_size;
        self.content_height = (self.height as f64 + 1.5 * self.font_size as f64) as i32;

        let sw = if enable_axis_edge { 10 } else { 0 };
        self.shade_width = sw;
        self.total_width = self.content_width + sw * 2;
        self.total_height = self.content_height + sw * 2;

        self.lx = win_x;
        self.ly = win_y;

        self.px = self.width / 2;
        self.py = self.width / 2;
        self.locate_size = self.width / 30;
    }

    /// 数据面回 preview 初值 (Java closeAll = 实例销毁 + refreshPreview 工厂新建
    /// initPreview 实例的 "Initial Values (50)" 段; D8 单条目跨重建存活的补口 —
    /// live 会话残留的 num 串/游标位置在 preview 重开前清除, 否则预览窗显示
    /// 上次 live 数据)。几何不动 (reinit 闭包负责刷新)。
    pub fn reset_preview(&mut self) {
        self.elevator_num = fast_number_format::format(50.0, 0);
        self.aileron_num = fast_number_format::format(50.0, 0);
        self.rudder_num = fast_number_format::format(50.0, 0);
        self.wing_sweep_num = fast_number_format::format(50.0, 0);
        self.px = self.width / 2;
        self.py = self.width / 2;
        self.rudder_val_pix = (50 + 100) * self.width / 200;
    }

    /// onFlightData (Java :280-312) 的单事件语义: 50ms 节流 → (EDT lambda 内)
    /// xs != null 才更新数据; 返回值 = 是否需要重绘 (Java 末尾无条件 repaint)。
    /// PORT: System.currentTimeMillis 由调用方注入 (now_ms), 便于测试。
    pub fn on_flight_data(
        &mut self,
        now_ms: i64,
        aileron: f64,
        elevator: f64,
        rudder: f64,
        wing_sweep: f64,
        wing_sweep_valid: bool,
    ) -> bool {
        // Throttling prevents EDT task accumulation
        if now_ms - self.last_refresh_time < REFRESH_INTERVAL_MS {
            return false; // Skip this update, too soon
        }
        self.last_refresh_time = now_ms;
        if self.has_service {
            self.update_flight_data(aileron, elevator, rudder, wing_sweep, wing_sweep_valid);
        }
        true
    }

    /// onFlightData 的 invokeLater lambda 数据面 (Java :289-309):
    /// (int) 截断 ±100 域遥测 → 十字游标 (px/py) + 方向舵条 (rudderValPix) +
    /// FastNumberFormatter 整数格式化。
    pub fn update_flight_data(
        &mut self,
        aileron: f64,
        elevator: f64,
        rudder: f64,
        wing_sweep: f64,
        wing_sweep_valid: bool,
    ) {
        // (int) double 截断向零 ↔ as i32 同 (NaN→0, 域内 ±100 无饱和差异)
        let aileron_val = aileron as i32;
        let elevator_val = elevator as i32;
        let rudder_val = rudder as i32;
        let ws_val = if wing_sweep_valid { (wing_sweep * 100.0) as i32 } else { 0 };

        self.px = (100 + aileron_val) * self.width / 200;
        self.py = (100 + elevator_val) * self.width / 200;
        self.rudder_val_pix = (rudder_val + 100) * self.width / 200;

        self.aileron_num = fast_number_format::format(aileron_val as f64, 0);
        self.elevator_num = fast_number_format::format(elevator_val as f64, 0);
        self.rudder_num = fast_number_format::format(rudder_val as f64, 0);
        self.wing_sweep_num = fast_number_format::format(ws_val as f64, 0);
    }

    /// locater (Java :177-205): 边框 (BasicStroke(1), colorShadeShape) +
    /// 影子十字 (BasicStroke(stroke), colorShadeShape) + 主十字 (colorNum,
    /// 相对影子 -1px 偏移)。裸 BasicStroke = CAP_SQUARE/JOIN_MITER。
    /// 参数名对齐 Java: `x`,`y` = 游标中心; `r` = 边框边长 (width 字段);
    /// `width` = 十字臂半长参数 (locateSize 实参); `stroke` = 线宽 (strokeSize)。
    #[allow(clippy::too_many_arguments)] // 对齐 Java locater(g2d, x, y, r, width, stroke)
    fn locater(&self, cv: &mut PixCanvas, x: i32, y: i32, r: i32, width: i32, stroke: f32, aa: bool) {
        // 绘制边框
        for &(x0, y0, x1, y1) in &[(0, 0, 0, r), (0, 0, r, 0), (0, r - 1, r - 1, r - 1), (r - 1, 0, r - 1, r - 1)] {
            cv.draw_line_cap(x0, y0, x1, y1, 1.0, colors().shade_shape, LineCapStyle::Square, aa);
        }

        // 绘制影子 (横线 + 竖线)
        cv.draw_line_cap(x - width / 2, y, x + width / 2, y, stroke, colors().shade_shape, LineCapStyle::Square, aa);
        cv.draw_line_cap(x, y - width / 2, x, y + width / 2, stroke, colors().shade_shape, LineCapStyle::Square, aa);

        // 主十字 (colorNum, -1 偏移): 横线 + 竖线
        cv.draw_line_cap(x - width / 2 - 1, y - 1, x + width / 2 - 1, y - 1, stroke, colors().num, LineCapStyle::Square, aa);
        cv.draw_line_cap(x - 1, y - width / 2 - 1, x - 1, y + width / 2 - 1, stroke, colors().num, LineCapStyle::Square, aa);
    }

    /// topPanel.paintComponent (Java :116-149) 的绘制序:
    /// locater → 4 行 BOS 标签 (升降舵/副翼/方向舵/可变翼, dy 步进 1.5·fontSize)
    /// → 底部方向舵横条 (drawHBarTextNum)。
    /// 画布须为 content_width × content_height (Swing 裁剪语义, 防呆断言)。
    pub fn draw(&self, cv: &mut PixCanvas, fonts: &CsFonts, aa: bool) {
        debug_assert!(
            cv.width() == self.content_width && cv.height() == self.content_height,
            "画布须为 {}×{}, 实为 {}×{}",
            self.content_width, self.content_height, cv.width(), cv.height()
        );
        self.locater(cv, self.px, self.py, self.width, self.locate_size, self.stroke_size as f32, aa);

        // dy 序列: fontSize>>1 起步, 每行 +1.5·fontSize (Java 复合赋值隐式 (int) 截断)
        let mut dy = self.font_size >> 1;
        draw_label_bos_type(
            cv, fonts.num, fonts.label, fonts.unit, self.width, dy,
            &self.elevator_num, &self.s_elevator_label, &self.s_elevator_unit, 9, aa,
        );
        dy = ((dy as f64) + 1.5 * self.font_size as f64) as i32;
        draw_label_bos_type(
            cv, fonts.num, fonts.label, fonts.unit, self.width, dy,
            &self.aileron_num, &self.s_aileron_label, &self.s_aileron_unit, 9, aa,
        );
        dy = ((dy as f64) + 1.5 * self.font_size as f64) as i32;
        draw_label_bos_type(
            cv, fonts.num, fonts.label, fonts.unit, self.width, dy,
            &self.rudder_num, &self.s_rudder_label, &self.s_rudder_unit, 9, aa,
        );
        dy = ((dy as f64) + 1.5 * self.font_size as f64) as i32;
        draw_label_bos_type(
            cv, fonts.num, fonts.label, fonts.unit, self.width, dy,
            &self.wing_sweep_num, &self.s_wing_sweep_label, &self.s_wing_sweep_unit, 9, aa,
        );

        // 底部方向舵横条: drawHBarTextNum(g2d, 0, height, width, fontSize>>1,
        // rudderValPix, 1, colorNum, lbl, num, fontLabel, fontLabel) (Java :146-148)
        draw_h_bar_text_num(
            cv, fonts.label, fonts.label,
            0, self.height, self.width, self.font_size >> 1, self.rudder_val_pix, 1,
            colors().num, &self.rudder_num, aa,
        );
    }
}

// ---------------------------------------------------------------------------
// OverlayHost 挂载 (Java Controller.java:680 registerWithPreview("enableAxis"))
// ---------------------------------------------------------------------------

/// 操纵面共享句柄 (minihud_overlay_spec 先例: render 闭包与喂入方共享 state)
pub type ControlSurfacesHandle = Rc<RefCell<ControlSurfacesOverlay>>;

/// 操纵面 OverlaySpec + live 句柄。参数为 init(:80-160)/reinitConfig (:225-271)
/// 的配置面, 经 [`ReinitParams`] 仓读取: font_add = "舵面值" panel 的 fontSize
/// 增量, enable_axis_edge = enableAxisEdge (cfg 缺省 false)。
/// PORT(边框不承载): Java totalWidth = twidth+sw·2 的 sw 是 WebLaF 窗口装饰边距,
/// host 无边框层 — spec 尺寸 = 内容区 content_width×content_height (draw 的画布
/// 断言钉内容尺寸, Swing 裁剪语义)。
/// PORT(数据门控): Java init(S) 置 xs!=null (has_service) 才更新数据、initPreview
/// 置 false; Rust 单实例形态下由 win32 命令处理点按**会话窗口形态**切换 has_service
/// (app_shell OpenAllOverlays→true / CloseAllOverlays→false, 对位 init(S)/实例销毁;
/// 喂入点 feed_overlays_live 幂等置 true) — 初值随 init_preview 为 false。
/// PORT(WYSIWYG): reinit 闭包 = reinit_config 的几何段 (字号/edge → 宽高派生) +
/// 三字体重载 (Java :225-241 的 fontNum/fontLabel/fontUnit new Font)
pub fn control_surfaces_overlay_spec(
    fonts_dir: &std::path::Path,
    params: &Rc<RefCell<ReinitParams>>,
) -> Result<(ControlSurfacesHandle, OverlaySpec), String> {
    let (font_add, dpi_scale, enable_axis_edge) = {
        let p = params.borrow();
        (p.font_add_axis, p.dpi_scale, p.axis_show_edge)
    };
    let mut cs = ControlSurfacesOverlay::new();
    // win_x/win_y = 0: 窗口定位归 host 位置存档 (HudSettingsSnapshot 同规)
    cs.init_preview(font_add, dpi_scale, enable_axis_edge, 0, 0);
    // 三字体 (Java init :96-103): num = NumFont BOLD(fontSize),
    // label = FontName BOLD(round(fontSize/2)), unit = NumFont PLAIN(round(fontSize/2))
    let bold_path = fonts_dir.join("sarasa-mono-sc-bold.ttf");
    let regular_path = fonts_dir.join("sarasa-mono-sc-regular.ttf");
    let f_num = Rc::new(RefCell::new(Rc::new(LoadedFont::new(&bold_path, cs.font_size)?)));
    let f_label = Rc::new(RefCell::new(Rc::new(LoadedFont::new(
        &bold_path,
        cs.label_font_size,
    )?)));
    let f_unit = Rc::new(RefCell::new(Rc::new(LoadedFont::new(
        &regular_path,
        cs.label_font_size,
    )?)));
    let (w, h) = (cs.content_width, cs.content_height);
    let handle: ControlSurfacesHandle = Rc::new(RefCell::new(cs));
    let render_handle = Rc::clone(&handle);
    let (render_num, render_label, render_unit) =
        (Rc::clone(&f_num), Rc::clone(&f_label), Rc::clone(&f_unit));
    // reinit 闭包: 几何 + 三字体重建, 返回新内容区尺寸 (Java setBounds 内容面)
    let reinit_handle = Rc::clone(&handle);
    let (reinit_num, reinit_label, reinit_unit) =
        (Rc::clone(&f_num), Rc::clone(&f_label), Rc::clone(&f_unit));
    let reinit_params = Rc::clone(params);
    let (reinit_bold, reinit_regular) = (bold_path, regular_path);
    let reinit: ReinitFn = Box::new(move || {
        let (fa, dpi, edge) = {
            let p = reinit_params.borrow();
            (p.font_add_axis, p.dpi_scale, p.axis_show_edge)
        };
        let mut cs = reinit_handle.borrow_mut();
        cs.reinit_config(fa, dpi, edge, 0, 0);
        let (fs, lfs) = (cs.font_size, cs.label_font_size);
        let (w, h) = (cs.content_width, cs.content_height);
        drop(cs);
        let fonts = match (
            LoadedFont::new(&reinit_bold, fs),
            LoadedFont::new(&reinit_bold, lfs),
            LoadedFont::new(&reinit_regular, lfs),
        ) {
            (Ok(n), Ok(l), Ok(u)) => (Rc::new(n), Rc::new(l), Rc::new(u)),
            (r, _, _) => {
                if let Err(e) = r {
                    vm_core::base::logger::error("ControlSurfaces", &format!("reinit 字体重载失败: {}", e));
                }
                return None;
            }
        };
        *reinit_num.borrow_mut() = fonts.0;
        *reinit_label.borrow_mut() = fonts.1;
        *reinit_unit.borrow_mut() = fonts.2;
        Some((w, h))
    });
    Ok((
        handle,
        OverlaySpec {
            // Java LinkedHashMap 键 = configKey (Controller.java:680)
            id: "enableAxis".to_string(),
            config_key: "enableAxis".to_string(),
            width: w,
            height: h,
            render: Box::new(move |cv: &mut PixCanvas| {
                // aa = 运行时仓 (cfg AAEnable 可关)
                let (num, label, unit) =
                    (render_num.borrow(), render_label.borrow(), render_unit.borrow());
                let fonts = CsFonts {
                    num: &num,
                    label: &label,
                    unit: &unit,
                };
                render_handle.borrow().draw(cv, &fonts, aa());
            }),
            reinit: Some(reinit),
        },
    ))
}
