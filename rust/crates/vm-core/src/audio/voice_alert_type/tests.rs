use super::*;

/// 对应 Java testAlertTypeKeys
#[test]
fn test_alert_type_keys() {
    // 验证关键的告警类型存在
    assert_eq!(VoiceAlertType::AoaCrit.get_key(), "aoaCrit", "AOA_CRIT key");
    assert_eq!(
        VoiceAlertType::WarnGear.get_key(),
        "warn_gear",
        "WARN_GEAR key"
    );
    assert_eq!(
        VoiceAlertType::FailEngine.get_key(),
        "fail_engine",
        "FAIL_ENGINE key"
    );
    assert_eq!(VoiceAlertType::Start1.get_key(), "start1", "START1 key");

    // 验证 fromKey 查找
    assert_eq!(
        VoiceAlertType::from_key(Some("aoaCrit")),
        Some(VoiceAlertType::AoaCrit),
        "fromKey aoaCrit"
    );
    assert_eq!(
        VoiceAlertType::from_key(Some("nonexistent")),
        None,
        "fromKey nonexistent"
    );
}

/// 对应 Java testAlertTypeCooldowns
#[test]
fn test_alert_type_cooldowns() {
    assert_eq!(
        VoiceAlertType::AoaCrit.get_cooldown_seconds(),
        1,
        "AOA_CRIT cooldown"
    );
    assert_eq!(
        VoiceAlertType::AoaHigh.get_cooldown_seconds(),
        8,
        "AOA_HIGH cooldown"
    );
    assert_eq!(
        VoiceAlertType::WarnEngineoverheat.get_cooldown_seconds(),
        60,
        "WARN_ENGINEOVERHEAT cooldown"
    );
    assert_eq!(
        VoiceAlertType::WarnCompressor.get_cooldown_seconds(),
        0,
        "WARN_COMPRESSOR cooldown"
    );
}
