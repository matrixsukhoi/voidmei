//! 对应 Java: `src/ui/model/FieldDefinition.java` (一比一翻译)

/// Definition of a flight info field.
/// Used to externalize field configuration from the FlightInfo class.
pub struct FieldDefinition {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub config_key: String,
    pub hide_when_na: bool,
    pub hide_when_zero: bool,
    pub preview_value: String,
    pub format: Option<String>,
}

// PORT: Java 4 个 telescoping 构造器重载 → Rust 无重载, 主构造器保留 `new`,
// 其余按区分参数更名 (interpolation.rs `interp1d_extrapolate` 先例);
// 委托链 (4→3→1) 原样保留。
impl FieldDefinition {
    /// Java 主构造器 (7 参: 含 hideWhenZero + previewValue)
    pub fn new(
        key: &str,
        label: &str,
        unit: &str,
        config_key: &str,
        hide_when_na: bool,
        hide_when_zero: bool,
        preview_value: &str,
    ) -> FieldDefinition {
        FieldDefinition {
            key: key.to_string(),
            label: label.to_string(),
            unit: unit.to_string(),
            config_key: config_key.to_string(),
            hide_when_na,
            hide_when_zero,
            preview_value: preview_value.to_string(),
            format: None,
        }
    }

    /// Java 重载 (8 参, 附加 format): 委托主构造器后设置 format
    // PORT: Java 保真 — 参数表逐个对应 Java 重载形参, 不打包成结构体
    #[allow(clippy::too_many_arguments)]
    pub fn with_format(
        key: &str,
        label: &str,
        unit: &str,
        config_key: &str,
        hide_when_na: bool,
        hide_when_zero: bool,
        preview_value: &str,
        format: Option<&str>,
    ) -> FieldDefinition {
        let mut def = FieldDefinition::new(key, label, unit, config_key, hide_when_na, hide_when_zero, preview_value);
        def.format = format.map(|f| f.to_string());
        def
    }

    /// Java 重载 (6 参, 缺 hideWhenZero → false)
    pub fn new_without_hide_zero(
        key: &str,
        label: &str,
        unit: &str,
        config_key: &str,
        hide_when_na: bool,
        preview_value: &str,
    ) -> FieldDefinition {
        FieldDefinition::new(key, label, unit, config_key, hide_when_na, false, preview_value)
    }

    /// Java 重载 (5 参, 缺 previewValue → "-")
    pub fn new_with_default_preview(
        key: &str,
        label: &str,
        unit: &str,
        config_key: &str,
        hide_when_na: bool,
    ) -> FieldDefinition {
        FieldDefinition::new_without_hide_zero(key, label, unit, config_key, hide_when_na, "-")
    }
}

#[cfg(test)]
mod tests {
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
}
