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
        field.into(),
    ])
    .spacing(8)
    .into()
}

// =====================================================================
// Tests — 真实链路 (main_form::update → combo::apply), 无 mock 造假
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::main_form::{update, Message};
    use crate::renderers::test_util::{state_from_cfg, MapCtx};
    use vm_core::config_loader::ConfigValue;

    fn text_row(prop: Option<&str>, value: Option<&str>) -> RowConfig {
        let mut r = RowConfig::new("端口".into(), None, "%s".into());
        r.r#type = "INPUT".into();
        r.property = prop.map(str::to_string);
        r.value = value.map(|v| ConfigValue::Str(v.to_string()));
        r
    }

    // 读链: 服务值压制 row 默认 (Java L27); 空服务回落默认; 无 :target 恒默认
    #[test]
    fn read_current_service_default_and_unkeyed() {
        let panel = GroupConfig::new("p".into());
        let row = text_row(Some("httpPort"), Some("8111"));
        let mut ctx = MapCtx::default();
        ctx.set("httpPort", "9222");
        assert_eq!(read_current(&row, &panel, &ctx), "9222");
        let ctx2 = MapCtx::default();
        assert_eq!(read_current(&row, &panel, &ctx2), "8111");
        // Java L20: row.value null → 默认 ""
        let row2 = text_row(Some("k"), None);
        assert_eq!(read_current(&row2, &panel, &MapCtx::default()), "");
        // Java L29: prop=null → 默认直出 (不经服务)
        let row3 = text_row(None, Some("v"));
        let mut ctx3 = MapCtx::default();
        ctx3.set("端口", "服务值");
        assert_eq!(read_current(&row3, &panel, &ctx3), "v");
    }

    // 读链: PropertyBinder 组字段 (fontName) 压制服务同键 (Java L24-25)
    #[test]
    fn read_current_group_field_wins() {
        let mut panel = GroupConfig::new("引擎信息".into());
        panel.font_name = Some("DIN Pro 400".into());
        let mut ctx = MapCtx::default();
        ctx.set("fontName", "Arial");
        let row = text_row(Some("fontName"), Some("X"));
        assert_eq!(read_current(&row, &panel, &ctx), "DIN Pro 400");
    }

    // 真实链: INPUT 行经 Message::Combo 写回 — 服务 + 快照 + on_save 即落盘
    // (ui_layout.cfg 实况: "8111端口" :type input :target httpPort)
    #[test]
    fn combo_message_routes_text_write_chain() {
        let mut state = state_from_cfg(
            "text_route",
            r#"(panel "连接" (item "8111端口" :type input :target "httpPort" :value 8111 :default 8111))"#,
            None,
        );
        update(
            &mut state,
            Message::Combo { panel: "连接".into(), key: "httpPort".into(), value: "9222".into() },
        );
        assert_eq!(state.service_string("httpPort"), "9222");
        // Int 行经 setConfig 保持 Int 形态 (Java instanceof Integer 分支)
        assert_eq!(state.snapshot_row("连接", "httpPort").unwrap().get_int(), 9222);
    }

    // 真实链: 无 :target 文本行 — row.value 落快照 + onSave 即落盘 (persist 收敛
    // 服务树; Java L57-67: prop=null 不落服务, row.value 在共享树本体上)
    #[test]
    fn combo_message_unkeyed_row_writes_row_value_only() {
        let persist = std::env::temp_dir().join("vm_ui_text_unkeyed_user.cfg");
        let _ = std::fs::remove_file(&persist);
        let mut state = state_from_cfg(
            "text_unkeyed",
            r#"(panel "P" (item "备注" :type text :value "旧"))"#,
            Some(persist.to_string_lossy().into_owned()),
        );
        update(
            &mut state,
            Message::Combo { panel: "P".into(), key: "备注".into(), value: "新".into() },
        );
        let row = state.snapshot_row("P", "备注").unwrap();
        assert_eq!(row.value, Some(ConfigValue::Str("新".into())));
        assert!(persist.exists(), "onSave 即落盘");
        // 挂起重放 → 落盘 → 重载: 服务树同 label 行持 "新" (get_config 按 label 命中)
        assert_eq!(state.service_string("备注"), "新");
        let _ = std::fs::remove_file(&persist);
    }

    // 视图构建冒烟 (键控/无键两形态)
    #[test]
    fn view_row_builds() {
        let panel = GroupConfig::new("p".into());
        let ctx = MapCtx::default();
        let r1 = text_row(Some("httpPort"), Some("8111"));
        let _el = view_row(&r1, &panel, &ctx, "连接");
        let r2 = text_row(None, Some("v"));
        let _el2 = view_row(&r2, &panel, &ctx, "连接");
    }
}
