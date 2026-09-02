// PORT: Java 保真 — 测试构造沿用 Java `new X(); x.f = v;` 逐字段赋值形态,
// 不改成 struct 字面量以保持与 Java 测试源逐行对应
#![allow(clippy::field_reassign_with_default)]

use super::*;

/// Java 8 oracle: `new HUDData.Builder()` 的全部默认值
/// (ias=0.0 ... throttle=0, pitchValid=false, 三色 GREEN, 全部空串, sb_rudder=0.0)。
#[test]
fn builder_defaults_match_java() {
    let b = Builder::default();
    assert_eq!(b.ias, 0.0);
    assert_eq!(b.mach, 0.0);
    assert_eq!(b.altitude, 0.0);
    assert_eq!(b.radio_altitude, 0.0);
    assert_eq!(b.vertical_speed, 0.0);
    assert_eq!(b.heading, 0.0);
    assert_eq!(b.pitch, 0.0);
    assert_eq!(b.roll, 0.0);
    assert_eq!(b.slip, 0.0);
    assert_eq!(b.aoa, 0.0);
    assert!(!b.pitch_valid);
    assert_eq!(b.throttle, 0);
    assert_eq!(b.flaps, 0.0);
    assert_eq!(b.gear, 0.0);
    assert_eq!(b.airbrake, 0.0);
    assert_eq!(b.flap_allow_angle, 0.0);
    assert_eq!(b.energy_m, 0.0);
    assert_eq!(b.g_load, 0.0);
    assert_eq!(b.turn_rate, 0.0);
    assert_eq!(b.maneuver_index, 0.0);
    assert!(!b.is_mach_mode);
    assert!(!b.is_gear_down);
    assert!(!b.is_flaps_down);
    assert!(!b.is_airbrake_active);
    assert!(!b.warn_vne);
    assert!(!b.warn_stall);
    assert!(!b.warn_gear);
    assert!(!b.warn_altitude);
    assert_eq!(b.aoa_color, [0, 255, 0, 255]);
    assert_eq!(b.aoa_bar_color, [0, 255, 0, 255]);
    assert_eq!(b.throttle_color, [0, 255, 0, 255]);
    assert_eq!(b.map_grid, "");
    assert_eq!(b.time_str, "");
    assert_eq!(b.speed_str, "");
    assert_eq!(b.alt_str, "");
    assert_eq!(b.aoa_str, "");
    assert_eq!(b.energy_str, "");
    assert_eq!(b.mechanization_str, "");
    assert_eq!(b.flaps_wing_str, "");
    assert_eq!(b.airbrake_str, "");
    assert_eq!(b.gear_str, "");
    assert_eq!(b.sep_str, "");
    assert_eq!(b.maneuver_state_str, "");
    assert!(!b.warn_configuration);
    assert_eq!(b.aoa_ratio, 0.0);
    assert_eq!(b.speed_bar_speed_ratio, 0.0);
    assert_eq!(b.speed_bar_stall_ratio, 0.0);
    assert_eq!(b.speed_bar_unit_mach_limit_ratio, 0.0);
    assert_eq!(b.speed_bar_aileron_lock_ratio, 0.0);
    assert_eq!(b.speed_bar_rudder_lock_ratio, 0.0);
}

/// build() 逐字段拷贝: 对全部字段设置非默认哨兵值后核对。
#[test]
fn build_copies_all_fields() {
    let mut b = Builder::default();
    b.ias = 412.5;
    b.mach = 0.82;
    b.altitude = 5300.0;
    b.radio_altitude = 245.7;
    b.vertical_speed = -13.2;
    b.heading = 359.9;
    b.pitch = -12.25;
    b.roll = 60.0;
    b.slip = 0.5;
    b.aoa = 18.3;
    b.pitch_valid = true;
    b.throttle = 110;
    b.flaps = 100.0;
    b.gear = 55.0;
    b.airbrake = 20.0;
    b.flap_allow_angle = 25.0;
    b.energy_m = 1520.4;
    b.g_load = 7.5;
    b.turn_rate = 22.4;
    b.maneuver_index = 0.97;
    b.is_mach_mode = true;
    b.is_gear_down = true;
    b.is_flaps_down = true;
    b.is_airbrake_active = true;
    b.warn_vne = true;
    b.warn_stall = true;
    b.warn_gear = true;
    b.warn_altitude = true;
    b.aoa_color = [255, 64, 64, 255];
    b.aoa_bar_color = [255, 192, 0, 128];
    b.throttle_color = [64, 255, 128, 200];
    b.map_grid = "C4".to_string();
    b.time_str = "12:34".to_string();
    b.speed_str = "413".to_string();
    b.alt_str = "5300".to_string();
    b.aoa_str = "18.3".to_string();
    b.energy_str = "1520".to_string();
    b.mechanization_str = "F100".to_string();
    b.flaps_wing_str = "W 50".to_string();
    b.airbrake_str = "BRK".to_string();
    b.gear_str = "GEA".to_string();
    b.sep_str = "-13".to_string();
    b.maneuver_state_str = "OVERLOAD".to_string();
    b.warn_configuration = true;
    b.aoa_ratio = 1.05;
    b.speed_bar_speed_ratio = 0.83;
    b.speed_bar_stall_ratio = 0.31;
    b.speed_bar_unit_mach_limit_ratio = 0.9;
    b.speed_bar_aileron_lock_ratio = 0.72;
    b.speed_bar_rudder_lock_ratio = 0.66;

    let h = b.build();
    assert_eq!(h.ias, 412.5);
    assert_eq!(h.mach, 0.82);
    assert_eq!(h.altitude, 5300.0);
    assert_eq!(h.radio_altitude, 245.7);
    assert_eq!(h.vertical_speed, -13.2);
    assert_eq!(h.heading, 359.9);
    assert_eq!(h.pitch, -12.25);
    assert_eq!(h.roll, 60.0);
    assert_eq!(h.slip, 0.5);
    assert_eq!(h.aoa, 18.3);
    assert!(h.pitch_valid);
    assert_eq!(h.throttle, 110);
    assert_eq!(h.flaps, 100.0);
    assert_eq!(h.gear, 55.0);
    assert_eq!(h.airbrake, 20.0);
    assert_eq!(h.flap_allow_angle, 25.0);
    assert_eq!(h.energy_m, 1520.4);
    assert_eq!(h.g_load, 7.5);
    assert_eq!(h.turn_rate, 22.4);
    assert_eq!(h.maneuver_index, 0.97);
    assert!(h.is_mach_mode);
    assert!(h.is_gear_down);
    assert!(h.is_flaps_down);
    assert!(h.is_airbrake_active);
    assert!(h.warn_vne);
    assert!(h.warn_stall);
    assert!(h.warn_gear);
    assert!(h.warn_altitude);
    assert_eq!(h.aoa_color, [255, 64, 64, 255]);
    assert_eq!(h.aoa_bar_color, [255, 192, 0, 128]);
    assert_eq!(h.throttle_color, [64, 255, 128, 200]);
    assert_eq!(h.map_grid, "C4");
    assert_eq!(h.time_str, "12:34");
    assert_eq!(h.speed_str, "413");
    assert_eq!(h.alt_str, "5300");
    assert_eq!(h.aoa_str, "18.3");
    assert_eq!(h.energy_str, "1520");
    assert_eq!(h.mechanization_str, "F100");
    assert_eq!(h.flaps_wing_str, "W 50");
    assert_eq!(h.airbrake_str, "BRK");
    assert_eq!(h.gear_str, "GEA");
    assert_eq!(h.sep_str, "-13");
    assert_eq!(h.maneuver_state_str, "OVERLOAD");
    assert_eq!(h.aoa_ratio, 1.05);
    assert!(h.warn_configuration);
    assert_eq!(h.speed_bar_speed_ratio, 0.83);
    assert_eq!(h.speed_bar_stall_ratio, 0.31);
    assert_eq!(h.speed_bar_unit_mach_limit_ratio, 0.9);
    assert_eq!(h.speed_bar_aileron_lock_ratio, 0.72);
    assert_eq!(h.speed_bar_rudder_lock_ratio, 0.66);
}

/// Java `build()` 非消费: 同一 Builder 重复 build 得到相等 (但独立) 的实例。
#[test]
fn builder_is_reusable() {
    let mut b = Builder::default();
    b.ias = 250.0;
    b.map_grid = "A1".to_string();
    let h1 = b.build();
    let h2 = b.build();
    assert_eq!(h1, h2);
    // Java 每次返回 new HUDData (独立实例); Rust 值语义下以变异隔离证明非别名
    let mut h1 = h1;
    h1.ias = 999.0;
    assert_ne!(h1.ias, h2.ias);
    assert_eq!(h2.ias, 250.0);
}
