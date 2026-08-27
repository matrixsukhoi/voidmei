//! vm-overlay: overlay 渲染与平台窗口层 (POC 语义复刻成果)
pub mod compare;
pub mod config;
pub mod flight_info;
pub mod font;
pub mod gauge_attitude;
pub mod global_colors;
pub mod gauge_compass;
pub mod gauge_crosshair;
pub mod gauges_bars;
pub mod host;
pub mod hotkey;
pub mod minihud;
pub mod minihud_layout;
pub mod overlay_list;
pub mod overlays_field1;
pub mod overlays_field2;
pub mod parity_gauges;
pub mod parity_minihud;
pub mod platform;
pub mod platform_extras;
pub mod reinit;
pub mod render;
pub mod render2d;
pub mod renderers;
pub mod rows;
#[cfg(target_os = "windows")]
pub mod tray;
pub mod warning_overlay;
pub mod window;

pub use config::{load_pos, save_pos};
pub use gauge_attitude::{
    attitude_overlay_spec, AttitudeIndicatorGauge, AttitudeOverlay, AttitudeOverlayHandle,
};
pub use gauge_compass::CompassGauge;
pub use gauge_crosshair::CrosshairGauge;
pub use gauges_bars::{FlapAngleBar, LabeledLinearGauge, LinearGauge, SpeedRatioBar};
pub use host::{DialogHooks, OverlayEntry, OverlayHost, OverlaySpec, RenderFn, ReinitFn, WindowFactory};
pub use reinit::ReinitParams;
pub use hotkey::{
    vk_to_vc, ChannelHotkeySink, HotkeyEvent, HotkeyEventSink, HotkeyManager, VC_CAPS_LOCK,
    VC_NUM_LOCK, VC_P, VC_SCROLL_LOCK, VC_UNDEFINED,
};
pub use minihud::{
    minihud_overlay_spec, CompCell, MiniHudComponent, MiniHudComponentInner, MiniHudFonts,
    MiniHudHandle, MiniHudOverlay, MinimalHudContext,
};
pub use minihud_layout::{
    build_mihud_layout, debug_frame_color, java_string_hashcode, AutoSizingPlan,
    BuiltMiniHudLayout, CfgDefault, HasVisibility, MiniHudComp, MiniHudCfgItem, MiniHudItemType,
    MiniHudLayoutConfig, MiniHudNodeSpec, MiniHudParts, ModernHUDLayoutEngine,
    ENABLE_LAYOUT_DEBUG_ITEM, LAYOUT_PADDING, MINIHUD_NODE_SPECS, MINIHUD_PANEL_ITEMS,
};
pub use overlay_list::{BaseListOverlay, ZebraList};
pub use flight_info::{build_texts_from_values, flight_info_overlay_spec, FlightInfoHandle};
pub use overlays_field1::{
    engine_control_overlay_spec, engine_control_preview_spec, gear_flaps_overlay_spec,
    ENGINE_DISABLE_KEYS,
    gear_flaps_preview_spec, power_info_overlay_spec, power_info_preview_spec, DynSource,
    EngineControlHandle, EngineControlState, EngineGauge, EngineGaugeDef, GaugeBarStyle,
    GaugeMarker, GaugeType, GearFlapsHandle, GearFlapsState, MarkedGauge, MarkerType,
    PowerFieldDef, PowerFormat, PowerInfoHandle, PowerInfoState, PowerSource, VisExpr,
    ENGINE_GAUGE_DEFS, ENGINE_REFRESH_MULTIPLIER, FIELD_OVERLAY_REFRESH_INTERVAL_MS,
    GEAR_FLAPS_REFRESH_INTERVAL_MS, POWER_FIELD_DEFS,
};
pub use overlays_field2::{
    control_surfaces_overlay_spec, ControlSurfacesHandle, ControlSurfacesOverlay, CsFonts,
    FmUnpackedDataOverlay, REFRESH_INTERVAL_MS,
};
pub use platform_extras::{parse_wav_duration, DpiHelper};
pub use render::{draw_fields, render_fields, render_fields_fixed, FieldText, FontTriple, RenderColors, DEFAULT_COLORS};
pub use render2d::{LineCapStyle, PixCanvas};
pub use renderers::{
    BosStyleRenderer, Field, LinearGaugeRenderer, OverlayRenderer, RenderContext, RenderPalette, TextGauge,
    TextOnlyRenderer, APPLICATION_COLORS, WHITE,
};
pub use rows::{HUDAkbRow, HUDEnergyRow, HUDFlapsRow, HUDManeuverRow, HUDTextRow};
#[cfg(target_os = "windows")]
pub use tray::{TrayConfig, TrayHandler, TrayIcon};
pub use warning_overlay::{WarningBlinkHost, WarningOverlay};
pub use window::{run, run_live, OverlayMode};
