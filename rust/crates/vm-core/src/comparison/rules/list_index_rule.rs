//! 对应 Java: `src/ui/window/comparison/logic/rules/ListIndexRule.java` (一比一翻译)

use crate::comparison::comparison_rule::ComparisonRule;

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
        Self { index, lower_is_better }
    }
}

impl ComparisonRule for ListIndexRule {
    fn extract_value(&self, raw_value: Option<&str>) -> Option<f64> {
        let raw_value = raw_value?;
        if raw_value.is_empty() {
            return None;
        }

        // Java: try { ... } catch (Exception e) { // ignore } — 本体为纯扫描,
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
mod tests {
    use super::*;

    #[test]
    fn extracts_index_from_list() {
        // oracle: "[144, 1167]" idx=1 → 1167.0
        let r = ListIndexRule::new(1, false);
        assert_eq!(r.extract_value(Some("[144, 1167]")), Some(1167.0));
        assert!(!r.is_lower_better());
    }

    #[test]
    fn only_first_bracketed_list_matched() {
        // oracle: 只取首个 [..] 列表, 第二个 "[10.1, -5.3]" 不参与
        let s = Some("[8.5, -4.2], [10.1, -5.3]");
        assert_eq!(ListIndexRule::new(0, false).extract_value(s), Some(8.5));
        assert_eq!(
            ListIndexRule::new(1, false).extract_value(s).map(|v| v.to_bits()),
            Some((-4606957238818648883_i64) as u64)
        );
    }

    #[test]
    fn index_out_of_range_returns_none() {
        // oracle: idx=2 / idx=3 超 parts.length → null
        assert_eq!(ListIndexRule::new(2, false).extract_value(Some("[144, 1167]")), None);
        assert_eq!(ListIndexRule::new(3, false).extract_value(Some("[1, 2, 3]")), None);
        // oracle: 单元素列表取 idx=1 → null
        assert_eq!(ListIndexRule::new(1, false).extract_value(Some("[0.5]")), None);
    }

    #[test]
    fn negative_index_returns_none() {
        // oracle: idx=-1 → guard `index >= 0` 短路 → null
        assert_eq!(ListIndexRule::new(-1, false).extract_value(Some("[1,2]")), None);
    }

    #[test]
    fn non_numeric_item_returns_none() {
        // oracle: "[a, b]" idx=1 → null (无数字); "[x-4.5]" idx=1 → 越界 null
        assert_eq!(ListIndexRule::new(1, false).extract_value(Some("[a, b]")), None);
        assert_eq!(ListIndexRule::new(1, false).extract_value(Some("[x-4.5]")), None);
    }

    #[test]
    fn no_brackets_returns_none() {
        // oracle: 无 '[' → LIST_PATTERN 不命中; 无 ']' 收尾 (未闭合) 同样不命中
        assert_eq!(ListIndexRule::new(1, false).extract_value(Some("144, 1167]")), None);
        assert_eq!(ListIndexRule::new(1, false).extract_value(Some("[unterminated")), None);
    }

    #[test]
    fn empty_list_returns_none() {
        // oracle: "[]" → [^\]]+ 要求至少 1 字符 → 不命中;
        // "[ ]" → 内容 " " 无数字 → null
        assert_eq!(ListIndexRule::new(0, false).extract_value(Some("[]")), None);
        assert_eq!(ListIndexRule::new(0, false).extract_value(Some("[ ]")), None);
    }

    #[test]
    fn nested_bracket_takes_inner_prefix_as_content() {
        // oracle: "[[a],[b]]" idx=0 → 首个匹配内容为 "[a" (贪婪到首个 ']'), 无数字 → null
        assert_eq!(ListIndexRule::new(0, false).extract_value(Some("[[a],[b]]")), None);
    }

    #[test]
    fn java_split_trailing_empty_semantics() {
        // oracle: "[1,2,]" → 尾部空段被 Java split 移除, idx=1 = "2" → 2.0;
        // "[1, 2, 3, ]" → 尾段 " " 非空保留, idx=1 = " 2" trim → 2.0;
        // "[,]" → split 后长度 0 → 越界 null; "[1,,2]" → idx=1 为空串 → null
        assert_eq!(ListIndexRule::new(1, false).extract_value(Some("[1,2,]")), Some(2.0));
        assert_eq!(ListIndexRule::new(1, false).extract_value(Some("[1, 2, 3, ]")), Some(2.0));
        assert_eq!(ListIndexRule::new(1, false).extract_value(Some("[,]")), None);
        assert_eq!(ListIndexRule::new(1, false).extract_value(Some("[1,,2]")), None);
    }

    #[test]
    fn null_and_empty_return_none() {
        let r = ListIndexRule::new(1, false);
        assert_eq!(r.extract_value(None), None);
        assert_eq!(r.extract_value(Some("")), None);
    }
}
