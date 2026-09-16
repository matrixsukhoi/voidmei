//! core.layout.list — 列表容器 (块): 子项顺序语义 + 排列策略。
//!
//! 组件本体是**空壳** (无渲染面): 排列策略挂在布局节点上
//! (page_layout 解析 props → node.set_arrange), 引擎求解时接管子树;
//! 子项照常是页面普通组件 (数据分发/编辑命中/渲染走各自既有链路)。
//! 壳存在的意义: 注册表可拖拽建块 + 编辑面可选中/右键"块"。

use crate::layout::hud_layout_node::Dimension;
use crate::render::canvas::PixCanvas;

use super::env::{FactoryCtx, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::registry::{PageFonts, PropKind, PropSchema, WidgetCategory, WidgetMeta};

/// 类型名 (page_layout 的容器接线判别键)
pub const LIST_CONTAINER_TYPE: &str = "core.layout.list";

/// 列表容器壳 (无视觉; 排列逻辑在布局层)
pub struct ListContainerWidget;

fn f_list_container(
    _props: &serde_json::Value,
    _fctx: &FactoryCtx,
) -> Result<Box<dyn super::registry::HudWidget>, String> {
    Ok(Box::new(ListContainerWidget))
}

pub(super) const LIST_CONTAINER_META: WidgetMeta = WidgetMeta {
    type_name: LIST_CONTAINER_TYPE,
    display_zh: "列表容器",
    category: WidgetCategory::Layout,
    composite: false,
    props_schema: &[
        PropSchema {
            key: "arrange",
            display_zh: "排列",
            kind: PropKind::Enum(&["column", "columns", "wrap", "grid"]),
        },
        PropSchema {
            key: "columns",
            display_zh: "列数",
            kind: PropKind::Int,
        },
        PropSchema {
            key: "wrapHeight",
            display_zh: "换行高度",
            kind: PropKind::Int,
        },
        PropSchema {
            key: "gridRows",
            display_zh: "网格行数",
            kind: PropKind::Int,
        },
        PropSchema {
            key: "gridCols",
            display_zh: "网格列数",
            kind: PropKind::Int,
        },
        PropSchema {
            key: "gap",
            display_zh: "间距",
            kind: PropKind::Int,
        },
    ],
    config_keys: &[],
    data_shorts: &[],
    default_props: r#"{"arrange":"column","gap":0}"#,
    factory: f_list_container,
};

impl super::registry::HudWidget for ListContainerWidget {
    fn apply_style(&mut self, _env: &StyleEnv) {}

    fn push_templates(&mut self, _t: &MiniHudTemplates) {}

    fn on_data_update(&mut self, _env: &UpdateEnv) {}

    fn draw(&mut self, _cv: &mut PixCanvas, _x: i32, _y: i32, _fonts: &PageFonts, _aa: bool) {
        // 壳不绘制 (子项由引擎按排列结果逐节点绘制)
    }

    /// 占位 0×0: 容器尺寸 = 排列产物 (引擎 measure 递归接管, 不读本值)
    fn preferred_size(&self, _fonts: &PageFonts) -> Dimension {
        Dimension::new(0, 0)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
