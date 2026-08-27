use crate::ui_model::config_stub::RowConfig;
use crate::ui_model::field_manager::test_support::MockConfigProvider;

use super::*;

#[test]
fn create_default_with_null_inputs_keeps_defaults() {
    let cfg = FlightInfoConfig::create_default(None, None);
    assert_eq!(cfg.title, "FlightInfo");
    assert!(!cfg.show_edge);
    assert_eq!(cfg.column_num, 3);
    assert_eq!(cfg.num_font_key, "GlobalNumFont");
    assert_eq!(cfg.label_font_key, "flightInfoFontC");
    assert_eq!(cfg.column_key, "flightInfoColumn");
    assert_eq!(cfg.pos_x_key, "flightInfoX");
    assert_eq!(cfg.pos_y_key, "flightInfoY");
    assert_eq!(cfg.edge_key, "flightInfoEdge");
    assert!(cfg.group_config.is_none());
    assert!(cfg.get_field_definitions().is_empty());
}

/// edgeVal != null 即执行赋值: 空串 → false; "true" → true; 大小写敏感
#[test]
fn edge_switch_parsing() {
    let mut mock = MockConfigProvider::new();
    mock.values.insert("flightInfoEdge".to_string(), "true".to_string());
    let cfg = FlightInfoConfig::create_default(Some(&mock), None);
    assert!(cfg.show_edge);

    let mut mock = MockConfigProvider::new();
    mock.values.insert("flightInfoEdge".to_string(), "false".to_string());
    assert!(!FlightInfoConfig::create_default(Some(&mock), None).show_edge);

    // key 未设置 (Java null) → 保持默认 false
    assert!(!FlightInfoConfig::create_default(Some(&MockConfigProvider::new()), None).show_edge);

    // 空串: Java edgeVal != null 成立 → "true".equals("") = false
    let mut mock = MockConfigProvider::new();
    mock.values.insert("flightInfoEdge".to_string(), String::new());
    assert!(!FlightInfoConfig::create_default(Some(&mock), None).show_edge);

    // 大小写敏感
    let mut mock = MockConfigProvider::new();
    mock.values.insert("flightInfoEdge".to_string(), "TRUE".to_string());
    assert!(!FlightInfoConfig::create_default(Some(&mock), None).show_edge);
}

/// columnNum: 合法数字生效; 非法/空白 → 3; 空串/未设置 → 跳过保持 3
#[test]
fn column_parsing() {
    let mut mock = MockConfigProvider::new();
    mock.values.insert("flightInfoColumn".to_string(), "5".to_string());
    assert_eq!(FlightInfoConfig::create_default(Some(&mock), None).column_num, 5);

    for bad in ["abc", " 4", "3.5", "1e2"] {
        let mut mock = MockConfigProvider::new();
        mock.values.insert("flightInfoColumn".to_string(), bad.to_string());
        assert_eq!(
            FlightInfoConfig::create_default(Some(&mock), None).column_num,
            3,
            "非法值 {bad} 应回落 3"
        );
    }

    let mut mock = MockConfigProvider::new();
    mock.values.insert("flightInfoColumn".to_string(), String::new());
    assert_eq!(FlightInfoConfig::create_default(Some(&mock), None).column_num, 3, "空串跳过解析");
}

/// DATA 行基础填充: label 兜底 / preview 兜底 "-" / hideWhenZero 透传
#[test]
fn populate_from_group_basic() {
    let mut group = GroupConfig::new("飞行信息");
    let mut row = RowConfig::new("转半径");
    row.property = Some("getTurnRadius".to_string());
    row.unit = "M".to_string();
    row.hide_when_zero = true;
    group.rows.push(row);

    let cfg = FlightInfoConfig::create_default(None, Some(group));
    let defs = cfg.get_field_definitions();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].key, "getTurnRadius");
    assert_eq!(defs[0].label, "转半径", "无 targetName 时回退 label");
    assert_eq!(defs[0].unit, "M");
    assert_eq!(defs[0].config_key, "getTurnRadius", "configKey = property");
    assert!(defs[0].hide_when_na, "populateFromGroup 固定传 true");
    assert!(defs[0].hide_when_zero);
    assert_eq!(defs[0].preview_value, "-", "previewValue 缺省兜底 '-'");
}

/// targetName 非空优先; previewValue 非空透传
#[test]
fn populate_from_group_prefers_target_name() {
    let mut group = GroupConfig::new("g");
    let mut row = RowConfig::new("原始标签");
    row.property = Some("getIAS".to_string());
    row.target_name = Some("表  速".to_string());
    row.preview_value = Some("500".to_string());
    row.unit = "Km/h".to_string();
    group.rows.push(row);

    let cfg = FlightInfoConfig::create_default(None, Some(group));
    let defs = cfg.get_field_definitions();
    assert_eq!(defs[0].label, "表  速");
    assert_eq!(defs[0].preview_value, "500");
    // 空 targetName 视为未提供
    let mut group = GroupConfig::new("g");
    let mut row = RowConfig::new("L");
    row.property = Some("p".to_string());
    row.target_name = Some(String::new());
    group.rows.push(row);
    assert_eq!(FlightInfoConfig::create_default(None, Some(group)).get_field_definitions()[0].label, "L");
}

/// 非 DATA 行与空 property 行跳过; children 递归拾取
#[test]
fn populate_from_group_skips_and_recurses() {
    let mut group = GroupConfig::new("g");
    let mut header = RowConfig::new("标题行");
    header.r#type = "HEADER".to_string();
    header.property = Some("h".to_string());
    group.rows.push(header);

    let no_prop = RowConfig::new("无属性");
    group.rows.push(no_prop);

    let mut empty_prop = RowConfig::new("空属性");
    empty_prop.property = Some(String::new());
    group.rows.push(empty_prop);

    let mut parent = RowConfig::new("容器");
    let mut child = RowConfig::new("子行");
    child.property = Some("childProp".to_string());
    child.unit = "%".to_string();
    parent.children.push(child);
    group.rows.push(parent);

    let cfg = FlightInfoConfig::create_default(None, Some(group));
    let defs = cfg.get_field_definitions();
    assert_eq!(defs.len(), 1, "仅子行 DATA 命中");
    assert_eq!(defs[0].key, "childProp");
    assert_eq!(defs[0].unit, "%");
}

/// Java addFieldDefinition 两重载 (6 参缺 hideWhenZero / 7 参全参)
#[test]
fn add_field_definition_overloads() {
    let mut cfg = FlightInfoConfig::new();
    cfg.add_field_definition("a", "A", "u", "ca", true, "1");
    cfg.add_field_definition_full("b", "B", "u", "cb", true, true, "2");
    let defs = cfg.get_field_definitions();
    assert_eq!(defs.len(), 2);
    assert!(!defs[0].hide_when_zero, "6 参重载缺省 hideWhenZero=false");
    assert!(defs[1].hide_when_zero);
    assert_eq!(defs[1].preview_value, "2");
}

#[test]
fn group_config_stored() {
    let cfg = FlightInfoConfig::create_default(None, Some(GroupConfig::new("飞行信息")));
    assert_eq!(cfg.group_config.as_ref().unwrap().title, "飞行信息");
}
