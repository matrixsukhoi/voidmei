//! 对应 Java: `src/ui/window/comparison/logic/rules/LambdaRule.java` (一比一翻译)

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::comparison::comparison_rule::ComparisonRule;

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
        // Java: try { return extractor.apply(rawValue); } catch (Exception e) { return null; }
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
mod tests {
    use super::*;

    /// Java `Double.parseDouble` 的等价闭包: 其内部先 String.trim() 再解析
    /// (Java 8 oracle: " 3.5 " → 3.5; "abc" → NumberFormatException → catch → null)。
    fn parse_double_like(x: &str) -> Option<f64> {
        super::super::java_trim(x).parse::<f64>().ok()
    }

    #[test]
    fn null_and_empty_return_none() {
        // oracle: LAMBDA <null>/[] → null (extractor 不被调用)
        let r = LambdaRule::new(Box::new(parse_double_like), false);
        assert_eq!(r.extract_value(None), None);
        assert_eq!(r.extract_value(Some("")), None);
    }

    #[test]
    fn delegates_to_extractor_with_parse_double_trim_semantics() {
        // oracle: " 3.5 " → 3.5 (parseDouble 内部 trim)
        let r = LambdaRule::new(Box::new(parse_double_like), false);
        assert_eq!(r.extract_value(Some(" 3.5 ")).map(|v| v.to_bits()), Some(4615063718147915776));
    }

    #[test]
    fn extractor_parse_failure_returns_none() {
        // oracle: "abc" → parse 抛异常被 catch → null ↔ Rust parse 失败 → None
        let r = LambdaRule::new(Box::new(parse_double_like), false);
        assert_eq!(r.extract_value(Some("abc")), None);
    }

    #[test]
    fn extractor_panic_swallowed_like_java_exception() {
        // Java catch (Exception) → return null 的对应路径: 提取器 panic 被吞。
        // PORT: Rust 默认 panic hook 会向 stderr 打印 "boom" (Java 静默 catch 无
        // 输出) —— 换成仅对本文构造的 "boom" payload 静默的过滤 hook, 其余
        // panic 链式透传给原 hook (不干扰并行测试的失败诊断), 结束后还原。
        use std::sync::Arc;

        fn is_boom_payload(info: &std::panic::PanicHookInfo<'_>) -> bool {
            let s = info.payload();
            s.downcast_ref::<&str>().is_some_and(|p| *p == "boom")
                || s.downcast_ref::<String>().is_some_and(|p| p == "boom")
        }

        let prev = Arc::new(std::panic::take_hook());
        let filter = Arc::clone(&prev);
        std::panic::set_hook(Box::new(move |info| {
            if !is_boom_payload(info) {
                filter(info);
            }
        }));

        let r = LambdaRule::new(
            Box::new(|_| -> Option<f64> { panic!("boom") }),
            true,
        );
        let got = r.extract_value(Some("x"));

        let restore = Arc::clone(&prev);
        std::panic::set_hook(Box::new(move |info| restore(info)));

        assert_eq!(got, None);
        assert!(r.is_lower_better());
    }

    #[test]
    fn direction_flag_preserved() {
        assert!(!LambdaRule::new(Box::new(|_| None), false).is_lower_better());
    }
}
