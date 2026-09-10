use super::*;

/// 21 个常量逐项与 UIConstants.java 数值核对。
/// 字面量带 i32/i64 后缀: 若类型映射写错 (int→i64 / long→i32) 编译期即报错,
/// 对齐 Java 的 int/long 区分 (ATTITUDE_REFRESH_MS 等 5 个 long 常量)。
#[test]
fn constants_match_java_values() {
    // ===== DPI 缩放基准 =====
    assert_eq!(BASE_SCREEN_HEIGHT, 1440i32);
    assert_eq!(BASE_FONT_SIZE, 16i32);
    // ===== BaseOverlay 尺寸 =====
    assert_eq!(WIDTH_MULTIPLIER, 36i32);
    assert_eq!(HEIGHT_MULTIPLIER, 72i32);
    // ===== AttitudeOverlay =====
    assert_eq!(MAX_AOA, 30i32);
    assert_eq!(MAX_AOS, 15i32);
    assert_eq!(ATTITUDE_BASE_WIDTH, 100i32);
    assert_eq!(ATTITUDE_BASE_HEIGHT, 200i32);
    assert_eq!(ATTITUDE_REFRESH_MS, 40i64);
    // ===== EngineControlOverlay =====
    assert_eq!(ENGINE_BASE_FONT_SIZE, 24i32);
    assert_eq!(ENGINE_WIDTH_MULTIPLIER, 8i32);
    assert_eq!(ENGINE_SHADE_WIDTH, 10i32);
    assert_eq!(ENGINE_DEFAULT_REFRESH_MS, 100i64);
    // ===== 时间常量 =====
    assert_eq!(DELAY_SHORT_MS, 100i64);
    assert_eq!(DELAY_MEDIUM_MS, 500i64);
    assert_eq!(DELAY_LONG_MS, 1000i64);
    // ===== 颜色相关 =====
    assert_eq!(DEFAULT_ALPHA, 255i32);
    assert_eq!(SEMI_TRANSPARENT_ALPHA, 128i32);
    // ===== 边距和间距 =====
    assert_eq!(SPACING_SMALL, 5i32);
    assert_eq!(SPACING_MEDIUM, 10i32);
    assert_eq!(SPACING_LARGE, 20i32);
}
