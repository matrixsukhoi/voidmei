use super::*;

#[test]
fn java_split_comma_matches_java_oracle() {
    // 历史基线:
    // "1,2,".split(",")=[1, 2]; "1,,2"→[1, , 2]; ",,,".length=0;
    // "".split(",")=[ ]; ",".length=0; " "→[ ]; "1".split(",")=[1]; "a,,b,"→[a,,b]
    assert_eq!(java_split_comma("1,2,"), vec!["1", "2"]);
    assert_eq!(java_split_comma("1,,2"), vec!["1", "", "2"]);
    assert!(java_split_comma(",,,").is_empty());
    assert_eq!(java_split_comma(""), vec![""]);
    assert!(java_split_comma(",").is_empty());
    assert_eq!(java_split_comma(" "), vec![" "]);
    assert_eq!(java_split_comma("1"), vec!["1"]);
    assert_eq!(java_split_comma("a,,b,"), vec!["a", "", "b"]);
}

#[test]
fn java_trim_strips_le_u0020() {
    assert_eq!(java_trim("  7.25  "), "7.25");
    assert_eq!(java_trim("\t\n\u{0b}\r\u{c}4"), "4");
    assert_eq!(java_trim("千米5"), "千米5"); // 多字节不受影响
    assert_eq!(java_trim(""), "");
    assert_eq!(java_trim("   "), "");
}
