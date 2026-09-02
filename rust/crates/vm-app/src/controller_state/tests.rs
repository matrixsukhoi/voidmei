use super::*;

/// Java 8 oracle 对拍 (§5.1): 声明序 legacyValue = 1/2/3/4,
/// toString 与 name() 同形 (日志拼接依赖)。
#[test]
fn test_display_and_legacy_values_match_java() {
    let expected = [
        ("INIT", 1),
        ("CONNECTED", 2),
        ("IN_GAME", 3),
        ("PREVIEW", 4),
    ];
    for (i, (name, legacy)) in expected.iter().enumerate() {
        let s = VALUES[i];
        assert_eq!(s.to_string(), *name);
        assert_eq!(s.get_legacy_value(), *legacy);
    }
}

/// Java 8 oracle 对拍: fromLegacyValue 命中 1~4 (声明序首个匹配)。
#[test]
fn test_from_legacy_value_round_trip() {
    assert_eq!(ControllerState::from_legacy_value(1), ControllerState::Init);
    assert_eq!(ControllerState::from_legacy_value(2), ControllerState::Connected);
    assert_eq!(ControllerState::from_legacy_value(3), ControllerState::InGame);
    assert_eq!(ControllerState::from_legacy_value(4), ControllerState::Preview);
}

/// Java 8 oracle 对拍: 未匹配值 (0/5/-1/INT_MIN/INT_MAX) 兜底 INIT。
#[test]
fn test_from_legacy_value_unknown_defaults_to_init() {
    for v in [0, 5, -1, i32::MIN, i32::MAX] {
        assert_eq!(ControllerState::from_legacy_value(v), ControllerState::Init);
    }
}
