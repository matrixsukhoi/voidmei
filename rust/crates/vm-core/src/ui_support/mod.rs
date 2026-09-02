//! UI 支撑域 (vm-app 与 vm-overlay 双消费, 不下沉单一消费 crate 的部分):
//! 行定义 (row_def, cfg 驱动) + 机型对比计算 (comparison)。

pub mod comparison;
pub mod row_def;
