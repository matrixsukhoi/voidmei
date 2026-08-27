use super::*;

/// set/colors/reset 往返 + 默认值 = Java 静态初始值
#[test]
fn set_colors_and_reset_default() {
    assert_eq!(colors(), GlobalColors::JAVA_DEFAULT);
    let custom = GlobalColors {
        num: [255, 255, 255, 255],
        ..GlobalColors::JAVA_DEFAULT
    };
    set(custom);
    assert_eq!(colors(), custom);
    reset_default();
    assert_eq!(colors(), GlobalColors::JAVA_DEFAULT);
}

/// AA 仓往返 (set_aa/aa/reset)
#[test]
fn set_aa_and_reset() {
    assert!(aa(), "默认 true (旧渲染路径取值)");
    set_aa(false);
    assert!(!aa());
    reset_default();
    assert!(aa());
}
