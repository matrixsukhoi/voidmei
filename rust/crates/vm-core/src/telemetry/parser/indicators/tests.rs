use super::*;

/// 真机抓取的 /indicators 快照 (bf109f4, mock 8111 线上格式)。断言值 = Java 8 oracle 实测。
const IND_MOCK: &str = "{\"valid\": true,\"army\": \"air\",\"type\": \"bf-109f-4\",\"speed\": 131.007797,\"pedals1\": -0.465775,\"pedals2\": -0.465775,\"pedals3\": -0.465775,\"pedals4\": -0.465775,\"stick_elevator\": 0.187344,\"stick_elevator1\": 0.187344,\"stick_ailerons\": -0.477257,\"vario\": -7.342558,\"altitude_hour\": 151.084213,\"altitude_min\": 151.084213,\"altitude_10k\": 151.084213,\"aviahorizon_roll\": -40.553505,\"aviahorizon_pitch\": 0.632352,\"bank\": 8.0,\"turn\": 0.131359,\"compass\": 164.09729,\"compass1\": 164.09729,\"clock_hour\": 2.7,\"clock_min\": 42.0,\"clock_sec\": 6.0,\"manifold_pressure\": 2.243279,\"rpm\": 3001.457031,\"oil_pressure\": 1.0,\"oil_temperature\": 90.373169,\"water_temperature\": 121.324493,\"mixture\": 0.833333,\"carb_temperature\": 0.0,\"fuel1\": 196.846497,\"fuel2\": 0.0,\"fuel_pressure\": 11.0,\"gears\": 0.0,\"gear_lamp_down\": 0.0,\"gear_lamp_up\": 0.0,\"gear_lamp_off\": 0.0,\"flaps\": 0.0,\"throttle\": 1.1,\"weapon1\": 0.0,\"prop_pitch\": 1.0}";

#[test]
fn update_full_snapshot_matches_java_oracle() {
    let mut id = Indicators::new();
    id.init();
    id.update(IND_MOCK);
    assert_eq!(id.valid.as_deref(), Some("true"));
    // type 去引号 + toUpperCase; 9 字符不 >9 → stype 同 type (oracle 实测)
    assert_eq!(id.r#type.as_deref(), Some("BF-109F-4"));
    assert_eq!(id.stype.as_deref(), Some("BF-109F-4"));
    assert!(id.flag);
    // "pedals" 键搜索命中 "pedals1" 前缀 (getString 首次子串匹配, Java 同此)
    assert_eq!(id.speed, 131.007_8_f32 as f64);
    assert_eq!(id.pedals, -0.465775f32 as f64);
    assert_eq!(id.stick_elevator, 0.187344f32 as f64);
    assert_eq!(id.stick_ailerons, -0.477257f32 as f64);
    assert_eq!(id.altitude_hour, 151.084_21_f32 as f64);
    assert_eq!(id.bank, 8.0);
    assert_eq!(id.turn, 0.131359f32 as f64);
    assert_eq!(id.compass, 164.09729f32 as f64);
    assert_eq!(id.clock_hour, 2.7f32 as f64);
    assert_eq!(id.clock_min, 42.0);
    assert_eq!(id.clock_sec, 6.0);
    assert_eq!(id.manifold_pressure, 2.243279f32 as f64);
    assert_eq!(id.rpm, 3_001.457_f32 as f64);
    assert_eq!(id.oil_pressure, 1.0);
    // water_temperature 字段从未赋值 (赋值行已注释) → 恒 0.0; head_temperature 缺失 → 哨兵
    assert_eq!(id.water_temperature, 0.0);
    assert_eq!(id.engine_temperature, -65535.0);
    assert_eq!(id.mixture, 0.833333f32 as f64);
    // "\"fuel\"" 带引号键在 payload 中不存在 → fuel[0] 哨兵 → 归 0 (oracle 实测)
    assert_eq!(id.fuel[0], 0.0);
    assert_eq!(id.fuel[1], 196.846_5_f32 as f64);
    assert_eq!(id.fuel[2], 0.0); // fuel2=0.0 有效值 (≠哨兵), 不归零处理前也计入
    assert_eq!(id.fuel[3], 0.0); // 缺失 → 哨兵 → 0
    assert_eq!(id.fuelnum, 3); // 1 (基准) + fuel1 + fuel2
    assert_eq!(id.fuel_pressure, 11.0);
    assert_eq!(id.oxygen, -65535.0);
    assert_eq!(id.gears_lamp, -65535.0);
    assert_eq!(id.flaps, 0.0);
    assert_eq!(id.trimmer, -65535.0);
    assert_eq!(id.throttle, 1.1f32 as f64);
    assert_eq!(id.weapon1, 0.0);
    assert_eq!(id.weapon2, -65535.0);
    assert_eq!(id.weapon3, -65535.0);
    assert_eq!(id.vario, -7.342558f32 as f64);
    assert_eq!(id.aviahorizon_pitch, 0.632352f32 as f64);
    assert_eq!(id.aviahorizon_roll, -40.553505f32 as f64);
    assert_eq!(id.wsweep_indicator, -65535.0);
    assert_eq!(id.radio_altitude, -65535.0);
    assert_eq!(id.mach, -65535.0);
    assert_eq!(id.oil_temp, 90.373_17_f32 as f64);
    assert_eq!(id.water_temp, 121.324_49_f32 as f64);
}

#[test]
fn init_sets_nastring_and_defaults() {
    let mut id = Indicators::new();
    id.init();
    assert_eq!(id.valid.as_deref(), Some("-")); // Service.nastring 内联
    assert_eq!(id.fuelnum, 0);
    assert_eq!(id.fuel, [0.0; 5]);
    assert!(!id.flag);
    assert_eq!(id.mach, 0.0);
}

#[test]
fn update_tank_army_still_parsed_as_aircraft() {
    // getString 返回值含引号 → army == "\"tank\"" ≠ "tank" → 不走 else 分支
    // (Java 同此, oracle 实测 type=TU-4) — tank 过滤名存实亡, 保真保留
    let mut id = Indicators::new();
    id.init();
    id.update("{\"valid\": true, \"army\": \"tank\", \"type\": \"tu-4\"}");
    assert!(id.flag);
    assert_eq!(id.r#type.as_deref(), Some("TU-4"));
}

#[test]
fn update_missing_type_gives_empty_type_null_stype() {
    // 防御分支: type 缺失 → "" ; stype 不赋值保持 null (oracle 实测 stype=null)
    let mut id = Indicators::new();
    id.init();
    id.update("{\"valid\": true, \"speed\": 55.5}");
    assert!(id.flag);
    assert_eq!(id.r#type.as_deref(), Some(""));
    assert!(id.stype.is_none());
    assert_eq!(id.speed, 55.5);
}

#[test]
fn update_long_type_truncates_stype_to_8() {
    let mut id = Indicators::new();
    id.init();
    id.update("{\"valid\": true, \"type\": \"aaaaaaaaaaaaaaaaaaaa\"}");
    assert_eq!(id.r#type.as_deref(), Some("AAAAAAAAAAAAAAAAAAAA"));
    assert_eq!(id.stype.as_deref(), Some("AAAAAAAA")); // >9 → 前 8 字符
}

#[test]
fn update_short_type_keeps_stype_equal() {
    // 去壳后单字符: type="A" (带引号 3 字符 >1 → 进去壳分支), ≤9 → stype=type
    let mut id = Indicators::new();
    id.init();
    id.update("{\"valid\": true, \"type\": \"a\"}");
    assert_eq!(id.r#type.as_deref(), Some("A"));
    assert_eq!(id.stype.as_deref(), Some("A"));
}

#[test]
fn update_invalid_gives_no_cockpit() {
    let mut id = Indicators::new();
    id.init();
    id.update("{\"valid\": false}");
    assert_eq!(id.r#type.as_deref(), Some("No Cockpit"));
    assert_eq!(id.stype.as_deref(), Some("NoCockpit"));
    assert!(!id.flag);
    assert_eq!(id.speed, 0.0);
}

#[test]
fn update_cjk_type_bmp_equivalent() {
    // CJK 机型名 (BMP): 去引号去壳按字符, 与 Java UTF-16 码元语义等价
    let mut id = Indicators::new();
    id.init();
    id.update("{\"valid\": true, \"type\": \"歼-20\"}");
    assert_eq!(id.r#type.as_deref(), Some("歼-20"));
    assert_eq!(id.stype.as_deref(), Some("歼-20"));
}

#[test]
fn update_fuel_sentinel_zeroing() {
    // fuel1 哨兵 → 0 且不计数; fuel2 有效 (含 0.0) → 计数
    let mut id = Indicators::new();
    id.init();
    id.update("{\"valid\": true, \"fuel1\": 12.5, \"fuel2\": 0.0, \"fuel3\": 7.5}");
    assert_eq!(id.fuel[1], 12.5);
    assert_eq!(id.fuel[2], 0.0);
    assert_eq!(id.fuel[3], 7.5);
    assert_eq!(id.fuel[4], 0.0); // 缺失 → 哨兵 → 0
                                 // 基准 1 + fuel1/fuel2/fuel3 各计数 +1, fuel4 缺失不计数
    assert_eq!(id.fuelnum, 4);
}
