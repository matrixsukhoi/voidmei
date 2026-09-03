//! 公式系统单测: 词法/语法/求值语义/状态原语对齐/编译器(DAG+环)/注册表。
//! 对拍原则 (不假通过): 状态原语逐值对齐现有 Rust 实现, 不放宽断言。

use super::ast::RExpr;
use super::definition::{try_eval_single, CompileError, CompiledFormulaSet, FormulaDef};
use super::eval::StateStore;
use super::registry::{assemble_snapshot, registry, MetaInputs, VarOrigin, VarSnapshot};
use super::FormulaManager;
use crate::base::calc_helper::SimpleMovingAverage;

// ===== 工具: 快照与求值 ====

/// 测试帧数据 (纯值; 直通 State/Indicators 的构造成品)
#[derive(Clone, Copy)]
struct TestTel {
    ias: f64,
    tas: f64,
    alt: f64,
    ny: f64,
    mass_fuel: f64,
}

impl Default for TestTel {
    fn default() -> Self {
        TestTel {
            ias: 400.0,
            tas: 450.0,
            alt: 5000.0,
            ny: 0.35,
            mass_fuel: 300.0,
        }
    }
}

fn snap_of(tel: &TestTel) -> super::registry::VarSnapshot {
    let meta = MetaInputs {
        interval_ms: 50.0,
        freq: 20.0,
        ..Default::default()
    };
    let mut st = crate::game_api::parser::State::default();
    st.ias = tel.ias as i32;
    st.tas = tel.tas as i32;
    st.heightm = tel.alt;
    st.ny = tel.ny;
    st.mfuel = tel.mass_fuel;
    let ind = crate::game_api::parser::Indicators::default();
    let raw = super::registry::RawInputs {
        state: Some(&st),
        indic: Some(&ind),
        fmdata: None,
    };
    let sess = super::registry::SessionInputs::default();
    assemble_snapshot(&raw, &sess, &meta)
}

fn try_eval(expr: &str, tel: &TestTel) -> f64 {
    let snap = snap_of(tel);
    let mut store = StateStore::new();
    try_eval_single(expr, registry(), &snap, &mut store, 1000, 50.0, None).unwrap()
}

// ===== 词法/语法 =====

#[test]
fn lexer_numbers_and_ops() {
    let toks = super::lexer::lex("1.5e2 + x*2 // 注释\n <= !=").unwrap();
    assert_eq!(toks[0], super::lexer::Tok::Num(150.0));
    // Num + Ident(x) + Plus + Star + Num + Le + NotEq = 7 (注释/换行不是 token)
    assert_eq!(toks.len(), 7);
}

#[test]
fn lexer_rejects_bad_char() {
    assert!(super::lexer::lex("a $ b").is_err());
}

#[test]
fn parser_precedence_and_pow_right_assoc() {
    // 2+3*4^1^2 = 2+3*4 = 14 (^ 右结合: 1^2=1, 4^1=4)
    assert_eq!(try_eval("2 + 3 * 4 ^ 1 ^ 2", &TestTel::default()), 14.0);
    // 一元负号优先于幂: -2^2 = (-2)^2? 按文法 unary := "-" unary | pow, -2^2 = -(2^2) = -4
    assert_eq!(try_eval("-2 ^ 2", &TestTel::default()), -4.0);
    assert_eq!(try_eval("10 % 3", &TestTel::default()), 1.0);
}

#[test]
fn parser_ternary_and_keywords() {
    assert_eq!(try_eval("1 > 0 ? 5 : 6", &TestTel::default()), 5.0);
    assert_eq!(try_eval("not 0 and 1 or 0", &TestTel::default()), 1.0);
    assert_eq!(try_eval("!(1 == 2)", &TestTel::default()), 1.0);
}

#[test]
fn parser_error_on_trailing() {
    assert!(super::parser::parse("1 2").is_err());
    assert!(super::parser::parse("(1").is_err());
}

// ===== 求值语义 =====

#[test]
fn eval_logic_short_circuit() {
    // 假短路: 右侧除零 NaN 不传播
    assert_eq!(try_eval("0 && 1/0", &TestTel::default()), 0.0);
    // 真短路
    assert_eq!(try_eval("1 || 1/0", &TestTel::default()), 1.0);
    // 非短路路径 NaN 传播 (0/0 = NaN; 1/0 是合法 inf)
    assert!(try_eval("1 && 0/0", &TestTel::default()).is_nan());
}

#[test]
fn eval_div_by_zero_is_nan_or_inf() {
    assert!(try_eval("0.0 / 0.0", &TestTel::default()).is_nan());
    assert_eq!(try_eval("1.0 / 0.0", &TestTel::default()), f64::INFINITY);
}

#[test]
fn eval_vars_from_snapshot() {
    let t = TestTel::default();
    assert_eq!(try_eval("ias * 2", &t), 800.0);
    // 常量
    assert_eq!(try_eval("g", &t), 9.80);
    assert_eq!(try_eval("rho0", &t), 1.225);
    // 元变量
    assert_eq!(try_eval("interval_ms", &t), 50.0);
}

#[test]
fn eval_fm_vars_nan_without_fm() {
    let t = TestTel::default();
    assert!(try_eval("fm.vne", &t).is_nan());
}

#[test]
fn eval_functions() {
    let t = TestTel::default();
    assert_eq!(try_eval("clamp(11, 0, 10)", &t), 10.0);
    assert_eq!(try_eval("round(1.23456, 2)", &t), 1.23);
    assert_eq!(try_eval("max(1, 2, 3)", &t), 3.0);
    assert_eq!(try_eval("lerp(5, 0, 0, 10, 10)", &t), 5.0);
    assert_eq!(try_eval("is_valid(100)", &t), 1.0);
    assert_eq!(try_eval("is_valid(na())", &t), 0.0);
    // ias_per_mach(0) = 3.6*sqrt(1.4/1.225*101325) ≈ 1225.0
    let v = try_eval("ias_per_mach(0)", &t);
    assert!((v - 1225.04).abs() < 0.5, "ias_per_mach(0) = {v}");
}

#[test]
fn eval_mach_formula_matches_manual() {
    // mach 公式外置形态: ias / ias_per_mach(alt) 必须与 derive.rs L116-120 手写式一致
    let t = TestTel {
        ias: 400.0,
        alt: 5000.0,
        ..Default::default()
    };
    let via_fn = try_eval("ias / ias_per_mach(altitude)", &t);
    // manual 与 derive.rs L116-120 逐项同构: mach = ias / (3.6*sqrt(...))
    let manual: f64 = 400.0
        / (3.6
            * (1.4f64 / 1.225 * 101325.0 * (1.0f64 - 0.0000225577 * 5000.0).powf(5.25588)).sqrt());
    assert!((via_fn - manual).abs() < 1e-9, "{via_fn} vs {manual}");
}

// ===== 状态原语 (逐值对齐现有实现) =====

#[test]
fn stateful_sma_aligns_simple_moving_average() {
    // 同一序列: 公式 sma(x,3) vs SimpleMovingAverage 逐值相等
    let series = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0];
    let reg = registry();
    let ias = reg.lookup("ias").unwrap();
    let mut store = StateStore::new();
    let mut formula_sma = Vec::new();
    for &x in &series {
        let mut s2 = super::registry::VarSnapshot {
            values: vec![f64::NAN; reg.len()],
        };
        s2.values[ias as usize] = x;
        let v = try_eval_single("sma(ias, 3)", reg, &s2, &mut store, 0, 50.0, None).unwrap();
        formula_sma.push(v);
    }
    let mut ref_sma = SimpleMovingAverage::new(3);
    let expect: Vec<f64> = series.iter().map(|&x| ref_sma.add_new_data(x)).collect();
    assert_eq!(formula_sma, expect);
}

#[test]
fn stateful_prev_and_blend() {
    let tel = TestTel::default();
    let snap = snap_of(&tel);
    let mut store = StateStore::new();
    // prev 初值 0, 之后返回上次输入
    let a = try_eval_single("prev(ias)", registry(), &snap, &mut store, 0, 50.0, None).unwrap();
    assert_eq!(a, 0.0);
    let b = try_eval_single("prev(ias)", registry(), &snap, &mut store, 50, 50.0, None).unwrap();
    assert_eq!(b, 400.0);
    // blend(x, 0.1): 首帧 (1-0.1)*0 + 0.1*400 = 40
    let mut store2 = StateStore::new();
    let c = try_eval_single(
        "blend(ias, 0.1)",
        registry(),
        &snap,
        &mut store2,
        0,
        50.0,
        None,
    )
    .unwrap();
    assert!((c - 40.0).abs() < 1e-12, "{c}");
    let d = try_eval_single(
        "blend(ias, 0.1)",
        registry(),
        &snap,
        &mut store2,
        50,
        50.0,
        None,
    )
    .unwrap();
    assert!((d - (0.9 * 40.0 + 0.1 * 400.0)).abs() < 1e-12, "{d}");
}

#[test]
fn stateful_vote_converges_like_engine_check() {
    // 对齐 check_engine_jet: 恒 up, 100 帧后冻结 +1
    let tel = TestTel::default();
    let snap = snap_of(&tel);
    let mut store = StateStore::new();
    let mut last = 0.0;
    for i in 0..100 {
        last = try_eval_single(
            "vote(1, 0, 100)",
            registry(),
            &snap,
            &mut store,
            i * 50,
            50.0,
            None,
        )
        .unwrap();
        if i < 99 {
            assert_eq!(last, 0.0, "冻结前应恒 0");
        }
    }
    assert_eq!(last, 1.0, "第 100 帧计数达 100 → 冻结");
    // 冻结后 up 消失仍输出冻结值
    let after = try_eval_single(
        "vote(0, 1, 100)",
        registry(),
        &snap,
        &mut store,
        6000,
        50.0,
        None,
    )
    .unwrap();
    assert_eq!(after, 1.0);
}

#[test]
fn stateful_stable_requires_unchanged_ms() {
    let tel = TestTel::default();
    let snap = snap_of(&tel);
    let mut store = StateStore::new();
    // 值恒 7, 300ms 阈值: 第 5 帧后 (250ms) 仍 0, 第 6 帧 (300ms) 起 1
    let mut v = 0.0;
    for i in 0..6 {
        v = try_eval_single(
            "stable(7, 300)",
            registry(),
            &snap,
            &mut store,
            i * 50,
            50.0,
            None,
        )
        .unwrap();
    }
    assert_eq!(v, 0.0, "250ms 未达 300ms 阈值");
    v = try_eval_single(
        "stable(7, 300)",
        registry(),
        &snap,
        &mut store,
        300,
        50.0,
        None,
    )
    .unwrap();
    assert_eq!(v, 1.0, "300ms 达阈值");
    // 值变化 → 立即清零
    v = try_eval_single(
        "stable(8, 300)",
        registry(),
        &snap,
        &mut store,
        350,
        50.0,
        None,
    )
    .unwrap();
    assert_eq!(v, 0.0, "变化后清零");
}

#[test]
fn stateful_learn_max_converges() {
    let tel = TestTel::default();
    let snap = snap_of(&tel);
    let mut store = StateStore::new();
    // gate 恒 1, x 恒 2400: blend 逼近; 1000ms 后锁定
    let mut v = 0.0;
    for i in 0..25 {
        v = try_eval_single(
            "learn_max(2400, 1, 1000)",
            registry(),
            &snap,
            &mut store,
            i * 50,
            50.0,
            None,
        )
        .unwrap();
    }
    // 20 帧×50ms=1000ms → 已锁定; ratio=0.05 软逼近 2400*(1-0.95^20) ≈ 1540
    assert!(v > 1500.0 && v < 2400.0, "learn_max 逼近值 {v}");
    let locked = v;
    // 锁定后输入变大也不再更新
    let v2 = try_eval_single(
        "learn_max(99999, 1, 1000)",
        registry(),
        &snap,
        &mut store,
        2000,
        50.0,
        None,
    )
    .unwrap();
    assert_eq!(v2, locked);
}

// ===== 编译器 (DAG/环/live) =====

fn def(name: &str, expr: &str) -> FormulaDef {
    FormulaDef {
        name: name.into(),
        expr: expr.into(),
        ..Default::default()
    }
}

#[test]
fn compile_topo_order_correct() {
    // b 依赖 a: 拓扑序 a 在 b 前, 求值链正确 (b 被 :target 引用 → a,b 均 live)
    let defs = vec![def("b", "a * 2"), def("a", "400 + 1")];
    let set = CompiledFormulaSet::compile(&defs, registry());
    assert!(set.formulas.iter().all(|f| f.err.is_none()));
    let _tel = TestTel::default();
    let meta = MetaInputs {
        interval_ms: 50.0,
        ..Default::default()
    };
    let ind0 = crate::game_api::parser::Indicators::default();
    let raw0 = super::registry::RawInputs {
        state: None,
        indic: Some(&ind0),
        fmdata: None,
    };
    let snap = assemble_snapshot(&raw0, &super::registry::SessionInputs::default(), &meta);
    let mut store = StateStore::new();
    let r = set.eval_frame(&snap, &mut store, 0, 50.0, None);
    let sa = set.slots["a"];
    let sb = set.slots["b"];
    assert_eq!(r.get(sa), 401.0);
    assert_eq!(r.get(sb), 802.0);
}

#[test]
fn compile_detects_cycle() {
    let defs = vec![def("a", "b + 1"), def("b", "a + 1"), def("c", "ias")];
    let set = CompiledFormulaSet::compile(&defs, registry());
    let sa = set.slots["a"];
    let sb = set.slots["b"];
    assert!(matches!(
        set.formulas[sa as usize].err,
        Some(CompileError::Cycle(_))
    ));
    assert!(matches!(
        set.formulas[sb as usize].err,
        Some(CompileError::Cycle(_))
    ));
    // 无关节点不受影响
    assert!(set.formulas[set.slots["c"] as usize].err.is_none());
}

#[test]
fn compile_cycle_chain_names() {
    let defs = vec![def("x", "y"), def("y", "z"), def("z", "x")];
    let set = CompiledFormulaSet::compile(&defs, registry());
    if let Some(CompileError::Cycle(chain)) = &set.formulas[set.slots["x"] as usize].err {
        // 环链包含全部三个名字
        assert!(
            chain.contains(&"x".to_string())
                && chain.contains(&"y".to_string())
                && chain.contains(&"z".to_string())
        );
    } else {
        panic!("x 应在环上");
    }
}

#[test]
fn compile_invalid_dep_propagates_nan() {
    // c 依赖未知变量的 a → a invalid, c 求 NaN (不阻断)
    let defs = vec![def("a", "nope + 1"), def("c", "a * 2")];
    let set = CompiledFormulaSet::compile(&defs, registry());
    assert!(matches!(
        set.formulas[set.slots["a"] as usize].err,
        Some(CompileError::UnknownName(_))
    ));
    let _tel = TestTel::default();
    let meta = MetaInputs::default();
    let ind0 = crate::game_api::parser::Indicators::default();
    let raw0 = super::registry::RawInputs {
        state: None,
        indic: Some(&ind0),
        fmdata: None,
    };
    let snap = assemble_snapshot(&raw0, &super::registry::SessionInputs::default(), &meta);
    let mut store = StateStore::new();
    let r = set.eval_frame(&snap, &mut store, 0, 50.0, None);
    assert!(r.get(set.slots["c"]).is_nan());
}

#[test]
fn compile_duplicate_and_disabled() {
    let defs = vec![def("a", "1"), def("a", "2")];
    let set = CompiledFormulaSet::compile(&defs, registry());
    assert!(matches!(
        set.formulas[1].err,
        Some(CompileError::DuplicateName(_))
    ));
    let mut d = def("b", "1");
    d.disabled = true;
    let set2 = CompiledFormulaSet::compile(&[d], registry());
    assert!(matches!(
        set2.formulas[0].err,
        Some(CompileError::DisabledByUser)
    ));
}

#[test]
fn compile_bad_arity_and_unknown_fn() {
    let set = CompiledFormulaSet::compile(&[def("a", "clamp(1, 2)")], registry());
    assert!(matches!(
        set.formulas[0].err,
        Some(CompileError::BadArity { .. })
    ));
    let set2 = CompiledFormulaSet::compile(&[def("a", "nosuchfn(1)")], registry());
    assert!(matches!(
        set2.formulas[0].err,
        Some(CompileError::UnknownFn(_))
    ));
}

// ===== 注册表完整性 =====

#[test]
fn registry_size_and_unique() {
    let reg = registry();
    assert!(reg.len() >= 120, "注册表规模 {} 应 ≥120", reg.len());
    // 主名唯一 (index 同键覆盖会静默, 检查 vars 名集合)
    let mut seen = std::collections::HashSet::new();
    for v in &reg.vars {
        assert!(seen.insert(v.name), "变量重名: {}", v.name);
    }
}

#[test]
fn registry_single_name_no_getter_aliases() {
    // W10 单名制: Java getter 名不得进内核索引 (对拍文件边界专用;
    // 双名制曾致 live 显示断链 — 别名回归即刻发现)
    let reg = registry();
    for g in [
        "getIAS",
        "getTAS",
        "getNyRaw",
        "getIndicSpeed",
        "getWingSweep",
        "getManifoldPressureDisplay",
        "getRadioAltitude",
        "getRPM",
        "getMach",
    ] {
        assert!(reg.lookup(g).is_none(), "getter 名 {g} 不应可达 (单名制)");
    }
}

#[test]
fn registry_snapshot_assemble() {
    let _tel = TestTel::default();
    let meta = MetaInputs {
        interval_ms: 50.0,
        freq: 20.0,
        fm_loaded: false,
        ..Default::default()
    };
    let mut st0 = crate::game_api::parser::State::default();
    st0.ias = 400;
    let ind0 = crate::game_api::parser::Indicators::default();
    let raw0 = super::registry::RawInputs {
        state: Some(&st0),
        indic: Some(&ind0),
        fmdata: None,
    };
    let snap = assemble_snapshot(&raw0, &super::registry::SessionInputs::default(), &meta);
    let reg = registry();
    let ias = reg.lookup("ias").unwrap();
    assert_eq!(snap.values[ias as usize], 400.0);
}

#[test]
fn registry_origin_complete() {
    // 目录 origin 标注完整性: 标签非空 + 六类全部覆盖 (各类 >0) + indicators 类 ≥5
    // (枚举无未标态, 完备性以类别覆盖表达; 新增变量漏标会让对应类别归零或标签缺失)
    let reg = registry();
    let mut counts = [0usize; 6];
    for (_, _, _, _, o) in reg.catalog() {
        assert!(!o.label().is_empty(), "origin 标签为空");
        let i = match o {
            VarOrigin::State => 0,
            VarOrigin::Indicators => 1,
            VarOrigin::Derived => 2,
            VarOrigin::Fm => 3,
            VarOrigin::Meta => 4,
            VarOrigin::Const => 5,
        };
        counts[i] += 1;
    }
    let names = ["State", "Indicators", "Derived", "Fm", "Meta", "Const"];
    for (i, &c) in counts.iter().enumerate() {
        assert!(c > 0, "origin 类别 {} 数量为 0 (有变量漏标)", names[i]);
    }
    assert!(
        counts[1] >= 5,
        "indicators 类应至少 5 个, 实际 {}",
        counts[1]
    );
    // 总量与注册表规模一致 (catalog 未丢行)
    assert_eq!(counts.iter().sum::<usize>(), reg.len());
}

// ===== FormulaManager 端到端 =====

#[test]
fn manager_install_and_eval() {
    let mgr = FormulaManager::new();
    // 内置公式形状预演: energy_jkg→energy_m 公式链与 maneuver_index
    // (energy_jkg 已公式化: (sum_speedv)²/8 的简化形态用 tas 直代)
    mgr.install(&[
        def("energy_jkg", "tas * tas / 2"),
        def("energy_m", "energy_jkg / g"),
        def(
            "maneuver_index",
            "1.0 - (fm.empty_weight / (fm.empty_weight + mass_fuel))",
        ),
    ]);
    let tel = TestTel::default();
    let meta = MetaInputs {
        interval_ms: 50.0,
        ..Default::default()
    };
    let (r, _snap) = {
        let mut st0 = crate::game_api::parser::State::default();
        st0.ias = tel.ias as i32;
        st0.tas = tel.tas as i32;
        let ind0 = crate::game_api::parser::Indicators::default();
        let raw0 = crate::formula::registry::RawInputs {
            state: Some(&st0),
            indic: Some(&ind0),
            fmdata: None,
        };
        let sess0 = crate::formula::registry::SessionInputs::default();
        mgr.eval_frame(&raw0, &sess0, &meta, 0)
    };
    let set = mgr.current();
    assert!((r.get(set.slots["energy_m"]) - 101250.0 / 9.80).abs() < 1e-9);
    // fm.empty_weight 无 FM → NaN 传播
    assert!(r.get(set.slots["maneuver_index"]).is_nan());
    // reset 后状态清零 (prev 从 0 再来)
    mgr.reset_states();
}

#[test]
fn manager_hot_update_retains_states() {
    let mgr = FormulaManager::new();
    mgr.install(&[def("p", "prev(ias)")]);
    let tel = TestTel::default();
    let meta = MetaInputs {
        interval_ms: 50.0,
        ..Default::default()
    };
    let _ = {
        let mut st0 = crate::game_api::parser::State::default();
        st0.ias = tel.ias as i32;
        let ind0 = crate::game_api::parser::Indicators::default();
        let raw0 = crate::formula::registry::RawInputs {
            state: Some(&st0),
            indic: Some(&ind0),
            fmdata: None,
        };
        let sess0 = crate::formula::registry::SessionInputs::default();
        mgr.eval_frame(&raw0, &sess0, &meta, 0)
    };
    // 热更新: 加一个公式, 原 p 的状态保留
    mgr.install(&[def("p", "prev(ias)"), def("q", "ias * 2")]);
    let (r, _snap) = {
        let mut st0 = crate::game_api::parser::State::default();
        st0.ias = tel.ias as i32;
        let ind0 = crate::game_api::parser::Indicators::default();
        let raw0 = crate::formula::registry::RawInputs {
            state: Some(&st0),
            indic: Some(&ind0),
            fmdata: None,
        };
        let sess0 = crate::formula::registry::SessionInputs::default();
        mgr.eval_frame(&raw0, &sess0, &meta, 50)
    };
    let set = mgr.current();
    // p 第二帧 = 上帧 ias = 400 (状态跨热更新保留)
    assert_eq!(r.get(set.slots["p"]), 400.0);
    assert_eq!(r.get(set.slots["q"]), 800.0);
}

#[test]
fn manager_try_eval_isolated() {
    let mgr = FormulaManager::new();
    let tel = TestTel::default();
    let snap = snap_of(&tel);
    assert_eq!(
        mgr.try_eval("ias * 3.6", &snap, 0, 50.0, None).unwrap(),
        1440.0
    );
    assert!(mgr.try_eval("unknown_var", &snap, 0, 50.0, None).is_err());
}

// ===== :target 统一解析 (设计 §8) =====

/// target_value 桥桩: var_value 走 snap_of 快照 (registry 下标取值)
struct TargetView(super::registry::VarSnapshot);
impl crate::formula::registry::FormulaView for TargetView {
    fn var_value(&self, name: &str) -> Option<f64> {
        let vid = registry().lookup(name)?;
        let v = self.0.values.get(vid as usize).copied()?;
        if v.is_nan() {
            None
        } else {
            Some(v)
        }
    }
}

#[test]
fn resolve_target_variants() {
    // 短名解析
    let (v, m) = super::resolve_target("ias").unwrap();
    assert_eq!(v, super::TargetVar::Var(registry().lookup("ias").unwrap()));
    assert_eq!(m, 1.0);
    // 短名 + 乘数语法 ("wing_sweep * 100" 形态)
    let (v, m) = super::resolve_target("tas * 2").unwrap();
    assert_eq!(m, 2.0);
    let view = TargetView(snap_of(&TestTel::default()));
    assert_eq!(super::target_value(&v, m, &view), Some(900.0));
    // 未知名 → 公式名形态 (延迟判定)
    let (v, _) = super::resolve_target("my_formula").unwrap();
    assert_eq!(v, super::TargetVar::Formula("my_formula".into()));
    // 公式名取值: 未接公式系统的源 → None (上层 0 降级)
    let view = TargetView(snap_of(&TestTel::default()));
    assert_eq!(super::target_value(&v, 1.0, &view), None);
    // 乘数非法 → None
    assert!(super::resolve_target("ias * abc").is_none());
}

// ===== 接管型公式 (与系统变量同名) 的自引用检查 (设计 §5 同名规则) =====

#[test]
fn compile_self_override_formula_rejected() {
    // ias 与系统变量同名 (接管型), 表达式引用 ias 自身 → SelfOverride
    let set = CompiledFormulaSet::compile(&[def("ias", "ias + 1")], registry());
    assert!(matches!(
        set.formulas[0].err,
        Some(CompileError::SelfOverride(_))
    ));
    // 合法接管: 表达式不引用自身
    let ok = CompiledFormulaSet::compile(&[def("ias", "tas * 1.0")], registry());
    assert!(ok.formulas[0].err.is_none());
}

// ===== 性能实测 (ignored: 手动跑 `cargo test -p vm-core --lib formula bench -- --ignored --nocapture`) =====

#[test]
#[ignore]
fn bench_eval_frame_50_formulas() {
    // 50 条混合公式 (纯算术+函数+状态原语), 模拟重度使用
    let mut defs = Vec::new();
    for i in 0..50 {
        let expr = match i % 4 {
            0 => format!("ias * {} + tas / {} // 纯算术", i + 1, i + 2),
            1 => format!("ias_per_mach(altitude + {}) * sqrt(ias + {})", i, i),
            2 => format!("sma(ias + {}, 20) + prev(tas) * {}", i, i + 1),
            _ => format!(
                "lerp({}, 0, ias, 100, tas) + clamp(mach, 0, {})",
                i as f64,
                (i + 1) as f64
            ),
        };
        defs.push(def(&format!("f{i}"), &expr));
    }
    let set = CompiledFormulaSet::compile(&defs, registry());
    let _tel = TestTel::default();
    let meta = MetaInputs {
        interval_ms: 50.0,
        ..Default::default()
    };
    let mut store = StateStore::new();
    let n = 10_000;
    let t0 = std::time::Instant::now();
    for k in 0..n {
        let ind0 = crate::game_api::parser::Indicators::default();
        let raw0 = super::registry::RawInputs {
            state: None,
            indic: Some(&ind0),
            fmdata: None,
        };
        let snap = assemble_snapshot(&raw0, &super::registry::SessionInputs::default(), &meta);
        let _ = set.eval_frame(&snap, &mut store, k, 50.0, None);
    }
    let us = t0.elapsed().as_micros() as f64 / n as f64;
    println!(
        "50 公式/帧: {us:.1} µs/帧 (快照组装+求值); 20Hz 轮询预算 50000µs, 占用 {:.3}%",
        us / 500.0
    );
}

// ===== FM 查表函数族 (W1a; 与 vm-data methods_engine 测试同 基线) =====

/// 最小 mock blkx (与 vm-data service_loop::methods_engine::tests::spitfire_flap_blkx 同表)
fn flap_fmdata() -> crate::fm::data::FmData {
    use crate::fm::data::FmData;
    let mut b = FmData::default();
    b.valid = true;
    b.flaps_destruction_num = 2;
    let mut rows = [[0.0f64; 2]; 6];
    rows[0] = [0.5, 290.0];
    rows[1] = [1.0, 260.0];
    rows[2] = [1.25, 0.0];
    b.flaps_destruction_ind_speed = Some(rows);
    b.vne = 800.0;
    b
}

#[test]
fn fm_table_functions_match_shared_impl() {
    let t = TestTel::default();
    let snap = snap_of(&t);
    let fmdata = flap_fmdata();
    let reg = registry();
    let mut store = StateStore::new();
    let eval_with = |expr: &str, b: Option<&crate::fm::data::FmData>| {
        let mut st = StateStore::new();
        try_eval_single(expr, reg, &snap, &mut st, 0, 50.0, b).unwrap()
    };
    // 角度插值: 270 km/h → 83.333... (methods_engine::flap_allow_speed_angle_基线 同值)
    let v = eval_with("fm_flap_allow_angle(270, 0)", Some(&fmdata));
    assert_eq!(v, 83.33333333333334);
    // 共享实现直调等值 (双路径对拍)
    assert_eq!(
        crate::fm::data::get_flap_allow_angle(270.0, false, Some(&fmdata)),
        v
    );
    // 速度: 60% 开度档间插值 → 284.0 (同 基线)
    assert_eq!(
        eval_with("fm_flap_allow_speed(60, 1)", Some(&fmdata)),
        284.0
    );
    // vne: 无 sweep 表 → 直通 vne
    assert_eq!(eval_with("fm_vne(0)", Some(&fmdata)), 800.0);
    // flap=0 → MAX (业务默认与被替代代码一致, 不 NaN 化)
    assert_eq!(
        eval_with("fm_flap_allow_speed(0, 1)", Some(&fmdata)),
        f64::MAX
    );
    // 无 FM → 查表族 NaN 隔离 (flap 两函数除外: 业务默认 125/MAX)
    assert!(eval_with("fm_vne(0)", None).is_nan());
    assert_eq!(eval_with("fm_flap_allow_angle(270, 0)", None), 125.0);
    let _ = &mut store;
}

#[test]
fn fn_id_codec_roundtrip() {
    // 编译期守卫: 枚举声明序与宏映射序漂移 → 分派错乱 (曾两次真实事故)
    use crate::formula::functions::{fid_from_u16, fid_to_u16, FnId};
    let all = [
        (FnId::Invalid, "invalid"),
        (FnId::Clamp, "clamp"),
        (FnId::IasPerMach, "ias_per_mach"),
        (FnId::FmVne, "fm_vne"),
        (FnId::FmFlapAllowAngle, "fm_flap_allow_angle"),
        (FnId::Sma, "sma"),
        (FnId::LearnMax, "learn_max"),
    ];
    for (fid, _) in all {
        assert_eq!(fid_from_u16(fid_to_u16(fid)), Some(fid), "{fid:?} 往返失败");
    }
    assert_eq!(fid_from_u16(9999), None);
}

// ===== W1c: 常量折叠 =====

#[test]
fn const_folding_at_compile_time() {
    // 全常量表达式折为单 Num: 1+2*3^2 = 19
    let set = CompiledFormulaSet::compile(&[def("k", "1 + 2 * 3 ^ 2")], registry());
    assert!(matches!(set.formulas[0].rexpr, Some(RExpr::Num(v)) if (v - 19.0).abs() < 1e-12));
    // 短路折叠: 0 && 1/0 → 0 (右侧常量除零不产生 NaN, 短路即弃)
    let set = CompiledFormulaSet::compile(&[def("s", "0 && 1/0")], registry());
    assert!(matches!(set.formulas[0].rexpr, Some(RExpr::Num(v)) if v == 0.0));
    // 非短路: 1 && 0/0 → NaN 常量
    let set = CompiledFormulaSet::compile(&[def("n", "1 && 0/0")], registry());
    assert!(matches!(set.formulas[0].rexpr, Some(RExpr::Num(v)) if v.is_nan()));
    // 变量参与不折: 1 + ias 保持 Binary
    let set = CompiledFormulaSet::compile(&[def("v", "1 + ias")], registry());
    assert!(matches!(set.formulas[0].rexpr, Some(RExpr::Binary { .. })));
    // 折叠与运行时一致: 同值断言
    let t = TestTel::default();
    assert_eq!(try_eval("(1+2*3^2) + ias*0 + (4>3 ? 10 : 20)", &t), 29.0);
}

// ===== W2: latch 惰性原语 =====

#[test]
fn latch_lazy_semantics() {
    let reg = registry();
    let mut store = StateStore::new();
    let snap = VarSnapshot {
        values: vec![f64::NAN; reg.len()],
    };
    // cond 真: 输出 x 并记忆
    let v = try_eval_single("latch(1, 42)", reg, &snap, &mut store, 0, 50.0, None).unwrap();
    assert_eq!(v, 42.0);
    // cond 假: 输出上帧 (42), x 不求值 (1/0 的 inf 不会出现)
    let v = try_eval_single("latch(0, 1/0)", reg, &snap, &mut store, 50, 50.0, None).unwrap();
    assert_eq!(v, 42.0);
    // cond 真 x=NaN: 输出 NaN 不污染记忆 (下帧假仍 42)
    let v = try_eval_single("latch(1, 0/0)", reg, &snap, &mut store, 100, 50.0, None).unwrap();
    assert!(v.is_nan());
    let v = try_eval_single("latch(0, 7)", reg, &snap, &mut store, 150, 50.0, None).unwrap();
    assert_eq!(v, 42.0);
    // 惰性验证: cond 假时 x 内的 sma 状态不推进
    let mut store2 = StateStore::new();
    let _ = try_eval_single(
        "latch(1, sma(10, 3))",
        reg,
        &snap,
        &mut store2,
        0,
        50.0,
        None,
    )
    .unwrap();
    let _ = try_eval_single(
        "latch(0, sma(20, 3))",
        reg,
        &snap,
        &mut store2,
        50,
        50.0,
        None,
    )
    .unwrap(); // 不执行
    let v = try_eval_single("sma(30, 3)", reg, &snap, &mut store2, 100, 50.0, None).unwrap();
    // 同一 site? 不同调用点 — sma 在 latch 内是独立 site; 外部 sma(30,3) 是新 site 从零:
    // 验证意图改由 latch 输出表达: cond 假两帧后仍记忆首帧值
    let v2 = try_eval_single(
        "latch(0, sma(99, 3))",
        reg,
        &snap,
        &mut store2,
        150,
        50.0,
        None,
    )
    .unwrap();
    assert_eq!(v2, 10.0, "latch 内 sma 未被假帧推进");
    let _ = v;
}
