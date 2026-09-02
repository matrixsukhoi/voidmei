//! types.rs 单测 — blkx→json 迁移后保留面: 5 个数据类的 Java 语义保真
//! (文本版燃油修正/cut_static 族及其 oracle 用例已随文本链退役删除,
//!  JSON 版等价分支在 json/tests.rs)。

use super::*;
#[test]
fn java8_oracle_fuel_type_display() {
    assert_eq!(FuelType::None.to_string(), "NONE");
    assert_eq!(FuelType::SovietB95.to_string(), "SOVIET_B95");
    assert_eq!(FuelType::SovietB100.to_string(), "SOVIET_B100");
    assert_eq!(FuelType::British150Octane.to_string(), "BRITISH_150_OCTANE");
    assert_eq!(FuelType::British100Spitfire.to_string(), "BRITISH_100_SPITFIRE");
}

/// Java 字段初始化器保真: new FuelModification() 的五字段初值
#[test]
fn fuel_modification_default_matches_java_initializers() {
    let m = FuelModification::new();
    assert_eq!(m.soviet_octane_hp_bonus, 0.0);
    assert_eq!(m.british_afterburner_mult, 1.0);
    assert_eq!(m.british_afterburner_compressor_mult, 1.0);
    assert!(!m.british_invert_logic);
    assert_eq!(m.r#type, FuelType::None);
    // Default trait 与 new() 同源 (Java 只有一个构造路径)
    assert_eq!(FuelModification::default().british_afterburner_mult, 1.0);
}

/// XY::new — 定长零填充数组 + cur=0 (Java 构造器 L227-231)
#[test]
fn xy_new_zero_fills_arrays() {
    let xy = XY::new(5);
    assert_eq!(xy.x, vec![0.0; 5]);
    assert_eq!(xy.y, vec![0.0; 5]);
    assert_eq!(xy.cur, 0);
    let empty = XY::new(0);
    assert!(empty.x.is_empty() && empty.y.is_empty(), "num=0 空数组");
}

/// EngineLoad — Java 隐式零初始化 (§2.10)
#[test]
fn engine_load_default_all_zero() {
    let e = EngineLoad::default();
    assert_eq!(e.water_limit, 0.0);
    assert_eq!(e.oil_limit, 0.0);
    assert_eq!(e.work_time, 0.0);
    assert_eq!(e.recover_time, 0.0);
    assert_eq!(e.cur_water_work_time_mili, 0.0);
    assert_eq!(e.cur_oil_work_time_mili, 0.0);
}

/// FmParts — Java 隐式零初始化 + name=null
#[test]
fn fm_parts_default_zero_and_unnamed() {
    let p = FmParts::default();
    assert!(p.name.is_none(), "name 未赋值 ≈ Java null");
    assert_eq!(p.sq, 0.0);
    assert_eq!(p.cd_min, 0.0);
    assert_eq!(p.cl0, 0.0);
    assert_eq!(p.cl_crit_high, 0.0);
    assert_eq!(p.cl_crit_low, 0.0);
    assert_eq!(p.cl_after_crit, 0.0);
    assert_eq!(p.aoa_crit_high, 0.0);
    assert_eq!(p.aoa_crit_low, 0.0);
    assert_eq!(p.line_cl_coeff, 0.0);
}

/// SweepLevel — Java 隐式零初始化 + noFlaps/fullFlaps=null (构造后赋值前)
#[test]
fn sweep_level_default_zero_and_unassigned_parts() {
    let s = SweepLevel::default();
    assert_eq!(s.sweep, 0.0);
    assert_eq!(s.vne, 0.0);
    assert_eq!(s.vne_mach, 0.0);
    assert!(s.no_flaps.is_none(), "noFlaps 未赋值 ≈ Java null");
    assert!(s.full_flaps.is_none(), "fullFlaps 未赋值 ≈ Java null");
}
