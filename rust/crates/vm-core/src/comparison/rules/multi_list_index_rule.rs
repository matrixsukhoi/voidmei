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

        // Java: try { ... } catch (Exception e) { // ignore } — 本体为纯扫描,
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
mod tests {
    use super::*;

    const TWO_LISTS: Option<&str> = Some("[8.5, -4.2], [10.1, -5.3]");

    fn bits(v: Option<f64>) -> Option<u64> {
        v.map(|x| x.to_bits())
    }

    #[test]
    fn extracts_from_specified_list_and_item() {
        // oracle: (0,1)→-4.2; (1,1)→-5.3; (1,0)→10.1
        assert_eq!(
            bits(MultiListIndexRule::new(0, 1, false).extract_value(TWO_LISTS)),
            Some((-4606957238818648883_i64) as u64)
        );
        assert_eq!(
            bits(MultiListIndexRule::new(1, 1, false).extract_value(TWO_LISTS)),
            Some((-4605718748921121997_i64) as u64)
        );
        assert_eq!(
            bits(MultiListIndexRule::new(1, 0, false).extract_value(TWO_LISTS)),
            Some(4621875412584313651)
        );
    }

    #[test]
    fn list_index_out_of_range_returns_none() {
        // oracle: li=2 → 越界 null
        assert_eq!(MultiListIndexRule::new(2, 0, false).extract_value(TWO_LISTS), None);
    }

    #[test]
    fn item_index_out_of_range_returns_none() {
        // oracle: ii=5 → 越界 null
        assert_eq!(MultiListIndexRule::new(0, 5, false).extract_value(TWO_LISTS), None);
    }

    #[test]
    fn negative_list_index_returns_none() {
        // oracle: li=-1 → guard `listIndex >= 0` 短路 → null
        assert_eq!(MultiListIndexRule::new(-1, 0, false).extract_value(Some("[8.5, -4.2]")), None);
    }

    #[test]
    fn no_brackets_returns_none() {
        // oracle: 无任何 [..] 列表 → lists 为空 → null
        assert_eq!(MultiListIndexRule::new(0, 0, false).extract_value(Some("no list")), None);
    }

    #[test]
    fn nested_lists_content_is_inner_prefix() {
        // oracle: "[[a, b], [c, d]]" — find_all 取到 ["[a, b", "c, d"]:
        // (0,0) = "[a" 无数字 → null; (1,1) = "d" 无数字 → null
        let s = Some("[[a, b], [c, d]]");
        assert_eq!(MultiListIndexRule::new(0, 0, false).extract_value(s), None);
        assert_eq!(MultiListIndexRule::new(1, 1, false).extract_value(s), None);
    }

    #[test]
    fn null_and_empty_return_none() {
        let r = MultiListIndexRule::new(0, 1, false);
        assert_eq!(r.extract_value(None), None);
        assert_eq!(r.extract_value(Some("")), None);
        assert!(!r.is_lower_better());
    }
}
