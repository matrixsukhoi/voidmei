//! 对应 Java: `src/ui/model/FlightInfoConfig.java` (一比一翻译)

use crate::ui_model::config_stub::{ConfigProvider, GroupConfig, RowConfig};
use crate::ui_model::field_definition::FieldDefinition;

/// Configuration for FlightInfo overlay.
/// Externalizes all configuration from the FlightInfo class.
pub struct FlightInfoConfig {
    // Field definitions
    field_definitions: Vec<FieldDefinition>,

    // Overlay title
    pub title: String,

    // Style configuration
    pub show_edge: bool,
    pub column_num: i32,

    // Font configuration keys
    pub num_font_key: String,
    pub label_font_key: String,
    // public String fontAddKey = "flightInfoFontaddC"; // Legacy, removed
    pub column_key: String,

    // Position keys (inherited by DraggableOverlay)
    pub pos_x_key: String,
    pub pos_y_key: String,

    // Edge style key
    pub edge_key: String,

    // Layout Config
    // PORT: Java 持 GroupConfig 活引用共享 (调用方 Controller 注册表另持一份);
    // 本翻译取所有权 —— populateFromGroup 只读 rows, 无行为差异。批二 config_api
    // 落地时如需共享再裁决 Arc。
    pub group_config: Option<GroupConfig>,
}

impl FlightInfoConfig {
    /// Java 字段声明默认值初始化 (隐式无参构造器 + 字段初始化器)
    fn new() -> FlightInfoConfig {
        FlightInfoConfig {
            field_definitions: Vec::new(),
            title: "FlightInfo".to_string(),
            show_edge: false,
            column_num: 3,
            num_font_key: "GlobalNumFont".to_string(),
            label_font_key: "flightInfoFontC".to_string(),
            column_key: "flightInfoColumn".to_string(),
            pos_x_key: "flightInfoX".to_string(),
            pos_y_key: "flightInfoY".to_string(),
            edge_key: "flightInfoEdge".to_string(),
            group_config: None,
        }
    }

    pub fn get_field_definitions(&self) -> &[FieldDefinition] {
        &self.field_definitions
    }

    /// Java 重载 (6 参, 缺 hideWhenZero → FieldDefinition 构造器 3)
    pub fn add_field_definition(
        &mut self,
        key: &str,
        label: &str,
        unit: &str,
        config_key: &str,
        hide_when_na: bool,
        example_value: &str,
    ) {
        self.field_definitions
            .push(FieldDefinition::new_without_hide_zero(key, label, unit, config_key, hide_when_na, example_value));
    }

    /// Java 重载 (7 参, 全参) —— Rust 主名占用 6 参形态, 全参版加 _full 后缀区分
    // PORT: Java 保真 — 参数表逐个对应 Java 重载形参, 不打包成结构体
    #[allow(clippy::too_many_arguments)]
    pub fn add_field_definition_full(
        &mut self,
        key: &str,
        label: &str,
        unit: &str,
        config_key: &str,
        hide_when_na: bool,
        hide_when_zero: bool,
        preview_value: &str,
    ) {
        self.field_definitions
            .push(FieldDefinition::new(key, label, unit, config_key, hide_when_na, hide_when_zero, preview_value));
    }

    /// Create default configuration with standard flight info fields.
    /// This factory method contains the field definitions that were previously
    /// hardcoded in FlightInfo.
    pub fn create_default(
        config_provider: Option<&dyn ConfigProvider>,
        group_config: Option<GroupConfig>,
    ) -> FlightInfoConfig {
        let mut cfg = FlightInfoConfig::new();
        // Java: cfg.groupConfig = groupConfig; —— Rust 先借用 rows 填充再 move
        // (纯语句序交换, populateFromGroup 只读, 无行为差异)

        if let Some(cp) = config_provider {
            let edge_val = cp.get_config("flightInfoEdge");
            if edge_val.is_some() {
                // Java: cfg.showEdge = "true".equals(edgeVal); —— 非 null 即赋值 (空串 → false)
                cfg.show_edge = edge_val.as_deref() == Some("true");
            }

            let col_str = cp.get_config("flightInfoColumn");
            if let Some(col) = col_str {
                if !col.is_empty() {
                    // PORT: Java Integer.parseInt 抛 NumberFormatException → catch 置 3 (§2.15);
                    // Rust parse 同样不接受空白/小数点, 语义一致
                    cfg.column_num = col.parse::<i32>().unwrap_or(3);
                }
            }
        }

        // Dynamically populate fields from the loaded configuration groups
        if let Some(gc) = &group_config {
            cfg.populate_from_group(&gc.rows);
        }
        cfg.group_config = group_config;

        cfg
    }

    fn populate_from_group(&mut self, rows: &[RowConfig]) {
        // Java: if (rows == null) return; —— &[RowConfig] 非空类型, 判空恒假
        for row in rows {
            if row.r#type == "DATA" && row.property.as_deref().is_some_and(|p| !p.is_empty()) {
                // Use targetName if provided, otherwise fallback to label
                let display_label = match row.target_name.as_deref() {
                    Some(t) if !t.is_empty() => t.to_string(),
                    _ => row.label.clone(),
                };
                let def_val = row.preview_value.clone().unwrap_or_else(|| "-".to_string());
                let property = row.property.as_deref().unwrap();
                self.add_field_definition_full(
                    property,
                    &display_label,
                    &row.unit,
                    property,
                    true,
                    row.hide_when_zero,
                    &def_val,
                );
            }
            // Java: if (row.children != null) populateFromGroup(row.children);
            // PORT: children 默认非 null (ArrayList), 判空恒真; 空 Vec 递归自然终止
            self.populate_from_group(&row.children);
        }
    }
}

#[cfg(test)]
mod tests;
