use super::*;

/// 对应 Java testBasicParsing
#[test]
fn test_basic_parsing() {
    // Parse with enabled=true
    let config = VoicePackConfig::parse(Some("jarvis|true"));
    assert_eq!(config.pack_name, "jarvis", "parse jarvis|true packName");
    assert!(config.enabled, "parse jarvis|true enabled");

    // Parse with enabled=false
    let config = VoicePackConfig::parse(Some("jarvis|false"));
    assert_eq!(config.pack_name, "jarvis", "parse jarvis|false packName");
    assert!(!config.enabled, "parse jarvis|false enabled");

    // Parse without enabled (defaults to true)
    let config = VoicePackConfig::parse(Some("jarvis"));
    assert_eq!(config.pack_name, "jarvis", "parse jarvis packName");
    assert!(config.enabled, "parse jarvis enabled (default)");
}

/// 对应 Java testNullAndEmptyParsing
#[test]
fn test_null_and_empty_parsing() {
    // Parse null
    let config = VoicePackConfig::parse(None);
    assert_eq!(config.pack_name, "default", "parse null packName");
    assert!(config.enabled, "parse null enabled");

    // Parse empty string
    let config = VoicePackConfig::parse(Some(""));
    assert_eq!(config.pack_name, "default", "parse empty packName");
    assert!(config.enabled, "parse empty enabled");
}

/// 对应 Java testSerialization
#[test]
fn test_serialization() {
    let config = VoicePackConfig::new(Some("jarvis"), false);
    assert_eq!(config.to_config_string(), "jarvis|false", "toConfigString");

    let config = VoicePackConfig::new(Some("default"), true);
    assert_eq!(
        config.to_config_string(),
        "default|true",
        "toConfigString default|true"
    );
}

/// 对应 Java testWithMethods
#[test]
fn test_with_methods() {
    let original = VoicePackConfig::new(Some("jarvis"), true);

    // withEnabled
    let updated = original.with_enabled(false);
    assert_eq!(
        updated.pack_name, "jarvis",
        "withEnabled packName unchanged"
    );
    assert!(!updated.enabled, "withEnabled new value");
    assert!(original.enabled, "withEnabled original unchanged");

    // withPackName
    let updated = original.with_pack_name(Some("custom"));
    assert_eq!(updated.pack_name, "custom", "withPackName new value");
    assert_eq!(
        original.pack_name, "jarvis",
        "withPackName original unchanged"
    );
}

/// 对应 Java testPrefixMethods
#[test]
fn test_prefix_methods() {
    // stripVoicePrefix
    assert_eq!(
        VoicePackConfig::strip_voice_prefix(Some("voice_aoaCrit")).as_deref(),
        Some("aoaCrit"),
        "strip voice_aoaCrit"
    );
    assert_eq!(
        VoicePackConfig::strip_voice_prefix(Some("aoaCrit")).as_deref(),
        Some("aoaCrit"),
        "strip aoaCrit (no prefix)"
    );
    assert_eq!(
        VoicePackConfig::strip_voice_prefix(None),
        None,
        "strip null"
    );

    // withVoicePrefix
    assert_eq!(
        VoicePackConfig::with_voice_prefix(Some("aoaCrit")).as_deref(),
        Some("voice_aoaCrit"),
        "with aoaCrit"
    );
    assert_eq!(
        VoicePackConfig::with_voice_prefix(Some("voice_aoaCrit")).as_deref(),
        Some("voice_aoaCrit"),
        "with voice_aoaCrit (already has)"
    );
    assert_eq!(VoicePackConfig::with_voice_prefix(None), None, "with null");
}

/// 验证新解析逻辑与原代码完全一致
/// 原代码位于 VoiceWarning.audClip.reload() 第 90-116 行
#[test]
fn test_parsing_consistency_with_original() {
    let test_cases: [Option<&str>; 8] = [
        Some("jarvis|true"),
        Some("jarvis|false"),
        Some("jarvis"),
        Some("default|true"),
        Some("default|false"),
        Some("custom_pack|true"),
        Some(""),
        None,
    ];

    for val in test_cases {
        // 原逻辑 (从 VoiceWarning.audClip.reload() 复制)
        let mut old_pack_name = "default";
        let mut old_enabled = true;
        if let Some(v) = val {
            if !v.is_empty() {
                if v.contains('|') {
                    // Java split("\\|") 无 limit 会丢弃尾部空串 —
                    // 弹出尾部空串复刻该语义, 保证副本忠实于 Java 旧逻辑
                    let mut parts: Vec<&str> = v.split('|').collect();
                    while parts.last().copied() == Some("") {
                        parts.pop();
                    }
                    old_pack_name = parts[0];
                    if parts.len() > 1 {
                        old_enabled = parse_boolean(parts[1]);
                    }
                } else {
                    old_pack_name = v;
                }
            }
        }

        // 新逻辑
        let config = VoicePackConfig::parse(val);

        let test_name = format!(
            "consistency: {}",
            match val {
                None => "null".to_string(),
                Some(v) => format!("\"{}\"", v),
            }
        );
        assert_eq!(config.pack_name, old_pack_name, "{} packName", test_name);
        assert_eq!(config.enabled, old_enabled, "{} enabled", test_name);
    }
}
