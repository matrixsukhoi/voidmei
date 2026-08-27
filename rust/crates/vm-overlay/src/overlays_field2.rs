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

use crate::global_colors::{aa, colors};
use crate::font::LoadedFont;

use crate::host::{OverlaySpec, ReinitFn};
use crate::overlay_list::BaseListOverlay;
use crate::reinit::ReinitParams;
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
    cv.draw_text(font, x + 1, y + 1, s, colors().shade_shape, aa);
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
    draw_string_shade(cv, num, x_offset, num_y, s_num, colors().num, aa);
    // 标签名
    draw_string_shade(cv, label, x_offset + lwidth, y_offset, s_label, colors().label, aa);
    // 单位名
    draw_string_shade(cv, unit, x_offset + lwidth, y_offset + label.size, s_unit, colors().unit, aa);
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
    draw_string_shade(
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
                    vm_core::logger::error("ControlSurfaces", &format!("reinit 字体重载失败: {}", e));
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
                // 生产 AA 恒开 (Application.java:102 graphAASetting 默认 ON)
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
mod tests;
