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
