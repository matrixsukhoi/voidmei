use super::*;
use std::collections::HashSet;
use std::mem::discriminant;

/// 四个变体互异 (对应 Java enum 常量两两 !=), 与 Java 名字一一对应。
#[test]
fn four_distinct_variants() {
    let all = [
        MarkerType::LineFull,
        MarkerType::LinePartial,
        MarkerType::Zone,
        MarkerType::TickLabeled,
    ];
    let mut seen_disc = HashSet::new();
    let mut seen_name = HashSet::new();
    for v in &all {
        assert!(seen_disc.insert(discriminant(v)), "重复变体: {:?}", v);
        assert!(seen_name.insert(format!("{:?}", v)));
    }
    assert_eq!(seen_name.len(), 4);
    assert_eq!(
        seen_name,
        ["LineFull", "LinePartial", "Zone", "TickLabeled"]
            .into_iter()
            .map(String::from)
            .collect::<HashSet<_>>()
    );
}

/// Copy 语义: 值传递不移动原值 (Java 枚举是引用语义的不可变常量,
/// Rust Copy 等价地支持随处复制的用法)。
#[test]
fn marker_type_is_copy() {
    fn assert_copy<T: Copy>() {}
    assert_copy::<MarkerType>();
}
