//! MarkerType 的 Rust 移植 (src/ui/component/gauge/MarkerType.java)。
//! PORT: Java 枚举常量 SCREAMING_SNAKE (LINE_FULL) → Rust 变体 PascalCase
//! (LineFull), 命名映射规则同类名 (non_camel_case_types lint)。

/// Enumeration of marker types for MarkedGauge component.
///
/// Each type represents a different visual style for gauge markers:
/// - LINE_FULL: A line spanning the full width of the gauge (like Mach red line in SpeedRatioBar)
/// - LINE_PARTIAL: A partial-width tick mark (like aileron/rudder lock lines)
/// - ZONE: A filled region marking a range (like stall warning zone)
/// - TICK_LABELED: A tick mark with an associated text label
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerType {
    /// Full-width line spanning the entire gauge width.
    /// Example: Mach limit red line, optimal compressor stage indicator.
    LineFull,

    /// Partial-width tick mark, typically on one side of the gauge.
    /// Use GaugeMarker.side to control position (-1=left, 0=center, 1=right).
    /// Use GaugeMarker.widthRatio to control tick length as a ratio of gauge width.
    LinePartial,

    /// Filled rectangular zone marking a range.
    /// Use GaugeMarker.widthRatio to control zone width as a ratio of gauge width.
    /// Use GaugeMarker.side to position the zone.
    Zone,

    /// Tick mark with an attached text label.
    /// Use GaugeMarker.label for the text content.
    TickLabeled,
}

#[cfg(test)]
mod tests {
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
}
