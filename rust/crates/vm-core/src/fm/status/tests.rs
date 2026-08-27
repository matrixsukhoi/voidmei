use super::*;

/// Java 8 oracle 对拍 (§5.1): 枚举默认 toString()=name() 的六个常量名
/// (build/oracle/FMHandleOracle.java 在 OpenJDK 1.8.0_342 dump, 用完已删)
#[test]
fn display_matches_java_constant_names() {
    assert_eq!(FMStatus::Unresolved.to_string(), "UNRESOLVED");
    assert_eq!(FMStatus::Loading.to_string(), "LOADING");
    assert_eq!(FMStatus::Ready.to_string(), "READY");
    assert_eq!(FMStatus::Missing.to_string(), "MISSING");
    assert_eq!(FMStatus::Corrupt.to_string(), "CORRUPT");
    assert_eq!(FMStatus::NotAircraft.to_string(), "NOT_AIRCRAFT");
}
