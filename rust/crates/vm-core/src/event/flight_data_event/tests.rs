#![allow(deprecated)] // 测试刻意覆盖 Java @Deprecated 的 legacy 构造器/getData/get

use super::*;

// Java 构造器: payload/state/indicators 就位, timestamp 取当前毫秒, hudData=null
#[test]
fn test_new_sets_fields() {
    let payload = EventPayload::builder().map_grid("K3".to_string()).build();
    #[derive(Debug, PartialEq)]
    struct FakeState {
        id: i32,
    }
    let event = FlightDataEvent::new(payload.clone(), Some(Box::new(FakeState { id: 7 })), None);
    assert_eq!(event.get_payload(), &payload);
    // state 可 downcast 回具体类型 (Java Object 语义)
    let st = event.get_state().unwrap().downcast_ref::<FakeState>().unwrap();
    assert_eq!(st.id, 7);
    // indicators 缺省 → None
    assert!(event.get_indicators().is_none());
    // hudData 构造时为 null → None
    assert!(event.get_hud_data().is_none());
}

// timestamp = System.currentTimeMillis(): 合理的当代 epoch 毫秒区间
#[test]
fn test_timestamp_epoch_millis() {
    let event = FlightDataEvent::new(EventPayload::builder().build(), None, None);
    let ts = event.get_timestamp();
    // 2018-01-01 之后, 2100 年之前
    assert!(ts >= 1_515_000_000_000, "timestamp too old: {ts}");
    assert!(ts < 41_024_480_000_000, "timestamp too far: {ts}");
}

// setHudData 在构造后写入, getHudData 读出; 重复 set 覆盖旧值
#[test]
fn test_hud_data_set_after_construct() {
    #[derive(Debug, PartialEq)]
    struct FakeHud {
        v: f64,
    }
    let mut event = FlightDataEvent::new(EventPayload::builder().build(), None, None);
    event.set_hud_data(Box::new(FakeHud { v: 9.5 }));
    let hud = event.get_hud_data().unwrap().downcast_ref::<FakeHud>().unwrap();
    assert_eq!(hud.v, 9.5);
    event.set_hud_data(Box::new(FakeHud { v: 1.0 }));
    let hud = event.get_hud_data().unwrap().downcast_ref::<FakeHud>().unwrap();
    assert_eq!(hud.v, 1.0);
}

// Java: mapToPayload(null) → 全缺省 Builder
#[test]
fn test_from_data_null_defaults() {
    let event = FlightDataEvent::from_data(None);
    let p = event.get_payload();
    assert_eq!(p.map_grid, "--");
    assert_eq!(p.time_str, "--:--");
    assert!(!p.fatal_warn);
    assert!(!p.radio_alt_valid);
    assert!(!p.is_downing_flap);
    assert!(!p.is_jet);
    assert!(!p.engine_check_done);
    // mapToPayload 不读 stage/mismatch 两字段 → 保持 Builder 缺省
    assert_eq!(p.optimal_compressor_stage, -1);
    assert!(!p.compressor_stage_mismatch);
}

// Java: mapToPayload(空 Map) → 全缺省
#[test]
fn test_from_data_empty_map_defaults() {
    let empty: HashMap<String, String> = HashMap::new();
    let event = FlightDataEvent::from_data(Some(&empty));
    assert_eq!(event.get_payload(), &EventPayload::builder().build());
}

// 7 键齐备时的映射: 字符串直传, 布尔走 parseBoolean, 键名 is_jet/engine_check_done 原样
#[test]
fn test_from_data_full_map() {
    let mut m = HashMap::new();
    m.insert("mapGrid".to_string(), "C4".to_string());
    m.insert("fatalWarn".to_string(), "true".to_string());
    m.insert("radioAltValid".to_string(), "false".to_string());
    m.insert("isDowningFlap".to_string(), "true".to_string());
    m.insert("timeStr".to_string(), "07:55".to_string());
    m.insert("is_jet".to_string(), "true".to_string());
    m.insert("engine_check_done".to_string(), "true".to_string());
    let p = FlightDataEvent::from_data(Some(&m)).get_payload().clone();
    assert_eq!(p.map_grid, "C4");
    assert!(p.fatal_warn);
    assert!(!p.radio_alt_valid);
    assert!(p.is_downing_flap);
    assert_eq!(p.time_str, "07:55");
    assert!(p.is_jet);
    assert!(p.engine_check_done);
}

// Boolean.parseBoolean 全域边界 (Java 8 oracle 对拍):
// 混合大小写真, 前后空白/其它文本/缺键均假
#[test]
fn test_parse_boolean_semantics() {
    let mk = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<String, String>>()
    };
    let m = mk(&[
        ("fatalWarn", "TrUe"),
        ("radioAltValid", " true"),
        ("isDowningFlap", "true "),
        ("is_jet", "yes"),
    ]);
    let p = FlightDataEvent::from_data(Some(&m)).get_payload().clone();
    assert!(p.fatal_warn, "equalsIgnoreCase(\"true\") 混合大小写");
    assert!(!p.radio_alt_valid, "parseBoolean 不 trim 前导空白");
    assert!(!p.is_downing_flap, "parseBoolean 不 trim 尾随空白");
    assert!(!p.is_jet, "非 true 文本 → false");
    // 缺键 → parseBoolean(null) → false (oracle: null=false)
    assert!(!p.engine_check_done);
}

// 部分键缺失: 字符串键回退 "--"/"--:--", 布尔键回退 false
#[test]
fn test_from_data_partial_map() {
    let mut m = HashMap::new();
    m.insert("fatalWarn".to_string(), "TRUE".to_string());
    let p = FlightDataEvent::from_data(Some(&m)).get_payload().clone();
    assert_eq!(p.map_grid, "--");
    assert!(p.fatal_warn);
    assert_eq!(p.time_str, "--:--");
    assert!(!p.is_jet);
}

// getData(): 7 键内容与布尔串化 ("true"/"false") 逐项核对 (键集合与 Java put 序一致)
#[test]
fn test_get_data_contents() {
    let payload = EventPayload::builder()
        .map_grid("D5".to_string())
        .fatal_warn(true)
        .radio_alt_valid(false)
        .is_downing_flap(true)
        .time_str("11:11".to_string())
        .is_jet(false)
        .engine_check_done(true)
        .build();
    let event = FlightDataEvent::new(payload, None, None);
    let data = event.get_data();
    assert_eq!(data.len(), 7); // 不含 stage/mismatch 两字段 (Java 亦不 put)
    assert_eq!(data["mapGrid"], "D5");
    assert_eq!(data["fatalWarn"], "true");
    assert_eq!(data["radioAltValid"], "false");
    assert_eq!(data["isDowningFlap"], "true");
    assert_eq!(data["timeStr"], "11:11");
    assert_eq!(data["is_jet"], "false");
    assert_eq!(data["engine_check_done"], "true");
}

// get(key): 命中返回值, 缺键返回 None (Java null)
#[test]
fn test_get_by_key() {
    let event = FlightDataEvent::new(
        EventPayload::builder().map_grid("E6".to_string()).build(),
        None,
        None,
    );
    assert_eq!(event.get("mapGrid").as_deref(), Some("E6"));
    assert_eq!(event.get("nonexistent"), None);
}
