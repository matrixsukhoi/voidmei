//! vm-overlay: overlay 渲染与平台窗口层 (POC 语义复刻成果)。
//! 波10 分域: platform(窗口/托盘/热键/host) / render(canvas/fields/renderers/font/
//! palette/primitives) / overlays(~17 组件, 原 overlays_field1/2 壳退役) /
//! layout(布局引擎+常量) / ui_model(数据字段模型)。
//! 根 re-export 面保持既有符号路径 (外部消费 `vm_overlay::X` 零感知)。

// ---- 域模块 (5) ----
pub mod layout;
pub mod overlays;
pub mod platform;
pub mod render;
pub mod ui_model;

// ---- 根 re-export 面 (既有公共 API, 指向分域后位置) ----
pub use layout::hud_layout_node;
pub use layout::minihud_layout::{
    build_mihud_layout, debug_frame_color, java_string_hashcode, AutoSizingPlan,
    BuiltMiniHudLayout, CfgDefault, HasVisibility, MiniHudComp, MiniHudCfgItem, MiniHudItemType,
    MiniHudLayoutConfig, MiniHudNodeSpec, MiniHudParts, ModernHUDLayoutEngine,
    ENABLE_LAYOUT_DEBUG_ITEM, LAYOUT_PADDING, MINIHUD_NODE_SPECS, MINIHUD_PANEL_ITEMS,
};
pub use overlays::attitude::{
    attitude_overlay_spec, AttitudeIndicatorGauge, AttitudeOverlay, AttitudeOverlayHandle,
};
pub use overlays::bars::{FlapAngleBar, LabeledLinearGauge, LinearGauge, SpeedRatioBar};
pub use overlays::compass::CompassGauge;
pub use overlays::control_surfaces::{
    control_surfaces_overlay_spec, ControlSurfacesHandle, ControlSurfacesOverlay, CsFonts,
    REFRESH_INTERVAL_MS,
};
pub use overlays::draw_frame_simpl::{
    draw_frame_simpl_spec, DfsFlight, DrawFrameSimplFeed, DrawFrameSimplHandle, DrawFrameSimpl,
};
pub use overlays::engine_control::{
    engine_control_overlay_spec, EngineControlHandle, EngineControlState, EngineGauge,
    EngineGaugeDef, ENGINE_DISABLE_KEYS, ENGINE_GAUGE_DEFS, ENGINE_REFRESH_MULTIPLIER, GaugeType,
};
pub use overlays::flight_info::{build_texts, flight_info_overlay_spec, FlightInfoHandle};
pub use overlays::fm_unpacked::{
    fm_unpacked_data_overlay_spec, FmUnpackedDataHandle, FmUnpackedDataOverlay, FmUnpackedFeed,
};
pub use overlays::gear_flaps::{
    gear_flaps_overlay_spec, GearFlapsHandle, GearFlapsState, FIELD_OVERLAY_REFRESH_INTERVAL_MS,
    GEAR_FLAPS_REFRESH_INTERVAL_MS,
};
pub use overlays::list::{BaseListOverlay, ZebraList};
pub use overlays::minihud::{
    minihud_overlay_spec, CompCell, MiniHudComponent, MiniHudComponentInner, MiniHudFonts,
    MiniHudHandle, MiniHudOverlay, MinimalHudContext,
};
pub use overlays::power_info::{power_info_overlay_spec, PowerInfoHandle, PowerInfoState};
pub use overlays::rows::{HUDAkbRow, HUDEnergyRow, HUDManeuverRow, HUDTextRow};
pub use overlays::warning::{WarningBlinkHost, WarningOverlay};
pub use platform::extras::{parse_wav_duration, DpiHelper};
pub use platform::hotkey::{
    vk_to_vc, ChannelHotkeySink, HotkeyEvent, HotkeyEventSink, HotkeyManager, VC_CAPS_LOCK,
    VC_NUM_LOCK, VC_P, VC_SCROLL_LOCK, VC_UNDEFINED,
};
pub use platform::host::{DialogHooks, OverlayEntry, OverlayHost, OverlaySpec, RenderFn, ReinitFn, WindowFactory};
pub use platform::position::{load_pos, save_pos};
pub use platform::reinit::ReinitParams;
#[cfg(target_os = "windows")]
pub use platform::tray::{TrayConfig, TrayHandler, TrayIcon};
pub use render::canvas::{to_premul_bgra, LineCapStyle, PixCanvas};
pub use render::fields::{draw_fields, render_fields, render_fields_fixed, FieldText, FontTriple, RenderColors, DEFAULT_COLORS};
pub use render::palette;
pub use render::renderers::{
    BosStyleRenderer, Field, OverlayRenderer, RenderContext, RenderPalette, TextGauge,
    APPLICATION_COLORS, WHITE,
};
