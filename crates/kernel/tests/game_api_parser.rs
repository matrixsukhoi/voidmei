//! 8111 遥测解析黑盒场景 (kernel::game_api::parser)。
//! 输入 = 真机快照 (script/mock_scenarios, mock_8111 同源) + 合成边界串;
//! 观测 = State/Indicators 解析后的公开字段。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


use expect_test::expect;
use kernel::game_api::parser::{Indicators, State};

/// 真机 p51d 快照的端点原文 (mock_scenarios 同源, 防双份漂移)
fn snapshot_body(_name: &str, endpoint: &str) -> String {
    let raw = include_str!("../../../script/mock_scenarios/snapshots/plane_p51d.json");
    let v: serde_json::Value = serde_json::from_str(raw).expect("快照非 JSON");
    v[endpoint].to_string()
}

/// 真机快照全量解析: 关键飞行数据与快照逐值对齐
#[test]
fn state_真机快照解析() {
    let mut s = State::new();
    let con = s.update(&snapshot_body("plane_p51d", "/state"));
    assert_eq!(con, 0, "valid 在场 → conState 0");
    let actual = format!(
        "valid={:?} flag={} ias={} tas={} heightm={:.1} m={:.1}\n\
         throttle={} rpm={} watertemp={:.1} oiltemp={:.1}\n\
         mfuel={:.1} total_thr={:.1} engine_num={}",
        s.valid, s.flag, s.ias, s.tas, s.heightm, s.m, s.throttle, s.rpm, s.watertemp, s.oiltemp, s.mfuel, s.total_thr, s.engine_num,
    );
    expect![[r#"
        valid=Some(true) flag=true ias=474 tas=454 heightm=46.0 m=0.4
        throttle=110 rpm=3001 watertemp=121.0 oiltemp=90.0
        mfuel=197.0 total_thr=840.0 engine_num=1"#]]
    .assert_eq(&actual);
}

/// 缺 valid 键 → conState -1 且**早退不解析其余键** (连接信号丢失 = 全帧丢弃)
#[test]
fn state_缺valid键_返回负1不解析() {
    let mut s = State::new();
    let con = s.update(r#"{"IAS, km/h": 100}"#);
    assert_eq!(con, -1, "缺 valid = 连接信号丢失");
    assert_eq!(s.ias, 0, "早退: 其余键不解析");
    assert_eq!(s.valid, None);
}

/// 畸形 JSON → 同缺 valid 路径: -1 + flag false (不 panic)
#[test]
fn state_畸形json_flag翻false() {
    let mut s = State::new();
    let con = s.update("{\"broken");
    assert_eq!(con, -1, "畸形 JSON 归入缺 valid 协议信号");
    assert!(!s.flag, "畸形输入后 flag 应为 false");
}

/// 未知键忽略 + 缺数值键回 I_INVALID 哨兵 (-65535, 缺失 ≠ 0)
#[test]
fn state_未知键与缺键哨兵() {
    let mut s = State::new();
    s.update(r#"{"valid": true, "unknown_key": 1}"#);
    assert_eq!(s.valid, Some(true));
    assert_eq!(s.ias, -65535, "缺 IAS 键 → I_INVALID 哨兵");
}

/// indicators: type 大写化 + 空对象/缺 valid → "No Cockpit" 观战形态
#[test]
fn indicators_type大写化与缺键() {
    let mut i = Indicators::new();
    i.update(&snapshot_body("plane_p51d", "/indicators"));
    let actual = format!(
        "type={:?} valid={:?}",
        i.r#type, i.valid,
    );
    expect![[r#"
        type=Some("P-51D-20_CHINA") valid=Some(true)"#]]
    .assert_eq(&actual);

    let mut j = Indicators::new();
    j.update(r#"{}"#);
    assert_eq!(j.r#type.as_deref(), Some("No Cockpit"), "缺 valid → 观战形态");
}

/// indicators 畸形输入: 同缺 valid 路径 → No Cockpit, 不 panic
#[test]
fn indicators_畸形输入不panic() {
    let mut i = Indicators::new();
    i.update("not json at all");
    assert_eq!(i.r#type.as_deref(), Some("No Cockpit"));
    assert!(!i.flag);
}
