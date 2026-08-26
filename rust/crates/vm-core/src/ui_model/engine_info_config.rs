//! 对应 Java: `src/ui/model/EngineInfoConfig.java` (一比一翻译)

use crate::ui_model::config_stub::{ConfigProvider, GroupConfig, RowConfig};
use crate::ui_model::field_definition::FieldDefinition;

/// Configuration for EngineInfo overlay.
/// Replaces hardcoded defaults in EngineInfo.
pub struct EngineInfoConfig {
    field_definitions: Vec<FieldDefinition>,

    pub title: String,

    // Style configuration
    pub show_edge: bool,
    pub column_num: i32, // Default 2 columns for EngineInfo

    // Font configuration keys
    pub num_font_key: String,
    pub label_font_key: String, // Was 'engineInfoFont' in old config, mapped to 'fontName' in ui_layout
    pub font_add_key: String,  // Was 'engineInfoFontadd', mapped to 'fontSize'
    pub column_key: String,    // 重命名避免与 GroupConfig.columns 字段冲突

    // Position keys
    pub pos_x_key: String,
    pub pos_y_key: String,

    // Edge style key
    // ui_layout.cfg doesn't seem to have a specific edge key for EngineInfo in the
    // default layout provided?
    // Checking ui_layout.cfg... [引擎信息] section has: Font, FontSize, Columns.
    // FlightInfo had 'flightInfoEdge'.
    // EngineInfo seems to lack a specific edge switch in default layout, but we can
    // define one for future or consistency.
    // Let's assume standard key or default false.
    pub edge_key: String,

    // Layout Config
    // PORT: Java 持 GroupConfig 活引用共享 (调用方 Controller 注册表另持一份);
    // 本翻译取所有权 —— populateFromGroup 只读 rows, 无行为差异。批二 config_api
    // 落地时如需共享再裁决 Arc。
    pub group_config: Option<GroupConfig>,
}

impl EngineInfoConfig {
    /// Java 字段声明默认值初始化 (隐式无参构造器 + 字段初始化器)
    fn new() -> EngineInfoConfig {
        EngineInfoConfig {
            field_definitions: Vec::new(),
            title: "EngineInfo".to_string(),
            show_edge: false,
            column_num: 2,
            num_font_key: "GlobalNumFont".to_string(),
            label_font_key: "fontName".to_string(),
            font_add_key: "fontSize".to_string(),
            column_key: "hudColumns".to_string(),
            pos_x_key: "engineInfoX".to_string(),
            pos_y_key: "engineInfoY".to_string(),
            edge_key: "engineInfoEdge".to_string(),
            group_config: None,
        }
    }

    pub fn get_field_definitions(&self) -> &[FieldDefinition] {
        &self.field_definitions
    }

    /// Java 重载 (7 参, 全参)
    // PORT: Java 保真 — 参数表逐个对应 Java 重载形参, 不打包成结构体
    #[allow(clippy::too_many_arguments)]
    pub fn add_field_definition(
        &mut self,
        key: &str,
        label: &str,
        unit: &str,
        config_key: &str,
        hide_when_na: bool,
        hide_when_zero: bool,
        example_value: &str,
    ) {
        self.field_definitions
            .push(FieldDefinition::new(key, label, unit, config_key, hide_when_na, hide_when_zero, example_value));
    }

    /// Java 重载 (6 参, 缺 hideWhenZero → 委托 7 参版传 false)
    pub fn add_field_definition_simple(
        &mut self,
        key: &str,
        label: &str,
        unit: &str,
        config_key: &str,
        hide_when_na: bool,
        example_value: &str,
    ) {
        self.add_field_definition(key, label, unit, config_key, hide_when_na, false, example_value);
    }

    pub fn create_default(
        config: Option<&dyn ConfigProvider>,
        group_config: Option<GroupConfig>,
    ) -> EngineInfoConfig {
        let mut cfg = EngineInfoConfig::new();
        // Java: cfg.groupConfig = groupConfig; —— Rust 先借用 rows 填充再 move
        // (纯语句序交换, populateFromGroup 只读, 无行为差异)

        // NOTE: EngineInfo in ui_layout.cfg uses "fontName", "fontSize", "columns" as
        // keys
        // because they are generic within the [Engine Info] group.
        // FieldOverlay's reinitConfig uses these keys to fetch from the passed
        // ConfigProvider (which is usually the GroupConfig wrapper).
        // However, FieldOverlay expects unique keys if fetching from Global config.
        // BUT, Reference to `FlightFactory` usage implies `ConfigProvider` is the
        // GroupConfig.
        // Let's verify `FlightInfo` usage. `FlightInfo` uses specific keys like
        // `flightInfoFontC`.
        // EngineInfo's `ui_layout.cfg` section uses generic keys.
        // If `ConfigProvider` passed to `init` is the `GroupConfig` (wrapped), then
        // `getConfig("fontName")` works.
        // In `Controller`, `registerWithPreview` passes `new EngineInfo()`.
        // `EngineInfo.init` receives `Controller` which IS a `ConfigProvider`.
        // BUT `Controller.getConfig` fetches global properties.
        // `EngineInfo` currently does: `xc.getConfig("engineInfoFont")` which implies
        // global keys were used in legacy.
        // BUT `ConfigLoader` loads `ui_layout.cfg`.
        // If we use `FieldOverlay`, we need to ensure keys match.

        // Refactoring Decision:
        // Use the Keys defined in `ui_layout.cfg` [Engine Info] section if possible?
        // Actually, `FieldOverlay` uses `renderContext =
        // RenderContext.fromConfig(...)`.
        // If we want to use the generic keys defined in `ui_layout.cfg` (like
        // `Font=...`),
        // we might need to update how `RenderContext` works or map them.
        // `EngineInfo` old code: `xc.getConfig("engineInfoFont")`.
        // ConfigurationService DOES NOT seem to have flattened keys for dynamic groups
        // unless `bind` is used?
        // Wait, `Controller`'s `dynamicConfigs` list contains `GroupConfig` objects
        // with `rows`.

        // Let's look at `FlightInfoConfig` again.
        // `cfg.labelFontKey = "flightInfoFontC";`
        // `ui_layout.cfg`: `字体 || COMBO:flightInfoFontC:_FONTS_ || %s || Sarasa Mono
        // SC`
        // So `FlightInfo` uses explicit global keys.

        // `EngineInfo` section in `ui_layout.cfg`:
        // `字体 || COMBO:fontName:_FONTS_ || %s || Sarasa Mono SC`
        // It uses `fontName`.
        // So `cfg.labelFontKey = "fontName";` is correct.

        if let Some(cp) = config {
            // EngineInfo doesn't seem to have an edge switch config in the layout provided,
            // default false.
            // But we can check "engineInfoEdge" just in case legacy exists.
            // Java: if ("true".equals(config.getConfig("engineInfoEdge"))) —— 与
            // FlightInfo 不同, 仅在 true 时置位 (null 安全的字面量前置 equals)
            if cp.get_config("engineInfoEdge").as_deref() == Some("true") {
                cfg.show_edge = true;
            }

            // 优先读取新 key，回退读取旧 key 以保持向后兼容
            let mut col_str = cp.get_config("hudColumns");
            if col_str.as_deref().is_none_or(|s| s.is_empty()) {
                col_str = cp.get_config("columns"); // 旧 key 回退
            }
            if col_str.as_deref().is_some_and(|s| !s.is_empty()) {
                // PORT: Java Integer.parseInt 抛 NumberFormatException → catch 置 2 (§2.15)
                cfg.column_num = col_str.as_deref().unwrap().parse::<i32>().unwrap_or(2);
            }
        }

        // Add fields matching Service.java updates
        // label comes from Lang class
        // configKey is for visibility check (e.g. "HorsePower")
        // Note: FieldOverlay uses the configKey to check "disable"+Key or just
        // Key=false?
        // FlightInfo: `disableFlightInfoIAS`.
        // EngineInfo legacy: `disableEngineInfoHorsePower`.
        // But `ui_layout.cfg` has: `功率 || S.sTotalHp || %s Hp || true`.
        // It uses `S.sTotalHp` as the config Key for visibility in `ConfigLoader`? No,
        // RowConfig has `visible`.
        // `FieldOverlay` uses `FieldDefinition.configKey`.
        // `FieldManager` checks `config.getConfig(def.configKey)`.

        // CRITICAL DIFFERENCE:
        // `FlightInfo` in `ui_layout.cfg`: `示空速_IAS || SWITCH_INV:disableFlightInfoIAS
        // || %s || true`
        // The visibility is controlled by a SWITCH that sets `disableFlightInfoIAS`.

        // `EngineInfo` in `ui_layout.cfg`: `功率 || S.sTotalHp || %s Hp || true`
        // Result: It is a DATA row. `visible` is set directly in the row.
        // It DOES NOT use a separate switch key like `disableEngineInfo...`.
        // The old `EngineInfo` check `isFieldEnabledFromConfig` looked up the row by
        // label.

        // If we convert to `FieldOverlay`, `FieldManager` expects a config key to check
        // visibility.
        // If we provide a config key that doesn't exist (because it's just a row
        // visibility), it might fail?
        // `DefaultFieldManager` checks:
        // `String val = config.getConfig(configKey);`
        // If `ui_layout.cfg` rows are loaded, they are in `GroupConfig.rows`.
        // `Controller` (ConfigProvider) might not expose these row visibilities as flat
        // keys!

        // `FlightInfo` works because it has explicit switches defined in
        // `ui_layout.cfg`.
        // `EngineInfo` DOES NOT have switches for individual items in `ui_layout.cfg`.
        // It simply lists them.

        // To make `FieldOverlay` work for `EngineInfo`, we might need to:
        // 1. Update `ui_layout.cfg` to add switches? (User didnt ask for file change)
        // 2. Or, make `EngineInfoConfig` or `FieldManager` smarter to lookup row
        // visibility?

        // The User asked to refactor `EngineInfo` to "Reference flightinfo".
        // `FlightInfo` works because of `disableFlightInfo...` switches.
        // `EngineInfo` legacy: checked `disableEngineInfoHorsePower` (legacy) OR row
        // visibility.

        // If we stick to `FieldOverlay`, `FieldManager` uses `configProvider`.
        // We can create a `RowVisibilityConfigProvider` adapter?

        // Let's define the config keys as "engineInfoSwitch" (the Group switch) or
        // similar? No.
        // We map `HorsePower` -> `disableEngineInfoHorsePower`?
        // Does `ui_layout.cfg` have these switches?
        // [Engine Info] section:
        // `功率 || S.sTotalHp || %s Hp || true`
        // No explicit switch.

        // Wait! `EngineInfo` legacy code:
        // `isFieldEnabledFromConfig` iterates `GroupConfig.rows` and finds matching
        // label.

        // Strategy:
        // `EngineInfo` (new) will extend `FieldOverlay`.
        // We override `initFields` or use a custom `FieldManager` that knows how to
        // check RowConfig visibility!

        // Or, better: `EngineInfoConfig` can populate `FieldDefinition` with `visible`
        // defaults based on `GroupConfig`!
        // But `GroupConfig` is dynamic (loaded from file).

        // Let's pass the `GroupConfig` to `EngineInfoConfig.createDefault`?
        if let Some(gc) = &group_config {
            cfg.populate_from_group(&gc.rows);
        }

        // Actually, `FlightInfo` setup:
        // `overlayManager.registerWithPreview(..., () -> new FlightInfo(), ...)`
        // `FlightInfo.init(..., FlightInfoConfig.createDefault(this))`

        // If we want `FieldOverlay` to verify visibility against `ui_layout.cfg` rows
        // dynamically:
        // `FieldOverlay.onFlightData` -> `fieldManager.updateField`.
        // It checks `field.visible`.
        // `FieldManager` initialization sets `field.visible`.
        // We need updates when config changes. `reinitConfig` calls `initFields`.

        // `DefaultFieldManager.addField` logic:
        // `boolean isVisible = !hideWhenNA;` (initial)
        // Then `FieldManager` doesn't strictly check configKey for visibility
        // constantly?
        // `DefaultFieldManager` constructor takes `ConfigProvider`.
        // It has `updateVisibility`?

        // Let's check `DefaultFieldManager`.

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
                // EngineInfo 版兜底是 "0" (FlightInfo 版是 "-")
                let def_val = row.preview_value.clone().unwrap_or_else(|| "0".to_string());
                let property = row.property.as_deref().unwrap();
                self.add_field_definition(
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
}
