//! core.gearflaps.flapbar / core.gearflaps.warn — 起落襟翼页原子组件 (拆解)。
//!
//! 原黑盒 core.gearflaps.status (GearFlapsState 整窗包装) 退役: 襟翼竖条与
//! 起落架/减速板告警文本各自独立摆位。数据换算/节流/preview 语义自
//! GearFlapsState 摘出独立化 (同源 GearFlapsOverlay.onFlightData):
//! 100ms 节流; preview = 襟翼 50% 无告警; gear<0 (无数据) 保留上次告警。
//! 字号 = round((24 + 组增量 + props.fontAdd) × dpi), 组增量基准
//! = GaugeCfg.gear.0 (页面主字号同源)。

use vm_core::base::format::java_round_f64;
use vm_core::lang::Lang;

use crate::layout::hud_layout_node::Dimension;
use crate::overlays::gear_flaps::draw_v_bar_text_num;
use crate::render::canvas::PixCanvas;
use crate::render::font::LoadedFont;
use crate::render::palette::colors;

use super::env::{FactoryCtx, GaugeCfg, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::registry::PageFonts;
use super::registry::{HudWidget, PropKind, PropSchema, WidgetCategory, WidgetMeta};

/// 节流间隔 (gear/flaps 为低频数据; GearFlapsOverlay REFRESH_INTERVAL_MS)
pub const GEAR_FLAPS_REFRESH_INTERVAL_MS: i64 = 100;

/// props.fontAdd 解析 (字号增量; 缺省 0)
fn font_add_of(props: &serde_json::Value) -> i32 {
    props.get("fontAdd").and_then(|v| v.as_i64()).unwrap_or(0) as i32
}

/// 字号 = 页面字号 (R4 字号合一: 组增量退役, 页 doc.font.size_add + dpi 已含)
/// + props 增量 × dpi
fn font_size_of(fctx: &FactoryCtx, cfg: &GaugeCfg, props: &serde_json::Value) -> i32 {
    fctx.fonts.draw.size + java_round_f64(font_add_of(props) as f64 * cfg.dpi_scale)
}

/// BOLD 字体路径 (各旧工厂同款: fonts_dir/sarasa-mono-sc-bold.ttf)
fn bold_path(fctx: &FactoryCtx) -> Result<std::path::PathBuf, String> {
    Ok(fctx
        .fonts_dir
        .as_deref()
        .ok_or("gearflaps 组件需要 FactoryCtx.fonts_dir")?
        .join("sarasa-mono-sc-bold.ttf"))
}

// =====================================================================
// core.gearflaps.flapbar — 襟翼竖条 + F 数值
// =====================================================================

/// 襟翼竖条原子组件。原版几何 (GearFlapsState 内容区):
/// 条宽 fs/2、条长 4fs、条底在 (fs>>1)+4fs 处 (顶部留 fs/2), 数值 "F%3d"
/// 随值指针基线 = 条底 - val_h - 2。
pub struct FlapBarWidget {
    font_size: i32,
    bar_width: i32,
    bar_height: i32,
    font_num: LoadedFont,
    /// 100ms 节流基准
    last_refresh: i64,
    /// 襟翼填充像素高
    flap_pix: i32,
    /// 襟翼百分比文本 (Java "%3d")
    flap_text: String,
}

fn f_flap_bar(props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let cfg: GaugeCfg = fctx.gauge_cfg.cloned().unwrap_or_default();
    let font_size = font_size_of(fctx, &cfg, props);
    let font_num = LoadedFont::new(&bold_path(fctx)?, font_size)?;
    let bar_height = 4 * font_size;
    // preview 初值: 襟翼 50%
    let w = FlapBarWidget {
        font_size,
        bar_width: font_size >> 1,
        bar_height,
        font_num,
        last_refresh: 0,
        flap_pix: bar_height * 50 / 100,
        flap_text: format!("{:>3}", 50),
    };
    Ok(Box::new(w))
}

impl FlapBarWidget {
    /// 测试断言面: 襟翼填充像素高
    pub fn flap_pix(&self) -> i32 {
        self.flap_pix
    }

    /// 测试断言面: 襟翼百分比文本
    pub fn flap_text(&self) -> &str {
        &self.flap_text
    }
}

impl HudWidget for FlapBarWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {
        // 预览初值 (襟翼 50%) 已由构造落位
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // preview (frame 缺席) 保持静态 — 对位 Java initPreview 不订阅
        let Some(frame) = env.frame else { return };
        // 节流防高频事件任务堆积
        if env.now_ms - self.last_refresh < GEAR_FLAPS_REFRESH_INTERVAL_MS {
            return;
        }
        self.last_refresh = env.now_ms;
        // (int) 截断; flaps<0 (无数据) 归零显示 (GearFlapsState 同)
        let mut flaps = frame.var_value("flaps").unwrap_or(0.0) as i32;
        if flaps >= 0 {
            self.flap_pix = flaps * self.bar_height / 100;
        } else {
            self.flap_pix = 0;
            flaps = 0;
        }
        self.flap_text = format!("{:>3}", flaps);
    }

    fn reset_preview(&mut self) {
        // 数据面回构造初值 (襟翼 50% + 节流基准归零; Java 新实例等价)
        self.flap_pix = self.bar_height * 50 / 100;
        self.flap_text = format!("{:>3}", 50);
        self.last_refresh = 0;
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        let num = format!("F{}", self.flap_text);
        // 条底基点 = 组件内 (fs>>1)+4fs (原内容区几何换算: 顶部留 fs/2)
        draw_v_bar_text_num(
            cv,
            x,
            y + (self.font_size >> 1) + self.bar_height,
            self.bar_width,
            self.bar_height,
            self.flap_pix,
            1,
            colors().num,
            "",
            &num,
            &self.font_num,
            &self.font_num,
            aa,
        );
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        // 原内容区量级: 宽 2fs (条 + 数值列), 高 5fs (fs/2 顶距 + 4fs 条)
        Dimension::new(2 * self.font_size, 5 * self.font_size)
    }
}

// =====================================================================
// core.gearflaps.warn — 起落架/减速板告警文本
// =====================================================================

/// 告警文本原子组件。原版几何: 文本在内容区 (width=2fs, 基线 fs) 处,
/// fontLabel (BOLD fs/2), 无阴影; 文案/颜色 = gear/airbrake 状态机
/// (gear=100 已放 / 0<gear<100 收起中 / airbrake>0 追加减速板 → 警告色)。
pub struct GearWarnWidget {
    font_size: i32,
    font_label: LoadedFont,
    /// 100ms 节流基准
    last_refresh: i64,
    warn_text: String,
    warn_color: [u8; 4],
}

fn f_gear_warn(props: &serde_json::Value, fctx: &FactoryCtx) -> Result<Box<dyn HudWidget>, String> {
    let cfg: GaugeCfg = fctx.gauge_cfg.cloned().unwrap_or_default();
    let font_size = font_size_of(fctx, &cfg, props);
    let half = vm_core::base::format::java_round_f32(font_size as f32 / 2.0);
    let font_label = LoadedFont::new(&bold_path(fctx)?, half)?;
    let w = GearWarnWidget {
        font_size,
        font_label,
        last_refresh: 0,
        warn_text: String::new(), // preview 无告警
        warn_color: colors().num,
    };
    Ok(Box::new(w))
}

impl GearWarnWidget {
    /// 测试断言面: 告警文本
    pub fn warn_text(&self) -> &str {
        &self.warn_text
    }

    /// 测试断言面: 告警颜色
    pub fn warn_color(&self) -> [u8; 4] {
        self.warn_color
    }
}

impl HudWidget for GearWarnWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {
        // 预览初值 (无告警) 已由构造落位
    }

    fn on_data_update(&mut self, env: &UpdateEnv) {
        let (Some(frame), Some(lang)) = (env.frame, env.lang) else {
            return; // preview 保持静态 (无告警)
        };
        // 节流防高频事件任务堆积
        if env.now_ms - self.last_refresh < GEAR_FLAPS_REFRESH_INTERVAL_MS {
            return;
        }
        self.last_refresh = env.now_ms;
        // 状态机 (GearFlapsState.update_tick 同源); gear<0 保留上次告警
        self.update_warn(frame, lang);
    }

    fn reset_preview(&mut self) {
        // 数据面回构造初值 (告警清空 + 节流基准归零; Java 新实例等价)
        self.warn_text.clear();
        self.warn_color = colors().num;
        self.last_refresh = 0;
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        // 告警文本: 基线 fs, fontLabel, 无阴影; 空串绘制无输出 (等价无绘制)
        cv.draw_text(
            &self.font_label,
            x,
            y + self.font_size,
            &self.warn_text,
            self.warn_color,
            aa,
        );
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        // 文本带 (基线 fs + 下降余量); 宽 = 告警极值 "起落架 减速板" (半字号
        // 7 全角 + 1 空格 ≈ 4fs) 的裕量口径
        Dimension::new(4 * self.font_size, self.font_size + self.font_size / 2)
    }
}

impl GearWarnWidget {
    /// gear/airbrake → 告警文本/颜色 (GearFlapsState.update_tick 的告警段)
    fn update_warn(&mut self, s: &dyn vm_core::formula::registry::FormulaView, lang: &Lang) {
        // Java (int) 强转截断; 值域 0..100
        let gear = s.var_value("gear").unwrap_or(0.0) as i32;
        let airbrake = s.var_value("airbrake").unwrap_or(0.0) as i32;
        if gear >= 0 {
            if gear == 0 {
                self.warn_text.clear();
                self.warn_color = colors().num;
            } else if gear == 100 {
                self.warn_text = lang.g_gear.to_string();
                self.warn_color = colors().num;
            } else {
                self.warn_text = lang.g_gear_down.to_string();
                self.warn_color = colors().warning;
            }
            if airbrake > 0 {
                self.warn_text.push(' ');
                self.warn_text.push_str(lang.g_brake);
                self.warn_color = colors().warning;
            }
        }
        // gear < 0 (无数据): 保留上次告警状态 (Java 同)
    }
}

// =====================================================================
// 注册表
// =====================================================================

const KEYS: &[&str] = &["fontSize"];

const FONT_ADD_PROP: PropSchema = PropSchema {
    key: "fontAdd",
    display_zh: "字号增量",
    kind: PropKind::Int,
};

/// 起落襟翼原子组件注册表 (palette 展示序)
pub(super) const REGISTRY_ENTRIES: &[WidgetMeta] = &[
    WidgetMeta {
        type_name: "core.gearflaps.flapbar",
        display_zh: "襟翼竖条",
        category: WidgetCategory::Gauge,
        composite: false,
        props_schema: &[FONT_ADD_PROP],
        config_keys: KEYS,
        data_shorts: &["flaps"],
        default_props: "{}",
        factory: f_flap_bar,
    },
    WidgetMeta {
        type_name: "core.gearflaps.warn",
        display_zh: "起落架告警",
        category: WidgetCategory::Text,
        composite: false,
        props_schema: &[FONT_ADD_PROP],
        config_keys: KEYS,
        data_shorts: &["gear", "airbrake"],
        default_props: "{}",
        factory: f_gear_warn,
    },
];
