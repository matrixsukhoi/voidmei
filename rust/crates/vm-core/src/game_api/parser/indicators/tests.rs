use super::*;

/// 真机抓取的 /indicators 快照 (bf109f4, mock 8111 线上格式)。
/// 波20 serde 化: 断言从 f32 单精度拓宽位级值改为 f64 直读值。
const IND_MOCK: &str = "{\"valid\": true,\"army\": \"air\",\"type\": \"bf-109f-4\",\"speed\": 131.007797,\"pedals1\": -0.465775,\"pedals2\": -0.465775,\"pedals3\": -0.465775,\"pedals4\": -0.465775,\"stick_elevator\": 0.187344,\"stick_elevator1\": 0.187344,\"stick_ailerons\": -0.477257,\"vario\": -7.342558,\"altitude_hour\": 151.084213,\"altitude_min\": 151.084213,\"altitude_10k\": 151.084213,\"aviahorizon_roll\": -40.553505,\"aviahorizon_pitch\": 0.632352,\"bank\": 8.0,\"turn\": 0.131359,\"compass\": 164.09729,\"compass1\": 164.09729,\"clock_hour\": 2.7,\"clock_min\": 42.0,\"clock_sec\": 6.0,\"manifold_pressure\": 2.243279,\"rpm\": 3001.457031,\"oil_pressure\": 1.0,\"oil_temperature\": 90.373169,\"water_temperature\": 121.324493,\"mixture\": 0.833333,\"carb_temperature\": 0.0,\"fuel1\": 196.846497,\"fuel2\": 0.0,\"fuel_pressure\": 11.0,\"gears\": 0.0,\"gear_lamp_down\": 0.0,\"gear_lamp_up\": 0.0,\"gear_lamp_off\": 0.0,\"flaps\": 0.0,\"throttle\": 1.1,\"weapon1\": 0.0,\"prop_pitch\": 1.0}";

#[test]
fn update_full_snapshot() {
    let mut id = Indicators::new();
    id.update(IND_MOCK);
    assert_eq!(id.valid, Some(true));
    // type 裸串 toUpperCase (波21: Java 时代的 stype 截 8 死字段已删)
    assert_eq!(id.r#type.as_deref(), Some("BF-109F-4"));
    assert!(id.flag);
    // pedals 显式取 pedals1 (快照无裸 pedals 键; 手写时代靠子串碰撞取到同值)
    assert_eq!(id.speed, 131.007797);
    assert_eq!(id.pedals, -0.465775);
    assert_eq!(id.stick_elevator, 0.187344);
    assert_eq!(id.stick_ailerons, -0.477257);
    assert_eq!(id.altitude_hour, 151.084213);
    assert_eq!(id.bank, 8.0);
    assert_eq!(id.turn, 0.131359);
    assert_eq!(id.compass, 164.09729);
    assert_eq!(id.clock_hour, 2.7);
    assert_eq!(id.clock_min, 42.0);
    assert_eq!(id.clock_sec, 6.0);
    assert_eq!(id.manifold_pressure, 2.243279);
    assert_eq!(id.rpm, 3001.457031);
    assert_eq!(id.oil_pressure, 1.0);
    // water_temperature 字段从未赋值 (赋值行已注释) → 恒 0.0; head_temperature 缺失 → 哨兵
    assert_eq!(id.water_temperature, 0.0);
    assert_eq!(id.engine_temperature, -65535.0);
    assert_eq!(id.mixture, 0.833333);
    // 裸 fuel 键真机不存在 → fuel[0] 哨兵 → 归 0
    assert_eq!(id.fuel[0], 0.0);
    assert_eq!(id.fuel[1], 196.846497);
    assert_eq!(id.fuel[2], 0.0); // fuel2=0.0 有效值 (≠哨兵), 不归零处理前也计入
    assert_eq!(id.fuel[3], 0.0); // 缺失 → 哨兵 → 0
    assert_eq!(id.fuelnum, 3); // 1 (基准) + fuel1 + fuel2
    assert_eq!(id.fuel_pressure, 11.0);
    assert_eq!(id.oxygen, -65535.0);
    // 真键是 gear_lamp_down/up/off 族 → gears_lamp 恒哨兵 (手写时代同此)
    assert_eq!(id.gears_lamp, -65535.0);
    assert_eq!(id.flaps, 0.0);
    assert_eq!(id.trimmer, -65535.0);
    assert_eq!(id.throttle, 1.1);
    assert_eq!(id.weapon1, 0.0);
    assert_eq!(id.weapon2, -65535.0);
    assert_eq!(id.weapon3, -65535.0);
    assert_eq!(id.vario, -7.342558);
    assert_eq!(id.aviahorizon_pitch, 0.632352);
    assert_eq!(id.aviahorizon_roll, -40.553505);
    assert_eq!(id.wsweep_indicator, -65535.0);
    assert_eq!(id.radio_altitude, -65535.0);
    assert_eq!(id.mach, -65535.0);
    assert_eq!(id.oil_temp, 90.373169);
    assert_eq!(id.water_temp, 121.324493);
}

#[test]
fn new_sets_defaults() {
    // 波21: new+init 两段式退役 — 构造即就绪 (valid=None 表示未见过响应)
    let id = Indicators::new();
    assert_eq!(id.valid, None);
    assert_eq!(id.fuelnum, 0);
    assert_eq!(id.fuel, [0.0; 5]);
    assert!(!id.flag);
    assert_eq!(id.mach, 0.0);
}

#[test]
fn update_army_key_ignored() {
    // 波20: army=="tank" 死分支 (手写时代字符串带引号永不成立) 已随 army 字段删除 —
    // army 键存在与否不影响解析, 坦克/飞机统一按 type 识别 (FMManager 侧另有 NOT_AIRCRAFT 短路)
    let mut id = Indicators::new();
    id.update("{\"valid\": true, \"army\": \"tank\", \"type\": \"tu-4\"}");
    assert!(id.flag);
    assert_eq!(id.r#type.as_deref(), Some("TU-4"));
}

#[test]
fn update_missing_type_gives_empty() {
    // 防御分支: type 缺失 → ""
    let mut id = Indicators::new();
    id.update("{\"valid\": true, \"speed\": 55.5}");
    assert!(id.flag);
    assert_eq!(id.r#type.as_deref(), Some(""));
    assert_eq!(id.speed, 55.5);
}

#[test]
fn update_invalid_gives_no_cockpit() {
    let mut id = Indicators::new();
    id.update("{\"valid\": false}");
    assert_eq!(id.r#type.as_deref(), Some("No Cockpit"));
    assert!(id.is_no_cockpit()); // 波21: 字符串状态谓词化
    assert!(!id.flag);
    assert_eq!(id.speed, 0.0);
}

#[test]
fn update_cjk_type() {
    // CJK 机型名: to_uppercase 对 CJK 无变化
    let mut id = Indicators::new();
    id.update("{\"valid\": true, \"type\": \"歼-20\"}");
    assert_eq!(id.r#type.as_deref(), Some("歼-20"));
}

#[test]
fn update_fuel_sentinel_zeroing() {
    // fuel1 哨兵 → 0 且不计数; fuel2 有效 (含 0.0) → 计数
    let mut id = Indicators::new();
    id.update("{\"valid\": true, \"fuel1\": 12.5, \"fuel2\": 0.0, \"fuel3\": 7.5}");
    assert_eq!(id.fuel[1], 12.5);
    assert_eq!(id.fuel[2], 0.0);
    assert_eq!(id.fuel[3], 7.5);
    assert_eq!(id.fuel[4], 0.0); // 缺失 → 哨兵 → 0
                                 // 基准 1 + fuel1/fuel2/fuel3 各计数 +1, fuel4 缺失不计数
    assert_eq!(id.fuelnum, 4);
}
