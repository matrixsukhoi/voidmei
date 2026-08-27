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
