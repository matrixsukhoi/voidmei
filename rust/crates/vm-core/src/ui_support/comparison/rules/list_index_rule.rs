//! 对应 Java: `src/ui/window/comparison/logic/rules/ListIndexRule.java` (一比一翻译)

use crate::ui_support::comparison::comparison_rule::ComparisonRule;

/// Rule that extracts a value from a specific index in a list/array format.
///
/// Example: "[144, 1167]" with index=1 extracts 1167
// PORT: `private static final Pattern LIST_PATTERN / NUMBER_PATTERN` (与
/// MultiListIndexRule 逐字相同) → super 共享扫描函数。
pub struct ListIndexRule {
    index: i32,
    lower_is_better: bool,
}

impl ListIndexRule {
    /// @param index the 0-based index to extract from the list
    /// @param lower_is_better true if lower values are better
    pub fn new(index: i32, lower_is_better: bool) -> Self {
        Self {
            index,
            lower_is_better,
        }
    }
}

impl ComparisonRule for ListIndexRule {
    fn extract_value(&self, raw_value: Option<&str>) -> Option<f64> {
        let raw_value = raw_value?;
        if raw_value.is_empty() {
            return None;
        }

        // 与 Java 一致无异常路径, 末尾统一 return null。
        if let Some(list_content) = super::find_bracket_list(raw_value) {
            let parts = super::java_split_comma(list_content);

            if self.index >= 0 && (self.index as usize) < parts.len() {
                let part = super::java_trim(parts[self.index as usize]);
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
