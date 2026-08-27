use super::*;

/// 最小 mock 实现: 锁定 trait 的方法签名全集 (任何签名漂移即编译失败)
/// 与对象安全 (消费方以 `TelemetrySource` 接口引用 Service, 如
/// `MyOverlay.init` 的 `this.source = service`)。
struct MockTelemetry {
    ias: f64,
    fuel_time_mili: i64,
    manifold_unit: &'static str,
    manifold_precision: i32,
    jet: bool,
    imperial: bool,
}

impl TelemetrySource for MockTelemetry {
    fn get_ias(&self) -> f64 { self.ias }
    fn get_tas(&self) -> f64 { 0.0 }
    fn get_mach(&self) -> f64 { 0.0 }
    fn get_aoa(&self) -> f64 { 0.0 }
    fn get_aos(&self) -> f64 { 0.0 }
    fn get_ny(&self) -> f64 { 0.0 }
    fn get_vario(&self) -> f64 { 0.0 }
    fn get_altitude(&self) -> f64 { 0.0 }
    fn get_radio_altitude(&self) -> f64 { 0.0 }
    fn is_radio_altitude_valid(&self) -> bool { false }
    fn get_compass(&self) -> f64 { 0.0 }
    fn get_sep(&self) -> f64 { 0.0 }
    fn get_acceleration(&self) -> f64 { 0.0 }
    fn get_turn_rate(&self) -> f64 { 0.0 }
    fn get_turn_radius(&self) -> f64 { 0.0 }
    fn is_turn_radius_valid(&self) -> bool { false }
    fn get_roll_rate(&self) -> f64 { 0.0 }
    fn get_energy_jkg(&self) -> f64 { 0.0 }
    fn get_mass_fuel(&self) -> f64 { 0.0 }
    fn get_total_weight(&self) -> f64 { 0.0 }
    fn get_fuel_time_mili(&self) -> i64 { self.fuel_time_mili }
    fn get_throttle(&self) -> f64 { 0.0 }
    fn get_rpm(&self) -> f64 { 0.0 }
    fn get_manifold_pressure(&self) -> f64 { 0.0 }
    fn get_water_temp(&self) -> f64 { 0.0 }
    fn get_oil_temp(&self) -> f64 { 0.0 }
    fn get_pitch(&self) -> f64 { 0.0 }
    fn get_eff_hp(&self) -> f64 { 0.0 }
    fn get_thrust(&self) -> f64 { 0.0 }
    fn get_horse_power(&self) -> f64 { 0.0 }
    fn get_engine_response(&self) -> f64 { 0.0 }
    fn get_prop_efficiency(&self) -> f64 { 0.0 }
    fn get_wep_kg(&self) -> f64 { 0.0 }
    fn get_wep_time(&self) -> f64 { 0.0 }
    fn get_heat_tolerance(&self) -> f64 { 0.0 }
    fn get_power_percent(&self) -> f64 { 0.0 }
    fn get_manifold_pressure_pounds(&self) -> f64 { 0.0 }
    fn get_manifold_pressure_inch_hg(&self) -> f64 { 0.0 }
    fn get_manifold_pressure_display(&self) -> f64 { 0.0 }
    fn get_manifold_pressure_display_unit(&self) -> String { self.manifold_unit.to_string() }
    fn get_manifold_pressure_display_precision(&self) -> i32 { self.manifold_precision }
    fn get_unknown_mixture(&self) -> f64 { 0.0 }
    fn get_radiator(&self) -> f64 { 0.0 }
    fn get_compressor_stage(&self) -> f64 { 0.0 }
    fn get_fuel_percent(&self) -> f64 { 0.0 }
    fn get_rpm_throttle(&self) -> f64 { 0.0 }
    fn get_gear(&self) -> f64 { 0.0 }
    fn get_flaps(&self) -> f64 { 0.0 }
    fn get_airbrake(&self) -> f64 { 0.0 }
    fn get_aileron(&self) -> f64 { 0.0 }
    fn get_elevator(&self) -> f64 { 0.0 }
    fn get_rudder(&self) -> f64 { 0.0 }
    fn get_wing_sweep(&self) -> f64 { 0.0 }
    fn is_wing_sweep_valid(&self) -> bool { false }
    fn get_speed_limit_ratio(&self) -> f64 { 0.0 }
    fn get_aileron_lock_ratio(&self) -> f64 { 0.0 }
    fn get_rudder_lock_ratio(&self) -> f64 { 0.0 }
    fn get_unit_mach_limit_ratio(&self) -> f64 { 0.0 }
    fn get_stall_speed(&self) -> f64 { 0.0 }
    fn is_imperial(&self) -> bool { self.imperial }
    fn get_aviahorizon_pitch(&self) -> f64 { 0.0 }
    fn get_aviahorizon_roll(&self) -> f64 { 0.0 }
    fn is_jet_engine(&self) -> bool { self.jet }
    fn is_prop_engine(&self) -> bool { false }
    fn is_piston_engine(&self) -> bool { false }
    fn is_turboprop_engine(&self) -> bool { false }
    fn is_engine_check_done(&self) -> bool { false }
    fn has_wep(&self) -> bool { false }
    fn get_booster_fuel_kg(&self) -> f64 { 0.0 }
    fn get_booster_fuel_percent(&self) -> f64 { 0.0 }
    fn has_booster(&self) -> bool { false }
}

#[test]
fn trait_is_object_safe_and_dispatches() {
    let mock = MockTelemetry {
        ias: 502.5,
        fuel_time_mili: 2_700_000i64,
        manifold_unit: "P/30.1''",
        manifold_precision: 1,
        jet: true,
        imperial: true,
    };
    let src: Box<dyn TelemetrySource> = Box::new(mock);
    assert_eq!(src.get_ias(), 502.5);
    assert_eq!(src.get_fuel_time_mili(), 2_700_000i64, "Java long → i64");
    assert_eq!(src.get_manifold_pressure_display_unit(), "P/30.1''");
    assert_eq!(src.get_manifold_pressure_display_precision(), 1);
    assert!(src.is_jet_engine());
    assert!(src.is_imperial());
}
