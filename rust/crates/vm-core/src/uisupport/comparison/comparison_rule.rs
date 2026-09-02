//! 对应 Java: `src/ui/window/comparison/logic/ComparisonRule.java` (一比一翻译)
//!
//! FM 对比规则接口。Java interface 有 4 个实现类 (SimpleRule / ListIndexRule /
//! MultiListIndexRule / LambdaRule), 按 PORTING.md §1 映射为 dyn trait。

/// Interface for FM comparison rules.
/// Each rule defines how to extract a comparable numeric value from a raw string
/// and whether lower values are considered better.
// PORT: Java interface(多实现) → dyn trait (PORTING.md §1);
// 返回 `Double` (可 null) → Option<f64>; 参数可空 `String rawValue`
// (各实现首行判 null) → Option<&str>。
pub trait ComparisonRule {
    /// Extract a comparable numeric value from the raw string.
    ///
    /// @param raw_value the raw value string (e.g., "4644.0", "[144, 1167]")
    /// @return the extracted Double value, or null if extraction fails
    fn extract_value(&self, raw_value: Option<&str>) -> Option<f64>;

    /// @return true if lower values are better, false if higher values are better
    fn is_lower_better(&self) -> bool;
}
