use super::*;

// 8 个事件常量值与 Java 字面量逐一相等 (字符串键是总线分发契约, 不得漂移)
#[test]
fn test_constant_values() {
    assert_eq!(FM_PRINT_SWITCH_CHANGED, "fmPrintSwitchChanged");
    assert_eq!(CONFIG_CHANGED, "configChanged");
    assert_eq!(FM_CHANGED, "fmChanged");
    assert_eq!(UI_READY, "uiReady");
    assert_eq!(ACTION_RESET_REQUEST, "RESET_REQUEST");
    assert_eq!(ACTION_RESET_COMPLETED, "RESET_COMPLETED");
    assert_eq!(VOICE_PACKS_REFRESH, "voicePacksRefresh");
    assert_eq!(FM_OVERLAY_TOGGLE, "fmOverlayToggle");
}

// 事件类型字符串互不相同 (UIStateBus 按字符串键订阅, 撞键即串台)
#[test]
fn test_constants_unique() {
    let all = [
        FM_PRINT_SWITCH_CHANGED,
        CONFIG_CHANGED,
        FM_CHANGED,
        UI_READY,
        VOICE_PACKS_REFRESH,
        FM_OVERLAY_TOGGLE,
    ];
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            assert_ne!(all[i], all[j], "event type collision: {}", all[i]);
        }
    }
    // ACTION_RESET_* 是 CONFIG_CHANGED 的 payload 值而非事件键, 不与上表判重,
    // 但二者之间也不得相同 (Java 中分别用作 "请求/完成" 两个语义标记)
    assert_ne!(ACTION_RESET_REQUEST, ACTION_RESET_COMPLETED);
}
