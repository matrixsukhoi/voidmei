//! ConfigLoader 的 Rust 移植 (src/prog/config/ConfigLoader.java)
//!
//! Loader for dynamic overlay configuration files.
//! Refactored to use S-Expression (Lisp-like) syntax.
//!
//! PORT: Java `public Object value` (Boolean/Integer/Double/String/null 动态类型)
//! → `Option<ConfigValue>` 枚举 (§2.11: 原 Object 单态化为封闭 4+1 域)。
//! PORT: 异常控制流 (§2.7) — Java loadConfig/saveConfig 的 `catch (Exception e)
//! { e.printStackTrace(); }` 吞掉一切异常后**返回已累积的部分结果**:
//! 解析期 panic (asAtom/getDouble 的 IllegalStateException/NumberFormatException)
//! 经 catch_unwind 复刻, IO 错误经 Result 复刻, 二者统一打印 stderr 后继续。

use std::fs;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;
use std::sync::RwLock;

use crate::sexp_parser::{AtomType, SExp, SExpParser, SList};

// --- Java `Object value` 的类型域 (extractValue 实际产出) ---

/// Typed value (Boolean, Integer, String) — Java 注释只列三种,
/// extractValue 对非整数 NUMBER 另产出 Double, 一并收编。
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    Bool(bool),
    Int(i32),
    Double(f64),
    Str(String),
}

/// Java: `public static class RowConfig`
#[derive(Debug, Clone, PartialEq)]
pub struct RowConfig {
    pub label: String,
    pub target_name: Option<String>, // Display name for overlay if different from label
    pub formula: Option<String>, // Kept for reflection paths (e.g. S.rpm)
    pub format: String,
    pub unit: String, // Unit string (e.g. "Hp")
    pub value: Option<ConfigValue>, // Typed value (Boolean, Integer, String)
    pub default_value: Option<ConfigValue>, // Default value for reset
    pub fg_color: Option<String>, // Foreground color (e.g. for buttons)
    pub desc: Option<String>, // Help description tooltip
    pub desc_img: Option<String>, // Help image path (relative to project root)
    pub preview_value: Option<String>, // Default value for UI preview/placeholder
    pub hide_when_zero: bool, // Hide if value is zero
    pub precision: i32, // Number of decimal places
    pub unit_source: Option<String>, // Method name for dynamic unit (e.g., "getManifoldPressureDisplayUnit")
    pub precision_source: Option<String>, // Method name for dynamic precision (e.g., "getManifoldPressureDisplayPrecision")
    pub visible_when: Option<Rc<SExp>>, // 显示条件表达式（S-expression），用于控制字段可见性
    pub na_when: Option<Rc<SExp>>, // NA显示条件表达式（S-expression），满足条件时显示 "-" 而非数值

    // Extended fields for control-type rows
    pub r#type: String, // DATA, HEADER, SLIDER, COMBO, SWITCH, BUTTON
    pub property: Option<String>, // Bound GroupConfig property (e.g., "fontSize")
    pub min_val: i32, // For SLIDER
    pub max_val: i32, // For SLIDER
    pub group_columns: i32, // For HEADER: specify columns for this group
    pub children: Vec<RowConfig>,
}

impl RowConfig {
    /// Java: `public RowConfig(String label, String formula, String format)`
    /// (其余字段走 Java 声明默认值 — §2.10 按有意保真)
    pub fn new(label: String, formula: Option<String>, format: String) -> RowConfig {
        RowConfig {
            label,
            target_name: None,
            formula,
            format,
            unit: String::new(),
            value: Some(ConfigValue::Bool(true)),
            default_value: None,
            fg_color: None,
            desc: None,
            desc_img: None,
            preview_value: None,
            hide_when_zero: false,
            precision: 0,
            unit_source: None,
            precision_source: None,
            visible_when: None,
            na_when: None,
            r#type: "DATA".to_string(),
            property: None,
            min_val: 0,
            max_val: 100,
            group_columns: 0,
            children: Vec::new(),
        }
    }

    /// Java: `public int getInt()`
    /// null 值时 Java `value.toString()` 抛 NullPointerException, 被
    /// `catch (Exception e)` 兜住 → 0 (§2.7 异常控制流, 语义在此收敛为常量)。
    pub fn get_int(&self) -> i32 {
        match &self.value {
            // Java: value instanceof Number → ((Number) value).intValue()
            // Int 原值; Double 走 JLS 5.1.3 (Rust `as i32` 同义, 见 sexp_parser
            // get_int 的 oracle 对拍注释)
            Some(ConfigValue::Int(i)) => *i,
            Some(ConfigValue::Double(d)) => *d as i32,
            // Java: Integer.parseInt(value.toString()) 捕获一切异常 → 0 (§2.15)
            Some(v) => java_parse_int(&config_value_to_string(v)).unwrap_or(0),
            None => 0, // Java: null → NullPointerException → catch → 0
        }
    }

    /// Java: `public boolean getBool()`
    /// PORT: null 值时 Java `value.toString()` 抛 NullPointerException **无 catch 传播** —
    /// 保持 panic (调用方 ConfigurationService 波次须知晓此契约)。
    pub fn get_bool(&self) -> bool {
        match &self.value {
            Some(ConfigValue::Bool(b)) => *b, // Java: value instanceof Boolean
            Some(v) => java_parse_boolean(&config_value_to_string(v)),
            None => panic!("java.lang.NullPointerException: value is null in getBool()"),
        }
    }

    /// Java: `public String getStr()` — `String.valueOf(value)`, null → "null"
    pub fn get_str(&self) -> String {
        match &self.value {
            None => "null".to_string(),
            Some(v) => config_value_to_string(v),
        }
    }
}

/// Java: `public static class GroupConfig`
#[derive(Debug, Clone, PartialEq)]
pub struct GroupConfig {
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub alpha: i32,
    pub hotkey: i32, // 0 means no hotkey
    pub visible: bool, // Default to false (hidden)
    pub font_name: Option<String>,
    pub font_size: i32, // Font size adjustment (-6 to +20)
    pub columns: i32, // Number of columns for layout
    pub panel_columns: i32, // Number of columns for SETTINGS PANEL layout
    pub switch_key: Option<String>, // Config key for visibility switch (e.g., "flightInfoSwitch")
    pub rows: Vec<RowConfig>,
}

impl GroupConfig {
    /// Java: `public GroupConfig(String title)` (其余字段走声明默认值)
    pub fn new(title: String) -> GroupConfig {
        GroupConfig {
            title,
            x: 0.1,
            y: 0.1,
            alpha: 150,
            hotkey: 0,
            visible: false,
            font_name: None,
            font_size: 0,
            columns: 2,
            panel_columns: 2,
            switch_key: None,
            rows: Vec::new(),
        }
    }
}

/// `java.awt.Toolkit.getDefaultToolkit().getScreenSize()` 的注入点
/// (loadConfig 的 legacy 像素坐标换算专用)。
/// PORT: AWT 平台调用 (C 类), vm-core 无从自取 — vm-app 启动时以实际屏幕尺寸注入;
/// 未注入时按 Java headless 形态处理: `Toolkit.getDefaultToolkit()` 抛
/// HeadlessException (未受检) → 外层 catch → 打印 + 返回部分结果。
static LEGACY_SCREEN_SIZE: RwLock<Option<(i32, i32)>> = RwLock::new(None);

/// 注册屏幕尺寸 (px), 对应 Java 侧 Toolkit 每次调用即时查询 — 分辨率热变更后可重设。
pub fn set_legacy_screen_size(width: i32, height: i32) {
    *LEGACY_SCREEN_SIZE.write().unwrap() = Some((width, height));
}

fn legacy_screen_size() -> Result<(i32, i32), String> {
    LEGACY_SCREEN_SIZE
        .read()
        .unwrap()
        .ok_or_else(|| "java.awt.HeadlessException: getScreenSize 未注入 (vm-app 应调用 set_legacy_screen_size)".to_string())
}

/// Java `String.trim()`: 剥首尾所有 `<= U+0020` 的字符 (含 \n/\r/\t,
/// 不含 NBSP) — 与 Rust `str::trim` (Unicode White_Space) 不同, blkx/reader 同款。
fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| (c as u32) <= 0x20)
}

/// Java `Integer.parseInt(String)` (radix 10) 复刻:
/// 可选 +/-, 至少一位数字, 溢出/空/非法 → Err (= NumberFormatException)。
/// PORT: Java Character.digit 接受 Unicode Nd 数字 (如 '٥'); parseInt 无 trim —
/// 域内 cfg 值为 ASCII, §2.15 (catch 吞异常给默认值由调用方 unwrap_or 完成)。
fn java_parse_int(s: &str) -> Result<i32, ()> {
    let b = s.as_bytes();
    let (neg, digits) = match b.first() {
        Some(b'-') => (true, &b[1..]),
        Some(b'+') => (false, &b[1..]),
        _ => (false, b),
    };
    if digits.is_empty() {
        return Err(());
    }
    let mut acc: i64 = 0;
    for &d in digits {
        if !d.is_ascii_digit() {
            return Err(());
        }
        acc = acc * 10 + i64::from(d - b'0');
        if acc > i32::MAX as i64 + 1 {
            return Err(()); // 溢出 — Java 抛 NumberFormatException (§2.2 静默回绕不适用: parseInt 是解析不是运算)
        }
    }
    if neg {
        acc = -acc;
    }
    if !(i32::MIN as i64..=i32::MAX as i64).contains(&acc) {
        return Err(());
    }
    Ok(acc as i32)
}

/// Java `Boolean.parseBoolean(String)` = equalsIgnoreCase("true") — 非 "true" 一律 false。
fn java_parse_boolean(s: &str) -> bool {
    s.eq_ignore_ascii_case("true")
}

/// Java `String.valueOf(value)` (value 非 null) — 各装箱类型的 toString 格式
fn config_value_to_string(v: &ConfigValue) -> String {
    match v {
        ConfigValue::Bool(b) => b.to_string(),
        ConfigValue::Int(i) => i.to_string(),
        ConfigValue::Double(d) => java_double_to_string(*d),
        ConfigValue::Str(s) => s.clone(),
    }
}

/// Java `System.getProperty("line.separator")` — PrintWriter.println 的行终止符。
/// PORT: 平台相关 (Windows "\r\n" / 类 Unix "\n"), 与同平台 Java 输出逐字节一致;
/// loadConfig 的 readLine 对 \r\n/\r/\n 三形都归一, 故 round-trip 与平台无关。
fn java_line_separator() -> &'static str {
    if cfg!(windows) {
        "\r\n"
    } else {
        "\n"
    }
}

/// Java `String.format("%.4f", d)` 一比一复刻 (saveConfig 的 :x/:y 写回)。
/// 语义模型 (Java 8 oracle fuzz 200k 例实证): 等价
/// `new BigDecimal(Double.toString(d)).setScale(4, HALF_UP)` — 即对**最短往返十进制
/// 表示**做 HALF_UP, 而非 double 精确二进制值的展开 (0.00015: 精确值 0.0001499…
/// 但 Java 输出 "0.0002"; Rust `{:.4}` 是对精确值的半偶舍入, 双重分歧)。
/// 整数也带 4 位小数; NaN/Infinity 原样输出; -0.0 → "-0.0000"。
/// PORT: 数字串取 Rust `{:e}` 最短往返表示 — 语义模型依赖 `Double.toString(d)`
/// 的输出, 而 JDK-4511638 域 (如 1e23) Java 给的不是最短表示
/// ("9.999999999999999E22" 17 位), Formatter 据此展开:
/// %.4f → "99999999999999990000000.0000"
/// vs 本实现按最短 "1E23" → "100000000000000000000000.0000" (Java 8 oracle 实测,
/// 见 java_double_to_string 的 JDK-4511638 域注记与分歧固化测试)。
/// saveConfig 的 :x/:y 域 (0..1 比例坐标 / 像素坐标) 该量级不可达, 分歧无实际面。
/// exp10 > 25 的巨值全为整数, 无舍入, 走零填充字符串路径。
fn java_format_f4(d: f64) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    let neg = d.is_sign_negative(); // 含 -0.0 → "-0.0000" (Java 亦然)
    let a = d.abs();
    // "{:e}" → "D.DDDe±n" (a ≥ 0; 0.0 → "0e0" 走通用路径)
    let sci = format!("{:e}", a);
    let epos = sci.find('e').unwrap();
    let mant = &sci[..epos];
    let exp10: i32 = sci[epos + 1..].parse().unwrap();
    let digits = mant.replace('.', "");
    let digits = digits.as_bytes();
    let n = digits.len() as i32;

    let mut out = String::new();
    if exp10 > 25 {
        // 巨整数域 (≥ 10^21 → double 间距 ≥ 1, 恒无小数): digits + 隐含尾零 + ".0000"
        out.push_str(&sci[..epos].replace('.', ""));
        out.push_str(&"0".repeat((exp10 - n + 1) as usize));
        out.push_str(".0000");
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
        // 保留到 10^-4 位: i ≤ exp10+5; 判定位 = 其后一位 (HALF_UP: ≥5 进位,
        // 再后的剩余数字 < 1 单位不影响判定)
        let keep = exp10 + 5;
        let mut scaled: u128 = 0; // = (整数 + 前 4 位小数) 的 10^4 倍
        if keep > 0 {
            for i in 1..=keep {
                scaled = scaled * 10 + digit_at(i);
            }
        }
        if digit_at(keep + 1) >= 5 {
            scaled += 1; // HALF_UP (含精确 .5 进位; 进位可级联到整数部分)
        }
        let int_part = scaled / 10_000;
        let frac4 = scaled % 10_000;
        out.push_str(&format!("{int_part}.{frac4:04}"));
    }
    if neg {
        out.insert(0, '-');
    }
    out
}

/// Java `Double.toString(double)` 一比一复刻 (getStr/String.valueOf(Double) 与
/// saveConfig serializeAtom 的 Double 分支共用):
/// - 10^-3 ≤ |d| < 10^7 → 十进制平原式, 恒至少一位小数 ("1.0");
/// - 否则科学计数 "D.DDDE±x" ('E' 后仅负指数带 '-', 正指数无 '+');
/// - 最短可区分数字串; NaN/±0/±Inf 特判。
///
/// PORT: 数字串取 Rust `{:e}` 最短往返表示, 与 Java FloatingDecimal 在
/// JDK-4511638 域 (极罕见多位尾数) 外逐位一致 — cfg 值域 oracle 对拍无差异。
fn java_double_to_string(d: f64) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d == 0.0 {
        return if d.is_sign_negative() { "-0.0".to_string() } else { "0.0".to_string() };
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    let neg = d.is_sign_negative();
    let a = d.abs();
    // "{:e}" → "D.DDDe±n"; a > 0 有限, 恒此形态 (最短往返数字, 无尾随零)
    let sci = format!("{:e}", a);
    let epos = sci.find('e').unwrap();
    let mant = &sci[..epos];
    let exp10: i32 = sci[epos + 1..].parse().unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let mut s = String::new();
    if (-3..=6).contains(&exp10) {
        // 平原式
        if exp10 >= 0 {
            let ip = exp10 as usize + 1; // 整数部分位数
            if digits.len() > ip {
                s.push_str(&digits[..ip]);
                s.push('.');
                s.push_str(&digits[ip..]);
            } else {
                s.push_str(&digits);
                s.push_str(&"0".repeat(ip - digits.len()));
                s.push_str(".0"); // 恒至少一位小数
            }
        } else {
            s.push_str("0.");
            s.push_str(&"0".repeat((-exp10 - 1) as usize));
            s.push_str(&digits);
        }
    } else {
        // 科学计数
        s.push_str(&digits[..1]);
        s.push('.');
        if digits.len() > 1 {
            s.push_str(&digits[1..]);
        } else {
            s.push('0');
        }
        s.push('E');
        s.push_str(&exp10.to_string());
    }
    if neg {
        s.insert(0, '-');
    }
    s
}

// --- jnativehook 键码文本 (NativeKeyEvent.getKeyText) ---

/// jnativehook 2.2.2 `NativeKeyEvent.getKeyText` 的 VC 码→文本表 (139 项),
/// bytecode ldc 默认值 + en_US locale Java 8 oracle 全量对拍生成。
/// PORT: 原实现经 `Toolkit.getProperty("AWT.xxx", 默认值)` 查 JDK 的 awt.properties —
/// **随 JDK locale 本地化** (zh JDK: 1→"Esc"、54→"未知 keyCode: 0x36"), 本表为
/// 英文 canonical 默认值 (en_US oracle); 中文 JDK 上 Java 侧 hotkey 往返取不到
/// 英文名属环境差异, 非 C 类接线可解 — 已在迁移报告中上报。
fn key_text_table(code: i32) -> Option<&'static str> {
    match code {
        0 => Some("Undefined"),
        1 => Some("Escape"),
        2 => Some("1"),
        3 => Some("2"),
        4 => Some("3"),
        5 => Some("4"),
        6 => Some("5"),
        7 => Some("6"),
        8 => Some("7"),
        9 => Some("8"),
        10 => Some("9"),
        11 => Some("0"),
        12 => Some("Minus"),
        13 => Some("Equals"),
        14 => Some("Backspace"),
        15 => Some("Tab"),
        16 => Some("Q"),
        17 => Some("W"),
        18 => Some("E"),
        19 => Some("R"),
        20 => Some("T"),
        21 => Some("Y"),
        22 => Some("U"),
        23 => Some("I"),
        24 => Some("O"),
        25 => Some("P"),
        26 => Some("Open Bracket"),
        27 => Some("Close Bracket"),
        28 => Some("Enter"),
        29 => Some("Ctrl"),
        30 => Some("A"),
        31 => Some("S"),
        32 => Some("D"),
        33 => Some("F"),
        34 => Some("G"),
        35 => Some("H"),
        36 => Some("J"),
        37 => Some("K"),
        38 => Some("L"),
        39 => Some("Semicolon"),
        40 => Some("Quote"),
        41 => Some("Back Quote"),
        42 => Some("Shift"),
        43 => Some("Back Slash"),
        44 => Some("Z"),
        45 => Some("X"),
        46 => Some("C"),
        47 => Some("V"),
        48 => Some("B"),
        49 => Some("N"),
        50 => Some("M"),
        51 => Some("Comma"),
        52 => Some("Period"),
        53 => Some("Slash"),
        56 => Some("Alt"),
        57 => Some("Space"),
        58 => Some("Caps Lock"),
        59 => Some("F1"),
        60 => Some("F2"),
        61 => Some("F3"),
        62 => Some("F4"),
        63 => Some("F5"),
        64 => Some("F6"),
        65 => Some("F7"),
        66 => Some("F8"),
        67 => Some("F9"),
        68 => Some("F10"),
        69 => Some("Num Lock"),
        70 => Some("Scroll Lock"),
        83 => Some("NumPad ,"),
        87 => Some("F11"),
        88 => Some("F12"),
        91 => Some("F13"),
        92 => Some("F14"),
        93 => Some("F15"),
        99 => Some("F16"),
        100 => Some("F17"),
        101 => Some("F18"),
        102 => Some("F19"),
        103 => Some("F20"),
        104 => Some("F21"),
        105 => Some("F22"),
        106 => Some("F23"),
        107 => Some("F24"),
        112 => Some("Katakana"),
        115 => Some("Underscore"),
        119 => Some("Furigana"),
        121 => Some("Kanji"),
        123 => Some("Hiragana"),
        125 => Some("¥"),
        3639 => Some("Print Screen"),
        3653 => Some("Pause"),
        3655 => Some("Home"),
        3657 => Some("Page Up"),
        3663 => Some("End"),
        3665 => Some("Page Down"),
        3666 => Some("Insert"),
        3667 => Some("Delete"),
        3675 => Some("Meta"),
        3677 => Some("Context Menu"),
        57360 => Some("Previous"),
        57369 => Some("Next"),
        57376 => Some("Mute"),
        57377 => Some("App Calculator"),
        57378 => Some("Play"),
        57380 => Some("Stop"),
        57388 => Some("Eject"),
        57390 => Some("Volume Down"),
        57392 => Some("Volume Up"),
        57394 => Some("Browser Home"),
        57404 => Some("App Music"),
        57416 => Some("Up"),
        57419 => Some("Left"),
        57420 => Some("Clear"),
        57421 => Some("Right"),
        57424 => Some("Down"),
        57438 => Some("Power"),
        57439 => Some("Sleep"),
        57443 => Some("Wake"),
        57444 => Some("App Pictures"),
        57445 => Some("Browser Search"),
        57446 => Some("Browser Favorites"),
        57447 => Some("Browser Refresh"),
        57448 => Some("Stop"),
        57449 => Some("Browser Forward"),
        57450 => Some("Browser Back"),
        57452 => Some("App Mail"),
        57453 => Some("Select"),
        65396 => Some("Sun Open"),
        65397 => Some("Sun Help"),
        65398 => Some("Sun Props"),
        65399 => Some("Sun Front"),
        65400 => Some("Sun Stop"),
        65401 => Some("Sun Again"),
        65402 => Some("Sun Undo"),
        65403 => Some("Sun Cut"),
        65404 => Some("Sun Copy"),
        65405 => Some("Sun Insert"),
        65406 => Some("Sun Find"),
        _ => None,
    }
}

/// Java: `NativeKeyEvent.getKeyText(int keyCode)`
pub fn get_key_text(key_code: i32) -> String {
    if let Some(t) = key_text_table(key_code) {
        return t.to_string();
    }
    // Java default 分支: getProperty("AWT.unknown", "Unknown") + " keyCode: 0x" +
    // Integer.toString(keyCode, 16) — 负数带 '-' 前缀 (有符号幅值), 十六进制小写无补零
    let hex = if key_code < 0 {
        format!("-{:x}", (i64::from(key_code)).unsigned_abs())
    } else {
        format!("{:x}", key_code)
    };
    format!("Unknown keyCode: 0x{hex}")
}

/// Attempts to resolve key code from string (either "P" or "25")
fn get_key_code_from_text(text: Option<&str>) -> i32 {
    let Some(text) = text else {
        return 0; // Java: text == null
    };
    if java_trim(text).is_empty() {
        return 0;
    }
    let t = java_trim(text);
    // 1. Try numeric
    match java_parse_int(t) {
        Ok(i) => i,
        Err(()) => {
            // 2. Brute force lookup in common JNativeHook VC codes (typically < 256)
            for i in 1..256i32 {
                // Java equalsIgnoreCase — 键名域 ASCII, eq_ignore_ascii_case 等价
                if get_key_text(i).eq_ignore_ascii_case(t) {
                    return i;
                }
            }
            0
        }
    }
}

// --- S-Expression Parsing Helpers ---

fn get_keyword_string(list: &SList, keyword: &str, def: Option<&str>) -> Option<String> {
    let n = list.children.len();
    let mut i = 0;
    while i + 1 < n {
        let curr = &list.children[i];
        if curr.is_atom()
            && curr.as_atom().is_keyword()
            && curr.as_atom().get_string().eq_ignore_ascii_case(keyword)
        {
            let next = &list.children[i + 1];
            if next.is_atom() {
                return Some(next.as_atom().get_string().to_string());
            }
        }
        i += 1;
    }
    def.map(|d| d.to_string())
}

fn get_keyword_double(list: &SList, keyword: &str, def: f64) -> f64 {
    let n = list.children.len();
    let mut i = 0;
    while i + 1 < n {
        let curr = &list.children[i];
        if curr.is_atom()
            && curr.as_atom().is_keyword()
            && curr.as_atom().get_string().eq_ignore_ascii_case(keyword)
        {
            // Java 有 isAtom 守卫 (L139-141): 值为列表时跳过本关键字继续循环
            // (后续重复关键字仍可命中, Java 8 oracle: ":x (1 2) :x 0.5" → 0.5);
            // 非数值 atom 的 getDouble() 抛 NumberFormatException → panic 复刻,
            // 由 load_config 的 catch 兜住返回部分组 (Java 8 oracle: ":x abc" 同流)
            let next = &list.children[i + 1];
            if next.is_atom() {
                return next.as_atom().get_double();
            }
        }
        i += 1;
    }
    def
}

fn get_keyword_int(list: &SList, keyword: &str, def: i32) -> i32 {
    // Java: (int) getKeywordDouble(...) — JLS 5.1.3, Rust as i32 同义
    get_keyword_double(list, keyword, f64::from(def)) as i32
}

fn get_keyword_bool(list: &SList, keyword: &str, def: bool) -> bool {
    let n = list.children.len();
    let mut i = 0;
    while i + 1 < n {
        let curr = &list.children[i];
        if curr.is_atom()
            && curr.as_atom().is_keyword()
            && curr.as_atom().get_string().eq_ignore_ascii_case(keyword)
        {
            let next = &list.children[i + 1];
            if next.is_atom() {
                return next.as_atom().get_bool();
            }
        }
        i += 1;
    }
    def
}

/// 获取关键字对应的 SExp 值（用于 :visible-when 等需要完整表达式的属性）
/// (Java: getKeywordSExp — 返回关键字的下一个兄弟节点, 不查类型)
fn get_keyword_sexp(list: &SList, keyword: &str) -> Option<Rc<SExp>> {
    let n = list.children.len();
    let mut i = 0;
    while i + 1 < n {
        let curr = &list.children[i];
        if curr.is_atom()
            && curr.as_atom().is_keyword()
            && curr.as_atom().get_string().eq_ignore_ascii_case(keyword)
        {
            return Some(list.children[i + 1].clone()); // 直接返回 SExp 对象 (Rc 共享)
        }
        i += 1;
    }
    None
}

/// Java `catch (Exception e) { e.printStackTrace(); }` 的 stderr 近似:
/// 可观测意图 (异常文本打到 stderr 后继续) 等价, 不复刻栈帧行。
fn print_java_exception(msg: &str) {
    eprintln!("{msg}");
    eprintln!("\tat prog.config.ConfigLoader(ConfigLoader.java)");
}

/// Java: `public static List<GroupConfig> loadConfig(String path)`
///
/// 文件不存在 → 空; 任何异常 (IO / 解析 panic) → stderr 打印 + 返回**已累积**的
/// 部分 groups (Java catch 语义, §2.7)。
pub fn load_config(path: &str) -> Vec<GroupConfig> {
    let mut groups: Vec<GroupConfig> = Vec::new();

    if !Path::new(path).exists() {
        return groups;
    }

    // Java: try { 读文件+解析+建组 } catch (Exception e) { e.printStackTrace(); }
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        build_groups_from_file(path, &mut groups)
    }));
    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(io_msg)) => print_java_exception(&io_msg), // IOException → printStackTrace
        Err(payload) => {
            // 解析期未受检异常 (IllegalStateException / NumberFormatException) → panic 复刻
            let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "java.lang.RuntimeException".to_string()
            };
            print_java_exception(&format!("java.lang.Exception: {msg}"));
        }
    }
    groups
}

/// loadConfig 的 try 块主体 — Err = IO 异常, panic = 未受检运行时异常。
fn build_groups_from_file(path: &str, groups: &mut Vec<GroupConfig>) -> Result<(), String> {
    // Read file content
    // Java: BufferedReader.readLine() 把 \r\n / \r / \n 都视为行终止符, 每行 append("\n")
    // — 净效果 = 行终止符归一为 "\n"; InputStreamReader(UTF-8) 的非法字节替换 U+FFFD
    // ↔ from_utf8_lossy
    let raw = fs::read(path).map_err(|e| format!("java.io.IOException: {e}"))?;
    let content = String::from_utf8_lossy(&raw)
        .replace("\r\n", "\n")
        .replace('\r', "\n");

    let content = content.as_str();
    // Legacy compatibility: Check if it starts with '[' (INI format)
    if java_trim(content).starts_with('[') {
        // Fallback to legacy parser if needed, or just warn.
        // For now, let's assume valid S-Expr input or strict migration.
        // If you needed legacy support, we'd paste the old parser code here.
    }

    let mut parser = SExpParser::new();
    let panels = parser.parse(content);

    for exp in &panels {
        if !exp.is_list() {
            continue;
        }
        let panel_exp = exp.as_list();
        if panel_exp.children.is_empty() || !is_symbol(&panel_exp.children[0], "panel") {
            continue;
        }

        // (panel "Title" :k v ...)
        let mut title = "Unknown".to_string();
        if panel_exp.children.len() > 1 && panel_exp.children[1].is_atom() {
            title = panel_exp.children[1].as_atom().get_string().to_string();
        }

        let mut group = GroupConfig::new(title);
        group.x = get_keyword_double(panel_exp, ":x", 0.1);
        group.y = get_keyword_double(panel_exp, ":y", 0.1);

        // Legacy coord conversion
        // PORT: Java `java.awt.Toolkit.getDefaultToolkit().getScreenSize()` —
        // AWT 平台调用 (C 类), 经 set_legacy_screen_size 注入 (见函数注释);
        // 未注入时以 HeadlessException 形态中断本文件加载 (Java headless 同流)。
        if group.x > 2.0 {
            group.x /= f64::from(legacy_screen_size()?.0);
        }
        if group.y > 2.0 {
            group.y /= f64::from(legacy_screen_size()?.1);
        }

        group.alpha = get_keyword_int(panel_exp, ":alpha", 150);
        group.visible = get_keyword_bool(panel_exp, ":visible", false);
        group.font_name = get_keyword_string(panel_exp, ":font", None);
        group.font_size = get_keyword_int(panel_exp, ":font-size", 0);
        group.columns = get_keyword_int(panel_exp, ":columns", 2);
        group.panel_columns = get_keyword_int(panel_exp, ":panel-columns", 2);
        group.switch_key = get_keyword_string(panel_exp, ":switch-key", None);
        let hotkey_str = get_keyword_string(panel_exp, ":hotkey", None);
        if let Some(hotkey_str) = hotkey_str {
            group.hotkey = get_key_code_from_text(Some(&hotkey_str));
        }

        // Process children (items and groups)
        process_panel_children(&mut group.rows, panel_exp);

        groups.push(group);
    }
    Ok(())
}

/// Java: `private static void processPanelChildren(List<RowConfig> targetList, SList parentList)`
fn process_panel_children(target_list: &mut Vec<RowConfig>, parent_list: &SList) {
    for child in &parent_list.children {
        if !child.is_list() {
            continue;
        }
        let list = child.as_list();
        if list.children.is_empty() {
            continue;
        }

        let head = &list.children[0];
        if !head.is_atom() {
            continue;
        }
        let type_str = head.as_atom().get_string();

        if "group".eq_ignore_ascii_case(type_str) {
            // (group "Label" :k v ... children...)
            // Create a HEADER row
            let mut label = "Group".to_string();
            if list.children.len() > 1 {
                // PORT: Java 无 isAtom 守卫 — 第二子节点为列表时 asAtom() 抛
                // IllegalStateException → panic, 外层 catch 兜住 (保真)
                label = list.children[1].as_atom().get_string().to_string();
            }

            let mut header_row = RowConfig::new(label, None, "%s".to_string());
            header_row.r#type = "HEADER".to_string();
            header_row.group_columns = get_keyword_int(list, ":column", 0);
            header_row.value = Some(ConfigValue::Bool(true));

            // Recurse for group children
            process_panel_children(&mut header_row.children, list);

            target_list.push(header_row);
        } else if "item".eq_ignore_ascii_case(type_str) {
            // (item "Label" :k v ...)
            let mut label = "Item".to_string();
            if list.children.len() > 1 {
                // PORT: 同上, 无 isAtom 守卫
                label = list.children[1].as_atom().get_string().to_string();
            }

            let mut row = RowConfig::new(label, None, "%s".to_string());
            let raw_type = get_keyword_string(list, ":type", Some("DATA")).unwrap();

            // Map logical types to internal types
            // PORT: Java String.toUpperCase() 默认 locale — ASCII 域内与 Rust 一致
            row.r#type = raw_type.to_uppercase().replace('-', "_"); // switch-inv -> SWITCH_INV

            row.property = get_keyword_string(list, ":target", None);
            row.unit = get_keyword_string(list, ":unit", Some("")).unwrap();
            row.format = get_keyword_string(list, ":format", Some("%s")).unwrap();

            // Special handling for COMBO source and List paths which use format field
            // internally
            let source = get_keyword_string(list, ":source", None);
            if let Some(source) = source {
                row.format = source;
            }

            // Value extraction
            // value defaults to true for switches, 0 for slider, null/string for others
            // But we need to check the SExp type
            row.value = extract_value(list, ":value");
            row.default_value = extract_value(list, ":default");
            row.fg_color = get_keyword_string(list, ":fgcolor", None);
            row.desc = get_keyword_string(list, ":desc", None);
            row.desc_img = get_keyword_string(list, ":desc-img", None);
            row.preview_value = get_keyword_string(list, ":preview-value", None);
            row.hide_when_zero = get_keyword_bool(list, ":hide-when-zero", false);
            row.precision = get_keyword_int(list, ":precision", 0);
            row.unit_source = get_keyword_string(list, ":unit-source", None);
            row.precision_source = get_keyword_string(list, ":precision-source", None);
            row.target_name = get_keyword_string(list, ":target-name", None);
            row.visible_when = get_keyword_sexp(list, ":visible-when"); // 解析显示条件表达式
            row.na_when = get_keyword_sexp(list, ":na-when"); // 解析NA显示条件表达式

            if row.value.is_none() {
                if row.r#type.contains("SWITCH") {
                    row.value = Some(ConfigValue::Bool(true));
                }
                if row.r#type == "SLIDER" {
                    row.value = Some(ConfigValue::Int(0));
                }
            }

            // If default is missing, fallback to initial value
            if row.default_value.is_none() && row.r#type != "BUTTON" {
                row.default_value = row.value.clone();
            }

            row.min_val = get_keyword_int(list, ":min", 0);
            row.max_val = get_keyword_int(list, ":max", 100);

            // Compatibility mapping for 'formula' field
            // Legacy system used 'formula' for Reflection variable path (DATA rows)
            if row.r#type == "DATA" {
                row.formula = row.property.clone();
            } else {
                // For controls, formula isn't strictly needed by runtime if type is set,
                // but let's be safe if something uses it for debugging or fallback
                row.formula = row.property.clone();
            }

            target_list.push(row);
        }
    }
}

/// Java: `private static Object extractValue(SList list, String keyword)`
fn extract_value(list: &SList, keyword: &str) -> Option<ConfigValue> {
    let n = list.children.len();
    let mut i = 0;
    while i + 1 < n {
        let curr = &list.children[i];
        if curr.is_atom()
            && curr.as_atom().is_keyword()
            && curr.as_atom().get_string().eq_ignore_ascii_case(keyword)
        {
            // PORT: Java asAtom() 无 isAtom 守卫 — 列表值 (如 :value (a b)) 抛
            // IllegalStateException → panic, 外层 catch 兜住 (保真)
            let val = list.children[i + 1].as_atom();
            if val.r#type == AtomType::Boolean {
                return Some(ConfigValue::Bool(val.get_bool()));
            }
            if val.r#type == AtomType::Number {
                let d = val.get_double();
                // Java: if (d == (int) d) return (int) d; — NaN/±Inf/巨值不等 → Double
                if d == f64::from(d as i32) {
                    return Some(ConfigValue::Int(d as i32));
                }
                return Some(ConfigValue::Double(d));
            }
            return Some(ConfigValue::Str(val.get_string().to_string()));
        }
        i += 1;
    }
    None
}

/// Java: `private static boolean isSymbol(SExp exp, String name)`
fn is_symbol(exp: &SExp, name: &str) -> bool {
    exp.is_atom() && exp.as_atom().is_symbol() && exp.as_atom().get_string().eq_ignore_ascii_case(name)
}

// --- Serialization ---

/// Java: `public static void saveConfig(String path, List<GroupConfig> groups)`
///
/// PORT: FileOutputStream 打开失败 → FileNotFoundException → printStackTrace → 静默
/// 返回; 写入期 IO 错误 Java PrintWriter 自吞 (checkError 无人调用) → `let _ =` 等价。
/// 行终止符 = 平台 line.separator (java_line_separator)。
pub fn save_config(path: &str, groups: &[GroupConfig]) {
    let Ok(mut file) = fs::File::create(path) else {
        print_java_exception(&format!("java.io.FileNotFoundException: {path}"));
        return;
    };

    let mut pw = String::new();
    let jls = java_line_separator();

    for group in groups {
        pw.push_str("(panel ");
        pw.push_str(&quote(Some(&group.title)));
        pw.push_str(jls);

        let indent = "  "; // 2 spaces base indent for panel attributes as per sample
        let x_s = java_format_f4(group.x);
        let y_s = java_format_f4(group.y);
        write_attr_line(&mut pw, indent, ":x", AttrVal::Str(Some(&x_s)));
        write_attr_line(&mut pw, indent, ":y", AttrVal::Str(Some(&y_s)));
        write_attr_line(&mut pw, indent, ":alpha", AttrVal::Int(group.alpha));
        write_attr_line(&mut pw, indent, ":visible", AttrVal::Bool(group.visible));
        if let Some(sk) = &group.switch_key {
            write_attr_line(&mut pw, indent, ":switch-key", AttrVal::Str(Some(sk)));
        }
        write_attr_line(&mut pw, indent, ":font", AttrVal::Str(group.font_name.as_deref()));
        if group.hotkey != 0 {
            let t = get_key_text(group.hotkey);
            write_attr_line(&mut pw, indent, ":hotkey", AttrVal::Str(Some(&t)));
        }
        if group.font_size != 0 {
            write_attr_line(&mut pw, indent, ":font-size", AttrVal::Int(group.font_size));
        }
        if group.columns != 2 {
            write_attr_line(&mut pw, indent, ":columns", AttrVal::Int(group.columns));
        }
        if group.panel_columns != 0 {
            write_attr_line(&mut pw, indent, ":panel-columns", AttrVal::Int(group.panel_columns));
        }

        pw.push_str(jls);
        pw.push_str(jls);

        write_children(&mut pw, &group.rows, "  ");

        pw.push(')');
        pw.push_str(jls); // Close panel
        pw.push_str(jls);
    }

    let _ = file.write_all(pw.as_bytes()); // PrintWriter 吞 IO 错误
}

/// Java writeChildren
fn write_children(pw: &mut String, rows: &[RowConfig], indent: &str) {
    let jls = java_line_separator();
    for row in rows {
        if row.r#type == "HEADER" {
            pw.push_str(indent);
            pw.push_str("(group ");
            pw.push_str(&quote(Some(&row.label)));
            if row.group_columns > 0 {
                pw.push_str(&format!(" :column {}", row.group_columns));
            }
            pw.push_str(jls);

            // Recurse for children
            write_children(pw, &row.children, &format!("{indent}  "));

            pw.push_str(indent);
            pw.push(')');
            pw.push_str(jls); // Close group
        } else {
            // Item
            pw.push_str(indent);
            pw.push_str("(item ");
            pw.push_str(&quote(Some(&row.label)));

            // PORT: Java toLowerCase() 默认 locale — ASCII 域内一致
            let lisp_type = row.r#type.to_lowercase().replace('_', "-");
            pw.push_str(&format!(" :type {lisp_type}"));

            if let Some(property) = &row.property {
                pw.push_str(&format!(" :target {}", quote(Some(property))));
            }

            // Java: row.unit != null && !row.unit.isEmpty() — loader 路径 unit 恒非 null
            // (默认 ""), null 检查按死代码折叠
            if !row.unit.is_empty() {
                pw.push_str(&format!(" :unit {}", quote(Some(&row.unit))));
            }

            if row.formula.is_some() && row.r#type == "DATA" {
                pw.push_str(&format!(" :target {}", quote(row.formula.as_deref())));
            }

            // Type specific fields
            if lisp_type == "slider" {
                pw.push_str(&format!(" :min {} :max {}", row.min_val, row.max_val));
            }

            if lisp_type == "combo" || lisp_type == "filelist" || lisp_type == "fmlist" {
                pw.push_str(&format!(" :source {}", quote(Some(&row.format))));
            } else {
                if row.format != "%s" {
                    pw.push_str(&format!(" :format {}", quote(Some(&row.format))));
                }
            }

            // Value is last
            if lisp_type != "button" {
                pw.push_str(&format!(" :value {}", serialize_atom(row.value.as_ref())));
            }
            if row.default_value.is_some() && lisp_type != "button" {
                pw.push_str(&format!(" :default {}", serialize_atom(row.default_value.as_ref())));
            }
            if let Some(desc) = &row.desc {
                pw.push_str(&format!(" :desc {}", quote(Some(desc))));
            }
            if let Some(desc_img) = &row.desc_img {
                pw.push_str(&format!(" :desc-img {}", quote(Some(desc_img))));
            }
            if let Some(preview_value) = &row.preview_value {
                pw.push_str(&format!(" :preview-value {}", quote(Some(preview_value))));
            }
            if row.hide_when_zero {
                pw.push_str(" :hide-when-zero true");
            }
            if row.precision != 0 {
                pw.push_str(&format!(" :precision {}", row.precision));
            }
            if let Some(unit_source) = &row.unit_source {
                pw.push_str(&format!(" :unit-source {}", quote(Some(unit_source))));
            }
            if let Some(precision_source) = &row.precision_source {
                pw.push_str(&format!(" :precision-source {}", quote(Some(precision_source))));
            }
            if let Some(target_name) = &row.target_name {
                pw.push_str(&format!(" :target-name {}", quote(Some(target_name))));
            }
            if let Some(fg_color) = &row.fg_color {
                pw.push_str(&format!(" :fgcolor {}", quote(Some(fg_color))));
            }
            // 序列化 :visible-when 和 :na-when 表达式
            if let Some(visible_when) = &row.visible_when {
                pw.push_str(&format!(" :visible-when {visible_when}"));
            }
            if let Some(na_when) = &row.na_when {
                pw.push_str(&format!(" :na-when {na_when}"));
            }

            pw.push(')');
            pw.push_str(jls);
        }
    }
}

/// Java writeAttrLine 的 `Object val` 实参域 (String/Integer/Boolean, String 可空)
#[derive(Debug, Clone, Copy)]
enum AttrVal<'a> {
    Str(Option<&'a str>),
    Int(i32),
    Bool(bool),
}

/// Java: `private static void writeAttrLine(PrintWriter pw, String indent, String key, Object val)`
fn write_attr_line(pw: &mut String, indent: &str, key: &str, val: AttrVal<'_>) {
    pw.push_str(indent);
    pw.push_str(key);
    pw.push(' ');
    match val {
        AttrVal::Str(s) => pw.push_str(&serialize_atom_str(s)),
        AttrVal::Int(i) => pw.push_str(&i.to_string()),      // String.valueOf(Integer)
        AttrVal::Bool(b) => pw.push_str(&b.to_string()),     // String.valueOf(Boolean)
    }
    pw.push_str(java_line_separator()); // pw.println(serializeAtom(val))
}

/// Java: `private static String quote(String s)`
fn quote(s: Option<&str>) -> String {
    match s {
        None => "\"\"".to_string(),
        Some(s) => format!("\"{}\"", s.replace('"', "\\\"")),
    }
}

/// Java: `private static String serializeAtom(Object val)` — RowConfig.value 域
fn serialize_atom(val: Option<&ConfigValue>) -> String {
    match val {
        None => "\"\"".to_string(),
        Some(ConfigValue::Str(s)) => serialize_atom_str(Some(s)),
        // String.valueOf(Object) → toString(): Boolean/Integer/Double 各自格式
        Some(v) => config_value_to_string(v),
    }
}

/// serializeAtom 的 String 分支: 数字形字符串不加引号
fn serialize_atom_str(s: Option<&str>) -> String {
    match s {
        None => "\"\"".to_string(),
        Some(s) => {
            if is_numeric(s) {
                s.to_string()
            } else {
                quote(Some(s))
            }
        }
    }
}

/// Java: `private static boolean isNumeric(String s)`
/// PORT: Character.isDigit 是 Unicode Nd — cfg 值域 ASCII, §2.1 域内等价;
/// s == null 分支已由调用方 Option 分流 (Java null → false, 此处不可达 None)。
fn is_numeric(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut dot_count = 0;
    for (i, c) in s.chars().enumerate() {
        if c == '-' {
            if i != 0 {
                return false;
            }
        } else if c == '.' {
            dot_count += 1;
            if dot_count > 1 {
                return false;
            }
        } else if !c.is_ascii_digit() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    // ---- Java 语义辅助的 oracle 对拍 (Java 8, build/oracle 实测) ----

    /// String.format("%.4f") — Formatter HALF_UP on 精确十进制展开。
    /// 0.03125/0.09375 是精确半点 (dyadic 奇分母 32), Rust {:.4} 半偶会给 0.0312/0.0937。
    #[test]
    fn java_format_f4_matches_java8_oracle() {
        let cases = [
            (0.5, "0.5000"),
            (0.3891, "0.3891"),
            (0.0602, "0.0602"),
            (0.3125, "0.3125"),
            (0.03125, "0.0313"),
            (0.09375, "0.0938"),
            (-0.03125, "-0.0313"),
            (0.15, "0.1500"),
            (0.85, "0.8500"),
            (1.0, "1.0000"),
            (2.5, "2.5000"),
            (0.00005, "0.0001"),
            (0.00015, "0.0002"),
            (0.12345, "0.1235"),
            (0.1234501, "0.1235"),
            (-0.85, "-0.8500"),
            (12345.67895, "12345.6790"),
            (9_999_999.499_999_998, "9999999.5000"),
            (9999999.5, "9999999.5000"),
            (10000000.0, "10000000.0000"),
            (6.02214e23, "602214000000000000000000.0000"),
            (-0.00004, "-0.0000"),
            (0.0, "0.0000"),
            (-0.0, "-0.0000"),
            (2.00005, "2.0001"),
            (0.7, "0.7000"),
            (0.28845, "0.2885"),
            (1.00005, "1.0001"),
            (0.0613, "0.0613"),
            (0.1299, "0.1299"),
            (0.0100, "0.0100"),
        ];
        for (d, want) in cases {
            assert_eq!(java_format_f4(d), want, "{d} → 期望 {want}");
        }
        // NaN/Infinity 分支 (Formatter 原样输出)
        assert_eq!(java_format_f4(f64::NAN), "NaN");
        assert_eq!(java_format_f4(f64::INFINITY), "Infinity");
        assert_eq!(java_format_f4(f64::NEG_INFINITY), "-Infinity");
    }

    /// f4 侧的 JDK-4511638 已知分歧面固化 (Java 8 oracle 实测):
    /// Double.toString(1e23)="9.999999999999999E22" (17 位非最短) → Formatter 展开
    /// "%.4f" → "99999999999999990000000.0000"; 本实现取最短 "1E23" →
    /// "100000000000000000000000.0000"。对照: 6.02214e23 双方一致 (已在上方 battery)。
    /// saveConfig :x/:y 域 (0..1/像素坐标) 该量级不可达 — 见 java_format_f4 PORT 注释。
    #[test]
    fn java_format_f4_jdk_4511638_domain_divergence() {
        assert_eq!(java_format_f4(1.0e23), "100000000000000000000000.0000"); // Java: 99999999999999990000000.0000
    }

    /// Double.toString — 最短区分 + [1e-3, 1e7) 平原式 / 恒一位小数 / E 计数。
    /// 全部期望值来自 Java 8 oracle 逐字面量对拍。
    #[test]
    // approx_constant: 3.14159 是 Java oracle 对拍表字面量, 禁换 std PI
    #[allow(clippy::approx_constant)]
    fn java_double_to_string_matches_java8_oracle() {
        let cases = [
            (0.03125, "0.03125"),
            (1.5, "1.5"),
            (1.0e7, "1.0E7"),
            (9999999.0, "9999999.0"),
            (0.001, "0.001"),
            (1.0e-4, "1.0E-4"),
            (1.0, "1.0"),
            (100.0, "100.0"),
            (1.2345e7, "1.2345E7"),
            (-2.5e-9, "-2.5E-9"),
            (0.85, "0.85"),
            // PORT: Java oracle 对拍表字面量, 禁换 std::f64::consts::PI (对拍的就是这个字面量)
            #[allow(clippy::approx_constant)]
            (3.14159, "3.14159"),
            (2.5e9, "2.5E9"),
            (1.0e10, "1.0E10"),
            (0.1, "0.1"),
            (0.2, "0.2"),
            (0.3, "0.3"),
            (9.999999999999999e-4, "9.999999999999998E-4"),
            (1.7976931348623157e308, "1.7976931348623157E308"),
            (123456789012345.6, "1.234567890123456E14"),
            (0.0, "0.0"),
            (1.0000000000000002, "1.0000000000000002"),
            (0.002, "0.002"),
        ];
        for (d, want) in cases {
            assert_eq!(java_double_to_string(d), want, "{d} → 期望 {want}");
        }
        assert_eq!(java_double_to_string(-0.0), "-0.0");
        assert_eq!(java_double_to_string(f64::NAN), "NaN");
        assert_eq!(java_double_to_string(f64::INFINITY), "Infinity");
        assert_eq!(java_double_to_string(f64::NEG_INFINITY), "-Infinity");
    }

    /// JDK-4511638 域: Java 8 FloatingDecimal 对个别位形**不是最短表示** —
    /// oracle: Double.toString(1e23) = "9.999999999999999E22" (17 位) 而最短往返是
    /// "1E23"; Double.toString(5e-324) = "4.9E-324" 而最短是 "5E-324"。
    /// 本实现取最短往返 (Rust {:e}), 在这些 oracle 位形上与 Java 有末位差异 —
    /// cfg :value 域 (手写短小数) 不可达, 已在迁移报告上报; 本测试固化已知分歧面。
    /// (9.999999999999999E-4 经 oracle 复核 Java 亦给最短 "9.999999999999998E-4",
    /// 归入上方一致 battery。)
    #[test]
    fn java_double_to_string_jdk_4511638_domain_divergence() {
        assert_eq!(java_double_to_string(1.0e23), "1.0E23"); // Java: 9.999999999999999E22
        assert_eq!(java_double_to_string(5e-324), "5.0E-324"); // Java: 4.9E-324
        assert_eq!(java_double_to_string(4.9e-324), "5.0E-324"); // Java: 4.9E-324
    }

    /// getKeyCodeFromText — en_US locale oracle (中文 JDK 的 AWT 属性表本地化
    /// 会导致 "Space" 解析失败, 属 Java 侧环境差异, 见 key_text_table PORT 注释)
    #[test]
    fn key_code_from_text_matches_java8_oracle() {
        let cases = [
            (Some("P"), 25),
            (Some("p"), 25),
            (Some("30"), 30),
            (Some("  25  "), 25), // Java trim
            (Some("F5"), 63),
            (Some("f5"), 63),
            (Some("Space"), 57),
            (Some("space"), 57),
            (Some("Escape"), 1),
            (Some("esc"), 0),
            (Some("A"), 30),
            (Some("a"), 30),
            (Some("Unknown keyCode: 0x36"), 54), // 未知码文本可反向命中其自身
            (Some(""), 0),
            (Some("   "), 0),
            (Some("Enter"), 28),
            (Some("enter"), 28),
            (Some("Comma"), 51),
            (Some("Minus"), 12),
            (Some("Backspace"), 14),
            (Some("1"), 1), // 数字直取 (1 = VC_ESCAPE)
            (Some("256"), 256),
            (Some("255"), 255),
            (Some("-5"), -5),
            (Some("0x1E"), 0),
            (Some("Back Quote"), 41),
            (Some("Open Bracket"), 26),
            (Some("Tab"), 15),
            (Some("CAPS LOCK"), 58), // equalsIgnoreCase
            (None, 0),
        ];
        for (t, want) in cases {
            assert_eq!(get_key_code_from_text(t), want, "{t:?} → 期望 {want}");
        }
    }

    /// getKeyText: 表内条目 + 未知码 default 分支 (英文 canonical, en_US oracle)
    #[test]
    fn key_text_known_and_unknown() {
        let cases = [
            (0, "Undefined"),
            (1, "Escape"),
            (12, "Minus"),
            (25, "P"),
            (28, "Enter"),
            (30, "A"),
            (41, "Back Quote"),
            (57, "Space"),
            (58, "Caps Lock"),
            (63, "F5"),
            (70, "Scroll Lock"),
            (83, "NumPad ,"),
            (87, "F11"),
            (91, "F13"),
            (112, "Katakana"),
            (119, "Furigana"),
            (121, "Kanji"),
            (123, "Hiragana"),
            (125, "¥"),
            (3639, "Print Screen"),
            (57404, "App Music"),
            (65406, "Sun Find"),
        ];
        for (c, want) in cases {
            assert_eq!(get_key_text(c), want, "{c} → 期望 {want}");
        }
        // 未知码: "Unknown keyCode: 0x" + Integer.toString(code, 16) (小写, 负数带 '-')
        assert_eq!(get_key_text(54), "Unknown keyCode: 0x36");
        assert_eq!(get_key_text(254), "Unknown keyCode: 0xfe");
        assert_eq!(get_key_text(255), "Unknown keyCode: 0xff");
        assert_eq!(get_key_text(65435), "Unknown keyCode: 0xff9b");
        assert_eq!(get_key_text(-5), "Unknown keyCode: 0x-5");
    }

    // ---- RowConfig 值访问器 (Java instanceof/toString 分支) ----

    #[test]
    fn row_config_typed_value_accessors() {
        let mk = |v: Option<ConfigValue>| {
            let mut r = RowConfig::new("t".into(), None, "%s".into());
            r.value = v;
            r
        };
        // Integer
        let r = mk(Some(ConfigValue::Int(42)));
        assert_eq!(r.get_int(), 42);
        assert_eq!(r.get_str(), "42");
        assert!(!r.get_bool()); // parseBoolean("42") = false
        // Number.intValue() 饱和 (JLS 5.1.3)
        let r = mk(Some(ConfigValue::Double(2.5e9)));
        assert_eq!(r.get_int(), 2147483647);
        assert_eq!(r.get_str(), "2.5E9");
        // parseInt fallback: 字符串数字可解析, 其余吞异常 → 0 (无 trim 语义)
        let r = mk(Some(ConfigValue::Str("42".into())));
        assert_eq!(r.get_int(), 42);
        let r = mk(Some(ConfigValue::Str("4.5".into())));
        assert_eq!(r.get_int(), 0);
        let r = mk(Some(ConfigValue::Str(" 12".into())));
        assert_eq!(r.get_int(), 0);
        let r = mk(Some(ConfigValue::Str("+12".into())));
        assert_eq!(r.get_int(), 12);
        let r = mk(Some(ConfigValue::Str("2147483648".into())));
        assert_eq!(r.get_int(), 0); // 溢出 → NumberFormatException → 0
        let r = mk(Some(ConfigValue::Str("abc".into())));
        assert_eq!(r.get_int(), 0);
        // Boolean
        let r = mk(Some(ConfigValue::Bool(true)));
        assert_eq!(r.get_int(), 0); // parseInt("true") 失败
        assert_eq!(r.get_str(), "true");
        assert!(r.get_bool());
        // parseBoolean = equalsIgnoreCase("true")
        let r = mk(Some(ConfigValue::Str("True".into())));
        assert!(r.get_bool());
        let r = mk(Some(ConfigValue::Str("yes".into())));
        assert!(!r.get_bool());
        // null → String.valueOf(null) = "null"; getInt 走 NPE→catch→0
        let r = mk(None);
        assert_eq!(r.get_str(), "null");
        assert_eq!(r.get_int(), 0);
    }

    /// Java getBool 对 null value 抛 NullPointerException (无 catch) — panic 复刻
    #[test]
    #[should_panic(expected = "NullPointerException")]
    fn get_bool_on_null_value_panics_like_npe() {
        let mut r = RowConfig::new("t".into(), None, "%s".into());
        r.value = None;
        r.get_bool();
    }

    // ---- quote / isNumeric / serializeAtom ----

    #[test]
    fn quote_and_is_numeric_edges() {
        assert_eq!(quote(None), "\"\"");
        assert_eq!(quote(Some("a\"b")), "\"a\\\"b\"");
        assert_eq!(quote(Some("")), "\"\"");
        assert!(is_numeric("123"));
        assert!(is_numeric("-5"));
        assert!(is_numeric("1.5"));
        assert!(is_numeric(".5"));
        assert!(is_numeric("5."));
        assert!(is_numeric("-")); // Java: '-' 仅限首位, 单字符通过 — 保真
        assert!(!is_numeric(""));
        assert!(!is_numeric("1.2.3"));
        assert!(!is_numeric("12a"));
        assert!(!is_numeric("1,2"));
        assert!(!is_numeric("1-2"));
        assert!(!is_numeric("--1"));
        // serializeAtom: 数字形字符串不加引号, 其余加引号; null → ""
        assert_eq!(serialize_atom_str(Some("123")), "123");
        assert_eq!(serialize_atom_str(Some("-1.5")), "-1.5");
        assert_eq!(serialize_atom_str(Some("abc")), "\"abc\"");
        assert_eq!(serialize_atom_str(Some("true")), "\"true\""); // 非 isNumeric → 加引号
        assert_eq!(serialize_atom_str(None), "\"\"");
        assert_eq!(serialize_atom(Some(&ConfigValue::Bool(false))), "false");
        assert_eq!(serialize_atom(Some(&ConfigValue::Int(-7))), "-7");
        assert_eq!(serialize_atom(Some(&ConfigValue::Double(1.5))), "1.5");
        assert_eq!(serialize_atom(None), "\"\"");
    }

    // ---- load/save: 文件级 ----

    fn tmp(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("vm_core_config_loader_{name}"))
            .to_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn missing_file_returns_empty() {
        let p = tmp("nonexistent_zzz.cfg");
        let _ = fs::remove_file(&p);
        assert!(load_config(&p).is_empty());
    }

    /// 无 :value 行的类型默认 (Java L324-334: SWITCH→true / SLIDER→0 / 其余 null;
    /// default 缺省回落 value, BUTTON 除外)
    #[test]
    fn value_defaults_by_row_type() {
        let cfg = "(panel \"p\"\n\
                   (item \"sw\" :type switch :target \"k1\")\n\
                   (item \"inv\" :type switch-inv :target \"k2\")\n\
                   (item \"sl\" :type slider :target \"k3\")\n\
                   (item \"co\" :type combo :target \"k4\")\n\
                   (item \"bt\" :type button :target \"k5\")\n\
                   )\n";
        let p = tmp("defaults.cfg");
        fs::write(&p, cfg).unwrap();
        let groups = load_config(&p);
        assert_eq!(groups.len(), 1);
        let rows = &groups[0].rows;
        assert_eq!(rows[0].value, Some(ConfigValue::Bool(true))); // contains("SWITCH")
        assert_eq!(rows[0].default_value, Some(ConfigValue::Bool(true)));
        assert_eq!(rows[1].value, Some(ConfigValue::Bool(true))); // SWITCH_INV 也含 SWITCH
        assert_eq!(rows[2].value, Some(ConfigValue::Int(0))); // SLIDER
        assert_eq!(rows[2].default_value, Some(ConfigValue::Int(0)));
        assert_eq!(rows[3].value, None); // COMBO 无默认
        assert_eq!(rows[3].default_value, None); // 回落 value = null
        assert_eq!(rows[4].value, None); // BUTTON
        assert_eq!(rows[4].default_value, None); // BUTTON 不回落
    }

    /// :value 为列表 → Java asAtom() IllegalStateException → 外层 catch → 返回已建组
    #[test]
    fn malformed_value_list_aborts_load_with_partial_groups() {
        let cfg = "(panel \"A\" (item \"ok\" :type switch :value true))\n\
                   (panel \"B\" (item \"bad\" :value (a b)))\n";
        let p = tmp("malformed.cfg");
        fs::write(&p, cfg).unwrap();
        let groups = load_config(&p);
        assert_eq!(groups.len(), 1, "异常后应保留首个 panel (Java 部分返回语义)");
        assert_eq!(groups[0].title, "A");
    }

    /// getKeywordDouble/Int 的 isAtom 守卫 (Java L139-141): 值为列表时跳过本关键字
    /// 继续循环 → 默认值 + panel 完整加载 (不 abort), 后续重复关键字仍可命中。
    /// 期望值全部来自 Java 8 oracle 实测 (build/oracle/cfgguard, ojdkbuild8 1.8.0_342)。
    #[test]
    fn keyword_list_value_tolerated_like_java() {
        // :x (1 2) → x=0.1 默认, 两组完整 (Java: groups=2, A.x=0.1000)
        let cfg = "(panel \"A\" :x (1 2)\n (item \"i\" :type switch :value true))\n\
                   (panel \"B\" :x 0.7)\n";
        let p = tmp("kwlist_x.cfg");
        fs::write(&p, cfg).unwrap();
        let groups = load_config(&p);
        assert_eq!(groups.len(), 2, "列表值不应中止加载 (Java isAtom 守卫)");
        assert!((groups[0].x - 0.1).abs() < 1e-12);
        assert_eq!(groups[0].rows.len(), 1);
        assert!((groups[1].x - 0.7).abs() < 1e-12);

        // :x (1 2) :x 0.5 → 守卫跳过列表后命中第二个关键字 → 0.5 (Java oracle)
        let cfg = "(panel \"A\" :x (1 2) :x 0.5)\n";
        let p = tmp("kwlist_dup.cfg");
        fs::write(&p, cfg).unwrap();
        let groups = load_config(&p);
        assert_eq!(groups.len(), 1);
        assert!((groups[0].x - 0.5).abs() < 1e-12);

        // :alpha (200) → getKeywordInt 委托同路径 → 默认 150, 两组完整 (Java oracle)
        let cfg = "(panel \"A\" :alpha (200))\n(panel \"B\")\n";
        let p = tmp("kwlist_alpha.cfg");
        fs::write(&p, cfg).unwrap();
        let groups = load_config(&p);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].alpha, 150);

        // :font-size (3) → 默认 0 (Java oracle: fontSize=0)
        let cfg = "(panel \"A\" :font-size (3))\n";
        let p = tmp("kwlist_fs.cfg");
        fs::write(&p, cfg).unwrap();
        assert_eq!(load_config(&p)[0].font_size, 0);

        // item 内 :min (1 2) :max 9 → min=0 默认, max=9, 行完整 (Java oracle)
        let cfg = "(panel \"A\" (item \"sl\" :type slider :min (1 2) :max 9 :value 4))\n";
        let p = tmp("kwlist_min.cfg");
        fs::write(&p, cfg).unwrap();
        let groups = load_config(&p);
        assert_eq!(groups[0].rows.len(), 1);
        assert_eq!(groups[0].rows[0].min_val, 0);
        assert_eq!(groups[0].rows[0].max_val, 9);
    }

    /// 守卫只挡 List — 非数值 atom 的 getDouble() 仍抛 NumberFormatException
    /// → panic → 外层 catch → 部分组 (Java 8 oracle: ":x abc" → 1 组保留 A)
    #[test]
    fn keyword_non_numeric_atom_aborts_load_like_java() {
        let cfg = "(panel \"A\" (item \"ok\" :type switch :value true))\n\
                   (panel \"B\" :x abc)\n";
        let p = tmp("kwnonnum.cfg");
        fs::write(&p, cfg).unwrap();
        let groups = load_config(&p);
        assert_eq!(groups.len(), 1, "非数值关键字值应中止后续加载 (NumberFormatException)");
        assert_eq!(groups[0].title, "A");
    }

    /// extractValue 数值类型化: 整数→Int, 非整数→Double, NaN/巨值保持 Double
    #[test]
    fn extract_value_number_typing() {
        let cfg = "(panel \"p\"\n\
                   (item \"i\" :type info :value 5)\n\
                   (item \"d\" :type info :value 5.5)\n\
                   (item \"neg\" :type info :value -3)\n\
                   (item \"huge\" :type info :value 10000000000)\n\
                   (item \"nan\" :type info :value NaN)\n\
                   (item \"b\" :type info :value true)\n\
                   (item \"s\" :type info :value \"txt\")\n\
                   )\n";
        let p = tmp("typing.cfg");
        fs::write(&p, cfg).unwrap();
        let rows = &load_config(&p)[0].rows;
        assert_eq!(rows[0].value, Some(ConfigValue::Int(5)));
        assert_eq!(rows[1].value, Some(ConfigValue::Double(5.5)));
        assert_eq!(rows[2].value, Some(ConfigValue::Int(-3)));
        // 1e10: (int) 饱和 2147483647 ≠ 1e10 → 保持 Double
        assert_eq!(rows[3].value, Some(ConfigValue::Double(1.0e10)));
        // NaN == (int)NaN(=0) 为 false → Double(NaN)
        match &rows[4].value {
            Some(ConfigValue::Double(d)) => assert!(d.is_nan()),
            v => panic!("NaN 应为 Double: {v:?}"),
        }
        assert_eq!(rows[5].value, Some(ConfigValue::Bool(true)));
        assert_eq!(rows[6].value, Some(ConfigValue::Str("txt".into())));
    }

    /// legacy 像素坐标 (>2.0) 除以注入的屏幕尺寸; 未注入 → HeadlessException 形态中断
    /// (全局注入点, 顺序执行避免与其余用例竞争 — 其余用例坐标 ≤ 1.0 不触发读取)
    #[test]
    fn legacy_pixel_coord_uses_injected_screen_size() {
        let cfg = "(panel \"p\" :x 1280 :y 720\n(item \"a\" :type switch :value true))\n";
        let p = tmp("pixel.cfg");
        fs::write(&p, cfg).unwrap();

        set_legacy_screen_size(2560, 1440);
        let groups = load_config(&p);
        assert_eq!(groups.len(), 1);
        assert!((groups[0].x - 0.5).abs() < 1e-12);
        assert!((groups[0].y - 0.5).abs() < 1e-12);

        // 未注入 = Java headless: HeadlessException → catch → 空
        *LEGACY_SCREEN_SIZE.write().unwrap() = None;
        assert!(load_config(&p).is_empty());
    }

    // ---- 固定样本: Java oracle 双实现对拍 (en_US locale, Java 8) ----

    /// 与 Java oracle DumpCfg 完全同构的模型转储 (逐字段对拍辅助)
    fn dump_val(v: &Option<ConfigValue>) -> String {
        match v {
            None => "N".to_string(),
            Some(ConfigValue::Bool(b)) => format!("B:{b}"),
            Some(ConfigValue::Int(i)) => format!("I:{i}"),
            Some(ConfigValue::Double(d)) => format!("D:{}", java_double_to_string(*d)),
            Some(ConfigValue::Str(s)) => format!("S:[{s}]"),
        }
    }
    fn dump_opt(s: &Option<String>) -> String {
        match s {
            None => "N".to_string(),
            Some(s) => format!("[{s}]"),
        }
    }
    /// Java DumpCfg 对非空 String 字段 (format/unit) 的 `[{s}]` 形态
    fn dump_br(s: &str) -> String {
        format!("[{s}]")
    }
    fn dump_row(sb: &mut String, r: &RowConfig, depth: usize) {
        for _ in 0..depth {
            sb.push_str("  ");
        }
        let vis = match &r.visible_when {
            None => "N".to_string(),
            Some(e) => e.to_string(),
        };
        let na = match &r.na_when {
            None => "N".to_string(),
            Some(e) => e.to_string(),
        };
        sb.push_str(&format!(
            "ROW|{}|type={}|formula={}|format={}|unit={}|value={}|default={}|fgColor={}|desc={}|descImg={}|preview={}|hideWhenZero={}|precision={}|unitSource={}|precisionSource={}|targetName={}|visibleWhen={}|naWhen={}|property={}|min={}|max={}|groupColumns={}|children={}\n",
            r.label, r.r#type, dump_opt(&r.formula), dump_br(&r.format),
            dump_br(&r.unit), dump_val(&r.value), dump_val(&r.default_value),
            dump_opt(&r.fg_color), dump_opt(&r.desc), dump_opt(&r.desc_img),
            dump_opt(&r.preview_value), r.hide_when_zero, r.precision,
            dump_opt(&r.unit_source), dump_opt(&r.precision_source), dump_opt(&r.target_name),
            vis, na, dump_opt(&r.property), r.min_val, r.max_val, r.group_columns,
            r.children.len(),
        ));
        for c in &r.children {
            dump_row(sb, c, depth + 1);
        }
    }
    fn dump_groups(groups: &[GroupConfig]) -> String {
        let mut sb = String::new();
        for g in groups {
            sb.push_str(&format!(
                "GROUP|{}|x={}|y={}|alpha={}|hotkey={}|visible={}|font={}|fontSize={}|columns={}|panelColumns={}|switchKey={}|rows={}\n",
                g.title,
                java_double_to_string(g.x),
                java_double_to_string(g.y),
                g.alpha,
                g.hotkey,
                g.visible,
                dump_opt(&g.font_name),
                g.font_size,
                g.columns,
                g.panel_columns,
                dump_opt(&g.switch_key),
                g.rows.len(),
            ));
            for r in &g.rows {
                dump_row(&mut sb, r, 1);
            }
        }
        sb
    }

    /// 固定样本 (Java oracle 输入, en_US locale)
    const SAMPLE_CFG: &str = r#"(panel "采样Alpha"
  :x 0.03125
  :y 0.123456
  :alpha 200
  :visible false
  :switch-key "panelSwitch"
  :font "DIN Pro 400"
  :hotkey "Space"
  :font-size 3
  :columns 1
  :panel-columns 3

  (group "Header" :column 3
    (item "开关" :type switch :target "sw1" :value true :default false :desc "描述一")
    (item "反相" :type switch-inv :target "inv1" :value false)
    (item "滑条" :type slider :target "sl1" :min -5 :max 55 :value 7 :unit "px")
    (item "下拉" :type combo :target "co1" :source "_FONTS_" :value "B" :default "A")
    (item "数据" :type data :target "getIAS" :target-name "表  速" :unit "Km/h" :precision 2 :preview-value "500" :hide-when-zero true :value true :default true :visible-when (> value 0) :na-when (> value 9999) :unit-source "getU" :precision-source "getP")
    (item "格式" :type data :target "getX" :format "%.1f" :value true)
    (item "按钮" :type button :target "doIt" :fgcolor "255,100,100" :desc-img "img.png")
    (item "数字串" :type info :value "123")
    (item "小数值" :type info :value 1.5)
    (group "嵌套"
      (item "内层" :type switch :target "n1" :value true))
  )
)
(panel "Second"
  :x 0.5
  :y 0.5
)
"#;

    /// Java ConfigLoader.loadConfig(样本) 的模型转储 (oracle 逐字节)
    const SAMPLE_DUMP_JAVA: &str = concat!(
        "GROUP|采样Alpha|x=0.03125|y=0.123456|alpha=200|hotkey=57|visible=false|font=[DIN Pro 400]|fontSize=3|columns=1|panelColumns=3|switchKey=[panelSwitch]|rows=1
",
        "  ROW|Header|type=HEADER|formula=N|format=[%s]|unit=[]|value=B:true|default=N|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=N|min=0|max=100|groupColumns=3|children=10
",
        "    ROW|开关|type=SWITCH|formula=[sw1]|format=[%s]|unit=[]|value=B:true|default=B:false|fgColor=N|desc=[描述一]|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[sw1]|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|反相|type=SWITCH_INV|formula=[inv1]|format=[%s]|unit=[]|value=B:false|default=B:false|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[inv1]|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|滑条|type=SLIDER|formula=[sl1]|format=[%s]|unit=[px]|value=I:7|default=I:7|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[sl1]|min=-5|max=55|groupColumns=0|children=0
",
        "    ROW|下拉|type=COMBO|formula=[co1]|format=[_FONTS_]|unit=[]|value=S:[B]|default=S:[A]|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[co1]|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|数据|type=DATA|formula=[getIAS]|format=[%s]|unit=[Km/h]|value=B:true|default=B:true|fgColor=N|desc=N|descImg=N|preview=[500]|hideWhenZero=true|precision=2|unitSource=[getU]|precisionSource=[getP]|targetName=[表  速]|visibleWhen=(> value 0)|naWhen=(> value 9999)|property=[getIAS]|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|格式|type=DATA|formula=[getX]|format=[%.1f]|unit=[]|value=B:true|default=B:true|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[getX]|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|按钮|type=BUTTON|formula=[doIt]|format=[%s]|unit=[]|value=N|default=N|fgColor=[255,100,100]|desc=N|descImg=[img.png]|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[doIt]|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|数字串|type=INFO|formula=N|format=[%s]|unit=[]|value=S:[123]|default=S:[123]|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=N|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|小数值|type=INFO|formula=N|format=[%s]|unit=[]|value=D:1.5|default=D:1.5|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=N|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|嵌套|type=HEADER|formula=N|format=[%s]|unit=[]|value=B:true|default=N|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=N|min=0|max=100|groupColumns=0|children=1
",
        "      ROW|内层|type=SWITCH|formula=[n1]|format=[%s]|unit=[]|value=B:true|default=B:true|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[n1]|min=0|max=100|groupColumns=0|children=0
",
        "GROUP|Second|x=0.5|y=0.5|alpha=150|hotkey=0|visible=false|font=N|fontSize=0|columns=2|panelColumns=2|switchKey=N|rows=0
",
    );

    /// Java ConfigLoader.saveConfig 输出 (LF 归一形态; 平台行尾在断言时还原)
    const SAMPLE_SAVED_JAVA: &str = concat!(
        "(panel \"采样Alpha\"
",
        "  :x 0.0313
",
        "  :y 0.1235
",
        "  :alpha 200
",
        "  :visible false
",
        "  :switch-key \"panelSwitch\"
",
        "  :font \"DIN Pro 400\"
",
        "  :hotkey \"Space\"
",
        "  :font-size 3
",
        "  :columns 1
",
        "  :panel-columns 3
",
        "
",
        "
",
        "  (group \"Header\" :column 3
",
        "    (item \"开关\" :type switch :target \"sw1\" :value true :default false :desc \"描述一\")
",
        "    (item \"反相\" :type switch-inv :target \"inv1\" :value false :default false)
",
        "    (item \"滑条\" :type slider :target \"sl1\" :unit \"px\" :min -5 :max 55 :value 7 :default 7)
",
        "    (item \"下拉\" :type combo :target \"co1\" :source \"_FONTS_\" :value \"B\" :default \"A\")
",
        "    (item \"数据\" :type data :target \"getIAS\" :unit \"Km/h\" :target \"getIAS\" :value true :default true :preview-value \"500\" :hide-when-zero true :precision 2 :unit-source \"getU\" :precision-source \"getP\" :target-name \"表  速\" :visible-when (> value 0) :na-when (> value 9999))
",
        "    (item \"格式\" :type data :target \"getX\" :target \"getX\" :format \"%.1f\" :value true :default true)
",
        "    (item \"按钮\" :type button :target \"doIt\" :desc-img \"img.png\" :fgcolor \"255,100,100\")
",
        "    (item \"数字串\" :type info :value 123 :default 123)
",
        "    (item \"小数值\" :type info :value 1.5 :default 1.5)
",
        "    (group \"嵌套\"
",
        "      (item \"内层\" :type switch :target \"n1\" :value true :default true)
",
        "    )
",
        "  )
",
        ")
",
        "
",
        "(panel \"Second\"
",
        "  :x 0.5000
",
        "  :y 0.5000
",
        "  :alpha 150
",
        "  :visible false
",
        "  :font \"\"
",
        "  :panel-columns 2
",
        "
",
        "
",
        ")
",
        "
",
    );

    /// Java 重读 (load(save(load))) 的模型转储 — 与首读的三处 Java 原生不对称:
    /// x/y 被 %.4f 重写 (0.03125→0.0313), 数字形字符串 "123" → Int, 缺省 :font null → ""
    const SAMPLE_DUMP_RELOAD_JAVA: &str = concat!(
        "GROUP|采样Alpha|x=0.0313|y=0.1235|alpha=200|hotkey=57|visible=false|font=[DIN Pro 400]|fontSize=3|columns=1|panelColumns=3|switchKey=[panelSwitch]|rows=1
",
        "  ROW|Header|type=HEADER|formula=N|format=[%s]|unit=[]|value=B:true|default=N|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=N|min=0|max=100|groupColumns=3|children=10
",
        "    ROW|开关|type=SWITCH|formula=[sw1]|format=[%s]|unit=[]|value=B:true|default=B:false|fgColor=N|desc=[描述一]|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[sw1]|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|反相|type=SWITCH_INV|formula=[inv1]|format=[%s]|unit=[]|value=B:false|default=B:false|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[inv1]|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|滑条|type=SLIDER|formula=[sl1]|format=[%s]|unit=[px]|value=I:7|default=I:7|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[sl1]|min=-5|max=55|groupColumns=0|children=0
",
        "    ROW|下拉|type=COMBO|formula=[co1]|format=[_FONTS_]|unit=[]|value=S:[B]|default=S:[A]|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[co1]|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|数据|type=DATA|formula=[getIAS]|format=[%s]|unit=[Km/h]|value=B:true|default=B:true|fgColor=N|desc=N|descImg=N|preview=[500]|hideWhenZero=true|precision=2|unitSource=[getU]|precisionSource=[getP]|targetName=[表  速]|visibleWhen=(> value 0)|naWhen=(> value 9999)|property=[getIAS]|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|格式|type=DATA|formula=[getX]|format=[%.1f]|unit=[]|value=B:true|default=B:true|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[getX]|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|按钮|type=BUTTON|formula=[doIt]|format=[%s]|unit=[]|value=N|default=N|fgColor=[255,100,100]|desc=N|descImg=[img.png]|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[doIt]|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|数字串|type=INFO|formula=N|format=[%s]|unit=[]|value=I:123|default=I:123|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=N|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|小数值|type=INFO|formula=N|format=[%s]|unit=[]|value=D:1.5|default=D:1.5|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=N|min=0|max=100|groupColumns=0|children=0
",
        "    ROW|嵌套|type=HEADER|formula=N|format=[%s]|unit=[]|value=B:true|default=N|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=N|min=0|max=100|groupColumns=0|children=1
",
        "      ROW|内层|type=SWITCH|formula=[n1]|format=[%s]|unit=[]|value=B:true|default=B:true|fgColor=N|desc=N|descImg=N|preview=N|hideWhenZero=false|precision=0|unitSource=N|precisionSource=N|targetName=N|visibleWhen=N|naWhen=N|property=[n1]|min=0|max=100|groupColumns=0|children=0
",
        "GROUP|Second|x=0.5|y=0.5|alpha=150|hotkey=0|visible=false|font=[]|fontSize=0|columns=2|panelColumns=2|switchKey=N|rows=0
",
    );

    /// 解析对拍: 固定样本 → 模型转储 == Java oracle 转储 (逐字段)
    /// (三个 sample 用例各用独立文件名 — cargo test 并行下共用路径有撕裂读窗口)
    #[test]
    fn sample_parse_dump_matches_java_oracle() {
        let p = tmp("sample_parse.cfg");
        fs::write(&p, SAMPLE_CFG).unwrap();
        let groups = load_config(&p);
        let dump = dump_groups(&groups);
        assert_eq!(dump, SAMPLE_DUMP_JAVA);
    }

    /// 写回对拍: 固定样本 → saveConfig 输出 == Java oracle 输出 (逐字节, 平台行尾)
    #[test]
    fn sample_save_bytes_match_java_oracle() {
        let p_in = tmp("sample_save.cfg");
        fs::write(&p_in, SAMPLE_CFG).unwrap();
        let groups = load_config(&p_in);

        let p_out = tmp("sample_saved_rust.cfg");
        save_config(&p_out, &groups);
        let bytes = fs::read(&p_out).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        // LF 归一后与 oracle 逐字节一致
        assert_eq!(text.replace(java_line_separator(), "\n"), SAMPLE_SAVED_JAVA);
        // 行终止符与同平台 Java println 一致 (Windows CRLF)
        if cfg!(windows) {
            assert_eq!(text.matches('\r').count(), text.matches('\n').count());
        }
    }

    /// round-trip: 重读模型 == Java 重读转储; 再存字节稳定 (Java oracle 同构验证)
    #[test]
    fn sample_round_trip_matches_java_oracle() {
        let p_in = tmp("sample_rt_in.cfg");
        fs::write(&p_in, SAMPLE_CFG).unwrap();
        let groups1 = load_config(&p_in);

        let p_mid = tmp("sample_rt1.cfg");
        save_config(&p_mid, &groups1);
        let groups2 = load_config(&p_mid);
        assert_eq!(dump_groups(&groups2), SAMPLE_DUMP_RELOAD_JAVA);

        // save 稳定: 二次保存与一次保存逐字节一致 (Java: save∘load∘save = save)
        let p_fin = tmp("sample_rt2.cfg");
        save_config(&p_fin, &groups2);
        assert_eq!(fs::read(&p_mid).unwrap(), fs::read(&p_fin).unwrap());
    }

    /// 仓库真实 ui_layout.cfg 的解析→保存→再解析 round-trip (tmp 文件):
    /// 自洽性断言 (load∘save 不变量), 输入随仓库演化 — Java 侧同一不变量
    /// 已由 oracle 验证 (模型与字节双稳定), 此处固化 Rust 侧行为一致。
    #[test]
    fn repo_ui_layout_round_trip_self_consistent() {
        let cfg_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../ui_layout.cfg");
        let cfg_path = cfg_path.to_str().unwrap();
        let groups1 = load_config(cfg_path);
        assert!(!groups1.is_empty(), "ui_layout.cfg 应在仓库根且含 panel");

        let p_mid = tmp("repo_rt1.cfg");
        save_config(&p_mid, &groups1);
        let groups2 = load_config(&p_mid);
        assert_eq!(
            dump_groups(&groups1),
            dump_groups(&groups2),
            "round-trip 后模型应自洽"
        );

        let p_fin = tmp("repo_rt2.cfg");
        save_config(&p_fin, &groups2);
        assert_eq!(
            fs::read(&p_mid).unwrap(),
            fs::read(&p_fin).unwrap(),
            "二次保存字节应稳定"
        );
    }
}

