//! HUD 组件注册表: 闭式枚举 → type_name 键控工厂。
//!
//! 组件清单编译期已知 → const 表 (不引 inventory/proc-macro);
//! `type_name` 命名空间 "core." 预留 "ext."/"user." (插件市场留口)。

use std::cell::RefCell;
use std::rc::Rc;

use crate::layout::hud_layout_node::{Dimension, HasPreferredSize};
use crate::layout::minihud_layout::HasVisibility;
use crate::render::canvas::PixCanvas;

use super::env::{FactoryCtx, MiniHudTemplates, StyleEnv, UpdateEnv};
use super::{
    axes_atom, data_field, engine_gauge, fm_field, fm_list, fm_sidecar, gauges_composite,
    gear_flaps_atom, minihud,
};

// =====================================================================
// 组件契约
// =====================================================================

/// HUD 组件契约 (MiniHudComponentInner 枚举分发面的 trait 化;
/// fonts 经 WidgetCell 外壳传入 — 组件内部不持字体;
/// visible 由外壳承担 (WidgetBox.visible), 组件不感知)
pub trait HudWidget {
    /// 风格注入 + 配置驱动可见性 (原 applyStyleToComponents 各具名臂 +
    /// update_component/row_visibility 的组件自治段 — 组件自取所需)
    fn apply_style(&mut self, env: &StyleEnv);

    /// preview 模板与静态值推送 (原 refreshTemplates 尾部 + update_row_values)
    fn push_templates(&mut self, t: &MiniHudTemplates);

    /// 数据更新 (原 MiniHudComponent::on_data_update 的各枚举臂)
    fn on_data_update(&mut self, env: &UpdateEnv);

    /// 绘制 (fonts = 页面字体档, WidgetCell 注入)
    fn draw(&mut self, cv: &mut PixCanvas, x: i32, y: i32, fonts: &PageFonts, aa: bool);

    /// 期望尺寸 (fonts 注入 — 行族签名需要字体; WidgetCell 转无参 HasPreferredSize)
    fn preferred_size(&self, fonts: &PageFonts) -> Dimension;

    /// 具体类型借出 (测试断言的 downcast 面)
    fn as_any(&self) -> &dyn std::any::Any;

    /// FM 黑盒组件的特殊数据面 (W3C: tick 由渲染线程节拍驱动,
    /// 非 FormulaView 喂数; 普通组件恒 None。
    /// 组件全为 owned 数据 → dyn 钉 'static, 免 &mut trait 对象不变性冲突)
    fn sidecar(&mut self) -> Option<&mut (dyn super::fm_sidecar::WidgetSidecar + 'static)> {
        None
    }

    /// preview 复位 (live 会话残留值清回 preview 静态; 默认空 —
    /// 有状态组件覆写, 对位旧 reset_preview 族)
    fn reset_preview(&mut self) {}
}

/// 页面字体档别名 (widgets 域不依赖 minihud 内部类型名的边界缝合)
pub type PageFonts = crate::overlays::minihud::MiniHudFonts;

// =====================================================================
// 共享句柄 (原 CompCell — 编排器具名槽位与布局节点图双持)
// =====================================================================

/// 组装层组件 = Box<dyn HudWidget> + visible + 字体共享。
/// (visible 在外壳而非组件内 — 布局引擎经 HasVisibility 门控,
/// 编排器经 set_visible 写入, 组件本体不感知)
pub struct WidgetBox {
    pub inner: Box<dyn HudWidget>,
    visible: bool,
    fonts: Rc<PageFonts>,
    /// 尺寸覆盖 (R6 编辑面 resize 手柄): Some 时 preferred_size 钉此值,
    /// hidden 塌缩语义仍优先 (不可见 = 0×0, 链式补位不受遮蔽)
    size_override: Option<(i32, i32)>,
}

impl WidgetBox {
    pub fn new(inner: Box<dyn HudWidget>, fonts: Rc<PageFonts>) -> Self {
        WidgetBox {
            inner,
            visible: true, // AbstractHUDComponent.visible 初始 true
            fonts,
            size_override: None,
        }
    }

    pub fn set_visible(&mut self, v: bool) {
        self.visible = v;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 字体档换新 (reinit 重建 ctx 后整体换新; Java setStyle 的 Font 形参)
    pub fn set_fonts(&mut self, fonts: Rc<PageFonts>) {
        self.fonts = fonts;
    }

    pub fn set_size_override(&mut self, size: Option<(i32, i32)>) {
        self.size_override = size;
    }

    pub fn size_override(&self) -> Option<(i32, i32)> {
        self.size_override
    }
}

/// 组件共享句柄 (Rc<RefCell>; overlay 具名槽位与布局节点图双持 —
/// Java components 列表与节点图共享同一批对象的落地)
#[derive(Clone)]
pub struct WidgetCell(pub(super) Rc<RefCell<WidgetBox>>);

impl WidgetCell {
    pub fn new(inner: Box<dyn HudWidget>, fonts: Rc<PageFonts>) -> Self {
        WidgetCell(Rc::new(RefCell::new(WidgetBox::new(inner, fonts))))
    }

    /// 借出内件执行 (闭包内不得再借同一 cell — RefCell 独占借用)
    pub fn with_inner<R>(&self, f: impl FnOnce(&mut Box<dyn HudWidget>) -> R) -> R {
        f(&mut self.0.borrow_mut().inner)
    }

    pub fn set_visible(&self, v: bool) {
        self.0.borrow_mut().set_visible(v);
    }

    pub fn is_visible(&self) -> bool {
        self.0.borrow().is_visible()
    }

    pub fn set_fonts(&self, fonts: Rc<PageFonts>) {
        self.0.borrow_mut().set_fonts(fonts);
    }

    /// 尺寸覆盖 (R6 编辑面 resize; None = 恢复内容自适应)
    pub fn set_size_override(&self, size: Option<(i32, i32)>) {
        self.0.borrow_mut().set_size_override(size);
    }

    pub fn apply_style(&self, env: &StyleEnv) {
        self.0.borrow_mut().inner.apply_style(env);
    }

    pub fn push_templates(&self, t: &MiniHudTemplates) {
        self.0.borrow_mut().inner.push_templates(t);
    }

    pub fn on_data_update(&self, env: &UpdateEnv) {
        self.0.borrow_mut().inner.on_data_update(env);
    }

    pub fn reset_preview(&self) {
        self.0.borrow_mut().inner.reset_preview();
    }

    /// 测试断言面: 借出内件具体类型 (生产勿用 — 组件自治原则;
    /// downcast 失败 = 组件类型不符, None)
    pub fn downcast_ref<T: 'static>(&self) -> Option<std::cell::Ref<'_, T>> {
        let borrow = self.0.borrow();
        std::cell::Ref::filter_map(borrow, |b| b.inner.as_any().downcast_ref::<T>()).ok()
    }

    /// sidecar 面借出 (渲染线程 tick 驱动; 返回的 RefMut 守卫期内完成 tick 调用 —
    /// 守卫存活期间不得再借本 cell)
    pub fn sidecar(
        &self,
    ) -> Option<std::cell::RefMut<'_, dyn super::fm_sidecar::WidgetSidecar + 'static>> {
        let borrow = self.0.borrow_mut();
        std::cell::RefMut::filter_map(borrow, |b| b.inner.sidecar()).ok()
    }

    pub fn draw(&self, cv: &mut PixCanvas, x: i32, y: i32, aa: bool) {
        let f = Rc::clone(&self.0.borrow().fonts); // Rc 引用计数, 零堆分配
        self.0.borrow_mut().inner.draw(cv, x, y, &f, aa);
    }
}

impl HasPreferredSize for WidgetCell {
    fn preferred_size(&self) -> Dimension {
        // 与节点图的 RefCell 相互独立 (组件内省不回指节点图);
        // 字体从外壳注入 (trait 带参签名的无参桥)
        let b = self.0.borrow();
        if !b.is_visible() {
            return Dimension::new(0, 0); // hidden 塌缩优先于 override
        }
        if let Some((w, h)) = b.size_override() {
            return Dimension::new(w, h);
        }
        let f = Rc::clone(&b.fonts);
        b.inner.preferred_size(&f)
    }
}

impl HasVisibility for WidgetCell {
    fn is_visible(&self) -> bool {
        self.0.borrow().is_visible()
    }
}

// =====================================================================
// 注册表
// =====================================================================

/// palette 分类 (编辑器分组用)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetCategory {
    Text,
    Gauge,
    Chart,
    List,
    Composite,
    Decor,
}

/// 属性 schema 项 (W2 空; W4 编辑器 inspector 的表单定义)
#[derive(Debug, Clone, Copy)]
pub struct PropSchema {
    pub key: &'static str,
    pub display_zh: &'static str,
    pub kind: PropKind,
}

/// 属性类型 (W4 编辑器控件映射)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    Bool,
    Int,
    Str,
    Color,
    Target, // 数据绑定 (变量目录下拉)
    /// 枚举 (受限下拉)
    Enum(&'static [&'static str]),
}

/// 组件工厂签名 (props + 工厂环境 → 组件实例)
pub type WidgetFactory =
    fn(&serde_json::Value, &FactoryCtx) -> Result<Box<dyn HudWidget>, String>;

/// 组件元数据 (注册表条目)
pub struct WidgetMeta {
    /// 类型名 (命名空间 "core." / 预留 "ext." "user.")
    pub type_name: &'static str,
    /// palette 显示名
    pub display_zh: &'static str,
    pub category: WidgetCategory,
    /// 黑盒复合组件 (内部不可拆; palette 打标)
    pub composite: bool,
    /// 属性表单定义 (W2 空表)
    pub props_schema: &'static [PropSchema],
    /// 组件自读配置键 (页面 interest 键派生源; W3 接管手工键表)
    pub config_keys: &'static [&'static str],
    /// 数据依赖短名 (palette 提示/校验)
    pub data_shorts: &'static [&'static str],
    /// palette 新建组件的合法初值 (const JSON): 空值工厂 Err → 组件静默不建,
    /// 编辑器无从反馈 — 前端用此值兜底 (显示层预设可覆盖同名键)
    pub default_props: &'static str,
    pub factory: WidgetFactory,
}

/// 组件注册表 (编译期已知集合; 各族表拼接 — palette 展示序)
pub fn widget_registry() -> &'static [&'static WidgetMeta] {
    static REGISTRY: std::sync::OnceLock<Vec<&'static WidgetMeta>> = std::sync::OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            minihud::REGISTRY_ENTRIES
                .iter()
                .chain(std::iter::once(&data_field::DATA_FIELD_META))
                .chain(std::iter::once(&engine_gauge::ENGINE_GAUGE_META))
                .chain(gauges_composite::REGISTRY_ENTRIES.iter())
                .chain(gear_flaps_atom::REGISTRY_ENTRIES.iter())
                .chain(axes_atom::REGISTRY_ENTRIES.iter())
                .chain(fm_field::REGISTRY_ENTRIES.iter())
                .chain(fm_list::REGISTRY_ENTRIES.iter())
                .chain(fm_sidecar::REGISTRY_ENTRIES.iter())
                .collect()
        })
        .as_slice()
}

/// type_name 查表
pub fn lookup_widget(type_name: &str) -> Option<&'static WidgetMeta> {
    widget_registry().iter().find(|m| m.type_name == type_name).copied()
}
