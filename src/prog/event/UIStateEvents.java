package prog.event;

/**
 * Event type constants for UIStateBus.
 * Centralizes all UI State event identifiers for easy discovery and
 * refactoring.
 */
public final class UIStateEvents {

    private UIStateEvents() {
    }

    /**
     * Published when the FM Print switch State changes.
     * Payload: Boolean (new State)
     */
    public static final String FM_PRINT_SWITCH_CHANGED = "fmPrintSwitchChanged";

    /**
     * Published when any configuration value is updated in memory.
     * Payload: String (the config key that changed)
     */
    public static final String CONFIG_CHANGED = "configChanged";

    // 旧 FM_DATA_LOADED 事件（payload=String 机型名）已退役（P5）——
    // FM 状态变化统一订阅 FM_CHANGED

    /**
     * P2 重构新增：FMManager 管理的当前 FM 句柄发生变化（READY/MISSING/CORRUPT 落定，
     * 或负缓存命中直接落 MISSING）。
     * Payload: prog.fm.FMHandle（不可变句柄）。
     * 发布线程 = FM-Loader 后台线程（同步派发），订阅方碰 Swing 必须自行 invokeLater。
     */
    public static final String FM_CHANGED = "fmChanged";

    /**
     * Published when the Main Form is fully initialized and visible.
     * Payload: None
     */
    public static final String UI_READY = "uiReady";

    /**
     * Payload for CONFIG_CHANGED event when a UI request to reset all configs is
     * made.
     */
    public static final String ACTION_RESET_REQUEST = "RESET_REQUEST";

    /**
     * Payload for CONFIG_CHANGED event when a global reset operation has finished.
     */
    public static final String ACTION_RESET_COMPLETED = "RESET_COMPLETED";

    /**
     * Published when the list of available voice packs has changed.
     * Payload: None
     */
    public static final String VOICE_PACKS_REFRESH = "voicePacksRefresh";

    /**
     * Published when the FM overlay toggle hotkey is pressed.
     * Payload: Integer (key code)
     */
    public static final String FM_OVERLAY_TOGGLE = "fmOverlayToggle";

    // Add more event types as needed
}
