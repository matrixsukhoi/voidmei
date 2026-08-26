//! DataRowRenderer 的 iced 语义复刻 (src/ui/layout/renderer/DataRowRenderer.java)。
//!
//! Java 语义: DATA 行是**开关** (WebSwitch, 控制行可见性/字段启停), 不是只读显示
//! (DataRowRenderer.java:20-21 注释 "data toggles"; 仅 RowRendererRegistry 未注册
//! 类型的兜底渲染才落本渲染器)。mod.rs 的未知类型默认路由亦走本 view_row —
//! 对位 Java defaultRenderer=DataRowRenderer 的可交互开关面。
//!
//! 交互语义保真 (Java L20-34):
//! - 读: display = Boolean.parseBoolean(getStringFromConfigService(prop ?? label, ""))
//!   — 空 (键无命中) → false; 非 PropertyBinder 链。
//! - 写: sw 监听 syncStringToConfigService(prop ?? label, String.valueOf(选中)) +
//!   onSave — 无 PropertyBinder、无 row.value 直写 (服务侧 setConfig 的
//!   update_rows_recursive 负责行值)。
//!
//! PORT(路由等价备案): 本视图发 Message::Toggle → main_form::update 的 Toggle 臂现
//! 路由 switch::apply (main_form.rs 本批禁改, 无 DATA 专属臂)。两者终态等价:
//! 1. switch::apply 的 write_bool → sync_to_config_service(key, "true"/"false") =
//!    Java syncStringToConfigService 同串同链 (含 enableFMPrint 特例同在
//!    DynamicDataPage 上下文实现);
//! 2. Java 不经 PropertyBinder — Rust set_bool 对非组字段 :target 恒不绑定
//!    (ui_layout.cfg 的 DATA :target 皆遥测 getter/getIAS 等 + 端口, 无组字段名);
//! 3. Java 不直写 row.value — setConfig 递归更新服务树行值 (instanceof Boolean →
//!    Bool), Rust switch::apply 直写快照后被 with_panel 的 mirror 以服务树值回拷,
//!    终态一致。
//! 若未来 DATA 行 :target 撞组字段名 (fontSize 等), Rust 会多写组字段 (Java 不会)
//! — 接线批可加 DATA 专属消息臂调本文件语义收敛差异。

use iced::widget::{checkbox, Row};
use iced::Element;
use vm_core::config_loader::RowConfig;
use vm_core::row_renderer_registry::RenderContext;

use crate::main_form::Message;

/// Java Boolean.parseBoolean: 仅忽略大小写的 "true" 为真
fn java_parse_boolean(s: &str) -> bool {
    s.eq_ignore_ascii_case("true")
}

/// 显示值 (Java L20): parseBoolean(getStringFromConfigService(prop ?? label, ""))。
pub fn read_display(row: &RowConfig, ctx: &dyn RenderContext) -> bool {
    // Java L20/27: property null 时以 label 为键
    let key = row.property.as_deref().unwrap_or(&row.label);
    java_parse_boolean(&ctx.get_string_from_config_service(key, ""))
}

/// DATA 行视图: 开关 (Java createSwitchItem + WebSwitch)。
pub fn view_row<'a>(
    row: &'a RowConfig,
    ctx: &dyn RenderContext,
    panel_title: &'a str,
) -> Element<'a, Message> {
    let display = read_display(row, ctx);
    // 消息键: :target 优先, 无 :target 以 label (Java L20/27 同一取键规则; label
    // 亦空的双空残端 Java 会向 "" 键写值 — 无意义写, 折叠为纯展示)
    let key = row
        .property
        .clone()
        .or_else(|| (!row.label.is_empty()).then(|| row.label.clone()));
    let cb = match key {
        // value = 新选中态 (Java String.valueOf(sw.isSelected()))
        Some(key) => {
            let p = panel_title.to_string();
            checkbox(row.label.clone(), display)
                .on_toggle(move |v| Message::Toggle { panel: p.clone(), key: key.clone(), value: v })
        }
        None => checkbox(row.label.clone(), display),
    };
    Row::with_children(vec![cb.into()]).spacing(8).into()
}

// =====================================================================
// Tests — 真实链路 (main_form::update → switch::apply), 见路由等价备案
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::main_form::{update, Message};
    use crate::renderers::test_util::{state_from_cfg, MapCtx};
    use vm_core::config_loader::ConfigValue;

    fn data_row(prop: Option<&str>, value: Option<bool>) -> RowConfig {
        let mut r = RowConfig::new("表速".into(), None, "%s".into());
        r.r#type = "DATA".into();
        r.property = prop.map(str::to_string);
        r.value = value.map(ConfigValue::Bool);
        r.unit = "Km/h".into();
        r
    }

    // 读链: 行值经服务可读 (get_config 命中行 → "true"); 键无命中 → "" → false
    // (Java L20 的空默认)
    #[test]
    fn read_display_service_and_empty() {
        let row = data_row(Some("getIAS"), Some(true));
        let mut ctx = MapCtx::default();
        ctx.set("getIAS", "true");
        assert!(read_display(&row, &ctx));
        ctx.set("getIAS", "false");
        assert!(!read_display(&row, &ctx));
        // 键无命中 → 默认 "" → parseBoolean("") = false
        assert!(!read_display(&row, &MapCtx::default()));
        // 大小写不敏感 (Boolean.parseBoolean 语义)
        let mut ctx2 = MapCtx::default();
        ctx2.set("getIAS", "TRUE");
        assert!(read_display(&row, &ctx2));
    }

    // 读链: 无 :target 以 label 为键 (Java L20/27)
    #[test]
    fn read_display_label_key() {
        let row = data_row(None, None);
        let mut ctx = MapCtx::default();
        ctx.set("表速", "true");
        assert!(read_display(&row, &ctx));
        assert!(!read_display(&row, &MapCtx::default()));
    }

    // 真实链: DATA 开关经 Message::Toggle → switch::apply — 服务值 + 快照行值
    // (ui_layout.cfg 实况: "示空速/表速/IAS" :type data :target getIAS :value true)
    #[test]
    fn toggle_message_routes_data_write_chain() {
        let mut state = state_from_cfg(
            "data_route",
            r#"(panel "数据" (item "表速" :type data :target-name "表  速" :target "getIAS" :unit "Km/h" :value true :default true))"#,
            None,
        );
        update(
            &mut state,
            Message::Toggle { panel: "数据".into(), key: "getIAS".into(), value: false },
        );
        assert_eq!(state.service_string("getIAS"), "false");
        // 服务树行值 Bool(false) (setConfig instanceof Boolean 分支), mirror 回快照
        assert_eq!(
            state.snapshot_row("数据", "getIAS").unwrap().value,
            Some(ConfigValue::Bool(false))
        );
    }

    // 真实链: DATA 开为 true → 服务 "true" (往返)
    #[test]
    fn toggle_message_routes_data_on() {
        let mut state = state_from_cfg(
            "data_on",
            r#"(panel "数据" (item "马赫数" :type data :target "getMach" :precision 2 :value true))"#,
            None,
        );
        update(
            &mut state,
            Message::Toggle { panel: "数据".into(), key: "getMach".into(), value: false },
        );
        update(
            &mut state,
            Message::Toggle { panel: "数据".into(), key: "getMach".into(), value: true },
        );
        assert_eq!(state.service_string("getMach"), "true");
        assert!(state.snapshot_row("数据", "getMach").unwrap().get_bool());
    }

    // 视图构建冒烟 (键控/无键两形态)
    #[test]
    fn view_row_builds() {
        let ctx = MapCtx::default();
        let r1 = data_row(Some("getIAS"), Some(true));
        let _el = view_row(&r1, &ctx, "数据");
        let r2 = data_row(None, None);
        let _el2 = view_row(&r2, &ctx, "数据");
    }
}
