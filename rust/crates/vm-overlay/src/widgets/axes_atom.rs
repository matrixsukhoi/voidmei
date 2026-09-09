//! core.axes.rudderbar — 方向舵横条原子组件 (舵面页拆解)。
//!
//! 原黑盒 core.axes.crosshair 的 4 行 BOS 标签 + 底部方向舵条拆出:
//! 标签行由 core.data.field 承担, 本组件承载横条 (drawHBarTextNum 复刻:
//! 边框横条 + 值游标竖线 + 值数字)。数据换算/节流自 ControlSurfacesOverlay
//! 摘出独立化: rudder_pix = (rudder+100)·width/200, 50ms 节流,
//! preview = 中位 50。字号 = round((24 + 组增量 + props.fontAdd) × dpi),
//! 组增量基准 = GaugeCfg.axis.0。

use vm_core::base::format;
use vm_core::base::format::java_round_f64;

use crate::layout::hud_layout_node::Dimension;
use crate::overlays::control_surfaces::{draw_h_bar_text_num, REFRESH_INTERVAL_MS};
use crate::render::canvas::PixCanvas;
use crate::render::font::LoadedFont;
use crate::render::palette::colors;

use super::env::{FactoryCtx, GaugeCfg, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::registry::PageFonts;
use super::registry::{HudWidget, PropKind, PropSchema, WidgetCategory, WidgetMeta};

/// 方向舵横条原子组件。原版几何 (ControlSurfacesOverlay.draw 底部条):
/// 条宽 width (十字边长 6fs 同口径)、条高 fs/2、数字基线 = 条底 + fs/2
/// (fontLabel 半字号)。
pub struct RudderBarWidget {
    font_size: i32,
    /// 条宽 = 6·fontSize (十字区边长同源)
    width: i32,
    font_label: LoadedFont,
    /// 50ms 节流基准
    last_refresh: i64,
    /// 游标像素位 ((rudder+100)·width/200)
    rudder_pix: i32,
    value_text: String,
}

fn f_rudder_bar(
    props: &serde_json::Value,
    fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    let cfg: GaugeCfg = fctx.gauge_cfg.cloned().unwrap_or_default();
    let font_add =
        cfg.axis.0 + props.get("fontAdd").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let font_size = java_round_f64((24.0 + font_add as f64) * cfg.dpi_scale);
    let fonts_dir = fctx
        .fonts_dir
        .as_deref()
        .ok_or("axes.rudderbar 需要 FactoryCtx.fonts_dir")?;
    let half = vm_core::base::format::java_round_f32(font_size as f32 / 2.0);
    let font_label = LoadedFont::new(&fonts_dir.join("sarasa-mono-sc-bold.ttf"), half)?;
    let width = 6 * font_size;
    let w = RudderBarWidget {
        font_size,
        width,
        font_label,
        last_refresh: 0,
        // preview 初值: 中位 50 ((50+100)·width/200)
        rudder_pix: (50 + 100) * width / 200,
        value_text: format::format(50.0, 0),
    };
    Ok(Box::new(w))
}

impl RudderBarWidget {
    /// 测试断言面: 游标像素位
    pub fn rudder_pix(&self) -> i32 {
        self.rudder_pix
    }

    /// 测试断言面: 值文本
    pub fn value_text(&self) -> &str {
        &self.value_text
    }
}

impl HudWidget for RudderBarWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {
        // 预览初值 (中位 50) 已由构造落位
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // preview (frame 缺席) 保持静态 — 对位 Java initPreview 不订阅
        let Some(frame) = env.frame else { return };
        // 节流防高频事件任务堆积 (ControlSurfaces REFRESH_INTERVAL_MS)
        if env.now_ms - self.last_refresh < REFRESH_INTERVAL_MS {
            return;
        }
        self.last_refresh = env.now_ms;
        // (int) 截断 ±100 域遥测 → 游标位 + 整数格式化 (update_flight_data 同)
        let val = frame.var_value("rudder").unwrap_or(0.0) as i32;
        self.rudder_pix = (val + 100) * self.width / 200;
        self.value_text = format::format(val as f64, 0);
    }

    fn reset_preview(&mut self) {
        // 数据面回构造初值 (中位 50 + 节流基准归零; Java 新实例等价)
        self.rudder_pix = (50 + 100) * self.width / 200;
        self.value_text = format::format(50.0, 0);
        self.last_refresh = 0;
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        // 原调用点: drawHBarTextNum(g2d, 0, height, width, fontSize>>1,
        // rudderValPix, 1, colorNum, lbl, num, fontLabel, fontLabel)
        draw_h_bar_text_num(
            cv,
            &self.font_label,
            &self.font_label,
            x,
            y,
            self.width,
            self.font_size >> 1,
            self.rudder_pix,
            1,
            colors().num,
            &self.value_text,
            aa,
        );
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        // 原底部条区: 宽 6fs (条 + 游标数字), 高 1.5fs (条 fs/2 + 数字基线 fs)
        Dimension::new(self.width, self.width / 4)
    }
}

/// 舵面原子组件注册表 (palette 展示序)
pub(super) const REGISTRY_ENTRIES: &[WidgetMeta] = &[WidgetMeta {
    type_name: "core.axes.rudderbar",
    display_zh: "方向舵横条",
    category: WidgetCategory::Gauge,
    composite: false,
    props_schema: &[PropSchema {
        key: "fontAdd",
        display_zh: "字号增量",
        kind: PropKind::Int,
    }],
    config_keys: &["fontSize"],
    data_shorts: &["rudder"],
    factory: f_rudder_bar,
}];
