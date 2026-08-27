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
