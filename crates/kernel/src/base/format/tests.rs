use super::*;

#[test]
fn nan_to_na() {
    assert_eq!(format(f64::NAN, 0), "N/A");
}

#[test]
fn negative_zero_suppression() {
    // -0.001 精度2: 不足 0.005 阈值, 无负号
    assert_eq!(format(-0.001, 2), "0.00");
    // 恰好 -0.005: 达到阈值, 有负号
    assert_eq!(format(-0.005, 2), "-0.01");
    // -0.4 精度0: 不足 0.5 阈值, 无负号 (Java 语义)
    assert_eq!(format(-0.4, 0), "0");
}

#[test]
fn rounding() {
    // 波21: Rust nearest-even 语义
    assert_eq!(format(999.5, 0), "1000");
    assert_eq!(format(0.45, 1), "0.5");
    assert_eq!(format(0.44, 1), "0.4");
    // 整数+小数进位
    assert_eq!(format(1.99, 1), "2.0");
}

#[test]
fn integers_and_precision() {
    assert_eq!(format(500.0, 0), "500");
    assert_eq!(format(4.2, 1), "4.2");
    assert_eq!(format(0.456, 2), "0.46");
    // 整数部分补零语义: 0.5 精度1 → "0.5"
    assert_eq!(format(0.5, 1), "0.5");
    // 小数位补零
    assert_eq!(format(10.0, 2), "10.00");
}

#[test]
fn negative_values() {
    assert_eq!(format(-7.34, 1), "-7.3");
    assert_eq!(format(-123.456, 2), "-123.46");
}

#[test]
fn time_format() {
    assert_eq!(format_time(0.0), "00'00");
    assert_eq!(format_time(65.9), "01'05");
    assert_eq!(format_time(600.0), "10'00");
    assert_eq!(format_time(7200.0), "120'00");
    assert_eq!(format_time(-1.0), "--'--");
    assert_eq!(format_time(f64::NAN), "--'--");
    // 分钟溢出封顶 999
    assert_eq!(format_time(60000.0), "999'00");
}
