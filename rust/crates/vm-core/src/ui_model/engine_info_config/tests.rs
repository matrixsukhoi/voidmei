use crate::ui_model::config_stub::RowConfig;
use crate::ui_model::field_manager::test_support::MockConfigProvider;

use super::*;

#[test]
fn defaults_match_java_field_initializers() {
    let cfg = EngineInfoConfig::new();
    assert_eq!(cfg.title, "EngineInfo");
    assert!(!cfg.show_edge);
    assert_eq!(cfg.column_num, 2);
    assert_eq!(cfg.num_font_key, "GlobalNumFont");
    assert_eq!(cfg.label_font_key, "fontName");
    assert_eq!(cfg.font_add_key, "fontSize");
    assert_eq!(cfg.column_key, "hudColumns");
    assert_eq!(cfg.pos_x_key, "engineInfoX");
    assert_eq!(cfg.pos_y_key, "engineInfoY");
    assert_eq!(cfg.edge_key, "engineInfoEdge");
    assert!(cfg.group_config.is_none());
    assert!(cfg.get_field_definitions().is_empty());
}

/// Java `"true".equals(getConfig(...))`: 仅 true 置位, 其余 (含 null) 保持 false
#[test]
fn edge_only_true_enables() {
    let mut mock = MockConfigProvider::new();
    mock.values.insert("engineInfoEdge".to_string(), "true".to_string());
    assert!(EngineInfoConfig::create_default(Some(&mock), None).show_edge);

    let mut mock = MockConfigProvider::new();
    mock.values.insert("engineInfoEdge".to_string(), "false".to_string());
    assert!(!EngineInfoConfig::create_default(Some(&mock), None).show_edge);

    // 未设置 / 空串 / 大小写 → 均 false
    assert!(!EngineInfoConfig::create_default(Some(&MockConfigProvider::new()), None).show_edge);
    let mut mock = MockConfigProvider::new();
    mock.values.insert("engineInfoEdge".to_string(), "TRUE".to_string());
    assert!(!EngineInfoConfig::create_default(Some(&mock), None).show_edge);
}

/// 列数: 新 key hudColumns 优先, 空缺回退旧 key columns; 非法 → 2
#[test]
fn column_new_key_first_legacy_fallback() {
    let mut mock = MockConfigProvider::new();
    mock.values.insert("hudColumns".to_string(), "4".to_string());
    mock.values.insert("columns".to_string(), "3".to_string());
    assert_eq!(EngineInfoConfig::create_default(Some(&mock), None).column_num, 4);

    // hudColumns 未设置 → 回退 columns
    let mut mock = MockConfigProvider::new();
    mock.values.insert("columns".to_string(), "3".to_string());
    assert_eq!(EngineInfoConfig::create_default(Some(&mock), None).column_num, 3);

    // hudColumns 为空串同样触发回退
    let mut mock = MockConfigProvider::new();
    mock.values.insert("hudColumns".to_string(), String::new());
    mock.values.insert("columns".to_string(), "6".to_string());
    assert_eq!(EngineInfoConfig::create_default(Some(&mock), None).column_num, 6);

    // 两 key 均缺 → 保持默认 2
    assert_eq!(EngineInfoConfig::create_default(Some(&MockConfigProvider::new()), None).column_num, 2);

    // 非法数字 → catch 置 2
    let mut mock = MockConfigProvider::new();
    mock.values.insert("hudColumns".to_string(), "NaN".to_string());
    assert_eq!(EngineInfoConfig::create_default(Some(&mock), None).column_num, 2);
}

/// EngineInfo 版 previewValue 兜底是 "0"
#[test]
fn populate_from_group_defaults_to_zero_preview() {
    let mut group = GroupConfig::new("引擎信息");
    let mut row = RowConfig::new("功率");
    row.property = Some("S.sTotalHp".to_string());
    row.unit = "Hp".to_string();
    group.rows.push(row);

    let cfg = EngineInfoConfig::create_default(None, Some(group));
    let defs = cfg.get_field_definitions();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].preview_value, "0", "EngineInfo 兜底 '0' (FlightInfo 为 '-')");
    assert_eq!(defs[0].config_key, "S.sTotalHp");
    assert!(defs[0].hide_when_na);

    // previewValue 非空透传
    let mut group = GroupConfig::new("g");
    let mut row = RowConfig::new("油温");
    row.property = Some("oilTemp".to_string());
    row.preview_value = Some("85".to_string());
    row.target_name = Some("油  温".to_string());
    group.rows.push(row);
    let cfg2 = EngineInfoConfig::create_default(None, Some(group));
    let defs = cfg2.get_field_definitions();
    assert_eq!(defs[0].preview_value, "85");
    assert_eq!(defs[0].label, "油  温");
}

/// 非 DATA 行跳过 + 子行递归
#[test]
fn populate_from_group_skips_non_data_and_recurses() {
    let mut group = GroupConfig::new("g");
    let mut slider = RowConfig::new("字号");
    slider.r#type = "SLIDER".to_string();
    slider.property = Some("fontSize".to_string());
    group.rows.push(slider);
    let mut child_container = RowConfig::new("容器");
    let mut child = RowConfig::new("水温");
    child.property = Some("waterTemp".to_string());
    child_container.children.push(child);
    group.rows.push(child_container);

    let cfg3 = EngineInfoConfig::create_default(None, Some(group));
    let defs = cfg3.get_field_definitions();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].key, "waterTemp");
}

/// Java addFieldDefinition 6 参重载委托 7 参 (hideWhenZero=false)
#[test]
fn add_field_definition_simple_delegates_with_false() {
    let mut cfg = EngineInfoConfig::new();
    cfg.add_field_definition_simple("k", "标签", "Hp", "c", true, "1200");
    cfg.add_field_definition("k2", "标签2", "Kgf", "c2", true, true, "1800");
    let defs = cfg.get_field_definitions();
    assert_eq!(defs.len(), 2);
    assert!(!defs[0].hide_when_zero);
    assert_eq!(defs[0].preview_value, "1200");
    assert!(defs[1].hide_when_zero);
    assert_eq!(defs[1].preview_value, "1800");
}
