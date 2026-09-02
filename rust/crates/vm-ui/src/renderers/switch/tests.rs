use super::*;
use crate::renderers::test_util::MapCtx;
use vm_core::config::config_loader::ConfigValue;

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

// SWITCH 读路径: 服务值压制 row 默认 (SwitchRowRenderer.java:30-33)
#[test]
fn read_display_switch_prefers_service() {
    let row = switch_row(Some("showSpeedBar"), Some(false)); // 默认 false
    let mut ctx = MapCtx::default();
    ctx.set("showSpeedBar", "true");
    let panel = GroupConfig::new("p".into());
    assert!(read_display(&row, &panel, &ctx));
    // 服务空值 → 回落 row.getBool() 默认
    let ctx2 = MapCtx::default();
    assert!(!read_display(&row, &panel, &ctx2));
}

// SWITCH 读路径: PropertyBinder 组字段 (visible) 压制服务同键 (read_bool 优先级 1)
#[test]
fn read_display_switch_group_field_wins() {
    let mut row = switch_row(Some("visible"), Some(false));
    row.value = Some(ConfigValue::Bool(false));
    let mut panel = GroupConfig::new("p".into());
    panel.visible = true; // 组字段
    let mut ctx = MapCtx::default();
    ctx.set("visible", "false"); // 服务侧 false 应被组字段压制
    assert!(read_display(&row, &panel, &ctx));
}

// SWITCH_INV 读路径: 双重取反 (配置 true=禁用 → 显示 OFF)
#[test]
fn read_display_switch_inv_double_inversion() {
    let row = inv_row("disableX", true); // row.value=true (显示值语义)
    let mut ctx = MapCtx::default();
    ctx.set("disableX", "true"); // 服务: 禁用
    let panel = GroupConfig::new("p".into());
    assert!(!read_display(&row, &panel, &ctx), "disableX=true → 显示 OFF");
    ctx.set("disableX", "false");
    assert!(read_display(&row, &panel, &ctx), "disableX=false → 显示 ON");
    // 服务空 → !(!row.getBool()) = row.getBool()
    let ctx2 = MapCtx::default();
    assert!(read_display(&row, &panel, &ctx2));
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
