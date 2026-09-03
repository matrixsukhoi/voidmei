use super::*;

// ---- get_string: 游戏 8111 JSON (`"key": value` 冒号后带空格) 真实形态 ----

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
    // indexOf 取第一次出现 (与 getStringBuilder 的 lastIndexOf 相对)
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

// ---- get_string_builder ----

#[test]
fn get_string_builder_writes_at_offset() {
    let r = "{\"a\": 1, \"speed\": 7.5}";
    let mut buf = [0u8; 8];
    get_string_builder(r, "speed", &mut buf, 2);
    assert_eq!(&buf[..2], &[0, 0]); // 偏移前不动
    assert_eq!(&buf[2..5], b"7.5"); // 写入子串
    assert_eq!(&buf[5..], &[0, 0, 0]); // 尾部不动
}

#[test]
fn get_string_builder_last_occurrence() {
    // lastIndexOf 定位最后一次出现
    let r = "{\"speed\": 1, \"speed\": 2}";
    let mut buf = [0u8; 4];
    get_string_builder(r, "speed", &mut buf, 0);
    assert_eq!(&buf[..1], b"2");
    assert_eq!(&buf[1..], &[0, 0, 0]);
}

#[test]
fn get_string_builder_not_found_untouched() {
    let mut buf = [7u8; 4];
    get_string_builder("{\"a\": 1}", "speed", &mut buf, 0);
    assert_eq!(buf, [7, 7, 7, 7]);
}

#[test]
fn get_string_builder_cjk_bytes() {
    // PORT: Java char[] 按 UTF-16 码元写 (歼 = 1); UTF-8 缓冲下歼占 3 字节
    let r = "{\"type\": \"歼\"}";
    let mut buf = [0u8; 8];
    get_string_builder(r, "type", &mut buf, 0);
    assert_eq!(&buf[..5], "\"歼\"".as_bytes());
    assert_eq!(&buf[5..], &[0, 0, 0]);
}

#[test]
#[should_panic]
fn get_string_builder_buffer_overflow_panics_like_java() {
    // Java getChars 越界抛 IndexOutOfBoundsException ↔ panic
    let mut buf = [0u8; 2];
    get_string_builder("{\"speed\": 7.5}", "speed", &mut buf, 0);
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
fn get_data_c_variants() {
    // C 版 (CharSequence 形参) 行为与非 C 版一致;
    // getDataIntC 在 Java 源码里返回类型就是 double, 保真保留
    assert_eq!(get_data_float_c(Some("2.5")), 2.5);
    assert_eq!(get_data_float_c(None), -65535.0);
    assert_eq!(get_data_int_c(Some("42")), 42.0);
    assert_eq!(get_data_int_c(Some("42")) as i32, 42);
    assert_eq!(get_data_int_c(None), -65535.0);
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
fn get_data_float_c_bad_number_panics_like_java() {
    // C 版坏输入同样 panic (与 get_data_float 一致)
    get_data_float_c(Some("abc"));
}

#[test]
#[should_panic]
fn get_data_int_leading_space_panics_like_java() {
    // Java 8 oracle 实测 Integer.parseInt(" 5") 抛 — parseInt 不 trim, 保真不加
    get_data_int(Some(" 5"));
}

#[test]
#[should_panic]
fn get_string_builder_empty_needle_panics_like_java() {
    // srcBegin > srcEnd 抛 StringIndexOutOfBoundsException (oracle 实测) ↔ panic
    let mut buf = [0u8; 8];
    get_string_builder("ab", "", &mut buf, 0);
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
