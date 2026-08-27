use super::*;

/// 桩默认值须与 Java 字段声明默认一致 —— populateFromGroup 的
/// "DATA".equals(row.type) / previewValue==null → "-" 等分支语义依赖这些默认。
#[test]
fn row_config_defaults_match_java() {
    let row = RowConfig::new("转半径");
    assert_eq!(row.label, "转半径");
    assert_eq!(row.target_name, None);
    assert_eq!(row.unit, "");
    assert_eq!(row.preview_value, None);
    assert!(!row.hide_when_zero);
    assert_eq!(row.r#type, "DATA");
    assert_eq!(row.property, None);
    assert!(row.children.is_empty());
}

#[test]
fn group_config_new() {
    let gc = GroupConfig::new("飞行信息");
    assert_eq!(gc.title, "飞行信息");
    assert!(gc.rows.is_empty());
}
