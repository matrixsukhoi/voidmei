//! prog.event 数据侧 (A 类): 事件载荷 / 遥测事件对象 / 监听 trait / UI 状态事件常量。
//! 总线 (FlightDataBus/UIStateBus) 属 B 类后续批次, 不在本模块。
//! 类型顶层 re-export 镜像 Java `import prog.event.EventPayload` 的扁平引用;
//! ui_state_events 是纯常量容器, 刻意不做扁平 re-export——消费方走全路径
//! `crate::base::event::ui_state_events::X`, 对应 Java `UIStateEvents.X` 类名前缀引用
//! (physics_constants.rs 同款先例, P1 裁决: 常量容器保持模块限定)。

pub mod event_payload;
pub mod flight_data_event;
pub mod flight_data_listener;
pub mod ui_state_events;

pub use event_payload::{EventPayload, EventPayloadBuilder};
pub use flight_data_event::FlightDataEvent;
pub use flight_data_listener::FlightDataListener;
