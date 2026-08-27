//! TextRowRenderer 的读链复刻 (src/ui/layout/renderer/TextRowRenderer.java,
//! registry 键 INPUT/TEXT 两个别名共用, RowRendererRegistry.java:25-26)。
//!
//! **D9 变更**: 原 iced view_row 已删 (渲染归 vm-webui web 壳), 本模块仅存读链
//! (read_current); 写回无独立函数 — 与 ComboRowRenderer 的闭包体 (L52-62) 逐步
//! 同构, 直接复用 [`super::combo::apply`] (经 Message::Combo 路由, main_form::update
//! 已接线, 见 tests 的真实链用例)。
//!
//! 交互语义保真:
//! - 读 (Java L20-30): 默认 = row.value!=null ? row.getStr() : "";
//!   readString 优先级 PropertyBinder 组字段 → 服务 → 默认 (与 combo 同链)。
//!
//! PORT(提交时机备案): Java 在 Enter/失焦提交 (L37-49); D1 期 iced 无失焦消息 →
//! 逐键提交 (终态等价)。D9 后提交时机归 web 壳 (JS 输入框), Message::Combo
//! 消息形状不变。

use vm_core::config_loader::{GroupConfig, RowConfig};
use vm_core::renderer_config_helper;
use vm_core::row_renderer_registry::RenderContext;

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

// =====================================================================
// Tests — 真实链路 (main_form::update → combo::apply), 无 mock 造假
// =====================================================================
#[cfg(test)]
mod tests;
