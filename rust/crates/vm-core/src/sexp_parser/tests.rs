use super::*;
use std::fs;
use std::path::Path;

/// 解析并取各顶层表达式的 Display 形式 (测试辅助)
fn parse_str(s: &str) -> Vec<String> {
    let mut parser = SExpParser::new();
    parser.parse(s).iter().map(|e| e.to_string()).collect()
}

/// 解析并取唯一顶层表达式
fn parse_one(s: &str) -> Rc<SExp> {
    let mut parser = SExpParser::new();
    let es = parser.parse(s);
    assert_eq!(es.len(), 1);
    es.into_iter().next().unwrap()
}

/// 顶层原子序列的 (值, 类型) — 分类测试辅助
fn atom_types(s: &str) -> Vec<(String, AtomType)> {
    let mut parser = SExpParser::new();
    parser
        .parse(s)
        .into_iter()
        .map(|e| {
            let a = e.as_atom();
            (a.get_string().to_string(), a.r#type)
        })
        .collect()
}

// ---- tokenize / parse 边界 ----

#[test]
fn empty_and_whitespace_input_yield_no_expressions() {
    assert_eq!(parse_str(""), Vec::<String>::new());
    assert_eq!(parse_str("   \t\r\n"), Vec::<String>::new());
}

#[test]
fn simple_list_of_symbols() {
    let e = parse_one("(a b c)");
    assert!(e.is_list() && !e.is_atom());
    let l = e.as_list();
    assert_eq!(l.children.len(), 3);
    for c in &l.children {
        assert!(c.is_atom() && c.as_atom().is_symbol());
    }
    assert_eq!(l.children[1].as_atom().get_string(), "b");
    assert_eq!(e.to_string(), "(a b c)");
}

#[test]
fn multiple_top_level_expressions() {
    assert_eq!(parse_str("a b c"), vec!["a", "b", "c"]);
}

#[test]
fn nested_lists() {
    let e = parse_one("(a (b c) d)");
    let l = e.as_list();
    assert_eq!(l.children.len(), 3);
    let inner = l.children[1].as_list();
    assert_eq!(inner.to_string(), "(b c)");
    assert_eq!(e.to_string(), "(a (b c) d)");
}

#[test]
fn empty_list_and_nested_empty() {
    let e = parse_one("()");
    assert_eq!(e.as_list().children.len(), 0);
    assert_eq!(e.to_string(), "()");
    assert_eq!(parse_one("(())").to_string(), "(())");
}

#[test]
fn string_literal_with_spaces() {
    let e = parse_one("(a \"hello world\")");
    let a = e.as_list().children[1].as_atom();
    assert_eq!(a.r#type, AtomType::String);
    assert_eq!(a.get_string(), "hello world");
    assert_eq!(e.to_string(), "(a \"hello world\")");
}

#[test]
fn string_escape_handling() {
    // 输入字符: " a \" b \\ c " — \" → ", \\ → \
    let e = parse_one(r#""a\"b\\c""#);
    let a = e.as_atom();
    assert_eq!(a.r#type, AtomType::String);
    assert_eq!(a.get_string(), "a\"b\\c");
}

#[test]
fn string_escape_keeps_char_verbatim() {
    // Java: sb.append(input.charAt(i)) — \n 收编的是字母 'n', 不解释为换行
    let e = parse_one(r#""\n""#);
    assert_eq!(e.as_atom().get_string(), "n");
}

#[test]
fn string_escape_at_end_appends_backslash() {
    // 末尾孤立反斜杠 (i+1 == len): 条件不满足, 走 else 原样收编 '\'
    let e = parse_one(r#""ab\"#);
    assert_eq!(e.as_atom().get_string(), "ab\\");
}

#[test]
fn unterminated_string_takes_rest() {
    let e = parse_one("\"abc");
    let a = e.as_atom();
    assert_eq!(a.r#type, AtomType::String);
    assert_eq!(a.get_string(), "abc");
}

#[test]
fn quote_does_not_break_atom() {
    // Java 原子定界符不含引号 — a"b" 整体是一个 SYMBOL
    let e = parse_one("a\"b\"c");
    let a = e.as_atom();
    assert!(a.is_symbol());
    assert_eq!(a.get_string(), "a\"b\"c");
}

#[test]
fn semicolon_inside_string_kept() {
    let e = parse_one(r#"("a;b")"#);
    let a = e.as_list().children[0].as_atom();
    assert_eq!(a.r#type, AtomType::String);
    assert_eq!(a.get_string(), "a;b");
}

#[test]
fn comments_skipped_to_end_of_line() {
    assert_eq!(parse_str("; (a)\n(b)"), vec!["(b)"]);
    assert_eq!(parse_str("(a) ; trailing comment"), vec!["(a)"]);
    // \r\n: 注释循环只认 \n, \r 留在注释里, \n 交回外层当空白
    assert_eq!(parse_str("x ; c\r\ny"), vec!["x", "y"]);
}

#[test]
fn comment_without_newline_swallows_rest() {
    assert_eq!(parse_str("(a) ; no newline (b)"), vec!["(a)"]);
    // 未加引号的 a;b — 原子断在 ';', 注释吞掉 b
    assert_eq!(parse_str("a;b"), vec!["a"]);
}

#[test]
fn keyword_atoms() {
    let types = atom_types(":x :type :cols");
    assert!(types.iter().all(|(_, t)| *t == AtomType::Keyword));
    // ':' 前缀优先于布尔/数字判定 — :true 是 KEYWORD 不是 BOOLEAN
    assert_eq!(atom_types(":true :5"), vec![
        (":true".into(), AtomType::Keyword),
        (":5".into(), AtomType::Keyword),
    ]);
}

#[test]
fn boolean_atoms_exact_case() {
    assert_eq!(
        atom_types("true false"),
        vec![
            ("true".into(), AtomType::Boolean),
            ("false".into(), AtomType::Boolean),
        ]
    );
    // tokenizer 用 equals 精确匹配 — "True"/"TRUE" 落 SYMBOL
    assert_eq!(
        atom_types("True TRUE"),
        vec![
            ("True".into(), AtomType::Symbol),
            ("TRUE".into(), AtomType::Symbol),
        ]
    );
}

#[test]
fn number_atom_classification() {
    // oracle 实测 parseDouble 均收 (含 NaN/Infinity/十六进制/后缀)
    assert!(atom_types("123 12.34 -5 +5 1e5 1E-5 .5 5. 5f 5d 1e5f NaN -NaN Infinity -Infinity 0x1p1 0X1.8P1")
        .iter()
        .all(|(_, t)| *t == AtomType::Number));
    // oracle 实测 parseDouble 均拒 → SYMBOL
    assert!(atom_types("5,5 abc 12.34.56 1_000 nan infinity INF 0x8 e5 5-")
        .iter()
        .all(|(_, t)| *t == AtomType::Symbol));
}

#[test]
fn stray_rparen_becomes_symbol_atom() {
    // parseExpression 对非 LPAREN 一律走 parseAtom — 顶层多余的 ) 收编为 SYMBOL 原子
    let types = atom_types("a) b");
    assert_eq!(
        types,
        vec![
            ("a".into(), AtomType::Symbol),
            (")".into(), AtomType::Symbol),
            ("b".into(), AtomType::Symbol),
        ]
    );
    assert_eq!(parse_str("(a))"), vec!["(a)", ")"]);
}

#[test]
fn unclosed_paren_terminates_at_eof() {
    let e = parse_one("(a b");
    assert_eq!(e.as_list().children.len(), 2);
    assert_eq!(e.to_string(), "(a b)");
}

#[test]
fn parser_instance_reusable() {
    // Java 字段 pos/tokens 在 parse() 开头重置 — 同实例多次 parse 互不残留
    let mut parser = SExpParser::new();
    assert_eq!(parser.parse("(a) (b)").len(), 2);
    let second = parser.parse("(x y)");
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].to_string(), "(x y)");
}

#[test]
fn cjk_and_astral_chars_in_atoms() {
    // Vec<char> 逐步推进: CJK/BMP 内与 Java charAt 等价
    let e = parse_one("(速度 🚀)");
    let a = e.as_list().children[1].as_atom();
    assert_eq!(a.get_string(), "🚀");
    assert_eq!(e.to_string(), "(速度 🚀)");
}

// ---- 空白语义 (Character.isWhitespace 复刻) ----

#[test]
fn java_is_whitespace_matches_jdk8_oracle() {
    for c in [
        ' ', '\t', '\n', '\u{b}', '\u{c}', '\r', '\u{1c}', '\u{1d}', '\u{1e}', '\u{1f}',
        '\u{1680}', '\u{180e}', '\u{2000}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{205f}',
        '\u{3000}',
    ] {
        assert!(java_is_whitespace(c), "U+{:04X} 应为空白", c as u32);
    }
    for c in ['\u{85}', '\u{a0}', '\u{2007}', '\u{202f}', '\u{feff}', '\u{1b}', 'a', '0'] {
        assert!(!java_is_whitespace(c), "U+{:04X} 不应为空白", c as u32);
    }
}

#[test]
fn nbsp_is_not_a_delimiter() {
    // U+00A0/U+202F 是 Java 非空白 → 原子的一部分 (与 Rust is_whitespace 相反, 保真点)
    for ws in ['\u{a0}', '\u{202f}'] {
        let src = format!("a{}b", ws);
        let e = parse_one(&src);
        let a = e.as_atom();
        assert!(a.is_symbol());
        assert_eq!(a.get_string(), src);
    }
}

#[test]
fn info_separators_and_mongolian_vowel_split_atoms() {
    // U+001C..U+001F/U+180E 在 JDK8 是空白 → 切分原子 (Rust 原生不切, 保真点)
    for ws in ['\u{1c}', '\u{1f}', '\u{180e}'] {
        let src = format!("a{}b", ws);
        assert_eq!(parse_str(&src), vec!["a", "b"]);
    }
    assert_eq!(parse_str("a\r\nb"), vec!["a", "b"]);
    assert_eq!(parse_str("a\rb"), vec!["a", "b"]);
}

// ---- SAtom getters (Java 8 oracle 数值) ----

#[test]
fn get_double_oracle_table() {
    let cases = [
        ("123", 123.0),
        ("12.34", 12.34),
        (" 42 ", 42.0),        // parseDouble 隐含 trim
        ("\t+5\n", 5.0),
        (".5", 0.5),
        ("5.", 5.0),
        ("+.5", 0.5),
        ("1e5", 100000.0),
        ("1E-5", 1.0e-5),
        ("5f", 5.0),
        ("5d", 5.0),
        ("1.5F", 1.5),
        ("5e2d", 500.0),
        ("5.e2", 500.0),
        (".5f", 0.5),
        ("5.d", 5.0),
        ("0x1p1", 2.0),
        ("0X1.8P1", 3.0),
        ("0x.8p1", 1.0),
        ("0x1.p1", 2.0),
        ("0x8.p1", 16.0),
        ("0x1p1f", 2.0),
        ("0x1p-2", 0.25),
        ("-0x1p2", -4.0),
        ("+0x1p1", 2.0),
        ("0x1P+2", 4.0),
        ("0x1p-1075", 0.0), // oracle: 舍入到 0 (min subnormal 的一半, round-half-even)
        ("2147483647.9", 2147483647.9),
    ];
    for (s, want) in cases {
        let a = SAtom::new(s.into(), AtomType::Number);
        let got = a.get_double();
        assert!(
            (got - want).abs() < f64::EPSILON * want.abs().max(1.0),
            "{} → {} != {}",
            s,
            got,
            want
        );
    }
    // 特殊值
    assert!(SAtom::new("NaN".into(), AtomType::Number).get_double().is_nan());
    assert!(SAtom::new("-NaN".into(), AtomType::Number).get_double().is_nan());
    assert_eq!(SAtom::new("Infinity".into(), AtomType::Number).get_double(), f64::INFINITY);
    assert_eq!(SAtom::new("-Infinity".into(), AtomType::Number).get_double(), f64::NEG_INFINITY);
    assert_eq!(SAtom::new("1e310".into(), AtomType::Number).get_double(), f64::INFINITY);
    // 次正规数: 0 < 1e-310 < 最小正规数
    let sub = SAtom::new("1e-310".into(), AtomType::Number).get_double();
    assert!(sub > 0.0 && sub < f64::MIN_POSITIVE);
}

#[test]
fn get_double_rejects_like_java() {
    for s in [
        "", "-", "+", "1e", "1e+", "1_000", "5,5", "nan", "infinity", "INF", "0x8", "0x8f",
        "0x1p", "0x.p1", "5-", "..5", "5..", "e5", "E5", "+.e5", ".e2", "00x1p1", "0 x1",
        "0x1p 2", "1e 5", "--5", "true", "5.5.5",
    ] {
        assert!(java_parse_double(s).is_err(), "[{}] 应抛 NumberFormatException", s);
    }
}

#[test]
#[should_panic(expected = "For input string")]
fn get_double_panics_like_java_number_format_exception() {
    // STRING 原子非数字 → Java NumberFormatException (未受检) 传播
    SAtom::new("abc".into(), AtomType::String).get_double();
}

#[test]
#[should_panic(expected = "empty String")]
fn get_double_panics_on_empty_string() {
    // Java 8 oracle: parseDouble("")/parseDouble("   ") 抛 NumberFormatException:
    // empty String (小写 e) — 与非空非法串的 "For input string" 消息分支不同
    SAtom::new(String::new(), AtomType::String).get_double();
}

#[test]
#[should_panic(expected = "empty String")]
fn get_double_panics_on_whitespace_only_string() {
    SAtom::new("   ".into(), AtomType::String).get_double();
}

#[test]
fn hex_extreme_exponent_bit_exact() {
    // Java 8 oracle (1.8.0_342) doubleToLongBits 逐例核对 — 单次舍入语义。
    // 直接 `m as f64 * 2f64.powi(shift)` 整体求幂会提前下溢: 前三例旧实现
    // 分别得 0.0/0.0/2.2250738585072014e-308 (2 倍偏差)
    let cases = [
        ("0x40p-1080", 0x1u64),                      // 4.9E-324 最小次正规
        ("0x1fffffffffffff8p-1077", 0x30000000000000), // 8.900295434028806E-308
        ("0x10000000000000p-1075", 0x8000000000000),  // 1.1125369292536007E-308
        ("0x3p-1075", 0x2),                           // 1.0E-323 half-even 舍入
        ("0x1p-1074", 0x1),                           // 4.9E-324
        ("0x1p-1075", 0x0),                           // 半 ulp 舍入到 0 (偶)
        ("0x1p1023", 0x7fe0000000000000),             // 最大正规指数
        ("0x1p1024", 0x7ff0000000000000),             // Infinity
        ("0x1p-2000", 0x0),                           // 深度下溢
        ("0x7fp1", 0x406fc00000000000),               // 254.0 常规域回归
        ("0x1.0000000000001p0", 0x3ff0000000000001),  // 1.0000000000000002
    ];
    for (s, bits) in cases {
        let got = java_parse_double(s).unwrap();
        assert_eq!(got.to_bits(), bits, "{} → {:x} != {:x}", s, got.to_bits(), bits);
    }
}

#[test]
fn get_int_jls_saturation_semantics() {
    // Java (int) double = JLS 5.1.3; Rust f64 as i32 同义 — oracle 逐值核对
    let cases = [
        ("3.99", 3),
        ("-3.99", -3),
        ("0.9999999999", 0),
        ("1e10", 2147483647),
        ("-1e10", i32::MIN),
        ("2.5e9", 2147483647),
        ("-2.5e9", i32::MIN),
        ("2147483647.9", 2147483647),
        ("-2147483648.9", i32::MIN),
        ("NaN", 0),
        ("Infinity", 2147483647),
        ("-Infinity", i32::MIN),
        ("9999", 9999),
    ];
    for (s, want) in cases {
        let a = SAtom::new(s.into(), AtomType::Number);
        assert_eq!(a.get_int(), want, "{}", s);
    }
}

#[test]
fn get_bool_matches_java_parse_boolean() {
    // oracle: "TRUE"/"True" → true; 带空格/其他串 → false
    for s in ["true", "TRUE", "True"] {
        assert!(SAtom::new(s.into(), AtomType::Boolean).get_bool(), "{}", s);
    }
    for s in [" false", "false ", "yes", "", "truetrue", "ｔｒｕｅ", "false"] {
        assert!(!SAtom::new(s.into(), AtomType::Boolean).get_bool(), "{}", s);
    }
}

#[test]
fn get_string_and_type_predicates() {
    let kw = SAtom::new(":type".into(), AtomType::Keyword);
    assert!(kw.is_keyword() && !kw.is_symbol());
    assert_eq!(kw.get_string(), ":type");
    let sym = SAtom::new("panel".into(), AtomType::Symbol);
    assert!(sym.is_symbol() && !sym.is_keyword());
    assert_eq!(sym.get_string(), "panel");
}

// ---- asList/asAtom 异常路径 ----

#[test]
#[should_panic(expected = "Not a list")]
fn as_list_on_atom_panics() {
    parse_one("a").as_list();
}

#[test]
#[should_panic(expected = "Not an atom")]
fn as_atom_on_list_panics() {
    parse_one("(a)").as_atom();
}

// ---- Display (Java toString) ----

#[test]
fn display_list_joins_with_single_space() {
    let mut l = SList::new();
    l.add(Rc::new(SExp::Atom(SAtom::new("a".into(), AtomType::Symbol))));
    l.add(Rc::new(SExp::Atom(SAtom::new("b".into(), AtomType::Symbol))));
    l.add(Rc::new(SExp::Atom(SAtom::new("c".into(), AtomType::Symbol))));
    assert_eq!(l.to_string(), "(a b c)");
}

#[test]
fn display_string_atom_does_not_reescape() {
    // Java toString: 直接拼引号, 内部引号不转义 (忠实保留原行为)
    let a = SAtom::new("he\"llo".into(), AtomType::String);
    assert_eq!(a.to_string(), "\"he\"llo\"");
    // 键值/数字/布尔原子原样输出
    for (v, t) in [
        (":type", AtomType::Keyword),
        ("12.34", AtomType::Number),
        ("true", AtomType::Boolean),
        ("panel", AtomType::Symbol),
    ] {
        assert_eq!(SAtom::new(v.into(), t).to_string(), v);
    }
}

#[test]
fn display_is_virtual_dispatch() {
    // Rc<SExp> 的 Display 派发到运行时类型 (对应 Java toString 虚分派)
    let inner = Rc::new(SExp::List(SList::new()));
    let mut outer = SList::new();
    outer.add(inner.clone());
    outer.add(Rc::new(SExp::Atom(SAtom::new("x".into(), AtomType::Symbol))));
    assert_eq!(outer.to_string(), "(() x)");
    assert_eq!(Rc::new(SExp::List(outer)).to_string(), "(() x)");
}

// ---- :na-when 语义 (TestNaWhenParsing.java 用例移植) ----

#[test]
fn na_when_expression_structure() {
    // ui_layout.cfg 转半径: :na-when (> value 9999)
    let e = parse_one("(> value 9999)");
    let l = e.as_list();
    assert_eq!(l.children.len(), 3);
    assert_eq!(l.children[0].as_atom().get_string(), ">");
    assert!(l.children[0].as_atom().is_symbol());
    assert_eq!(l.children[1].as_atom().get_string(), "value");
    let n = l.children[2].as_atom();
    assert_eq!(n.r#type, AtomType::Number);
    assert_eq!(n.get_double(), 9999.0);
    assert_eq!(n.get_int(), 9999);

    // 复合表达式 (visible-when 形态): (and (not (isJetEngine)) (> value 0))
    let e = parse_one("(and (not (isJetEngine)) (> value 0))");
    let l = e.as_list();
    assert_eq!(l.children.len(), 3);
    assert_eq!(l.children[0].as_atom().get_string(), "and");
    assert_eq!(l.children[1].to_string(), "(not (isJetEngine))");
    assert_eq!(l.children[2].to_string(), "(> value 0)");
    // 与 Java toString 一致 — ConfigLoader.saveConfig 按此回写 cfg
    assert_eq!(
        e.to_string(),
        "(and (not (isJetEngine)) (> value 0))"
    );
}

/// 模拟 ConfigLoader.getKeywordSExp: 递归收集 keyword 后一个兄弟节点
fn collect_keyword_values(exprs: &[Rc<SExp>], keyword: &str, out: &mut Vec<Rc<SExp>>) {
    for e in exprs {
        if let SExp::List(l) = &**e {
            let n = l.children.len();
            for i in 0..n {
                if i + 1 < n {
                    if let SExp::Atom(a) = &*l.children[i] {
                        if a.is_keyword() && a.get_string().eq_ignore_ascii_case(keyword) {
                            out.push(l.children[i + 1].clone());
                        }
                    }
                }
            }
            collect_keyword_values(&l.children, keyword, out);
        }
    }
}

#[test]
fn ui_layout_cfg_na_when_expressions_parsed() {
    // TestNaWhenParsing.java 移植: 加载 ui_layout.cfg, 断言 :na-when / :visible-when
    // 的值都解析成了非空列表 (对应 "naWhen 表达式已解析!" 而非 "[警告] naWhen 为 null!")
    let cfg_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../ui_layout.cfg");
    let content = fs::read_to_string(&cfg_path).expect("ui_layout.cfg 应在仓库根");
    let mut parser = SExpParser::new();
    let panels = parser.parse(&content);
    assert!(!panels.is_empty(), "cfg 应解析出顶层 panel");

    for keyword in [":na-when", ":visible-when"] {
        let mut values = Vec::new();
        collect_keyword_values(&panels, keyword, &mut values);
        assert!(
            !values.is_empty(),
            "{} 在 ui_layout.cfg 中应存在",
            keyword
        );
        for v in &values {
            assert!(v.is_list(), "{} 的值应为表达式列表", keyword);
            assert!(
                !v.as_list().children.is_empty(),
                "{} 表达式不应为空列表",
                keyword
            );
        }
        if keyword == ":na-when" {
            // 当前 cfg 有 7 处 :na-when (grep 核对), 快照防回归
            assert!(values.len() >= 7, ":na-when 数量 {}", values.len());
            let reprs: Vec<String> = values.iter().map(|v| v.to_string()).collect();
            for expect in [
                "(> value 9999)",
                "(<= value 0)",
                "(= value -65535)",
                "(> value 90000)",
                "(<= value -65535)",
            ] {
                assert!(reprs.iter().any(|r| r == expect), "缺 {}", expect);
            }
        }
    }

    // 对应 TestNaWhenParsing 的搜索目标: 转半径行 (target = turn_rds) 的
    // :na-when 表达式确已解析为 (> value 9999)
    let found = find_turn_radius_na_when(&panels);
    assert_eq!(found.as_deref(), Some("(> value 9999)"));
}

fn find_turn_radius_na_when(exprs: &[Rc<SExp>]) -> Option<String> {
    fn walk(e: &SExp) -> Option<String> {
        let SExp::List(l) = e else {
            return None;
        };
        let has_target = l.children.iter().any(|c| {
            matches!(
                &**c,
                SExp::Atom(a) if a.r#type == AtomType::String && a.get_string() == "turn_rds"
            )
        });
        if has_target {
            let n = l.children.len();
            for i in 0..n {
                if i + 1 < n {
                    if let SExp::Atom(a) = &*l.children[i] {
                        if a.is_keyword() && a.get_string().eq_ignore_ascii_case(":na-when") {
                            return Some(l.children[i + 1].to_string());
                        }
                    }
                }
            }
        }
        l.children.iter().find_map(|c| walk(c))
    }
    exprs.iter().find_map(|e| walk(e))
}
