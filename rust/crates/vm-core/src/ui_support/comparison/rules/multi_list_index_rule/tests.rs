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
    assert_eq!(
        MultiListIndexRule::new(2, 0, false).extract_value(TWO_LISTS),
        None
    );
}

#[test]
fn item_index_out_of_range_returns_none() {
    // oracle: ii=5 → 越界 null
    assert_eq!(
        MultiListIndexRule::new(0, 5, false).extract_value(TWO_LISTS),
        None
    );
}

#[test]
fn negative_list_index_returns_none() {
    // oracle: li=-1 → guard `listIndex >= 0` 短路 → null
    assert_eq!(
        MultiListIndexRule::new(-1, 0, false).extract_value(Some("[8.5, -4.2]")),
        None
    );
}

#[test]
fn no_brackets_returns_none() {
    // oracle: 无任何 [..] 列表 → lists 为空 → null
    assert_eq!(
        MultiListIndexRule::new(0, 0, false).extract_value(Some("no list")),
        None
    );
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
