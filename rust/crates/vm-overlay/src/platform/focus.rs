//! Windows 前台焦点检测 — Java `src/prog/util/WindowsFocusDetector.java`,
//! 实现 vm-core `focus_monitor::FocusDetector` trait (Windows 腿落地)。
//! 跨平台分派腿 (FocusDetector.java 的 os.name 判定, A 类) 不在本文件,
//! 见 vm-core focus_monitor.rs trait 注 (FocusDetector trait 归属维持在那侧)。
//! 波16 自 extras.rs 按域拆出 (原三合一文件备案的拆分落地)。

// =====================================================================
// Windows 平台腿: 前台焦点检测
// =====================================================================
#[cfg(target_os = "windows")]
mod win {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    use vm_core::platform::focus_monitor::FocusDetector;

    /// 对应 Java: `src/prog/util/WindowsFocusDetector.java` (C 类/P4,
    /// windows crate 绑定)。Windows 平台专用焦点检测器。
    ///
    /// 实现合同 (锁内回调, focus_monitor.rs): 纯 Win32 调用 ~3-5ms,
    /// 不等待其他锁/线程, 满足非阻塞要求。
    pub struct WindowsFocusDetector;

    impl FocusDetector for WindowsFocusDetector {
        /// 检测 War Thunder 是否为前台窗口。
        /// 安全降级原则：检测失败或非 Windows 平台时返回 true，不误隐藏 overlay。
        fn is_war_thunder_focused(&self) -> bool {
            is_war_thunder_focused()
        }
    }

    /// Java: `WindowsFocusDetector.isWarThunderFocused()` (静态方法本体)
    pub fn is_war_thunder_focused() -> bool {
        // Step 1: 获取前台窗口句柄 (~0.1ms)
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            // 无前台窗口（极罕见情况），安全降级 — Java JNA 返回 null 的分支
            return true;
        }

        // Step 2: 获取窗口所属进程 ID (~0.5ms)
        // 调用失败 → pid 保持 0, 由下方系统进程分支统一降级
        let mut pid: u32 = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

        // PID 0 或 4 是系统进程（Idle/System），表示检测时机不对
        // 这是 PowerShell 方案返回 "Idle" 的根本原因
        if pid == 0 || pid == 4 {
            return true;
        }

        // Step 3: 获取进程可执行文件名 (~2-4ms)
        match process_image_name(pid) {
            // 无法获取进程名（权限不足等），安全降级
            None => true,
            // War Thunder 的进程名为 "aces.exe" (LIFETIMES: Windows 用 aces)
            // PORT: Java equalsIgnoreCase 走 Unicode 单字符大小写映射 (如 'ſ'
            // U+017F 判等于 's'), eq_ignore_ascii_case 仅 ASCII — Windows 进程
            // 镜像名域为 ASCII, 该差异不可达, 取 ASCII 版避免全串 Unicode 折开
            Some(name) => "aces.exe".eq_ignore_ascii_case(&name),
        }
        // Java 外层 try/catch(Exception) 的兜底 true 由各步失败分支承担
        // (空句柄 / pid 0 / OpenProcess、QueryFullProcessImageName 失败)
    }

    /// Java: `getProcessName(int pid)` — 进程可执行文件名, 失败 None (Java null)。
    pub fn process_image_name(pid: u32) -> Option<String> {
        // 以最小权限打开进程 (PROCESS_QUERY_LIMITED_INFORMATION = 0x1000,
        // Vista+ 获取进程信息所需的最小权限)
        let h_process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        // finally: 必须关闭句柄，避免资源泄漏 — 取名与关句柄分离, 关闭必经
        let result = query_image_name(h_process);
        unsafe {
            let _ = CloseHandle(h_process);
        }
        result
    }

    /// Java: `QueryFullProcessImageName` 调用段 (Vista+, 比 GetModuleFileNameEx
    /// 可靠, 不需要 PROCESS_VM_READ 权限; 参数 0 = Win32 路径格式而非 NT 路径)。
    fn query_image_name(h_process: HANDLE) -> Option<String> {
        // 准备缓冲区接收路径 (MAX_PATH = 260)
        let mut buffer = [0u16; 260];
        let mut size = buffer.len() as u32;
        let ok = unsafe {
            QueryFullProcessImageNameW(
                h_process,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut size,
            )
        };
        if ok.is_err() {
            return None;
        }
        // PORT: Java String(char[]) 对未配对代理原样保留 (无 U+FFFD 替换),
        // from_utf16_lossy 则替换为 U+FFFD — 存在串级分歧, 但两侧结果都不可能
        // 等于 "aces.exe", 比对布尔不变 (真实进程路径为合法 UTF-16, 分歧不可达)
        let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
        Some(file_name_after_last_backslash(&full_path).to_string())
    }

    /// Java: `fullPath.lastIndexOf('\\')` + substring — 提取路径尾文件名。
    /// PORT (§2.1): '\\' 是 ASCII 字节, rfind 的字节索引与 Java UTF-16 索引
    /// 在 ASCII 分隔符处等价, 切点必落在字符边界上。
    pub fn file_name_after_last_backslash(path: &str) -> &str {
        match path.rfind('\\') {
            Some(i) => &path[i + 1..],
            // 无分隔符 → 整串即文件名 (Java substring 分支)
            None => path,
        }
    }
}

#[cfg(target_os = "windows")]
pub use win::WindowsFocusDetector;

// Tests — Win32 腿以真实 API 冒烟, 全部断言真实行为, 不做条件跳过。
#[cfg(test)]
mod tests;
