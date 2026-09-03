use super::*;

fn extract(prop: &str, raw: Option<&str>) -> Option<f64> {
    ComparisonRules::get(prop)
        .expect("rule registered")
        .extract_value(raw)
}

fn bits(v: Option<f64>) -> Option<u64> {
    v.map(|x| x.to_bits())
}

#[test]
fn all_registered_keys_present_with_direction() {
    // 11 条规则齐全 + isLowerBetter 方向 (Java static 块逐条对照)
    let expect = [
        ("空重(kg)", true),
        ("最大燃油重量(kg)", false),
        ("临界速度(km/h)", false),
        ("允许过载(满/半油)", false),
        ("平均耐热条恢复速率", false),
        ("千米最大升力过载", false),
        ("主升力面积因数载荷", false),
        ("翼展效率", false),
        ("主阻力面积因数及加速度系数", true),
        ("诱导阻力因数及加速度系数", true),
        ("散热/油冷器阻力系数", true),
    ];
    for (key, lower) in expect {
        let r = ComparisonRules::get(key).unwrap_or_else(|| panic!("{key} 未注册"));
        assert_eq!(r.is_lower_better(), lower, "{key}");
    }
}

#[test]
fn missing_property_has_no_rule() {
    // 基线: REG 不存在的属性 NORULE; HAS foo false; HAS 散热/油冷器阻力系数 true
    assert!(ComparisonRules::get("不存在的属性").is_none());
    assert!(!ComparisonRules::has_rule("foo"));
    assert!(ComparisonRules::has_rule("空重(kg)"));
    assert!(ComparisonRules::has_rule("散热/油冷器阻力系数")); // 键本身含 '/'
}

#[test]
fn builtin_rules_extract_via_registry() {
    // 基线: REG 空重 4644.0; [1,2]→null; 最大燃油 705.0; 临界速度 1167.0;
    // 允许过载 (0,1)→-4.2; 耐热 0.87; 千米升力 12.5; 主升力 123.4; 翼展 0.95
    assert_eq!(
        bits(extract("空重(kg)", Some("4644.0"))),
        Some(4661828146700484608)
    );
    assert_eq!(extract("空重(kg)", Some("[1,2]")), None);
    assert_eq!(
        bits(extract("最大燃油重量(kg)", Some("705.0"))),
        Some(4649412461399638016)
    );
    assert_eq!(
        bits(extract("临界速度(km/h)", Some("[144, 1167]"))),
        Some(4652847335724810240)
    );
    assert_eq!(
        bits(extract(
            "允许过载(满/半油)",
            Some("[8.5, -4.2], [10.1, -5.3]")
        )),
        Some((-4606957238818648883_i64) as u64)
    );
    assert_eq!(
        bits(extract("平均耐热条恢复速率", Some("0.87"))),
        Some(4606011482896901079)
    );
    assert_eq!(
        bits(extract("千米最大升力过载", Some("12.5 / 13.0"))),
        Some(4623226492472524800)
    );
    assert_eq!(
        bits(extract("主升力面积因数载荷", Some("123.4"))),
        Some(4638383919968393626)
    );
    assert_eq!(
        bits(extract("翼展效率", Some("0.95"))),
        Some(4606732058837280358)
    );
    // null 原始值 → null
    assert_eq!(extract("空重(kg)", None), None);
}

#[test]
fn slash_second_rule_extracts_number_after_slash() {
    // 基线: 0.25 / 0.35 → 0.35 (空格可有可无); 1 / 2 / 3 → 2 (首个 '/');
    // -1.5 / -2.5 → -2.5; abc / 0.5 → 0.5 ('/' 前内容不参与)
    let prop = "主阻力面积因数及加速度系数";
    assert_eq!(
        bits(extract(prop, Some("0.25 / 0.35"))),
        Some(4599976659396224614)
    );
    assert_eq!(
        bits(extract(prop, Some("0.25/0.35"))),
        Some(4599976659396224614)
    );
    assert_eq!(
        bits(extract(prop, Some("1 / 2 / 3"))),
        Some(4611686018427387904)
    );
    assert_eq!(
        bits(extract(prop, Some("-1.5 / -2.5"))),
        Some((-4610560118520545280_i64) as u64)
    );
    assert_eq!(
        bits(extract(prop, Some("abc / 0.5"))),
        Some(4602678819172646912)
    );
    assert_eq!(
        bits(extract(prop, Some("a/3.5"))),
        Some(4615063718147915776)
    );
}

#[test]
fn slash_second_rule_no_match_returns_none() {
    // 基线: "/" (后无数字) / "0.25 / " (尾无数字) / "1.5/abc" ('/' 后非数字)
    // / "x /y" ('/' 后零空白紧跟非数字) → null
    let prop = "主阻力面积因数及加速度系数";
    assert_eq!(extract(prop, Some("/")), None);
    assert_eq!(extract(prop, Some("0.25 / ")), None);
    assert_eq!(extract(prop, Some("1.5/abc")), None);
    assert_eq!(extract(prop, Some("x /y")), None);
}

#[test]
fn slash_both_rule_sums_two_numbers() {
    // 基线: 0.1 / 0.2 → 0.30000000000000004 (逐位); 1/2 → 3.0;
    // 3.0 / 4.0 / 5.0 → 7.0 (取前两个数); a 1 / 2 b → 3.0; "  0.5/0.6  " → 1.1
    let prop = "散热/油冷器阻力系数";
    assert_eq!(
        bits(extract(prop, Some("0.1 / 0.2"))),
        Some(4599075939470750516)
    );
    assert_eq!(bits(extract(prop, Some("1/2"))), Some(4613937818241073152));
    assert_eq!(
        bits(extract(prop, Some("3.0 / 4.0 / 5.0"))),
        Some(4619567317775286272)
    );
    assert_eq!(
        bits(extract(prop, Some("a 1 / 2 b"))),
        Some(4613937818241073152)
    );
    assert_eq!(
        bits(extract(prop, Some("  0.5/0.6  "))),
        Some(4607632778762754458)
    );
}

#[test]
fn slash_both_rule_backtracks_fraction_part() {
    // 基线: "12.3.4/5" → 3.4 + 5 = 8.4 — 起点处贪婪吞 "12.3" 后 '.' 令
    // `\s*/` 失败, 放弃小数段的回溯同样恒败 ('.' 非 `\s` 非 '/'), 由
    // find() 起点右移至内层数字 '3' 命中 "3.4/5" (leftmost-first 推进)
    assert_eq!(
        bits(extract("散热/油冷器阻力系数", Some("12.3.4/5"))),
        Some(4620918397663497421)
    );
}

#[test]
fn slash_both_rule_no_match_returns_none() {
    // 基线: "1.5/abc" ('/' 后无数字) / "仅/中文" (两侧均无数字) → null
    let prop = "散热/油冷器阻力系数";
    assert_eq!(extract(prop, Some("1.5/abc")), None);
    assert_eq!(extract(prop, Some("仅/中文")), None);
}

#[test]
fn slash_rules_null_and_empty_return_none() {
    // 基线: 空串/null → LambdaRule 前置守卫返回 null
    assert_eq!(extract("散热/油冷器阻力系数", Some("")), None);
    assert_eq!(extract("散热/油冷器阻力系数", None), None);
    // 诱导阻力 (SLASH_SECOND 同款 lambda): 0.10 / 0.20 → 0.2
    assert_eq!(
        bits(extract("诱导阻力因数及加速度系数", Some("0.10 / 0.20"))),
        Some(4596373779694328218)
    );
}
