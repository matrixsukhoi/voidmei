//! 对应 Java: `src/ui/window/comparison/logic/rules/SimpleRule.java` (一比一翻译)

use crate::comparison::comparison_rule::ComparisonRule;

/// Simple rule that extracts the first number from the value string.
/// Skips array/list values (starting with '[').
// PORT: Java `private static final Pattern NUMBER_PATTERN` (与 ListIndexRule/
/// MultiListIndexRule 逐字相同) → super::find_number 共享扫描函数。
pub struct SimpleRule {
    lower_is_better: bool,
}

impl SimpleRule {
    // Java: public SimpleRule(boolean lowerIsBetter)
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

        // Java: try { Matcher m = NUMBER_PATTERN.matcher(rawValue); if (m.find()) {
        //           return Double.parseDouble(m.group(1)); } } catch (Exception e) { // ignore }
        // PORT: 组1 文本仅含 [-.0-9], parse 对其不可能失败; Java catch 吞异常
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
// Tests — 期望值取自 Java 8 oracle 实测 (原类直跑, 逐位对拍)。
#[cfg(test)]
mod tests {
    use super::*;

    fn bits(v: Option<f64>) -> Option<u64> {
        v.map(|x| x.to_bits())
    }

    #[test]
    fn extracts_plain_number() {
        // oracle: "4644.0" → 4644.0
        let r = SimpleRule::lower_is_better();
        assert_eq!(r.extract_value(Some("4644.0")), Some(4644.0));
        assert_eq!(bits(r.extract_value(Some("4644.0"))), Some(4661828146700484608));
    }

    #[test]
    fn extracts_with_trailing_garbage() {
        // oracle: "-123.45xyz" → -123.45 (首个匹配, 后缀忽略)
        assert_eq!(
            bits(SimpleRule::new(true).extract_value(Some("-123.45xyz"))),
            Some((-4584984598449173299_i64) as u64)
        );
    }

    #[test]
    fn skips_array_values() {
        // oracle: "[144, 1167]" → null; 前导空格则不视为数组 (startsWith('[') 判定)
        assert_eq!(SimpleRule::lower_is_better().extract_value(Some("[144, 1167]")), None);
        assert_eq!(SimpleRule::lower_is_better().extract_value(Some(" [1,2]")), Some(1.0));
    }

    #[test]
    fn null_and_empty_return_none() {
        let r = SimpleRule::lower_is_better();
        assert_eq!(r.extract_value(None), None);
        assert_eq!(r.extract_value(Some("")), None);
        assert_eq!(r.extract_value(Some("abc")), None);
    }

    #[test]
    fn decimal_point_edges() {
        // oracle: ".5"→5.0 (无前导数字, 匹配到 "5"); "12."→12.0 (点后无数字, 点不入匹配);
        // "1.2.3"→1.2 (贪婪小数段)
        assert_eq!(SimpleRule::lower_is_better().extract_value(Some(".5")), Some(5.0));
        assert_eq!(SimpleRule::lower_is_better().extract_value(Some("12.")), Some(12.0));
        assert_eq!(
            bits(SimpleRule::lower_is_better().extract_value(Some("1.2.3"))),
            Some(4608083138725491507)
        );
    }

    #[test]
    fn minus_sign_edges() {
        // oracle: "- 5"→5.0 ('-' 后非数字, 起点右移); "--5"→-5.0 (第二个 '-' 生效)
        assert_eq!(SimpleRule::lower_is_better().extract_value(Some("- 5")), Some(5.0));
        assert_eq!(
            bits(SimpleRule::lower_is_better().extract_value(Some("--5"))),
            Some((-4606056518893174784_i64) as u64)
        );
    }

    #[test]
    fn cjk_and_scientific_notation() {
        // oracle: "千米5"→5.0 (\d 为 ASCII 定义, 多字节字符跳过);
        // "1e3"→1.0 (模式无指数部分, 首个匹配 "1")
        assert_eq!(SimpleRule::lower_is_better().extract_value(Some("千米5")), Some(5.0));
        assert_eq!(SimpleRule::lower_is_better().extract_value(Some("1e3")), Some(1.0));
    }

    #[test]
    fn zero_and_negative_zero() {
        // oracle: "0"→+0.0; "-0"→-0.0 (位型保真)
        assert_eq!(bits(SimpleRule::lower_is_better().extract_value(Some("0"))), Some(0));
        assert_eq!(
            bits(SimpleRule::lower_is_better().extract_value(Some("-0"))),
            Some((-9223372036854775808_i64) as u64)
        );
    }

    #[test]
    fn extreme_magnitudes_round_like_java() {
        // oracle: 47 个 9 → 1.0E47; 0.00…01 (1e-49) → 1.0E-49 — 十进制正确舍入
        assert_eq!(
            bits(SimpleRule::lower_is_better()
                .extract_value(Some("99999999999999999999999999999999999999999999999"))),
            Some(5310170741700075612)
        );
        assert_eq!(
            bits(SimpleRule::lower_is_better()
                .extract_value(Some("0.0000000000000000000000000000000000000000000000001"))),
            Some(3873857694494683923)
        );
    }

    #[test]
    fn factories_set_direction() {
        assert!(SimpleRule::lower_is_better().is_lower_better());
        assert!(!SimpleRule::higher_is_better().is_lower_better());
        assert_eq!(SimpleRule::higher_is_better().extract_value(Some("12")), Some(12.0));
        assert!(SimpleRule::new(true).is_lower_better());
    }
}
