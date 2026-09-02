//! vm-overlay: overlay 渲染与平台窗口层 (POC 语义复刻成果)。
//! 波10 分域: platform(窗口/托盘/热键/host) / render(canvas/fields/renderers/font/
//! palette/primitives) / overlays(~17 组件, 原 overlays_field1/2 壳退役) /
//! layout(布局引擎+常量) / ui_model(数据字段模型)。
//! 单一真相路径 (波16, 对齐 vm-core 波9 原则): 全库唯一 `vm_overlay::<域>::<模块>`
//! 访问, 根 re-export 壳已退役。

// ---- 域模块 (5) ----
pub mod layout;
pub mod overlays;
pub mod platform;
pub mod render;
pub mod ui_model;
