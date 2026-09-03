//! 组件环境 (W2: MiniHUD 族 — fonts/模板/风格派生量;
//! W3 泛化: frame 数据面与 config 视图并入)。

use std::rc::Rc;

use vm_core::config::config_api::HudSettingsSnapshot;
use vm_core::derived::hud_data::HUDData;

use crate::overlays::minihud::{MinimalHudContext, MiniHudFonts};
use crate::overlays::rows::TickScale;

/// 风格注入环境 (原 applyStyleToComponents 各组件自取面的收敛)
pub struct StyleEnv<'a> {
    pub fonts: Rc<MiniHudFonts>,
    pub settings: &'a HudSettingsSnapshot,
    pub ctx: &'a MinimalHudContext,
}

/// preview 模板与静态值 (原 refreshTemplates + update_row_values 的组件推送面;
/// overlay 级只写字段 (throttley 等死字段) 不在组件模板内)
#[derive(Clone)]
pub struct MiniHudTemplates {
    /// 行模板 [row0..row4] (row2 = 机械化三段旧格式串)
    pub lines: [String; 5],
    pub line_aoa: String,
    pub rel_energy: String,
    /// preview row0 的 aoa_y (init 钳 rightDraw 后值)
    pub aoa_y: i32,
    pub aoa_color: [u8; 4],
    pub aoa_bar_color: [u8; 4],
    /// row2 预览入参 (Java inAction 恒 false)
    pub in_action: bool,
    /// preview throttle (update_components 的 service=None 分支值)
    pub throttle: i32,
    /// row4 预览机动条 (maneuver_index, len, ticks)
    pub maneuver: (f64, i32, TickScale),
}

/// 数据更新环境 (W3 并入 frame/config 视图后取代裸 HUDData)
pub struct UpdateEnv<'a> {
    pub data: &'a HUDData,
    /// 机动条会话量 (maneuverIndexLen/TickScale — 编排器持有)
    pub maneuver_len: i32,
    pub maneuver_ticks: TickScale,
}

/// 工厂环境 (组件构造所需的页面派生量)
pub struct FactoryCtx<'a> {
    pub ctx: &'a MinimalHudContext,
    pub fonts: Rc<MiniHudFonts>,
}
