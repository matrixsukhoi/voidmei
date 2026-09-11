//! UI 支撑域黑盒场景 (kernel::ui_support):
//! 行可见条件编译/求值 (row_def) + 机型对比规则提取 (comparison) + 胜负计算。
#![allow(non_snake_case)] // 测试名 = 模块前缀英文 + 中文场景 (风格约定, 豁免蛇形)

use std::collections::HashMap;

use expect_test::expect;
use kernel::formula::registry::FormulaView;
use kernel::ui_support::comparison::{ComparisonCalculator, ComparisonRules, WinState};
use kernel::ui_support::row_def::{compile_cond, Cond};

// ---- FormulaView 最小桩: 环境谓词短名 → 数值 ----

struct View(HashMap<&'static str, f64>);

impl FormulaView for View {
    fn var_value(&self, name: &str) -> Option<f64> {
        self.0.get(name).copied()
    }
}

/// 行可见条件: compile_cond 的编译产物 (Debug 形态) + eval 求值表
#[test]
fn 行条件_编译与求值() {
    let view = View(HashMap::from([
        ("is_jet_engine", 1.0),
        ("is_prop_engine", 0.0),
        ("has_wep", 1.0),
    ]));

    // 注: 快照行首勿带差异化前导空格 (expect-test 公共前缀剥离)
    let cases: Vec<(&str, f64, &View)> = vec![
        ("value > 100", 150.0, &view),
        ("value > 100", 100.0, &view),   // 恰等 → false (严格大于)
        ("value >= 100.5", 100.5, &view), // Gte 含等
        ("value == 1.00005", 1.0, &view), // Eq 0.0001 容差内 → true
        ("value != 5", 5.0, &view),       // NotEq 容差内判等 → false
        ("value < -3.5", -4.0, &view),
        ("!isJetEngine", 0.0, &view),
        ("isJetEngine", 0.0, &view),
        ("hasWep && value > 0", 50.0, &view),
        ("hasWep && value > 0", 0.0, &view),
        ("isPropEngine || hasWep", 0.0, &view), // 左 false 右 true
        ("(value > 10) && !isPropEngine", 20.0, &view),
    ];
    let actual = cases
        .iter()
        .map(|(expr, val, v)| {
            let c = compile_cond(expr).expect("合法表达式应编译");
            format!("{expr} @{val} => {:?} eval={}", c, c.eval(*v, *val))
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        value > 100 @150 => Gt(100.0) eval=true
        value > 100 @100 => Gt(100.0) eval=false
        value >= 100.5 @100.5 => Gte(100.5) eval=true
        value == 1.00005 @1 => Eq(1.00005) eval=true
        value != 5 @5 => NotEq(5.0) eval=false
        value < -3.5 @-4 => Lt(-3.5) eval=true
        !isJetEngine @0 => Not(IsJetEngine) eval=false
        isJetEngine @0 => IsJetEngine eval=true
        hasWep && value > 0 @50 => And(HasWep, Gt(0.0)) eval=true
        hasWep && value > 0 @0 => And(HasWep, Gt(0.0)) eval=false
        isPropEngine || hasWep @0 => Or(IsPropEngine, HasWep) eval=true
        (value > 10) && !isPropEngine @20 => And(Gt(10.0), Not(IsPropEngine)) eval=true"#]]
    .assert_eq(&actual);
}

/// 行条件: 语法错误 → None (调用方按无条件显示处理)
#[test]
fn 行条件_语法错误拒编译() {
    for bad in [
        "value =",   // 单 = 非法
        "value >",   // 缺数字
        "> 100",     // 数字开头
        "value > 100 &&", // 尾部悬空运算符
        "value >≥ 1", // 非法字符
        "unknownPred", // 未知谓词名
        "(value > 1", // 括号失配
    ] {
        assert!(compile_cond(bad).is_none(), "{bad:?} 应编译失败");
    }
    // Cond: PartialEq/Clone 面自查
    assert_eq!(compile_cond("value > 1"), Some(Cond::Gt(1.0)));
}

/// 机型对比规则注册表: 规则存在性 + 各类规则对原始串的数值提取 (expect 表)
#[test]
fn 机型对比_规则清单与数值提取() {
    // 注册表覆盖面: 有规则属性 vs 无规则属性 (无规则 → 平局灰色)
    let has: Vec<(&str, bool)> = vec![
        ("空重(kg)", ComparisonRules::has_rule("空重(kg)")),
        ("最大燃油重量(kg)", ComparisonRules::has_rule("最大燃油重量(kg)")),
        ("临界速度(km/h)", ComparisonRules::has_rule("临界速度(km/h)")),
        ("允许过载(满/半油)", ComparisonRules::has_rule("允许过载(满/半油)")),
        ("主阻力面积因数及加速度系数", ComparisonRules::has_rule("主阻力面积因数及加速度系数")),
        ("散热/油冷器阻力系数", ComparisonRules::has_rule("散热/油冷器阻力系数")),
        ("无规则属性", ComparisonRules::has_rule("无规则属性")),
        ("翼展效率", ComparisonRules::has_rule("翼展效率")),
    ];
    let listing = has
        .iter()
        .map(|(k, v)| format!("{k} = {v}"))
        .collect::<Vec<_>>()
        .join("\n");

    // 各规则族的提取行为 (raw 串 → Option<f64>, lower_better)
    let extracts: Vec<(String, String)> = vec![
        // SimpleRule: 首个数字; 数组串拒收
        (
            ComparisonRules::get("空重(kg)").unwrap().extract_value(Some("4644.0 kg")).map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
            ComparisonRules::get("空重(kg)").unwrap().extract_value(Some("[144, 1167]")).map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
        ),
        // ListIndexRule(1): "[min, max]" 取后者 (vne)
        (
            ComparisonRules::get("临界速度(km/h)").unwrap().extract_value(Some("[144, 1167]")).map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
            ComparisonRules::get("临界速度(km/h)").unwrap().extract_value(Some("[999]")).map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
        ),
        // MultiListIndexRule(0, 1): 第一列表的第二项 (满油负过载)
        (
            ComparisonRules::get("允许过载(满/半油)").unwrap().extract_value(Some("[7, -3], [9, -4]")).map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
            ComparisonRules::get("允许过载(满/半油)").unwrap().extract_value(Some("[8, -2]")).map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
        ),
        // LambdaRule 散热 "X / Y": 两数求和, 总和小好
        (
            ComparisonRules::get("散热/油冷器阻力系数").unwrap().extract_value(Some("0.02 / 0.03")).map(|v| format!("{v:.4}")).unwrap_or_else(|| "null".into()),
            ComparisonRules::get("散热/油冷器阻力系数").unwrap().extract_value(Some("无斜杠串")).map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
        ),
        // LambdaRule 主阻力 "X / Y": 第二个数, 小好
        (
            ComparisonRules::get("主阻力面积因数及加速度系数").unwrap().extract_value(Some("0.41 / 0.00123")).map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
            ComparisonRules::get("主阻力面积因数及加速度系数").unwrap().extract_value(Some("没有数字")).map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
        ),
    ];
    let extract_out = extracts
        .iter()
        .map(|(a, b)| format!("{a} | {b}"))
        .collect::<Vec<_>>()
        .join("\n");

    let actual = format!("== 注册表 ==\n{listing}\n== 提取 ==\n{extract_out}");
    expect![[r#"
        == 注册表 ==
        空重(kg) = true
        最大燃油重量(kg) = true
        临界速度(km/h) = true
        允许过载(满/半油) = true
        主阻力面积因数及加速度系数 = true
        散热/油冷器阻力系数 = true
        无规则属性 = false
        翼展效率 = true
        == 提取 ==
        4644 | null
        1167 | null
        -3 | -2
        0.0500 | null
        0.00123 | null"#]]
    .assert_eq(&actual);
}

/// 对比计算: 零值守卫 / epsilon 平局 / higher-lower 双向胜负 (expect 表)
#[test]
fn 机型对比_胜负计算() {
    let rows: Vec<(f64, f64, bool)> = vec![
        (0.0, 100.0, true),   // val0 == 0 → Unknown
        (100.0, 0.0, true),   // val1 == 0 → Unknown
        (100.0, 100.0005, true),  // diff < 0.001 → Draw
        (100.0, 110.0, true),    // higher better + 正差 → Win
        (100.0, 90.0, true),     // higher better + 负差 → Loss
        (100.0, 110.0, false),   // lower better + 正差 → Loss
        (100.0, 90.0, false),    // lower better + 负差 → Win
    ];
    let actual = rows
        .iter()
        .map(|(a, b, hib)| {
            let r = ComparisonCalculator::compare(*a, *b, *hib);
            format!("{a} vs {b} hib={hib} => {:?} diff={:.2} pct={:.2}%", r.win, r.diff, r.percent)
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        0 vs 100 hib=true => Unknown diff=0.00 pct=0.00%
        100 vs 0 hib=true => Unknown diff=0.00 pct=0.00%
        100 vs 100.0005 hib=true => Draw diff=0.00 pct=0.00%
        100 vs 110 hib=true => Win diff=10.00 pct=10.00%
        100 vs 90 hib=true => Loss diff=-10.00 pct=-10.00%
        100 vs 110 hib=false => Loss diff=10.00 pct=10.00%
        100 vs 90 hib=false => Win diff=-10.00 pct=-10.00%"#]]
    .assert_eq(&actual);
    // WinState 全集形态自查
    assert_eq!(
        format!("{:?}/{:?}/{:?}/{:?}", WinState::Win, WinState::Loss, WinState::Draw, WinState::Unknown),
        "Win/Loss/Draw/Unknown"
    );
}
