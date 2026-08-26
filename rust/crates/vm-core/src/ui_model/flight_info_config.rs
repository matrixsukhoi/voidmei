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
mod tests {
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
}
