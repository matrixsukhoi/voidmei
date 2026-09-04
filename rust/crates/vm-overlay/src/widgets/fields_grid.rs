//! fields.grid — 字段网格复合组件 (飞行信息/动力信息两面板的统一形态)。
//!
//! 黑盒复合 (composite): 内部保留两条历史渲染子管线 (像素保真优先 —
//! "飞行信息" = fields 直通 Canvas + 整帧桥; 其余 = BOS PixCanvas),
//! 页面/编辑器只管摆位; 两管线的收敛属后续惯用化 pass (登记: 坏味道表)。
//! 行定义 (RowDef) 与 (字号, 列数) 经 FactoryCtx 注入 (reinit 整体重建)。

use std::sync::Arc;

use vm_core::formula::registry::FormulaView;
use vm_core::ui_support::row_def::RowDef;

use crate::layout::hud_layout_node::Dimension;
use crate::overlays::flight_info::{FlightInfoState, default_num_height};
use crate::overlays::power_info::PowerInfoState;
use crate::render::canvas::PixCanvas;
use crate::render::fields::{render_fields_fixed, FieldText, RenderColors};
use crate::render::renderers::{BosStyleRenderer, Field, OverlayRenderer, RenderContext};
use crate::render::palette::colors;

use super::env::{FactoryCtx, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::registry::PageFonts;
use super::registry::{HudWidget, PropSchema, WidgetCategory, WidgetMeta};

/// 两条历史子管线 (构造时按 fieldSet 定; 复合组件内部实现不外露)
#[allow(clippy::large_enum_variant)] // Straight/Bos 尺寸差 = 历史管线资源, 页面级单实例无复制面
enum Pipeline {
    /// 飞行信息: fields 直通 Canvas + 整帧偏移桥 (POC 像素对拍管线)
    Straight(FlightInfoState),
    /// 动力信息: BOS 字段网格 (BosStyleRenderer)
    Bos {
        state: PowerInfoState,
        ctx: RenderContext,
        renderer: BosStyleRenderer,
    },
}

/// 字段网格复合组件
pub struct FieldsGridWidget {
    pipeline: Pipeline,
    /// (w, h) 构造期快照 (preferred_size 的缓存; reinit 重建即刷新)
    size: (i32, i32),
}

/// 工厂: props.fieldSet 定面板 ("飞行信息" 走直通管线, 其余 BOS)
fn f_fields_grid(
    props: &serde_json::Value,
    fctx: &FactoryCtx,
) -> Result<Box<dyn HudWidget>, String> {
    let field_set = props
        .get("fieldSet")
        .and_then(|v| v.as_str())
        .ok_or("fields.grid 需要 props.fieldSet")?;
    let defs: Arc<Vec<RowDef>> = fctx
        .rows
        .get(field_set)
        .cloned()
        .ok_or_else(|| format!("fields.grid 行源缺失: {field_set}"))?;
    let (font_add, columns) = fctx
        .fields_cfg
        .ok_or("fields.grid 需要 FactoryCtx.fields_cfg (font_add, columns)")?;
    let fonts_dir = fctx
        .fonts_dir
        .as_deref()
        .ok_or("fields.grid 需要 FactoryCtx.fonts_dir")?;

    if field_set == "飞行信息" {
        // 直通管线 (POC 对拍形态: RenderCtx + FontTriple + Canvas)
        let ctx = crate::layout::RenderCtx::new(font_add, columns, default_num_height(font_add));
        let fonts = crate::render::fields::FontTriple::load(fonts_dir, &ctx)?;
        let size = (ctx.total_width(), ctx.total_height(defs.len() as i32));
        let mut st = FlightInfoState::with_resources(defs, ctx, fonts);
        st.reset_preview_rows();
        Ok(Box::new(FieldsGridWidget {
            pipeline: Pipeline::Straight(st),
            size,
        }))
    } else {
        let ctx = RenderContext::load(fonts_dir, font_add, columns)?;
        let mut state = PowerInfoState::new(defs);
        let size = state.preferred_size(&ctx);
        state.reset_preview();
        Ok(Box::new(FieldsGridWidget {
            pipeline: Pipeline::Bos {
                state,
                ctx,
                renderer: BosStyleRenderer::default(),
            },
            size,
        }))
    }
}

impl FieldsGridWidget {
    /// 直通管线 (飞行信息) 的 state 借出 (测试断言面; 非此管线 None —
    /// 管线选择构造期定, 见模块头)
    pub fn straight(&self) -> Option<&FlightInfoState> {
        match &self.pipeline {
            Pipeline::Straight(st) => Some(st),
            _ => None,
        }
    }

    /// BOS 管线 (动力信息) 的 state 借出 (测试断言面; 非此管线 None)
    pub fn bos(&self) -> Option<&PowerInfoState> {
        match &self.pipeline {
            Pipeline::Bos { state, .. } => Some(state),
            _ => None,
        }
    }
}

const FIELDS_KEYS: &[&str] = &["fontSize", "flightInfoColumn", "hudColumns"];

/// 注册 (composite = 黑盒)
pub(super) const FIELDS_GRID_META: WidgetMeta = WidgetMeta {
    type_name: "core.fields.grid",
    display_zh: "字段网格",
    category: WidgetCategory::Composite,
    composite: true,
    props_schema: &[PropSchema {
        key: "fieldSet",
        display_zh: "数据面板",
        kind: super::registry::PropKind::Str,
    }],
    config_keys: FIELDS_KEYS,
    data_shorts: &[],
    factory: f_fields_grid,
};

impl HudWidget for FieldsGridWidget {
    fn apply_style(&mut self, _env: &StyleEnv) {
        // fields 管线无风格注入面 (字体/列度量在构造期定, reinit 重建)
    }

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, env: &UpdateEnv) {
        // preview (frame None) 保持静态行 — 对位 Java initPreview 不订阅
        let Some(frame): Option<&dyn FormulaView> = env.frame else {
            return;
        };
        match &mut self.pipeline {
            Pipeline::Straight(st) => st.update(frame),
            Pipeline::Bos { state, .. } => {
                state.update(env.now_ms, frame);
            }
        }
    }

    fn reset_preview(&mut self) {
        match &mut self.pipeline {
            Pipeline::Straight(st) => st.reset_preview_rows(),
            Pipeline::Bos { state, .. } => state.reset_preview(),
        }
    }

    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, _fonts: &PageFonts, aa: bool) {
        match &mut self.pipeline {
            Pipeline::Straight(st) => {
                // 直通管线: 清零重绘到伴随 Canvas → 整帧偏移桥入 (POC 语义)
                let pal = RenderColors {
                    num: colors().num,
                    label: colors().label,
                    unit: colors().unit,
                    shade: colors().shade_shape,
                };
                let FlightInfoState {
                    defs,
                    rows,
                    canvas,
                    ctx,
                    fonts,
                } = st;
                let texts: Vec<FieldText> = rows
                    .iter()
                    .map(|(i, v)| FieldText {
                        label: &defs[*i].label,
                        unit: &defs[*i].unit,
                        value: v,
                    })
                    .collect();
                render_fields_fixed(canvas, &texts, ctx, fonts, &pal, aa);
                if !cv.composite_straight_frame_at(
                    x,
                    y,
                    &canvas.buf,
                    canvas.width,
                    canvas.height,
                    aa,
                ) {
                    vm_core::base::logger::warn("fields.grid", "整帧桥尺寸不符, 本帧丢弃");
                }
            }
            Pipeline::Bos { state, ctx, renderer } => {
                let fields: Vec<Field> = state.fields().iter().map(Field::Data).collect();
                let mut offset = [x, y];
                OverlayRenderer::render(renderer, cv, &fields, ctx, &mut offset);
            }
        }
    }

    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        // 构造期快照 (行数/列数/字号变化走 reinit 整体重建)
        Dimension::new(self.size.0, self.size.1)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
