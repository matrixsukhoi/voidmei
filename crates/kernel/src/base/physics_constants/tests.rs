use super::*;

// 断言值 = Java 字面量原值; 同一十进制字面量在 Java/Rust 解析为
// 完全相同的 IEEE 754 f64 位模式, 故可安全用精确相等。
#[test]
fn test_gravitational_constants() {
    assert_eq!(G, 9.80);
    assert_eq!(g, 9.80);
    // Java `g = G` 别名语义: 两者恒等
    assert_eq!(g, G);
}

#[test]
fn test_isa_constants() {
    assert_eq!(PRESSURE_ALTITUDE_COEFF, 0.0000225577);
    assert_eq!(PRESSURE_ALTITUDE_EXP, 5.25588);
    assert_eq!(TEMP_LAPSE_RATE, 0.0065);
}

#[test]
fn test_sea_level_constants() {
    assert_eq!(SEA_LEVEL_PRESSURE, 101325.0);
    assert_eq!(SEA_LEVEL_DENSITY, 1.225);
    assert_eq!(R_SPECIFIC_AIR, 287.0500676);
    assert_eq!(KELVIN_OFFSET, 273.15);
}
