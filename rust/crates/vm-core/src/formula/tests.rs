//! 公式系统单测: 词法/语法/求值语义/状态原语对齐/编译器(DAG+环)/注册表。
//! 对拍原则 (不假通过): 状态原语逐值对齐现有 Rust 实现, 不放宽断言。

use super::definition::{try_eval_single, CompileError, CompiledFormulaSet, FormulaDef};
use super::eval::StateStore;
use super::registry::{assemble_snapshot, registry, MetaInputs, VarOrigin};
use super::FormulaManager;
use crate::calc_helper::SimpleMovingAverage;
use crate::ui_model::fm_data_source::FMDataSource;
use crate::ui_model::telemetry_source::TelemetrySource;

// ===== 测试桩: TelemetrySource (关键字段可配, 其余 0) =====

struct TestTel {
    ias: f64,
    tas: f64,
    alt: f64,
    mach: f64,
    ny: f64,
    energy_jkg: f64,
    mass_fuel: f64,
    total_weight: f64,
}

impl Default for TestTel {
    fn default() -> Self {
        TestTel {
            ias: 400.0,
            tas: 450.0,
            alt: 5000.0,
            mach: 0.66,
            ny: 2.0,
            energy_jkg: 9000.0,
            mass_fuel: 300.0,
            total_weight: 4000.0,
        }
    }
}

macro_rules! tel_field {
    ($m:ident, $f:ident) => {
        fn $m(&self) -> f64 {
            self.$f
        }
    };
}
macro_rules! tel_zero {
    ($m:ident) => {
        fn $m(&self) -> f64 {
            0.0
        }
    };
}
macro_rules! tel_false {
    ($m:ident) => {
        fn $m(&self) -> bool {
            false
        }
    };
}

impl TelemetrySource for TestTel {
    tel_field!(get_ias, ias);
    tel_field!(get_tas, tas);
    tel_field!(get_mach, mach);
    tel_zero!(get_aoa);
    tel_zero!(get_aos);
    tel_field!(get_ny, ny);
    tel_zero!(get_vario);
    tel_field!(get_altitude, alt);
    tel_zero!(get_radio_altitude);
    tel_false!(is_radio_altitude_valid);
    tel_zero!(get_compass);
    tel_zero!(get_sep);
    tel_zero!(get_acceleration);
    tel_zero!(get_turn_rate);
    tel_zero!(get_turn_radius);
    tel_false!(is_turn_radius_valid);
    tel_zero!(get_roll_rate);
    tel_field!(get_energy_jkg, energy_jkg);
    tel_field!(get_mass_fuel, mass_fuel);
    tel_field!(get_total_weight, total_weight);
    fn get_fuel_time_mili(&self) -> i64 {
        0
    }
    tel_zero!(get_throttle);
    tel_zero!(get_rpm);
    tel_zero!(get_manifold_pressure);
    tel_zero!(get_water_temp);
    tel_zero!(get_oil_temp);
    tel_zero!(get_pitch);
    tel_zero!(get_eff_hp);
    tel_zero!(get_thrust);
    tel_zero!(get_horse_power);
    tel_zero!(get_engine_response);
    tel_zero!(get_prop_efficiency);
    tel_zero!(get_wep_kg);
    tel_zero!(get_wep_time);
    tel_zero!(get_heat_tolerance);
    tel_zero!(get_power_percent);
    tel_zero!(get_manifold_pressure_pounds);
    tel_zero!(get_manifold_pressure_inch_hg);
    tel_zero!(get_manifold_pressure_display);
    fn get_manifold_pressure_display_unit(&self) -> String {
        "Ata".into()
    }
    fn get_manifold_pressure_display_precision(&self) -> i32 {
        2
    }
    tel_zero!(get_unknown_mixture);
    tel_zero!(get_radiator);
    tel_zero!(get_compressor_stage);
    tel_zero!(get_fuel_percent);
    tel_zero!(get_rpm_throttle);
    tel_zero!(get_gear);
    tel_zero!(get_flaps);
    tel_zero!(get_airbrake);
    tel_zero!(get_aileron);
    tel_zero!(get_elevator);
    tel_zero!(get_rudder);
    tel_zero!(get_wing_sweep);
    tel_false!(is_wing_sweep_valid);
    tel_zero!(get_speed_limit_ratio);
    tel_zero!(get_aileron_lock_ratio);
    tel_zero!(get_rudder_lock_ratio);
    tel_zero!(get_unit_mach_limit_ratio);
    tel_zero!(get_stall_speed);
    tel_false!(is_imperial);
    tel_zero!(get_aviahorizon_pitch);
    tel_zero!(get_aviahorizon_roll);
    tel_false!(is_jet_engine);
    tel_false!(is_prop_engine);
    tel_false!(is_piston_engine);
    tel_false!(is_turboprop_engine);
    tel_false!(is_engine_check_done);
    tel_false!(has_wep);
    tel_zero!(get_booster_fuel_kg);
    tel_zero!(get_booster_fuel_percent);
    tel_false!(has_booster);
}

// ===== 测试桩: FMDataSource (全 0) =====

macro_rules! fm_zero {
    ($m:ident) => {
        fn $m(&self) -> f64 {
            0.0
        }
    };
}

struct TestFm;

impl FMDataSource for TestFm {
    fn get_fm_version(&self) -> String {
        "".into()
    }
    fm_zero!(get_empty_weight);
    fm_zero!(get_max_fuel_weight);
    fm_zero!(get_critical_speed);
    fm_zero!(get_vne);
    fm_zero!(get_vne_mach);
    fm_zero!(get_full_fuel_pos_g);
    fm_zero!(get_full_fuel_neg_g);
    fm_zero!(get_half_fuel_pos_g);
    fm_zero!(get_half_fuel_neg_g);
    fm_zero!(get_elevator_eff_speed);
    fm_zero!(get_aileron_eff_speed);
    fm_zero!(get_rudder_eff_speed);
    fm_zero!(get_elevator_power_loss);
    fm_zero!(get_aileron_power_loss);
    fm_zero!(get_rudder_power_loss);
    fm_zero!(get_nitro_amount);
    fm_zero!(get_nitro_time);
    fm_zero!(get_avg_eng_recovery_rate);
    fm_zero!(get_no_flap_wing_load);
    fm_zero!(get_full_flap_wing_load);
    fm_zero!(get_moi_pitch);
    fm_zero!(get_moi_roll);
    fm_zero!(get_moi_yaw);
    fm_zero!(get_wing_area);
    fm_zero!(get_fuselage_area);
    fm_zero!(get_oswalds_efficiency);
    fm_zero!(get_aspect_ratio);
    fm_zero!(get_swept_wing_angle);
    fm_zero!(get_cd_s);
    fm_zero!(get_ind_cd_f);
    fm_zero!(get_radiator_cd);
    fm_zero!(get_oil_radiator_cd);
    fm_zero!(get_no_flaps_wing_cd_min);
    fm_zero!(get_no_flaps_wing_cl0);
    fm_zero!(get_no_flaps_wing_aoa_crit_high);
    fm_zero!(get_no_flaps_wing_aoa_crit_low);
    fm_zero!(get_no_flaps_wing_cl_crit_high);
    fm_zero!(get_no_flaps_wing_cl_crit_low);
    fm_zero!(get_full_flaps_wing_cd_min);
    fm_zero!(get_full_flaps_wing_cl0);
    fm_zero!(get_full_flaps_wing_aoa_crit_high);
    fm_zero!(get_full_flaps_wing_aoa_crit_low);
    fm_zero!(get_fuselage_cd_min);
    fm_zero!(get_fin_cd_min);
    fm_zero!(get_stab_cd_min);
    fm_zero!(get_flap0_speed);
    fm_zero!(get_flap1_speed);
    fm_zero!(get_flap2_speed);
    fm_zero!(get_flap3_speed);
    fm_zero!(get_gear_destruction_speed);
    fn get_engine_num(&self) -> i32 {
        0
    }
    fn is_nitro_amount_valid(&self) -> bool {
        false
    }
    fn is_flap0_speed_valid(&self) -> bool {
        false
    }
    fn is_flap1_speed_valid(&self) -> bool {
        false
    }
    fn is_flap2_speed_valid(&self) -> bool {
        false
    }
    fn is_flap3_speed_valid(&self) -> bool {
        false
    }
    fn is_jet(&self) -> bool {
        false
    }
}

// ===== 工具: 快照与求值 ====

fn snap_of(tel: &TestTel) -> super::registry::VarSnapshot {
    let meta = MetaInputs { interval_ms: 50.0, freq: 20.0, ..Default::default() };
    assemble_snapshot(tel, None, &meta)
}

fn try_eval(expr: &str, tel: &TestTel) -> f64 {
    let snap = snap_of(tel);
    let mut store = StateStore::new();
    try_eval_single(expr, registry(), &snap, &mut store, 1000, 50.0).unwrap()
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
    // getter 别名同值
    assert_eq!(try_eval("getIAS * 2", &t), 800.0);
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
    assert_eq!(try_eval("round(3.14159, 2)", &t), 3.14);
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
    let t = TestTel { ias: 400.0, alt: 5000.0, ..Default::default() };
    let via_fn = try_eval("ias / ias_per_mach(altitude)", &t);
    // manual 与 derive.rs L116-120 逐项同构: mach = ias / (3.6*sqrt(...))
    let manual: f64 = 400.0
        / (3.6
            * (1.4f64 / 1.225 * 101325.0 * (1.0f64 - 0.0000225577 * 5000.0).powf(5.25588))
                .sqrt());
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
        let mut s2 = super::registry::VarSnapshot { values: vec![f64::NAN; reg.len()] };
        s2.values[ias as usize] = x;
        let v = try_eval_single("sma(ias, 3)", reg, &s2, &mut store, 0, 50.0).unwrap();
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
    let a = try_eval_single("prev(ias)", registry(), &snap, &mut store, 0, 50.0).unwrap();
    assert_eq!(a, 0.0);
    let b = try_eval_single("prev(ias)", registry(), &snap, &mut store, 50, 50.0).unwrap();
    assert_eq!(b, 400.0);
    // blend(x, 0.1): 首帧 (1-0.1)*0 + 0.1*400 = 40
    let mut store2 = StateStore::new();
    let c = try_eval_single("blend(ias, 0.1)", registry(), &snap, &mut store2, 0, 50.0).unwrap();
    assert!((c - 40.0).abs() < 1e-12, "{c}");
    let d = try_eval_single("blend(ias, 0.1)", registry(), &snap, &mut store2, 50, 50.0).unwrap();
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
        last = try_eval_single("vote(1, 0, 100)", registry(), &snap, &mut store, i * 50, 50.0)
            .unwrap();
        if i < 99 {
            assert_eq!(last, 0.0, "冻结前应恒 0");
        }
    }
    assert_eq!(last, 1.0, "第 100 帧计数达 100 → 冻结");
    // 冻结后 up 消失仍输出冻结值
    let after =
        try_eval_single("vote(0, 1, 100)", registry(), &snap, &mut store, 6000, 50.0).unwrap();
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
        v = try_eval_single("stable(7, 300)", registry(), &snap, &mut store, i * 50, 50.0)
            .unwrap();
    }
    assert_eq!(v, 0.0, "250ms 未达 300ms 阈值");
    v = try_eval_single("stable(7, 300)", registry(), &snap, &mut store, 300, 50.0).unwrap();
    assert_eq!(v, 1.0, "300ms 达阈值");
    // 值变化 → 立即清零
    v = try_eval_single("stable(8, 300)", registry(), &snap, &mut store, 350, 50.0).unwrap();
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
        v = try_eval_single("learn_max(2400, 1, 1000)", registry(), &snap, &mut store, i * 50, 50.0)
            .unwrap();
    }
    // 20 帧×50ms=1000ms → 已锁定; ratio=0.05 软逼近 2400*(1-0.95^20) ≈ 1540
    assert!(v > 1500.0 && v < 2400.0, "learn_max 逼近值 {v}");
    let locked = v;
    // 锁定后输入变大也不再更新
    let v2 =
        try_eval_single("learn_max(99999, 1, 1000)", registry(), &snap, &mut store, 2000, 50.0)
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
    let defs = vec![def("b", "a * 2"), def("a", "ias + 1")];
    let set = CompiledFormulaSet::compile(&defs, registry(), &["b".to_string()]);
    assert!(set.formulas.iter().all(|f| f.err.is_none()));
    let tel = TestTel::default();
    let meta = MetaInputs { interval_ms: 50.0, ..Default::default() };
    let snap = assemble_snapshot(&tel, None, &meta);
    let mut store = StateStore::new();
    let r = set.eval_frame(&snap, &mut store, 0, 50.0);
    let sa = set.slots["a"];
    let sb = set.slots["b"];
    assert_eq!(r.get(sa), 401.0);
    assert_eq!(r.get(sb), 802.0);
}

#[test]
fn compile_detects_cycle() {
    let defs = vec![def("a", "b + 1"), def("b", "a + 1"), def("c", "ias")];
    let set = CompiledFormulaSet::compile(&defs, registry(), &["c".to_string()]);
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
    let set = CompiledFormulaSet::compile(&defs, registry(), &[]);
    if let Some(CompileError::Cycle(chain)) = &set.formulas[set.slots["x"] as usize].err {
        // 环链包含全部三个名字
        assert!(chain.contains(&"x".to_string()) && chain.contains(&"y".to_string()) && chain.contains(&"z".to_string()));
    } else {
        panic!("x 应在环上");
    }
}

#[test]
fn compile_invalid_dep_propagates_nan() {
    // c 依赖未知变量的 a → a invalid, c 求 NaN (不阻断)
    let defs = vec![def("a", "nope + 1"), def("c", "a * 2")];
    let set = CompiledFormulaSet::compile(&defs, registry(), &["c".to_string()]);
    assert!(matches!(
        set.formulas[set.slots["a"] as usize].err,
        Some(CompileError::UnknownName(_))
    ));
    let tel = TestTel::default();
    let meta = MetaInputs::default();
    let snap = assemble_snapshot(&tel, None, &meta);
    let mut store = StateStore::new();
    let r = set.eval_frame(&snap, &mut store, 0, 50.0);
    assert!(r.get(set.slots["c"]).is_nan());
}

#[test]
fn compile_duplicate_and_disabled() {
    let defs = vec![def("a", "1"), def("a", "2")];
    let set = CompiledFormulaSet::compile(&defs, registry(), &[]);
    assert!(matches!(
        set.formulas[1].err,
        Some(CompileError::DuplicateName(_))
    ));
    let mut d = def("b", "1");
    d.disabled = true;
    let set2 = CompiledFormulaSet::compile(&[d], registry(), &[]);
    assert!(matches!(set2.formulas[0].err, Some(CompileError::DisabledByUser)));
}

#[test]
fn compile_bad_arity_and_unknown_fn() {
    let set = CompiledFormulaSet::compile(&[def("a", "clamp(1, 2)")], registry(), &[]);
    assert!(matches!(
        set.formulas[0].err,
        Some(CompileError::BadArity { .. })
    ));
    let set2 = CompiledFormulaSet::compile(&[def("a", "nosuchfn(1)")], registry(), &[]);
    assert!(matches!(
        set2.formulas[0].err,
        Some(CompileError::UnknownFn(_))
    ));
}

#[test]
fn compile_live_marking() {
    // external_refs 只引 c: c 与其依赖 a live, 孤儿 b 死
    let defs = vec![def("a", "ias"), def("b", "tas"), def("c", "a + 1")];
    let set = CompiledFormulaSet::compile(&defs, registry(), &["c".to_string()]);
    assert!(set.formulas[set.slots["a"] as usize].live);
    assert!(set.formulas[set.slots["c"] as usize].live);
    assert!(!set.formulas[set.slots["b"] as usize].live);
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
fn registry_getter_aliases_hit() {
    // 存量 ui_layout.cfg :target 的 getter 名必须命中 (向后兼容)
    let reg = registry();
    for g in ["getIAS", "getTAS", "getMach", "getNy", "getFuelTimeMili", "getWingSweep",
        "getManifoldPressureDisplay", "getSpeedLimitRatio", "getStallSpeed"] {
        assert!(reg.lookup(g).is_some(), "getter 别名未命中: {g}");
    }
}

#[test]
fn registry_snapshot_assemble() {
    let tel = TestTel::default();
    let meta = MetaInputs { interval_ms: 50.0, freq: 20.0, fm_loaded: false, ..Default::default() };
    let snap = assemble_snapshot(&tel, Some(&TestFm), &meta);
    let reg = registry();
    let ias = reg.lookup("ias").unwrap();
    assert_eq!(snap.values[ias as usize], 400.0);
    let has_wep = reg.lookup("has_wep").unwrap();
    assert_eq!(snap.values[has_wep as usize], 0.0);
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
    assert!(counts[1] >= 5, "indicators 类应至少 5 个, 实际 {}", counts[1]);
    // 总量与注册表规模一致 (catalog 未丢行)
    assert_eq!(counts.iter().sum::<usize>(), reg.len());
}

// ===== FormulaManager 端到端 =====

#[test]
fn manager_install_and_eval() {
    let mgr = FormulaManager::new();
    // 内置公式形状预演: energy_m 与 maneuver_index (hud_calculator A 级外置目标)
    mgr.install(
        &[def("energy_m", "energy_jkg / g"), def("maneuver_index", "1.0 - (fm.empty_weight / (fm.empty_weight + mass_fuel))")],
        &["energy_m".to_string(), "maneuver_index".to_string()],
    );
    let tel = TestTel { energy_jkg: 9000.0, mass_fuel: 300.0, ..Default::default() };
    let meta = MetaInputs { interval_ms: 50.0, ..Default::default() };
    let r = mgr.eval_frame(&tel, None, &meta, 0);
    let set = mgr.current();
    assert!((r.get(set.slots["energy_m"]) - 9000.0 / 9.80).abs() < 1e-9);
    // fm.empty_weight 无 FM → NaN 传播
    assert!(r.get(set.slots["maneuver_index"]).is_nan());
    // reset 后状态清零 (prev 从 0 再来)
    mgr.reset_states();
}

#[test]
fn manager_hot_update_retains_states() {
    let mgr = FormulaManager::new();
    mgr.install(&[def("p", "prev(ias)")], &["p".to_string()]);
    let tel = TestTel::default();
    let meta = MetaInputs { interval_ms: 50.0, ..Default::default() };
    let _ = mgr.eval_frame(&tel, None, &meta, 0);
    // 热更新: 加一个公式, 原 p 的状态保留
    mgr.install(&[def("p", "prev(ias)"), def("q", "ias * 2")], &["p".to_string(), "q".to_string()]);
    let r = mgr.eval_frame(&tel, None, &meta, 50);
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
    assert_eq!(mgr.try_eval("ias * 3.6", &snap, 0, 50.0).unwrap(), 1440.0);
    assert!(mgr.try_eval("unknown_var", &snap, 0, 50.0).is_err());
}

// ===== :target 统一解析 (设计 §8) =====

#[test]
fn resolve_target_variants() {
    // getter 别名
    let (v, m) = super::resolve_target("getIAS").unwrap();
    assert_eq!(v, super::TargetVar::Var(registry().lookup("ias").unwrap()));
    assert_eq!(m, 1.0);
    // 短名 + 乘数语法 ("getWingSweep * 100" 形态)
    let (v, m) = super::resolve_target("tas * 2").unwrap();
    assert_eq!(m, 2.0);
    let tel = TestTel::default();
    assert_eq!(super::target_value(&v, m, &tel), Some(900.0));
    // 未知名 → 公式名形态 (延迟判定)
    let (v, _) = super::resolve_target("my_formula").unwrap();
    assert_eq!(v, super::TargetVar::Formula("my_formula".into()));
    // 公式名取值: 未接公式系统的源 → None (上层 0 降级)
    assert_eq!(super::target_value(&v, 1.0, &tel), None);
    // 乘数非法 → None
    assert!(super::resolve_target("ias * abc").is_none());
}

// ===== 接管型公式 (与系统变量同名) 的自引用检查 (设计 §5 同名规则) =====

#[test]
fn compile_self_override_formula_rejected() {
    // mach 与系统变量同名 (接管型), 表达式引用 mach 自身 → SelfOverride
    let set = CompiledFormulaSet::compile(
        &[def("mach", "mach + 1")],
        registry(),
        &["mach".to_string()],
    );
    assert!(matches!(
        set.formulas[0].err,
        Some(CompileError::SelfOverride(_))
    ));
    // 合法接管: 表达式不引用自身 (与内置公式同形态)
    let ok = CompiledFormulaSet::compile(
        &[def("mach", "ias / ias_per_mach(altitude)")],
        registry(),
        &["mach".to_string()],
    );
    assert!(ok.formulas[0].err.is_none());
}
