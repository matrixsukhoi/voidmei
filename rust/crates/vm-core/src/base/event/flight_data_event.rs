//! 飞行数据事件: 瘦载荷 (W-B 事件瘦身)。
//!
//! 事件只承载标量 payload + 时间戳, 作为 Service→win32 的帧节拍与轻数据通道;
//! State/Indicators/派生量一律由消费方持共享 ServiceData guard 现取, 不再装箱
//! (原 Java Object 引用跨线程传递的直译形态已废)。

use std::time::{SystemTime, UNIX_EPOCH};

use crate::base::event::event_payload::EventPayload;

pub struct FlightDataEvent {
    payload: EventPayload,
    timestamp: i64,
}

impl FlightDataEvent {
    pub fn new(payload: EventPayload) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        FlightDataEvent { payload, timestamp }
    }

    pub fn get_payload(&self) -> &EventPayload {
        &self.payload
    }

    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }
}

#[cfg(test)]
mod tests;
