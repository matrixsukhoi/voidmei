// 焦点检测 Win32 腿以真实 API 冒烟, 全部断言真实行为, 不做条件跳过。
#[cfg(target_os = "windows")]
mod win_tests {
    use super::super::win::{
        file_name_after_last_backslash, is_war_thunder_focused, process_image_name,
        WindowsFocusDetector,
    };
    use vm_core::platform::focus_monitor::FocusDetector;
    use windows::Win32::System::Threading::GetCurrentProcessId;

    #[test]
    fn focus_process_name_of_current_process() {
        // OpenProcess + QueryFullProcessImageNameW 真实链路: 本测试进程
        let pid = unsafe { GetCurrentProcessId() };
        let name = process_image_name(pid).expect("打开自身进程不应失败");
        assert!(
            name.to_ascii_lowercase().ends_with(".exe"),
            "Windows 进程镜像名应以 .exe 结尾, got {}",
            name
        );
    }

    #[test]
    fn focus_filename_extraction_and_aces_compare() {
        assert_eq!(
            file_name_after_last_backslash(r"C:\Games\War Thunder\aces.exe"),
            "aces.exe"
        );
        assert_eq!(
            file_name_after_last_backslash("aces.exe"),
            "aces.exe",
            "无分隔符 → 整串"
        );
        // Java equalsIgnoreCase 比对语义
        assert!("aces.exe".eq_ignore_ascii_case("ACES.EXE"));
        assert!(!"aces.exe".eq_ignore_ascii_case("aces2.exe"));
    }

    #[test]
    fn focus_smoke_safe_degradation() {
        // 平台合同: 任意前台状态都不得 panic, 恒返回 bool (安全降级)
        let _ = is_war_thunder_focused();
        let det = WindowsFocusDetector;
        let _ = det.is_war_thunder_focused();
    }
}
