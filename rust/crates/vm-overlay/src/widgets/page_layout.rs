//! PageDoc → 布局引擎 (原 MINIHUD_NODE_SPECS 常量拓扑表的数据化驱动)。
//!
//! 语义裁决 (对齐原 build_mihud_layout 行为):
//! - visibleWhen 求值 false → 节点不建 (原 crosshair 的 cfg 门控语义);
//!   求值时机 = build/reinit (重建式, 不逐帧);
//! - 父 id 缺席 (被裁剪/误删) → 退化根节点 (宽容; 原 speedBar 语义,
//!   防用户编辑删父时子组件无故消失);
//! - 组件工厂失败 (类型未注册/props 非法) → warn 跳过 (出厂页不可达)。

use std::collections::HashMap;

use vm_core::base::logger;
use vm_core::config::json_model::{ComponentDoc, PageDoc};

use crate::layout::anchor::Anchor;
use crate::layout::hud_layout_node::{HUDLayoutNode, HUDLayoutNodeExt};
use crate::layout::minihud_layout::{AutoSizingPlan, ModernHUDLayoutEngine};

use super::env::FactoryCtx;
use super::registry::{lookup_widget, WidgetCell};

/// 锚点名解析 ("TopLeft"…; 空串/未知 → TopLeft)
fn anchor_from_name(name: &str) -> Anchor {
    match name {
        "TopCenter" => Anchor::TopCenter,
        "TopRight" => Anchor::TopRight,
        "MiddleLeft" => Anchor::MiddleLeft,
        "Center" => Anchor::Center,
        "MiddleRight" => Anchor::MiddleRight,
        "BottomLeft" => Anchor::BottomLeft,
        "BottomCenter" => Anchor::BottomCenter,
        "BottomRight" => Anchor::BottomRight,
        _ => Anchor::TopLeft,
    }
}

/// build 输入 (组件工厂环境与页面几何)
pub struct PageBuildInputs<'a> {
    pub doc: &'a PageDoc,
    pub fctx: &'a FactoryCtx<'a>,
    /// visibleWhen 求值源 (配置 bool 快照优先, 遥测后继)
    pub visible_src: &'a dyn Fn(&str) -> Option<bool>,
    /// 求值源 None 时的兜底: minihud = false (整树缺失关准星 Java 兜底),
    /// 通用页 = true (宽容建成 — 数据条件归组件 props.visibleWhen 运行时)
    pub visible_default: bool,
    /// 画布宽 (crosshair 在右半区时 = 基宽×2, 原 layoutWidth 语义)
    pub canvas_w: i32,
    pub canvas_h: i32,
    /// line_height (字号相对坐标的换算基)
    pub line_height: f64,
    pub debug: bool,
}

/// build 产物: 引擎 + 自动尺寸计划 + 组件句柄表 (编排器具名访问面)
pub struct BuiltPageLayout {
    pub engine: ModernHUDLayoutEngine<WidgetCell>,
    pub sizing: Option<AutoSizingPlan>,
    /// id → 组件句柄 (build 后编排器/数据面分发用)
    pub cells: HashMap<String, WidgetCell>,
}

impl BuiltPageLayout {
    /// 空态 (init 占位 — 未建树: 空引擎 + 无尺寸计划)
    pub fn empty(w: i32, h: i32) -> Self {
        BuiltPageLayout {
            engine: ModernHUDLayoutEngine::new(w, h),
            sizing: None,
            cells: HashMap::new(),
        }
    }
}

/// PageDoc → 布局引擎 + 组件实例。
/// 返回 (cells, engine, sizing); 组件空 → sizing=None (宿主保持初始窗口)。
pub fn build_page_layout(inputs: &PageBuildInputs) -> BuiltPageLayout {
    let doc = inputs.doc;
    let mut engine = ModernHUDLayoutEngine::new(inputs.canvas_w, inputs.canvas_h);
    engine.set_debug(inputs.debug);
    engine.set_line_height(inputs.line_height);

    let mut cells: HashMap<String, WidgetCell> = HashMap::new();
    if doc.components.is_empty() {
        return BuiltPageLayout {
            engine,
            sizing: None,
            cells,
        };
    }

    for comp in &doc.components {
        let Some(cell) = build_component(comp, inputs) else { continue };
        // 父解析: 缺席退化根 (模块头语义裁决)
        let parent = comp.parent.as_deref().and_then(|pid| engine.get_node(pid));
        let node = HUDLayoutNode::new(comp.id.clone(), cell.clone());
        node.set_parent(parent.as_ref())
            .set_relative_position(comp.pos[0], comp.pos[1])
            .set_anchors(anchor_from_name(&comp.anchor[1]), anchor_from_name(&comp.anchor[0]));
        engine.add_node(node);
        cells.insert(comp.id.clone(), cell);
    }

    if cells.is_empty() {
        return BuiltPageLayout {
            engine,
            sizing: None,
            cells,
        };
    }

    engine.do_layout();
    let sizing = engine.apply_auto_sizing(doc.padding);
    engine.log_topology();

    BuiltPageLayout {
        engine,
        sizing: Some(sizing),
        cells,
    }
}

/// 单组件建身: enabled + visibleWhen 门控 → 工厂造件
fn build_component(comp: &ComponentDoc, inputs: &PageBuildInputs) -> Option<WidgetCell> {
    if !comp.enabled {
        return None;
    }
    if let Some(cond) = &comp.visible_when {
        // None 兜底语义归编排器: minihud = false (整树缺失关准星的 Java 兜底),
        // 通用页 = true (宽容建成 — 数据条件由组件 props.visibleWhen 运行时承担)
        let on = (inputs.visible_src)(cond).unwrap_or(inputs.visible_default);
        if !on {
            return None; // 原语义: 条件不满足 → 节点不建
        }
    }
    let Some(meta) = lookup_widget(&comp.r#type) else {
        logger::warn(
            "PageLayout",
            &format!("组件类型未注册: {} (id={})", comp.r#type, comp.id),
        );
        return None;
    };
    match (meta.factory)(&comp.props, inputs.fctx) {
        Ok(inner) => Some(WidgetCell::new(inner, inputs.fctx.fonts.clone())),
        Err(e) => {
            logger::warn(
                "PageLayout",
                &format!("组件构造失败: {} (id={}: {e})", comp.r#type, comp.id),
            );
            None
        }
    }
}
