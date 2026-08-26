//! 对应 Java: `src/prog/event/FlightDataEvent.java`
//! Immutable event carrying a snapshot of flight telemetry data.
//! Thread-safe for passing between Service thread and EDT.
//!
//! Primary access is via `get_payload()` for type-safe fields.
//! Legacy `get_data()` / `get()` are retained for un-migrated consumers
//! (e.g. FieldOverlay legacy path).

use std::any::Any;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::event::event_payload::EventPayload;

/// PORT: Java 的 `Object state` (实为 parser.State) / `Object indicators` (实为
/// parser.Indicators) / `ui.overlay.model.HUDData hudData` 三个引用字段——具体类型
/// 分属 B/C 类后续批次, 尚未落地; 以 `Box<dyn Any + Send + Sync>` 保有 Java
/// "Object 引用跨线程 (Service 线程→EDT) 传递" 的语义。各类型翻译完成后收紧。
/// PORT: downcast 失败返回 None 是静默的, 不同于 Java 错误强转抛 ClassCastException
/// 的快速失败——B 批消费端必须按具体类型 downcast, 不得依赖 None 值存活分支。
pub type OpaqueObject = Box<dyn Any + Send + Sync>;

/// Immutable event carrying a snapshot of flight telemetry data. 字段与顺序与 Java 一致。
///
/// Primary access is via [`FlightDataEvent::get_payload`] for type-safe fields.
pub struct FlightDataEvent {
    payload: EventPayload,
    state: Option<OpaqueObject>,      // parser.State
    indicators: Option<OpaqueObject>, // parser.Indicators
    timestamp: i64,

    /// Pre-computed HUD data (calculated on Service thread, consumed on EDT). May be null.
    /// PORT: Java 字段缺省 null → None; set-after-construct (Service.java:473
    /// 构造后、发布前 setHudData) 语义保持为 `&mut self` 可变方法。
    hud_data: Option<OpaqueObject>,
}

impl FlightDataEvent {
    /// 对应 Java `FlightDataEvent(EventPayload payload, Object state, Object indicators)`。
    /// PORT: System.currentTimeMillis → SystemTime (§3 库映射); as_millis 的 u128
    /// 经 `as i64` 截断; 时钟早于 epoch 时 Java 可得负值而 duration_since 报错 → 取 0。
    pub fn new(
        payload: EventPayload,
        state: Option<OpaqueObject>,
        indicators: Option<OpaqueObject>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        FlightDataEvent {
            payload,
            state,
            indicators,
            timestamp,
            hud_data: None,
        }
    }

    /// @deprecated Use `FlightDataEvent::new(payload, state, indicators)`
    /// 对应 Java `FlightDataEvent(Map<String, String> data)` → `this(data, null, null)`。
    /// PORT: Java Map 参数可为 null → `Option<&HashMap>`; 重载无对应物, 命名 from_data。
    /// PORT: 同类内委托调用 deprecated 成员, javac 不告警 ↔ 此处局部 allow。
    #[deprecated(note = "Use FlightDataEvent::new(payload, state, indicators)")]
    #[allow(deprecated)]
    pub fn from_data(data: Option<&HashMap<String, String>>) -> Self {
        Self::from_data_with_state(data, None, None)
    }

    /// @deprecated Use `FlightDataEvent::new(payload, state, indicators)`
    /// 对应 Java `FlightDataEvent(Map<String, String> data, Object state, Object indicators)`
    /// → `this(mapToPayload(data), state, indicators)`。
    #[deprecated(note = "Use FlightDataEvent::new(payload, state, indicators)")]
    pub fn from_data_with_state(
        data: Option<&HashMap<String, String>>,
        state: Option<OpaqueObject>,
        indicators: Option<OpaqueObject>,
    ) -> Self {
        Self::new(map_to_payload(data), state, indicators)
    }

    pub fn get_payload(&self) -> &EventPayload {
        &self.payload
    }

    /// 对应 Java `Object getState()` (可能为 null) — 消费方自行 downcast 到 parser.State。
    pub fn get_state(&self) -> Option<&(dyn Any + Send + Sync)> {
        self.state.as_deref()
    }

    /// 对应 Java `Object getIndicators()` (可能为 null)。
    pub fn get_indicators(&self) -> Option<&(dyn Any + Send + Sync)> {
        self.indicators.as_deref()
    }

    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }

    /// Gets pre-computed HUD data (calculated on Service thread).
    /// @return HUDData if computed, null otherwise → None
    pub fn get_hud_data(&self) -> Option<&(dyn Any + Send + Sync)> {
        self.hud_data.as_deref()
    }

    /// Sets pre-computed HUD data (called on Service thread before publishing).
    /// @param data The pre-computed HUD data
    pub fn set_hud_data(&mut self, data: OpaqueObject) {
        self.hud_data = Some(data);
    }

    /// @deprecated Use `get_payload()` for type-safe access.
    /// 对应 Java `Map<String, String> getData()` — 7 个键逐项 put, 顺序即代码顺序。
    /// PORT: String.valueOf(boolean) 与 Rust bool::to_string 同为 "true"/"false"
    /// (Java 8 oracle 实测一致)。返回 std HashMap (§2.5: 本类内仅按键查找消费,
    /// 无迭代序依赖; 若未来 legacy 消费方迭代序敏感再换 IndexMap)。
    #[deprecated(note = "Use get_payload() for type-safe access")]
    pub fn get_data(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("mapGrid".to_string(), self.payload.map_grid.clone());
        map.insert("fatalWarn".to_string(), self.payload.fatal_warn.to_string());
        map.insert(
            "radioAltValid".to_string(),
            self.payload.radio_alt_valid.to_string(),
        );
        map.insert(
            "isDowningFlap".to_string(),
            self.payload.is_downing_flap.to_string(),
        );
        map.insert("timeStr".to_string(), self.payload.time_str.clone());
        map.insert("is_jet".to_string(), self.payload.is_jet.to_string());
        map.insert(
            "engine_check_done".to_string(),
            self.payload.engine_check_done.to_string(),
        );
        map
    }

    /// @deprecated Use `get_payload().field_name` instead.
    /// 对应 Java `String get(String key)` — getData().get(key), 缺键返回 null → None。
    /// PORT: 同类内调用 deprecated getData, javac 不告警 ↔ 此处局部 allow。
    #[deprecated(note = "Use get_payload().field instead")]
    #[allow(deprecated)]
    pub fn get(&self, key: &str) -> Option<String> {
        self.get_data().get(key).cloned()
    }
}

/// Convert legacy Map constructor arg into EventPayload.
/// 对应 Java `private static EventPayload mapToPayload(Map<String, String> data)`。
/// PORT: Java `data == null` 分支由参数 Option 表达; Rust HashMap<String, String>
/// 无法存 null 值, Java "containsKey 且值为 null → mapGrid/timeStr 置 null" 的
/// 病态分支在此坍缩为缺省 "--"/"--:--" (Rust 侧不可达, 行为差异仅限该病态输入)。
fn map_to_payload(data: Option<&HashMap<String, String>>) -> EventPayload {
    // Java: if (data == null || data.isEmpty()) return EventPayload.builder().build();
    let data = match data {
        None => return EventPayload::builder().build(),
        Some(d) => d,
    };
    if data.is_empty() {
        return EventPayload::builder().build();
    }
    // Java: data.containsKey("mapGrid") ? data.get("mapGrid") : "--"
    EventPayload::builder()
        .map_grid(
            data.get("mapGrid")
                .cloned()
                .unwrap_or_else(|| "--".to_string()),
        )
        .fatal_warn(parse_boolean(data.get("fatalWarn")))
        .radio_alt_valid(parse_boolean(data.get("radioAltValid")))
        .is_downing_flap(parse_boolean(data.get("isDowningFlap")))
        .time_str(
            data.get("timeStr")
                .cloned()
                .unwrap_or_else(|| "--:--".to_string()),
        )
        .is_jet(parse_boolean(data.get("is_jet")))
        .engine_check_done(parse_boolean(data.get("engine_check_done")))
        .build()
}

/// 对应 `java.lang.Boolean.parseBoolean(String)`: 仅当串非 null 且忽略大小写等于
/// "true" 时返回 true, 其余 (含 null/空串/前后带空白/任意其它文本) 均为 false。
/// Java 8 oracle 实测 (build/oracle): null→false, "TrUe"→true, " true"/"true "→false
/// (不 trim, 区别于 parseFloat §2.15), "yes"/"1"/""→false。
/// PORT: eq_ignore_ascii_case 对目标串 "true" (纯 ASCII) 与 Java equalsIgnoreCase
/// 折叠域一致 (无任何非 ASCII 字符经 toUpperCase/toLowerCase 落到 t/r/u/e)。
fn parse_boolean(s: Option<&String>) -> bool {
    s.is_some_and(|v| v.eq_ignore_ascii_case("true"))
}

#[cfg(test)]
mod tests {
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
}
