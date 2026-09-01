use super::*;

/// 真机抓取的 /state 快照 (script/mock_scenarios/snapshots/plane_bf109f4.json,
/// mock 8111 线上格式: 冒号后一空格, 逗号后无空格)。断言值 = Java 8 oracle 实测。
const STATE_MOCK: &str = "{\"valid\": true,\"aileron, %\": -48,\"elevator, %\": 20,\"rudder, %\": -47,\"flaps, %\": 0,\"gear, %\": 0,\"H, m\": 46,\"TAS, km/h\": 454,\"IAS, km/h\": 474,\"M\": 0.39,\"AoA, deg\": -1.6,\"AoS, deg\": -5.9,\"Ny\": 0.35,\"Vy, m/s\": -7.3,\"Wx, deg/s\": -34,\"Mfuel, kg\": 197,\"Mfuel0, kg\": 734,\"throttle 1, %\": 110,\"RPM throttle 1, %\": 100,\"mixture 1, %\": 100,\"radiator 1, %\": 42,\"magneto 1\": 3,\"power 1, hp\": 1597.8,\"RPM 1\": 3001,\"manifold pressure 1, atm\": 2.24,\"water temp 1, C\": 121,\"oil temp 1, C\": 90,\"pitch 1, deg\": 35.5,\"thrust 1, kgs\": 840,\"efficiency 1, %\": 87}";

#[test]
fn update_full_snapshot_matches_java_oracle() {
    let mut st = State::new();
    st.init();
    assert_eq!(st.update(STATE_MOCK), 0);
    assert_eq!(st.valid.as_deref(), Some("true"));
    assert!(st.flag);
    assert_eq!(st.engine_num, 1);
    assert_eq!(st.aileron, -48);
    assert_eq!(st.elevator, 20);
    assert_eq!(st.rudder, -47);
    assert_eq!(st.flaps, 0);
    assert_eq!(st.gear, 0);
    assert_eq!(st.airbrake, -65535);
    assert_eq!(st.tas, 454);
    assert_eq!(st.ias, 474);
    // Float.parseFloat 单精度拓宽 (0.39f32 的精确 double 展开)
    assert_eq!(st.m, 0.39f32 as f64);
    assert_eq!(st.aoa, -1.6f32 as f64);
    assert_eq!(st.aos, -5.9f32 as f64);
    assert_eq!(st.ny, 0.35f32 as f64);
    assert_eq!(st.vy, -7.3f32 as f64);
    assert_eq!(st.wx, -34.0);
    assert_eq!(st.heightm, 46.0);
    assert_eq!(st.throttle, 110);
    assert_eq!(st.rpm_throttle, 100);
    assert_eq!(st.radiator, 42);
    assert_eq!(st.mixture, 100);
    assert_eq!(st.compressorstage, 0);
    assert_eq!(st.magenato, 3);
    assert_eq!(st.rpm, 3001);
    assert_eq!(st.manifoldpressure, 2.24f32 as f64);
    assert_eq!(st.watertemp, 121.0);
    assert_eq!(st.oiltemp, 90.0);
    assert_eq!(st.mfuel, 197.0);
    assert_eq!(st.mfuel_1, -65535.0);
    assert_eq!(st.mfuel0, 734.0);
    assert_eq!(st.mfuel0_1, -65535.0);
    assert_eq!(st.total_thr, 840.0);
    // 引擎数组: i=0 有效; i=1 已赋值哨兵后 break; i>=2 保持 init 的 0
    assert_eq!(st.throttles[0], 110);
    assert_eq!(st.power[0], 1597.8f32 as f64);
    assert_eq!(st.thrust[0], 840);
    assert_eq!(st.pitch[0], 35.5f32 as f64);
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
fn update_valid_false_keeps_fields() {
    let mut st = State::new();
    st.init();
    assert_eq!(st.update("{\"valid\": false}"), 0);
    assert_eq!(st.valid.as_deref(), Some("false"));
    assert!(!st.flag);
    assert_eq!(st.engine_num, 0);
}

#[test]
fn update_sentinel_normalizations() {
    // P-63 类自动桨: RPM throttle/mixture/compressorstage 缺失时哨兵归一化 (oracle 实测);
    // magneto 不归一化, 保留 -65535
    let mut st = State::new();
    st.init();
    st.update("{\"valid\": true, \"RPM throttle\": -65535, \"mixture\": -65535, \"compressor stage\": -65535, \"magneto\": 1}");
    assert_eq!(st.rpm_throttle, -1);
    assert_eq!(st.mixture, -1);
    assert_eq!(st.compressorstage, 0);
    assert_eq!(st.magenato, 1);
    // 无任何 thrust N 键 → 引擎循环第一轮即 break, engineNum=0
    assert_eq!(st.engine_num, 0);
}

#[test]
fn update_multi_engine() {
    let mut st = State::new();
    st.init();
    st.update("{\"valid\": true, \"throttle 1\": 90, \"throttle 2\": 95, \"power 1\": 1000.5, \"power 2\": 1100.25, \"thrust 1\": 500, \"thrust 2\": 600, \"pitch 1\": 30.5, \"pitch 2\": 31.5, \"efficiency 1\": 80, \"efficiency 2\": 81}");
    assert_eq!(st.engine_num, 2);
    assert_eq!(st.total_thr, 1100.0);
    assert_eq!(st.throttles[1], 95);
    assert_eq!(st.power[1], 1100.25f32 as f64);
    assert_eq!(st.thrust[1], 600);
    assert_eq!(st.pitch[1], 31.5f32 as f64);
    assert_eq!(st.efficiency[1], 81.0);
    // i=2 轮先赋哨兵再 break → 哨兵留在数组里; i>=3 保持 init 的 0 (oracle 实测)
    assert_eq!(st.throttles[2], -65535);
    assert_eq!(st.thrust[2], -65535);
    assert_eq!(st.throttles[3], 0);
}

#[test]
fn get_eng_num_counts_zero_based_thrust_keys() {
    // Java getEngNum 用 "thrust " + i (0 起始), 与 update 的 1 起始键名不同
    let mut st = State::new();
    st.init();
    st.get_eng_num("{\"thrust 0\": 100,\"thrust 1\": 200,\"thrust 2\": 300}");
    assert_eq!(st.engine_num, 3);
    assert_eq!(st.thrust[0], 100);
    assert_eq!(st.thrust[1], 200);
    assert_eq!(st.thrust[3], -65535); // 缺失键 → 哨兵, 不计数
}

#[test]
fn init_resets_arrays_and_sentinels() {
    let mut st = State::new();
    st.init();
    assert_eq!(st.valid.as_deref(), Some("false"));
    assert_eq!(st.throttles.len(), MAX_ENG_NUM);
    assert_eq!(st.power.len(), MAX_ENG_NUM);
    assert_eq!(st.thrust.len(), MAX_ENG_NUM);
    assert!(st.throttles.iter().all(|&v| v == 0));
    assert_eq!(st.engine_num, 0);
    assert_eq!(st.airbrake, 0);
}
