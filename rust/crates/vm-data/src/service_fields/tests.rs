// PORT: Java 保真 — 测试构造沿用 Java `new X(); x.f = v;` 逐字段赋值形态,
// 不改成 struct 字面量以保持与 Java 测试源逐行对应
#![allow(clippy::field_reassign_with_default)]

use super::*;
use vm_core::blkx::Blkx;

/// 真机抓取的 /state 快照 (state.rs 测试同源, 断言值 = Java 8 oracle 实测)
const STATE_MOCK: &str = "{\"valid\": true,\"aileron, %\": -48,\"elevator, %\": 20,\"rudder, %\": -47,\"flaps, %\": 0,\"gear, %\": 0,\"H, m\": 46,\"TAS, km/h\": 454,\"IAS, km/h\": 474,\"M\": 0.39,\"AoA, deg\": -1.6,\"AoS, deg\": -5.9,\"Ny\": 0.35,\"Vy, m/s\": -7.3,\"Wx, deg/s\": -34,\"Mfuel, kg\": 197,\"Mfuel0, kg\": 734,\"throttle 1, %\": 110,\"RPM throttle 1, %\": 100,\"mixture 1, %\": 100,\"radiator 1, %\": 42,\"magneto 1\": 3,\"power 1, hp\": 1597.8,\"RPM 1\": 3001,\"manifold pressure 1, atm\": 2.24,\"water temp 1, C\": 121,\"oil temp 1, C\": 90,\"pitch 1, deg\": 35.5,\"thrust 1, kgs\": 840,\"efficiency 1, %\": 87}";

fn mock_state() -> State {
    let mut st = State::new();
    st.init();
    st.update(STATE_MOCK);
    st
}

fn mock_indicators() -> Indicators {
    let mut i = Indicators::new();
    // 手工装填 getter 所需字段 (aviahorizon/wsweep), 走 Indicators::new 的 0 默认
    i.aviahorizon_pitch = 12.5;
    i.aviahorizon_roll = -30.0;
    i.wsweep_indicator = 0.55;
    i
}

/// Java 字段声明默认值 + 显式初始化器 (§2.10) 逐项核对
#[test]
fn default_matches_java_field_initializers() {
    let d = ServiceData::default();
    // 显式初始化器
    assert_eq!(d.pressure_unit_str.as_deref(), Some("Ata"));
    assert_eq!(d.fatal_warn, Some(false));
    assert_eq!(d.optimal_compressor_stage, -1);
    assert!(!d.compressor_stage_mismatch);
    assert_eq!(d.prev_actual_compressor_stage, -1);
    assert_eq!(d.prev_optimal_compressor_stage, -1);
    assert_eq!(d.port_ocupied, Some(false));
    assert!(!d.check_engine_flag);
    // 隐式默认: 引用 null / 数值 0 / boolean false
    assert!(d.s_state.is_none());
    assert!(d.s_indic.is_none());
    assert!(d.mapinfo.is_none());
    assert!(d.loc.is_none());
    assert!(d.dir.is_none());
    assert!(d.diff_speed_sma.is_none());
    assert!(d.fuel_time_sma.is_none());
    assert!(d.radio_alt_valid.is_none(), "Boolean 无初始化器 → null");
    assert_eq!(d.fueltime, 0);
    assert_eq!(d.i_eng_type, 0, "Java int 默认 0 (resetvaria 才置 UNKNOWN=-1)");
    assert_eq!(d.turn_rds, 0.0);
    assert_eq!(d.mach, 0.0);
    assert!(!d.player_live);
    assert!(d.fm.blkx.is_none(), "fm 快照初值 = UNRESOLVED (blkx=null)");
    // 常量 (Java L213-216/227-228/236)
    assert_eq!(ENGINE_TYPE_PROP, 0);
    assert_eq!(ENGINE_TYPE_JET, 1);
    assert_eq!(ENGINE_TYPE_TURBOPROP, 2);
    assert_eq!(ENGINE_TYPE_UNKNOWN, -1);
    assert_eq!(NASTRING, "-");
    assert_eq!(NULLSTRING, "");
    assert_eq!(PRESSURE_UNIT, "Ata");
}

/// 默认态 (sState/sIndic null, fm=UNRESOLVED) 下 getter 的降级语义
/// (Java L1877 起 `sState != null ? ... : 0` 全分支 + FM 守卫)
#[test]
fn null_state_getters_return_defaults() {
    let src: Box<dyn TelemetrySource> = Box::new(ServiceData::default());
    assert_eq!(src.get_ias(), 0.0);
    assert_eq!(src.get_tas(), 0.0);
    assert_eq!(src.get_aoa(), 0.0);
    assert_eq!(src.get_aos(), 0.0);
    assert_eq!(src.get_throttle(), 0.0);
    assert_eq!(src.get_rpm(), 0.0);
    assert_eq!(src.get_manifold_pressure(), 0.0);
    assert_eq!(src.get_manifold_pressure_pounds(), 0.0);
    assert_eq!(src.get_manifold_pressure_inch_hg(), 0.0);
    assert_eq!(src.get_unknown_mixture(), 0.0);
    assert_eq!(src.get_radiator(), 0.0);
    assert_eq!(src.get_compressor_stage(), 0.0);
    assert_eq!(src.get_rpm_throttle(), 0.0);
    assert_eq!(src.get_gear(), 0.0);
    assert_eq!(src.get_flaps(), 0.0);
    assert_eq!(src.get_airbrake(), 0.0);
    assert_eq!(src.get_aileron(), 0.0);
    assert_eq!(src.get_elevator(), 0.0);
    assert_eq!(src.get_rudder(), 0.0);
    assert_eq!(src.get_pitch(), 0.0);
    assert_eq!(src.get_thrust(), 0.0);
    assert_eq!(src.get_roll_rate(), 0.0);
    assert_eq!(src.get_aviahorizon_pitch(), 0.0);
    assert_eq!(src.get_aviahorizon_roll(), 0.0);
    assert_eq!(src.get_wing_sweep(), 0.0);
    assert!(!src.is_wing_sweep_valid());
    // FM 守卫: UNRESOLVED 句柄 blkx=null
    assert_eq!(src.get_total_weight(), 0.0);
    assert!(!src.has_wep());
    // 助推器守卫
    assert_eq!(src.get_booster_fuel_kg(), 0.0);
    assert_eq!(src.get_booster_fuel_percent(), 0.0);
    assert!(!src.has_booster());
    // 英制切换默认关 (checkAlt=0)
    assert!(!src.is_imperial());
    assert_eq!(src.get_manifold_pressure_display(), 0.0);
    assert_eq!(src.get_manifold_pressure_display_unit(), "Ata");
    assert_eq!(src.get_manifold_pressure_display_precision(), 2);
    // radioAltValid null → false (Boolean 装箱语义)
    assert!(!src.is_radio_altitude_valid());
    // turnRds=0 → |0| <= 9999 → 有效 (Java 同此)
    assert!(src.is_turn_radius_valid());
    assert_eq!(src.get_turn_radius(), 0.0);
    // 引擎检测未完成 → 类型判断全 false
    assert!(!src.is_jet_engine());
    assert!(!src.is_prop_engine());
    assert!(!src.is_engine_check_done());
    // trait 对象分发 (消费方以 dyn TelemetrySource 引用)
    assert_eq!(src.get_fuel_time_mili(), 0);
}

/// 快照直读族: int→double 拓宽 / 纯字段读 / 派生量直读 (Deriver 写入后)
#[test]
fn snapshot_reads_widen_and_passthrough() {
    let mut d = ServiceData::default();
    d.s_state = Some(mock_state());
    d.s_indic = Some(mock_indicators());
    // Deriver 写入的派生量 (来源: data/derive.rs FlightValues)
    d.an = 17.34;
    d.n_vy = -7.3;
    d.mach = 0.391;
    d.sep = 12.5;
    d.acceleration = 1.25;
    d.turn_rds = -8000.0;
    d.turn_rate = 4.25;
    d.compass_delta = 164.1;
    d.alt = 46.0;
    d.radio_alt = 120.5;
    d.total_fuel = 197.0;
    d.total_hp = 1597;
    d.total_hp_eff = 1620;
    d.fuel_percent = 26;
    d.t_eng_response = 3.75;
    d.avgeff = 101.5;
    d.nitrokg = 85.0;
    d.s_wep_time_val = 270;
    d.fueltime = 2_700_000;
    d.cur_load_min_work_time = 300_000.0;
    d.energy_j_kg = 1234.5;
    d.noil_temp = 90.0;
    d.nwater_temp = 121.0;
    d.speed_limit_ratio = 0.72;
    d.aileron_lock_ratio = 0.41;
    d.rudder_lock_ratio = 0.33;
    d.unit_mach_limit_ratio = 0.66;
    d.stall_speed = 155.5;

    // int → double 拓宽 (Java State int 字段)
    assert_eq!(d.get_ias(), 474.0);
    assert_eq!(d.get_tas(), 454.0);
    assert_eq!(d.get_throttle(), 110.0);
    assert_eq!(d.get_rpm(), 3001.0);
    assert_eq!(d.get_rpm_throttle(), 100.0);
    assert_eq!(d.get_radiator(), 42.0);
    assert_eq!(d.get_unknown_mixture(), 100.0);
    assert_eq!(d.get_compressor_stage(), 0.0);
    assert_eq!(d.get_gear(), 0.0);
    assert_eq!(d.get_flaps(), 0.0);
    assert_eq!(d.get_airbrake(), -65535.0, "哨兵 int 原样拓宽 (Java 同此)");
    assert_eq!(d.get_aileron(), -48.0);
    assert_eq!(d.get_elevator(), 20.0);
    assert_eq!(d.get_rudder(), -47.0);
    assert_eq!(d.get_thrust(), 840.0);
    assert_eq!(d.get_fuel_percent(), 26.0);
    assert_eq!(d.get_horse_power(), 1597.0);
    assert_eq!(d.get_eff_hp(), 1620.0);
    // double 直读 (Float.parseFloat 拓宽值)
    assert_eq!(d.get_aoa(), -1.6f32 as f64);
    assert_eq!(d.get_aos(), -5.9f32 as f64);
    assert_eq!(d.get_manifold_pressure(), 2.24f32 as f64);
    assert_eq!(d.get_pitch(), 35.5f32 as f64);
    assert_eq!(d.get_roll_rate(), 34.0, "Math.abs(Wx)");
    assert_eq!(d.get_aviahorizon_pitch(), 12.5);
    assert_eq!(d.get_aviahorizon_roll(), -30.0);
    // getNy = An/g (Java L1901-1903)
    assert!((d.get_ny() - 17.34 / g).abs() < 1e-12);
    // 派生量直读
    assert_eq!(d.get_vario(), -7.3);
    assert_eq!(d.get_mach(), 0.391);
    assert_eq!(d.get_sep(), 12.5);
    assert_eq!(d.get_acceleration(), 1.25);
    assert_eq!(d.get_turn_rate(), 4.25);
    assert_eq!(d.get_compass(), 164.1);
    assert_eq!(d.get_altitude(), 46.0);
    assert_eq!(d.get_radio_altitude(), 120.5);
    assert_eq!(d.get_mass_fuel(), 197.0);
    assert_eq!(d.get_engine_response(), 3.75);
    assert_eq!(d.get_prop_efficiency(), 101.5);
    assert_eq!(d.get_wep_kg(), 85.0);
    assert_eq!(d.get_energy_jkg(), 1234.5);
    assert_eq!(d.get_water_temp(), 121.0);
    assert_eq!(d.get_oil_temp(), 90.0);
    assert_eq!(d.get_speed_limit_ratio(), 0.72);
    assert_eq!(d.get_aileron_lock_ratio(), 0.41);
    assert_eq!(d.get_rudder_lock_ratio(), 0.33);
    assert_eq!(d.get_unit_mach_limit_ratio(), 0.66);
    assert_eq!(d.get_stall_speed(), 155.5);
    // long 拓宽 / i64 直读
    assert_eq!(d.get_wep_time(), 270.0, "sWepTimeVal long → double");
    assert_eq!(d.get_fuel_time_mili(), 2_700_000);
    // 派生换算
    assert_eq!(d.get_heat_tolerance(), 300.0, "curLoadMinWorkTime / 1000.0");
    // Math.abs + 9999 边界
    assert_eq!(d.get_turn_radius(), 8000.0, "Math.abs(turnRds)");
    assert!(d.is_turn_radius_valid());
    d.turn_rds = -9999.0;
    assert!(d.is_turn_radius_valid(), "<= 9999 边界含等号");
    d.turn_rds = 9999.1;
    assert!(!d.is_turn_radius_valid());
    assert!(d.is_player_live() == d.player_live);
}

/// 英制/公制切换 (checkAlt 符号) 驱动的进气压显示三件套 (Java L1926-1951/2184-2186)
#[test]
fn imperial_switch_manifold_display() {
    let mut d = ServiceData::default();
    let mut st = State::new();
    st.manifoldpressure = 2.25;
    d.s_state = Some(st);

    // 公制 (checkAlt <= 0): 值 = Ata 原值, 单位 Ata, 精度 2
    assert!(!d.is_imperial());
    assert_eq!(d.get_manifold_pressure_display(), 2.25);
    assert_eq!(d.get_manifold_pressure_display_unit(), "Ata");
    assert_eq!(d.get_manifold_pressure_display_precision(), 2);

    // 英制 (checkAlt > 0): 值 = Boost(psi), 单位 = P/xx.x'' (live inHg), 精度 1
    d.check_alt = 1;
    assert!(d.is_imperial());
    assert!((d.get_manifold_pressure_pounds() - (2.25 - 1.0) * 14.696).abs() < 1e-12);
    // 2.25 * 760 / 25.4 = 67.322834... → %.1f HALF_UP → 67.3
    assert_eq!(d.get_manifold_pressure_inch_hg(), 2.25 * 760.0 / 25.4);
    assert_eq!(d.get_manifold_pressure_display_unit(), "P/67.3''");
    assert_eq!(d.get_manifold_pressure_display(), (2.25 - 1.0) * 14.696);
    assert_eq!(d.get_manifold_pressure_display_precision(), 1);

    // Float.parseFloat 单精度拓宽值的公式一致性 (mock 快照 2.24f32)
    d.check_alt = 0;
    let mut st2 = mock_state();
    st2.manifoldpressure = 2.24f32 as f64;
    d.s_state = Some(st2);
    d.check_alt = 5;
    let mp = 2.24f32 as f64;
    assert!((d.get_manifold_pressure_pounds() - (mp - 1.0) * 14.696).abs() < 1e-12);
    assert!((d.get_manifold_pressure_inch_hg() - mp * 760.0 / 25.4).abs() < 1e-12);
}

/// 引擎类型判断: checkEngineFlag 闸门 + 四型组合 (Java L2235-2272)
#[test]
fn engine_type_flags_require_check_done() {
    let mut d = ServiceData::default();
    // 检测未完成: 即便 iEngType 已有值也全 false
    d.i_eng_type = ENGINE_TYPE_JET;
    assert!(!d.check_engine_flag);
    assert!(!d.is_jet_engine());
    assert!(!d.is_prop_engine());
    assert!(!d.is_piston_engine());
    assert!(!d.is_turboprop_engine());
    assert!(!d.is_engine_check_done());

    d.check_engine_flag = true;
    d.i_eng_type = ENGINE_TYPE_JET;
    assert!(d.is_jet_engine());
    assert!(!d.is_prop_engine());
    assert!(!d.is_piston_engine());
    assert!(!d.is_turboprop_engine());

    d.i_eng_type = ENGINE_TYPE_PROP;
    assert!(!d.is_jet_engine());
    assert!(d.is_prop_engine());
    assert!(d.is_piston_engine());
    assert!(!d.is_turboprop_engine());

    d.i_eng_type = ENGINE_TYPE_TURBOPROP;
    assert!(!d.is_jet_engine());
    assert!(d.is_prop_engine());
    assert!(!d.is_piston_engine());
    assert!(d.is_turboprop_engine());

    // UNKNOWN: 既非 jet 也非 prop
    d.i_eng_type = ENGINE_TYPE_UNKNOWN;
    assert!(!d.is_jet_engine() && !d.is_prop_engine() && !d.is_piston_engine());
    assert!(d.is_engine_check_done(), "闸门只看 checkEngineFlag");
}

/// radioAltValid 的 Boolean 装箱三态 (null/false/true, Java L1989-1991)
#[test]
fn radio_alt_valid_null_semantics() {
    let mut d = ServiceData::default();
    d.radio_alt_valid = None;
    assert!(!d.is_radio_altitude_valid());
    d.radio_alt_valid = Some(false);
    assert!(!d.is_radio_altitude_valid());
    d.radio_alt_valid = Some(true);
    assert!(d.is_radio_altitude_valid());
}

/// 可变翼哨兵: -65535 → 0 且无效; 有效值直通 (Java L2121-2128/2189-2191)
#[test]
fn wing_sweep_sentinel() {
    let mut d = ServiceData::default();
    let mut i = Indicators::new();
    i.wsweep_indicator = F_INVALID;
    d.s_indic = Some(i);
    assert_eq!(d.get_wing_sweep(), 0.0);
    assert!(!d.is_wing_sweep_valid());

    let mut i2 = Indicators::new();
    i2.wsweep_indicator = 0.55;
    d.s_indic = Some(i2);
    assert_eq!(d.get_wing_sweep(), 0.55);
    assert!(d.is_wing_sweep_valid());
}

/// 助推器 (Issue #52): 哨兵归零 / 百分比封顶 / 组合判断 (Java L2153-2170)
#[test]
fn booster_sentinels_and_cap() {
    let mut d = ServiceData::default();
    let mut st = State::new();
    st.mfuel_1 = 200.0;
    st.mfuel0_1 = 100.0;
    d.s_state = Some(st);
    assert_eq!(d.get_booster_fuel_kg(), 200.0);
    assert_eq!(d.get_booster_fuel_percent(), 100.0, "Math.min(100, 200) 封顶");
    assert!(d.has_booster());

    // 经 struct 字段原地改 (State 非 Copy, 已移入 s_state)
    if let Some(s) = d.s_state.as_mut() {
        s.mfuel_1 = 25.0;
    }
    assert_eq!(d.get_booster_fuel_percent(), 25.0);

    // mfuel_1 哨兵 (-65535) → kg/hasBooster 归零; percent 守卫只看 mfuel0_1,
    // Java min(100, 100*(-65535)/100) = -65535.0 原样返回 (UI 端配合 hasBooster
    // 隐藏, 保真保留负值泄漏)
    if let Some(s) = d.s_state.as_mut() {
        s.mfuel_1 = -65535.0;
    }
    assert_eq!(d.get_booster_fuel_kg(), 0.0);
    assert_eq!(d.get_booster_fuel_percent(), -65535.0);
    assert!(!d.has_booster());

    // mfuel0_1 哨兵 → percent 归零, hasBooster false (mfuel_1 有效也不算)
    let mut st2 = State::new();
    st2.mfuel_1 = 100.0;
    st2.mfuel0_1 = -65535.0;
    d.s_state = Some(st2);
    assert_eq!(d.get_booster_fuel_kg(), 100.0);
    assert_eq!(d.get_booster_fuel_percent(), 0.0);
    assert!(!d.has_booster());
}

/// NaN 穿透语义 (§2.12 原样保持): Java 守卫 `mfuel_1 <= 0` / `mfuel0_1 <= 0` 对
/// NaN 判 false → 穿透; Math.min(double,double) NaN 传播。现解析层
/// (get_data_float) 只产哨兵或有效值, NaN 不可达 —— 本测试锁的是守卫极性
/// 不被未来改回 `> 0.0` (那会把 NaN 静默归零)
#[test]
fn nan_passthrough_matches_java_guard_polarity() {
    let mut d = ServiceData::default();
    let mut st = State::new();
    st.mfuel_1 = f64::NAN;
    st.mfuel0_1 = f64::NAN;
    d.s_state = Some(st);
    assert!(d.get_booster_fuel_kg().is_nan(), "NaN <= 0 为 false → 穿透");
    assert!(d.get_booster_fuel_percent().is_nan());
    // hasBooster 的 `mfuel_1 > 0 && mfuel0_1 > 0` 对 NaN 判 false (Java/Rust 同)
    assert!(!d.has_booster());
    // Math.min(thurstPercent, 100.0) NaN 传播 (f64::min 会返 100.0, 已手写复刻)
    d.thurst_percent = f64::NAN;
    assert!(d.get_power_percent().is_nan());
}

/// FM 周期快照驱动 get_total_weight / has_wep (Java L2038-2046/2280-2283)
#[test]
fn fm_snapshot_drives_total_weight_and_wep() {
    let mut d = ServiceData::default();
    d.s_state = Some(mock_state()); // mfuel = 197
    // UNRESOLVED (默认): blkx=null → 0 / false
    assert_eq!(d.get_total_weight(), 0.0);
    assert!(!d.has_wep());

    // READY + blkx: nofuelweight + mfuel; nitro > 0 → hasWep
    let mut blkx = Blkx::default();
    blkx.nofuelweight = 3000.0;
    blkx.nitro = 120.0;
    d.fm = Arc::new(FMHandle::ready(Some("bf109f-4".into()), Some(blkx), 0.0, 0.0, None));
    assert_eq!(d.get_total_weight(), 3000.0 + 197.0);
    assert!(d.has_wep());

    // nitro = 0 → hasWep false (无加力系统)
    let mut blkx2 = Blkx::default();
    blkx2.nofuelweight = 3000.0;
    blkx2.nitro = 0.0;
    d.fm = Arc::new(FMHandle::ready(Some("bf109f-4".into()), Some(blkx2), 0.0, 0.0, None));
    assert!(!d.has_wep());
    assert_eq!(d.get_total_weight(), 3197.0);

    // blkx 有但 sState null → 0 (守卫的另一半)
    d.s_state = None;
    assert_eq!(d.get_total_weight(), 0.0);
}

/// get_power_percent 的 Math.min 封顶 (Java L2179-2181)
#[test]
fn power_percent_caps_at_100() {
    let mut d = ServiceData::default();
    d.thurst_percent = 150.0;
    assert_eq!(d.get_power_percent(), 100.0);
    d.thurst_percent = 42.5;
    assert_eq!(d.get_power_percent(), 42.5);
    d.thurst_percent = 0.0;
    assert_eq!(d.get_power_percent(), 0.0);
}

/// get_pitch 对未 init 的 State (pitch 数组空) panic — 对应 Java
/// `sState.pitch[0]` 的 NPE (构造器窗口内不可达, 轮询线程 catch 兜底, §6)
#[test]
#[should_panic]
fn get_pitch_on_uninit_state_panics_like_java_npe() {
    let mut d = ServiceData::default();
    d.s_state = Some(State::new()); // 未 init: pitch 为空 Vec (≈ Java null)
    let _ = d.get_pitch();
}
