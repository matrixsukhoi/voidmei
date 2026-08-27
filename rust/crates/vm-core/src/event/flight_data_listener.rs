//! 对应 Java: `src/prog/event/FlightDataListener.java`

use crate::event::flight_data_event::FlightDataEvent;

/// Interface for consuming Data Plane events.
/// PORT: Java interface → Rust trait (§1 多实现接口); 参数按 Java 引用传递语义
/// 取 `&FlightDataEvent` (同一事件对象按序发布给多个订阅者, 只读)。
pub trait FlightDataListener {
    /// 对应 Java `void onFlightData(FlightDataEvent event)`。
    /// 回调发生在发布线程 (Service 线程), 订阅方碰 UI 须自行切线程。
    fn on_flight_data(&self, event: &FlightDataEvent);
}

#[cfg(test)]
mod tests;
