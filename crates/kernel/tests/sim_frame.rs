//! 试驾场模拟帧黑盒场景 (kernel::derived::sim_frame): 覆盖表快照 /
//! 场景分型 (喷气/起落架/无数据) / 时间流动性。纯时间函数 — 零窗口零字体。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


use expect_test::expect;

use kernel::derived::sim_frame::{SimFrame, SimScenario};
use kernel::formula::registry::FormulaView;

/// 固定时刻取值表 (normal 场景): 出厂列表页 target 集 + 引擎/襟翼量
#[test]
fn sim_frame_normal_覆盖表() {
    let f = SimFrame::new(SimScenario::Normal, 60_000); // t=60s
    let names = [
        "ias", "tas", "mach", "compass", "altitude", "vario", "ny", "aoa", "aos",
        "roll_rate", "turn_rate", "turn_rds", "radio_altitude",
        "horse_power", "thrust", "rpm", "mass_fuel", "total_weight",
        "water_temp", "oil_temp", "throttle", "gear", "flaps", "is_imperial",
    ];
    let actual = names
        .iter()
        .map(|n| {
            format!(
                "{n} = {}",
                match f.var_value(n) {
                    Some(v) => format!("{v:.3}"),
                    None => "None".to_string(),
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        ias = 530.139
        tas = 573.649
        mach = 0.417
        compass = 274.234
        altitude = 2751.175
        vario = -8.049
        ny = 1.548
        aoa = 10.601
        aos = 1.301
        roll_rate = -54.335
        turn_rate = -6.528
        turn_rds = 893.800
        radio_altitude = 2600.940
        horse_power = 88.742
        thrust = 1856.123
        rpm = 2756.123
        mass_fuel = 876.000
        total_weight = 6976.000
        water_temp = 90.912
        oil_temp = 94.897
        throttle = 0.737
        gear = 0.000
        flaps = 1.000
        is_imperial = 0.000"#]]
    .assert_eq(&actual);
}

/// 场景分型: 喷气 (螺旋桨族 None) / 起落架 (gear 置位) / 无数据 (全 None)
#[test]
fn sim_frame_场景分型() {
    let t = 10_000i64;
    let jet = SimFrame::new(SimScenario::Jet, t);
    let gear = SimFrame::new(SimScenario::GearDown, t);
    let nodata = SimFrame::new(SimScenario::NoData, t);
    let show = |v: Option<f64>| v.map(|x| format!("{x:.1}")).unwrap_or("None".into());
    expect![[r#"
        jet: rpm=None prop_pitch=None thrust=8358.5 mach=0.5
        gear: gear=1.0 ias=539.6
        nodata: ias=None gear=None"#]]
    .assert_eq(&format!(
        "jet: rpm={} prop_pitch={} thrust={} mach={}\ngear: gear={} ias={}\nnodata: ias={} gear={}",
        show(jet.var_value("rpm")),
        show(jet.var_value("prop_pitch")),
        show(jet.var_value("thrust")),
        show(jet.var_value("mach")),
        show(gear.var_value("gear")),
        show(gear.var_value("ias")),
        show(nodata.var_value("ias")),
        show(nodata.var_value("gear")),
    ));

    // 拨杆位解析: 未知名宽容退 Normal
    expect![[r#"
        jet -> Jet
        gear -> GearDown
        nodata -> NoData
        weird -> Normal"#]]
    .assert_eq(
        &["jet", "gear", "nodata", "weird"]
            .iter()
            .map(|n| format!("{n} -> {:?}", SimScenario::parse(n)))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// 时间流动性: 两个时刻的值不同 (数据在流不在停 — 试驾场核心性质)
#[test]
fn sim_frame_时间流动() {
    let a = SimFrame::new(SimScenario::Normal, 60_000);
    let b = SimFrame::new(SimScenario::Normal, 61_000);
    let va = a.var_value("ias");
    let vb = b.var_value("ias");
    let (Some(va), Some(vb)) = (va, vb) else {
        panic!("ias 应有值");
    };
    assert!((va - vb).abs() > 0.1, "1s 间隔 ias 应有可感变化: {va} vs {vb}");
    // 油量单调缓降 (30 分钟见底语义)
    let f0 = SimFrame::new(SimScenario::Normal, 0).var_value("mass_fuel").unwrap();
    let f1 = SimFrame::new(SimScenario::Normal, 60_000).var_value("mass_fuel").unwrap();
    assert!(f1 < f0, "油量应随时间下降: {f0} -> {f1}");
}
