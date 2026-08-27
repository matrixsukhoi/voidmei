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
mod tests;
