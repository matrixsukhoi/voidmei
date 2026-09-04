//! widgets 域: HUD 组件注册表 (组件自组装 overlay 的运行时底座)。
//!
//! W2 形态: MiniHUD 12 组件 trait 化 (闭式枚举退役) + 工厂注册表 +
//! PageDoc 驱动建树。W3 起泛化数据面 (frame/FormulaView 进 env) 与
//! sidecar (FM 黑盒组件)。

pub mod data_field;
pub mod env;
pub mod fm_sidecar;
pub mod gauges_composite;
pub mod minihud;
pub mod page_layout;
pub mod page_overlay;
pub mod registry;

pub use env::{FactoryCtx, GaugeCfg, MiniHudTemplates, StyleEnv, UpdateEnv};
pub use fm_sidecar::{SidecarAction, SidecarCtx, WidgetSidecar};
pub use page_layout::{build_page_layout, BuiltPageLayout, PageBuildInputs};
pub use page_overlay::{page_font_size, page_overlay_spec, solve_page_snapshot, PageHandle, PageOverlay, PageSpecParams, SolveResult};
pub use registry::{lookup_widget, widget_registry, HudWidget, WidgetCell, WidgetCategory, WidgetMeta};
