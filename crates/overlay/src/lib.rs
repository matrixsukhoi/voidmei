//! overlay: overlay 渲染与平台窗口层 (POC 语义复刻成果)。
//! 波10 分域: platform(窗口/托盘/热键/host) / render(canvas/fields/font/
//! palette/primitives) / overlays(组件 state 族, 原 overlays_field1/2 壳退役) /
//! layout(布局引擎+常量) / widgets(组件注册表+页面编排, W2+)。
//! 单一真相路径 (波16, 对齐 kernel 波9 原则): 全库唯一 `overlay::<域>::<模块>`
//! 访问, 根 re-export 壳已退役。ui_model 域已随 fields.grid 原子化退役。

// ---- 域模块 (5) ----
pub mod layout;
pub mod overlays;
pub mod platform;
pub mod render;
pub mod widgets;
