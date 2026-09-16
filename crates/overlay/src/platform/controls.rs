//! 编辑 chrome 的 Win32 子控件基座 (Opaque 窗口专属): EDIT 文本框 + GDI 字体。
//! 自绘 IME 明确不做 (重造输入法轮子风险不可控) — 子控件 = 系统白给的
//! 中文输入/光标/选择/剪贴板。通知经 WM_COMMAND → OverlayEvent::Control 进既有事件队列。

#![allow(non_snake_case)]

use windows::core::{w, HSTRING};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, DeleteObject, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH,
    DEFAULT_QUALITY, FW_NORMAL, OUT_DEFAULT_PRECIS, HFONT, HGDIOBJ,
};
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetWindowTextLengthW, GetWindowTextW, MoveWindow, SendMessageW,
    SetWindowTextW, ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, ES_AUTOHSCROLL, HMENU,
    SW_HIDE, SW_SHOW, WM_SETFONT, WS_BORDER, WS_CHILD, WS_VISIBLE,
};

/// GDI 字体 (WM_SETFONT 注入子控件; Drop 归还系统)
pub struct UiFont {
    hfont: HFONT,
}

impl UiFont {
    /// 按字符高度建 "Microsoft YaHei UI" (系统自带, 与 overlay 自绘字体视觉同源)
    pub fn create(size: i32) -> Self {
        // SAFETY: 纯句柄创建无指针参数; hfont 归本实例独有 (Drop 时删除)
        let hfont = unsafe {
            CreateFontW(
                -size, // 负 height = 字符高度语义 (正值是含内部行距的 cell 高)
                0,
                0,
                0,
                FW_NORMAL.0 as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                DEFAULT_QUALITY,
                DEFAULT_PITCH.0 as u32,
                w!("Microsoft YaHei UI"),
            )
        };
        Self { hfont }
    }
}

impl Drop for UiFont {
    fn drop(&mut self) {
        // SAFETY: hfont 独有且未选入任何 DC; 删除失败仅返回 FALSE 不抛
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.hfont.0));
        }
    }
}

/// 系统 EDIT 文本框 (中文输入/光标/选择/剪贴板全由系统提供)。
/// 路由 id 经 new 的 HMENU 槽位登记 (WM_COMMAND 键), 不存字段 — 认领方自持
pub struct EditBox {
    hwnd: HWND,
}

impl EditBox {
    /// 建子控件 (parent = 父窗口 hwnd 值, 须为渲染线程自建 Opaque 窗口)
    pub fn new(parent: usize, id: u32, x: i32, y: i32, w: i32, h: i32) -> Result<Self, String> {
        if parent == 0 {
            return Err("EditBox 父窗口句柄为 0".into());
        }
        // SAFETY: "EDIT" 为系统注册类; HMENU 槽位对子窗口语义 = 控件 id
        // (WM_COMMAND 路由键), hinstance None 走进程模块
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("EDIT"),
                w!(""),
                WS_CHILD | WS_VISIBLE | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                x,
                y,
                w,
                h,
                Some(HWND(parent as *mut std::ffi::c_void)),
                Some(HMENU(id as usize as *mut std::ffi::c_void)),
                None,
                None,
            )
        }
        .map_err(|e| format!("CreateWindowExW EDIT: {e}"))?;
        Ok(Self { hwnd })
    }

    /// 移位/改尺寸 (同步重绘, 防 MoveWindow 后残帧)
    pub fn set_rect(&mut self, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
        // SAFETY: hwnd 由本实例拥有, Drop 前未销毁
        unsafe { MoveWindow(self.hwnd, x, y, w, h, true) }.map_err(|e| format!("MoveWindow: {e}"))
    }

    pub fn set_text(&mut self, text: &str) -> Result<(), String> {
        // SAFETY: hwnd 有效; HSTRING 值拷贝进系统, 生命周期不悬挂
        unsafe { SetWindowTextW(self.hwnd, &HSTRING::from(text)) }
            .map_err(|e| format!("SetWindowTextW: {e}"))
    }

    /// 取全文 (先量长再取; 返回值不含终止符)
    pub fn text(&self) -> String {
        // SAFETY: hwnd 有效; 缓冲按实长 + 终止符分配, GetWindowTextW 自带截断保护
        unsafe {
            let len = GetWindowTextLengthW(self.hwnd);
            let mut buf = vec![0u16; len as usize + 1];
            let n = GetWindowTextW(self.hwnd, &mut buf);
            String::from_utf16_lossy(&buf[..n.clamp(0, len) as usize])
        }
    }

    pub fn set_font(&self, font: &UiFont) {
        // SAFETY: WM_SETFONT 系统约定 wparam = HFONT 值, lparam = 1 表示立即重绘;
        // 字体所有权归调用方 (EditBox 不删, UiFont 生命周期须覆盖控件)
        unsafe {
            let _ = SendMessageW(
                self.hwnd,
                WM_SETFONT,
                Some(WPARAM(font.hfont.0 as usize)),
                Some(LPARAM(1)),
            );
        }
    }

    pub fn set_focus(&self) {
        // SAFETY: SetFocus 要求目标窗口与本调用同线程 — 控件与 chrome 同在渲染线程
        unsafe {
            let _ = SetFocus(Some(self.hwnd));
        }
    }

    /// 显隐 (属性区随选中变化用)
    pub fn show(&self, on: bool) {
        // SAFETY: hwnd 有效; 对已显/已藏状态幂等
        unsafe {
            let _ = ShowWindow(self.hwnd, if on { SW_SHOW } else { SW_HIDE });
        }
    }
}

impl Drop for EditBox {
    /// Drop = 销毁控件 (Rust 对象生命周期与 Win32 窗口对齐, 不会留孤儿 EDIT)
    fn drop(&mut self) {
        // SAFETY: hwnd 由本实例独有, 销毁后不再被访问
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
