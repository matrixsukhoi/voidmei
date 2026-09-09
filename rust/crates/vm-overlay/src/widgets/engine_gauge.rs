//! core.engine.gauge — 单引擎仪表原子组件 (引擎控制面板拆解)。
//!
//! 每仪表一实例 (props.kind 定 7 种之一): 数据取值/jet 门控/节流/compressor
//! 量程与 optimal 标记逻辑自 EngineControlState 摘出独立化; 隐藏 (jet 机型/
//! 无桨距/0 增压档) 时 preferred 归零 = 链式排布自动补位 (原面板 draw 跳过
//! 隐藏仪表的 dx/dy 推进语义)。面板级 disable 开关由"组件在不在页里"表达。

use vm_core::base::format;
use vm_core::lang::Lang;

use crate::layout::hud_layout_node::Dimension;
use crate::overlays::engine_control::{EngineGaugeDef, ENGINE_GAUGE_DEFS, GaugeType};
use crate::overlays::bars::LabeledLinearGauge;
use crate::overlays::gauges::{GaugeBarStyle, GaugeMarker, MarkerType, MarkedGauge};
use crate::render::canvas::PixCanvas;
use crate::render::font::LoadedFont;
use crate::render::palette::colors;

use super::env::{FactoryCtx, GaugeCfg, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::registry::PageFonts;
use super::registry::{HudWidget, PropKind, PropSchema, WidgetCategory, WidgetMeta};

/// 单引擎仪表
pub struct EngineGaugeWidget {
    def: &'static EngineGaugeDef,
    gauge: LabeledLinearGauge,
    /// COMPRESSOR 专用 (optimal 档标记)
    marked: Option<MarkedGauge>,
    visible: bool,
    /// 字号 (24+add)×dpi (构造期定)
    font_size: i32,
    font_label: std::rc::Rc<LoadedFont>,
    /// 节流 (refreshInterval = 轮询×2; 默认 100ms) + 基准
    refresh_interval: i64,
    last_refresh: i64,
    /// jet 检测一次性闩 (engine_check_done 后锁定)
    is_jet: bool,
    jet_latched: bool,
    /// 增压器量程一次性闩
    compressor_max_set: bool,
}

fn f_engine_gauge(
    props: &serde_json::Value,
    fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    let kind = props
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or("engine.gauge 需要 props.kind")?;
    let def = ENGINE_GAUGE_DEFS
        .iter()
        .find(|d| d.key == kind)
        .ok_or_else(|| format!("engine.gauge 未知仪表 kind: {kind}"))?;
    let lang: &Lang = fctx.lang.ok_or("engine.gauge 需要 FactoryCtx.lang")?;
    let cfg: GaugeCfg = fctx.gauge_cfg.cloned().unwrap_or_default();
    let fonts_dir = fctx
        .fonts_dir
        .as_deref()
        .ok_or("engine.gauge 需要 FactoryCtx.fonts_dir")?;
    let font_size =
        vm_core::base::format::java_round_f64((24.0 + cfg.engine_font_add as f64) * cfg.dpi_scale);
    let label = (def.label)(lang);
    let gauge = LabeledLinearGauge::new(label, def.max_value, !def.is_horizontal);
    let mut marked = None;
    if def.gauge_type == GaugeType::Compressor {
        let style = GaugeBarStyle {
            fill_color: colors().num,
            background_color: [0, 0, 0, 0],
            border_color: colors().shade_shape,
            show_border: true,
            vertical: !def.is_horizontal,
            stroke_width: 2,
        };
        let mut mg = MarkedGauge::new();
        mg.label = label.to_string();
        mg.set_max_value(def.max_value as f64);
        mg.set_bar_style(style);
        mg.add_marker(GaugeMarker {
            id: "optimal".to_string(),
            marker_type: MarkerType::LineFull,
            ratio: -1.0,
            color: colors().warning,
            ..GaugeMarker::default()
        });
        marked = Some(mg);
    }
    let half = vm_core::base::format::java_round_f32(font_size as f32 / 2.0);
    let font_label = LoadedFont::new_cached(&fonts_dir.join("sarasa-mono-sc-bold.ttf"), half)?;
    let refresh_interval =
        (cfg.service_loop_interval_ms as f64 * crate::overlays::engine_control::ENGINE_REFRESH_MULTIPLIER) as i64;
    let refresh_interval = if refresh_interval > 0 {
        refresh_interval
    } else {
        crate::layout::ui_constants::ENGINE_DEFAULT_REFRESH_MS
    };
    let mut w = EngineGaugeWidget {
        def,
        gauge,
        marked,
        visible: true,
        font_size,
        font_label,
        refresh_interval,
        last_refresh: 0,
        is_jet: false,
        jet_latched: false,
        compressor_max_set: false,
    };
    w.update_preview();
    Ok(Box::new(w))
}

impl EngineGaugeWidget {
    fn update_preview(&mut self) {
        let val = self.def.max_value / 2;
        let is_compressor = self.def.gauge_type == GaugeType::Compressor;
        let text = (if is_compressor { val + 1 } else { val }).to_string();
        self.gauge.gauge.update(val, &text);
        self.visible = true;
        if let Some(mg) = self.marked.as_mut() {
            mg.update_display(val, &text);
            mg.update_marker_ratio("optimal", 0.5);
        }
    }

    /// 测试断言面: 当前值
    pub fn cur_value(&self) -> i32 {
        self.gauge.gauge.cur_value
    }

    /// 测试断言面: 可见性
    pub fn visible(&self) -> bool {
        self.visible
    }
}

/// jet 机型隐藏族 (is_jet_hidden_gauge 同表)
fn jet_hidden(t: GaugeType) -> bool {
    matches!(
        t,
        GaugeType::Pitch | GaugeType::Radiator | GaugeType::Compressor | GaugeType::Mixture
    )
}

impl HudWidget for EngineGaugeWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // preview 保持半量程静态
        let (Some(frame), Some(payload)) = (env.frame, env.payload) else {
            return;
        };
        // 节流闩
        if env.now_ms - self.last_refresh < self.refresh_interval {
            return;
        }
        self.last_refresh = env.now_ms;
        // jet 检测闩 + compressor 量程一次性写 (update_state_from_payload 同源)
        if !self.jet_latched && payload.engine_check_done {
            self.is_jet = payload.is_jet;
            self.jet_latched = true;
            if self.def.gauge_type == GaugeType::Compressor && !self.compressor_max_set {
                if let Some(stages) = env.compressor_stages {
                    if stages > 1 {
                        self.gauge.gauge.max_value = stages - 1;
                        if let Some(mg) = self.marked.as_mut() {
                            mg.set_max_value((stages - 1) as f64);
                        }
                    }
                }
                self.compressor_max_set = true;
            }
        }
        // jet 机型隐藏 (持续判定 — 数据面同族仪表可能回归)
        if self.is_jet && jet_hidden(self.def.gauge_type) {
            self.visible = false;
            return;
        }
        // 取值分支 (update_gauges_zero_gc 单仪表摘出)
        let mut val;
        let mut has_val = true;
        match self.def.gauge_type {
            GaugeType::Throttle => val = frame.var_value("throttle").unwrap_or(0.0),
            GaugeType::Pitch => {
                val = frame.var_value("rpm_throttle").unwrap_or(0.0);
                self.visible = val >= 0.0;
                if !self.visible {
                    has_val = false;
                }
            }
            GaugeType::Power => val = frame.var_value("power_percent").unwrap_or(0.0),
            GaugeType::Mixture => {
                val = frame.var_value("mixture_state").unwrap_or(0.0);
                self.visible = val >= 0.0;
                if !self.visible {
                    has_val = false;
                }
            }
            GaugeType::Radiator => val = frame.var_value("radiator").unwrap_or(0.0),
            GaugeType::Compressor => {
                val = frame.var_value("compressor_stage").unwrap_or(0.0);
                let stage = val as i32;
                self.visible = stage > 0;
                if stage > 0 {
                    val = (stage - 1) as f64; // 显示 1 基档号, 条 0 基值
                } else {
                    has_val = false;
                }
                // optimal 档标记 (每帧)
                if let Some(mg) = self.marked.as_mut() {
                    match env.compressor_stages {
                        Some(stages) if payload.optimal_compressor_stage >= 0 && stages > 1 => {
                            mg.update_marker_ratio(
                                "optimal",
                                payload.optimal_compressor_stage as f64 / (stages - 1) as f64,
                            );
                        }
                        _ => mg.update_marker_ratio("optimal", -1.0),
                    }
                }
            }
            GaugeType::Fuel => val = frame.var_value("fuel_percent").unwrap_or(0.0),
        }
        if has_val && self.visible {
            let int_val = val as i32;
            let text = if self.def.gauge_type == GaugeType::Compressor {
                format::format((int_val + 1) as f64, 0)
            } else {
                format::format(val, 0)
            };
            self.gauge.gauge.update(int_val, &text);
            if let Some(mg) = self.marked.as_mut() {
                mg.update_buffer(int_val, &text);
            }
        }
    }

    fn reset_preview(&mut self) {
        self.update_preview();
        self.last_refresh = 0;
        self.is_jet = false;
        self.jet_latched = false;
        // compressor 量程闩同步复位 (跨会话残留的 FM 档位量程一并清)
        self.compressor_max_set = false;
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        if !self.visible {
            return;
        }
        let fs = self.font_size;
        let (len, th) = (4 * fs, fs >> 1);
        self.gauge.gauge.vertical = !self.def.is_horizontal;
        if let Some(mg) = self.marked.as_mut() {
            // MarkedGauge (COMPRESSOR) 与 LabeledLinearGauge 同区域口径
            mg.draw(cv, x, y, len, th, &self.font_label, aa);
        } else {
            self.gauge.draw(cv, x, y, len, th, &self.font_label, aa);
        }
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        // 隐藏 → 归零 (链式排布自动补位 = 原面板跳过隐藏的 dx/dy 推进语义)
        if !self.visible {
            return Dimension::new(0, 0);
        }
        let fs = self.font_size;
        if self.def.is_horizontal {
            Dimension::new(4 * fs, fs) // 条 fs/2 + label 文本行
        } else {
            Dimension::new(2 * fs, 4 * fs) // 条 fs/2 + 刻度文本列, 高 = 条长
        }
    }
}

const ENGINE_GAUGE_KEYS: &[&str] = &["fontSize", "dataPollIntervalMs"];

const KINDS: &[&str] = &[
    "throttle", "pitch", "power", "mixture", "radiator", "compressor", "fuel",
];

pub(super) const ENGINE_GAUGE_META: WidgetMeta = WidgetMeta {
    type_name: "core.engine.gauge",
    display_zh: "引擎仪表",
    category: WidgetCategory::Gauge,
    composite: false,
    props_schema: &[PropSchema {
        key: "kind",
        display_zh: "仪表",
        kind: PropKind::Enum(KINDS),
    }],
    config_keys: ENGINE_GAUGE_KEYS,
    data_shorts: &[],
    default_props: r#"{"kind":"throttle"}"#,
    factory: f_engine_gauge,
};
