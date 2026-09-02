//! overlays_field1: FieldOverlay 系三 overlay 的内容复刻 (渲染函数级, 挂 OverlayHost 渲染闭包)
//! 重构波2: 内容已按组件拆至 overlay_*.rs (gauges/engine_control/power_info/gear_flaps),
//! 本文件保留为转发壳 (lib.rs 的 re-export 面与 tests 路径不变)。
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | [`PowerInfoState`] | ui/overlay/PowerInfoOverlay.java | BOS 字段网格: 常量表快照 (ui_layout.cfg "动力信息" 段) + FieldOverlay.onFlightData 50ms 节流 + 零 GC 更新路径 + BosStyleRenderer 绘制 |
//! | [`EngineControlState`] | ui/overlay/EngineControlOverlay.java | LabeledLinearGauge 条形仪表 (竖条 throttle/pitch/power + 横条 mixture/radiator/compressor/fuel), COMPRESSOR 走 MarkedGauge 画 optimal 档标记; onFlightData 节流间隔配置驱动 (loadRefreshInterval) |
//! | [`GearFlapsState`] | ui/overlay/GearFlapsOverlay.java | 襟翼竖条 (UIBaseElements.drawVBarTextNum) + 起落架/减速板状态告警文本; onFlightData 100ms 节流 |
//! | [`MarkedGauge`] 族 | ui/component/gauge/{MarkedGauge,GaugeBarStyle,GaugeMarker,MarkerType}.java | 条 + 可插拔标记系统 (LINE_FULL/LINE_PARTIAL/ZONE/TICK_LABELED) |
//!
//! 三者均为 "数据 struct + 内容绘制 fn" 形态: 上层把 state 与画布闭包捕获进
//! [`crate::host::OverlaySpec`] 的 render (FnMut(&mut PixCanvas)) 即挂入 OverlayHost;
//! 各 `*_overlay_spec` 工厂给出 live 喂入形态的现成闭包。
//!
//! 字段定义快照模式 (POC fields.rs 先例已随 W-D cfg 驱动化退役): ui_layout.cfg 对应 panel 段
//! 的 (item :type data ...) 逐行转常量表, 不运行时解析 cfg。
//!
//! 视觉语义逐项对照 Java paintComponent/drawTick/drawGauges; Java char[] 零 GC buffer
//! 统一为 String (gauges_bars 先例, 无 stale tail)。

pub use crate::overlay_engine_control::{
    engine_control_overlay_spec, EngineControlHandle, EngineControlState, EngineGauge,
    EngineGaugeDef, ENGINE_DISABLE_KEYS, ENGINE_GAUGE_DEFS, ENGINE_REFRESH_MULTIPLIER, GaugeType,
};
pub use crate::overlay_gauges::{GaugeBarStyle, GaugeMarker, MarkedGauge, MarkerType};
pub use crate::overlay_gear_flaps::{
    gear_flaps_overlay_spec, GearFlapsHandle, GearFlapsState, FIELD_OVERLAY_REFRESH_INTERVAL_MS,
    GEAR_FLAPS_REFRESH_INTERVAL_MS,
};
pub use crate::overlay_power_info::{power_info_overlay_spec, PowerInfoHandle, PowerInfoState};

// tests.rs 经 `use super::*` 消费的原文件头 use 转发面 (cfg(test) 免生产 unused)
#[cfg(test)]
use crate::font::LoadedFont;
#[cfg(test)]
use crate::global_colors::colors;
#[cfg(test)]
use crate::render2d::PixCanvas;
#[cfg(test)]
use crate::renderers::{BosStyleRenderer, RenderContext};
#[cfg(test)]
use crate::reinit::ReinitParams;
#[cfg(test)]
use crate::ui_constants::ENGINE_DEFAULT_REFRESH_MS;
#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::rc::Rc;
#[cfg(test)]
use vm_core::base::event::EventPayload;
#[cfg(test)]
use vm_core::lang::Lang;
// tests 直测基元 (butt_line 为 pub(crate) 项, cfg(test) 私有引入即可见)
#[cfg(test)]
use crate::overlay_gauges::butt_line;

#[cfg(test)]
mod tests;
