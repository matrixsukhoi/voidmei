//! ui.model 数据模型层 (A 类): 对应 Java `src/ui/model/` 10 文件一比一翻译。
//! overlay 显示字段模型 (DataField 族)、字段管理 (FieldManager)、
//! 遥测/FM 数据源接口 (TelemetrySource / FMDataSource) 与 Blkx 适配器。
//!
//! PORT: Java 包内扁平引用 (`ui.model.DataField`) → 类型在 mod 根 re-export 镜像;
//! config_stub 的三个类型刻意不做扁平 re-export —— 它们是 prog.config 的依赖桩
//! (非本包类型), 消费方走全路径 `crate::ui_model::config_stub::X`。

pub mod config_stub;
pub mod data_field;
pub mod default_field_manager;
pub mod engine_info_config;
pub mod field_definition;
pub mod field_manager;
pub mod flight_info_config;
pub mod gauge_field;
pub mod telemetry_source;

pub use data_field::DataField;
pub use default_field_manager::DefaultFieldManager;
pub use engine_info_config::EngineInfoConfig;
pub use field_definition::FieldDefinition;
pub use field_manager::FieldManager;
pub use flight_info_config::FlightInfoConfig;
pub use gauge_field::GaugeField;
pub use telemetry_source::TelemetrySource;
