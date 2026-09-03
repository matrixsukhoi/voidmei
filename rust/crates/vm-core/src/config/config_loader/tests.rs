use super::*;
use std::fs;
use std::path::Path;

// ---- Java 语义辅助的 oracle 对拍 (Java 8, build/oracle 实测) ----

/// String.format("%.4f") — Formatter HALF_UP on 精确十进制展开。
/// 0.03125/0.09375 是精确半点 (dyadic 奇分母 32), Rust {:.4} 半偶会给 0.0312/0.0937。
#[test]
fn java_f_prec4_matches_java8_oracle() {
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
        assert_eq!(java_f(d, 4), want, "{d} → 期望 {want}");
    }
    // NaN/Infinity 分支 (Formatter 原样输出)
    assert_eq!(java_f(f64::NAN, 4), "NaN");
    assert_eq!(java_f(f64::INFINITY, 4), "Infinity");
    assert_eq!(java_f(f64::NEG_INFINITY, 4), "-Infinity");
}

/// %.4f 的 JDK-4511638 已知分歧面固化 (Java 8 oracle 实测):
/// Double.toString(1e23)="9.999999999999999E22" (17 位非最短) → Formatter 展开
/// "%.4f" → "99999999999999990000000.0000"; 本实现取最短 "1E23" →
/// "100000000000000000000000.0000"。对照: 6.02214e23 双方一致 (已在上方 battery)。
/// saveConfig :x/:y 域 (0..1/像素坐标) 该量级不可达 — 见 base::format::java_f 的 JDK-4511638 注记。
#[test]
fn java_f_prec4_jdk_4511638_domain_divergence() {
    assert_eq!(java_f(1.0e23, 4), "100000000000000000000000.0000");
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
    assert_eq!(java_double_to_string(1.0e23), "1.0E23");
    assert_eq!(java_double_to_string(5e-324), "5.0E-324");
    assert_eq!(java_double_to_string(4.9e-324), "5.0E-324");
}

/// getKeyCodeFromText — en_US locale oracle (中文 JDK 的 AWT 属性表本地化
/// 会导致 "Space" 解析失败, 属 Java 侧环境差异, 见 key_text.rs key_text_table PORT 注释)
#[test]
fn key_code_from_text_matches_java8_oracle() {
    let cases = [
        (Some("P"), 25),
        (Some("p"), 25),
        (Some("30"), 30),
        (Some("  25  "), 25),
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
    assert!(is_numeric("-"));
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
    // 掺 PID: 固定名文件在两个测试进程并发跑同一套测试时会互踩
    // (A 进程 fs::write truncate 与 B 进程 read 竞争 → B 读到截断/空文件,
    // 解析组数断言假失败 — 实测于双 cargo test 并行场景, 同 vm_core_vrm
    // 残留缺陷的姊妹面)
    std::env::temp_dir()
        .join(format!(
            "vm_core_config_loader_{}_{name}",
            std::process::id()
        ))
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
    assert_eq!(
        groups.len(),
        1,
        "异常后应保留首个 panel (Java 部分返回语义)"
    );
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
    assert_eq!(
        groups.len(),
        1,
        "非数值关键字值应中止后续加载 (NumberFormatException)"
    );
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
