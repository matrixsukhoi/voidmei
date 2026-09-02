//! 对应 Java: `src/ui/window/comparison/logic/rules/LambdaRule.java` (一比一翻译)

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::ui_support::comparison::comparison_rule::ComparisonRule;

/// Rule that uses a custom lambda function to extract values.
/// Useful for complex extraction patterns that don't fit the standard rules.
///
/// Example usage:
/// ```ignore
/// LambdaRule::new(
///     Box::new(|raw: &str| /* Matcher m = Pattern.compile("副翼(\\d+)").matcher(raw);
///                              m.find() ? parse : None */ None),
///     false, // higher is better
/// )
/// ```
// PORT: Java `Function<String, Double>` (装箱 Double 可 null) →
/// `Box<dyn Fn(&str) -> Option<f64> + Send + Sync>` (失败语义由 Option 承载);
// +Send+Sync 仅为让本类型可进 ComparisonRules 的全局静态注册表 (static 要求
// Sync), 提取器均为纯函数, 与 Java 无线程语义差异。
pub struct LambdaRule {
    // PORT: Java 保真 — `Function<String, Double>` 的 Rust 对应形态 (见上注),
    // trait object + Send/Sync 约束为一体签名, 不拆 type 别名
    #[allow(clippy::type_complexity)]
    extractor: Box<dyn Fn(&str) -> Option<f64> + Send + Sync>,
    lower_is_better: bool,
}

impl LambdaRule {
    /// @param extractor function that extracts a Double from the raw value string
    /// @param lower_is_better true if lower values are better
    // PORT: Java 保真 — 形参类型即 extractor 字段类型 (Function 移植), 不拆别名
    #[allow(clippy::type_complexity)]
    pub fn new(
        extractor: Box<dyn Fn(&str) -> Option<f64> + Send + Sync>,
        lower_is_better: bool,
    ) -> Self {
        Self { extractor, lower_is_better }
    }
}

impl ComparisonRule for LambdaRule {
    fn extract_value(&self, raw_value: Option<&str>) -> Option<f64> {
        let raw_value = raw_value?;
        if raw_value.is_empty() {
            return None;
        }
        // PORT: Java 异常控制流 (提取器抛任意异常 → 吞掉返回 null) ↔ Rust panic
        // 捕获; 提取器为纯扫描时恒不触发。与 Java 的微小差异: Rust 默认 panic
        // hook 仍会向 stderr 打印 panic 消息 (Java 静默) —— 不在库代码里替换
        // 全局 hook (会波及并发线程真实 panic 的诊断输出), 测试侧按 payload
        // 精准抑制 (见 extractor_panic_swallowed_like_java_exception)。
        catch_unwind(AssertUnwindSafe(|| (self.extractor)(raw_value))).unwrap_or(None)
    }

    fn is_lower_better(&self) -> bool {
        self.lower_is_better
    }
}

// =====================================================================
// Tests — 期望值取自 Java 8 oracle 实测 (提取器取 `x -> Double.parseDouble(x)`)。
#[cfg(test)]
mod tests;
