//! 属性表单行模型 (D4b 自 voidmei edit_chrome 下沉): 组件/页面 doc → 属性行
//! 定义 (label + 控件分型 + 当前值文本)。纯计算 (schema→行映射 + 值三兜底),
//! rect/hits 布局留在调用方 (编辑 chrome 命中/绘制共源)。
//!
//! 特殊键约定: "__id"/"__enabled"/"__visible_when" = 组件通用行;
//! "__page_name"/"__page_padding"/"__page_font_add" = 页面属性行。

use kernel::config::json_model::{ComponentDoc, PageDoc};

use super::registry::{PropKind, WidgetMeta};

/// 属性行控件分型 (Text=输入框失焦提交; 其余自绘每击提交)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropCtrlKind {
    Text,
    IntStepper,
    Toggle,
    EnumCycle,
}

/// 属性行定义 (纯数据; 布局由调用方补)
#[derive(Debug, Clone, PartialEq)]
pub struct PropRowDef {
    pub label: String,
    pub key: String,
    pub kind: PropCtrlKind,
    /// EnumCycle 候选值 (其余分型恒空)
    pub enum_values: Vec<&'static str>,
    pub value_text: String,
}

/// 选中组件的属性行集: 通用 3 行 (标识/显示/显示条件) + 注册表 schema 行
/// (Str/Color/Target→Text, Int→步进, Bool→开关, Enum→循环)。
/// meta = None (未注册类型) 时仅通用行
pub fn component_prop_rows(
    comp: &ComponentDoc,
    meta: Option<&'static WidgetMeta>,
) -> Vec<PropRowDef> {
    let mk = |label: &str, key: &str, kind: PropCtrlKind, value: String| PropRowDef {
        label: label.to_string(),
        key: key.to_string(),
        kind,
        enum_values: Vec::new(),
        value_text: value,
    };
    let mut v = vec![
        mk("标识", "__id", PropCtrlKind::Text, comp.id.clone()),
        mk(
            "显示",
            "__enabled",
            PropCtrlKind::Toggle,
            comp.enabled.to_string(),
        ),
        mk(
            "显示条件",
            "__visible_when",
            PropCtrlKind::Text,
            comp.visible_when.clone().unwrap_or_default(),
        ),
    ];
    let Some(m) = meta else {
        return v;
    };
    for p in m.props_schema {
        let (kind, values) = match p.kind {
            PropKind::Str | PropKind::Color | PropKind::Target => (PropCtrlKind::Text, Vec::new()),
            PropKind::Int => (PropCtrlKind::IntStepper, Vec::new()),
            PropKind::Bool => (PropCtrlKind::Toggle, Vec::new()),
            PropKind::Enum(values) => (PropCtrlKind::EnumCycle, values.to_vec()),
        };
        v.push(PropRowDef {
            label: p.display_zh.to_string(),
            key: p.key.to_string(),
            kind,
            enum_values: values,
            value_text: prop_value_text(comp, Some(m), p.key),
        });
    }
    v
}

/// 页面属性行集 (无选中时的表单区): 页名 / 内边距 / 字号增量
pub fn page_prop_rows(page: &PageDoc) -> Vec<PropRowDef> {
    vec![
        PropRowDef {
            label: "页名".into(),
            key: "__page_name".into(),
            kind: PropCtrlKind::Text,
            enum_values: Vec::new(),
            value_text: page.name.clone(),
        },
        PropRowDef {
            label: "内边距".into(),
            key: "__page_padding".into(),
            kind: PropCtrlKind::IntStepper,
            enum_values: Vec::new(),
            value_text: page.padding.to_string(),
        },
        PropRowDef {
            label: "字号增量".into(),
            key: "__page_font_add".into(),
            kind: PropCtrlKind::IntStepper,
            enum_values: Vec::new(),
            value_text: page.font.size_add.to_string(),
        },
    ]
}

/// schema 属性键的当前值文本 (行显示/步进/循环共源):
/// 实值 → default_props 兜底 → 类型零值 (schema 缺项按 string 空)
pub fn prop_value_text(
    comp: &ComponentDoc,
    meta: Option<&'static WidgetMeta>,
    key: &str,
) -> String {
    if let Some(v) = comp.props.get(key).filter(|v| !v.is_null()) {
        return json_value_text(v);
    }
    if let Some(m) = meta {
        if let Ok(def) = serde_json::from_str::<serde_json::Value>(m.default_props) {
            if let Some(v) = def.get(key) {
                return json_value_text(v);
            }
        }
    }
    match meta
        .and_then(|m| m.props_schema.iter().find(|p| p.key == key))
        .map(|p| p.kind)
    {
        Some(PropKind::Int) => "0".into(),
        Some(PropKind::Bool) => "false".into(),
        _ => String::new(),
    }
}

/// JSON 值 → 显示文本 (props 实值/default 值共用)
fn json_value_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}
