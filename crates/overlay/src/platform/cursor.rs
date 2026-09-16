//! 全局鼠标查询 (拖放组件库面): webview 内按下组件卡发起拖拽 → 渲染线程
//! 编辑泵轮询屏幕坐标/左键状态 → 插入反馈装饰 → 释放结算落组件。
//! 编辑态专用 (~10ms 泵节拍), 非常驻钩子; 非 Windows 为退化 stub。

/// 屏幕物理坐标 (查询失败 = (0,0))
pub fn cursor_pos() -> (i32, i32) {
    #[cfg(target_os = "windows")]
    {
        #![allow(non_snake_case)]
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut pt = POINT::default();
        // SAFETY: 栈上 POINT 出参, 无句柄参与
        unsafe {
            if GetCursorPos(&mut pt).is_ok() {
                return (pt.x, pt.y);
            }
        }
        (0, 0)
    }
    #[cfg(not(target_os = "windows"))]
    {
        (0, 0)
    }
}

/// 左键当前是否按下 (查询失败 = false; 拖放释放结算的信号源)
pub fn left_down() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
        // SAFETY: 纯状态查询无副作用; 最高位 = 按下
        (unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } as u16 & 0x8000) != 0
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}
