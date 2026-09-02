use super::*;
use crate::renderers::test_util::MapCtx;
use vm_core::config::config_loader::{ConfigValue, RowConfig};

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

// 读链已删 (D9 回显走整树 DTO); 钳位/优先级语义由 web 壳 JS 与
// renderer_config_helper 的 read_int 测试各自锁定。

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
