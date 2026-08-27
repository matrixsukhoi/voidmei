use super::*;

fn p51d_snapshot() -> (StateRaw, IndicatorsRaw) {
    let raw = std::fs::read("../../../script/mock_scenarios/snapshots/plane_p51d.json").unwrap();
    let v: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let st = super::super::json::parse_state(
        v["/state"].to_string().as_bytes()).unwrap();
    let ind = super::super::json::parse_indicators(
        v["/indicators"].to_string().as_bytes()).unwrap();
    (st, ind)
}

/// 喂 p51d 快照: 直通字段 + 稳态派生量 (Δv=0 时 SEP→nVy, acceleration→0)
#[test]
fn p51d_derive_values() {
    let (st, ind) = p51d_snapshot();
    let mut d = Deriver::new(50);
    // 跑 200 轮让 SMA 收敛 (同数据喂入, 平均值收敛到常数)
    let mut out = FlightValues::default();
    for _ in 0..200 {
        out = d.step(&st, &ind, 50.0);
    }

    // 直通字段 (快照值)
    assert_eq!(out.ias, 474.0);
    assert!((out.vario - (-7.342558)).abs() < 1e-4);
    assert!((out.compass - 164.09729).abs() < 1e-4);
    assert!((out.aoa - st.aoa).abs() < 1e-9);
    assert_eq!(out.roll_rate, st.wx.abs());
    // 哨兵字段
    assert_eq!(out.radio_altitude, st.height_m);
    assert_eq!(out.wing_sweep, 0.0);

    // mach 手算对照 (Java L1214 同公式独立计算)
    let ias_per_mach = 3.6 * (1.4 / 1.225 * 101325.0
        * (1.0 - 0.0000225577 * st.height_m).powf(5.25588)).sqrt();
    assert!((out.mach - 474.0 / ias_per_mach).abs() < 1e-12);

    // 稳态: speedv 恒定 → Δv=0 → acceleration→0, SEP→nVy
    assert!(out.acceleration.abs() < 1e-6);
    assert!((out.sep - out.vario).abs() < 1e-6);

    // Ny = An/g, An = g*sqrt(Ny²+1-2Ny·cos(roll)·cos(pitch+AoA))
    let an_expect = G * (st.ny * st.ny + 1.0
        - 2.0 * st.ny * ind.aviahorizon_roll.to_radians().cos()
        * (ind.aviahorizon_pitch + st.aoa).to_radians().cos()).sqrt();
    assert!((out.ny - an_expect / G).abs() < 1e-9);
}

/// SMA 语义测试 (对应 CalcHelper.SimpleMovingAverage)
#[test]
fn sma_semantics() {
    // n=3: 预热段全量平均
    let mut s = Sma::new(3);
    assert_eq!(s.add_new_data(1.0), 1.0);
    assert_eq!(s.add_new_data(2.0), 1.5);
    assert_eq!(s.add_new_data(3.0), 2.0);
    // 环形段: avg += (new - oldest)/n → 覆盖 1.0 → (2+3+5)/3
    assert!((s.add_new_data(5.0) - (2.0 + (5.0 - 1.0) / 3.0)).abs() < 1e-12);
}
