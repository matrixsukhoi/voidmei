//! 对应 Java: `src/ui/window/comparison/logic/rules/MultiListIndexRule.java` (一比一翻译)

use crate::comparison::comparison_rule::ComparisonRule;

/// Rule that extracts a value from a specific position in nested lists.
///
/// Example: "[4.5, 11.2], [5.0, 12.0]" with listIndex=0, itemIndex=1 extracts 11.2
// PORT: `private static final Pattern LIST_PATTERN / NUMBER_PATTERN` (与
/// ListIndexRule 逐字相同) → super 共享扫描函数。
pub struct MultiListIndexRule {
    list_index: i32,
    item_index: i32,
    lower_is_better: bool,
}

impl MultiListIndexRule {
    /// @param list_index the 0-based index of the list to use
    /// @param item_index the 0-based index within that list
    /// @param lower_is_better true if lower values are better
    pub fn new(list_index: i32, item_index: i32, lower_is_better: bool) -> Self {
        Self { list_index, item_index, lower_is_better }
    }
}

impl ComparisonRule for MultiListIndexRule {
    fn extract_value(&self, raw_value: Option<&str>) -> Option<f64> {
        let raw_value = raw_value?;
        if raw_value.is_empty() {
            return None;
        }

        // 与 Java 一致无异常路径, 末尾统一 return null。
        // Find all lists
        let lists = super::find_all_bracket_lists(raw_value);

        if self.list_index >= 0 && (self.list_index as usize) < lists.len() {
            let list_content = lists[self.list_index as usize];
            let parts = super::java_split_comma(list_content);

            if self.item_index >= 0 && (self.item_index as usize) < parts.len() {
                let part = super::java_trim(parts[self.item_index as usize]);
                if let Some(num) = super::find_number(part) {
                    if let Ok(v) = num.parse::<f64>() {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    fn is_lower_better(&self) -> bool {
        self.lower_is_better
    }
}

// =====================================================================
// Tests — 期望值取自 Java 8 oracle 实测 (原类直跑, 逐位对拍)。
#[cfg(test)]
mod tests;
