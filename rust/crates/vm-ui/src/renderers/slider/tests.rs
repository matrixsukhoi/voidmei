use super::*;
use crate::renderers::test_util::MapCtx;
use vm_core::config_loader::ConfigValue;

fn slider_row(prop: Option<&str>, value: i32, min: i32, max: i32) -> RowConfig {
    let mut r = RowConfig::new("大小".into(), None, "%s".into());
    r.r#type = "SLIDER".into();
    r.property = prop.map(str::to_string);
    r.value = Some(ConfigValue::Int(value));
    r.min_val = min;
    r.max_val = max;
    r
}

// Java L33-37: min >= max → max = min + 100
#[test]
fn effective_range_guard() {
    assert_eq!(effective_range(0, 100), (0, 100));
    assert_eq!(effective_range(5, 5), (5, 105));
    assert_eq!(effective_range(10, 0), (10, 110));
    assert_eq!(effective_range(-6, 20), (-6, 20));
}

// 读链: 服务值钳入 [min,max] (Java L40-43 Clamp)
#[test]
fn read_current_clamps_service_value() {
    let mut ctx = MapCtx::default();
    let panel = GroupConfig::new("p".into());
    // 服务值 500 超 max=100 → 100
    let row = slider_row(Some("scale"), 50, 0, 100);
    ctx.set("scale", "500");
    assert_eq!(read_current(&row, &panel, &ctx), 100);
    // 服务值 -30 低于 min=0 → 0
    ctx.set("scale", "-30");
    assert_eq!(read_current(&row, &panel, &ctx), 0);
    // 服务值正常区间直通
    ctx.set("scale", "42");
    assert_eq!(read_current(&row, &panel, &ctx), 42);
}

// 读链: PropertyBinder 组字段 (fontSize) 压制服务同键; 空服务回落 row 默认
#[test]
fn read_current_group_field_and_default() {
    let mut panel = GroupConfig::new("MiniHUD".into());
    panel.font_size = 7;
    let mut ctx = MapCtx::default();
    ctx.set("fontSize", "99"); // 服务值应被组字段压制
    let row = slider_row(Some("fontSize"), 3, -10, 10);
    assert_eq!(read_current(&row, &panel, &ctx), 7);
    // 无绑定时回落 row 默认 (服务空)
    let row2 = slider_row(Some("absent"), 4, 0, 100);
    let ctx2 = MapCtx::default();
    assert_eq!(read_current(&row2, &panel, &ctx2), 4);
}

// 读链: 非守卫区间 (min==max) 下钳位基准 = min+100 (effective_range 联动)
#[test]
fn read_current_uses_guarded_range() {
    let mut ctx = MapCtx::default();
    ctx.set("k", "150");
    let panel = GroupConfig::new("p".into());
    let row = slider_row(Some("k"), 0, 5, 5); // min>=max → 实际 (5,105)
    assert_eq!(read_current(&row, &panel, &ctx), 105);
}

// 写链: row.value + 组字段 (fontSize) + 服务同步; 不触发 on_save (拖拽时机)
#[test]
fn apply_updates_row_group_field_and_service() {
    let mut panel = GroupConfig::new("MiniHUD".into());
    panel.rows.push(slider_row(Some("fontSize"), 0, -10, 10));
    let ctx = MapCtx::default();

    apply(&mut panel, "fontSize", 9, &ctx);
    assert_eq!(panel.rows[0].value, Some(ConfigValue::Int(9)));
    assert_eq!(panel.font_size, 9, "PropertyBinder 写组字段");
    // write_int 同步: syncStr:fontSize=9 (RendererConfigHelper.java:109-114)
    assert_eq!(*ctx.calls.borrow(), vec!["syncStr:fontSize=9".to_string()]);
}

// panelColumns 特例触发 onRebuild (Java L62-64); 普通键不触发
#[test]
fn apply_panel_columns_triggers_rebuild() {
    let mut panel = GroupConfig::new("p".into());
    panel.rows.push(slider_row(Some("panelColumns"), 2, 1, 4));
    let ctx = MapCtx::default();
    apply(&mut panel, "panelColumns", 3, &ctx);
    assert!(ctx.calls.borrow().contains(&"on_rebuild".to_string()));
    assert_eq!(panel.panel_columns, 3);

    panel.rows.push(slider_row(Some("plain"), 0, 0, 10));
    ctx.calls.borrow_mut().clear();
    apply(&mut panel, "plain", 5, &ctx);
    assert!(!ctx.calls.borrow().contains(&"on_rebuild".to_string()));
}
