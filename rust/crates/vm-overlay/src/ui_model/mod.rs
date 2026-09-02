//! ui.model 数据模型层: overlay 显示字段模型 (DataField)。
//! 其余 Java 镜像 (FieldManager 族/GaugeField/两个 Config/Deriver 桥/
//! TelemetrySource) 已随公式系统直通、字段表统一与波12 死代码清扫消解。

pub mod data_field;

pub use data_field::DataField;
