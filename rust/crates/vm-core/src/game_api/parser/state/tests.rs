use super::*;

/// 真机抓取的 /state 快照 (script/mock_scenarios/snapshots/plane_bf109f4.json,
/// mock 8111 线上格式: 冒号后一空格, 逗号后无空格)。
/// 波20 serde 化: 断言从 f32 单精度拓宽位级值改为 f64 直读值。
const STATE_MOCK: &str = "{\"valid\": true,\"aileron, %\": -48,\"elevator, %\": 20,\"rudder, %\": -47,\"flaps, %\": 0,\"gear, %\": 0,\"H, m\": 46,\"TAS, km/h\": 454,\"IAS, km/h\": 474,\"M\": 0.39,\"AoA, deg\": -1.6,\"AoS, deg\": -5.9,\"Ny\": 0.35,\"Vy, m/s\": -7.3,\"Wx, deg/s\": -34,\"Mfuel, kg\": 197,\"Mfuel0, kg\": 734,\"throttle 1, %\": 110,\"RPM throttle 1, %\": 100,\"mixture 1, %\": 100,\"radiator 1, %\": 42,\"magneto 1\": 3,\"power 1, hp\": 1597.8,\"RPM 1\": 3001,\"manifold pressure 1, atm\": 2.24,\"water temp 1, C\": 121,\"oil temp 1, C\": 90,\"pitch 1, deg\": 35.5,\"thrust 1, kgs\": 840,\"efficiency 1, %\": 87}";

#[test]
fn update_full_snapshot() {
    let mut st = State::new();
    st.init();
    assert_eq!(st.update(STATE_MOCK), 0);
    assert_eq!(st.valid, Some(true));
    assert!(st.flag);
    assert_eq!(st.engine_num, 1);
    assert_eq!(st.aileron, -48);
    assert_eq!(st.elevator, 20);
    assert_eq!(st.rudder, -47);
    assert_eq!(st.flaps, 0);
    assert_eq!(st.gear, 0);
    assert_eq!(st.airbrake, -65535); // 快照无 airbrake 键 → 哨兵
    assert_eq!(st.tas, 454);
    assert_eq!(st.ias, 474);
    assert_eq!(st.m, 0.39);
    assert_eq!(st.aoa, -1.6);
    assert_eq!(st.aos, -5.9);
    assert_eq!(st.ny, 0.35);
    assert_eq!(st.vy, -7.3);
    assert_eq!(st.wx, -34.0);
    assert_eq!(st.heightm, 46.0);
    assert_eq!(st.throttle, 110);
    assert_eq!(st.rpm_throttle, 100);
    assert_eq!(st.radiator, 42);
    assert_eq!(st.mixture, 100);
    assert_eq!(st.compressorstage, 0); // 缺键哨兵归一化
    assert_eq!(st.magneto, 3);
    assert_eq!(st.rpm, 3001);
    assert_eq!(st.manifoldpressure, 2.24);
    assert_eq!(st.watertemp, 121.0);
    assert_eq!(st.oiltemp, 90.0);
    assert_eq!(st.mfuel, 197.0);
    assert_eq!(st.mfuel_1, -65535.0);
    assert_eq!(st.mfuel0, 734.0);
    assert_eq!(st.mfuel0_1, -65535.0);
    assert_eq!(st.total_thr, 840.0);
    // 引擎数组: i=0 有效; i=1 已赋值哨兵后 break; i>=2 保持 init 的 0
    assert_eq!(st.throttles[0], 110);
    assert_eq!(st.power[0], 1597.8);
    assert_eq!(st.thrust[0], 840);
    assert_eq!(st.pitch[0], 35.5);
    assert_eq!(st.efficiency[0], 87.0);
    assert_eq!(st.throttles[1], -65535);
    assert_eq!(st.power[1], -65535.0);
    assert_eq!(st.thrust[1], -65535);
    assert_eq!(st.pitch[1], -65535.0);
    assert_eq!(st.efficiency[1], -65535.0);
    assert_eq!(st.throttles[2], 0);
    assert_eq!(st.thrust[15], 0);
}

#[test]
fn update_missing_valid_returns_minus_1() {
    let mut st = State::new();
    st.init();
    assert_eq!(st.update("{\"IAS, km/h\": 100}"), -1);
    assert!(st.valid.is_none());
    assert!(!st.flag);
    assert_eq!(st.ias, 0);
}

#[test]
fn update_malformed_json_returns_minus_1() {
    // 波20 serde 化: 畸形 JSON 等价 "缺 valid 键" → 端口翻转信号
    let mut st = State::new();
    st.init();
    assert_eq!(st.update("{\"valid\": tru"), -1);
    assert_eq!(st.update(""), -1);
    assert!(st.valid.is_none());
}

#[test]
fn update_valid_false_keeps_fields() {
    let mut st = State::new();
    st.init();
    assert_eq!(st.update("{\"valid\": false}"), 0);
    assert_eq!(st.valid, Some(false));
    assert!(!st.flag);
    assert_eq!(st.engine_num, 0);
}

#[test]
fn update_sentinel_normalizations() {
    // P-63 类自动桨: RPM throttle/mixture/compressorstage 缺键时哨兵归一化;
    // magneto 不归一化, 保留 -65535 (真键 "magneto 1")
    let mut st = State::new();
    st.init();
    st.update("{\"valid\": true, \"magneto 1\": 1}");
    assert_eq!(st.rpm_throttle, -1);
    assert_eq!(st.mixture, -1);
    assert_eq!(st.compressorstage, 0);
    assert_eq!(st.magneto, 1);
    // 无任何 thrust N 键 → 引擎循环第一轮即 break, engineNum=0
    assert_eq!(st.engine_num, 0);
}

#[test]
fn update_multi_engine() {
    let mut st = State::new();
    st.init();
    st.update("{\"valid\": true, \"throttle 1, %\": 90, \"throttle 2, %\": 95, \"power 1, hp\": 1000.5, \"power 2, hp\": 1100.25, \"thrust 1, kgs\": 500, \"thrust 2, kgs\": 600, \"pitch 1, deg\": 30.5, \"pitch 2, deg\": 31.5, \"efficiency 1, %\": 80, \"efficiency 2, %\": 81}");
    assert_eq!(st.engine_num, 2);
    assert_eq!(st.total_thr, 1100.0);
    assert_eq!(st.throttles[1], 95);
    assert_eq!(st.power[1], 1100.25);
    assert_eq!(st.thrust[1], 600);
    assert_eq!(st.pitch[1], 31.5);
    assert_eq!(st.efficiency[1], 81.0);
    // i=2 轮先赋哨兵再 break → 哨兵留在数组里; i>=3 保持 init 的 0
    assert_eq!(st.throttles[2], -65535);
    assert_eq!(st.thrust[2], -65535);
    assert_eq!(st.throttles[3], 0);
}

#[test]
fn init_resets_arrays_and_sentinels() {
    let mut st = State::new();
    st.init();
    assert_eq!(st.valid, Some(false));
    assert_eq!(st.throttles.len(), MAX_ENG_NUM);
    assert_eq!(st.power.len(), MAX_ENG_NUM);
    assert_eq!(st.thrust.len(), MAX_ENG_NUM);
    assert!(st.throttles.iter().all(|&v| v == 0));
    assert_eq!(st.engine_num, 0);
    assert_eq!(st.airbrake, 0);
}
