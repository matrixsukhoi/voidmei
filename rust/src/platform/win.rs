//! Windows 平台实现: WS_POPUP + WS_EX_LAYERED + UpdateLayeredWindow (纯 CPU, 无 GPU 上下文)
//! 行为对齐 Java AWT 透明窗 (同为 ULW 路径); 穿透 = WS_EX_TRANSPARENT 切换 (Java 版无, 属增强)

#![allow(non_snake_case)]

use std::collections::VecDeque;
use std::sync::Mutex;

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, MonitorFromWindow,
    ReleaseDC, SelectObject, AC_SRC_ALPHA, BLENDFUNCTION, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, MONITOR_DEFAULTTONEAREST, MONITORINFO,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DispatchMessageW, GetCursorPos, GetWindowLongPtrW, PeekMessageW,
    RegisterClassW, SetWindowLongPtrW, SetWindowPos,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, GWL_EXSTYLE, HTCLIENT, HWND_BOTTOM,
    IDC_ARROW, LoadCursorW, MSG, PM_REMOVE, ULW_ALPHA, WM_CAPTURECHANGED,
    WINDOW_EX_STYLE, WNDCLASSW, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_NCHITTEST, WM_POINTERUP, WM_DESTROY, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};

use super::{OverlayEvent, WindowConfig};

/// WNDPROC 投递的事件队列 (POC 单窗口; 多窗口需 hwnd 分流)
static EVENT_QUEUE: Mutex<VecDeque<OverlayEvent>> = Mutex::new(VecDeque::new());

pub struct WinOverlay {
    hwnd: HWND,
    width: i32,
    height: i32,
    // DIB 资源 (present 用)
    memdc: HDC,
    dib: HBITMAP,
    old_obj: HGDIOBJ,
    dib_bits: *mut u8,
}

unsafe impl Send for WinOverlay {}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => {
            // 整窗客户区 (Java setFocusable(false) 等价效果: 不进系统移动/缩放)
            LRESULT(HTCLIENT as isize)
        }
        WM_LBUTTONDOWN | WM_POINTERUP => {
            // 捕获鼠标: 快速拖拽滑出窗口后仍能收到 MOVE/UP (否则拖拽中断)
            let _ = SetCapture(hwnd);
            let (x, y) = cursor_root_pos();
            push_event(OverlayEvent::MousePress { root_x: x, root_y: y });
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let _ = ReleaseCapture();
            push_event(OverlayEvent::MouseRelease);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let (x, y) = cursor_root_pos();
            let left_down = wparam.0 & 0x0001 != 0;
            push_event(OverlayEvent::MouseMove { root_x: x, root_y: y, left_down });
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            // 系统夺走捕获时结束拖拽 (防 drag 状态卡死)
            push_event(OverlayEvent::MouseRelease);
            LRESULT(0)
        }
        WM_DESTROY => {
            push_event(OverlayEvent::Close);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn push_event(ev: OverlayEvent) {
    if let Ok(mut q) = EVENT_QUEUE.lock() {
        // MouseMove 合并: 鼠标事件风暴时只保留最新位置, 防队列积压导致拖拽迟滞
        if matches!(ev, OverlayEvent::MouseMove { .. }) {
            if let Some(last) = q.back_mut() {
                if matches!(last, OverlayEvent::MouseMove { .. }) {
                    *last = ev;
                    return;
                }
            }
        }
        q.push_back(ev);
    }
}

fn cursor_root_pos() -> (i32, i32) {
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        (pt.x, pt.y)
    }
}

// DefWindowProcW 在上面的 use 里没导入, 补在函数级
use windows::Win32::UI::WindowsAndMessaging::DefWindowProcW;

fn set_dpi_awareness() {
    // Per-Monitor V2: 全 API 物理像素, 对齐 Java -Dsun.java2d.uiScale=1 的行为
    unsafe {
        let _ = SetProcessDpiAwarenessContext(windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub fn create(cfg: WindowConfig) -> Result<WinOverlay, String> {
    set_dpi_awareness();

    unsafe {
        let hinstance = GetModuleHandleW(None)
            .map_err(|e| format!("GetModuleHandleW: {}", e))?;
        let class_name = w!("VoidMeiOverlay");

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hIcon: Default::default(),
            // 默认箭头光标 (preview 可见; Java applyPreviewStyle 的 setCursor(null) 等价;
            // live 模式穿透, 光标不落本窗口)
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: Default::default(),
            lpszMenuName: Default::default(),
        };
        let atom = RegisterClassW(&wc);
        if atom == 0 {
            return Err("RegisterClassW 失败".into());
        }

        let mut ex_style: WINDOW_EX_STYLE =
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        if cfg.click_through {
            ex_style |= WS_EX_TRANSPARENT;
        }

        let hwnd = CreateWindowExW(
            ex_style,
            class_name,
            w!("VoidMei FlightInfo"),
            WS_POPUP | WS_VISIBLE,
            cfg.x,
            cfg.y,
            cfg.width,
            cfg.height,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .map_err(|e| format!("CreateWindowExW: {}", e))?;

        // DIB: 32bpp top-down (负 biHeight)
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: cfg.width,
                biHeight: -cfg.height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let hdc_screen = GetDC(None);
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let dib = CreateDIBSection(Some(hdc_screen), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
            .map_err(|e| format!("CreateDIBSection: {}", e))?;
        let memdc = CreateCompatibleDC(Some(hdc_screen));
        let old_obj = SelectObject(memdc, HGDIOBJ(dib.0));
        ReleaseDC(None, hdc_screen);

        Ok(WinOverlay {
            hwnd,
            width: cfg.width,
            height: cfg.height,
            memdc,
            dib,
            old_obj,
            dib_bits: bits as *mut u8,
        })
    }
}

impl WinOverlay {
    #[allow(dead_code)]
    fn ex_style(&self) -> WINDOW_EX_STYLE {
        unsafe { WINDOW_EX_STYLE(GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) as u32) }
    }
}

impl super::OverlayWindow for WinOverlay {
    fn present(&mut self, buf: &[u8]) -> Result<(), String> {
        let expect = (self.width * self.height * 4) as usize;
        if buf.len() != expect {
            return Err(format!("缓冲尺寸不符: {} != {}", buf.len(), expect));
        }
        unsafe {
            // buf 已是预乘 BGRA, 直接拷入 DIB
            std::ptr::copy_nonoverlapping(buf.as_ptr(), self.dib_bits, expect);
            let blend = BLENDFUNCTION {
                BlendOp: 0, // AC_SRC_OVER
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let size = SIZE {
                cx: self.width,
                cy: self.height,
            };
            let pt_src = POINT { x: 0, y: 0 };
            // pptDst = None: 保持当前位置
            let ok = windows::Win32::UI::WindowsAndMessaging::UpdateLayeredWindow(
                self.hwnd,
                None,
                None,
                Some(&size),
                Some(self.memdc),
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
            if ok.is_err() {
                return Err("UpdateLayeredWindow 失败".into());
            }
        }
        Ok(())
    }

    fn set_position(&mut self, x: i32, y: i32) {
        unsafe {
            let _ = SetWindowPos(self.hwnd, Some(HWND_BOTTOM), x, y, 0, 0,
                windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE);
        }
    }

    fn position(&self) -> (i32, i32) {
        unsafe {
            let mut rc = RECT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(self.hwnd, &mut rc);
            (rc.left, rc.top)
        }
    }

    fn set_click_through(&mut self, on: bool) {
        let mut style = self.ex_style();
        if on {
            style |= WS_EX_TRANSPARENT;
        } else {
            style &= !WS_EX_TRANSPARENT;
        }
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, style.0 as isize);
        }
    }

    fn poll_event(&mut self) -> Option<OverlayEvent> {
        unsafe {
            // 泵消息 → WNDPROC → 队列
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        EVENT_QUEUE.lock().ok().and_then(|mut q| q.pop_front())
    }

    fn screen_size(&self) -> (i32, i32) {
        unsafe {
            // 以窗口所在显示器为准 (多屏时位置归一化稳定)
            let mon = MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if windows::Win32::Graphics::Gdi::GetMonitorInfoW(mon, &mut mi).as_bool() {
                let r = mi.rcMonitor;
                return (r.right - r.left, r.bottom - r.top);
            }
            (
                windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                    windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN),
                windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                    windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN),
            )
        }
    }
}

impl Drop for WinOverlay {
    fn drop(&mut self) {
        unsafe {
            let _ = SelectObject(self.memdc, self.old_obj);
            let _ = DeleteDC(self.memdc);
            let _ = DeleteObject(HGDIOBJ(self.dib.0));
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.hwnd);
        }
    }
}
