use super::*;
use crate::sexp_parser::SExpParser;
use std::fs;
use std::path::Path;

/// TestVisibilityExpressionEvaluator.assertEval 对应物:
/// 解析表达式 → 构造 evaluator (source=null) → 断言求值结果
fn assert_eval(expression: &str, value: f64, expected: bool, description: &str) {
    let mut parser = SExpParser::new();
    let exps = parser.parse(expression);

    if exps.is_empty() {
        panic!("{} - 解析表达式失败: {}", description, expression);
    }

    let evaluator = VisibilityExpressionEvaluator::new(Some(exps[0].clone()), None);
    let actual = evaluator.evaluate(value);

    assert_eq!(
        actual, expected,
        "{} (表达式: {}, 值: {})",
        description, expression, value
    );
}

// ---- TestVisibilityExpressionEvaluator.java 移植 (42 断言) ----

/// Java: testComparisons()
#[test]
fn test_comparisons() {
    // (> value 0)
    assert_eval("(> value 0)", 100.0, true, "100 > 0 应为 true");
    assert_eval("(> value 0)", 0.0, false, "0 > 0 应为 false");
    assert_eval("(> value 0)", -10.0, false, "-10 > 0 应为 false");

    // (>= value 0)
    assert_eval("(>= value 0)", 0.0, true, "0 >= 0 应为 true");
    assert_eval("(>= value 0)", -0.0001, false, "-0.0001 >= 0 应为 false");

    // (!= value -65535)
    assert_eval("(!= value -65535)", 100.0, true, "100 != -65535 应为 true");
    assert_eval("(!= value -65535)", -65535.0, false, "-65535 != -65535 应为 false");

    // (= value 1)
    // 容差为 0.0001，所以 |a - b| < 0.0001 视为相等
    assert_eval("(= value 1)", 1.0, true, "1 = 1 应为 true");
    assert_eval("(= value 1)", 1.0001, true, "1.0001 = 1 应为 true (差值 0.0001 < 0.0001 边界)");
    assert_eval("(= value 1)", 1.0002, false, "1.0002 = 1 应为 false (差值 0.0002 > 0.0001)");
    assert_eval("(= value 1)", 1.00005, true, "1.00005 = 1 应为 true (在容差内)");

    // (< value 100)
    assert_eval("(< value 100)", 50.0, true, "50 < 100 应为 true");
    assert_eval("(< value 100)", 100.0, false, "100 < 100 应为 false");

    // (<= value 100)
    assert_eval("(<= value 100)", 100.0, true, "100 <= 100 应为 true");
    assert_eval("(<= value 100)", 100.1, false, "100.1 <= 100 应为 false");
}

/// Java: testLogicalOperators()
#[test]
fn test_logical_operators() {
    // (not (> value 0))
    assert_eval("(not (> value 0))", 100.0, false, "not (100 > 0) 应为 false");
    assert_eval("(not (> value 0))", -10.0, true, "not (-10 > 0) 应为 true");

    // (and (> value 0) (< value 100))
    assert_eval("(and (> value 0) (< value 100))", 50.0, true, "50 在 (0, 100) 范围内");
    assert_eval("(and (> value 0) (< value 100))", 0.0, false, "0 不在 (0, 100) 范围内");
    assert_eval("(and (> value 0) (< value 100))", 100.0, false, "100 不在 (0, 100) 范围内");

    // (or (< value 0) (> value 100))
    assert_eval("(or (< value 0) (> value 100))", -10.0, true, "-10 < 0，满足 or 的第一个条件");
    assert_eval("(or (< value 0) (> value 100))", 150.0, true, "150 > 100，满足 or 的第二个条件");
    assert_eval("(or (< value 0) (> value 100))", 50.0, false, "50 不满足 or 的任何条件");

    // 嵌套 and/or
    assert_eval("(and (or (< value 0) (> value 10)) (< value 100))", -5.0, true, "-5 < 0 且 < 100");
    assert_eval("(and (or (< value 0) (> value 10)) (< value 100))", 5.0, false, "5 在 [0, 10] 之间，不满足 or");
    assert_eval("(and (or (< value 0) (> value 10)) (< value 100))", 150.0, false, "150 > 100，不满足 and 的第二个条件");
}

/// Java: testComplexExpressions()
#[test]
fn test_complex_expressions() {
    // 模拟功率字段: (and (not (isJetEngine)) (> value 0))
    // 由于没有 TelemetrySource，方法调用默认返回 true
    assert_eval("(and (not true) (> value 0))", 100.0, false, "not true = false，整个 and 为 false");
    assert_eval("(and (not false) (> value 0))", 100.0, true, "not false = true，100 > 0，整个为 true");

    // 模拟推力字段: (or (isJetEngine) (> value 0))
    assert_eval("(or true (> value 0))", 0.0, true, "第一个条件 true，整个为 true");
    assert_eval("(or false (> value 0))", 100.0, true, "第一个条件 false，但 100 > 0");
    assert_eval("(or false (> value 0))", 0.0, false, "两个条件都为 false");

    // 模拟加力时: (and (hasWep) (> value 0))
    assert_eval("(and true (> value 0))", 300.0, true, "有加力且值 > 0");
    assert_eval("(and true (> value 0))", 0.0, false, "有加力但值 = 0");
    assert_eval("(and false (> value 0))", 300.0, false, "无加力系统");
}

/// Java: testEdgeCases()
#[test]
fn test_edge_cases() {
    // 空列表
    assert_eval("()", 0.0, true, "空列表应返回 true");

    // 布尔字面量
    assert_eval("true", 0.0, true, "true 字面量");
    assert_eval("false", 0.0, false, "false 字面量");

    // 浮点数比较容差: |a - b| < 0.0001
    assert_eval("(= value 0)", 0.00001, true, "0.00001 应在 0.0001 容差内被视为等于 0");
    assert_eval("(= value 0)", 0.0001, false, "0.0001 刚好在容差边界 (0.0001 >= 0.0001，不相等)");
    assert_eval("(= value 0)", 0.001, false, "0.001 超出容差");

    // 多个 and 子表达式
    assert_eval("(and (> value 0) (< value 100) (> value 10) (< value 90))", 50.0, true, "50 满足所有条件");
    assert_eval("(and (> value 0) (< value 100) (> value 10) (< value 90))", 5.0, false, "5 不满足 > 10");
}

// ---- TestNaWhenBinding.java 相关断言移植 ----

/// Java 测试经 ConfigLoader 找转半径行并绑定 naWhen 求值。ConfigLoader 属 config
/// 批次未译, 此处按同一 ui_layout.cfg 源直接走 SExp 树 (与 sexp_parser 测试的
/// find_turn_radius_na_when 同构), 断言 Java 测试打印的四组求值结果。
#[test]
fn na_when_binding_turn_radius_evaluations() {
    let cfg_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../ui_layout.cfg");
    let content = fs::read_to_string(&cfg_path).expect("ui_layout.cfg 应在仓库根");
    let mut parser = SExpParser::new();
    let panels = parser.parse(&content);
    assert!(!panels.is_empty(), "cfg 应解析出顶层 panel");

    let na_when =
        find_row_na_when(&panels, "getTurnRadius").expect("应找到转半径行的 :na-when 表达式");
    // 对应 Java: "naWhen: " + turnRadiusRow.naWhen → (> value 9999)
    assert_eq!(na_when.to_string(), "(> value 9999)");

    // 对应 Java: df.naWhenEvaluator = new VisibilityExpressionEvaluator(naWhen, null)
    //            + "value=800/9999/10000/50000: false/false/true/true (应为 ...)"
    let evaluator = VisibilityExpressionEvaluator::new(Some(na_when.clone()), None);
    assert!(!evaluator.evaluate(800.0), "value=800 应为 false");
    assert!(!evaluator.evaluate(9999.0), "value=9999 应为 false");
    assert!(evaluator.evaluate(10000.0), "value=10000 应为 true");
    assert!(evaluator.evaluate(50000.0), "value=50000 应为 true");
}

/// 模拟 ConfigLoader.findRow + RowConfig.naWhen: 定位 :target 为 property 的行节点,
/// 返回其 :na-when 关键字的下一个兄弟节点 (找不到返回 None)
fn find_row_na_when(exprs: &[Rc<SExp>], property: &str) -> Option<Rc<SExp>> {
    fn walk(e: &SExp, property: &str) -> Option<Rc<SExp>> {
        let SExp::List(l) = e else {
            return None;
        };
        let has_target = l.children.iter().any(|c| {
            matches!(
                &**c,
                SExp::Atom(a) if a.r#type == AtomType::String && a.get_string() == property
            )
        });
        if has_target {
            let n = l.children.len();
            for i in 0..n {
                if i + 1 < n {
                    if let SExp::Atom(a) = &*l.children[i] {
                        if a.is_keyword() && a.get_string().eq_ignore_ascii_case(":na-when") {
                            return Some(l.children[i + 1].clone());
                        }
                    }
                }
            }
        }
        l.children.iter().find_map(|c| walk(c, property))
    }
    exprs.iter().find_map(|e| walk(e, property))
}

// ---- 边界测试 (期望值 = Java 8 oracle 实测, OpenJDK 1.8.0_342) ----

/// Java: evaluateExp(exp == null) → true (FieldOverlay 对 naWhen 缺失行的短路前置)
#[test]
fn null_expression_defaults_to_visible() {
    let evaluator = VisibilityExpressionEvaluator::new(None, None);
    assert!(evaluator.evaluate(0.0));
}

/// 结构边界 (oracle #2~#13)
#[test]
fn structural_edge_cases_match_oracle() {
    assert_eval("((>) value 0)", 100.0, true, "head 是列表 → 默认为 true");
    assert_eval("(not)", 0.0, true, "not 缺操作数 (size < 2) → true");
    assert_eval("(>)", 5.0, true, "比较缺操作数 (size < 3) → true");
    assert_eval("(> value)", 5.0, true, "比较只有 2 个元素 → true");
    assert_eval("(and)", 0.0, true, "and 无子表达式 → 循环不执行 → true");
    assert_eval("(or)", 0.0, false, "or 无子表达式 → 循环不执行 → false");
    assert_eval(
        "(and (not (not true)) (> value 0))",
        5.0,
        true,
        "嵌套 not: not(not(true)) = true",
    );
    assert_eval(":foo", 0.0, true, "KEYWORD 原子 → 其他原子类型默认为 true");
    assert_eval("5", 0.0, true, "NUMBER 原子 → 其他原子类型默认为 true");
    assert_eval("\"x\"", 0.0, true, "STRING 原子 → 其他原子类型默认为 true");
    assert_eval(
        "True",
        0.0,
        true,
        "SYMBOL \"True\" (tokenizer 精确匹配才成 BOOLEAN) → callMethod 未知方法 → true",
    );
    assert_eval("(fooBar)", 0.0, true, "未知方法列表形式 → default 分支 → true");
}

/// getValue 回退解析 — Java/Rust 双文法共同接受域 (oracle #14, #20~#27)
#[test]
fn get_value_fallback_matches_oracle() {
    assert_eval("(> \"5\" 3)", 0.0, true, "STRING \"5\" → 5.0 > 3");
    assert_eval("(> \"NaN\" 0)", 0.0, false, "STRING \"NaN\" → NaN > 0 恒 false");
    assert_eval("(> NaN 0)", 0.0, false, "NUMBER NaN → getDouble() = NaN → 比较恒 false");
    assert_eval("(> true 0)", 0.0, false, "BOOLEAN 原子 → parse 失败 → 0");
    assert_eval("(< :5 100)", 0.0, true, "KEYWORD 原子 → parse 失败 → 0 < 100");
    assert_eval("(= \"1\" 1)", 0.0, true, "STRING \"1\" → |1-1| < 容差");
    assert_eval("(> \"value\" 0)", 7.0, true, "STRING \"value\" 同样命中关键字 (只比内容)");
    assert_eval(
        "(!= \"value\" 1)",
        1.00005,
        false,
        "STRING value + 容差内相等 → != 为 false",
    );
    assert_eval("(> (and true) 0)", 0.0, false, "列表当操作数 → getValue 返回 0");
}

/// getValue 回退解析 — 已知文法分歧域 (characterization, 见实现处 // PORT: 注释)。
/// Java 8 oracle: (> " 5 " 3)/(> "5f" 3)/(> "0x1p1" 1) 均 true (parseDouble trim/
/// 后缀/十六进制), (> "inf" 0)/(> inf 0) 均 false (parseDouble 拒小写 inf);
/// Rust parse 恰好相反。真实 cfg 无此类操作数, 位级对齐待 sexp_parser 导出
/// java_parse_double 后替换 get_value 的回退行。
#[test]
fn get_value_fallback_known_divergence_from_java() {
    assert_eval("(> \" 5 \" 3)", 0.0, false, "Rust: 空白不 trim → 0 (Java: 5.0 → true)");
    assert_eval("(> \"5f\" 3)", 0.0, false, "Rust: f 后缀拒 → 0 (Java: 5.0 → true)");
    assert_eval("(> \"0x1p1\" 1)", 0.0, false, "Rust: 十六进制拒 → 0 (Java: 2.0 → true)");
    assert_eval("(> \"inf\" 0)", 0.0, true, "Rust: 大小写不敏感 inf → f64::INFINITY (Java: 拒 → 0)");
    assert_eval("(> inf 0)", 0.0, true, "SYMBOL inf 同上 (Java: 拒 → 0)");
}

/// 测试桩: evaluator 调用的 7 个布尔方法按字段返回 (Java oracle 用 reflect.Proxy
/// 同构实现)。PORT: Rust trait 无动态代理, 完整 TelemetrySource 的其余方法只能
/// 全量机械实现 — 测试不触及, 一律返回零值。
struct StubSource {
    is_jet: bool,
    is_prop: bool,
    is_piston: bool,
    is_turboprop: bool,
    is_engine_check_done: bool,
    has_wep: bool,
    has_booster: bool,
}

impl TelemetrySource for StubSource {
    fn is_jet_engine(&self) -> bool {
        self.is_jet
    }
    fn is_prop_engine(&self) -> bool {
        self.is_prop
    }
    fn is_piston_engine(&self) -> bool {
        self.is_piston
    }
    fn is_turboprop_engine(&self) -> bool {
        self.is_turboprop
    }
    fn is_engine_check_done(&self) -> bool {
        self.is_engine_check_done
    }
    fn has_wep(&self) -> bool {
        self.has_wep
    }
    fn has_booster(&self) -> bool {
        self.has_booster
    }
    // ---- 以下方法 evaluator 不调用, 机械零值 ----
    fn get_ias(&self) -> f64 {
        0.0
    }
    fn get_tas(&self) -> f64 {
        0.0
    }
    fn get_mach(&self) -> f64 {
        0.0
    }
    fn get_aoa(&self) -> f64 {
        0.0
    }
    fn get_aos(&self) -> f64 {
        0.0
    }
    fn get_ny(&self) -> f64 {
        0.0
    }
    fn get_vario(&self) -> f64 {
        0.0
    }
    fn get_altitude(&self) -> f64 {
        0.0
    }
    fn get_radio_altitude(&self) -> f64 {
        0.0
    }
    fn is_radio_altitude_valid(&self) -> bool {
        false
    }
    fn get_compass(&self) -> f64 {
        0.0
    }
    fn get_sep(&self) -> f64 {
        0.0
    }
    fn get_acceleration(&self) -> f64 {
        0.0
    }
    fn get_turn_rate(&self) -> f64 {
        0.0
    }
    fn get_turn_radius(&self) -> f64 {
        0.0
    }
    fn is_turn_radius_valid(&self) -> bool {
        false
    }
    fn get_roll_rate(&self) -> f64 {
        0.0
    }
    fn get_energy_jkg(&self) -> f64 {
        0.0
    }
    fn get_mass_fuel(&self) -> f64 {
        0.0
    }
    fn get_total_weight(&self) -> f64 {
        0.0
    }
    fn get_fuel_time_mili(&self) -> i64 {
        0
    }
    fn get_throttle(&self) -> f64 {
        0.0
    }
    fn get_rpm(&self) -> f64 {
        0.0
    }
    fn get_manifold_pressure(&self) -> f64 {
        0.0
    }
    fn get_water_temp(&self) -> f64 {
        0.0
    }
    fn get_oil_temp(&self) -> f64 {
        0.0
    }
    fn get_pitch(&self) -> f64 {
        0.0
    }
    fn get_eff_hp(&self) -> f64 {
        0.0
    }
    fn get_thrust(&self) -> f64 {
        0.0
    }
    fn get_horse_power(&self) -> f64 {
        0.0
    }
    fn get_engine_response(&self) -> f64 {
        0.0
    }
    fn get_prop_efficiency(&self) -> f64 {
        0.0
    }
    fn get_wep_kg(&self) -> f64 {
        0.0
    }
    fn get_wep_time(&self) -> f64 {
        0.0
    }
    fn get_heat_tolerance(&self) -> f64 {
        0.0
    }
    fn get_power_percent(&self) -> f64 {
        0.0
    }
    fn get_manifold_pressure_pounds(&self) -> f64 {
        0.0
    }
    fn get_manifold_pressure_inch_hg(&self) -> f64 {
        0.0
    }
    fn get_manifold_pressure_display(&self) -> f64 {
        0.0
    }
    fn get_manifold_pressure_display_unit(&self) -> String {
        String::new()
    }
    fn get_manifold_pressure_display_precision(&self) -> i32 {
        0
    }
    fn get_unknown_mixture(&self) -> f64 {
        0.0
    }
    fn get_radiator(&self) -> f64 {
        0.0
    }
    fn get_compressor_stage(&self) -> f64 {
        0.0
    }
    fn get_fuel_percent(&self) -> f64 {
        0.0
    }
    fn get_rpm_throttle(&self) -> f64 {
        0.0
    }
    fn get_gear(&self) -> f64 {
        0.0
    }
    fn get_flaps(&self) -> f64 {
        0.0
    }
    fn get_airbrake(&self) -> f64 {
        0.0
    }
    fn get_aileron(&self) -> f64 {
        0.0
    }
    fn get_elevator(&self) -> f64 {
        0.0
    }
    fn get_rudder(&self) -> f64 {
        0.0
    }
    fn get_wing_sweep(&self) -> f64 {
        0.0
    }
    fn is_wing_sweep_valid(&self) -> bool {
        false
    }
    fn get_speed_limit_ratio(&self) -> f64 {
        0.0
    }
    fn get_aileron_lock_ratio(&self) -> f64 {
        0.0
    }
    fn get_rudder_lock_ratio(&self) -> f64 {
        0.0
    }
    fn get_unit_mach_limit_ratio(&self) -> f64 {
        0.0
    }
    fn get_stall_speed(&self) -> f64 {
        0.0
    }
    fn is_imperial(&self) -> bool {
        false
    }
    fn get_aviahorizon_pitch(&self) -> f64 {
        0.0
    }
    fn get_aviahorizon_roll(&self) -> f64 {
        0.0
    }
    fn get_booster_fuel_kg(&self) -> f64 {
        0.0
    }
    fn get_booster_fuel_percent(&self) -> f64 {
        0.0
    }
}

/// 带 source 求值 (辅助)
fn eval_with(expression: &str, value: f64, source: &dyn TelemetrySource) -> bool {
    let mut parser = SExpParser::new();
    let exps = parser.parse(expression);
    assert!(!exps.is_empty(), "解析表达式失败: {}", expression);
    VisibilityExpressionEvaluator::new(Some(exps[0].clone()), Some(source)).evaluate(value)
}

/// callMethod 分派 (oracle #28~#39)
#[test]
fn call_method_dispatches_to_source() {
    // 喷气机桩: isJetEngine/isEngineCheckDone = true, 其余 false
    let jet = StubSource {
        is_jet: true,
        is_prop: false,
        is_piston: false,
        is_turboprop: false,
        is_engine_check_done: true,
        has_wep: false,
        has_booster: false,
    };
    assert!(eval_with("(isJetEngine)", 0.0, &jet));
    assert!(eval_with("(and (isJetEngine) (> value 0))", 100.0, &jet));
    assert!(!eval_with("(and (isJetEngine) (> value 0))", 0.0, &jet));
    assert!(!eval_with("(not (isJetEngine))", 0.0, &jet));
    assert!(!eval_with("(isPropEngine)", 0.0, &jet));
    // 未知方法即使有 source 也默认显示
    assert!(eval_with("(fooBar)", 0.0, &jet));

    // 活塞机桩: isPistonEngine/hasWep = true, hasBooster = false
    let prop = StubSource {
        is_jet: false,
        is_prop: false,
        is_piston: true,
        is_turboprop: false,
        is_engine_check_done: false,
        has_wep: true,
        has_booster: false,
    };
    // ui_layout.cfg 真实形态: 仅活塞机显示进气压
    assert!(eval_with("(and (isPistonEngine) (!= value 1))", 2.0, &prop));
    assert!(!eval_with("(and (isPistonEngine) (!= value 1))", 1.0, &prop));
    assert!(eval_with("(and (hasWep) (> value 0))", 300.0, &prop));
    assert!(!eval_with("(hasBooster)", 0.0, &prop));
    assert!(!eval_with("(or (isTurbopropEngine) (hasBooster))", 0.0, &prop));
    assert!(!eval_with("(isEngineCheckDone)", 0.0, &prop));
}

/// 预览模式: source = None 时方法调用一律 true (Java callMethod 首行)
#[test]
fn call_method_null_source_defaults_true() {
    for method in [
        "(isJetEngine)",
        "(isPropEngine)",
        "(isPistonEngine)",
        "(isTurbopropEngine)",
        "(isEngineCheckDone)",
        "(hasWep)",
        "(hasBooster)",
    ] {
        assert_eval(method, 0.0, true, "source=null 时默认显示");
    }
}
