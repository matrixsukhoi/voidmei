//! 对应 Java: `src/prog/event/UIStateEvents.java`
//! Event type constants for UIStateBus.
//! Centralizes all UI State event identifiers for easy discovery and
//! refactoring.
//!
//! PORT: Java final 类 + private 构造器 (纯常量容器, 禁实例化) → Rust 模块 pub const
//! (physics_constants.rs 先例); 无 struct 天然无法实例化, 原语义由模块边界保真。

/// Published when the FM Print switch State changes.
/// Payload: Boolean (new State)
pub const FM_PRINT_SWITCH_CHANGED: &str = "fmPrintSwitchChanged";

/// Published when any configuration value is updated in memory.
/// Payload: String (the config key that changed)
pub const CONFIG_CHANGED: &str = "configChanged";

// 旧 FM_DATA_LOADED 事件（payload=String 机型名）已退役（P5）——
// FM 状态变化统一订阅 FM_CHANGED

/// P2 重构新增：FMManager 管理的当前 FM 句柄发生变化（READY/MISSING/CORRUPT 落定，
/// 或负缓存命中直接落 MISSING）。
/// Payload: prog.fm.FMHandle（不可变句柄）。
/// 发布线程 = FM-Loader 后台线程（同步派发），订阅方碰 Swing 必须自行 invokeLater。
pub const FM_CHANGED: &str = "fmChanged";

/// Published when the Main Form is fully initialized and visible.
/// Payload: None
pub const UI_READY: &str = "uiReady";

/// Payload for CONFIG_CHANGED event when a UI request to reset all configs is
/// made.
pub const ACTION_RESET_REQUEST: &str = "RESET_REQUEST";

/// Payload for CONFIG_CHANGED event when a global reset operation has finished.
pub const ACTION_RESET_COMPLETED: &str = "RESET_COMPLETED";

/// Published when the list of available voice packs has changed.
/// Payload: None
pub const VOICE_PACKS_REFRESH: &str = "voicePacksRefresh";

/// Published when the FM overlay toggle hotkey is pressed.
/// Payload: Integer (key code)
pub const FM_OVERLAY_TOGGLE: &str = "fmOverlayToggle";

// Add more event types as needed

#[cfg(test)]
mod tests {
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
}
