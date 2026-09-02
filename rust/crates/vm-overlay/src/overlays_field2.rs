//! overlays_field2: 操纵面十字指示 + FM 调试列表 (P4 批十 C 类语义复刻)
//! 重构波2: 内容已按组件拆至 overlay_*.rs (control_surfaces/fm_unpacked),
//! 本文件保留为转发壳 (lib.rs 的 re-export 面与 tests 路径不变)。
//!
//! | Rust | Java 源 | 语义要点 |
//! |---|---|---|
//! | [`ControlSurfacesOverlay`] | ui/overlay/ControlSurfacesOverlay.java | 副翼/升降舵/方向舵/可变翼位置: 边框+十字游标 (locater) + 4 行 BOS 标签 + 底部方向舵横条; 50ms 节流 |
//! | [`FmUnpackedDataOverlay`] | ui/overlay/FMUnpackedDataOverlay.java | FM 调试列表: BaseOverlay 斑马纹基座 + blkx 字段直读清单 (D4 砍反射段后的等价实现) |
//!
//! ControlSurfaces 窗口/拖动/FlightDataBus 注册归组装层 (LIFETIMES §2.1 注销链),
//! 组件承载 paintComponent 的绘制序与 onFlightData 的数据换算。
//!
//! FMUnpackedData 的 UIStateBus 订阅 (FM_OVERLAY_TOGGLE/FM_CHANGED) 对应
//! [`FmUnpackedDataOverlay::toggle`]/[`FmUnpackedDataOverlay::reload_fm_data`],
//! 由组装层的事件循环驱动 (vm-app win32 线程: 总线订阅转 channel → 循环内消费);
//! dispose 的退订由所有权 Drop 根治 (LIFETIMES §2.3), 无需显式方法。
//!
//! 对拍备案 (审查 W3): rustcmp 套件覆盖 FlightInfo/gauges/MiniHUD; FMUnpackedData
//! (ZebraList 首个生产消费者) 的渲染证据 = 单测级 oracle 色/几何 (WebLaF 离屏
//! 实测值, overlay_list tests) + 组件模块墨迹断言; rustcmp 场景面扩充随渲染对拍
//! 工具批另行安排。

pub use crate::overlay_control_surfaces::{
    control_surfaces_overlay_spec, ControlSurfacesHandle, ControlSurfacesOverlay, CsFonts,
    REFRESH_INTERVAL_MS,
};
pub use crate::overlay_fm_unpacked::{
    fm_unpacked_data_overlay_spec, FmUnpackedDataHandle, FmUnpackedDataOverlay, FmUnpackedFeed,
};
// draw_frame_simpl 经本模块路径消费 (原 pub(crate) 项, 保持 crate 内可见)
pub(crate) use crate::overlay_fm_unpacked::java_format_f;

// tests.rs 经 `use super::*` 消费的原文件头 use 转发面 + 直测的模块内符号
// (FmtArg/java_string_format/generate_lines/add_lines 为 pub(crate), cfg(test) 引入)
#[cfg(test)]
use crate::overlay_fm_unpacked::{add_lines, generate_lines, java_string_format, FmtArg};
#[cfg(test)]
use crate::font::LoadedFont;
#[cfg(test)]
use crate::global_colors::{aa, colors};
#[cfg(test)]
use crate::host::OverlayHost;
#[cfg(test)]
use crate::reinit::ReinitParams;
#[cfg(test)]
use crate::render2d::PixCanvas;
#[cfg(test)]
use std::rc::Rc;
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
use vm_core::config::config_api::ConfigProvider;
#[cfg(test)]
use vm_core::fm::FMManager;
#[cfg(test)]
use vm_core::fm::data::{FmData, FmParts};

#[cfg(test)]
mod tests;
