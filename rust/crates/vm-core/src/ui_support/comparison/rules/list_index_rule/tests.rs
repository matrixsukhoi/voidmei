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
        ListIndexRule::new(1, false)
            .extract_value(s)
            .map(|v| v.to_bits()),
        Some((-4606957238818648883_i64) as u64)
    );
}

#[test]
fn index_out_of_range_returns_none() {
    // oracle: idx=2 / idx=3 超 parts.length → null
    assert_eq!(
        ListIndexRule::new(2, false).extract_value(Some("[144, 1167]")),
        None
    );
    assert_eq!(
        ListIndexRule::new(3, false).extract_value(Some("[1, 2, 3]")),
        None
    );
    // oracle: 单元素列表取 idx=1 → null
    assert_eq!(
        ListIndexRule::new(1, false).extract_value(Some("[0.5]")),
        None
    );
}

#[test]
fn negative_index_returns_none() {
    // oracle: idx=-1 → guard `index >= 0` 短路 → null
    assert_eq!(
        ListIndexRule::new(-1, false).extract_value(Some("[1,2]")),
        None
    );
}

#[test]
fn non_numeric_item_returns_none() {
    // oracle: "[a, b]" idx=1 → null (无数字); "[x-4.5]" idx=1 → 越界 null
    assert_eq!(
        ListIndexRule::new(1, false).extract_value(Some("[a, b]")),
        None
    );
    assert_eq!(
        ListIndexRule::new(1, false).extract_value(Some("[x-4.5]")),
        None
    );
}

#[test]
fn no_brackets_returns_none() {
    // oracle: 无 '[' → LIST_PATTERN 不命中; 无 ']' 收尾 (未闭合) 同样不命中
    assert_eq!(
        ListIndexRule::new(1, false).extract_value(Some("144, 1167]")),
        None
    );
    assert_eq!(
        ListIndexRule::new(1, false).extract_value(Some("[unterminated")),
        None
    );
}

#[test]
fn empty_list_returns_none() {
    // oracle: "[]" → [^\]]+ 要求至少 1 字符 → 不命中;
    // "[ ]" → 内容 " " 无数字 → null
    assert_eq!(ListIndexRule::new(0, false).extract_value(Some("[]")), None);
    assert_eq!(
        ListIndexRule::new(0, false).extract_value(Some("[ ]")),
        None
    );
}

#[test]
fn nested_bracket_takes_inner_prefix_as_content() {
    // oracle: "[[a],[b]]" idx=0 → 首个匹配内容为 "[a" (贪婪到首个 ']'), 无数字 → null
    assert_eq!(
        ListIndexRule::new(0, false).extract_value(Some("[[a],[b]]")),
        None
    );
}

#[test]
fn java_split_trailing_empty_semantics() {
    // oracle: "[1,2,]" → 尾部空段被 Java split 移除, idx=1 = "2" → 2.0;
    // "[1, 2, 3, ]" → 尾段 " " 非空保留, idx=1 = " 2" trim → 2.0;
    // "[,]" → split 后长度 0 → 越界 null; "[1,,2]" → idx=1 为空串 → null
    assert_eq!(
        ListIndexRule::new(1, false).extract_value(Some("[1,2,]")),
        Some(2.0)
    );
    assert_eq!(
        ListIndexRule::new(1, false).extract_value(Some("[1, 2, 3, ]")),
        Some(2.0)
    );
    assert_eq!(
        ListIndexRule::new(1, false).extract_value(Some("[,]")),
        None
    );
    assert_eq!(
        ListIndexRule::new(1, false).extract_value(Some("[1,,2]")),
        None
    );
}

#[test]
fn null_and_empty_return_none() {
    let r = ListIndexRule::new(1, false);
    assert_eq!(r.extract_value(None), None);
    assert_eq!(r.extract_value(Some("")), None);
}
