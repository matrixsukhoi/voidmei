package prog.util;

/**
 * 跨平台前台窗口焦点检测器。
 * 纯工具类，不维护状态，不创建线程。
 *
 * <p>仅支持 Windows 平台（使用 JNA 直接调用 Win32 API）。
 * 非 Windows 平台返回 true（安全降级，不隐藏 overlay）。
 *
 * <p>性能提升：从 PowerShell 方案的 300-400ms 降至 JNA 方案的 3-5ms。
 *
 * @see WindowsFocusDetector Windows 平台 JNA 实现
 */
public class FocusDetector {

    private static final String OS = System.getProperty("os.name", "").toLowerCase();

    /**
     * 检测 War Thunder 是否为当前前台窗口。
     *
     * <p>安全降级原则：检测失败或非 Windows 平台时返回 true，不误隐藏 overlay。
     *
     * @return true 如果 War Thunder 为前台窗口，或非 Windows 平台，或检测失败
     */
    public static boolean isWarThunderFocused() {
        // 仅支持 Windows 平台
        // Linux/macOS 检测依赖外部工具 (xdotool/AppleScript)，可靠性较低
        // 因此非 Windows 平台直接返回 true（安全降级，不隐藏 overlay）
        if (!OS.contains("win")) {
            return true;
        }

        // Windows 平台使用 JNA 直接调用 Win32 API
        // 性能：3-5ms（相比 PowerShell 的 300-400ms 提升 100 倍）
        return WindowsFocusDetector.isWarThunderFocused();
    }
}
