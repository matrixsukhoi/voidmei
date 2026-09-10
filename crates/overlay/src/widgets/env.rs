//! 组件环境 (W3 泛化: MiniHUD 族 HUDData + 通用短名面 + FM/payload 杂项)。

use std::rc::Rc;

use kernel::base::event::event_payload::EventPayload;
use kernel::config::config_api::HudSettingsSnapshot;
use kernel::derived::hud_data::HUDData;
use kernel::fm::data::FmData;
use kernel::formula::registry::FormulaView;
use kernel::lang::Lang;

use crate::overlays::minihud::{MinimalHudContext, MiniHudFonts};
use crate::overlays::rows::TickScale;

/// 风格注入环境 (原 applyStyleToComponents 各组件自取面的收敛)
pub struct StyleEnv<'a> {
    pub fonts: Rc<MiniHudFonts>,
    pub settings: &'a HudSettingsSnapshot,
    /// MiniHUD 派生上下文 (仅 minihud 族组件消费; W3 页面为 None)
    pub minihud_ctx: Option<&'a MinimalHudContext>,
}

/// preview 模板与静态值 (原 refreshTemplates + update_row_values 的组件推送面)
#[derive(Clone)]
pub struct MiniHudTemplates {
    /// 行模板 (槽序 speed/alt/mech/sep/gload; 槽 2 = 机械化三段旧格式串,
    /// 各 MechPart 独立解析本段)
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

/// 数据更新环境 (组件自取所需; frame = 通用短名面, None = preview)
pub struct UpdateEnv<'a> {
    /// MiniHUD 页面派生 (W3 页面传 default)
    pub data: &'a HUDData,
    /// 通用取数面 (var_value 短名; preview = None 组件走静态值)
    pub frame: Option<&'a dyn FormulaView>,
    pub fmdata: Option<&'a FmData>,
    pub payload: Option<&'a EventPayload>,
    /// 引擎控制: compressorStages 档位数 (FMManager.current 的 len)
    pub compressor_stages: Option<i32>,
    pub now_ms: i64,
    /// 机动条会话量 (maneuverIndexLen/TickScale — 编排器持有)
    pub maneuver_len: i32,
    pub maneuver_ticks: TickScale,
    /// 起落襟翼本地化文案源
    pub lang: Option<&'a Lang>,
}

impl<'a> UpdateEnv<'a> {
    /// preview 形态 (frame/fmdata 缺席, 组件走静态值)
    pub fn preview(now_ms: i64, data: &'a HUDData) -> Self {
        UpdateEnv {
            data,
            frame: None,
            fmdata: None,
            payload: None,
            compressor_stages: None,
            now_ms,
            maneuver_len: 0,
            maneuver_ticks: TickScale::default(),
            lang: None,
        }
    }

    /// 短名取值速记 (None = preview → 0.0)
    pub fn val(&self, name: &str) -> f64 {
        self.frame
            .and_then(|f| f.var_value(name))
            .unwrap_or(0.0)
    }
}

/// W3B 图形族复合组件 (engine/gearflaps/axes/attitude) 与 FM 黑盒组件的
/// reinit 参数快照 (源 = ReinitParams 各组; 页面编排器随 refresh 闭包注入,
/// 缺席 = preview/测试走 [`GaugeCfg::default`] 的 Java 回退缺省)。
/// R4 字号合一: 组字号增量 (engine/gear/axis/fm) 全部退役 — 组件字号 =
/// 页面字号 (FactoryCtx.fonts.draw.size, 即 24 + doc.font.size_add, dpi 后)
/// + props.fontAdd; 本结构只剩几何/节流/边缘开关
#[derive(Debug, Clone, PartialEq)]
pub struct GaugeCfg {
    /// Application.dpiScale (DPI 几何换算共用)
    pub dpi_scale: f64,
    /// Service 轮询间隔 (引擎控制 loadRefreshInterval 的 dataPollIntervalMs 源)
    pub service_loop_interval_ms: i64,
    /// 起落襟翼组边缘开关
    pub gear_show_edge: bool,
    /// 操纵面组边缘开关
    pub axis_show_edge: bool,
    /// 地平仪组 (宽, 高, 航向指针, 攻角极限线)
    pub attitude: (i32, i32, bool, bool),
    /// 地平仪数据节流 ms (attitudeIndicatorFreqMs)
    pub attitude_freq_ms: i64,
    /// 屏幕逻辑高 (FM 列表高度自适应的钳制上限)
    pub logical_height: i32,
}

impl Default for GaugeCfg {
    /// 缺省 = ReinitParams::default 同源的 Java 回退值
    fn default() -> Self {
        GaugeCfg {
            dpi_scale: 1.0,
            service_loop_interval_ms: 50,
            gear_show_edge: false,
            axis_show_edge: false,
            attitude: (150, 300, false, true),
            attitude_freq_ms: 40,
            logical_height: 1080,
        }
    }
}

impl GaugeCfg {
    /// ReinitParams → GaugeCfg 组装 (真窗注册面 / refresh 闭包 / 编辑器快照
    /// 三处同源的收敛点)。logical_height 由调用方传入 (env.dpi 真值)
    pub fn from_params(
        p: &crate::platform::reinit::ReinitParams,
        dpi: f64,
        logical_height: i32,
    ) -> Self {
        GaugeCfg {
            dpi_scale: dpi,
            service_loop_interval_ms: p.service_loop_interval_ms,
            gear_show_edge: p.gear.show_edge,
            axis_show_edge: p.axis.show_edge,
            attitude: (
                p.attitude.width,
                p.attitude.height,
                p.attitude.show_direction,
                p.attitude.show_aoa_limits,
            ),
            attitude_freq_ms: p.attitude_freq_ms,
            logical_height,
        }
    }
}

/// 工厂环境 (组件构造所需的页面派生量)
pub struct FactoryCtx<'a> {
    /// MiniHUD 派生上下文 (仅 minihud 族组件; W3 页面 None)
    pub minihud_ctx: Option<&'a MinimalHudContext>,
    pub fonts: Rc<MiniHudFonts>,
    /// 本地化文案源 (起落襟翼等)
    pub lang: Option<&'a Lang>,
    /// 字体目录 (data.field 等自管字体加载)
    pub fonts_dir: Option<std::path::PathBuf>,
    /// W3B/W3C 复合组件参数快照 (缺省 None → GaugeCfg::default)
    pub gauge_cfg: Option<&'a GaugeCfg>,
}
