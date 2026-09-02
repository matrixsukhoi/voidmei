//! ui.model 数据模型层: overlay 显示字段模型 (DataField/GaugeField)。
//! 其余 Java 镜像 (FieldManager 族/两个 Config/Deriver 桥/TelemetrySource)
//! 已随公式系统直通与字段表统一批次消解。

pub mod data_field;
pub mod gauge_field;

pub use data_field::DataField;
pub use gauge_field::GaugeField;
