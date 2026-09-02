//! W-B 事件瘦身后事件仅剩 payload + timestamp; 原 from_data/getData/get/
//! getHudData/setHudData/getState/getIndicators 通道已废, 对应用例随 API 删除。

use super::*;

// 构造器: payload 就位, get_payload 返回同一载荷
#[test]
fn test_new_sets_fields() {
    let payload = EventPayload::builder().map_grid("K3".to_string()).build();
    let event = FlightDataEvent::new(payload.clone());
    assert_eq!(event.get_payload(), &payload);
}

// timestamp = System.currentTimeMillis(): 合理的当代 epoch 毫秒区间
#[test]
fn test_timestamp_epoch_millis() {
    let event = FlightDataEvent::new(EventPayload::builder().build());
    let ts = event.get_timestamp();
    // 2018-01-01 之后, 2100 年之前
    assert!(ts >= 1_515_000_000_000, "timestamp too old: {ts}");
    assert!(ts < 41_024_480_000_000, "timestamp too far: {ts}");
}
