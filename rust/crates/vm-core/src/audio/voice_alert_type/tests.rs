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

    // 验证毫秒转换
    assert_eq!(
        VoiceAlertType::AoaCrit.get_cooldown_ms(),
        1000i64,
        "AOA_CRIT cooldown ms"
    );
    assert_eq!(
        VoiceAlertType::WarnEngineoverheat.get_cooldown_ms(),
        60000i64,
        "WARN_ENGINEOVERHEAT cooldown ms"
    );
}

/// 对应 Java testAlertTypeCount
#[test]
fn test_alert_type_count() {
    // 原硬编码列表 (从 VoiceGlobalRenderer 复制)
    let original_keys = [
        "aoaCrit",
        "aoaHigh",
        "warn_stall",
        "warn_gear",
        "warn_engineoverheat",
        "warn_lowfuel",
        "warn_altitude",
        "warn_terrain",
        "warn_flap",
        "warn_loadfactor",
        "rudderEff",
        "elevatorEff",
        "aileronEff",
        "warn_lowrpm",
        "warn_highrpm",
        "warn_ias",
        "warn_mach",
        "fail_engine",
        "warn_lowpressure",
        "fail_nofuel",
        "warn_highvario",
        "warn_brake",
        "warn_compressor",
        "start1",
    ];

    let new_keys = VoiceAlertType::get_all_keys();

    assert_eq!(new_keys.len(), original_keys.len(), "key count");

    // 验证所有原始 key 都存在
    for key in original_keys {
        assert!(new_keys.contains(&key), "contains key: {}", key);
    }

    // 验证 getAlertCount
    assert_eq!(VoiceAlertType::get_alert_count(), 24, "getAlertCount");
}
