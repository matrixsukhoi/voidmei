//! core.data.field — 原子数据字段组件 (数值 + 中文描述 + 单位, 单行三段)。
//!
//! fields.grid 黑盒退役后的字段原子形态: 画布编辑器里逐字段拖放/摆位/复制。
//! 视觉 = 原直通管线单行 (数值右对齐到固定 lwidth 竖线 → 多组件竖排时数值
//! 列天然对齐; label 半字号 BOLD; 单位半字号 PLAIN 下一基线; 三色 + (+1,+1)
//! 阴影)。数据条件 (visibleWhen/naWhen 中缀) 每帧求值 — visibleWhen 不满足
//! 时 preferred 高度归零 (链式排布的后件上移, 语义 = 原 grid 行消失)。

use vm_core::base::format;
use vm_core::formula::registry::FormulaView;
use vm_core::ui_support::row_def::{compile_cond, Cond};

use crate::layout::hud_layout_node::Dimension;
use crate::layout::RenderCtx;
use crate::overlays::flight_info::{default_num_height, flight_value};
use crate::render::canvas::PixCanvas;
use crate::render::fields::FontTriple;
use crate::render::palette::colors;

use super::env::{FactoryCtx, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::registry::PageFonts;
use super::registry::{HudWidget, PropKind, PropSchema, WidgetCategory, WidgetMeta};

/// 原子数据字段
pub struct DataFieldWidget {
    // --- 显示定义 (props 快照; reinit 整体重建) ---
    label: String,
    unit: String,
    preview_value: String,
    /// 取数表达式: 变量短名 | 公式名 | "X * N" 乘数
    source: String,
    precision: u8,
    time_format: bool,
    /// 进气压特例: is_imperial 驱动精度/单位切换 (P/x.x'' ↔ Ata)
    imperial: bool,
    visible_when: Option<Cond>,
    na_when: Option<Cond>,
    // --- 渲染资源 (构造期定) ---
    ctx: RenderCtx, // column=1 单列度量
    fonts: FontTriple,
    // --- 数据面 ---
    value_text: String,
    /// 单位显示串 (imperial 逐帧由 frame 驱动; preview = props.unit)
    unit_shown: String,
    /// visibleWhen 当前判定 (preferred 高度归零面)
    shown: bool,
}

impl DataFieldWidget {
    /// 测试断言面: 当前值文本
    pub fn value_text(&self) -> &str {
        &self.value_text
    }

    /// 测试断言面: 当前显示判定
    pub fn shown(&self) -> bool {
        self.shown
    }

    /// 阴影文本 (+1,+1) 阴影先画本体后画 — draw_shaded 同式, 直画 PixCanvas
    // (参数列对齐 fields.rs draw_shaded 原签名)
    #[allow(clippy::too_many_arguments)]
    fn draw_shaded(
        cv: &mut PixCanvas,
        font: &crate::render::font::LoadedFont,
        x: i32,
        baseline: i32,
        text: &str,
        color: [u8; 4],
        shade: [u8; 4],
        aa: bool,
    ) {
        cv.draw_text(font, x + 1, baseline + 1, text, shade, aa);
        cv.draw_text(font, x, baseline, text, color, aa);
    }
}

fn f_data_field(
    props: &serde_json::Value,
    fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    let fonts_dir = fctx
        .fonts_dir
        .as_deref()
        .ok_or("data.field 需要 FactoryCtx.fonts_dir")?;
    let source = props
        .get("target")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or("data.field 需要 props.target (变量短名/公式名/乘数式)")?
        .to_string();
    let label = props
        .get("label")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(&source)
        .to_string();
    // 字号 = 页面主字号 (面板"大小"滑条经页面 font_size 联动) + 组件相对增量
    let font_add = fctx.fonts.draw.size - 24
        + props.get("fontAdd").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let ctx = RenderCtx::new(font_add, 1, default_num_height(font_add));
    let fonts = FontTriple::load(fonts_dir, &ctx)?;
    let preview_value = props
        .get("previewValue")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("0")
        .to_string();
    let unit = props
        .get("unit")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let w = DataFieldWidget {
        label,
        unit: unit.clone(),
        unit_shown: unit,
        preview_value: preview_value.clone(),
        source,
        precision: props.get("precision").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
        time_format: props
            .get("format")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("TIME_MM_SS")),
        imperial: props.get("imperial").and_then(|v| v.as_bool()).unwrap_or(false),
        visible_when: props
            .get("visibleWhen")
            .and_then(|v| v.as_str())
            .and_then(compile_cond),
        na_when: props
            .get("naWhen")
            .and_then(|v| v.as_str())
            .and_then(compile_cond),
        ctx,
        fonts,
        value_text: preview_value.clone(),
        shown: true,
    };
    Ok(Box::new(w))
}

const DATA_FIELD_KEYS: &[&str] = &["fontSize"];

/// 单行值文本 (na/format/time 分派; PowerInfoState.update 同语义)
fn value_text_of(w: &DataFieldWidget, raw: f64, s: &dyn FormulaView) -> String {
    if let Some(cond) = &w.na_when {
        if cond.eval(s, raw) {
            return "-".to_string();
        }
    }
    if w.time_format {
        return format::format_time(raw);
    }
    let precision = if w.imperial {
        // 英制 1 位 / 公制 2 位 (全表仅进气压; BOS 管线原语义)
        if s.var_value("is_imperial").unwrap_or(0.0) > 0.0 {
            1
        } else {
            2
        }
    } else {
        w.precision
    };
    format::format(raw, precision)
}

/// 英制单位文本 (进气压: P/x.x''; 公制 Ata)
fn unit_text_of(w: &DataFieldWidget, s: &dyn FormulaView) -> String {
    if !w.imperial {
        return w.unit.clone();
    }
    if s.var_value("is_imperial").unwrap_or(0.0) > 0.0 {
        let inhg = s.var_value("manifold_pressure").unwrap_or(0.0) * 760.0 / 25.4;
        format!("P/{}''", format::format(inhg, 1))
    } else {
        "Ata".to_string()
    }
}

pub(super) const DATA_FIELD_META: WidgetMeta = WidgetMeta {
    type_name: "core.data.field",
    display_zh: "数据字段",
    category: WidgetCategory::Text,
    composite: false,
    props_schema: &[
        PropSchema {
            key: "target",
            display_zh: "数据绑定",
            kind: PropKind::Target,
        },
        PropSchema {
            key: "label",
            display_zh: "描述",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "unit",
            display_zh: "单位",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "precision",
            display_zh: "小数位",
            kind: PropKind::Int,
        },
        PropSchema {
            key: "format",
            display_zh: "格式",
            kind: PropKind::Enum(&["", "TIME_MM_SS"]),
        },
        PropSchema {
            key: "previewValue",
            display_zh: "预览值",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "visibleWhen",
            display_zh: "显示条件",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "naWhen",
            display_zh: "NA 条件",
            kind: PropKind::Str,
        },
        PropSchema {
            key: "imperial",
            display_zh: "英制切换",
            kind: PropKind::Bool,
        },
        PropSchema {
            key: "fontAdd",
            display_zh: "字号增量",
            kind: PropKind::Int,
        },
    ],
    config_keys: DATA_FIELD_KEYS,
    data_shorts: &[],
    default_props: r#"{"target":"ias","label":"表  速","unit":"Km/h","precision":0,"previewValue":"500"}"#,
    factory: f_data_field,
};

impl HudWidget for DataFieldWidget {
    fn apply_style(&mut self, _env: &StyleEnv) {
        // 字号/字体在构造期定 (props.fontAdd), 无运行时风格注入面
    }

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // preview (frame None) 保持静态值 — 对位 Java initPreview 不订阅
        let Some(frame): Option<&dyn FormulaView> = env.frame else {
            return;
        };
        let raw = flight_value(frame, &self.source).unwrap_or(0.0);
        self.shown = self.visible_when.as_ref().is_none_or(|c| c.eval(frame, raw));
        if self.shown {
            self.value_text = value_text_of(self, raw, frame);
            self.unit_shown = unit_text_of(self, frame);
        }
    }

    fn reset_preview(&mut self) {
        self.value_text = self.preview_value.clone();
        self.unit_shown = self.unit.clone();
        self.shown = true;
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        if !self.shown {
            return; // 行消失语义: 不画 (preferred 高度已归零)
        }
        let (ox, oy) = self.ctx.start_offset();
        let pal = colors();
        // --- 数值 (右对齐 lwidth 竖线, 基线 value_baseline) ---
        let vw = self.fonts.num.measure(&self.value_text);
        let vx = x + ox + self.ctx.lwidth() - vw - self.ctx.num_padding();
        Self::draw_shaded(
            cv,
            &self.fonts.num,
            vx,
            y + self.ctx.value_baseline(oy),
            &self.value_text,
            pal.num,
            pal.shade_shape,
            aa,
        );
        // --- 标签 (基线 oy) ---
        Self::draw_shaded(
            cv,
            &self.fonts.label,
            x + ox + self.ctx.lwidth(),
            y + oy,
            &self.label,
            pal.label,
            pal.shade_shape,
            aa,
        );
        // --- 单位 (基线 oy + label 字号, 半字号 PLAIN) ---
        Self::draw_shaded(
            cv,
            &self.fonts.unit,
            x + ox + self.ctx.lwidth(),
            y + self.ctx.unit_baseline(oy),
            &self.unit_shown,
            pal.unit,
            pal.shade_shape,
            aa,
        );
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        // 单行: 宽 = 单列网格同款 (数值列 + 标签区), 高 = num_height
        // (原网格行距 advance_y = num_height, 链式紧贴 → 行距与原视觉一致);
        // visibleWhen 不满足 → 高度归零 (行消失, 后件上移)
        let h = if self.shown {
            self.ctx.advance_y()
        } else {
            0
        };
        Dimension::new(self.ctx.total_width(), h)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// =====================================================================
// Tests (原 FlightInfoState/PowerInfoState 更新路径语义的组件化迁移)
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::registry::lookup_widget;

    fn fonts_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts")
    }

    fn make(props: serde_json::Value) -> Box<dyn HudWidget> {
        let font = crate::render::font::LoadedFont::new(
            &fonts_dir().join("sarasa-mono-sc-bold.ttf"),
            24,
        )
        .unwrap();
        let rc = std::rc::Rc::new(font);
        let fonts = std::rc::Rc::new(super::super::registry::PageFonts {
            draw: std::rc::Rc::clone(&rc),
            small: std::rc::Rc::clone(&rc),
            s_small: rc,
        });
        let fctx = FactoryCtx {
            minihud_ctx: None,
            fonts,
            lang: None,
            fonts_dir: Some(fonts_dir()),
            gauge_cfg: None,
        };
        (lookup_widget("core.data.field").unwrap().factory)(&props, &fctx).expect("工厂构造")
    }

    /// 取值桩: 名字可达性经 canonical (registry ∪ 公式集), 值按表
    struct TellyView(std::collections::HashMap<&'static str, f64>);
    impl FormulaView for TellyView {
        fn var_value(&self, name: &str) -> Option<f64> {
            self.0.get(name).copied()
        }
    }

    fn feed(w: &mut Box<dyn HudWidget>, view: &TellyView) {
        let data = vm_core::derived::hud_data::HUDData::empty();
        let env = UpdateEnv {
            data: &data,
            frame: Some(view),
            fmdata: None,
            payload: None,
            compressor_stages: None,
            now_ms: 0,
            maneuver_len: 0,
            maneuver_ticks: Default::default(),
            lang: None,
        };
        w.on_data_update(&env);
    }

    fn as_field(w: &dyn HudWidget) -> &DataFieldWidget {
        w.as_any().downcast_ref::<DataFieldWidget>().unwrap()
    }

    /// 基础取值 + 精度格式化
    #[test]
    fn formats_value_with_precision() {
        let mut w = make(serde_json::json!({
            "target": "ias", "label": "表  速", "unit": "Km/h",
            "precision": 0, "previewValue": "500"
        }));
        let v = TellyView(std::collections::HashMap::from([("ias", 555.4)]));
        feed(&mut w, &v);
        assert_eq!(as_field(w.as_ref()).value_text(), "555");
    }

    /// na-when → "-"
    #[test]
    fn na_when_renders_dash() {
        let mut w = make(serde_json::json!({
            "target": "horse_power", "label": "功  率", "unit": "Hp",
            "naWhen": "value <= 0", "previewValue": "1200"
        }));
        feed(&mut w, &TellyView(std::collections::HashMap::from([("horse_power", 0.0)])));
        assert_eq!(as_field(w.as_ref()).value_text(), "-");
    }

    /// visible-when 不满足 → shown=false (preferred 高度归零 = 行消失语义)
    #[test]
    fn visible_when_hides_row() {
        let mut w = make(serde_json::json!({
            "target": "horse_power", "label": "功  率", "unit": "Hp",
            "visibleWhen": "!isJetEngine", "previewValue": "1200"
        }));
        let v = TellyView(std::collections::HashMap::from([
            ("horse_power", 1200.0),
            ("is_jet_engine", 1.0),
        ]));
        feed(&mut w, &v);
        assert!(!as_field(w.as_ref()).shown(), "喷气机隐藏功率行");
    }

    /// TIME_MM_SS 格式
    #[test]
    fn time_mm_ss_format() {
        let mut w = make(serde_json::json!({
            "target": "fuel_time_mili * 0.001", "label": "燃油时", "unit": "s",
            "format": "TIME_MM_SS", "previewValue": "45"
        }));
        let v = TellyView(std::collections::HashMap::from([("fuel_time_mili", 2750.0)]));
        feed(&mut w, &v);
        assert_eq!(as_field(w.as_ref()).value_text(), "00'02");
    }

    /// 进气压英制切换 (imperial): 精度 1 位 + P/x.x'' 单位
    #[test]
    fn imperial_manifold_switches_unit_and_precision() {
        let mut w = make(serde_json::json!({
            "target": "manifold_pressure_display", "label": "进气压",
            "precision": 2, "imperial": true, "previewValue": "1.2"
        }));
        // 公制 (is_imperial 0): "Ata" + 2 位
        feed(
            &mut w,
            &TellyView(std::collections::HashMap::from([
                ("manifold_pressure_display", 0.98),
                ("manifold_pressure", 0.98),
            ])),
        );
        assert_eq!(as_field(w.as_ref()).value_text(), "0.98");
        // 英制: P/x.x'' + 1 位
        feed(
            &mut w,
            &TellyView(std::collections::HashMap::from([
                ("manifold_pressure_display", 44.6),
                ("manifold_pressure", 44.6),
                ("is_imperial", 1.0),
            ])),
        );
        assert_eq!(as_field(w.as_ref()).value_text(), "44.6");
        let inhg = vm_core::base::format::format(44.6 * 760.0 / 25.4, 1);
        assert_eq!(as_field(w.as_ref()).unit_shown, format!("P/{inhg}''"));
    }

    /// reset_preview: live 残留回 preview 静态
    #[test]
    fn reset_preview_restores_static() {
        let mut w = make(serde_json::json!({
            "target": "ias", "label": "表  速", "unit": "Km/h",
            "previewValue": "500"
        }));
        feed(&mut w, &TellyView(std::collections::HashMap::from([("ias", 555.0)])));
        assert_eq!(as_field(w.as_ref()).value_text(), "555");
        w.reset_preview();
        assert_eq!(as_field(w.as_ref()).value_text(), "500");
    }
}
