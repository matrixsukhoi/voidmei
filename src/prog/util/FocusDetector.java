package prog.util;

import java.io.BufferedReader;
import java.io.InputStreamReader;

/**
 * 跨平台前台窗口焦点检测器。
 * 纯工具类，不维护状态，不创建线程。
 *
 * 支持Windows、Linux和macOS平台，通过检测前台窗口进程名判断War Thunder是否获得焦点。
 */
public class FocusDetector {

    private static final String OS = System.getProperty("os.name", "").toLowerCase();

    /**
     * 检测War Thunder是否为当前前台窗口。
     * 检测失败时返回true（安全降级，不误隐藏overlay）。
     *
     * @return true 如果War Thunder为前台窗口或检测失败
     */
    public static boolean isWarThunderFocused() {
        try {
            if (OS.contains("win")) {
                return detectWindows();
            } else if (OS.contains("linux")) {
                return detectLinux();
            } else if (OS.contains("mac")) {
                return detectMacOS();
            }
        } catch (Exception e) {
            Logger.debug("FocusDetector", "检测失败: " + e.getMessage());
        }
        // 默认返回true（安全降级）
        return true;
    }

    /**
     * Windows平台检测：通过PowerShell调用user32.dll获取前台窗口进程名。
     */
    private static boolean detectWindows() throws Exception {
        // 使用PowerShell调用Win32 API获取前台窗口进程名
        String[] cmd = {"powershell", "-NoProfile", "-Command",
            "$w=Add-Type -M '[DllImport(\"user32.dll\")] public static extern IntPtr GetForegroundWindow();' -N W -P; " +
            "$p=Add-Type -M '[DllImport(\"user32.dll\")] public static extern int GetWindowThreadProcessId(IntPtr,out int);' -N P -P; " +
            "$id=0; $null=[P]::GetWindowThreadProcessId([W]::GetForegroundWindow(),[ref]$id); (gps -Id $id).ProcessName"};
        Process process = Runtime.getRuntime().exec(cmd);
        String result = readFirstLine(process);
        // War Thunder进程名为"aces"
        return result != null && result.toLowerCase().contains("aces");
    }

    /**
     * Linux平台检测：通过xdotool获取活动窗口标题。
     */
    private static boolean detectLinux() throws Exception {
        Process process = Runtime.getRuntime().exec(
            new String[]{"bash", "-c", "xdotool getactivewindow getwindowname 2>/dev/null"});
        String result = readFirstLine(process);
        return result != null && result.toLowerCase().contains("war thunder");
    }

    /**
     * macOS平台检测：通过AppleScript获取前台应用名称。
     */
    private static boolean detectMacOS() throws Exception {
        Process process = Runtime.getRuntime().exec(new String[]{"osascript", "-e",
            "tell application \"System Events\" to get name of first process whose frontmost is true"});
        String result = readFirstLine(process);
        return result != null &&
               (result.toLowerCase().contains("war thunder") || result.toLowerCase().contains("aces"));
    }

    /**
     * 读取进程输出的第一行。
     */
    private static String readFirstLine(Process process) throws Exception {
        try (BufferedReader reader = new BufferedReader(
                new InputStreamReader(process.getInputStream()))) {
            return reader.readLine();
        }
    }
}
