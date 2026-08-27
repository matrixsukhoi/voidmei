//! 对应 Java: `src/ui/model/DefaultFieldManager.java` (一比一翻译)

use crate::ui_model::config_stub::ConfigProvider;
use crate::ui_model::data_field::DataField;
use crate::ui_model::field_manager::FieldManager;
use std::collections::HashMap;

/// Default implementation of FieldManager.
/// Uses ArrayList for ordered storage and HashMap for fast lookup.
pub struct DefaultFieldManager {
    // PORT: Java `List<DataField> fields` + `Map<String, DataField> fieldMap` 两处
    // 持同一对象引用 → Vec 持有 + 键→下标映射 (无 Rc<RefCell> 共享必要:
    // 所有读改经 manager 自身方法或借用通道, clear_all 前 Vec 无删除操作,
    // 下标恒有效)。fieldMap 无迭代点, 输出顺序不受 §2.5 影响 → std HashMap。
    fields: Vec<DataField>,
    field_map: HashMap<String, usize>,
    config: Option<Box<dyn ConfigProvider>>,
}

impl DefaultFieldManager {
    pub fn new(config: Option<Box<dyn ConfigProvider>>) -> DefaultFieldManager {
        DefaultFieldManager {
            fields: Vec::new(),
            field_map: HashMap::new(),
            config,
        }
    }
}

impl FieldManager for DefaultFieldManager {
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
    ) {
        // Java: if (config != null && configKey != null) { if (config.isFieldDisabled(configKey)) return; }
        if let (Some(config), Some(cfg_key)) = (&self.config, config_key) {
            if config.is_field_disabled(cfg_key) {
                return;
            }
        }

        // PORT: Java 此处可把 null configKey 透传进 DataField.configKey (该字段全库
        // 无读取点, 为 write-only 状态) —— Rust String 以 "" 哨兵承接, 零行为差异
        let mut field = DataField::new(key, label, unit, config_key.unwrap_or(""), hide_when_na, hide_when_zero);
        field.format = format.map(|f| f.to_string());
        if let Some(preview) = preview_value {
            // PORT: Java 直接赋 currentValue (绕过 setValue 的 %5s 右对齐) —— 原样
            field.current_value = preview.to_string();
        }
        self.fields.push(field);
        // Java fieldMap.put(key, field): 重复 key 覆盖旧映射 → getField 指向最新
        self.field_map.insert(key.to_string(), self.fields.len() - 1);
    }

    fn update_field(&mut self, key: &str, value: &str, na_string: &str) {
        if let Some(&idx) = self.field_map.get(key) {
            self.fields[idx].set_value_with_visibility(value, na_string);
        }
    }

    fn update_field_unit(&mut self, key: &str, unit: &str) {
        if let Some(&idx) = self.field_map.get(key) {
            self.fields[idx].set_unit(unit);
        }
    }

    fn bind(&mut self, key: &str, supplier: Box<dyn Fn() -> f64>, precision: i32) {
        self.bind_with_visibility(key, supplier, None, precision, None);
    }

    fn bind_with_visibility(
        &mut self,
        key: &str,
        value_supplier: Box<dyn Fn() -> f64>,
        visibility_supplier: Option<Box<dyn Fn() -> bool>>,
        precision: i32,
        format: Option<&str>,
    ) {
        if let Some(&idx) = self.field_map.get(key) {
            let field = &mut self.fields[idx];
            field.value_supplier = Some(value_supplier);
            field.visibility_supplier = visibility_supplier;
            field.precision = precision;
            // PORT: Java 原样覆盖 (format=null 也会清掉 addField 设置的 format)
            field.format = format.map(|f| f.to_string());
        }
    }

    fn bind_dynamic(
        &mut self,
        key: &str,
        value_supplier: Box<dyn Fn() -> f64>,
        visibility_supplier: Option<Box<dyn Fn() -> bool>>,
        precision: i32,
        format: Option<&str>,
        unit_supplier: Option<Box<dyn Fn() -> String>>,
        precision_supplier: Option<Box<dyn Fn() -> i32>>,
    ) {
        if let Some(&idx) = self.field_map.get(key) {
            let field = &mut self.fields[idx];
            field.value_supplier = Some(value_supplier);
            field.visibility_supplier = visibility_supplier;
            field.precision = precision;
            field.format = format.map(|f| f.to_string());
            field.unit_supplier = unit_supplier;
            field.precision_supplier = precision_supplier;
        }
    }

    fn get_fields(&self) -> &[DataField] {
        &self.fields
    }

    fn get_fields_mut(&mut self) -> &mut [DataField] {
        &mut self.fields
    }

    fn get_field(&self, key: &str) -> Option<&DataField> {
        // Java fieldMap.get(key) → null 映射为 None
        self.field_map.get(key).map(|&idx| &self.fields[idx])
    }

    fn get_field_mut(&mut self, key: &str) -> Option<&mut DataField> {
        match self.field_map.get(key) {
            Some(&idx) => Some(&mut self.fields[idx]),
            None => None,
        }
    }

    fn clear_all(&mut self) {
        self.fields.clear();
        self.field_map.clear();
    }

    fn size(&self) -> i32 {
        self.fields.len() as i32
    }

    fn set_field_visible(&mut self, key: &str, visible: bool) {
        if let Some(&idx) = self.field_map.get(key) {
            self.fields[idx].visible = visible;
        }
    }

    fn visible_count(&self) -> i32 {
        let mut count = 0;
        for field in &self.fields {
            if field.visible {
                count += 1;
            }
        }
        count
    }
}

#[cfg(test)]
mod tests;
