//! 数值格式化语义黑盒场景 (kernel::base::format)。
//! expect 表承载批量边界值; 期望值语义 = Java HALF_UP/负零抑制族。

use expect_test::expect;
use kernel::base::format::{fmt_f, format, format_time, java_d0, java_f_plus, pad_width};

/// HALF_UP 舍入边界 + 负零抑制 (HUD 全部读数的显示地基)
#[test]
fn 格式化_HALF_UP边界与负零() {
    let table: Vec<(f64, u8, String)> = vec![
        (0.004, 2, "0.00".into()),
        (0.005, 2, "0.01".into()),  // HALF_UP 恰半进位
        (-0.004, 2, "0.00".into()), // 不足阈值, 无负号
        (-0.005, 2, "-0.01".into()),
        (2.675, 2, "2.67".into()), // 二进制误差下的 Java 对齐行为
        (1.005, 2, "1.00".into()),
        (123.456, 1, "123.5".into()),
        (-123.456, 0, "-123".into()),
        (0.5, 0, "1".into()),
        (-0.5, 0, "-1".into()),
        (-0.4, 0, "0".into()),
    ];
    // 注: 行首勿带差异化前导空格 (expect-test 公共前缀剥离会吃掉, 生成/断言不自洽)
    let actual = table
        .iter()
        .map(|(v, p, _)| format!("{v} p{p} = {}", format(*v, *p)))
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        0.004 p2 = 0.00
        0.005 p2 = 0.01
        -0.004 p2 = 0.00
        -0.005 p2 = -0.01
        2.675 p2 = 2.67
        1.005 p2 = 1.00
        123.456 p1 = 123.5
        -123.456 p0 = -123
        0.5 p0 = 0
        -0.5 p0 = 0
        -0.4 p0 = 0"#]]
    .assert_eq(&actual);
}

/// fmt_f: 字面量定点 (无千分位) + 负零
#[test]
fn fmt_f_定点与负零() {
    let actual = format!(
        "{}|{}|{}|{}|{}",
        fmt_f(0.0, 2),
        fmt_f(-0.0001, 2),
        fmt_f(1234.567, 1),
        fmt_f(-42.0, 0),
        fmt_f(0.125, 2),
    );
    expect!["0.00|-0.00|1234.6|-42|0.12"].assert_eq(&actual);
}

/// java_f_plus: 正数带 + (能量增减类读数)
#[test]
fn java_f_plus_带符号() {
    let actual = format!("{}|{}|{}", java_f_plus(1.5, 1), java_f_plus(-1.5, 1), java_f_plus(0.0, 0));
    expect!["+1.5|-1.5|+0"].assert_eq(&actual);
}

/// format_time: 秒 → mm'ss" 形态与进位
#[test]
fn format_time_分秒形态() {
    let actual = [
        format_time(0.0),
        format_time(5.4),
        format_time(59.9),
        format_time(60.0),
        format_time(125.0),
        format_time(-1.0),
    ]
    .join("|");
    expect!["00'00|00'05|00'59|01'00|02'05|--'--"].assert_eq(&actual);
}

/// pad_width / java_d0: 列对齐填充
#[test]
fn 对齐_宽度填充() {
    let actual = format!(
        "[{}]|[{}]|[{}]|[{}]",
        pad_width("42".into(), 5, true),
        pad_width("42".into(), 5, false),
        java_d0(42, 5),
        java_d0(-7, 3),
    );
    expect!["[42   ]|[   42]|[00042]|[-07]"].assert_eq(&actual);
}
