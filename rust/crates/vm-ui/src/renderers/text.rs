//! TextRowRenderer 的 iced 语义复刻 (src/ui/layout/renderer/TextRowRenderer.java,
//! registry 键 INPUT/TEXT 两个别名共用, RowRendererRegistry.java:25-26)。
//!
//! 交互语义保真:
//! - 读 (Java L20-30): 默认 = row.value!=null ? row.getStr() : "";
//!   readString 优先级 PropertyBinder 组字段 → 服务 → 默认 (与 combo 同链)。
//! - 写 (Java updateValue L53-71): row.value=新串 → PropertyBinder.setString →
//!   服务同步 (prop 非 null) → onSave — 与 ComboRowRenderer 的闭包体 (L52-62)
//!   逐步同构, 故写回直接复用 [`super::combo::apply`] (经 Message::Combo 路由,
//!   main_form::update 已接线), 见 view_row 的消息转发注释。
//!
//! PORT(提交时机分歧备案): Java 在 Enter/失焦提交 (L37-49); iced 无失焦消息且
//! 消息枚举本批冻结 (规格无 TextEdited 变体) → on_input 逐键提交 (终态等价,
//! 代价是逐键触发保存链落盘; 接线批可为 draft 状态 + on_submit 收敛回失焦语义)。

use iced::widget::{text, text_input, Row};
use iced::{Element, Length};
use vm_core::config_loader::{GroupConfig, RowConfig};
use vm_core::renderer_config_helper;
use vm_core::row_renderer_registry::RenderContext;

use crate::main_form::Message;

/// 显示值 (Java L20-30 读链)。
pub fn read_current(row: &RowConfig, panel: &GroupConfig, ctx: &dyn RenderContext) -> String {
    // Java L20: defaultVal = (row.value != null) ? row.getStr() : ""
    let default_val = match &row.value {
        Some(_) => row.get_str(),
        None => String::new(),
    };
    // Java L22-30: PropertyBinder 组字段 → 服务 → 默认 (= read_string 同链)
    renderer_config_helper::read_string(ctx, panel, row, &default_val)
}

/// 文本行视图: [label | 输入框] (Java createTextItem 布局, 列宽 15)。
pub fn view_row<'a>(
    row: &'a RowConfig,
    panel: &'a GroupConfig,
    ctx: &dyn RenderContext,
    panel_title: &'a str,
) -> Element<'a, Message> {
    let current = read_current(row, panel, ctx);
    // 消息键: :target 优先; 无 :target 以 label 为键 (Java L57-67: prop=null 仍写
    // row.value + PropertyBinder.setString(null) 无操作, 不落服务); 双空残端 → 只读
    let key = row
        .property
        .clone()
        .or_else(|| (!row.label.is_empty()).then(|| row.label.clone()));
    let field: Element<'a, Message> = match key {
        // 消息转发: 复用 Message::Combo (值串写链与 Java updateValue 逐步同构,
        // 见模块文档); panel 携带定位 — 与其余渲染器的键控消息一致
        Some(key) => text_input("", &current)
            .on_input(move |s| Message::Combo {
                panel: panel_title.to_string(),
                key: key.clone(),
                value: s,
            })
            .into(),
        None => text_input("", &current).into(), // 无 on_input → 禁用态
    };
    Row::with_children(vec![
        text(row.label.clone()).width(Length::Fill).into(),
        field,
    ])
    .spacing(8)
    .into()
}

// =====================================================================
// Tests — 真实链路 (main_form::update → combo::apply), 无 mock 造假
// =====================================================================
#[cfg(test)]
mod tests;
