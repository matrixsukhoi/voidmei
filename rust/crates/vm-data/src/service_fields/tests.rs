use super::*;


// W7: getter 体系消解后, 原 11 个 getter 行为测试收敛为 var_value 语义锚定
// (直通闭包的哨兵/拓宽语义在 formula/registry.rs 内联对齐; 这里锚定桥本身)

#[test]
fn var_value_null_state_defaults() {
    let d = ServiceData::default();
    use vm_core::formula::registry::FormulaView as _;
    assert_eq!(d.var_value("ias"), None, "无 state → None (NaN 隔离)");
}

#[test]
fn var_value_state_passthrough() {
    let mut d = ServiceData::default();
    d.s_state = Some(vm_core::parser::State::default());
    d.s_state.as_mut().unwrap().ias = 474;
    d.s_indic = Some(vm_core::parser::Indicators::default());
    use vm_core::formula::registry::FormulaView as _;
    assert_eq!(d.var_value("ias"), Some(474.0));
    assert_eq!(d.var_value("getIAS"), None, "单名制: getter 名不进内核 (W10)");
}

#[test]
fn var_value_wing_sweep_sentinel_zero() {
    let mut d = ServiceData::default();
    d.s_indic = Some(vm_core::parser::Indicators::default());
    use vm_core::string_helper::F_INVALID;
    d.s_indic.as_mut().unwrap().wsweep_indicator = F_INVALID;
    use vm_core::formula::registry::FormulaView as _;
    assert_eq!(d.var_value("wing_sweep"), Some(0.0), "哨兵归零");
    assert_eq!(d.var_value("wing_sweep_valid"), Some(0.0));
}

/// 公式槽取值 + 名字单轨锚定 (W10): 公式名直达公式值; Java getter 名不进
/// slots/registry (对拍文件边界专用, 曾以 :getter 别名双键兼容 — 双名制
/// 是 live 显示断链的温床, 已根除)
#[test]
fn var_value_formula_slot_and_single_name() {
    use vm_core::formula::{FormulaDef, FormulaManager};
    let mgr = FormulaManager::new();
    let defs = vec![FormulaDef {
        name: "mach".into(),
        expr: "0.72".into(),
        ..Default::default()
    }];
    mgr.install(&defs, &["mach".to_string()]);
    let mut d = ServiceData::default();
    d.formula_slots = mgr.current().slots_arc();
    let raw = vm_core::formula::registry::RawInputs::default();
    d.formula_values = mgr.eval_frame(&raw, &Default::default(), &Default::default(), 0);
    use vm_core::formula::registry::FormulaView as _;
    assert_eq!(d.var_value("mach"), Some(0.72));
    assert_eq!(d.var_value("getMach"), None, "getter 名不可达 (单名制, 边界外禁用)");
}

/// registry 补齐的助推器/WEP/油量五量 (live 显示回归锚): 直绑闭包语义
/// 对位原 getter (守卫 NaN 穿透 / min NaN 传播 / 聚合搬运)
#[test]
fn var_value_booster_wep_fuel_registry_vars() {
    let mut d = ServiceData::default();
    let mut s = vm_core::parser::State::default();
    s.mfuel_1 = 300.0;
    s.mfuel0_1 = 400.0;
    d.s_state = Some(s);
    use vm_core::formula::registry::FormulaView as _;
    assert_eq!(d.var_value("booster_fuel_kg"), Some(300.0));
    assert_eq!(d.var_value("getBoosterFuelKg"), None, "单名制: getter 名不进内核");
    assert_eq!(d.var_value("booster_fuel_percent"), Some(75.0));
    assert_eq!(d.var_value("has_booster"), Some(1.0));
    assert_eq!(d.var_value("has_wep"), None, "无 FM → None → 消费面 false (对位原 false)");
    // fuel_percent 走 SessionInputs 搬运
    assert_eq!(d.var_value("fuel_percent"), Some(0.0));
    // 无助推器 (哨兵) → 归零
    let mut d2 = ServiceData::default();
    let mut s2 = vm_core::parser::State::default();
    s2.mfuel_1 = -65535.0;
    d2.s_state = Some(s2);
    assert_eq!(d2.var_value("booster_fuel_kg"), Some(0.0));
    assert_eq!(d2.var_value("has_booster"), Some(0.0));
}
