use super::*;

// ---- get_string: 游戏 8111 JSON (`"key": value` 冒号后带空格) 真实形态 ----
// (波20 清场: get_string_builder 与 _c 变体系死代码, 对应用例已删)

#[test]
fn get_string_numeric_and_bool_values() {
    let r = "{\"valid\": true, \"speed\": 123.45, \"H, m\": 5000}";
    assert_eq!(get_string(r, "valid"), Some("true"));
    assert_eq!(get_string(r, "speed"), Some("123.45"));
    assert_eq!(get_string(r, "H, m"), Some("5000"));
}

#[test]
fn get_string_string_value_keeps_quotes() {
    // 跳过的是前导空格 → 返回值含首尾引号 (调用方 Indicators 自行去壳)
    let r = "{\"type\": \"tu-4\", \"army\": \"1\"}";
    assert_eq!(get_string(r, "type"), Some("\"tu-4\""));
    assert_eq!(get_string(r, "army"), Some("\"1\""));
}

#[test]
fn get_string_cjk_value() {
    // CJK 值 (BMP): Java UTF-16 码元推进与 Rust 整字符推进等价
    let r = "{\"valid\": true, \"type\": \"歼-20\"}";
    let v = get_string(r, "type").unwrap();
    assert_eq!(v, "\"歼-20\"");
    // 调用方去壳 (Indicators.java 的 substring(1, len-1)):
    assert_eq!(&v[1..v.len() - 1], "歼-20");
}

#[test]
fn get_string_cjk_compact_json() {
    // 紧凑 JSON (冒号后无空格): 跳过的是值首字符 (引号), 尾引号保留 — Java 同此
    let r = "{\"type\":\"歼-20\"}";
    assert_eq!(get_string(r, "type"), Some("歼-20\""));
}

#[test]
fn get_string_comma_key() {
    // 键本身含逗号/空格/斜杠 (State.java 的 "IAS, km/h"): indexOf 整段子串匹配不受影响
    let r = "{\"foo\": 1, \"IAS, km/h\": 505, \"M\": 0.62}";
    assert_eq!(get_string(r, "IAS, km/h"), Some("505"));
    // State.java 用带引号键 "\"M\"" 定位
    assert_eq!(get_string(r, "\"M\""), Some("0.62"));
}

#[test]
fn get_string_missing_key_returns_none() {
    assert_eq!(get_string("{\"speed\": 1}", "altitude"), None);
}

#[test]
fn get_string_truncated_tail() {
    // 截断 JSON (e2e s6 畸形场景): 扫不到 ','/'}' 时取到串尾, 不抛
    assert_eq!(get_string("{\"speed\": 12", "speed"), Some("12"));
}

#[test]
fn get_string_first_occurrence() {
    // indexOf 取第一次出现 (与原 getStringBuilder 的 lastIndexOf 相对)
    assert_eq!(
        get_string("{\"speed\": 1, \"speed\": 2}", "speed"),
        Some("1")
    );
}

#[test]
fn get_string_empty_needle() {
    // Java indexOf("") == 0 ↔ Rust find("") == Some(0)
    assert_eq!(get_string("ab:cd,", ""), Some("d"));
}

#[test]
#[should_panic]
fn get_string_no_colon_panics_like_java() {
    // 扫不到 ':' → Java substring 抛 StringIndexOutOfBoundsException ↔ panic
    get_string("{\"speed\"", "speed");
}

// ---- 数值解析 ----

#[test]
fn get_data_float_widens_through_f32() {
    // Float.parseFloat 单精度后拓宽: "0.1" ≠ double 0.1 (0.1f32 的精确展开)
    assert_eq!(get_data_float(Some("0.1")), 0.100_000_001_490_116_12);
    assert_eq!(get_data_float(Some("0.1")), 0.1f32 as f64);
    assert_eq!(get_data_float(Some("123.45")), 123.45f32 as f64);
    assert_eq!(get_data_float(Some("1e3")), 1000.0);
    assert_eq!(get_data_float(Some("-6.5")), -6.5); // 二进制精确值, 无损
    assert_eq!(get_data_float(None), F_INVALID);
    assert_eq!(get_data_float(None), -65535.0);
}

#[test]
#[should_panic]
fn get_data_float_bad_number_panics_like_java() {
    get_data_float(Some("abc"));
}

#[test]
fn get_data_int_parses() {
    assert_eq!(get_data_int(Some("505")), 505);
    assert_eq!(get_data_int(Some("-42")), -42);
    assert_eq!(get_data_int(Some("+7")), 7);
    assert_eq!(get_data_int(None), I_INVALID);
    assert_eq!(get_data_int(None), -65535);
}

#[test]
#[should_panic]
fn get_data_int_overflow_panics_like_java() {
    get_data_int(Some("99999999999"));
}

#[test]
#[should_panic]
fn get_data_int_fraction_panics_like_java() {
    // Integer.parseInt 不接受小数点
    get_data_int(Some("12.5"));
}

#[test]
fn get_data_float_trims_whitespace_like_java() {
    // Java 8 oracle 实测: Float.parseFloat 忽略首尾空白
    assert_eq!(get_data_float(Some(" 1.5 ")), 1.5);
    assert_eq!(get_data_float(Some("  2.25")), 2.25);
    // 冒号后双空格的脏 payload: getString 只跳 1 码元 → 子串带前导空格,
    // Java parseFloat 正常解析, Rust 靠 trim 对齐
    assert_eq!(
        get_data_float(get_string("{\"speed\":  7.5}", "speed")),
        7.5
    );
    // 全空白 trim 后为空串: 两边均 NumberFormatException/panic
}

#[test]
#[should_panic]
fn get_data_float_all_whitespace_panics_like_java() {
    get_data_float(Some("  "));
}

#[test]
#[should_panic]
fn get_data_int_leading_space_panics_like_java() {
    // Java 8 oracle 实测 Integer.parseInt(" 5") 抛 — parseInt 不 trim, 保真不加
    get_data_int(Some(" 5"));
}

#[test]
fn get_string_prefix_key_matches_first_occurrence() {
    // 键名互为前缀 (Indicators.java 的 fuel/fuel1 用法): indexOf("fuel")
    // 命中 "fuel1" 键名的前缀, 返回**第一次**出现处的值 (Java oracle 实测 [100])
    let r = "{\"fuel1\": 100, \"fuel\": 200}";
    assert_eq!(get_string(r, "fuel"), Some("100"));
    assert_eq!(get_string(r, "fuel1"), Some("100"));
}

#[test]
fn end_to_end_extract_then_parse() {
    // State/Indicators 的典型链路: getString → getDataFloat/Int
    let r = "{\"IAS, km/h\": 505, \"type\": \"歼-20\", \"speed\": 65.5}";
    assert_eq!(get_data_int(get_string(r, "IAS, km/h")), 505);
    assert_eq!(get_data_float(get_string(r, "speed")), 65.5f32 as f64);
    let t = get_string(r, "type").unwrap();
    assert_eq!(&t[1..t.len() - 1], "歼-20"); // 调用方去壳
}
