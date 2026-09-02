use super::*;
use crate::main_form::{update, Message};
use crate::renderers::test_util::{state_from_cfg, MapCtx};
use vm_core::config::config_loader::{ConfigValue, RowConfig};

fn switch_row(prop: Option<&str>, value: Option<bool>) -> RowConfig {
    let mut r = RowConfig::new("开关".into(), None, "%s".into());
    r.r#type = "SWITCH".into();
    r.property = prop.map(str::to_string);
    r.value = value.map(ConfigValue::Bool);
    r
}

fn inv_row(prop: &str, value: bool) -> RowConfig {
    let mut r = switch_row(Some(prop), Some(value));
    r.r#type = "SWITCH_INV".into();
    r
}

// SWITCH 写回: 非组字段 → write_bool 返回 false → row.value 回落 (Java L64-66)
#[test]
fn apply_switch_falls_back_to_row_value() {
    let mut panel = GroupConfig::new("p".into());
    let mut r = switch_row(Some("crosshairSwitch"), Some(true));
    r.value = Some(ConfigValue::Bool(true));
    panel.rows.push(r);
    let ctx = MapCtx::default();

    apply(&mut panel, "crosshairSwitch", false, &ctx);
    let row = row_by_path(&panel.rows, &[0]).unwrap();
    assert_eq!(row.value, Some(ConfigValue::Bool(false)));
    // write_bool 总同步服务 (RendererConfigHelper.java:127-134)
    assert_eq!(*ctx.calls.borrow(), vec!["sync:crosshairSwitch=false".to_string(), "on_save".to_string()]);
}

// SWITCH_INV 写回: 显示 true → 存 false; row.value=显示值 (Java L33-38)
#[test]
fn apply_switch_inv_stores_inverted() {
    let mut panel = GroupConfig::new("p".into());
    panel.rows.push(inv_row("disableLabel", false));
    let ctx = MapCtx::default();

    apply(&mut panel, "disableLabel", true, &ctx);
    let row = row_by_path(&panel.rows, &[0]).unwrap();
    assert_eq!(row.value, Some(ConfigValue::Bool(true))); // row.value=显示值
    assert_eq!(
        *ctx.calls.borrow(),
        vec!["sync:disableLabel=false".to_string(), "on_save".to_string()]
    );
}

// 组字段绑定开关: write_bool 命中 PropertyBinder, row.value 不回落
#[test]
fn apply_switch_binds_group_field() {
    let mut panel = GroupConfig::new("p".into());
    panel.rows.push(switch_row(Some("visible"), Some(false)));
    let ctx = MapCtx::default();

    apply(&mut panel, "visible", true, &ctx);
    assert!(panel.visible, "PropertyBinder 写组字段 visible");
    assert_eq!(panel.rows[0].value, Some(ConfigValue::Bool(false)), "绑定成功不回落 row.value");
}

// 未命中 key: 无副作用无 panic (消息域外防护, Java 闭包捕获无此面)
#[test]
fn apply_unknown_key_is_noop() {
    let mut panel = GroupConfig::new("p".into());
    panel.rows.push(switch_row(Some("k"), Some(true)));
    let ctx = MapCtx::default();
    apply(&mut panel, "absent", false, &ctx);
    assert_eq!(panel.rows[0].value, Some(ConfigValue::Bool(true)));
    assert!(ctx.calls.borrow().is_empty());
}

// 真实链: DATA 开关经 Message::Toggle → switch::apply — 服务值 + 快照行值
// (ui_layout.cfg 实况: "示空速/表速/IAS" :type data :target getIAS :value true)
// 路由等价备案 (原 data.rs): Java DATA 行经 syncStringToConfigService + onSave
// 无 PropertyBinder; Rust 走 switch::apply 的 write_bool → sync_to_config_service
// 同串同链 (DATA :target 皆遥测 getter, 恒不绑组字段), 终态一致。
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
