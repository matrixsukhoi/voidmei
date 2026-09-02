use super::*;

/// Java 8 oracle 对拍 (§5.1): toString 与 name() 同形 (日志拼接依赖)。
#[test]
fn test_display_matches_java_names() {
    let expected = [
        (ControllerState::Init, "INIT"),
        (ControllerState::Connected, "CONNECTED"),
        (ControllerState::InGame, "IN_GAME"),
        (ControllerState::Preview, "PREVIEW"),
    ];
    for (s, name) in expected {
        assert_eq!(s.to_string(), name);
    }
}
