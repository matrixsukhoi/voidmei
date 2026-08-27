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
mod tests;
