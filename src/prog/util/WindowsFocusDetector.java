package prog.util;

import com.sun.jna.platform.win32.Kernel32;
import com.sun.jna.platform.win32.User32;
import com.sun.jna.platform.win32.WinDef;
import com.sun.jna.platform.win32.WinNT;
import com.sun.jna.ptr.IntByReference;

/**
 * Windows 平台专用焦点检测器，使用 JNA 直接调用 Win32 API。
 *
 * <p>性能对比：
 * <ul>
 *   <li>PowerShell 方案: 300-400ms (进程启动 + Add-Type 编译 + 调用)</li>
 *   <li>JNA 方案: 3-5ms (直接 native 调用)</li>
 * </ul>
 *
 * <p>这是业界标准方案，OBS、Discord Overlay 等项目均采用类似实现。
 *
 * @see FocusDetector 跨平台焦点检测入口
 */
public class WindowsFocusDetector {

    /**
     * PROCESS_QUERY_LIMITED_INFORMATION 权限 (0x1000)
     * 这是获取进程信息所需的最小权限，适用于 Vista 及以上版本
     */
    private static final int PROCESS_QUERY_LIMITED_INFORMATION = 0x1000;

    /**
     * 检测 War Thunder 是否为前台窗口。
     *
     * <p>检测流程：
     * <ol>
     *   <li>GetForegroundWindow() 获取前台窗口句柄 (~0.1ms)</li>
     *   <li>GetWindowThreadProcessId() 获取进程 ID (~0.5ms)</li>
     *   <li>OpenProcess() + QueryFullProcessImageName() 获取进程名 (~2-4ms)</li>
     * </ol>
     *
     * <p>安全降级原则：任何异常情况返回 true，避免误隐藏 overlay。
     *
     * @return true 如果 War Thunder 获得焦点，或检测失败时安全降级
     */
    public static boolean isWarThunderFocused() {
        try {
            // Step 1: 获取前台窗口句柄 (~0.1ms)
            WinDef.HWND hwnd = User32.INSTANCE.GetForegroundWindow();
            if (hwnd == null) {
                // 无前台窗口（极罕见情况），安全降级
                return true;
            }

            // Step 2: 获取窗口所属进程 ID (~0.5ms)
            IntByReference pidRef = new IntByReference();
            User32.INSTANCE.GetWindowThreadProcessId(hwnd, pidRef);
            int pid = pidRef.getValue();

            // PID 0 或 4 是系统进程（Idle/System），表示检测时机不对
            // 这是 PowerShell 方案返回 "Idle" 的根本原因
            if (pid == 0 || pid == 4) {
                return true;
            }

            // Step 3: 获取进程可执行文件名 (~2-4ms)
            String processName = getProcessName(pid);
            if (processName == null) {
                // 无法获取进程名（权限不足等），安全降级
                return true;
            }

            // War Thunder 的进程名为 "aces.exe"
            return "aces.exe".equalsIgnoreCase(processName);

        } catch (Exception e) {
            // 任何异常都安全降级，保持 overlay 可见
            return true;
        }
    }

    /**
     * 根据进程 ID 获取进程可执行文件名。
     *
     * <p>使用 QueryFullProcessImageName API (Vista+)，比传统的
     * GetModuleFileNameEx 更可靠，不需要 PROCESS_VM_READ 权限。
     *
     * @param pid 进程 ID
     * @return 可执行文件名（如 "aces.exe"），失败返回 null
     */
    private static String getProcessName(int pid) {
        // 以最小权限打开进程
        WinNT.HANDLE hProcess = Kernel32.INSTANCE.OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION, false, pid);

        if (hProcess == null) {
            // 无法打开进程（可能是系统保护进程）
            return null;
        }

        try {
            // 准备缓冲区接收路径 (MAX_PATH = 260)
            char[] buffer = new char[260];
            IntByReference size = new IntByReference(buffer.length);

            // QueryFullProcessImageName 返回完整路径
            // 参数 0 表示返回 Win32 路径格式（而非 NT 路径）
            if (Kernel32.INSTANCE.QueryFullProcessImageName(hProcess, 0, buffer, size)) {
                String fullPath = new String(buffer, 0, size.getValue());
                // 提取文件名部分
                int lastSlash = fullPath.lastIndexOf('\\');
                return lastSlash >= 0 ? fullPath.substring(lastSlash + 1) : fullPath;
            }
        } finally {
            // 必须关闭句柄，避免资源泄漏
            Kernel32.INSTANCE.CloseHandle(hProcess);
        }
        return null;
    }
}
