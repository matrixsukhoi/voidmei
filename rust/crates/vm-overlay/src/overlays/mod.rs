//! overlay 组件域 (波10 分域: 原 45 顶层平铺模块按五域归组, 组件剥 gauge_/overlay_
//! 前缀迁此)。每个组件 = "数据 struct + 内容绘制" 模块; W3 组件化后 host 挂载面
//! = widgets 域复合组件 (包本域 state), 各旧 `*_overlay_spec` 工厂已退役
//! (minihud 编排器工厂除外 — widgets::minihud 消费)。
//! 单一真相路径: 符号一律 `vm_overlay::overlays::<组件>::<符号>`, 域级转发面
//! (原 field1/2 壳残留) 已随波16 裁撤。

// ---- 仪表组件 ----
pub mod attitude; // 人工地平仪 (Java AttitudeOverlay/AttitudeIndicatorGauge)
pub mod bars; // 条形仪表族 (LinearGauge/SpeedRatioBar/FlapAngleBar)
pub mod compass; // 罗盘 (CompassGauge)
pub mod crosshair; // 十字准星 (CrosshairGauge)
pub mod gauges; // MarkedGauge 条+可插拔标记系统 (Field 系共用)

// ---- Field 系内容组件 (原 overlays_field1 壳: engine_control/gauges/gear_flaps) ----
pub mod engine_control;
pub mod gear_flaps;

// ---- Field 系内容组件 (原 overlays_field2 壳: control_surfaces;
// fm_unpacked 已随字段原子化退役 — 见 widgets/fm_field) ----
pub mod control_surfaces;

// ---- 列表/信息组件 ----
pub mod draw_frame_simpl; // FM 曲线可视化 (Java DrawFrameSimpl)
pub mod flight_info; // 飞行数据文本
pub mod minihud; // 主 HUD (组件化架构)
pub mod rows; // HUD 行原子组件 (HUDTextRow/AoaGauge/EnergyReadout/MechPart/ManeuverBar)
pub mod warning; // 告警闪烁 (WarningOverlay)

// ---- spec 工厂公共脚手架 (波15: 字体热换槽 + 键控 spec 构造) ----
pub(crate) mod spec_common;

// Field 系组件域级集成测试 (波10 合并原 field1/2 壳下 tests.rs; 先例 fm::store_tests)
#[cfg(test)]
mod fields_tests;
