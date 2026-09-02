//! overlay 组件域 (波10 分域: 原 45 顶层平铺模块按五域归组, 组件剥 gauge_/overlay_
//! 前缀迁此)。每个组件 = "数据 struct + 内容绘制" 模块, 经 [`crate::platform::host`]
//! 的 OverlaySpec 挂入窗口宿主; `*_overlay_spec` 工厂给出 live 形态闭包。
//! 域级符号 re-export 面 = 原 overlays_field1/field2 壳的转发面 (壳已退役)。

// ---- 仪表组件 ----
pub mod attitude;     // 人工地平仪 (Java AttitudeOverlay/AttitudeIndicatorGauge)
pub mod compass;      // 罗盘 (CompassGauge)
pub mod crosshair;    // 十字准星 (CrosshairGauge)
pub mod bars;         // 条形仪表族 (LinearGauge/SpeedRatioBar/FlapAngleBar)
pub mod gauges;       // MarkedGauge 条+可插拔标记系统 (Field 系共用)

// ---- Field 系内容组件 (原 overlays_field1 壳: engine_control/gauges/gear_flaps/power_info) ----
pub mod engine_control;
pub mod gear_flaps;
pub mod power_info;

// ---- Field 系内容组件 (原 overlays_field2 壳: control_surfaces/fm_unpacked) ----
pub mod control_surfaces;
pub mod fm_unpacked;

// ---- 列表/信息组件 ----
pub mod list;             // 斑马纹列表基座 (BaseListOverlay/ZebraList)
pub mod warning;          // 告警闪烁 (WarningOverlay)
pub mod flight_info;      // 飞行数据文本
pub mod draw_frame_simpl; // FM 曲线可视化 (Java DrawFrameSimpl)
pub mod minihud;          // 主 HUD (组件化架构)
pub mod rows;             // HUD 行组件 (HUDTextRow/HUDAkbRow/HUDEnergyRow/HUDManeuverRow)

// 原 field1 壳转发面 (lib.rs 根 re-export 与 fields_tests 的取数面)
pub use engine_control::{
    engine_control_overlay_spec, EngineControlHandle, EngineControlState, EngineGauge,
    EngineGaugeDef, ENGINE_DISABLE_KEYS, ENGINE_GAUGE_DEFS, ENGINE_REFRESH_MULTIPLIER, GaugeType,
};
pub use gauges::{GaugeBarStyle, GaugeMarker, MarkedGauge, MarkerType};
pub use gear_flaps::{
    gear_flaps_overlay_spec, GearFlapsHandle, GearFlapsState, FIELD_OVERLAY_REFRESH_INTERVAL_MS,
    GEAR_FLAPS_REFRESH_INTERVAL_MS,
};
pub use power_info::{power_info_overlay_spec, PowerInfoHandle, PowerInfoState};
// 原 field2 壳转发面
pub use control_surfaces::{
    control_surfaces_overlay_spec, ControlSurfacesHandle, ControlSurfacesOverlay, CsFonts,
    REFRESH_INTERVAL_MS,
};
pub use fm_unpacked::{
    fm_unpacked_data_overlay_spec, FmUnpackedDataHandle, FmUnpackedDataOverlay, FmUnpackedFeed,
};

// ---- spec 工厂公共脚手架 (波15: 字体热换槽 + 键控 spec 构造) ----
pub(crate) mod spec_common;

// Field 系组件域级集成测试 (波10 合并原 field1/2 壳下 tests.rs; 先例 fm::store_tests)
#[cfg(test)]
mod fields_tests;
