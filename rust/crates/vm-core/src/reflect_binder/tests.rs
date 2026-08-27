use super::*;

/// 测试桩: 数值方法各返回声明序位常量 (1..=58, 锁死名→方法映射),
/// 可配置字段对齐 cfg 实际使用的动态通道。
struct MockTele {
    wing_sweep: f64,
    fuel_time_mili: i64,
    manifold_unit: &'static str,
    manifold_precision: i32,
}

impl Default for MockTele {
    fn default() -> Self {
        MockTele {
            wing_sweep: 49.0,
            fuel_time_mili: 19,
            manifold_unit: "Ata",
            manifold_precision: 2,
        }
    }
}

impl TelemetrySource for MockTele {
    fn get_ias(&self) -> f64 { 1.0 }
    fn get_tas(&self) -> f64 { 2.0 }
    fn get_mach(&self) -> f64 { 3.0 }
    fn get_aoa(&self) -> f64 { 4.0 }
    fn get_aos(&self) -> f64 { 5.0 }
    fn get_ny(&self) -> f64 { 6.0 }
    fn get_vario(&self) -> f64 { 7.0 }
    fn get_altitude(&self) -> f64 { 8.0 }
    fn get_radio_altitude(&self) -> f64 { 9.0 }
    fn is_radio_altitude_valid(&self) -> bool { false }
    fn get_compass(&self) -> f64 { 10.0 }
    fn get_sep(&self) -> f64 { 11.0 }
    fn get_acceleration(&self) -> f64 { 12.0 }
    fn get_turn_rate(&self) -> f64 { 13.0 }
    fn get_turn_radius(&self) -> f64 { 14.0 }
    fn is_turn_radius_valid(&self) -> bool { false }
    fn get_roll_rate(&self) -> f64 { 15.0 }
    fn get_energy_jkg(&self) -> f64 { 16.0 }
    fn get_mass_fuel(&self) -> f64 { 17.0 }
    fn get_total_weight(&self) -> f64 { 18.0 }
    fn get_fuel_time_mili(&self) -> i64 { self.fuel_time_mili }
    fn get_throttle(&self) -> f64 { 20.0 }
    fn get_rpm(&self) -> f64 { 21.0 }
    fn get_manifold_pressure(&self) -> f64 { 22.0 }
    fn get_water_temp(&self) -> f64 { 23.0 }
    fn get_oil_temp(&self) -> f64 { 24.0 }
    fn get_pitch(&self) -> f64 { 25.0 }
    fn get_eff_hp(&self) -> f64 { 26.0 }
    fn get_thrust(&self) -> f64 { 27.0 }
    fn get_horse_power(&self) -> f64 { 28.0 }
    fn get_engine_response(&self) -> f64 { 29.0 }
    fn get_prop_efficiency(&self) -> f64 { 30.0 }
    fn get_wep_kg(&self) -> f64 { 31.0 }
    fn get_wep_time(&self) -> f64 { 32.0 }
    fn get_heat_tolerance(&self) -> f64 { 33.0 }
    fn get_power_percent(&self) -> f64 { 34.0 }
    fn get_manifold_pressure_pounds(&self) -> f64 { 35.0 }
    fn get_manifold_pressure_inch_hg(&self) -> f64 { 36.0 }
    fn get_manifold_pressure_display(&self) -> f64 { 37.0 }
    fn get_manifold_pressure_display_unit(&self) -> String {
        self.manifold_unit.to_string()
    }
    fn get_manifold_pressure_display_precision(&self) -> i32 { self.manifold_precision }
    fn get_unknown_mixture(&self) -> f64 { 38.0 }
    fn get_radiator(&self) -> f64 { 39.0 }
    fn get_compressor_stage(&self) -> f64 { 40.0 }
    fn get_fuel_percent(&self) -> f64 { 41.0 }
    fn get_rpm_throttle(&self) -> f64 { 42.0 }
    fn get_gear(&self) -> f64 { 43.0 }
    fn get_flaps(&self) -> f64 { 44.0 }
    fn get_airbrake(&self) -> f64 { 45.0 }
    fn get_aileron(&self) -> f64 { 46.0 }
    fn get_elevator(&self) -> f64 { 47.0 }
    fn get_rudder(&self) -> f64 { 48.0 }
    fn get_wing_sweep(&self) -> f64 { self.wing_sweep }
    fn is_wing_sweep_valid(&self) -> bool { false }
    fn get_speed_limit_ratio(&self) -> f64 { 50.0 }
    fn get_aileron_lock_ratio(&self) -> f64 { 51.0 }
    fn get_rudder_lock_ratio(&self) -> f64 { 52.0 }
    fn get_unit_mach_limit_ratio(&self) -> f64 { 53.0 }
    fn get_stall_speed(&self) -> f64 { 54.0 }
    fn is_imperial(&self) -> bool { false }
    fn get_aviahorizon_pitch(&self) -> f64 { 55.0 }
    fn get_aviahorizon_roll(&self) -> f64 { 56.0 }
    fn is_jet_engine(&self) -> bool { false }
    fn is_prop_engine(&self) -> bool { false }
    fn is_piston_engine(&self) -> bool { false }
    fn is_turboprop_engine(&self) -> bool { false }
    fn is_engine_check_done(&self) -> bool { false }
    fn has_wep(&self) -> bool { false }
    fn get_booster_fuel_kg(&self) -> f64 { 57.0 }
    fn get_booster_fuel_percent(&self) -> f64 { 58.0 }
    fn has_booster(&self) -> bool { false }
}

/// Java TelemetrySource 数值方法面全集 (接口声明序); 期望值 = 桩的序位常量
const DOUBLE_GETTERS: &[(&str, f64)] = &[
    ("getIAS", 1.0),
    ("getTAS", 2.0),
    ("getMach", 3.0),
    ("getAoA", 4.0),
    ("getAoS", 5.0),
    ("getNy", 6.0),
    ("getVario", 7.0),
    ("getAltitude", 8.0),
    ("getRadioAltitude", 9.0),
    ("getCompass", 10.0),
    ("getSEP", 11.0),
    ("getAcceleration", 12.0),
    ("getTurnRate", 13.0),
    ("getTurnRadius", 14.0),
    ("getRollRate", 15.0),
    ("getEnergyJKg", 16.0),
    ("getMassFuel", 17.0),
    ("getTotalWeight", 18.0),
    ("getFuelTimeMili", 19.0),
    ("getThrottle", 20.0),
    ("getRPM", 21.0),
    ("getManifoldPressure", 22.0),
    ("getWaterTemp", 23.0),
    ("getOilTemp", 24.0),
    ("getPitch", 25.0),
    ("getEffHp", 26.0),
    ("getThrust", 27.0),
    ("getHorsePower", 28.0),
    ("getEngineResponse", 29.0),
    ("getPropEfficiency", 30.0),
    ("getWepKg", 31.0),
    ("getWepTime", 32.0),
    ("getHeatTolerance", 33.0),
    ("getPowerPercent", 34.0),
    ("getManifoldPressurePounds", 35.0),
    ("getManifoldPressureInchHg", 36.0),
    ("getManifoldPressureDisplay", 37.0),
    ("getUnknownMixture", 38.0),
    ("getRadiator", 39.0),
    ("getCompressorStage", 40.0),
    ("getFuelPercent", 41.0),
    ("getRPMThrottle", 42.0),
    ("getGear", 43.0),
    ("getFlaps", 44.0),
    ("getAirbrake", 45.0),
    ("getAileron", 46.0),
    ("getElevator", 47.0),
    ("getRudder", 48.0),
    ("getWingSweep", 49.0),
    ("getSpeedLimitRatio", 50.0),
    ("getAileronLockRatio", 51.0),
    ("getRudderLockRatio", 52.0),
    ("getUnitMachLimitRatio", 53.0),
    ("getStallSpeed", 54.0),
    ("getAviahorizonPitch", 55.0),
    ("getAviahorizonRoll", 56.0),
    ("getBoosterFuelKg", 57.0),
    ("getBoosterFuelPercent", 58.0),
];

// ---- 注册表完整性: 名→方法分派逐一锁死 (错配/漏配/重名即失败) ----

#[test]
fn double_registry_dispatches_full_interface_surface() {
    let m = MockTele::default();
    let mut seen: Vec<f64> = Vec::new();
    for (name, expected) in DOUBLE_GETTERS {
        let binding = resolve_double(name);
        assert!(binding.accessor.is_some(), "{name} 未注册");
        assert_eq!(binding.multiplier, 1.0, "{name} 无乘数语法时乘数应为 1.0");
        let value = binding.get(&m);
        assert_eq!(value, *expected, "{name} 分派错位");
        // 桩值去重校验: 重名/同变体映射会撞值
        assert!(!seen.contains(&value), "{name} 桩值重复, 映射锁死失效");
        seen.push(value);
    }
    assert_eq!(DOUBLE_GETTERS.len(), 58, "Java 接口数值方法共 58 个");
}

// 名字精确匹配 (findVirtual 大小写敏感, 无前缀匹配): getRPM ≠ getRPMThrottle
#[test]
fn double_names_match_exactly() {
    assert_eq!(
        resolve_double("getRPM").get(&MockTele::default()),
        21.0,
        "getRPM 应命中 get_rpm 而非 get_rpm_throttle"
    );
    assert_eq!(resolve_double("getRPMThrottle").get(&MockTele::default()), 42.0);
    // 大小写敏感
    assert_eq!(resolve_double("getias").accessor, None);
    assert_eq!(resolve_double("GetIAS").accessor, None);
}

// Java 未命中路径: NoSuchMethodException → warn + () -> 0.0 (恒 0.0 供应商)
#[test]
fn unknown_double_method_returns_zero_supplier() {
    let m = MockTele::default();
    assert_eq!(resolve_double("getFooBar").get(&m), 0.0);
    // Java 反射理论上可达的运行时类非接口方法 (Service/Thread) 在封闭注册表外
    assert_eq!(resolve_double("getId").get(&m), 0.0);
    assert_eq!(resolve_double("getPriority").get(&m), 0.0);
    // 空目标 → 静默 0.0 (Java null/isEmpty 分支, 无 warn)
    assert_eq!(resolve_double("").get(&m), 0.0);
}

// ---- "getter * 乘数" 语法 (Java resolveDouble 唯一的算术支持) ----

// ui_layout.cfg 两条真实乘数表达式 (L128 / L161)
#[test]
fn multiplier_syntax_from_ui_layout_cfg() {
    let m = MockTele::default();
    // "getWingSweep * 100" (可变翼 0-1 → 百分比)
    let b = resolve_double("getWingSweep * 100");
    assert_eq!(b.accessor, Some(DoubleAccessor::WingSweep));
    assert_eq!(b.multiplier, 100.0);
    assert_eq!(b.get(&m), 49.0 * 100.0);
    // "getFuelTimeMili * 0.001" (long 毫秒 → 秒, 走 Number.doubleValue 拓宽)
    let b = resolve_double("getFuelTimeMili * 0.001");
    assert_eq!(b.accessor, Some(DoubleAccessor::FuelTimeMili));
    assert_eq!(b.get(&m), 19.0 * 0.001);
    // 大数值 long → double 拓宽 (Java/Rust 同为就近舍入)
    let big = MockTele {
        fuel_time_mili: 2_700_000,
        ..Default::default()
    };
    assert_eq!(
        resolve_double("getFuelTimeMili * 0.001").get(&big),
        2_700_000.0 * 0.001
    );
}

// 乘数边界 = Java split("\\*") limit=0 语义逐条对齐
#[test]
fn multiplier_edge_cases_match_java_split_semantics() {
    let m = MockTele::default();
    // 无空格: split 不依赖空白 → ["getIAS","2"] → 乘数 2.0
    assert_eq!(resolve_double("getIAS*2").get(&m), 2.0);
    // 首尾空白: methodName = target.trim()
    assert_eq!(resolve_double("  getIAS  ").get(&m), 1.0);
    // 负数/小数乘数
    assert_eq!(resolve_double("getIAS * -2.5").get(&m), -2.5);
    // 非法乘数: warn 后乘数保持 1.0, 但 methodName 已取 parts[0] (仍绑定成功)
    let b = resolve_double("getIAS * abc");
    assert_eq!(b.multiplier, 1.0);
    assert_eq!(b.get(&m), 1.0);
    // 尾随星+尾随空串: "getIAS * 2 *" → split 丢尾空 → ["getIAS "," 2 "] len 2 → 正常
    assert_eq!(resolve_double("getIAS * 2 *").get(&m), 2.0);
    // 三个 '*': parts.len()==3 ≠ 2 → methodName 保持整串 → 查无此法 → 0.0
    assert_eq!(resolve_double("getIAS * 2 * 3").get(&m), 0.0);
    // 尾随 '*': 尾部空串被丢弃 → len 1 → methodName 保持整串 "getIAS *" → 0.0
    assert_eq!(resolve_double("getIAS *").get(&m), 0.0);
    // 前导 '*': ["","100"] len 2 → methodName "" → 查无 → 0.0
    assert_eq!(resolve_double("*100").get(&m), 0.0);
}

// 乘数解析文法分歧域 (characterization, §2.15; 实现处 // PORT: 注释同述)。
// Java parseDouble 收 "5f"/"0x1p1" 拒 "inf"; Rust parse 恰反。真实 cfg 乘数
// 均为纯小数字面量, 分歧域外 — 本测试钉死 Rust 侧现状, 位级对齐待统一
// java_parse_double 上提 (visibility_expression 同款上报)。
#[test]
fn multiplier_parse_known_divergence_from_java() {
    let m = MockTele::default();
    // Rust 拒 "5f" → 乘数 1.0 (Java: 5.0)
    assert_eq!(resolve_double("getIAS * 5f").multiplier, 1.0);
    // Rust 收 "inf" → 无穷乘数 (Java: 拒 → 1.0)
    assert_eq!(resolve_double("getIAS * inf").multiplier, f64::INFINITY);
    assert!(resolve_double("getIAS * inf").get(&m).is_infinite());
}

// ---- resolve_string / resolve_int (cfg 全表仅进气压动态通道一条) ----

#[test]
fn resolve_string_bindings() {
    let metric = MockTele::default();
    let imperial = MockTele {
        manifold_unit: "P/30.1''",
        ..Default::default()
    };
    let b = resolve_string("getManifoldPressureDisplayUnit");
    assert_eq!(b.accessor, Some(StringAccessor::ManifoldPressureDisplayUnit));
    assert_eq!(b.get(&metric), "Ata");
    assert_eq!(b.get(&imperial), "P/30.1''");
    // trim; 未知 → ""; 空 → ""
    assert_eq!(resolve_string("  getManifoldPressureDisplayUnit ").get(&metric), "Ata");
    assert_eq!(resolve_string("getFooBar").get(&metric), "");
    assert_eq!(resolve_string("").get(&metric), "");
}

#[test]
fn resolve_int_bindings() {
    let metric = MockTele::default();
    let imperial = MockTele {
        manifold_precision: 1,
        ..Default::default()
    };
    let b = resolve_int("getManifoldPressureDisplayPrecision");
    assert_eq!(
        b.accessor,
        Some(IntAccessor::ManifoldPressureDisplayPrecision)
    );
    assert_eq!(b.get(&metric), 2);
    assert_eq!(b.get(&imperial), 1);
    // trim; 未知 → 0; 空 → 0
    assert_eq!(
        resolve_int("  getManifoldPressureDisplayPrecision ").get(&metric),
        2
    );
    assert_eq!(resolve_int("getFooBar").get(&metric), 0);
    assert_eq!(resolve_int("").get(&metric), 0);
}

// string/int 通道不吃乘数语法 (Java 原实现即整串 trim 直查 → 未命中)
#[test]
fn string_int_channels_have_no_multiplier_syntax() {
    let m = MockTele::default();
    assert_eq!(
        resolve_string("getManifoldPressureDisplayUnit * 2").get(&m),
        ""
    );
    assert_eq!(
        resolve_int("getManifoldPressureDisplayPrecision * 2").get(&m),
        0
    );
}

// ---- java_split_star: JDK String.split 尾部空串丢弃语义 ----

#[test]
fn java_split_star_drops_only_trailing_empties() {
    assert_eq!(java_split_star("a*b"), vec!["a", "b"]);
    assert_eq!(java_split_star("a**b"), vec!["a", "", "b"], "中间空串保留");
    assert_eq!(java_split_star("*a"), vec!["", "a"], "前导空串保留");
    assert_eq!(java_split_star("a*"), vec!["a"]);
    assert_eq!(java_split_star("a**"), vec!["a"]);
    assert_eq!(java_split_star("*"), Vec::<&str>::new());
    assert_eq!(java_split_star(""), Vec::<&str>::new());
}
