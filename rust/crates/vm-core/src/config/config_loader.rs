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
//! PORT: jnativehook 键码表 (getKeyText/getKeyCodeFromText) 已抽出至
//! key_text.rs (波16 E6)。

use std::fs;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;
use std::sync::RwLock;

use crate::config::sexp_parser::{AtomType, SExp, SExpParser, SList};
// 键码↔文本映射 (波16 E6 抽出至 key_text.rs; :hotkey 装载/写回消费)
use super::key_text::{get_key_code_from_text, get_key_text};
use crate::base::java_compat::{java_double_to_string, java_parse_boolean, java_parse_int};

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
            // Int 原值; Double 走 JLS 5.1.3 (Rust `as i32` 同义, 见 sexp_parser
            // get_int 的 oracle 对拍注释)
            Some(ConfigValue::Int(i)) => *i,
            Some(ConfigValue::Double(d)) => *d as i32,
            Some(v) => java_parse_int(&config_value_to_string(v)).unwrap_or(0),
            None => 0,
        }
    }

    /// Java: `public boolean getBool()`
    /// PORT: null 值时 Java `value.toString()` 抛 NullPointerException **无 catch 传播** —
    /// 保持 panic (调用方 ConfigurationService 波次须知晓此契约)。
    pub fn get_bool(&self) -> bool {
        match &self.value {
            Some(ConfigValue::Bool(b)) => *b,
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

/// Java `String.valueOf(value)` (value 非 null) — 各装箱类型的 toString 格式。
/// PORT: config 域唯一实现 (configuration_service 原同名族副本已收敛于此);
/// Double 分支走 base::java_compat::java_double_to_string。
pub(crate) fn config_value_to_string(v: &ConfigValue) -> String {
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
    // — 净效果 = 行终止符归一为 "\n"; InputStreamReader(UTF-8) 的非法字节替换 U+FFFD
    // ↔ from_utf8_lossy
    let raw = fs::read(path).map_err(|e| format!("java.io.IOException: {e}"))?;
    let content = String::from_utf8_lossy(&raw)
        .replace("\r\n", "\n")
        .replace('\r', "\n");

    let content = content.as_str();

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
mod tests;
