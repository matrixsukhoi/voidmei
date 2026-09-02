use super::*;

/// Tests for AtmosphereModel.
///
/// Validates the ISA standard atmosphere calculations against known values.
///
/// Run with: ./script/test.sh
///
/// PORT: Java 断言助手 assertClose/assertTrue (计数式 pass/fail) → Rust
/// assert! 宏 (失败即 panic); `Math.abs(a-e) <= tol` 判定式逐字保留
fn assert_close(name: &str, actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "FAIL: {name} = {actual:.4} (expected {expected:.4}, tolerance {tolerance:.4})"
    );
}

fn assert_true(name: &str, condition: bool) {
    assert!(condition, "FAIL: {name}");
}

fn test_pressure() {
    // Testing pressure()...

    // Sea level
    assert_close("pressure(0m)", pressure(0.0), 1.0, 0.001);

    // Standard ISA values
    assert_close("pressure(5000m)", pressure(5000.0), 0.5334, 0.01);
    assert_close("pressure(10000m)", pressure(10000.0), 0.2615, 0.01);
    assert_close("pressure(11000m)", pressure(11000.0), 0.2240, 0.01);

    // Below sea level (higher pressure)
    assert_true("pressure(-1000m) > 1.0", pressure(-1000.0) > 1.0);
    assert_close("pressure(-1000m)", pressure(-1000.0), 1.127, 0.01);
}

fn test_altitude_at_pressure() {
    // Testing altitudeAtPressure()...

    // Round-trip tests (using actual ISA formula values)
    assert_close("altitudeAtPressure(1.0)", altitude_at_pressure(1.0), 0.0, 1.0);
    assert_close("altitudeAtPressure(0.5)", altitude_at_pressure(0.5), 5477.0, 50.0); // ISA formula result
    assert_close("altitudeAtPressure(0.25)", altitude_at_pressure(0.25), 10278.0, 50.0); // ISA formula result

    // Inverse function property
    // PORT: Java `for (int alt = 0; alt <= 15000; alt += 1000)` int 循环
    for alt in (0..=15000i32).step_by(1000) {
        let alt_f = alt as f64;
        let p = pressure(alt_f);
        let recovered = altitude_at_pressure(p);
        assert_close(&format!("round-trip at {alt}m"), recovered, alt_f, 1.0);
    }
}

fn test_density() {
    // Testing density()...

    // Sea level ISA: 1.225 kg/m³
    let rho0 = density(1.0, 15.0, 0.0);
    assert_close("density at sea level", rho0, 1.225, 0.001);

    // At 5000m
    let p5000 = pressure(5000.0);
    let rho5000 = density(p5000, 15.0, 5000.0);
    assert_close("density at 5000m", rho5000, 0.736, 0.01);

    // Density decreases with altitude
    assert_true("density decreases with altitude", rho5000 < rho0);

    // Temperature effect: higher temp = lower density
    let rho_hot = density(1.0, 30.0, 0.0);
    let rho_cold = density(1.0, 0.0, 0.0);
    assert_true("hot air less dense", rho_hot < rho0);
    assert_true("cold air more dense", rho_cold > rho0);
}

fn test_ias_tas_conversion() {
    // Testing IAS/TAS conversion...

    // At sea level, IAS = TAS
    let rho0 = 1.225;
    assert_close("TAS at sea level", ias_to_tas(400.0, rho0), 400.0, 0.1);
    assert_close("IAS at sea level", tas_to_ias(400.0, rho0), 400.0, 0.1);

    // At altitude, TAS > IAS
    let rho5000 = density(pressure(5000.0), 15.0, 5000.0);
    let tas = ias_to_tas(400.0, rho5000);
    assert_true("TAS > IAS at altitude", tas > 400.0);
    assert_close("TAS at 5000m", tas, 516.0, 5.0);

    // Round-trip
    let ias_back = tas_to_ias(tas, rho5000);
    assert_close("IAS round-trip", ias_back, 400.0, 0.1);
}

fn test_ram_effect() {
    // Testing ramEffectAltitude()...

    // Zero speed: no effect
    let no_ram = ram_effect_altitude(5000.0, 15.0, 0.0, true, 1.0);
    assert_close("no RAM at zero speed", no_ram, 5000.0, 1.0);

    // With speed: effective altitude is lower
    let with_ram = ram_effect_altitude(5000.0, 15.0, 500.0, true, 1.0);
    assert_true("RAM lowers effective altitude", with_ram < 5000.0);

    // Higher speed = more RAM effect
    let more_ram = ram_effect_altitude(5000.0, 15.0, 600.0, true, 1.0);
    assert_true("more speed = more RAM", more_ram < with_ram);

    // SpeedManifoldMult affects magnitude
    let less_efficient = ram_effect_altitude(5000.0, 15.0, 500.0, true, 0.5);
    assert_true("lower mult = less RAM", less_efficient > with_ram);

    // Typical values check
    // PORT: Java 此处另有 printf 打印 typical 值 (信息输出, 非断言), 测试移植不保留
    let typical = ram_effect_altitude(5000.0, 15.0, 500.0, true, 0.9);
    assert_true(
        "typical RAM ~1000-1500m reduction",
        typical > 3500.0 && typical < 4500.0,
    );
}

fn test_temperature() {
    // Testing temperatureAtAltitude()...

    // ISA sea level = 15°C
    assert_close(
        "temp at sea level",
        temperature_at_altitude(15.0, 0.0),
        15.0,
        0.01,
    );

    // Lapse rate: -6.5°C per 1000m
    assert_close(
        "temp at 1000m",
        temperature_at_altitude(15.0, 1000.0),
        8.5,
        0.1,
    );
    assert_close(
        "temp at 5000m",
        temperature_at_altitude(15.0, 5000.0),
        -17.5,
        0.1,
    );
    assert_close(
        "temp at 10000m",
        temperature_at_altitude(15.0, 10000.0),
        -50.0,
        0.1,
    );
}

#[test]
fn run_test_pressure() {
    test_pressure();
}

#[test]
fn run_test_altitude_at_pressure() {
    test_altitude_at_pressure();
}

#[test]
fn run_test_density() {
    test_density();
}

#[test]
fn run_test_ias_tas_conversion() {
    test_ias_tas_conversion();
}

#[test]
fn run_test_ram_effect() {
    test_ram_effect();
}

#[test]
fn run_test_temperature() {
    test_temperature();
}

/// Java 8 oracle 对拍 (PORTING.md §5.1 A 类策略):
/// 期望值 = build/oracle/AtmosphereOracle.java 在 OpenJDK 1.8.0_342 上
/// dump 的 %.17g 实测值 (用完已删除)。容差取混合式 1e-12·max(|expected|,1)
/// (|expected|<1 时退化为绝对容差): Math.pow 跨 libm 实现允许最后几位
/// ULP 差异, 远小于业务断言容差。
#[test]
fn java8_oracle_parity() {
    let tol = 1e-12;
    let check = |name: &str, actual: f64, expected: f64| {
        // 混合容差: expected 为 0 (如 altitudeAtPressure(1.0) 精确 0) 时退化为绝对容差
        let diff = (actual - expected).abs();
        assert!(
            diff <= tol * expected.abs().max(1.0),
            "oracle mismatch {name}: rust={actual:?} java={expected:?}"
        );
    };

    // pressure
    check("pressure(0)", pressure(0.0), 1.0);
    check("pressure(5000)", pressure(5000.0), 0.533_134_764_455_741_2);
    check("pressure(10000)", pressure(10000.0), 0.26090533938003835);
    check("pressure(11000)", pressure(11000.0), 0.22336078269487092);
    check("pressure(-1000)", pressure(-1000.0), 1.1243927510716922);
    check("pressure(-4000)", pressure(-4000.0), 1.574_680_522_330_881);
    check("pressure(20000)", pressure(20000.0), 0.042_715_265_672_239_31);
    check("pressure(12345.67)", pressure(12345.67), 0.17986185984783923);

    // altitudeAtPressure
    check("altitudeAtPressure(1.0)", altitude_at_pressure(1.0), 0.0);
    check(
        "altitudeAtPressure(0.5)",
        altitude_at_pressure(0.5),
        5_477.248_369_190_376,
    );
    check(
        "altitudeAtPressure(0.25)",
        altitude_at_pressure(0.25),
        10277.760105772719,
    );
    check(
        "altitudeAtPressure(0.224)",
        altitude_at_pressure(0.224),
        10981.872464581016,
    );
    check(
        "altitudeAtPressure(1.127)",
        altitude_at_pressure(1.127),
        -1019.9804232650228,
    );
    check(
        "altitudeAtPressure(0.05)",
        altitude_at_pressure(0.05),
        19_260.018_609_774_33,
    );
    check(
        "altitudeAtPressure(0.0)",
        altitude_at_pressure(0.0),
        20000.0,
    );
    check(
        "altitudeAtPressure(-0.5)",
        altitude_at_pressure(-0.5),
        20000.0,
    );
    check(
        "altitudeAtPressure(1.2)",
        altitude_at_pressure(1.2),
        -1_564.775_984_669_923,
    );

    // density
    check("density(1,15,0)", density(1.0, 15.0, 0.0), 1.2250119775015476);
    check(
        "density(p5000,15,5000)",
        density(0.533_134_764_455_741_2, 15.0, 5000.0),
        0.736_122_622_452_837_3,
    );
    check("density(1,30,0)", density(1.0, 30.0, 0.0), 1.1643978272045883);
    check("density(1,0,0)", density(1.0, 0.0, 0.0), 1.2922833656125605);
    check(
        "density(0.5,20,7000)",
        density(0.5, 20.0, 7000.0),
        0.712_673_533_852_354,
    );
    check(
        "density(1,15,5000)",
        density(1.0, 15.0, 5000.0),
        1.3807439910700994,
    );

    // iasToTas
    check("iasToTas(400,1.225)", ias_to_tas(400.0, 1.225), 400.0);
    check(
        "iasToTas(400,0.736)",
        ias_to_tas(400.0, 0.736),
        516.046_846_542_14,
    );
    check("iasToTas(400,0)", ias_to_tas(400.0, 0.0), 400.0);
    check("iasToTas(400,-1)", ias_to_tas(400.0, -1.0), 400.0);
    check(
        "iasToTas(300,0.5)",
        ias_to_tas(300.0, 0.5),
        469.574_275_274_955_8,
    );
    let rho5000 = density(pressure(5000.0), 15.0, 5000.0);
    check(
        "iasToTas(400,rho5000)",
        ias_to_tas(400.0, rho5000),
        516.003_863_509_406,
    );

    // tasToIas
    check(
        "tasToIas(516,0.736)",
        tas_to_ias(516.0, 0.736),
        399.963_688_147_730_1,
    );
    check("tasToIas(400,1.225)", tas_to_ias(400.0, 1.225), 400.0);
    check("tasToIas(400,0)", tas_to_ias(400.0, 0.0), 400.0);
    check(
        "tasToIas(516.1234,rho5000)",
        tas_to_ias(516.1234, rho5000),
        400.09266325238804,
    );

    // ramEffectAltitude
    check(
        "ram(5000,15,0,true,1.0)",
        ram_effect_altitude(5000.0, 15.0, 0.0, true, 1.0),
        5000.0,
    );
    check(
        "ram(5000,15,500,true,1.0)",
        ram_effect_altitude(5000.0, 15.0, 500.0, true, 1.0),
        3491.6138502207914,
    );
    check(
        "ram(5000,15,600,true,1.0)",
        ram_effect_altitude(5000.0, 15.0, 600.0, true, 1.0),
        2896.7696982911634,
    );
    check(
        "ram(5000,15,500,true,0.5)",
        ram_effect_altitude(5000.0, 15.0, 500.0, true, 0.5),
        4_215.653_131_351_711,
    );
    check(
        "ram(5000,15,500,true,0.9)",
        ram_effect_altitude(5000.0, 15.0, 500.0, true, 0.9),
        3_632.087_054_173_318,
    );
    check(
        "ram(5000,15,500,false,0.9)",
        ram_effect_altitude(5000.0, 15.0, 500.0, false, 0.9),
        4_154.419_392_579_673,
    );
    check(
        "ram(3000,10,700,false,0.85)",
        ram_effect_altitude(3000.0, 10.0, 700.0, false, 0.85),
        1_457.866_553_604,
    );
    check(
        "ram(5000,15,500,true,0.0)",
        ram_effect_altitude(5000.0, 15.0, 500.0, true, 0.0),
        5000.0,
    );
    check(
        "ram(5000,15,-100,true,1.0)",
        ram_effect_altitude(5000.0, 15.0, -100.0, true, 1.0),
        5000.0,
    );
    check(
        "ram(8000,-30,650,true,0.95)",
        ram_effect_altitude(8000.0, -30.0, 650.0, true, 0.95),
        4_924.232_345_340_428,
    );

    // temperatureAtAltitude
    check(
        "tempAtAlt(15,0)",
        temperature_at_altitude(15.0, 0.0),
        15.0,
    );
    check(
        "tempAtAlt(15,1000)",
        temperature_at_altitude(15.0, 1000.0),
        8.5,
    );
    check(
        "tempAtAlt(15,5000)",
        temperature_at_altitude(15.0, 5000.0),
        -17.5,
    );
    check(
        "tempAtAlt(15,10000)",
        temperature_at_altitude(15.0, 10000.0),
        -50.0,
    );
    check(
        "tempAtAlt(20,1234.5)",
        temperature_at_altitude(20.0, 1234.5),
        11.975_75,
    );
    check(
        "tempAtAlt(-40,11000)",
        temperature_at_altitude(-40.0, 11000.0),
        -111.5,
    );

    // densityAtAltitude
    check(
        "densityAtAltitude(0)",
        density_at_altitude(0.0),
        1.2250119775015476,
    );
    check(
        "densityAtAltitude(5000)",
        density_at_altitude(5000.0),
        0.736_122_622_452_837_3,
    );
    check(
        "densityAtAltitude(11000)",
        density_at_altitude(11000.0),
        0.363_921_059_623_6,
    );
    check(
        "densityAtAltitude(20000)",
        density_at_altitude(20000.0),
        0.095_339_501_000_056_31,
    );
    check(
        "densityAtAltitude(1234.5)",
        density_at_altitude(1234.5),
        1.0862741290083824,
    );
}
