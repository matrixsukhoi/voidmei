use super::*;

// ---- Java 8 oracle 对拍: extractFuelModifications 十个输入 ----

/// oracle: A_empty / B_nomods — 空串与无 modifications 块 → 全默认
#[test]
fn java8_oracle_extract_no_fuel() {
    for input in ["", "unit {\n\tfoo {\n\t}\n}\n"] {
        let m = extract_fuel_modifications(input);
        assert_eq!(m.r#type, FuelType::None, "type=NONE ({input:?})");
        assert_eq!(m.soviet_octane_hp_bonus, 0.0);
        assert_eq!(m.british_afterburner_mult, 1.0);
        assert_eq!(m.british_afterburner_compressor_mult, 1.0);
        assert!(!m.british_invert_logic);
    }
}

/// oracle: C_b100 / D_b95 / I_b100_bad / J_both — 苏联油料族
#[test]
fn java8_oracle_extract_soviet() {
    // yak-3 真实中央文件格式 (data/aces/gamedata/flightmodels/yak-3.blkx L1920-1928)
    let b100 = "modifications {\n\tussr_fuel_b-100 {\n\t\ttier:i = 4\n\t\tmodClass:t = \"lth\"\n\t\tdontDecreaseAirRCost:b = true\n\t\teffects {\n\t\t\taddHorsePowers:r = 50\n\t\t}\n\t}\n}\n";
    let m = extract_fuel_modifications(b100);
    assert_eq!(m.r#type, FuelType::SovietB100, "C_b100 type");
    assert_eq!(m.soviet_octane_hp_bonus, 50.0, "C_b100 hp");
    assert_eq!(m.british_afterburner_mult, 1.0);
    assert!(!m.british_invert_logic);

    let b95 = "modifications {\n\tussr_fuel_b-95 {\n\t\teffects {\n\t\t\taddHorsePowers:r = 30\n\t\t}\n\t}\n}\n";
    let m = extract_fuel_modifications(b95);
    assert_eq!(m.r#type, FuelType::SovietB95, "D_b95 type");
    assert_eq!(m.soviet_octane_hp_bonus, 30.0, "D_b95 hp");

    // 畸形数值 → NumberFormatException catch → 0
    let bad = "modifications {\n\tussr_fuel_b-100 {\n\t\teffects {\n\t\t\taddHorsePowers:r = abc\n\t\t}\n\t}\n}\n";
    let m = extract_fuel_modifications(bad);
    assert_eq!(m.r#type, FuelType::SovietB100, "I_b100_bad type");
    assert_eq!(m.soviet_octane_hp_bonus, 0.0, "I_b100_bad hp=0");

    // b-95 与 b-100 并存 → b-100 优先 (检查顺序)
    let both = "modifications {\n\tussr_fuel_b-95 {\n\t\teffects {\n\t\t\taddHorsePowers:r = 30\n\t\t}\n\t}\n\tussr_fuel_b-100 {\n\t\teffects {\n\t\t\taddHorsePowers:r = 50\n\t\t}\n\t}\n}\n";
    let m = extract_fuel_modifications(both);
    assert_eq!(m.r#type, FuelType::SovietB100, "J_both type");
    assert_eq!(m.soviet_octane_hp_bonus, 50.0, "J_both hp");
}

/// oracle: E_b150 / F_b150_zero_inv / G_b100spit / H_b150_noeff — 英国油料族
#[test]
fn java8_oracle_extract_british() {
    // spitfire_f24 真实中央文件格式 (datamine: invertEnableLogic:false, 1.42/1.33)
    let b150 = "modifications {\n\tnew_compressor {\n\n\t}\n\t150_octan_fuel {\n\t\tinvertEnableLogic:b = false\n\t\teffects {\n\t\t\tafterburnerMult:r = 1.42\n\t\t\tafterburnerCompressorMult:r = 1.33\n\t\t}\n\t}\n\thispano_universal {\n\n\t}\n}\n";
    let m = extract_fuel_modifications(b150);
    assert_eq!(m.r#type, FuelType::British150Octane, "E_b150 type");
    assert_eq!(m.british_afterburner_mult, 1.42, "E_b150 abm");
    assert_eq!(m.british_afterburner_compressor_mult, 1.33, "E_b150 abcm");
    assert!(!m.british_invert_logic, "E_b150 inv=false");

    // mult 为 0 → 回退 1.0; invertEnableLogic:b = true
    let zero_inv = "modifications {\n\t150_octan_fuel {\n\t\tinvertEnableLogic:b = true\n\t\teffects {\n\t\t\tafterburnerMult:r = 0\n\t\t\tafterburnerCompressorMult:r = 0\n\t\t}\n\t}\n}\n";
    let m = extract_fuel_modifications(zero_inv);
    assert_eq!(m.r#type, FuelType::British150Octane);
    assert_eq!(m.british_afterburner_mult, 1.0, "F abm 0→1.0");
    assert_eq!(m.british_afterburner_compressor_mult, 1.0, "F abcm 0→1.0");
    assert!(m.british_invert_logic, "F inv=true");

    let b100spit = "modifications {\n\t100_octan_spitfire {\n\t\tinvertEnableLogic:b = true\n\t\teffects {\n\t\t\tafterburnerMult:r = 1.1\n\t\t\tafterburnerCompressorMult:r = 1.08\n\t\t}\n\t}\n}\n";
    let m = extract_fuel_modifications(b100spit);
    assert_eq!(m.r#type, FuelType::British100Spitfire, "G type");
    assert_eq!(m.british_afterburner_mult, 1.1);
    assert_eq!(m.british_afterburner_compressor_mult, 1.08);
    assert!(m.british_invert_logic);

    // 无 effects 块 → mult 保持默认 1.0, invertEnableLogic 仍解析 (absent = false)
    let noeff = "modifications {\n\t150_octan_fuel {\n\t}\n}\n";
    let m = extract_fuel_modifications(noeff);
    assert_eq!(m.r#type, FuelType::British150Octane, "H type");
    assert_eq!(m.british_afterburner_mult, 1.0);
    assert_eq!(m.british_afterburner_compressor_mult, 1.0);
    assert!(!m.british_invert_logic, "H inv=false");
}

/// oracle: cut|c1~c8 — cutStatic 八个边界 (含大小写不敏感定位/嵌套花括号/未闭合)
#[test]
fn java8_oracle_cut_static() {
    assert_eq!(cut_static("a{b}", "a"), "b", "c1");
    assert_eq!(cut_static("a{b}", "zzz"), "null", "c2");
    assert_eq!(cut_static("x { inner { y } tail }", "x"), " inner { y } tail ", "c3");
    assert_eq!(cut_static("modifications{nested { a } }", "modifications"), "nested { a } ", "c4");
    assert_eq!(cut_static("a { b { c", "a"), "null", "c5 未闭合");
    assert_eq!(cut_static("abc", "abc"), "null", "c6 无花括号");
    assert_eq!(cut_static("pre { x } post { y }", "pre"), " x ", "c7 首块即止");
    assert_eq!(cut_static("MODS { q }", "mods"), " q ", "c8 大小写不敏感");
}

/// oracle: dbl|d1~d7 — getDoubleFromBlock 七个边界
#[test]
fn java8_oracle_get_double_from_block() {
    assert_eq!(get_double_from_block("key:r = 2.5", "key"), 2.5, "d1 typed");
    assert_eq!(get_double_from_block("key = 3.5", "key"), 3.5, "d2 plain");
    assert_eq!(get_double_from_block("key:r = 1.0, 2400", "key"), 1.0, "d3 逗号取首段");
    assert_eq!(get_double_from_block("key no eq", "key"), 0.0, "d4 无等号");
    assert_eq!(get_double_from_block("key:r = xyz", "key"), 0.0, "d5 畸形值");
    assert_eq!(get_double_from_block("key:r = 7", "key"), 7.0, "d6 行尾无换行");
    assert_eq!(get_double_from_block("other:r = 1\nkey:r = 9", "key"), 9.0, "d7 定位后取值");
}

/// oracle: bool|b1~b7 — getBoolFromBlock 七个边界
#[test]
fn java8_oracle_get_bool_from_block() {
    assert!(get_bool_from_block("k:b = true", "k"), "b1 typed true");
    assert!(get_bool_from_block("k:b = TRUE", "k"), "b2 equalsIgnoreCase");
    assert!(!get_bool_from_block("k:b = false", "k"), "b3");
    assert!(get_bool_from_block("k = true", "k"), "b4 plain key");
    assert!(!get_bool_from_block("nokey", "k"), "b5 键缺失");
    assert!(!get_bool_from_block("k:b = 1", "k"), "b6 非布尔值");
    assert!(!get_bool_from_block("k:b = true", "j"), "b7 他键");
}

/// oracle: enum| 五个常量名 — Java 枚举默认 toString()=name()
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
