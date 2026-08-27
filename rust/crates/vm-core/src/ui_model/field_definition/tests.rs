use super::*;

#[test]
fn new_full_constructor() {
    let def = FieldDefinition::new("getIAS", "表  速", "Km/h", "disableFlightInfoIAS", true, true, "500");
    assert_eq!(def.key, "getIAS");
    assert_eq!(def.label, "表  速");
    assert_eq!(def.unit, "Km/h");
    assert_eq!(def.config_key, "disableFlightInfoIAS");
    assert!(def.hide_when_na);
    assert!(def.hide_when_zero);
    assert_eq!(def.preview_value, "500");
    assert_eq!(def.format, None);
}

/// Java `this(key,...); this.format = format;` 委托后赋 format
#[test]
fn with_format_delegates_then_sets() {
    let def = FieldDefinition::with_format(
        "getFuelTime", "燃油时间", "M:s", "disableFuelTime", false, false, "45:00", Some("TIME_MM_SS"),
    );
    assert_eq!(def.preview_value, "45:00");
    assert_eq!(def.format.as_deref(), Some("TIME_MM_SS"));
    assert!(!def.hide_when_na);
}

/// Java 重载 3: `this(key, ..., hideWhenNA, false, previewValue)`
#[test]
fn without_hide_zero_defaults_false() {
    let def = FieldDefinition::new_without_hide_zero("getTAS", "真空速", "Km/h", "disableTAS", true, "550");
    assert!(def.hide_when_na);
    assert!(!def.hide_when_zero, "缺省 hideWhenZero 必须 false");
    assert_eq!(def.preview_value, "550");
    assert_eq!(def.format, None);
}

/// Java 重载 4: `this(key, ..., hideWhenNA, "-")` — preview 兜底 "-"
#[test]
fn with_default_preview_uses_dash() {
    let def = FieldDefinition::new_with_default_preview("getX", "X", "", "disableX", true);
    assert_eq!(def.preview_value, "-");
    assert!(!def.hide_when_zero);
}
