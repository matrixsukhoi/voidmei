use crate::ui_model::data_field::VisibilityExpressionEvaluatorPlaceholder;
use crate::ui_model::field_manager::test_support::MockConfigProvider;

use super::*;

/// Java addField: previewValue 原样落 currentValue (不经 %5s)
#[test]
fn add_field_stores_preview_value_raw() {
    let mut fm = DefaultFieldManager::new(None);
    fm.add_field("getTurnRadius", "转半径", "M", Some("getTurnRadius"), true, false, Some("800"), None);
    assert_eq!(fm.size(), 1);
    let df = fm.get_field("getTurnRadius").expect("字段应可按名索引");
    assert_eq!(df.current_value, "800", "previewValue 直赋, 非右对齐 '  800'");
    assert_eq!(df.format, None);
}

/// previewValue=null → currentValue 保持构造默认 "---"
#[test]
fn add_field_without_preview_keeps_default() {
    let mut fm = DefaultFieldManager::new(None);
    fm.add_field("k", "l", "u", Some("c"), false, false, None, Some("TIME_MM_SS"));
    let df = fm.get_field("k").unwrap();
    assert_eq!(df.current_value, "---");
    assert_eq!(df.format.as_deref(), Some("TIME_MM_SS"));
}

/// config!=null 且 isFieldDisabled → 直接 return (不加入)
#[test]
fn add_field_skipped_when_disabled() {
    let mut cfg = MockConfigProvider::new();
    cfg.disabled.insert("disableX".to_string());
    let mut fm = DefaultFieldManager::new(Some(Box::new(cfg)));
    fm.add_field("x", "l", "u", Some("disableX"), false, false, Some("1"), None);
    assert_eq!(fm.size(), 0);
    assert!(fm.get_field("x").is_none());
}

/// configKey=null → 即使 config 存在也不做禁用检查 (Java 短路顺序)
#[test]
fn add_field_without_config_key_bypasses_check() {
    let mut cfg = MockConfigProvider::new();
    cfg.disabled.insert("whatever".to_string());
    let mut fm = DefaultFieldManager::new(Some(Box::new(cfg)));
    fm.add_field("x", "l", "u", None, false, false, Some("1"), None);
    assert_eq!(fm.size(), 1);
}

/// 重复 key: fields 保序保留两条, fieldMap 指向最新 (Java put 覆盖语义)
#[test]
fn duplicate_key_maps_to_latest() {
    let mut fm = DefaultFieldManager::new(None);
    fm.add_field("k", "第一", "u", None, false, false, Some("1"), None);
    fm.add_field("k", "第二", "u", None, false, false, Some("2"), None);
    assert_eq!(fm.size(), 2);
    assert_eq!(fm.get_field("k").unwrap().label, "第二");
    // getFields 保持插入顺序
    let labels: Vec<&str> = fm.get_fields().iter().map(|f| f.label.as_str()).collect();
    assert_eq!(labels, vec!["第一", "第二"]);
}

#[test]
fn update_field_sets_value_and_visibility() {
    let mut fm = DefaultFieldManager::new(None);
    fm.add_field("k", "l", "u", None, true, false, Some("800"), None);
    fm.update_field("k", "9999", "-");
    let df = fm.get_field("k").unwrap();
    assert_eq!(df.current_value, " 9999");
    assert!(df.visible);
    fm.update_field("k", "-", "-");
    assert!(!fm.get_field("k").unwrap().visible, "NA 串应隐藏");
    // 不存在的 key: 无操作不 panic
    fm.update_field("nope", "1", "-");
}

#[test]
fn update_field_unit_by_key() {
    let mut fm = DefaultFieldManager::new(None);
    fm.add_field("m", "进气压", "Ata", None, false, false, Some("1.02"), None);
    fm.update_field_unit("m", "P/30.1''");
    assert_eq!(fm.get_field("m").unwrap().unit, "P/30.1''");
    fm.update_field_unit("nope", "x");
}

/// bind(key, supplier, precision) → 委托 bind_with_visibility(null, precision, null)
#[test]
fn bind_basic_sets_supplier_and_precision() {
    let mut fm = DefaultFieldManager::new(None);
    fm.add_field("rpm", "转速", "RPM", None, false, false, Some("2400"), None);
    fm.bind("rpm", Box::new(|| 2600.0), 0);
    let df = fm.get_field("rpm").unwrap();
    assert_eq!(df.precision, 0);
    assert!(df.visibility_supplier.is_none());
    assert_eq!((df.value_supplier.as_ref().unwrap())(), 2600.0);
}

/// bind 的 format=null 覆盖语义: addField 设置的 format 被清空 (Java 原样)
#[test]
fn bind_null_format_overwrites_existing() {
    let mut fm = DefaultFieldManager::new(None);
    fm.add_field("t", "时间", "M:s", None, false, false, Some("1:00"), Some("TIME_MM_SS"));
    assert!(fm.get_field("t").unwrap().format.is_some());
    fm.bind_with_visibility("t", Box::new(|| 60.0), None, 0, None);
    assert_eq!(fm.get_field("t").unwrap().format, None, "Java format=null 原样覆盖为 null");
}

/// bind_with_visibility: 可见性供应商就位
#[test]
fn bind_with_visibility_attaches_suppliers() {
    let mut fm = DefaultFieldManager::new(None);
    fm.add_field("ias", "表  速", "Km/h", None, false, false, Some("500"), None);
    fm.bind_with_visibility(
        "ias",
        Box::new(|| 501.5),
        Some(Box::new(|| true)),
        1,
        None,
    );
    let df = fm.get_field("ias").unwrap();
    assert_eq!(df.precision, 1);
    assert_eq!((df.value_supplier.as_ref().unwrap())(), 501.5);
    assert!((df.visibility_supplier.as_ref().unwrap())());
}

/// bind_dynamic: 动态单位/精度供应商 (进气压 Ata/psi 切换形态)
#[test]
fn bind_dynamic_attaches_unit_and_precision_suppliers() {
    let mut fm = DefaultFieldManager::new(None);
    fm.add_field("map", "进气压", "Ata", None, false, false, Some("1.05"), None);
    fm.bind_dynamic(
        "map",
        Box::new(|| 1.05),
        None,
        2,
        None,
        Some(Box::new(|| "P/30.1''".to_string())),
        Some(Box::new(|| 1)),
    );
    let df = fm.get_field("map").unwrap();
    assert_eq!(df.precision, 2);
    assert_eq!((df.unit_supplier.as_ref().unwrap())(), "P/30.1''");
    assert_eq!((df.precision_supplier.as_ref().unwrap())(), 1);
}

/// 不存在的 key: 各 bind 均为无操作
#[test]
fn bind_on_missing_key_is_noop() {
    let mut fm = DefaultFieldManager::new(None);
    fm.bind("nope", Box::new(|| 0.0), 3);
    fm.bind_dynamic("nope", Box::new(|| 0.0), None, 0, None, None, None);
    assert_eq!(fm.size(), 0);
}

#[test]
fn clear_all_resets_both_stores() {
    let mut fm = DefaultFieldManager::new(None);
    fm.add_field("a", "l", "u", None, false, false, Some("1"), None);
    fm.add_field("b", "l", "u", None, false, false, Some("2"), None);
    assert_eq!(fm.size(), 2);
    fm.clear_all();
    assert_eq!(fm.size(), 0);
    assert!(fm.get_field("a").is_none());
    assert!(fm.get_fields().is_empty());
}

/// visibleCount + setFieldVisible
#[test]
fn visible_count_and_set_field_visible() {
    let mut fm = DefaultFieldManager::new(None);
    fm.add_field("a", "l", "u", None, false, false, Some("1"), None);
    fm.add_field("b", "l", "u", None, false, false, Some("2"), None);
    fm.add_field("c", "l", "u", None, false, false, Some("3"), None);
    assert_eq!(fm.visible_count(), 3);
    fm.set_field_visible("b", false);
    assert_eq!(fm.visible_count(), 2);
    assert!(!fm.get_field("b").unwrap().visible);
    fm.set_field_visible("nope", false); // 无操作
    assert_eq!(fm.visible_count(), 2);
}

/// Java FieldOverlay.onFlightData 经 getFields() 返回的活列表就地改写
/// field.visible —— 对应 Rust get_fields_mut 通道
#[test]
fn get_fields_mut_allows_in_place_mutation() {
    let mut fm = DefaultFieldManager::new(None);
    fm.add_field("a", "l", "u", None, false, false, Some("1"), None);
    for field in fm.get_fields_mut() {
        field.visible = false;
        field.buffer = "42".to_string();
        field.length = 2;
    }
    let df = fm.get_field("a").unwrap();
    assert!(!df.visible);
    assert_eq!(df.buffer, "42");
    assert_eq!(df.length, 2);
    // get_field_mut 通道
    fm.get_field_mut("a").unwrap().precision = 3;
    assert_eq!(fm.get_field("a").unwrap().precision, 3);
}

/// trait 对象分发 (FieldOverlay 以 FieldManager 接口持有实现)
#[test]
fn trait_object_dispatch() {
    let mut fm = DefaultFieldManager::new(None);
    let mgr: &mut dyn FieldManager = &mut fm;
    mgr.add_field("x", "l", "u", None, false, false, Some("9"), None);
    assert_eq!(mgr.size(), 1);
    assert_eq!(mgr.visible_count(), 1);
}

/// TestNaWhenBinding.java 可移植片段 (DefaultFieldManager 面):
/// mock config (getConfig=null / isFieldDisabled=false) + addField + getField;
/// naWhen 求值部分依赖批二 visibility_expression, 此处仅验证字段位可设置。
#[test]
fn na_when_binding_fragment() {
    let mock = MockConfigProvider::new();
    let mut fm = DefaultFieldManager::new(Some(Box::new(mock)));
    let key = "getTurnRadius";
    fm.add_field(key, "转半径", "M", Some(key), true, false, Some("800"), None);
    let df1 = fm.get_field(key);
    assert!(df1.is_some(), "fm.getField 应找到字段");
    assert_eq!(df1.unwrap().current_value, "800");
    // Java: df.naWhenEvaluator = new VisibilityExpressionEvaluator(row.naWhen, null);
    // PORT: 求值器为占位类型, 仅验证字段位可写 (evaluate 对拍留给批二)
    fm.get_field_mut(key).unwrap().na_when_evaluator = Some(VisibilityExpressionEvaluatorPlaceholder);
    assert!(fm.get_field(key).unwrap().na_when_evaluator.is_some());
}
