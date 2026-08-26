//! 对应 Java: `src/ui/model/FieldManager.java` (一比一翻译)

use crate::ui_model::data_field::DataField;

/// Interface for managing overlay data fields.
/// Provides a unified API for adding, updating, and querying fields.
// PORT: Java 接口 → trait; 3 个 bind 重载 → 更名 bind / bind_with_visibility /
// bind_dynamic (DefaultFieldManager 同名对应); Java 返回活引用 (List/DataField)
// 的两个读方法按所有权各补一个 _mut 孪生 (FieldOverlay.onFlightData 经
// getFields() 就地改写字段, &mut 借用是 Rust 侧该用法的必要通道)。
pub trait FieldManager {
    /// Add a new field to the manager.
    // PORT: Java 保真 — 参数表逐个对应 Java addField 重载形参, 不打包成结构体
    #[allow(clippy::too_many_arguments)]
    fn add_field(
        &mut self,
        key: &str,
        label: &str,
        unit: &str,
        config_key: Option<&str>,
        hide_when_na: bool,
        hide_when_zero: bool,
        preview_value: Option<&str>,
        format: Option<&str>,
    );

    /// Update a field's value by key.
    fn update_field(&mut self, key: &str, value: &str, na_string: &str);

    /// Set a field's visibility by key.
    fn set_field_visible(&mut self, key: &str, visible: bool);

    /// Update a field's unit by key.
    fn update_field_unit(&mut self, key: &str, unit: &str);

    /// Bind a field to a zero-GC double supplier.
    fn bind(&mut self, key: &str, supplier: Box<dyn Fn() -> f64>, precision: i32);

    /// Bind a field to a zero-GC double supplier with an optional visibility
    /// supplier.
    fn bind_with_visibility(
        &mut self,
        key: &str,
        value_supplier: Box<dyn Fn() -> f64>,
        visibility_supplier: Option<Box<dyn Fn() -> bool>>,
        precision: i32,
        format: Option<&str>,
    );

    /// Bind a field with dynamic unit and precision suppliers.
    // PORT: Java 保真 — 参数表逐个对应 Java bindDynamic 形参, 不打包成结构体
    #[allow(clippy::too_many_arguments)]
    fn bind_dynamic(
        &mut self,
        key: &str,
        value_supplier: Box<dyn Fn() -> f64>,
        visibility_supplier: Option<Box<dyn Fn() -> bool>>,
        precision: i32,
        format: Option<&str>,
        unit_supplier: Option<Box<dyn Fn() -> String>>,
        precision_supplier: Option<Box<dyn Fn() -> i32>>,
    );

    /// Get all fields in order.
    fn get_fields(&self) -> &[DataField];

    /// Get all fields in order (mutable; PORT: Java List 活引用的就地改写通道).
    fn get_fields_mut(&mut self) -> &mut [DataField];

    /// Get a specific field by key.
    fn get_field(&self, key: &str) -> Option<&DataField>;

    /// Get a specific field by key (mutable; PORT: 同 get_fields_mut).
    fn get_field_mut(&mut self, key: &str) -> Option<&mut DataField>;

    /// Clear all fields.
    fn clear_all(&mut self);

    /// Get the number of fields.
    fn size(&self) -> i32;

    /// Get the number of currently visible fields.
    fn visible_count(&self) -> i32;
}

/// 测试用 ConfigProvider 桩实现是各翻译测试的公共依赖 —— 与 Java 侧
/// `FieldManager(接口)` 的消费点 (FieldOverlay) 一致, 本 trait 对象安全。
#[cfg(test)]
pub(crate) mod test_support {
    use crate::ui_model::config_stub::ConfigProvider;
    use std::collections::HashSet;

    /// 可配置的 mock: 可预设禁用键集合与键值表 (对应 TestNaWhenBinding 的匿名
    /// ConfigProvider 与 ConfigurationService 读取行为的最小面)。
    pub struct MockConfigProvider {
        pub values: std::collections::HashMap<String, String>,
        pub disabled: HashSet<String>,
    }

    impl MockConfigProvider {
        pub fn new() -> Self {
            MockConfigProvider {
                values: std::collections::HashMap::new(),
                disabled: HashSet::new(),
            }
        }
    }

    impl ConfigProvider for MockConfigProvider {
        fn get_config(&self, key: &str) -> Option<String> {
            self.values.get(key).cloned()
        }

        fn set_config(&mut self, key: &str, value: &str) {
            self.values.insert(key.to_string(), value.to_string());
        }

        fn is_field_disabled(&self, key: &str) -> bool {
            self.disabled.contains(key)
        }
    }
}
