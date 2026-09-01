use super::*;

/// Java 字段声明默认值: visible=true / currentValue="---" / length=0 /
/// precision=0 / format=null / 各 supplier null
#[test]
fn initial_state() {
    let df = DataField::new("getIAS", "表  速", "Km/h", "disableFlightInfoIAS", true, false);
    assert_eq!(df.key, "getIAS");
    assert_eq!(df.label, "表  速");
    assert_eq!(df.unit, "Km/h");
    assert_eq!(df.config_key, "disableFlightInfoIAS");
    assert!(df.hide_when_na);
    assert!(!df.hide_when_zero);
    assert!(df.visible);
    assert_eq!(df.current_value, "---");
    assert_eq!(df.buffer, "");
    assert_eq!(df.length, 0);
    assert_eq!(df.precision, 0);
    assert!(df.format.is_none());
    assert!(df.value_supplier.is_none());
    assert!(df.visibility_supplier.is_none());
    assert!(df.unit_supplier.is_none());
    assert!(df.precision_supplier.is_none());
}

/// Java String.format("%5s", v): 右对齐宽 5, 不足补前导空格, 超宽原样
#[test]
fn set_value_right_aligns_width_5() {
    let mut df = DataField::new("k", "l", "u", "c", false, false);
    df.set_value("123");
    assert_eq!(df.current_value, "  123");
    df.set_value("N/A");
    assert_eq!(df.current_value, "  N/A");
    df.set_value("-");
    assert_eq!(df.current_value, "    -");
    // 恰好 5 位: 不补不截
    df.set_value("12345");
    assert_eq!(df.current_value, "12345");
    // 超宽: 原样保留 (Java %5s 不截断)
    df.set_value("123456");
    assert_eq!(df.current_value, "123456");
}

#[test]
fn set_unit_replaces() {
    let mut df = DataField::new("k", "l", "Ata", "c", false, false);
    df.set_unit("P/XX.X''");
    assert_eq!(df.unit, "P/XX.X''");
}

/// hideWhenNA=true: value 与 naString 相等 → 隐藏; 不等 → 显示
#[test]
fn set_value_with_visibility_na_match() {
    let mut df = DataField::new("k", "l", "u", "c", true, false);
    df.set_value_with_visibility("-", "-");
    assert!(!df.visible, "NA 值应触发隐藏");
    assert_eq!(df.current_value, "    -");
    df.set_value_with_visibility("800", "-");
    assert!(df.visible, "非 NA 值应显示");
    assert_eq!(df.current_value, "  800");
}

/// hideWhenNA=false: visible 不被触碰 (Java 无 else 分支, 预置 false 也保持)
#[test]
fn set_value_with_visibility_no_na_flag_leaves_visible() {
    let mut df = DataField::new("k", "l", "u", "c", false, false);
    df.set_value_with_visibility("-", "-");
    assert!(df.visible);
    // 预置 false (模拟前一轮隐藏) 后再更新: 仍不被翻回
    df.visible = false;
    df.set_value_with_visibility("800", "-");
    assert!(!df.visible, "hideWhenNA=false 时 visible 原样保持");
}
