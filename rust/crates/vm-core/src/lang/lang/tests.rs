use super::*;

#[test]
fn update_language_hit() {
    let cfg = Config::new("./lang/cur.properties");
    assert_eq!(Lang::update_language(&cfg, "appName", "dft"), "VoidMei");
    assert_eq!(Lang::update_language(&cfg, "eTitle", "dft"), "发动机面板");
}

#[test]
fn update_language_missing_key_returns_empty_not_default() {
    // Java: getValue 缺失 → "" → dft 被覆写为 "" 返回 —— 默认值是死参数 (原行为)
    let cfg = Config::new("./lang/cur.properties");
    assert_eq!(Lang::update_language(&cfg, "httpPort", "12345"), "");
    assert_eq!(Lang::update_language(&cfg, "__no_such_key__", "fallback"), "");
}

#[test]
fn update_language_present_but_empty_value() {
    // cur.properties: mP1VoiceWarningBlank= (键存在, 值为空)
    let cfg = Config::new("./lang/cur.properties");
    assert_eq!(Lang::update_language(&cfg, "mP1VoiceWarningBlank", "x"), "");
}

#[test]
fn init_lang_common_values() {
    let lang = Lang::init_lang();
    assert_eq!(lang.app_name, "VoidMei");
    assert_eq!(lang.close, "Close");
    assert_eq!(lang.http_ip, "127.0.0.1");
    assert_eq!(lang.e_title, "发动机面板");
    assert_eq!(lang.s_enter, "等待飞机启动..");
    assert_eq!(lang.m_import_file_selected, "已选择: %s");
    assert_eq!(lang.l7, "温\u{3000}度/\u{2103},");
    assert_eq!(lang.f_sep, "ＳＥＰ");
    assert_eq!(lang.c_openpad, "'您已加入游戏，面板将在' s '秒内打开'");
    assert_eq!(
        lang.fm_missing_toast,
        "没有对应的 FM 数据文件\n可能是新出的飞机, FM 数据尚未更新"
    );
    assert_eq!(lang.b_flap_restrict, "襟翼限速(km/h)%d: %.0f%% / %.0f\n");
    assert_eq!(lang.http_header, "\n");
    assert_eq!(
        lang.m_reset_confirm_content,
        "确定要重置所有配置项吗？\\n此操作不可撤销。"
    );
}

#[test]
fn init_lang_missing_or_case_mismatched_keys_stay_empty() {
    let lang = Lang::init_lang();
    assert_eq!(lang.http_port, ""); // 键在 cur.properties 中不存在
    // Java 查 "mP1StatusBar"(大写 S), 文件里是 "mP1statusBar" — Properties 键大小写敏感
    assert_eq!(lang.m_p1_status_bar, "");
    assert_eq!(lang.m_p1_status_bar_blank, "");
    // Java 查 "mP3MonoFontBlank", 文件里是 "mP3MonoBlank"
    assert_eq!(lang.m_p3_mono_font_blank, "");
    // 键存在但值为空
    assert_eq!(lang.m_p1_voice_warning_blank, "");
    assert_eq!(lang.e_magneto, "");
}

#[test]
fn init_lang_whitespace_semantics() {
    let lang = Lang::init_lang();
    assert_eq!(lang.m_display_overlay, "显示Overlay: "); // 尾随空格保留
    assert_eq!(lang.f_a_roll1, "速度  "); // 两个尾随空格 (oracle)
    // 值全为 ASCII 空白 → Properties 前导空白全跳过 → 空串 (oracle)
    assert_eq!(lang.m_p4attitude_indicator_panel_blank, "");
    // 分隔符后前导空格被跳过: appTooltips 值无前导空格 (oracle)
    assert_eq!(lang.app_tooltips, "WT8111端口信息分析、显示、记录工具");
    assert!(lang.aboutcontent.ends_with("\n\r"));
}

#[test]
fn init_lang_never_assigned_fields_stay_none() {
    // Java: 三字段声明后从未在 initLang() 赋值, 保持 null;
    // cEnginedmg 被 ui.util.NotificationService.showNotification 读取
    let lang = Lang::init_lang();
    assert!(lang.c_enginedmg.is_none());
    assert!(lang.c_warn1min.is_none());
    assert!(lang.c_eng_bomb.is_none());
}

#[test]
fn init_lang_alignment_padding_survives() {
    // 对齐占位串 (全角空格) 逐字节保真
    let lang = Lang::init_lang();
    let v = lang.m_p1_temp_notification_blank;
    assert_eq!(v.chars().count(), 36);
    assert!(v.chars().all(|c| c == '\u{3000}'));
    // mP4PanelFont = 面板显示字体 + 44 个 ASCII 尾随空格 (oracle len=50)
    let v = lang.m_p4_panel_font;
    assert_eq!(v.chars().count(), 50);
    assert!(v.starts_with("面板显示字体"));
    assert!(v.chars().skip(6).all(|c| c == ' '));
}
