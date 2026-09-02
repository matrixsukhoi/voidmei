// PORT: Java 保真 — 测试构造沿用 Java `new X(); x.f = v;` 逐字段赋值形态,
// 不改成 struct 字面量以保持与 Java 测试源逐行对应
#![allow(clippy::field_reassign_with_default)]

use super::*;

/// Tests for PistonPowerModel.
///
/// Validates piston engine power curve calculations.
///
/// Run with: ./script/test.sh
///
/// PORT: Java 断言助手 assertClose/assertTrue (计数式 pass/fail) → Rust
/// assert! 宏 (失败即 panic); `Math.abs(a-e) <= tol` 判定式逐字保留。
/// Java 的 printf 信息输出 (非断言) 移植不保留
fn assert_close(name: &str, actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "FAIL: {name} = {actual:.4} (expected {expected:.4}, tolerance {tolerance:.4})"
    );
}

fn assert_true(name: &str, condition: bool) {
    assert!(condition, "FAIL: {name}");
}

fn test_torque_rpm_boost() {
    // Testing torqueRpmBoost()...

    // Same RPM = no boost
    assert_close("same RPM", torque_rpm_boost(2400.0, 2400.0), 1.0, 0.001);

    // Higher RPM = boost > 1
    let boost = torque_rpm_boost(2400.0, 2600.0);
    assert_true("higher RPM gives boost", boost > 1.0);

    // Invalid inputs
    assert_close("zero lowerRPM", torque_rpm_boost(0.0, 2600.0), 1.0, 0.001);
    assert_close("zero higherRPM", torque_rpm_boost(2400.0, 0.0), 1.0, 0.001);

    // Boost is monotonic with RPM difference
    let boost1 = torque_rpm_boost(2400.0, 2500.0);
    let boost2 = torque_rpm_boost(2400.0, 2600.0);
    let boost3 = torque_rpm_boost(2400.0, 2700.0);
    assert_true("boost increases with RPM diff", boost1 < boost2 && boost2 < boost3);
}

fn test_torque_from_hp() {
    // Testing torqueFromHp()...

    // Known conversion: 726.115 * hp / rpm
    let torque = torque_from_hp(1000.0, 2400.0);
    assert_close("torque at 1000hp/2400rpm", torque, 302.55, 0.1);

    // Zero RPM handling
    assert_close("zero RPM", torque_from_hp(1000.0, 0.0), 0.0, 0.001);

    // Torque proportional to power
    let torque2 = torque_from_hp(2000.0, 2400.0);
    assert_close("double power = double torque", torque2, torque * 2.0, 0.1);

    // Torque inversely proportional to RPM
    let torque3 = torque_from_hp(1000.0, 4800.0);
    assert_close("double RPM = half torque", torque3, torque / 2.0, 0.1);
}

fn test_supercharger_rpm_effect() {
    // Testing superchargerRpmEffect()...

    // Typical parameters
    let effect = supercharger_rpm_effect(2400.0, 2600.0, 0.2, 0.1);
    assert_true("effect > 1 with RPM increase", effect > 1.0);

    // No RPM difference = no effect
    assert_close("same RPM", supercharger_rpm_effect(2400.0, 2400.0, 0.2, 0.1), 1.0, 0.001);

    // Invalid input
    assert_close("zero milRPM", supercharger_rpm_effect(0.0, 2600.0, 0.2, 0.1), 1.0, 0.001);

    // Higher compressor factors = more effect
    let effect1 = supercharger_rpm_effect(2400.0, 2600.0, 0.1, 0.1);
    let effect2 = supercharger_rpm_effect(2400.0, 2600.0, 0.3, 0.1);
    assert_true("lower pressureAtRPM0 = more effect", effect1 > effect2);
}

fn test_interpolate_power() {
    // Testing interpolatePower()...

    // At lower altitude point
    let at_lower = interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 0.0, 1.0);
    assert_close("at lower point", at_lower, 2000.0, 1.0);

    // At higher altitude point
    let at_higher = interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 5000.0, 1.0);
    assert_close("at higher point", at_higher, 1800.0, 1.0);

    // In between (should be between the two values)
    let at_mid = interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 2500.0, 1.0);
    assert_true("mid point between", at_mid > 1800.0 && at_mid < 2000.0);

    // Curvature effect
    let linear = interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 2500.0, 1.0);
    let curved = interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 2500.0, 2.0);
    assert_true("curvature affects interpolation", (linear - curved).abs() > 1.0);
}

fn test_wep_power_multiplier() {
    // Testing wepPowerMultiplier()...

    // All factors = 1 and same RPM = multiplier of 1
    assert_close("baseline", wep_power_multiplier(1.0, 1.0, 1.0, 1.0, 2400.0, 2400.0), 1.0, 0.01);

    // Typical WEP parameters
    let mult = wep_power_multiplier(1.15, 1.0, 1.0, 1.0, 2400.0, 2600.0);
    assert_true("WEP mult > 1", mult > 1.0);

    // AfterburnerBoost effect
    let mult1 = wep_power_multiplier(1.10, 1.0, 1.0, 1.0, 2400.0, 2400.0);
    let mult2 = wep_power_multiplier(1.20, 1.0, 1.0, 1.0, 2400.0, 2400.0);
    assert_true("higher boost = higher mult", mult2 > mult1);
}

fn test_wep_critical_altitude() {
    // Testing wepCriticalAltitude()...

    // WEP requires higher manifold pressure, so critical altitude is lower
    let wep_crit_alt = wep_critical_altitude(7000.0, 1.42, 1.65, 1.0, 1.0);
    assert_true("WEP crit alt < mil crit alt", wep_crit_alt < 7000.0);

    // With supercharger RPM effect boost
    let wep_crit_alt2 = wep_critical_altitude(7000.0, 1.42, 1.65, 1.1, 1.0);
    assert_true("RPM effect raises WEP crit alt", wep_crit_alt2 > wep_crit_alt);

    // With pressure boost
    let wep_crit_alt3 = wep_critical_altitude(7000.0, 1.42, 1.65, 1.0, 1.1);
    assert_true("pressure boost raises WEP crit alt", wep_crit_alt3 > wep_crit_alt);
}

fn test_power_at_altitude() {
    // Testing powerAtAltitudeAdvanced()...

    // Create test engine (simplified P-47D-like)
    let mut stage = CompressorStageParams::default();
    stage.crit_alt = 7000.0;
    stage.crit_power = 2000.0;
    stage.deck_power = 1850.0;
    stage.deck_alt = 0.0;
    stage.curvature = 1.0;
    stage.wep_crit_alt = 6000.0;
    stage.wep_power_mult = 1.15;
    stage.speed_manifold_mult = 0.9;
    stage.old_altitude = 7000.0;
    stage.old_power = 2000.0;
    stage.old_power_new_rpm = 2000.0;

    // At sea level
    let p0 = power_at_altitude_advanced(&stage, 0.0, false, 0.0, false, 15.0);
    assert_close("power at sea level", p0, 1850.0, 10.0);

    // At critical altitude
    let p_crit = power_at_altitude_advanced(&stage, 7000.0, false, 0.0, false, 15.0);
    assert_close("power at crit alt", p_crit, 2000.0, 10.0);

    // Above critical altitude (power drops)
    let p_high = power_at_altitude_advanced(&stage, 9000.0, false, 0.0, false, 15.0);
    assert_true("power drops above crit alt", p_high < p_crit);

    // WEP gives more power
    let p_wep = power_at_altitude_advanced(&stage, 5000.0, true, 0.0, false, 15.0);
    let p_mil = power_at_altitude_advanced(&stage, 5000.0, false, 0.0, false, 15.0);
    assert_true("WEP > military", p_wep > p_mil);
}

fn test_multi_stage_supercharger() {
    // Testing multi-stage supercharger...

    // Two-stage supercharger (like Spitfire Merlin)
    let mut stage1 = CompressorStageParams::default();
    stage1.crit_alt = 3000.0;
    stage1.crit_power = 1400.0;
    stage1.deck_power = 1350.0;
    stage1.wep_crit_alt = 2500.0;
    stage1.wep_power_mult = 1.1;
    stage1.stage_index = 0;
    stage1.old_altitude = 3000.0;
    stage1.old_power = 1400.0;
    stage1.old_power_new_rpm = 1400.0;

    let mut stage2 = CompressorStageParams::default();
    stage2.crit_alt = 6500.0;
    stage2.crit_power = 1300.0;
    stage2.deck_power = 1100.0;
    stage2.wep_crit_alt = 6000.0;
    stage2.wep_power_mult = 1.1;
    stage2.stage_index = 1;
    stage2.old_altitude = 6500.0;
    stage2.old_power = 1300.0;
    stage2.old_power_new_rpm = 1300.0;

    let stages = [stage1, stage2];

    // At low altitude, stage 1 is better
    let p_low = optimal_power_advanced(&stages, 1000.0, false, 0.0, false, 15.0);
    let p1_low = power_at_altitude_advanced(&stage1, 1000.0, false, 0.0, false, 15.0);
    let p2_low = power_at_altitude_advanced(&stage2, 1000.0, false, 0.0, false, 15.0);
    assert_true("stage 1 better at low alt", p1_low > p2_low);
    assert_close("optimal selects stage 1", p_low, p1_low, 1.0);

    // At high altitude, stage 2 is better
    let p_high = optimal_power_advanced(&stages, 6000.0, false, 0.0, false, 15.0);
    let p1_high = power_at_altitude_advanced(&stage1, 6000.0, false, 0.0, false, 15.0);
    let p2_high = power_at_altitude_advanced(&stage2, 6000.0, false, 0.0, false, 15.0);
    assert_true("stage 2 better at high alt", p2_high > p1_high);
    assert_close("optimal selects stage 2", p_high, p2_high, 1.0);
}

fn test_generate_power_curve() {
    // Testing generatePowerCurveAdvanced()...

    let mut stage = CompressorStageParams::new(5000.0, 1500.0, 1400.0);
    stage.wep_crit_alt = 4500.0;
    stage.wep_power_mult = 1.1;
    stage.old_altitude = 5000.0;
    stage.old_power = 1500.0;
    stage.old_power_new_rpm = 1500.0;
    let stages = [stage];

    let curve = generate_power_curve_advanced(&stages, false, 0.0, false, 15.0, 50);

    // Check array size (range: 0m to 10000m, step 50m)
    let expected_size = 10000 / 50 + 1; // 201 points
    assert_true("curve size correct", curve.len() == expected_size);

    // Check values at known altitudes (index = altitude / step)
    let sea_level_idx = 0usize;
    assert_close("curve at sea level", curve[sea_level_idx], 1400.0, 10.0);

    // Index at 5000m = 5000 / 50 = 100
    let crit_alt_idx = 5000 / 50;
    assert_close("curve at crit alt", curve[crit_alt_idx], 1500.0, 10.0);

    // Power decreases after critical altitude (8000m = index 160)
    let high_alt_idx = 8000 / 50;
    assert_true("power decreases at high alt", curve[high_alt_idx] < curve[crit_alt_idx]);
}

fn test_ram_effect_integration() {
    // Testing RAM effect integration...

    let mut stage = CompressorStageParams::default();
    stage.crit_alt = 7000.0;
    stage.crit_power = 2000.0;
    stage.deck_power = 1850.0;
    stage.wep_crit_alt = 6500.0;
    stage.wep_power_mult = 1.1;
    stage.speed_manifold_mult = 0.9;
    stage.old_altitude = 7000.0;
    stage.old_power = 2000.0;
    stage.old_power_new_rpm = 2000.0;

    // Test RAM effect well ABOVE critical altitude where power drops with altitude
    // Use higher altitude to avoid effective altitude crossing critical altitude
    let p_static_12k = power_at_altitude_advanced(&stage, 12000.0, false, 0.0, false, 15.0);
    let p_moving_12k = power_at_altitude_advanced(&stage, 12000.0, false, 400.0, true, 15.0);

    // RAM effect should increase power above critical altitude
    assert_true("RAM increases power above crit alt", p_moving_12k > p_static_12k);

    // Higher speed = more RAM effect (use moderate speeds to stay above crit alt)
    let p_faster_12k = power_at_altitude_advanced(&stage, 12000.0, false, 500.0, true, 15.0);
    assert_true("faster = more RAM above crit alt", p_faster_12k > p_moving_12k);

    // Note: Below critical altitude, RAM effect may DECREASE power because
    // in this model deck power < crit power (power increases toward crit alt).
    // This is physically correct for supercharged engines where the
    // supercharger maintains constant boost up to critical altitude.
    let _p_static_low = power_at_altitude_advanced(&stage, 5000.0, false, 0.0, false, 15.0);
    let _p_moving_low = power_at_altitude_advanced(&stage, 5000.0, false, 500.0, true, 15.0);
    // (Below crit alt, RAM lowers effective alt, which may reduce power)
}

fn test_no_wep_aircraft_identical_curves() {
    // Testing no-WEP aircraft (Yak-3 style)...

    // Yak-3 Stage 0: critAlt=300m, Power=1310hp, Ceiling=5000m/670hp
    // AfterburnerBoost=1 → wepPowerMult=1.0, so WEP == military
    let mut stage0 = CompressorStageParams::default();
    stage0.crit_alt = 300.0;
    stage0.crit_power = 1310.0;
    stage0.deck_power = 1290.0;
    stage0.deck_alt = 0.0;
    stage0.curvature = 1.0;
    stage0.ceiling_alt = 5000.0;
    stage0.ceiling_power = 670.0;
    stage0.old_altitude = 300.0;
    stage0.old_power = 1310.0;
    stage0.old_power_new_rpm = 1310.0;
    stage0.exact_altitudes = true;
    stage0.stage_index = 0;
    // No WEP: wepPowerMult=1.0, wepCritAlt == critAlt
    stage0.wep_power_mult = 1.0;
    stage0.wep_crit_alt = 300.0; // Must equal critAlt when no WEP
    stage0.wep_deck_alt = 0.0;
    stage0.speed_manifold_mult = 1.0;

    // Yak-3 Stage 1: critAlt=2600m, Power=1240hp, Ceiling=9000m/510hp
    let mut stage1 = CompressorStageParams::default();
    stage1.crit_alt = 2600.0;
    stage1.crit_power = 1240.0;
    stage1.deck_power = 1290.0 * 0.8;
    stage1.deck_alt = 0.0;
    stage1.curvature = 1.0;
    stage1.ceiling_alt = 9000.0;
    stage1.ceiling_power = 510.0;
    stage1.old_altitude = 2600.0;
    stage1.old_power = 1240.0;
    stage1.old_power_new_rpm = 1240.0;
    stage1.exact_altitudes = true;
    stage1.stage_index = 1;
    stage1.wep_power_mult = 1.0;
    stage1.wep_crit_alt = 2600.0;
    stage1.wep_deck_alt = 0.0;
    stage1.speed_manifold_mult = 1.0;

    let stages = [stage0, stage1];

    // WEP and military curves must be identical at every altitude
    let mut all_match = true;
    // PORT: Java `for (int alt = 0; alt <= 10000; alt += 500)` int 步进循环
    for alt in (0..=10000i32).step_by(500) {
        let alt_f = alt as f64;
        let mil_power = optimal_power_advanced(&stages, alt_f, false, 0.0, false, 15.0);
        let wep_power = optimal_power_advanced(&stages, alt_f, true, 0.0, false, 15.0);
        if (mil_power - wep_power).abs() > 0.1 {
            all_match = false;
        }
    }
    assert_true("no-WEP: WEP curve equals military curve", all_match);

    // Also verify with RAM effect (301 km/h IAS)
    let mut all_match_ram = true;
    for alt in (0..=10000i32).step_by(500) {
        let alt_f = alt as f64;
        let mil_power = optimal_power_advanced(&stages, alt_f, false, 301.0, true, 15.0);
        let wep_power = optimal_power_advanced(&stages, alt_f, true, 301.0, true, 15.0);
        if (mil_power - wep_power).abs() > 0.1 {
            all_match_ram = false;
        }
    }
    assert_true("no-WEP with RAM: WEP curve equals military curve", all_match_ram);
}

fn test_peak_wep_power() {
    // Testing peakWepPower()...

    // Test 1: Single-stage supercharger
    let mut stage = CompressorStageParams::default();
    stage.crit_alt = 5000.0;
    stage.crit_power = 1500.0;
    stage.deck_power = 1400.0;
    stage.wep_crit_alt = 4500.0;
    stage.wep_power_mult = 1.15; // 15% WEP boost
    stage.old_altitude = 5000.0;
    stage.old_power = 1500.0;
    stage.old_power_new_rpm = 1500.0;
    let single_stage = [stage];

    let peak_single = peak_wep_power(&single_stage);
    // Peak should be critPower × wepPowerMult = 1500 × 1.15 = 1725 hp (approximately)
    let expected_peak = stage.crit_power * stage.wep_power_mult;
    assert_close("single-stage peak WEP power", peak_single, expected_peak, 20.0);

    // Test 2: Multi-stage supercharger (peak is max across all stages)
    let mut stage1 = CompressorStageParams::default();
    stage1.crit_alt = 3000.0;
    stage1.crit_power = 1400.0;
    stage1.deck_power = 1350.0;
    stage1.wep_crit_alt = 2500.0;
    stage1.wep_power_mult = 1.1;
    stage1.stage_index = 0;
    stage1.old_altitude = 3000.0;
    stage1.old_power = 1400.0;
    stage1.old_power_new_rpm = 1400.0;

    let mut stage2 = CompressorStageParams::default();
    stage2.crit_alt = 6500.0;
    stage2.crit_power = 1300.0;
    stage2.deck_power = 1100.0;
    stage2.wep_crit_alt = 6000.0;
    stage2.wep_power_mult = 1.1;
    stage2.stage_index = 1;
    stage2.old_altitude = 6500.0;
    stage2.old_power = 1300.0;
    stage2.old_power_new_rpm = 1300.0;

    let multi_stage = [stage1, stage2];
    let peak_multi = peak_wep_power(&multi_stage);
    // Stage 1 should give higher peak (1400 × 1.1 = 1540hp vs 1300 × 1.1 = 1430hp)
    let expected_multi_peak = stage1.crit_power * stage1.wep_power_mult;
    assert_close("multi-stage peak WEP power", peak_multi, expected_multi_peak, 20.0);

    // Test 3: Empty array returns 0
    let peak_empty = peak_wep_power(&[]);
    assert_close("empty array returns 0", peak_empty, 0.0, 0.001);

    // Test 4: Null array returns 0
    // PORT: Java null 参数在 Rust 切片模型下无对应 (无 null 切片),
    // 空切片走同一 `stages.length == 0` 提前返回路径, 行为等价 (0)
    // (与 Test 3 同路径, 保留用例编号以对齐 Java 测试文本)

    // Test 5: No-WEP aircraft (wepPowerMult = 1.0)
    let mut no_wep_stage = CompressorStageParams::default();
    no_wep_stage.crit_alt = 3000.0;
    no_wep_stage.crit_power = 1200.0;
    no_wep_stage.deck_power = 1150.0;
    no_wep_stage.wep_crit_alt = 3000.0;
    no_wep_stage.wep_power_mult = 1.0; // No WEP
    no_wep_stage.old_altitude = 3000.0;
    no_wep_stage.old_power = 1200.0;
    no_wep_stage.old_power_new_rpm = 1200.0;
    let no_wep_array = [no_wep_stage];

    let peak_no_wep = peak_wep_power(&no_wep_array);
    // With no WEP, peak should equal critPower
    assert_close("no-WEP peak equals critPower", peak_no_wep, no_wep_stage.crit_power, 10.0);
}

/// Java 8 oracle 对拍 (PORTING.md §5.1 A 类策略):
/// 期望值 = build/oracle/PistonPowerOracle{,2}.java 在 OpenJDK 1.8.0_342 上
/// dump 的 %.17g 实测值 (临时文件, 用完已删除)。容差取混合式 1e-12·max(|expected|,1)
/// (|expected|<1 时退化为绝对容差): Math.pow 跨 libm 实现允许最后几位 ULP 差异,
/// 远小于业务断言容差。覆盖 variabler 军用/WEP 全部分支形态。
#[test]
fn java8_oracle_parity() {
    let tol = 1e-12;
    let check = |name: &str, actual: f64, expected: f64| {
        // 混合容差: expected 为 0 时退化为绝对容差
        let diff = (actual - expected).abs();
        assert!(
            diff <= tol * expected.abs().max(1.0),
            "oracle mismatch {name}: rust={actual:?} java={expected:?}"
        );
    };

    // === Part A: 纯函数 ===
    check("tq_same", torque_rpm_boost(2400.0, 2400.0), 1.0);
    check("tq_2600", torque_rpm_boost(2400.0, 2600.0), 1.0171296296296297);
    check("tq_2700", torque_rpm_boost(2400.0, 2700.0), 1.0355113636363635);
    check("tq_2500", torque_rpm_boost(2400.0, 2500.0), 1.0046939300411524);
    check("tq_zero_lo", torque_rpm_boost(0.0, 2600.0), 1.0);
    check("tq_zero_hi", torque_rpm_boost(2400.0, 0.0), 1.0);
    check("tqf_1000_2400", torque_from_hp(1000.0, 2400.0), 302.54791666666665);
    check("tqf_1000_4800", torque_from_hp(1000.0, 4800.0), 151.27395833333333);
    check("tqf_2000_2400", torque_from_hp(2000.0, 2400.0), 605.0958333333333);
    check("tqf_zero_rpm", torque_from_hp(1000.0, 0.0), 0.0);
    check("sce_typ", supercharger_rpm_effect(2400.0, 2600.0, 0.2, 0.1), 1.0735730379653925);
    check("sce_same", supercharger_rpm_effect(2400.0, 2400.0, 0.2, 0.1), 1.0);
    check("sce_p03", supercharger_rpm_effect(2400.0, 2600.0, 0.3, 0.1), 1.0643506320619338);
    check("sce_om05", supercharger_rpm_effect(2400.0, 2600.0, 0.2, 0.5), 1.1016485962545541);
    check("ip_low", interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 0.0, 1.0), 2000.0);
    check("ip_high", interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 5000.0, 1.0), 1800.0);
    check("ip_mid1", interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 2500.0, 1.0), 1887.3589678159094);
    check("ip_mid2", interpolate_power(1800.0, 5000.0, 2000.0, 0.0, 2500.0, 2.0), 1936.5599893425133);
    check("ip_below", interpolate_power(1800.0, 5000.0, 2000.0, 0.0, -100.0, 1.0), 2005.1034456173313);
    check("ip_degen", interpolate_power(1000.0, 3000.0, 990.0, 3000.0, 2500.0, 1.0), 990.0);
    check("wepm_115", wep_power_multiplier(1.15, 1.0, 1.0, 1.0, 2400.0, 2600.0), 1.169_699_074_074_074);
    check("wepm_110", wep_power_multiplier(1.10, 1.0, 1.0, 1.0, 2400.0, 2400.0), 1.1);
    check("wepm_120", wep_power_multiplier(1.20, 1.0, 1.0, 1.0, 2400.0, 2400.0), 1.2);
    check("wepm_oct18", wep_power_multiplier(1.15, 1.0, 1.1, 1.8, 2400.0, 2600.0), 1.4209300925925925);
    check("wca_base", wep_critical_altitude(7000.0, 1.42, 1.65, 1.0, 1.0), 5_918.386_017_652_959);
    check("wca_rpm11", wep_critical_altitude(7000.0, 1.42, 1.65, 1.1, 1.0), 6_608.678_596_256_166);
    check("wca_pb11", wep_critical_altitude(7000.0, 1.42, 1.65, 1.0, 1.1), 6_608.678_596_256_166);

    // === Part B: P-47D-like 单级 ===
    let mut p47 = CompressorStageParams::default();
    p47.crit_alt = 7000.0;
    p47.crit_power = 2000.0;
    p47.deck_power = 1850.0;
    p47.deck_alt = 0.0;
    p47.curvature = 1.0;
    p47.wep_crit_alt = 6000.0;
    p47.wep_power_mult = 1.15;
    p47.speed_manifold_mult = 0.9;
    p47.old_altitude = 7000.0;
    p47.old_power = 2000.0;
    p47.old_power_new_rpm = 2000.0;
    check("p47_0", power_at_altitude_advanced(&p47, 0.0, false, 0.0, false, 15.0), 1850.0);
    check("p47_5000m", power_at_altitude_advanced(&p47, 5000.0, false, 0.0, false, 15.0), 1_967.744_152_587_75);
    check("p47_5000w", power_at_altitude_advanced(&p47, 5000.0, true, 0.0, false, 15.0), 2278.2116615819164);
    check("p47_7000", power_at_altitude_advanced(&p47, 7000.0, false, 0.0, false, 15.0), 2000.0);
    check("p47_7000w", power_at_altitude_advanced(&p47, 7000.0, true, 0.0, false, 15.0), 2001.6456091340815);
    check("p47_9000", power_at_altitude_advanced(&p47, 9000.0, false, 0.0, false, 15.0), 1497.4132153786454);
    check("p47_10000", power_at_altitude_advanced(&p47, 10000.0, false, 0.0, false, 15.0), 1287.6657433610794);
    check("p47_12000s", power_at_altitude_advanced(&p47, 12000.0, false, 0.0, false, 15.0), 939.280_857_552_052_6);
    check("p47_12000v400", power_at_altitude_advanced(&p47, 12000.0, false, 400.0, true, 15.0), 1270.7683498691983);
    check("p47_12000v500", power_at_altitude_advanced(&p47, 12000.0, false, 500.0, true, 15.0), 1_457.230_064_297_593);
    check("p47_5000v500", power_at_altitude_advanced(&p47, 5000.0, false, 500.0, true, 15.0), 1941.2765375362055);

    // === Part C: 两级机械增压器 ===
    let mut s1 = CompressorStageParams::default();
    s1.crit_alt = 3000.0;
    s1.crit_power = 1400.0;
    s1.deck_power = 1350.0;
    s1.wep_crit_alt = 2500.0;
    s1.wep_power_mult = 1.1;
    s1.stage_index = 0;
    s1.old_altitude = 3000.0;
    s1.old_power = 1400.0;
    s1.old_power_new_rpm = 1400.0;
    let mut s2 = CompressorStageParams::default();
    s2.crit_alt = 6500.0;
    s2.crit_power = 1300.0;
    s2.deck_power = 1100.0;
    s2.wep_crit_alt = 6000.0;
    s2.wep_power_mult = 1.1;
    s2.stage_index = 1;
    s2.old_altitude = 6500.0;
    s2.old_power = 1300.0;
    s2.old_power_new_rpm = 1300.0;
    let two = [s1, s2];
    check("two_1000opt", optimal_power_advanced(&two, 1000.0, false, 0.0, false, 15.0), 1368.3403761790846);
    check("two_1000s1", power_at_altitude_advanced(&s1, 1000.0, false, 0.0, false, 15.0), 1368.3403761790846);
    check("two_1000s2", power_at_altitude_advanced(&s2, 1000.0, false, 0.0, false, 15.0), 1_139.973_474_805_542);
    check("two_6000opt", optimal_power_advanced(&two, 6000.0, false, 0.0, false, 15.0), 1_289.016_680_952_667);
    check("two_6000s1", power_at_altitude_advanced(&s1, 6000.0, false, 0.0, false, 15.0), 942.159_246_548_316);
    check("two_6000s2", power_at_altitude_advanced(&s2, 6000.0, false, 0.0, false, 15.0), 1_289.016_680_952_667);
    check("two_4000w", optimal_power_advanced(&two, 4000.0, true, 0.0, false, 15.0), 1371.2487721441946);
    assert_eq!(find_optimal_stage_index(&two, 1000.0, false, 0.0, false, 15.0), 0);
    assert_eq!(find_optimal_stage_index(&two, 6000.0, false, 0.0, false, 15.0), 1);
    assert_eq!(find_optimal_stage_index(&two, 4000.0, true, 0.0, false, 15.0), 1);

    // === Part D: generatePowerCurveAdvanced ===
    let mut gc = CompressorStageParams::new(5000.0, 1500.0, 1400.0);
    gc.wep_crit_alt = 4500.0;
    gc.wep_power_mult = 1.1;
    gc.old_altitude = 5000.0;
    gc.old_power = 1500.0;
    gc.old_power_new_rpm = 1500.0;
    let gc_arr = [gc];
    let curve = generate_power_curve_advanced(&gc_arr, false, 0.0, false, 15.0, 50);
    assert_eq!(curve.len(), 201); // gc_size
    check("gc_0", curve[0], 1400.0);
    check("gc_100", curve[100], 1500.0);
    check("gc_160", curve[160], 988.518_743_244_433_6);
    check("gc_200", curve[200], 734.069_573_327_452);

    // === Part E: Yak-3 无 WEP (exactAltitudes=true) ===
    let mut y0 = CompressorStageParams::default();
    y0.crit_alt = 300.0;
    y0.crit_power = 1310.0;
    y0.deck_power = 1290.0;
    y0.deck_alt = 0.0;
    y0.curvature = 1.0;
    y0.ceiling_alt = 5000.0;
    y0.ceiling_power = 670.0;
    y0.old_altitude = 300.0;
    y0.old_power = 1310.0;
    y0.old_power_new_rpm = 1310.0;
    y0.exact_altitudes = true;
    y0.stage_index = 0;
    y0.wep_power_mult = 1.0;
    y0.wep_crit_alt = 300.0;
    y0.wep_deck_alt = 0.0;
    y0.speed_manifold_mult = 1.0;
    let mut y1 = CompressorStageParams::default();
    y1.crit_alt = 2600.0;
    y1.crit_power = 1240.0;
    y1.deck_power = 1290.0 * 0.8;
    y1.deck_alt = 0.0;
    y1.curvature = 1.0;
    y1.ceiling_alt = 9000.0;
    y1.ceiling_power = 510.0;
    y1.old_altitude = 2600.0;
    y1.old_power = 1240.0;
    y1.old_power_new_rpm = 1240.0;
    y1.exact_altitudes = true;
    y1.stage_index = 1;
    y1.wep_power_mult = 1.0;
    y1.wep_crit_alt = 2600.0;
    y1.wep_deck_alt = 0.0;
    y1.speed_manifold_mult = 1.0;
    let yak = [y0, y1];
    let yak_m = [
        1290.0, 1_194.470_459_653_116, 1196.6571288157202, 1_178.207_276_690_934,
        1034.4645783240283, 905.115_774_247_377_9, 789.031_271_730_281_6, 685.144_435_518_207_6,
        592.449_584_215_098, 510.0, 436.90595194307707,
    ];
    // Java dump: WEP 曲线与军用逐点相同 (无 WEP 机型)
    for (k, &exp) in yak_m.iter().enumerate() {
        let alt = (k as i32 * 1000) as f64;
        check(&format!("yak_{k}m"), optimal_power_advanced(&yak, alt, false, 0.0, false, 15.0), exp);
        check(&format!("yak_{k}w"), optimal_power_advanced(&yak, alt, true, 0.0, false, 15.0), exp);
    }
    let yakr = [
        1265.8932300069234, 1200.6608991925395, 977.797_114_068_640_6,
        710.139_880_165_562_4, 509.587_291_764_339_5,
    ];
    for (k, &exp) in yakr.iter().enumerate() {
        let alt = (k as i32 * 2500) as f64;
        check(&format!("yakr_{k}m"), optimal_power_advanced(&yak, alt, false, 301.0, true, 15.0), exp);
        check(&format!("yakr_{k}w"), optimal_power_advanced(&yak, alt, true, 301.0, true, 15.0), exp);
    }

    // === Part F1: ConstRPM 低于甲板零功率区 ===
    let mut f1 = CompressorStageParams::default();
    f1.crit_alt = 7000.0;
    f1.crit_power = 2000.0;
    f1.deck_power = 1850.0;
    f1.wep_crit_alt = 6000.0;
    f1.wep_power_mult = 1.15;
    f1.old_altitude = 7000.0;
    f1.old_power = 2000.0;
    f1.old_power_new_rpm = 2000.0;
    f1.const_rpm_alt = 0.0;
    f1.const_rpm_power = 100.0;
    check("f1_m100m", power_at_altitude_advanced(&f1, -100.0, false, 0.0, false, 15.0), 0.0);
    check("f1_m100w", power_at_altitude_advanced(&f1, -100.0, true, 0.0, false, 15.0), 0.0);

    // === Part F2: ConstRPM 弯折低于临界高度 (两段式) ===
    let mut f2 = CompressorStageParams::default();
    f2.crit_alt = 7000.0;
    f2.crit_power = 2000.0;
    f2.deck_power = 1850.0;
    f2.curvature = 1.5;
    f2.wep_crit_alt = 6500.0;
    f2.wep_power_mult = 1.15;
    f2.old_altitude = 7000.0;
    f2.old_power = 2000.0;
    f2.old_power_new_rpm = 2000.0;
    f2.const_rpm_alt = 3000.0;
    f2.const_rpm_power = 1900.0;
    f2.wep_const_rpm_alt = 2800.0;
    f2.stage0_deck_alt = 0.0;
    f2.wep_deck_alt = 0.0;
    check("f2_1500m", power_at_altitude_advanced(&f2, 1500.0, false, 0.0, false, 15.0), 1876.8592257962787);
    check("f2_5000m", power_at_altitude_advanced(&f2, 5000.0, false, 0.0, false, 15.0), 1941.2200836990635);
    check("f2_9000m", power_at_altitude_advanced(&f2, 9000.0, false, 0.0, false, 15.0), 1497.4132153786454);
    f2.exact_altitudes = true;
    check("f2e_1500w", power_at_altitude_advanced(&f2, 1500.0, true, 0.0, false, 15.0), 2158.3881096657205);
    check("f2e_5000w", power_at_altitude_advanced(&f2, 5000.0, true, 0.0, false, 15.0), 2_232.403_096_253_923);
    f2.exact_altitudes = false;
    check("f2n_1500w", power_at_altitude_advanced(&f2, 1500.0, true, 0.0, false, 15.0), 2160.2798813437057);
    check("f2n_5000w", power_at_altitude_advanced(&f2, 5000.0, true, 0.0, false, 15.0), 2_244.127_818_748_27);

    // === Part F3: powerIsDeckPower (critAlt==deckAlt) + ceiling ===
    let mut f3 = CompressorStageParams::default();
    f3.crit_alt = 5000.0;
    f3.crit_power = 2000.0;
    f3.deck_power = 1800.0;
    f3.deck_alt = 5000.0;
    f3.wep_crit_alt = 4500.0;
    f3.wep_power_mult = 1.1;
    f3.old_altitude = 5000.0;
    f3.old_power = 2000.0;
    f3.old_power_new_rpm = 2000.0;
    f3.ceiling_alt = 10000.0;
    f3.ceiling_power = 900.0;
    check("f3_2000m", power_at_altitude_advanced(&f3, 2000.0, false, 0.0, false, 15.0), 3015.9225747678397);
    check("f3_6000m", power_at_altitude_advanced(&f3, 6000.0, false, 0.0, false, 15.0), 1727.2740837849933);
    check("f3_8000w", power_at_altitude_advanced(&f3, 8000.0, true, 0.0, false, 15.0), 1280.6909993159848);
    f3.exact_altitudes = true;
    check("f3e_2000m", power_at_altitude_advanced(&f3, 2000.0, false, 0.0, false, 15.0), 3015.9225747678397);
    check("f3e_6000m", power_at_altitude_advanced(&f3, 6000.0, false, 0.0, false, 15.0), 1727.2740837849933);
    check("f3e_8000w", power_at_altitude_advanced(&f3, 8000.0, true, 0.0, false, 15.0), 1_291.654_865_726_723);

    // === Part F4: Fw-190A-1 式 oldAltitude < altRam <= wepCritAlt ===
    let mut f4 = CompressorStageParams::default();
    f4.crit_alt = 5000.0;
    f4.crit_power = 1800.0;
    f4.deck_power = 1750.0;
    f4.wep_crit_alt = 6500.0;
    f4.wep_power_mult = 1.1;
    f4.old_altitude = 5000.0;
    f4.old_power = 1800.0;
    f4.old_power_new_rpm = 1800.0;
    f4.ceiling_alt = 10000.0;
    f4.ceiling_power = 800.0;
    check("f4_6000w", power_at_altitude_advanced(&f4, 6000.0, true, 0.0, false, 15.0), 1980.0000000000002);
    f4.exact_altitudes = true;
    check("f4e_6000w", power_at_altitude_advanced(&f4, 6000.0, true, 0.0, false, 15.0), 1980.0000000000002);

    // === Part F5: Math.round 分支 (round(wepCritAlt) < altRam <= round(oldAltitude)) ===
    let mut f5 = CompressorStageParams::default();
    f5.crit_alt = 5000.0;
    f5.crit_power = 1800.0;
    f5.deck_power = 1750.0;
    f5.wep_crit_alt = 4400.6;
    f5.wep_power_mult = 1.1;
    f5.old_altitude = 5000.7;
    f5.old_power = 1800.0;
    f5.old_power_new_rpm = 1800.0;
    f5.ceiling_alt = 10000.0;
    f5.ceiling_power = 800.0;
    check("f5_4450w", power_at_altitude_advanced(&f5, 4450.0, true, 0.0, false, 15.0), 1966.0358146615285);
    f5.exact_altitudes = true;
    check("f5e_4450w", power_at_altitude_advanced(&f5, 4450.0, true, 0.0, false, 15.0), 1_961.272_999_158_355);
    // 无 ceiling 变体 (ceilingIsUseful=false)
    let mut f5b = CompressorStageParams::default();
    f5b.crit_alt = 5000.0;
    f5b.crit_power = 1800.0;
    f5b.deck_power = 1750.0;
    f5b.wep_crit_alt = 4400.6;
    f5b.wep_power_mult = 1.1;
    f5b.old_altitude = 5000.7;
    f5b.old_power = 1800.0;
    f5b.old_power_new_rpm = 1800.0;
    check("f5b_4450w", power_at_altitude_advanced(&f5b, 4450.0, true, 0.0, false, 15.0), 1_967.159_626_018_502);
    // constRpmBelowWepCritAlt=true 变体
    let mut b = CompressorStageParams::default();
    b.crit_alt = 5000.0;
    b.crit_power = 1800.0;
    b.deck_power = 1750.0;
    b.wep_crit_alt = 4400.6;
    b.wep_power_mult = 1.1;
    b.old_altitude = 5000.7;
    b.old_power = 1800.0;
    b.old_power_new_rpm = 1800.0;
    b.const_rpm_alt = 3000.0;
    b.const_rpm_power = 1650.0;
    b.curvature = 1.3;
    check("b_4450w", power_at_altitude_advanced(&b, 4450.0, true, 0.0, false, 15.0), 1_967.159_626_018_502);
    b.exact_altitudes = true;
    check("be_4450w", power_at_altitude_advanced(&b, 4450.0, true, 0.0, false, 15.0), 1915.0609690787915);

    // === Part F6: WEP 高于双临界高度 ===
    let mut f6 = CompressorStageParams::default();
    f6.crit_alt = 7000.0;
    f6.crit_power = 2000.0;
    f6.deck_power = 1850.0;
    f6.wep_crit_alt = 4500.0;
    f6.wep_power_mult = 1.15;
    f6.old_altitude = 7000.0;
    f6.old_power = 2000.0;
    f6.old_power_new_rpm = 2000.0;
    check("f6_8000w", power_at_altitude_advanced(&f6, 8000.0, true, 0.0, false, 15.0), 1418.3597215439743);
    f6.ceiling_alt = 11000.0;
    f6.ceiling_power = 700.0;
    check("f6c_8000w", power_at_altitude_advanced(&f6, 8000.0, true, 0.0, false, 15.0), 1291.1852371464227);
    f6.exact_altitudes = true;
    check("f6ce_8000w", power_at_altitude_advanced(&f6, 8000.0, true, 0.0, false, 15.0), 1_023.158_284_819_586);
    let mut f6d = CompressorStageParams::default();
    f6d.crit_alt = 7000.0;
    f6d.crit_power = 2000.0;
    f6d.deck_power = 1850.0;
    f6d.wep_crit_alt = 6500.0;
    f6d.wep_power_mult = 1.15;
    f6d.old_altitude = 7000.0;
    f6d.old_power = 2000.0;
    f6d.old_power_new_rpm = 2000.0;
    check("f6d_8000w", power_at_altitude_advanced(&f6d, 8000.0, true, 0.0, false, 15.0), 1859.4262634513013);
    f6d.exact_altitudes = true;
    check("f6de_8000w", power_at_altitude_advanced(&f6d, 8000.0, true, 0.0, false, 15.0), 1859.4262634513013);
    f6d.exact_altitudes = false;
    f6d.const_rpm_alt = 3000.0;
    f6d.const_rpm_power = 1900.0;
    check("f6dr_8000w", power_at_altitude_advanced(&f6d, 8000.0, true, 0.0, false, 15.0), 1859.4262634513013);

    // wepCritAlt == critAlt, 高于双临界 (无 constRPM → !constRpmBelowCritAlt 分支)
    let mut a = CompressorStageParams::default();
    a.crit_alt = 7000.0;
    a.crit_power = 2000.0;
    a.deck_power = 1850.0;
    a.wep_crit_alt = 7000.0;
    a.wep_power_mult = 1.15;
    a.old_altitude = 7000.0;
    a.old_power = 2000.0;
    a.old_power_new_rpm = 2000.0;
    check("a_8000w", power_at_altitude_advanced(&a, 8000.0, true, 0.0, false, 15.0), 1994.1079618146214);
    a.exact_altitudes = true;
    check("ae_8000w", power_at_altitude_advanced(&a, 8000.0, true, 0.0, false, 15.0), 1994.1079618146214);
    a.exact_altitudes = false;
    a.ceiling_alt = 11000.0;
    a.ceiling_power = 700.0;
    check("ac_8000w", power_at_altitude_advanced(&a, 8000.0, true, 0.0, false, 15.0), 1_825.875_275_812_053);
    a.exact_altitudes = true;
    check("ace_8000w", power_at_altitude_advanced(&a, 8000.0, true, 0.0, false, 15.0), 1856.9897108368868);
    a.exact_altitudes = false;
    a.curvature = 1.5;
    a.const_rpm_alt = 7000.0;
    a.const_rpm_power = 1700.0;
    check("acr_8000w", power_at_altitude_advanced(&a, 8000.0, true, 0.0, false, 15.0), 2041.9054028586359);
    a.exact_altitudes = true;
    check("acre_8000w", power_at_altitude_advanced(&a, 8000.0, true, 0.0, false, 15.0), 2041.9054028586359);

    // === Part F7: constRpmAboveCritAlt (P-63 弯折高于临界) ===
    let mut f7 = CompressorStageParams::default();
    f7.crit_alt = 5000.0;
    f7.crit_power = 1800.0;
    f7.deck_power = 1750.0;
    f7.curvature = 1.5;
    f7.wep_crit_alt = 4500.0;
    f7.wep_power_mult = 1.1;
    f7.old_altitude = 5000.0;
    f7.old_power = 1800.0;
    f7.old_power_new_rpm = 1800.0;
    f7.ceiling_alt = 10000.0;
    f7.ceiling_power = 800.0;
    f7.const_rpm_alt = 5000.0;
    f7.const_rpm_power = 1700.0;
    check("f7_5200m", power_at_altitude_advanced(&f7, 5200.0, false, 0.0, false, 15.0), 1788.2179935134372);
    check("f7_6000m", power_at_altitude_advanced(&f7, 6000.0, false, 0.0, false, 15.0), 1676.5473017913841);
    f7.exact_altitudes = true;
    check("f7e_5200m", power_at_altitude_advanced(&f7, 5200.0, false, 0.0, false, 15.0), 1788.2179935134372);
    check("f7e_6000m", power_at_altitude_advanced(&f7, 6000.0, false, 0.0, false, 15.0), 1676.5473017913841);

    // === Part F8: 无 ceiling 高空衰减 (military) ===
    let mut f8 = CompressorStageParams::default();
    f8.crit_alt = 5000.0;
    f8.crit_power = 1500.0;
    f8.deck_power = 1400.0;
    f8.wep_crit_alt = 4500.0;
    f8.wep_power_mult = 1.1;
    f8.old_altitude = 5000.0;
    f8.old_power = 1500.0;
    f8.old_power_new_rpm = 1500.0;
    check("f8_9000m", power_at_altitude_advanced(&f8, 9000.0, false, 0.0, false, 15.0), 853.641_937_572_037_8);
    check("f8_5200m", power_at_altitude_advanced(&f8, 5200.0, false, 0.0, false, 15.0), 1_460.341_572_137_396);

    // === RAM 使 effectiveAlt 落入 WEP 低于临界分支 ===
    let mut c = CompressorStageParams::default();
    c.crit_alt = 7000.0;
    c.crit_power = 2000.0;
    c.deck_power = 1850.0;
    c.wep_crit_alt = 6000.0;
    c.wep_power_mult = 1.15;
    c.speed_manifold_mult = 0.9;
    c.old_altitude = 7000.0;
    c.old_power = 2000.0;
    c.old_power_new_rpm = 2000.0;
    check("c_6500w_tas600", power_at_altitude_advanced(&c, 6500.0, true, 600.0, false, 15.0), 2_285.179_163_632_901);
    check("c_6500w_ias600", power_at_altitude_advanced(&c, 6500.0, true, 600.0, true, 15.0), 2261.2387414765312);
    check("c_6500w_ias600_t30", power_at_altitude_advanced(&c, 6500.0, true, 600.0, true, 30.0), 2261.2387414765312);

    // === Part G: peakWepPower ===
    let mut pk = CompressorStageParams::default();
    pk.crit_alt = 5000.0;
    pk.crit_power = 1500.0;
    pk.deck_power = 1400.0;
    pk.wep_crit_alt = 4500.0;
    pk.wep_power_mult = 1.15;
    pk.old_altitude = 5000.0;
    pk.old_power = 1500.0;
    pk.old_power_new_rpm = 1500.0;
    let pk_arr = [pk];
    check("pk_single", peak_wep_power(&pk_arr), 1724.9999999999998);
    check("pk_multi", peak_wep_power(&two), 1540.0000000000002);
    check("pk_empty", peak_wep_power(&[]), 0.0);
    let mut nw = CompressorStageParams::default();
    nw.crit_alt = 3000.0;
    nw.crit_power = 1200.0;
    nw.deck_power = 1150.0;
    nw.wep_crit_alt = 3000.0;
    nw.wep_power_mult = 1.0;
    nw.old_altitude = 3000.0;
    nw.old_power = 1200.0;
    nw.old_power_new_rpm = 1200.0;
    let nw_arr = [nw];
    check("pk_nowep", peak_wep_power(&nw_arr), 1200.0);

    // toString 对拍 (Java String.format %.0f/%.2f 输出)
    assert_eq!(
        pk.to_string(),
        "Stage0[critAlt=5000m, critPower=1500hp, deckPower=1400hp, wepMult=1.15]"
    );
    assert_eq!(
        s2.to_string(),
        "Stage1[critAlt=6500m, critPower=1300hp, deckPower=1100hp, wepMult=1.10]"
    );
}

#[test]
fn run_test_torque_rpm_boost() {
    test_torque_rpm_boost();
}

#[test]
fn run_test_torque_from_hp() {
    test_torque_from_hp();
}

#[test]
fn run_test_supercharger_rpm_effect() {
    test_supercharger_rpm_effect();
}

#[test]
fn run_test_interpolate_power() {
    test_interpolate_power();
}

#[test]
fn run_test_wep_power_multiplier() {
    test_wep_power_multiplier();
}

#[test]
fn run_test_wep_critical_altitude() {
    test_wep_critical_altitude();
}

#[test]
fn run_test_power_at_altitude() {
    test_power_at_altitude();
}

#[test]
fn run_test_multi_stage_supercharger() {
    test_multi_stage_supercharger();
}

#[test]
fn run_test_generate_power_curve() {
    test_generate_power_curve();
}

#[test]
fn run_test_ram_effect_integration() {
    test_ram_effect_integration();
}

#[test]
fn run_test_no_wep_aircraft_identical_curves() {
    test_no_wep_aircraft_identical_curves();
}

#[test]
fn run_test_peak_wep_power() {
    test_peak_wep_power();
}
