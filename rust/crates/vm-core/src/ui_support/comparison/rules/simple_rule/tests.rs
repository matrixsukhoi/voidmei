use super::*;

fn bits(v: Option<f64>) -> Option<u64> {
    v.map(|x| x.to_bits())
}

#[test]
fn extracts_plain_number() {
    // oracle: "4644.0" → 4644.0
    let r = SimpleRule::lower_is_better();
    assert_eq!(r.extract_value(Some("4644.0")), Some(4644.0));
    assert_eq!(
        bits(r.extract_value(Some("4644.0"))),
        Some(4661828146700484608)
    );
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
    assert_eq!(
        SimpleRule::lower_is_better().extract_value(Some("[144, 1167]")),
        None
    );
    assert_eq!(
        SimpleRule::lower_is_better().extract_value(Some(" [1,2]")),
        Some(1.0)
    );
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
    assert_eq!(
        SimpleRule::lower_is_better().extract_value(Some(".5")),
        Some(5.0)
    );
    assert_eq!(
        SimpleRule::lower_is_better().extract_value(Some("12.")),
        Some(12.0)
    );
    assert_eq!(
        bits(SimpleRule::lower_is_better().extract_value(Some("1.2.3"))),
        Some(4608083138725491507)
    );
}

#[test]
fn minus_sign_edges() {
    // oracle: "- 5"→5.0 ('-' 后非数字, 起点右移); "--5"→-5.0 (第二个 '-' 生效)
    assert_eq!(
        SimpleRule::lower_is_better().extract_value(Some("- 5")),
        Some(5.0)
    );
    assert_eq!(
        bits(SimpleRule::lower_is_better().extract_value(Some("--5"))),
        Some((-4606056518893174784_i64) as u64)
    );
}

#[test]
fn cjk_and_scientific_notation() {
    // oracle: "千米5"→5.0 (\d 为 ASCII 定义, 多字节字符跳过);
    // "1e3"→1.0 (模式无指数部分, 首个匹配 "1")
    assert_eq!(
        SimpleRule::lower_is_better().extract_value(Some("千米5")),
        Some(5.0)
    );
    assert_eq!(
        SimpleRule::lower_is_better().extract_value(Some("1e3")),
        Some(1.0)
    );
}

#[test]
fn zero_and_negative_zero() {
    // oracle: "0"→+0.0; "-0"→-0.0 (位型保真)
    assert_eq!(
        bits(SimpleRule::lower_is_better().extract_value(Some("0"))),
        Some(0)
    );
    assert_eq!(
        bits(SimpleRule::lower_is_better().extract_value(Some("-0"))),
        Some((-9223372036854775808_i64) as u64)
    );
}

#[test]
fn extreme_magnitudes_round_like_java() {
    // oracle: 47 个 9 → 1.0E47; 0.00…01 (1e-49) → 1.0E-49 — 十进制正确舍入
    assert_eq!(
        bits(
            SimpleRule::lower_is_better()
                .extract_value(Some("99999999999999999999999999999999999999999999999"))
        ),
        Some(5310170741700075612)
    );
    assert_eq!(
        bits(
            SimpleRule::lower_is_better()
                .extract_value(Some("0.0000000000000000000000000000000000000000000000001"))
        ),
        Some(3873857694494683923)
    );
}

#[test]
fn factories_set_direction() {
    assert!(SimpleRule::lower_is_better().is_lower_better());
    assert!(!SimpleRule::higher_is_better().is_lower_better());
    assert_eq!(
        SimpleRule::higher_is_better().extract_value(Some("12")),
        Some(12.0)
    );
    assert!(SimpleRule::new(true).is_lower_better());
}
