//! overlays_field2: 操纵面十字指示 + FM 调试列表 (P4 批十 C 类语义复刻)
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | [`ControlSurfacesOverlay`] | ui/overlay/ControlSurfacesOverlay.java | 副翼/升降舵/方向舵/可变翼位置: 边框+十字游标 (locater) + 4 行 BOS 标签 + 底部方向舵横条; 50ms 节流 |
//! | [`FmUnpackedDataOverlay`] | ui/overlay/FMUnpackedDataOverlay.java | FM 调试列表: BaseOverlay 斑马纹基座 + blkx 字段直读清单 (D4 砍反射段后的等价实现) |
//!
//! ControlSurfaces 窗口/拖动/FlightDataBus 注册归组装层 (LIFETIMES §2.1 注销链),
//! 本文件承载 paintComponent 的绘制序与 onFlightData 的数据换算。
//!
//! FMUnpackedData 的 UIStateBus 订阅 (FM_OVERLAY_TOGGLE/FM_CHANGED) 对应
//! [`FmUnpackedDataOverlay::toggle`]/[`FmUnpackedDataOverlay::reload_fm_data`],
//! 由组装层的事件循环驱动; dispose 的退订由所有权 Drop 根治 (LIFETIMES §2.3),
//! 无需显式方法。
//!
//! PORT(P5 组装契约, 现 host::OverlaySpec 不可表达, 组装层需扩展 host):
//! (a) 动态窗口高 — `base.adjust_position` 按行数改 `base.height`, host 的
//! canvas 按注册尺寸建一次, 需加 resize; (b) 逐条目可见性 — `base.window_visible`
//! 需 per-entry `set_visible` (现仅全局 hide/show_all); (c) 两组件的预览渲染
//! 闭包工厂 (field1 先例 `*_preview_spec`) 留组装层接线。

use crate::font::LoadedFont;
use crate::gauges_bars::{COLOR_LABEL, COLOR_NUM, COLOR_SHADE_SHAPE};
use crate::gauge_attitude::COLOR_UNIT;
use crate::host::OverlaySpec;
use crate::overlay_list::BaseListOverlay;
use crate::render2d::{LineCapStyle, PixCanvas};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use vm_core::blkx::{Blkx, FmParts};
use vm_core::config_api::ConfigProvider;
use vm_core::format as fast_number_format;
use vm_core::g;
use vm_core::lang::Lang;

// ---------------------------------------------------------------------------
// Java String.format 最小面 (Lang 模板域: %s / %d / %.0f~%.3f / %%)
// ---------------------------------------------------------------------------

/// printf 实参 (FMUnpackedDataOverlay.generateLines 传入 Lang 模板的三类占位)
#[derive(Clone, Copy, Debug)]
enum FmtArg<'a> {
    /// %s — null 实参以 "null" 文本呈现 (Java Formatter 行为)
    S(&'a str),
    /// %d — 襟翼档位序号 (i32 十进制)
    D(i32),
    /// %.Nf — 精度由模板解析
    F(f64),
}

/// Java `String.format(template, args...)` 一比一 (Lang.bXXX 模板 + 实参)。
/// 支持域: `%s`/`%d`/`%.0f`~`%.9f`/`%%`; 其余转换符 Java 抛
/// UnknownFormatConversionException ↔ 此处 panic; `%d` 位点收浮点/字符串实参
/// Java 抛 IllegalFormatConversionException ↔ 此处同 panic (模板与实参由本模块
/// 成对提供, 用户改 lang 文件破坏配对时两语言同为崩溃语义)。
/// `%s` 位点收数值实参在 Java 合法 (toString 输出), 本实现防御 panic — 域内
/// 实参编译期成对不可达。
fn java_string_format(template: &str, args: &[FmtArg]) -> String {
    let mut out = String::new();
    let mut arg_i = 0usize;
    let bytes = template.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b != b'%' {
            // PORT: 模板为 ASCII 控制符 + CJK 文本, 非控制字节段整段透传
            // (按字节推进仅发生在 ASCII 控制符处, UTF-8 多字节序列不越界)
            let start = i;
            while i < bytes.len() && bytes[i] != b'%' {
                i += 1;
            }
            out.push_str(&template[start..i]);
            continue;
        }
        // '%' 分发
        let next = bytes.get(i + 1).copied();
        match next {
            Some(b'%') => {
                out.push('%'); // %% → 字面 %
                i += 2;
            }
            Some(b's') | Some(b'd') => {
                let arg = args.get(arg_i).unwrap_or_else(|| {
                    panic!("String.format 实参不足: {template:?} 第 {arg_i} 个占位")
                });
                arg_i += 1;
                match *arg {
                    FmtArg::S(s) => match next {
                        Some(b's') => out.push_str(s),
                        // Java: %d 收 String 抛 IllegalFormatConversionException (§1 崩溃语义)
                        _ => panic!(
                            "String.format %d 收到字符串实参 (IllegalFormatConversionException): {template:?}"
                        ),
                    },
                    // Integer 的 %s/%d 位点 Java 均合法 (toString / 十进制)
                    FmtArg::D(v) => out.push_str(&v.to_string()),
                    FmtArg::F(_) => match next {
                        // Java: %d 收 Double 抛 IllegalFormatConversionException (§1 崩溃语义)
                        Some(b'd') => panic!(
                            "String.format %d 收到浮点实参 (IllegalFormatConversionException): {template:?}"
                        ),
                        // Java %s 收 Double 合法 (toString), 本实现防御 panic — 域内不可达
                        _ => panic!("模板 %s 位点收到数值实参 (域外防御): {template:?}"),
                    },
                }
                i += 2;
            }
            Some(b'.') => {
                // %.Nf
                let mut j = i + 2;
                let mut prec: u32 = 0;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    prec = prec * 10 + u32::from(bytes[j] - b'0');
                    j += 1;
                }
                if j >= bytes.len() || bytes[j] != b'f' {
                    panic!("String.format 未支持的转换符: {template:?} @ {i}");
                }
                // PORT: Java BigDecimal 任意精度合法, 本实现 u128 尾数累加上界 ≤9
                // (下方 as u8 截断与 10u128.pow 回绕均在此拦截); 超域仅模板漂移
                // 可达 → debug 断言, release 不引入 Java 没有的崩溃
                debug_assert!(prec <= 9, "String.format 精度超域 (.{prec}f > .9f): {template:?}");
                let arg = args.get(arg_i).unwrap_or_else(|| {
                    panic!("String.format 实参不足: {template:?} 第 {arg_i} 个占位")
                });
                arg_i += 1;
                match *arg {
                    FmtArg::F(v) => out.push_str(&java_format_f(v, prec as u8)),
                    FmtArg::S(_) | FmtArg::D(_) => {
                        panic!("模板 %.Nf 位点收到非数值实参: {template:?}")
                    }
                }
                i = j + 1;
            }
            _ => panic!("String.format 未支持的转换符: {template:?} @ {i}"),
        }
    }
    out
}

/// Java `String.format("%.{prec}f", d)` 一比一。
/// 语义模型 (vm-core flight_analyzer.rs java_format_f1 / config_loader.rs
/// java_format_f4 同源, Java 8 oracle 实证): 等价
/// `new BigDecimal(Double.toString(d)).setScale(prec, HALF_UP)` — 对**最短往返
/// 十进制表示**做 HALF_UP (5.25 → "5.3"), 而非精确二进制值展开; Rust `{:.N}`
/// 是对精确值的半偶舍入, 双重分歧 (2.675 → Java "2.68" vs Rust "2.67")。
/// NaN/Infinity 原样; 负号含 -0.0 (neg = is_sign_negative, Java Formatter 亦保留)。
/// 巨整数域 (exp10 > 25, double 间距 > 1 恒无有效小数): digits + 隐含尾零 + ".0"×prec。
fn java_format_f(d: f64, prec: u8) -> String {
    // 域界断言: prec≤9 时 u128 尾数 (整数部 ≤26 位 + 小数 9 位) 恒不溢出;
    // ≥39 时 10u128.pow 溢出 (Java BigDecimal 无此界, 属模板漂移信号)
    debug_assert!(prec <= 9, "java_format_f 精度超域: {prec}");
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    let neg = d.is_sign_negative(); // 含 -0.0 → "-0.0" (Java 亦然)
    let a = d.abs();
    let sci = format!("{:e}", a); // 最短往返表示 "D.DDDe±k"
    let epos = sci.find('e').unwrap();
    let mant = &sci[..epos];
    let exp10: i32 = sci[epos + 1..].parse().unwrap();
    let digits = mant.replace('.', "");
    let digits = digits.as_bytes();
    let n = digits.len() as i32;

    let mut out = String::new();
    if exp10 > 25 {
        // 巨整数域: 全整数输出 + prec 位零小数 (域内 FM 数值不可达, 防御分支)
        out.push_str(&sci[..epos].replace('.', ""));
        out.push_str(&"0".repeat((exp10 - n + 1) as usize));
        if prec > 0 {
            out.push('.');
            out.push_str(&"0".repeat(prec as usize));
        }
    } else {
        // 最短表示的 i 号数字 (1-based, place = 10^(exp10-i+1)); 越界补 0
        let digit_at = |i: i32| -> u128 {
            if i < 1 {
                0
            } else {
                let idx = (i - 1) as usize;
                if idx < digits.len() {
                    u128::from(digits[idx] - b'0')
                } else {
                    0
                }
            }
        };
        // 保留到 10^-prec 位: i ≤ exp10 + 1 + prec; 判定位 = 其后一位 (HALF_UP:
        // ≥5 进位, 再后的剩余数字 < 1 单位不影响判定)
        let keep = exp10 + 1 + prec as i32;
        let mut scaled: u128 = 0; // = 整数 × 10^prec + 小数
        if keep > 0 {
            for i in 1..=keep {
                scaled = scaled * 10 + digit_at(i);
            }
        }
        if digit_at(keep + 1) >= 5 {
            scaled += 1; // HALF_UP (含精确 .5 进位; 进位可级联到整数部分)
        }
        let div = 10u128.pow(prec as u32);
        let int_part = scaled / div;
        let frac = scaled % div;
        out.push_str(&int_part.to_string());
        if prec > 0 {
            out.push('.');
            let s = frac.to_string();
            for _ in s.len()..prec as usize {
                out.push('0');
            }
            out.push_str(&s);
        }
    }
    if neg {
        out.insert(0, '-');
    }
    out
}

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

/// __drawStringShade 的 char[] 版 (UIBaseElements.java:46-55): 黑影 (x+1, y+1)
/// → 本色 (x, y), 两遍 drawText; shadeWidth 只作用于 setStroke (对文本无效果), 不复刻
fn draw_string_shade(
    cv: &mut PixCanvas,
    font: &LoadedFont,
    x: i32,
    y: i32,
    s: &str,
    color: [u8; 4],
    aa: bool,
) {
    // drawshade (No Shape support for char[] yet, fallback to simple shade)
    cv.draw_text(font, x + 1, y + 1, s, COLOR_SHADE_SHAPE, aa);
    cv.draw_text(font, x, y, s, color, aa);
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
    draw_string_shade(cv, num, x_offset, num_y, s_num, COLOR_NUM, aa);
    // 标签名
    draw_string_shade(cv, label, x_offset + lwidth, y_offset, s_label, COLOR_LABEL, aa);
    // 单位名
    draw_string_shade(cv, unit, x_offset + lwidth, y_offset + label.size, s_unit, COLOR_UNIT, aa);
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
    draw_rect_perimeter(cv, x, y, width, height, COLOR_SHADE_SHAPE);
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
    draw_rect_perimeter(cv, x, y, width, height, COLOR_SHADE_SHAPE);
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
        COLOR_LABEL,
    );
    // 数字
    draw_string_shade(
        cv,
        lbl_font,
        x + val_width,
        y + height + num_font.size,
        num,
        COLOR_LABEL,
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
    elevator_num: String,
    aileron_num: String,
    rudder_num: String,
    wing_sweep_num: String,
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
    /// 游戏模式标记 (s != null 分支的 setVisible/register 归组装层)。
    /// * `win_x`/`win_y` — overlaySettings.getWindowX/Y(total) 的结果 (调用方取)。
    pub fn init(
        &mut self,
        font_add: i32,
        dpi_scale: f64,
        enable_axis_edge: bool,
        win_x: i32,
        win_y: i32,
        game_mode: bool,
    ) {
        self.has_service = game_mode;
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
        // Java: this.getContentPane().repaint() — 恒调度重绘 (preview 亦然)
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
            cv.draw_line_cap(x0, y0, x1, y1, 1.0, COLOR_SHADE_SHAPE, LineCapStyle::Square, aa);
        }

        // 绘制影子 (横线 + 竖线)
        cv.draw_line_cap(x - width / 2, y, x + width / 2, y, stroke, COLOR_SHADE_SHAPE, LineCapStyle::Square, aa);
        cv.draw_line_cap(x, y - width / 2, x, y + width / 2, stroke, COLOR_SHADE_SHAPE, LineCapStyle::Square, aa);

        // 主十字 (colorNum, -1 偏移): 横线 + 竖线
        cv.draw_line_cap(x - width / 2 - 1, y - 1, x + width / 2 - 1, y - 1, stroke, COLOR_NUM, LineCapStyle::Square, aa);
        cv.draw_line_cap(x - 1, y - width / 2 - 1, x - 1, y + width / 2 - 1, stroke, COLOR_NUM, LineCapStyle::Square, aa);
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
            COLOR_NUM, &self.rudder_num, aa,
        );
    }
}

// ---------------------------------------------------------------------------
// OverlayHost 挂载 (Java Controller.java:680 registerWithPreview("enableAxis"))
// ---------------------------------------------------------------------------

/// 操纵面共享句柄 (minihud_overlay_spec 先例: render 闭包与喂入方共享 state)
pub type ControlSurfacesHandle = Rc<RefCell<ControlSurfacesOverlay>>;

/// 操纵面 OverlaySpec + live 句柄。参数为 init(:80-160) 的配置面:
/// `font_add` = "舵面值" panel 的 fontSize 增量, `enable_axis_edge` = enableAxisEdge
/// (cfg 缺省 false)。
/// PORT(边框不承载): Java totalWidth = twidth+sw·2 的 sw 是 WebLaF 窗口装饰边距,
/// host 无边框层 — spec 尺寸 = 内容区 content_width×content_height (draw 的画布
/// 断言钉内容尺寸, Swing 裁剪语义)。
/// PORT(数据门控): Java init(S) 置 xs!=null (has_service) 才更新数据、initPreview
/// 置 false; Rust 单实例形态下由 win32 命令处理点按**会话窗口形态**切换 has_service
/// (app_shell OpenAllOverlays→true / CloseAllOverlays→false, 对位 init(S)/实例销毁;
/// 喂入点 feed_overlays_live 幂等置 true) — 初值随 init_preview 为 false
pub fn control_surfaces_overlay_spec(
    fonts_dir: &std::path::Path,
    font_add: i32,
    dpi_scale: f64,
    enable_axis_edge: bool,
) -> Result<(ControlSurfacesHandle, OverlaySpec), String> {
    let mut cs = ControlSurfacesOverlay::new();
    // win_x/win_y = 0: 窗口定位归 host 位置存档 (HudSettingsSnapshot 同规)
    cs.init_preview(font_add, dpi_scale, enable_axis_edge, 0, 0);
    // 三字体 (Java init :96-103): num = NumFont BOLD(fontSize),
    // label = FontName BOLD(round(fontSize/2)), unit = NumFont PLAIN(round(fontSize/2))
    let bold_path = fonts_dir.join("sarasa-mono-sc-bold.ttf");
    let regular_path = fonts_dir.join("sarasa-mono-sc-regular.ttf");
    let f_num = Rc::new(LoadedFont::new(&bold_path, cs.font_size)?);
    let f_label = Rc::new(LoadedFont::new(&bold_path, cs.label_font_size)?);
    let f_unit = Rc::new(LoadedFont::new(&regular_path, cs.label_font_size)?);
    let (w, h) = (cs.content_width, cs.content_height);
    let handle: ControlSurfacesHandle = Rc::new(RefCell::new(cs));
    let render_handle = Rc::clone(&handle);
    Ok((
        handle,
        OverlaySpec {
            // Java LinkedHashMap 键 = configKey (Controller.java:680)
            id: "enableAxis".to_string(),
            config_key: "enableAxis".to_string(),
            width: w,
            height: h,
            render: Box::new(move |cv: &mut PixCanvas| {
                // 生产 AA 恒开 (Application.java:102 graphAASetting 默认 ON)
                let fonts = CsFonts {
                    num: &f_num,
                    label: &f_label,
                    unit: &f_unit,
                };
                render_handle.borrow().draw(cv, &fonts, true);
            }),
        },
    ))
}

// ---------------------------------------------------------------------------
// FmUnpackedDataOverlay (ui/overlay/FMUnpackedDataOverlay.java)
// ---------------------------------------------------------------------------

/// FM 调试列表 overlay (FMUnpackedDataOverlay.java:32)。组合 BaseListOverlay
/// (Java extends BaseOverlay — §1 禁强行继承, 公共行为已上提基座):
/// 自管可见性 (游戏模式热键切换) + blkx 字段直读清单。
///
/// PORT: Java 经 FMDataAdapter 持 volatile blkx; vm-core 的 FMDataAdapter
/// 尚消费 BlkxPlaceholder (fm_data_adapter.rs TODO(port)), 本组件按任务裁决
/// 直读真实 `blkx::Blkx` (D4 model 字段面), 避免占位类型第二真相源。
/// set_blkx 的 volatile 赋值语义由"单写者(事件循环)+tick 前快照"承接。
pub struct FmUnpackedDataOverlay {
    /// BaseOverlay 基座 (run 循环状态机: 脏检查/高度自适应/可见门控)
    pub base: BaseListOverlay,
    /// Self-managed visibility state (game mode toggle) (Java :42)
    pub visible: bool,
    /// FMDataAdapter.getBlkx() 的等价持有 (None = 未加载 → 占位清单)
    blkx: Option<Arc<Blkx>>,
    /// Java :39 config = c.getConfigProvider() (None ↔ Java null 容忍)
    config: Option<Arc<dyn ConfigProvider>>,
}

impl FmUnpackedDataOverlay {
    /// 构造 + BaseOverlay.init 几何 (Java :46-48 super() 与 :90 super.init 的
    /// 几何段合一; 行高度量 setup_font 由 init/reinit 时调用方补)。
    pub fn new(logical_height: i32, dpi_scale: f64, default_fontsize: i32) -> Self {
        FmUnpackedDataOverlay {
            base: BaseListOverlay::new(logical_height, dpi_scale, default_fontsize),
            visible: true,
            blkx: None,
            config: None,
        }
    }

    /// 游戏模式 init (Java :57-94): config 注入 + 隐藏起步 + 表头谓词 +
    /// BaseOverlay 数据供给挂接 (tick 内联)。UIStateBus 订阅 (toggle/FM_CHANGED)
    /// 归组装层事件循环, 对应 [`Self::toggle`]/[`Self::reload_fm_data`]。
    pub fn init(&mut self, config: Option<Arc<dyn ConfigProvider>>, font: &LoadedFont) {
        self.config = config;
        // Java :64 this.isPreview = false — 继承自 BaseOverlay 的单一字段,
        // 对应 base.is_preview (run 门控 :235 的唯一读取方)
        self.base.is_preview = false;

        // Game mode: initially hidden
        self.visible = false;
        self.base.setup_font(font);

        // Set header matcher for styling (FM parts headers start with "------fm器件")
        self.base.set_header_matcher(Box::new(|line| {
            line.starts_with("FM文件") || line.starts_with("------fm器件")
        }));
    }

    /// 预览模式 initPreview (Java :103-122): 恒可见 + 同表头谓词。
    pub fn init_preview(&mut self, config: Option<Arc<dyn ConfigProvider>>, font: &LoadedFont) {
        self.config = config;
        // Java :110 this.isPreview = true (BaseOverlay 单一字段, 同上)
        self.base.is_preview = true;

        // Preview mode: always visible
        self.visible = true;
        self.base.setup_font(font);

        self.base.set_header_matcher(Box::new(|line| {
            line.starts_with("FM文件") || line.starts_with("------fm器件")
        }));
    }

    /// FM_CHANGED handler 的 reloadFMData (Java :130-136): 句柄换 blkx 快照。
    /// 非 READY 句柄 blkx=null → None → 占位 "[No Data Loaded]" (null 容忍)。
    /// 数据刷新由下一 tick 周期完成 (Java 注释 :135)。
    /// PORT: Java :131 的 `payload instanceof FMHandle` 过滤由组装层承担 —
    /// P5 事件路由时非 FMHandle 载荷应保留旧 blkx 不调用本方法。
    pub fn reload_fm_data(&mut self, blkx: Option<Arc<Blkx>>) {
        self.blkx = blkx;
        // Data will be refreshed on next run() cycle
    }

    /// reinitConfig (Java :142-151): adapter 直读 FMManager.current() 换新
    /// (调用方传入 current().blkx; 非 READY 句柄 blkx 为 null → None,
    /// setBlkx(null) 清空 → 占位容忍) + setupFont。
    pub fn reinit_config(&mut self, current_blkx: Option<Arc<Blkx>>, font: &LoadedFont) {
        self.blkx = current_blkx;
        // Font and display settings are handled by BaseOverlay
        self.base.setup_font(font);
    }

    /// FM_OVERLAY_TOGGLE handler (Java :72-75): 翻转自管可见性。
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// isVisibleNow 覆写 (Java :318-321)
    pub fn is_visible_now(&self) -> bool {
        self.visible
    }

    /// BaseOverlay.run() 单轮 (经 overlay_list 基座): 先同步 isVisibleNow
    /// 门控位, 再以当前 blkx/config 快照生成行清单。
    pub fn tick(&mut self) -> bool {
        self.base.visible_now = self.visible;
        let blkx = self.blkx.clone();
        let config = self.config.clone();
        self.base
            .tick(move || Some(generate_lines(blkx.as_deref(), config.as_deref())))
    }

    /// generateLines (Java :157-278) — 独立函数便于直测。
    pub fn generate_lines(&self) -> Vec<String> {
        generate_lines(self.blkx.as_deref(), self.config.as_deref())
    }

    /// updateUI 渲染段委托基座 (BaseOverlay.java:263-269)
    pub fn render(&mut self, cv: &mut PixCanvas, font: &LoadedFont, aa: bool) {
        self.base.render(cv, font, aa);
    }
}

/// generateLines (Java :157-278): 按 ui_layout.cfg 开关过滤的 blkx 字段清单。
/// Lang 模板取 init_lang() 快照 (Java 读全局静态字段, 值同源 cur.properties)。
fn generate_lines(blkx: Option<&Blkx>, config: Option<&dyn ConfigProvider>) -> Vec<String> {
    let lang = Lang::init_lang();
    let mut lines: Vec<String> = Vec::new();
    let blkx = match blkx {
        None => {
            lines.push("FM Data Preview".to_string());
            lines.push("[No Data Loaded]".to_string());
            return lines;
        }
        Some(b) => b,
    };

    // ==================== FM Version (always shown) ====================
    // PORT: Java %s 收 null 字段打印 "null" (Formatter 行为), Option 展开对齐
    let fm_version = java_string_format(
        lang.b_fm_version,
        &[
            FmtArg::S(blkx.read_file_name.as_deref().unwrap_or("null")),
            FmtArg::S(blkx.version.as_deref().unwrap_or("null")),
        ],
    );
    add_lines(&mut lines, &fm_version);

    // ==================== Weight ====================
    if is_field_enabled(config, "showWeight") {
        let weight = java_string_format(
            lang.b_weight,
            &[FmtArg::F(blkx.emptyweight), FmtArg::F(blkx.maxfuelweight)],
        );
        add_lines(&mut lines, &weight);
    }

    // ==================== Critical Speed ====================
    if is_field_enabled(config, "showCritSpeed") {
        let crit_speed = java_string_format(
            lang.b_crit_speed,
            &[FmtArg::F(blkx.critical_speed * 3.6), FmtArg::F(blkx.vne)],
        );
        add_lines(&mut lines, &crit_speed);
    }

    // ==================== G-Load Limits (combined full/half fuel) ====================
    if is_field_enabled(config, "showGLoadLimits") {
        if let Some(raw) = blkx.raw_wing_crit_overload {
            // PORT: 与 getMaxAllowGloadForWeight 同式内联 (Java 源如此, 不收敛去重)
            let full_neg = 1.2 * (2.0 * raw[0] / (g * blkx.grossweight) + 1.0);
            let full_pos = 1.2 * (2.0 * raw[1] / (g * blkx.grossweight) - 1.0);
            let half_neg = 1.2 * (2.0 * raw[0] / (g * blkx.halfweight) + 1.0);
            let half_pos = 1.2 * (2.0 * raw[1] / (g * blkx.halfweight) - 1.0);
            let load_factor = java_string_format(
                lang.b_allow_load_factor,
                &[FmtArg::F(full_neg), FmtArg::F(full_pos), FmtArg::F(half_neg), FmtArg::F(half_pos)],
            );
            add_lines(&mut lines, &load_factor);
        }
    }

    // ==================== Flap Speed Limits ====================
    // PORT: Java AIOOBE (num > 6) ↔ Rust 索引 panic 同构 (§1 崩溃语义)。
    // 线程模型差异: Java 仅杀死本 overlay 的 run 轮询线程, Rust tick/draw 在
    // 唯一主循环上 — P5 组装必须对逐 overlay tick/render 包 catch_unwind
    // (PORTING §6 先例), 本 panic 与 java_string_format 错配 panic 均属此契约
    if is_field_enabled(config, "showFlapLimits") {
        if let Some(table) = blkx.flaps_destruction_ind_speed {
            for i in 0..blkx.flaps_destruction_num {
                let flap_limit = java_string_format(
                    lang.b_flap_restrict,
                    &[
                        FmtArg::D(i),
                        FmtArg::F(table[i as usize][0] * 100.0),
                        FmtArg::F(table[i as usize][1]),
                    ],
                );
                add_lines(&mut lines, &flap_limit);
            }
        }
    }

    // ==================== Control Surface Effectiveness (combined) ====================
    if is_field_enabled(config, "showControlEffectiveness") {
        let eff_speed = java_string_format(
            lang.b_eff_speed_and_power_loss,
            &[
                FmtArg::F(blkx.elav_eff),
                FmtArg::F(blkx.aileron_eff),
                FmtArg::F(blkx.rudder_eff),
                FmtArg::F(blkx.elav_power_loss),
                FmtArg::F(blkx.aileron_power_loss),
                FmtArg::F(blkx.rudder_power_loss),
            ],
        );
        add_lines(&mut lines, &eff_speed);
    }

    // ==================== Nitro (only if present) ====================
    if is_field_enabled(config, "showNitro") && blkx.nitro > 0.0 {
        let nitro = java_string_format(
            lang.b_nitro,
            &[FmtArg::F(blkx.nitro), FmtArg::F(blkx.nitro / (blkx.nitro_decr * 60.0))],
        );
        add_lines(&mut lines, &nitro);
    }

    // ==================== Heat Recovery ====================
    if is_field_enabled(config, "showHeatRecovery") {
        let heat_recovery =
            java_string_format(lang.b_average_heat_recovery, &[FmtArg::F(blkx.avg_eng_recovery_rate)]);
        add_lines(&mut lines, &heat_recovery);
    }

    // ==================== Max Lift Load ====================
    if is_field_enabled(config, "showMaxLiftLoad") {
        let max_lift_load = java_string_format(
            lang.b_max_lift_load350,
            &[
                FmtArg::F((blkx.no_flap_wll + 1.0) / 2.0),
                FmtArg::F((blkx.full_flap_wll + 1.0) / 2.0),
            ],
        );
        add_lines(&mut lines, &max_lift_load);
    }

    // ==================== Inertia ====================
    if is_field_enabled(config, "showInertia") {
        if let Some(m) = blkx.moment_of_inertia {
            if m.len() >= 3 {
                let inertia = java_string_format(
                    lang.b_inertia,
                    &[FmtArg::F(m[2]), FmtArg::F(m[0]), FmtArg::F(m[1])],
                );
                add_lines(&mut lines, &inertia);
            }
        }
    }

    // ==================== Lift Parameters ====================
    if is_field_enabled(config, "showLift") {
        let lift = java_string_format(
            lang.b_lift,
            &[
                FmtArg::F(blkx.a_wing),
                FmtArg::F(blkx.a_fuselage),
                FmtArg::F(blkx.no_flap_wll),
                FmtArg::F(blkx.full_flap_wll),
                FmtArg::F(blkx.oswalds_efficiency_number),
                FmtArg::F(blkx.aspect_ratio),
                FmtArg::F(blkx.swept_wing_angle),
            ],
        );
        add_lines(&mut lines, &lift);
    }

    // ==================== Drag Parameters ====================
    if is_field_enabled(config, "showDrag") {
        let drag = java_string_format(
            lang.b_drag,
            &[
                FmtArg::F(blkx.cd_s),
                FmtArg::F(blkx.cd_s / (blkx.halfweight / 1000.0)),
                FmtArg::F(blkx.ind_cd_f),
                FmtArg::F(blkx.halfweight * blkx.ind_cd_f),
                FmtArg::F(blkx.radiator_cd),
                FmtArg::F(blkx.oil_radiator_cd),
            ],
        );
        add_lines(&mut lines, &drag);
    }

    // ==================== FM Parts Sections ====================
    if is_field_enabled(config, "showNoFlapsWing") {
        add_fm_parts(&mut lines, &lang, blkx.no_flaps_wing.as_ref());
    }
    if is_field_enabled(config, "showFullFlapsWing") {
        add_fm_parts(&mut lines, &lang, blkx.full_flaps_wing.as_ref());
    }
    if is_field_enabled(config, "showFuselage") {
        add_fm_parts(&mut lines, &lang, blkx.fuselage.as_ref());
    }
    if is_field_enabled(config, "showFin") {
        add_fm_parts(&mut lines, &lang, blkx.fin.as_ref());
    }
    if is_field_enabled(config, "showStab") {
        add_fm_parts(&mut lines, &lang, blkx.stab.as_ref());
    }

    // If no fields are enabled or all filtered out, show a placeholder
    // PORT: fmVersion 恒入列 (:169) 使本分支在 Java 亦不可达, 保真保留
    if lines.is_empty() {
        lines.push("FM Data Preview".to_string());
        lines.push("[No Fields Enabled]".to_string());
    }

    lines
}

/// addFmParts (Java :283-290): 表头 + 4 数据行 (null 部件整段跳过)。
fn add_fm_parts(lines: &mut Vec<String>, lang: &Lang, p: Option<&FmParts>) {
    let p = match p {
        None => return, // Java: if (p == null) return;
        Some(p) => p,
    };
    add_lines(
        lines,
        &java_string_format(lang.b_fm_parts, &[FmtArg::S(p.name.as_deref().unwrap_or("null"))]),
    );
    add_lines(lines, &java_string_format(lang.b_cd_min, &[FmtArg::F(p.cd_min)]));
    add_lines(lines, &java_string_format(lang.b_cl0, &[FmtArg::F(p.cl0)]));
    add_lines(
        lines,
        &java_string_format(lang.b_ao_a_crit, &[FmtArg::F(p.aoa_crit_low), FmtArg::F(p.aoa_crit_high)]),
    );
    add_lines(
        lines,
        &java_string_format(lang.b_ao_a_crit_cl, &[FmtArg::F(p.cl_crit_low), FmtArg::F(p.cl_crit_high)]),
    );
}

/// addLines (Java :296-303): 按 \n 拆行, 逐行 trim, 跳过空行。
/// PORT: Java String.trim 只剥 ≤ U+0020 的字符 (§2.1), Rust `str::trim` 会
/// 多剥 U+3000 等全角空白 — 用 trim_matches 精确复刻 Java 语义。
fn add_lines(lines: &mut Vec<String>, formatted: &str) {
    for line in formatted.split('\n') {
        let trimmed = line.trim_matches(|c: char| c <= '\u{20}');
        if !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }
    }
}

/// isFieldEnabled (Java :309-316): config 缺失/键空 → 默认启用;
/// 否则 Boolean.parseBoolean (仅忽略大小写的 "true" 为真)。
fn is_field_enabled(config: Option<&dyn ConfigProvider>, field_key: &str) -> bool {
    match config {
        None => true,
        Some(c) => match c.get_config(field_key) {
            None => true,
            Some(v) if v.is_empty() => true,
            Some(v) => v.eq_ignore_ascii_case("true"),
        },
    }
}

// ---------------------------------------------------------------------------
// 测试: 格式化 oracle / ControlSurfaces 几何+像素 / FMUnpackedData 清单+门控
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    const BOLD: &str = "../../../fonts/sarasa-mono-sc-bold.ttf";
    const REGULAR: &str = "../../../fonts/sarasa-mono-sc-regular.ttf";

    fn font(path: &str, size: i32) -> LoadedFont {
        LoadedFont::new(std::path::Path::new(path), size).unwrap()
    }

    /// 读预乘 RGBA 像素 (与 overlay_list/render2d 测试同约定)
    fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
        let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
        [d[0], d[1], d[2], d[3]]
    }

    /// 直通色 → tiny-skia 预乘取整 ((c*a+127)/255), 断言基准用
    fn premul(c: [u8; 4]) -> [u8; 4] {
        [
            ((c[0] as u32 * c[3] as u32 + 127) / 255) as u8,
            ((c[1] as u32 * c[3] as u32 + 127) / 255) as u8,
            ((c[2] as u32 * c[3] as u32 + 127) / 255) as u8,
            c[3],
        ]
    }

    // ---- java_format_f / java_string_format: Java 8 oracle 对拍 ----

    /// HALF_UP on 最短往返十进制 (Java Formatter 语义) vs Rust 半偶的判别值
    #[test]
    fn java_format_f_half_up_oracle() {
        // String.format("%.1f", 5.25) = "5.3" (Rust {:.1} 半偶 → "5.2")
        assert_eq!(java_format_f(5.25, 1), "5.3");
        // String.format("%.2f", 2.675) = "2.68" (Rust → "2.67")
        assert_eq!(java_format_f(2.675, 2), "2.68");
        // String.format("%.0f", 0.5) = "1" / (2.5) = "3" (Rust → 0 / 2)
        assert_eq!(java_format_f(0.5, 0), "1");
        assert_eq!(java_format_f(2.5, 0), "3");
        // 最短表示 2.675 的 %.1f = "2.7" (vm-core java_format_f1 文档 oracle)
        assert_eq!(java_format_f(2.675, 1), "2.7");
    }

    /// 常规/负数/补零/NaN/-0.0/整域
    #[test]
    fn java_format_f_domains() {
        assert_eq!(java_format_f(3050.0, 1), "3050.0");
        assert_eq!(java_format_f(-8.4, 1), "-8.4");
        assert_eq!(java_format_f(9.0, 2), "9.00");
        assert_eq!(java_format_f(0.105, 3), "0.105");
        assert_eq!(java_format_f(-0.04, 1), "-0.0", "负号保留 (Java Formatter)");
        assert_eq!(java_format_f(f64::NAN, 1), "NaN");
        assert_eq!(java_format_f(f64::INFINITY, 0), "Infinity");
        // 巨整数域: 1e26 → 全整数 + ".0"
        assert_eq!(java_format_f(1e26, 1), "100000000000000000000000000.0");
        // 小数 |x|<1 的 prec=0
        assert_eq!(java_format_f(0.49999999999999994, 0), "0");
    }

    /// java_string_format: %s/%d/%.Nf 顺序展开 + %% 字面 (bFlapRestrict 模板)
    #[test]
    fn java_string_format_engine() {
        let t = "襟翼限速(km/h)%d: %.0f%% / %.0f\n";
        assert_eq!(
            java_string_format(t, &[FmtArg::D(1), FmtArg::F(95.0), FmtArg::F(640.0)]),
            "襟翼限速(km/h)1: 95% / 640\n"
        );
        assert_eq!(
            java_string_format("FM文件: %s - %s", &[FmtArg::S("a"), FmtArg::S("b")]),
            "FM文件: a - b"
        );
        // %s 收 null 字段 → "null"
        assert_eq!(java_string_format("V: %s", &[FmtArg::S("null")]), "V: null");
    }

    /// 模板/实参错配 → panic (Java UnknownFormatConversionException /
    /// MissingFormatArgumentException 的崩溃语义)
    #[test]
    #[should_panic]
    fn java_string_format_missing_arg_panics() {
        let _ = java_string_format("%s %s", &[FmtArg::S("a")]);
    }

    /// %d 位点收浮点实参 → panic (Java IllegalFormatConversionException;
    /// 曾静默 `v as i64` 输出与 doc "两语言同为崩溃语义" 矛盾, 已对齐)
    #[test]
    #[should_panic(expected = "IllegalFormatConversionException")]
    fn java_string_format_f_at_d_panics() {
        let _ = java_string_format("%d", &[FmtArg::F(1.5)]);
    }

    /// %d 位点收字符串实参 → panic (Java 同抛 IllegalFormatConversionException)
    #[test]
    #[should_panic(expected = "IllegalFormatConversionException")]
    fn java_string_format_s_at_d_panics() {
        let _ = java_string_format("%d", &[FmtArg::S("x")]);
    }

    /// addLines 的 Java trim 语义: 只剥 ≤ U+0020, 全角空格 U+3000 保留
    /// (Rust `str::trim` 会多剥一层 — 域内不可达, 本测试锁定复刻边界)
    #[test]
    fn add_lines_java_trim_semantics() {
        let mut lines = Vec::new();
        add_lines(&mut lines, "a\u{3000}  \nb\u{3000}\n  \t\n");
        assert_eq!(lines, vec!["a\u{3000}".to_string(), "b\u{3000}".to_string()]);
    }

    // ---- ControlSurfacesOverlay ----

    /// init/reinitConfig 几何公式 (Java :225-271, :107-111):
    /// fontSize=24 → width=144, rudderValPix=108, twidth=240, theight=180,
    /// locate=4, stroke=2; enableAxisEdge 加 sw=10
    #[test]
    fn control_surfaces_geometry() {
        let mut ov = ControlSurfacesOverlay::new();
        ov.init(0, 1.0, false, 30, 40, true);
        assert_eq!(ov.font_size, 24);
        assert_eq!(ov.label_font_size, 12, "Math.round(24/2.0f)");
        assert_eq!((ov.width, ov.height), (144, 144));
        assert_eq!(ov.rudder_val_pix, 108, "(50+100)*144/200 初值");
        assert_eq!((ov.content_width, ov.content_height), (240, 180), "(int)(144+96)/(int)(144+36)");
        assert_eq!(ov.shade_width, 0);
        assert_eq!((ov.total_width, ov.total_height), (240, 180));
        assert_eq!((ov.px, ov.py), (72, 72));
        assert_eq!((ov.locate_size, ov.stroke_size), (4, 2));
        assert_eq!((ov.lx, ov.ly), (30, 40), "OverlaySettings 坐标透传");
        assert!(ov.has_service, "游戏模式");
        // 初值 50 (Java :91-94)
        assert_eq!(ov.elevator_num, "50");
        assert_eq!(ov.wing_sweep_num, "50");

        // enableAxisEdge: sw=10 外扩 (Java :250-256)
        let mut ov2 = ControlSurfacesOverlay::new();
        ov2.init(0, 1.0, true, 0, 0, false);
        assert_eq!(ov2.shade_width, 10);
        assert_eq!((ov2.total_width, ov2.total_height), (260, 200));
        assert!(!ov2.has_service, "preview: s == null");

        // fontadd=-6 → fontSize 18, width 108, twidth (int)(108+72)=180
        let mut ov3 = ControlSurfacesOverlay::new();
        ov3.init(-6, 1.0, false, 0, 0, false);
        assert_eq!(ov3.font_size, 18);
        assert_eq!(ov3.width, 108);
        assert_eq!(ov3.content_width, 180);
        assert_eq!(ov3.content_height, (108.0 + 27.0) as i32, "135");

        // 奇数字号: fontSize 25 (dpi 校准) → label = Math.round(12.5f) = 13
        let mut ov4 = ControlSurfacesOverlay::new();
        ov4.reinit_config(1, 1.0, false, 0, 0);
        assert_eq!(ov4.font_size, 25);
        assert_eq!(ov4.label_font_size, 13);
    }

    /// onFlightData: 50ms 节流 + preview 不更新数据 + 游标/条位换算 (Java :280-312)
    #[test]
    fn control_surfaces_throttle_and_mapping() {
        let mut ov = ControlSurfacesOverlay::new();
        ov.init(0, 1.0, false, 0, 0, true);

        // 首事件: lastRefreshTime=0 → 0-0 < 50 恒真 → 被跳过? Java 同:
        // 初值 0, now=0 时 0-0=0 < 50 → skip。用 now=100 起测
        assert!(!ov.on_flight_data(0, 0.0, 0.0, 0.0, 0.0, false), "0-0 < 50 跳过 (Java 同)");
        assert!(ov.on_flight_data(100, -100.0, 100.0, 0.0, 0.85, true));
        assert_eq!((ov.px, ov.py), (0, 144), "副翼 -100 → 左缘; 升降舵 100 → 底缘");
        assert_eq!(ov.rudder_val_pix, 72, "方向舵 0 → 中位");
        assert_eq!(ov.wing_sweep_num, "85", "可变翼 0.85 → 85 (isWingSweepValid)");
        assert_eq!(ov.elevator_num, "100");
        assert_eq!(ov.aileron_num, "-100");

        // 节流: +30ms 跳过, +50ms 放行
        assert!(!ov.on_flight_data(130, 0.0, 0.0, 0.0, 0.0, false));
        assert!(ov.on_flight_data(150, 50.7, -25.9, 100.0, -65535.0, false));
        // (int) 截断向零: 50.7→50, -25.9→-25
        assert_eq!((ov.px, ov.py), ((100 + 50) * 144 / 200, (100 - 25) * 144 / 200));
        assert_eq!(ov.rudder_val_pix, 144, "满舵 → 全宽");
        assert_eq!(ov.aileron_num, "50");
        assert_eq!(ov.elevator_num, "-25");
        assert_eq!(ov.wing_sweep_num, "0", "wsweep -65535 无效标记 → 0");

        // preview (s == null): 返回 true (repaint 恒调度) 但数据保持
        let mut pv = ControlSurfacesOverlay::new();
        pv.init_preview(0, 1.0, false, 0, 0);
        assert!(pv.on_flight_data(100, -100.0, -100.0, -100.0, 0.5, true));
        assert_eq!((pv.px, pv.py), (72, 72), "初值 50 → 中心");
        assert_eq!(pv.rudder_val_pix, 108, "初值条位");
        assert_eq!(pv.rudder_num, "50");
    }

    /// draw 像素: 边框/十字影子+主十字/横条+游标 (alpha=255 语义区, 预乘=直通)
    #[test]
    fn control_surfaces_draw_pixels() {
        let mut ov = ControlSurfacesOverlay::new();
        ov.init(0, 1.0, false, 0, 0, true);
        let f_num = font(BOLD, 24);
        let f_label = font(BOLD, 12);
        let f_unit = font(REGULAR, 12);
        let fonts = CsFonts { num: &f_num, label: &f_label, unit: &f_unit };
        let mut cv = PixCanvas::new(240, 180).unwrap();
        ov.draw(&mut cv, &fonts, false);

        // 边框 (BasicStroke(1), colorShadeShape): 四角 — Java 4 条 drawLine 各自独立
        // 描边, 角点被两条线 SrcOver 叠两次: 42 + 213·42/255 ≈ 77 (Java 同值)
        let corner_blend = [0u8, 0, 0, 77];
        for (x, y) in [(0, 0), (143, 0), (0, 143), (143, 143)] {
            assert_eq!(px(&cv, x, y), corner_blend, "边框角双叠 ({x},{y})");
        }
        assert_eq!(px(&cv, 0, 72), COLOR_SHADE_SHAPE, "左边框中点");
        // 边框外无字
        assert_eq!(px(&cv, 60, 60), [0, 0, 0, 0], "十字区中心空");

        // 主十字 (colorNum, 中心 (72,72) 偏移 -1, 线宽 2): 六条独立 drawLine 的
        // 描边互相交叠 — 断言取**单笔画覆盖**点 (Java 同样叠出混合 alpha):
        // 主横线 y=71 (行 70/71 实心, 臂 x∈[68,73]); 主竖线 x=71 (列 70/71 实心)
        assert_eq!(px(&cv, 69, 70), premul(COLOR_NUM), "主横线单覆盖点 (69,70)");
        assert_eq!(px(&cv, 70, 69), premul(COLOR_NUM), "主竖线单覆盖点 (70,69)");
        // 主线交叠中心 (行70/71 × 列70/71): 240+240·15/255 → 饱和 255
        assert_eq!(px(&cv, 70, 70)[3], 255, "主十字中心核心双叠饱和");
        // 影子十字 (colorShadeShape, 轴 y=72/x=72, 偏移 +1): 在主线臂端外侧露出 —
        // 影横臂延至 x=74 (主横臂 x≤73), 影竖臂延至 y=74 (主竖臂 y≤73) → 单覆盖点
        assert_eq!(px(&cv, 74, 71), COLOR_SHADE_SHAPE, "影横臂右尖端 (74,71)");
        assert_eq!(px(&cv, 71, 74), COLOR_SHADE_SHAPE, "影竖臂下尖端 (71,74)");
        // 影子自身交点 (72,72) 双叠: 42+213·42/255 ≈ 77 (Java 同)
        assert_eq!(px(&cv, 72, 72), [0, 0, 0, 77], "影子交点双叠");

        // 底部方向舵横条 (y=height=144 起, 高 12): 外框阴影 + 内填 colorNum。
        // 条顶左角 (0,144) 与 locater 左边框线端点 (drawLine(0,0,0,r), r=144
        // 含端点) 重叠 → SrcOver 双叠 77 (Java 同序同叠); 条底右角单覆盖
        assert_eq!(px(&cv, 0, 144), [0, 0, 0, 77], "条顶左角 (与边框线端点双叠)");
        assert_eq!(px(&cv, 143, 155), COLOR_SHADE_SHAPE, "条底边框右角 (144+12-1)");
        assert_eq!(px(&cv, 2, 150), premul(COLOR_NUM), "条内填充 (初值 108 宽)");
        assert_eq!(px(&cv, 105, 150), premul(COLOR_NUM), "条内填充右段 (x ≤ 106)");
        assert_eq!(px(&cv, 109, 150), [0, 0, 0, 0], "游标右缘外空 (x=109)");

        // 游标竖线 (x=106..108, y=144..167): 阴影框 + colorLabel 中心 1px。
        // 顶行与条顶边框重叠 → 双叠 77; 中心列 (x=107) 从 y=145 起, 底段无条遮挡
        assert_eq!(px(&cv, 106, 144), [0, 0, 0, 77], "游标左上角 (与条顶边框双叠)");
        assert_eq!(px(&cv, 106, 160), COLOR_SHADE_SHAPE, "游标左框单覆盖 (条外段)");
        assert_eq!(px(&cv, 107, 160), premul(COLOR_LABEL), "游标中心 colorLabel (条外段)");
        assert_eq!(px(&cv, 107, 166), premul(COLOR_LABEL), "游标下端 (144+24-2)");
    }

    /// draw 文本带: 4 行 BOS 标签 (数字 x=width 基线 24; 标签/单位 x=width+54)
    /// 与方向舵数字 (x=rudderValPix, 基线 168) 有字形像素落点
    #[test]
    fn control_surfaces_draw_text_zones() {
        let mut ov = ControlSurfacesOverlay::new();
        ov.init(0, 1.0, false, 0, 0, true);
        let f_num = font(BOLD, 24);
        let f_label = font(BOLD, 12);
        let f_unit = font(REGULAR, 12);
        let fonts = CsFonts { num: &f_num, label: &f_label, unit: &f_unit };
        let mut cv = PixCanvas::new(240, 180).unwrap();
        ov.draw(&mut cv, &fonts, false);

        let has_ink = |x0: i32, x1: i32, y0: i32, y1: i32| -> bool {
            (x0..x1).any(|x| (y0..y1).any(|y| px(&cv, x, y)[3] > 0))
        };
        // 数字 "50" @ (144, 24 基线), fontNum 24 — lwidth=(9*24)>>2=54
        assert!(has_ink(144, 180, 4, 26), "首行数字带 (升降舵 50)");
        // 标签名 "升降舵" @ (198, 12 基线) + 单位 "%" @ (198, 24 基线)
        assert!(has_ink(198, 240, 2, 14), "首行标签名带");
        assert!(has_ink(198, 216, 14, 26), "首行单位带");
        // 第四行 (可变翼) dy = 12 + 3*36 = 120 基线
        assert!(has_ink(198, 240, 110, 132), "第四行标签带 (dy=120)");
        // 方向舵数字 "50" @ (108, 168 基线) fontLabel 12
        assert!(has_ink(108, 132, 156, 170), "条值数字带");
    }

    // ---- FmUnpackedDataOverlay ----

    /// 测试用 ConfigProvider stub (HashMap + RefCell, 与 vm-core config_provider 测试同式)
    struct MapConfig {
        values: RefCell<HashMap<String, String>>,
    }

    impl MapConfig {
        fn new() -> Self {
            MapConfig { values: RefCell::new(HashMap::new()) }
        }
        fn set(&self, k: &str, v: &str) {
            self.values.borrow_mut().insert(k.to_string(), v.to_string());
        }
    }

    impl ConfigProvider for MapConfig {
        fn get_config(&self, key: &str) -> Option<String> {
            self.values.borrow().get(key).cloned()
        }
        fn set_config(&self, key: &str, value: &str) {
            self.values.borrow_mut().insert(key.to_string(), value.to_string());
        }
        fn is_field_disabled(&self, _key: &str) -> bool {
            false
        }
    }

    /// 全字段齐备的测试 blkx (期望值 = Java 8 oracle 手算, HALF_UP 判别值混入)
    fn full_blkx() -> Blkx {
        let mut b = Blkx::default();
        b.read_file_name = Some("spitfire_mk24".to_string());
        b.version = Some("2.35.0.9".to_string());
        b.emptyweight = 3050.0;
        b.maxfuelweight = 780.45; // %.1f HALF_UP → "780.5"
        b.critical_speed = 230.0; // ×3.6 = 828.000...01 → "828"
        b.vne = 1050.0;
        b.raw_wing_crit_overload = Some([-196000.0, 441000.0]);
        b.grossweight = 5000.0; // full: 1.2·(2·raw/(g·w)∓1) → (-8.4, 20.4)
        b.halfweight = 4000.0; // half → (-10.8, 25.8)
        b.flaps_destruction_num = 2;
        let mut flaps = [[0.0; 2]; 6];
        flaps[0] = [0.0, 640.0];
        flaps[1] = [0.95, 520.0]; // ×100 = 94.99... → %.0f → "95"
        b.flaps_destruction_ind_speed = Some(flaps);
        b.elav_eff = 580.0;
        b.aileron_eff = 640.0;
        b.rudder_eff = 700.0;
        b.elav_power_loss = 0.25; // %.1f HALF_UP → "0.3"
        b.aileron_power_loss = 0.35; // → "0.4"
        b.rudder_power_loss = 0.45; // → "0.5"
        b.nitro = 120.0;
        b.nitro_decr = 2.0; // 120/(2·60) = 1.0
        b.avg_eng_recovery_rate = 3.25; // %.1f HALF_UP → "3.3"
        b.no_flap_wll = 9.0; // (9+1)/2 = 5.0
        b.full_flap_wll = 13.0; // 7.0
        b.moment_of_inertia = Some([12000.0, 25000.0, 8000.0]); // [P:m[2], R:m[0], Y:m[1]]
        b.a_wing = 25.8;
        b.a_fuselage = 5.4;
        b.oswalds_efficiency_number = 0.75;
        b.aspect_ratio = 6.0;
        b.swept_wing_angle = 0.0;
        b.cd_s = 0.42;
        b.ind_cd_f = 0.003; // 4000·0.003 ≈ 12.000...002 → "12"
        b.radiator_cd = 0.021;
        b.oil_radiator_cd = 0.017;
        let mut wing = FmParts::default();
        wing.name = Some("机翼 无襟翼".to_string());
        wing.cd_min = 0.0285; // %.3f HALF_UP → "0.029"
        wing.cl0 = 0.05;
        wing.aoa_crit_low = -14.4;
        wing.aoa_crit_high = 18.6;
        wing.cl_crit_low = -1.15;
        wing.cl_crit_high = 1.55;
        b.no_flaps_wing = Some(wing.clone());
        let mut ff = FmParts::default();
        ff.name = Some("机翼 全襟翼".to_string());
        ff.cd_min = 0.0331;
        ff.cl0 = 0.12;
        ff.aoa_crit_low = -13.1;
        ff.aoa_crit_high = 20.2;
        ff.cl_crit_low = -1.35;
        ff.cl_crit_high = 1.85;
        b.full_flaps_wing = Some(ff);
        let mut fuse = FmParts::default();
        fuse.name = Some("机身".to_string());
        fuse.cd_min = 0.0151;
        fuse.cl0 = 0.02;
        fuse.aoa_crit_low = -27.9;
        fuse.aoa_crit_high = 27.9;
        fuse.cl_crit_low = -0.41;
        fuse.cl_crit_high = 0.49;
        b.fuselage = Some(fuse);
        let mut fin = FmParts::default();
        fin.name = Some("垂尾".to_string());
        fin.cd_min = 0.0081;
        fin.cl0 = 0.0;
        fin.aoa_crit_low = -16.2;
        fin.aoa_crit_high = 16.2;
        fin.cl_crit_low = -0.62;
        fin.cl_crit_high = 0.62;
        b.fin = Some(fin);
        let mut stab = FmParts::default();
        stab.name = Some("平尾".to_string());
        stab.cd_min = 0.0062;
        stab.cl0 = -0.06;
        stab.aoa_crit_low = -15.5;
        stab.aoa_crit_high = 15.5;
        stab.cl_crit_low = -0.55;
        stab.cl_crit_high = 0.55;
        b.stab = Some(stab);
        b
    }

    /// generateLines 全量 (config None → 全启用) 的逐行 oracle
    #[test]
    fn generate_lines_full_field_list() {
        let lines = generate_lines(Some(&full_blkx()), None);
        let expect_prefix = [
            "FM文件: spitfire_mk24 - 2.35.0.9",
            "空重(kg): 3050.0",
            "最大燃油重量(kg): 780.5", // %.1f HALF_UP 判别
            "临界速度(km/h): [828, 1050]",
            "允许过载(满/半油): [-8.4, 20.4], [-10.8, 25.8]",
            "襟翼限速(km/h)0: 0% / 640",
            "襟翼限速(km/h)1: 95% / 520",
            "三舵有效速度(km/h): [ 升降580, 副翼640, 方向700 ]",
            "三舵锁舵因数: [ 升降0.3, 副翼0.4, 方向0.5 ]", // %.1f HALF_UP ×3
            "加力(kg)/时限(分钟): 120.0 / 1.0",
            "平均耐热条恢复速率: 3.3", // %.1f HALF_UP 判别
            "千米最大升力过载: 5.0 / 7.0(襟) @ 350IAS",
            "三轴转动惯量: [ P: 8000, R: 12000, Y: 25000 ]",
            "主升力面积: 25.8机翼, 5.4机身",
            "主升力面积因数载荷: 9.00 / 13.00(襟)",
            "翼展效率: 0.75 展弦比: 6.0 后掠角: 0.0",
            "主阻力面积因数及加速度系数: 0.42 / 0.105",
            "诱导阻力因数及加速度系数: 0.003 / 12",
            "散热/油冷器阻力系数: 0.021 / 0.017",
        ];
        assert!(lines.len() >= expect_prefix.len() + 25, "全字段行数 ≥ 44, 实 {}", lines.len());
        for (i, want) in expect_prefix.iter().enumerate() {
            assert_eq!(&lines[i], want, "第 {i} 行");
        }
        // FM 器件段 (addFmParts ×5 段, 每段表头+4 行)
        assert_eq!(lines[19], "------fm器件 机翼 无襟翼------");
        assert_eq!(lines[20], "零升阻力系数: 0.029", "%.3f HALF_UP 判别");
        assert_eq!(lines[21], "零攻角升力: 0.050");
        assert_eq!(lines[22], "临界攻角: [-14.4, 18.6]");
        assert_eq!(lines[23], "临界攻角升力系数: [-1.15, 1.55]");
        let idx = lines
            .iter()
            .position(|l| l == "------fm器件 平尾------")
            .expect("第五段 (Stab)");
        assert_eq!(&lines[idx + 1..idx + 5], [
            "零升阻力系数: 0.006",
            "零攻角升力: -0.060",
            "临界攻角: [-15.5, 15.5]",
            "临界攻角升力系数: [-0.55, 0.55]",
        ]);
    }

    /// 无数据 / null 字段 ("null" 文本) / 空白模板行裁剪
    #[test]
    fn generate_lines_no_data_and_null_fields() {
        assert_eq!(
            generate_lines(None, None),
            vec!["FM Data Preview".to_string(), "[No Data Loaded]".to_string()]
        );
        // readFileName/version 为 null → %s 打 "null" (Java Formatter 行为)
        let mut b = Blkx::default();
        b.emptyweight = 1.0;
        let lines = generate_lines(Some(&b), None);
        assert_eq!(lines[0], "FM文件: null - null");
    }

    /// 字段开关: false 关 / 空串与缺失默认开 / parseBoolean 仅 "true" (忽略大小写)
    #[test]
    fn generate_lines_field_switches() {
        let cfg = MapConfig::new();
        cfg.set("showWeight", "false");
        cfg.set("showCritSpeed", "FALSE"); // parseBoolean 忽略大小写 → false
        cfg.set("showLift", ""); // 空串 → 默认启用
        cfg.set("showDrag", "yes"); // 非 "true" → false
        let lines = generate_lines(Some(&full_blkx()), Some(&cfg));
        assert!(!lines.iter().any(|l| l.starts_with("空重")), "showWeight=false 关");
        assert!(!lines.iter().any(|l| l.starts_with("临界速度")), "FALSE (忽略大小写) 关");
        assert!(lines.iter().any(|l| l.starts_with("主升力面积")), "空串默认开");
        assert!(!lines.iter().any(|l| l.starts_with("主阻力面积")), "yes → false");
        assert!(lines.iter().any(|l| l.starts_with("加力")), "其余段不受影响");
        // fmVersion 恒显 → "[No Fields Enabled]" 占位不可达 (Java 同)
        assert!(lines.iter().any(|l| l.starts_with("FM文件")));
    }

    /// nitro ≤ 0 段隐藏 (Java :212 blkx.nitro > 0 门控)
    #[test]
    fn generate_lines_nitro_gate() {
        let mut b = full_blkx();
        b.nitro = 0.0;
        let lines = generate_lines(Some(&b), None);
        assert!(!lines.iter().any(|l| l.contains("加力")));
        b.nitro = 60.0;
        b.nitro_decr = 1.0;
        let lines = generate_lines(Some(&b), None);
        assert!(lines.iter().any(|l| l == "加力(kg)/时限(分钟): 60.0 / 1.0"));
    }

    /// 表头谓词 (Java :87/:118 startsWith 覆盖默认 contains) + 斑马交互
    #[test]
    fn fm_overlay_header_matcher() {
        let f = font(REGULAR, 14);
        let mut ov = FmUnpackedDataOverlay::new(1440, 1.0, 12);
        ov.init(None, &f);
        assert!(ov.base.zebra.is_header("FM文件: x"));
        assert!(ov.base.zebra.is_header("------fm器件: 机翼"));
        assert!(!ov.base.zebra.is_header("prefix FM文件"), "startsWith 不含中缀");
        assert!(!ov.base.zebra.is_header("含 fm器件 中缀的行"), "默认 contains 已被覆盖");
    }

    /// 游戏模式门控: 初始隐藏不取数; toggle 后取数并脏; 同数据不脏 (Java :67/:318)
    #[test]
    fn fm_overlay_toggle_visibility_gating() {
        let f = font(REGULAR, 14);
        let mut ov = FmUnpackedDataOverlay::new(1440, 1.0, 12);
        ov.init(None, &f);
        assert!(!ov.is_visible_now(), "游戏模式初始隐藏");
        assert!(!ov.tick(), "隐藏分支不取数不显示");
        assert!(!ov.base.window_visible);

        ov.toggle();
        assert!(ov.is_visible_now());
        ov.reload_fm_data(Some(Arc::new(full_blkx())));
        assert!(ov.tick(), "首帧脏 (lastData=null → 行清单入基座)");
        assert!(ov.base.window_visible);
        assert!(!ov.tick(), "同数据 equals → 不脏");

        ov.toggle();
        assert!(!ov.tick(), "再隐藏 → 不取数");
        assert!(!ov.base.window_visible);
    }

    /// reload/reinit 换 blkx → 行清单随脏检查刷新; None → 占位 (Java :130-151)
    #[test]
    fn fm_overlay_reload_and_reinit() {
        let f = font(REGULAR, 14);
        let mut ov = FmUnpackedDataOverlay::new(1440, 1.0, 12);
        ov.init(None, &f);
        ov.toggle(); // 可见化以走取数分支

        // last_data 为基座私有字段, 内容经 generate_lines() 断言、刷新经脏标志断言
        ov.reload_fm_data(Some(Arc::new(full_blkx())));
        assert!(ov.tick());
        assert!(ov.generate_lines()[0].starts_with("FM文件: spitfire"));

        ov.reload_fm_data(None);
        assert!(ov.tick(), "清单变化 ([No Data Loaded]) → 脏");
        assert_eq!(
            ov.generate_lines(),
            vec!["FM Data Preview".to_string(), "[No Data Loaded]".to_string()]
        );
        assert!(!ov.tick(), "同清单 → 不脏");

        // reinit_config: FMManager.current() 快照注入 (Java :146-147)
        let mut b = Blkx::default();
        b.read_file_name = Some("tempest_mk5".to_string());
        ov.reinit_config(Some(Arc::new(b)), &f);
        assert!(ov.tick(), "reinit 换机 → 清单变化 → 脏");
        assert!(ov.generate_lines()[0].starts_with("FM文件: tempest_mk5"));
        // 预览模式绕过可见门控 (BaseOverlay.run:235 isPreview ||)
        let mut pv = FmUnpackedDataOverlay::new(1440, 1.0, 12);
        pv.init_preview(None, &f);
        assert!(pv.is_visible_now());
        assert!(pv.base.is_preview);
        assert!(pv.tick(), "preview 隐藏语义下仍取数");
    }

    /// QA 批十终检: 五个 overlay (field1 三件 + 本文件两件) 的内容渲染函数经
    /// OverlaySpec 装入 OverlayHost 走全链 (register → open_all → render_tick →
    /// present → close_all)。field2 两组件的完整组装 (动态窗口高/逐条目可见性/
    /// 预览闭包工厂) 按模块头 PORT 注留组装层, 此处只证 host 的 render 闭包通道
    /// (RenderFn) 对二者同样可用 — Java 侧五件同经 OverlayManager 注册装载。
    /// 窗口生命周期语义 (销毁序/分流/拖拽) 由 host.rs 自有测试覆盖, 此处 mock 只记
    /// present 次数并断言缓冲尺寸。
    #[test]
    fn five_overlays_mount_into_overlay_host() {
        use crate::host::{OverlayHost, OverlaySpec};
        use crate::platform::{OverlayEvent, OverlayWindow, WindowConfig};
        use std::cell::Cell;
        use std::rc::Rc;

        struct MiniWin {
            presents: Rc<Cell<u32>>,
            size: (i32, i32),
        }
        impl OverlayWindow for MiniWin {
            fn present(&mut self, buf: &[u8]) -> Result<(), String> {
                assert_eq!(buf.len(), (self.size.0 * self.size.1 * 4) as usize);
                self.presents.set(self.presents.get() + 1);
                Ok(())
            }
            fn set_position(&mut self, _x: i32, _y: i32) {}
            fn position(&self) -> (i32, i32) {
                (0, 0)
            }
            fn set_click_through(&mut self, _on: bool) {}
            fn poll_event(&mut self) -> Option<OverlayEvent> {
                None
            }
            fn screen_size(&self) -> (i32, i32) {
                (1920, 1080)
            }
        }

        let presents = Rc::new(Cell::new(0u32));
        let p_counter = Rc::clone(&presents);
        let mut host = OverlayHost::with_factory(Box::new(move |cfg: WindowConfig| {
            let size = (cfg.width, cfg.height);
            Ok(Box::new(MiniWin { presents: Rc::clone(&p_counter), size }) as Box<dyn OverlayWindow>)
        }));
        let fonts_dir = std::path::Path::new("../../../fonts");
        let lang = Lang::init_lang();

        // ① PowerInfo (Java 注册键 engineInfoSwitch, field1 预览工厂)
        host.register(crate::overlays_field1::power_info_preview_spec(fonts_dir, 0, 1).unwrap());
        // ② EngineControl (enableEngineControl)
        host.register(
            crate::overlays_field1::engine_control_preview_spec(fonts_dir, &lang, 0, 1.0).unwrap(),
        );
        // ③ GearFlaps (enablegearAndFlaps)
        host.register(
            crate::overlays_field1::gear_flaps_preview_spec(fonts_dir, 0, 1.0, false).unwrap(),
        );
        // ④ ControlSurfaces (Java 键 enableAxis): draw 内容函数手工包进 render 闭包
        //    (P5 组装契约 (c) 预览工厂留组装层, 此处同形态验证)
        let mut cs = ControlSurfacesOverlay::new();
        cs.init_preview(0, 1.0, false, 0, 0);
        let f_num = font(BOLD, cs.font_size);
        let f_label = font(BOLD, cs.label_font_size);
        let f_unit = font(REGULAR, cs.label_font_size);
        let (cw, ch) = (cs.total_width, cs.total_height);
        host.register(OverlaySpec {
            id: "enableAxis".into(),
            config_key: "enableAxis".into(),
            width: cw,
            height: ch,
            render: Box::new(move |cv| {
                let fonts = CsFonts { num: &f_num, label: &f_label, unit: &f_unit };
                cs.draw(cv, &fonts, true);
            }),
        });
        // ⑤ FMUnpackedData (Java 键 enableFMPrint): render(&mut) 同通道
        let f_list = font(REGULAR, 14);
        let mut fm = FmUnpackedDataOverlay::new(1440, 1.0, 12);
        fm.init_preview(None, &f_list);
        assert!(fm.tick(), "preview 首帧取数 (占位两行清单)");
        let (fw, fh) = (fm.base.width, fm.base.height);
        assert!(fw > 0 && fh > 0);
        host.register(OverlaySpec {
            id: "enableFMPrint".into(),
            config_key: "enableFMPrint".into(),
            width: fw,
            height: fh,
            render: Box::new(move |cv| {
                fm.render(cv, &f_list, true);
            }),
        });

        // 全链: 开 → 首帧五窗各 present 一次 (尺寸逐窗断言) → 静态内容脏检查抑制
        // → close_all 后槽位全空不再渲染
        host.open_all().unwrap();
        assert_eq!(host.active_ids().len(), 5, "五个 overlay 全部装载打开");
        host.render_tick().unwrap();
        assert_eq!(presents.get(), 5, "首帧五窗各一次 present");
        host.render_tick().unwrap();
        assert_eq!(presents.get(), 5, "静态预览内容: 脏检查抑制");
        host.close_all();
        host.render_tick().unwrap();
        assert_eq!(presents.get(), 5, "槽位全空: 不再 present");
        assert!(host.active_ids().is_empty());
    }

    /// live 工厂: 尺寸 = 内容区 (fontAdd 0/dpi 1 → fs=24, w=144, twidth=240,
    /// theight=180), has_service 初值 false (init_preview), 喂入侧置 true 后
    /// on_flight_data 才推数据; render 闭包共享句柄画到新值
    #[test]
    fn control_surfaces_overlay_spec_shared_state() {
        let fonts_dir = std::path::Path::new("../../../fonts");
        let (h, mut spec) = control_surfaces_overlay_spec(fonts_dir, 0, 1.0, false).unwrap();
        assert_eq!((spec.width, spec.height), (240, 180), "内容区尺寸 (无 sw 边框)");
        assert_eq!((spec.id.as_str(), spec.config_key.as_str()), ("enableAxis", "enableAxis"));
        // 初值 px = width/2 = 72 (游标居中, Java init :108)
        assert_eq!(h.borrow().px, 72);
        // has_service=false: 数据不更新 (preview 形态)
        assert!(h.borrow_mut().on_flight_data(100, 100.0, 0.0, 0.0, 0.0, false));
        assert_eq!(h.borrow().px, 72, "preview 门控: 数据保持");
        // 游戏形态 (喂入方切换 has_service, app_shell 承载): aileron=100 → px=144
        h.borrow_mut().has_service = true;
        assert!(h.borrow_mut().on_flight_data(200, 100.0, 0.0, 0.0, 0.0, false));
        assert_eq!(h.borrow().px, 144);
        assert_eq!(h.borrow().aileron_num, "100");
        let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
        (spec.render)(&mut cv);
        assert!(cv.pixmap().data().iter().any(|&b| b != 0));
    }
}
