//! 对应 Java: `src/ui/window/comparison/logic/rules/SimpleRule.java` (一比一翻译)

use crate::ui_support::comparison::comparison_rule::ComparisonRule;

/// Simple rule that extracts the first number from the value string.
/// Skips array/list values (starting with '[').
// Java `private static final Pattern NUMBER_PATTERN` (与 ListIndexRule/
/// MultiListIndexRule 逐字相同) → super::find_number 共享扫描函数。
pub struct SimpleRule {
    lower_is_better: bool,
}

impl SimpleRule {
    pub fn new(lower_is_better: bool) -> Self {
        Self { lower_is_better }
    }

    /// Create a rule where lower values are better (e.g., weight, drag)
    pub fn lower_is_better() -> Self {
        Self::new(true)
    }

    /// Create a rule where higher values are better (e.g., speed, thrust)
    pub fn higher_is_better() -> Self {
        Self::new(false)
    }
}

impl ComparisonRule for SimpleRule {
    fn extract_value(&self, raw_value: Option<&str>) -> Option<f64> {
        let raw_value = raw_value?;
        if raw_value.is_empty() {
            return None;
        }
        if raw_value.starts_with('[') {
            return None; // Skip array values - use ListIndexRule for these
        }

        //           return Double.parseDouble(m.group(1)); } } catch (Exception e) { // ignore }
        // 组1 文本仅含 [-.0-9], parse 对其不可能失败; Java catch 吞异常
        // 继续走到末尾 return null ↔ Rust `.ok()`。
        if let Some(num) = super::find_number(raw_value) {
            if let Ok(v) = num.parse::<f64>() {
                return Some(v);
            }
        }
        None
    }

    fn is_lower_better(&self) -> bool {
        self.lower_is_better
    }
}

// =====================================================================
// Tests — 期望值取自 历史基线 (原类直跑, 逐位对拍)。
#[cfg(test)]
mod tests;
