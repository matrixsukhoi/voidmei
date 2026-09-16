//! Windows 平台实现: WS_POPUP 窗口两种形态 (WindowKind, 纯 CPU, 无 GPU 上下文):
//! - Layered: WS_EX_LAYERED + UpdateLayeredWindow (HUD 悬浮透明面, 预乘 BGRA)
//! - Opaque: 普通窗口 (编辑 chrome; 直通 BGRA, WM_PAINT 经 DIB 重画, 可嵌子控件)
//! 行为对齐 Java AWT 透明窗 (同为 ULW 路径); 穿透 = WS_EX_TRANSPARENT 切换 (Java 版无, 属增强)

#![allow(non_snake_case)]

use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, EndPaint,
    GetDC, MonitorFromWindow, ReleaseDC, SelectObject, SetDIBitsToDevice, ValidateRect,
    AC_SRC_ALPHA, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP,
    HDC, HGDIOBJ, MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, SRCCOPY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, GetCursorPos, GetWindowLongPtrW, LoadCursorW,
    PeekMessageW, RegisterClassW, SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage,
    CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, GWL_EXSTYLE, HTCLIENT, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST,
    IDC_ARROW, MSG, PM_REMOVE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE,
    SW_SHOWNOACTIVATE, ULW_ALPHA, WINDOW_EX_STYLE, WM_CAPTURECHANGED, WM_DESTROY,
    WM_ERASEBKGND, WM_LBUTTONDOWN,
    WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCHITTEST, WM_PAINT,
    WM_POINTERUP, WM_RBUTTONDOWN,
    WM_COMMAND,
    WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};

use super::{OverlayEvent, WindowConfig, WindowKind};

/// WNDPROC 按 hwnd 分流的事件队列表 (多窗口支持)
/// key = HWND.0 as isize; create 时登记条目, Drop 时销毁条目
/// POC 期是全进程单队列 (EVENT_QUEUE), 多窗口下事件会串台;
/// Java 无此层 (Swing 事件按组件分发), 此处等价于"每窗口自己的事件流"
static EVENT_QUEUES: LazyLock<Mutex<HashMap<isize, VecDeque<OverlayEvent>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 裸指针 Send 包装 (静态注册表的类型约束; WNDPROC 与 create/present 同在
/// 渲染线程实际访问, 包装只为让静态表编译通过)
struct PaintBits(*const u8);
unsafe impl Send for PaintBits {}

/// Opaque 窗口的重画面 (hwnd → DIB 位图指针与尺寸): WM_PAINT 时
/// 系统表面已失效, 经此取 DIB 重画 (Layered 走 ULW 无此需求)。
/// 裸指针安全性: DIB 内存归 WinOverlay 拥有, Drop 先注销本表再释放
/// DIB, 注销后不再被访问; set_size 重建 DIB 时同步换新
static PAINT_SURFACES: LazyLock<Mutex<HashMap<isize, (PaintBits, i32, i32)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 查重画面 (None = Layered 窗口 → wnd_proc 走原逻辑)
fn paint_surface(hwnd: HWND) -> Option<(*const u8, i32, i32)> {
    PAINT_SURFACES
        .lock()
        .ok()
        .and_then(|m| m.get(&(hwnd.0 as isize)).map(|(b, w, h)| (b.0, *w, *h)))
}

/// 登记/更新重画面 (create 登记; set_size 重建 DIB 后换新指针)
fn upsert_paint_surface(hwnd: HWND, bits: *const u8, w: i32, h: i32) {
    if let Ok(mut m) = PAINT_SURFACES.lock() {
        m.insert(hwnd.0 as isize, (PaintBits(bits), w, h));
    }
}

/// 注销重画面 (Drop 用; 与 remove_queue 对称, Layered 无条目时幂等)
fn remove_paint_surface(hwnd: HWND) {
    if let Ok(mut m) = PAINT_SURFACES.lock() {
        m.remove(&(hwnd.0 as isize));
    }
}

pub struct WinOverlay {
    hwnd: HWND,
    width: i32,
    height: i32,
    kind: WindowKind,
    // DIB 资源 (present 用)
    memdc: HDC,
    dib: HBITMAP,
    old_obj: HGDIOBJ,
    dib_bits: *mut u8,
}

unsafe impl Send for WinOverlay {}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCHITTEST => {
            // 整窗客户区 (Java setFocusable(false) 等价效果: 不进系统移动/缩放)
            LRESULT(HTCLIENT as isize)
        }
        WM_LBUTTONDOWN | WM_POINTERUP => {
            // 捕获鼠标: 快速拖拽滑出窗口后仍能收到 MOVE/UP (否则拖拽中断)
            let _ = SetCapture(hwnd);
            let (x, y) = cursor_root_pos();
            push_event(
                hwnd,
                OverlayEvent::MousePress {
                    root_x: x,
                    root_y: y,
                },
            );
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            // R5 编辑面: 右键 (不捕获 — 无右键拖拽语义)
            let (x, y) = cursor_root_pos();
            push_event(
                hwnd,
                OverlayEvent::RightPress {
                    root_x: x,
                    root_y: y,
                },
            );
            LRESULT(0)
        }
        WM_LBUTTONDBLCLK => {
            // R5 编辑面: 双击 (窗口类需 CS_DBLCLKS 才会收到)
            let (x, y) = cursor_root_pos();
            push_event(
                hwnd,
                OverlayEvent::DoubleClick {
                    root_x: x,
                    root_y: y,
                },
            );
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let _ = ReleaseCapture();
            push_event(hwnd, OverlayEvent::MouseRelease);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let (x, y) = cursor_root_pos();
            let left_down = wparam.0 & 0x0001 != 0;
            push_event(
                hwnd,
                OverlayEvent::MouseMove {
                    root_x: x,
                    root_y: y,
                    left_down,
                },
            );
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            // 编辑面: 侧栏列表滚动 (delta = wparam 高位字, WHEEL_DELTA=120 阶数)
            let (x, y) = cursor_root_pos();
            let delta = ((wparam.0 >> 16) & 0xFFFF) as u16 as i16 as i32;
            push_event(
                hwnd,
                OverlayEvent::MouseWheel {
                    root_x: x,
                    root_y: y,
                    delta,
                },
            );
            LRESULT(0)
        }
        WM_COMMAND => {
            // B1 子控件通知: id = wparam 低 16 位, code = 高 16 位 (EN_* 通知码,
            // 解码与 WM_MOUSEWHEEL 的 GET_WHEEL_DELTA_WPARAM 同模式)。
            // lParam (控件 hwnd) 不路由; 简单起见全量转发, host 侧按 id 忽略未知控件
            let id = (wparam.0 & 0xFFFF) as u32;
            let code = ((wparam.0 >> 16) & 0xFFFF) as u16 as i32;
            push_event(hwnd, OverlayEvent::Control { id, code });
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            // 系统夺走捕获时结束拖拽 (防 drag 状态卡死)
            push_event(hwnd, OverlayEvent::MouseRelease);
            LRESULT(0)
        }
        WM_ERASEBKGND => {
            // Opaque: 返回 1 不擦背景 (present 自绘全幅, 擦了闪白);
            // Layered: 原逻辑 DefWindowProcW (内容恒由 ULW 提供)
            if paint_surface(hwnd).is_some() {
                LRESULT(1)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        WM_PAINT => {
            // WNDPROC 拿不到实例的 memdc — 重画面只能经 PAINT_SURFACES
            // 注册表取 (注册表存在的原因)。
            // Opaque: 系统表面已失效, 用 DIB (SetDIBitsToDevice) 全幅重画,
            // GDI 裁剪到失效区; EndPaint 校验。
            // Layered: 未注册 → ValidateRect 直接过 (ULW 窗口不响应 WM_PAINT)
            match paint_surface(hwnd) {
                Some((bits, w, h)) => unsafe {
                    let mut ps = PAINTSTRUCT::default();
                    let hdc = BeginPaint(hwnd, &mut ps);
                    let bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: w,
                            biHeight: -h, // top-down (与 create 的 DIB 一致)
                            biPlanes: 1,
                            biBitCount: 32,
                            biCompression: BI_RGB.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    SetDIBitsToDevice(
                        hdc,
                        0,
                        0,
                        w as u32,
                        h as u32,
                        0,
                        0,
                        0,
                        h as u32,
                        bits as *const std::ffi::c_void,
                        &bmi,
                        DIB_RGB_COLORS,
                    );
                    let _ = EndPaint(hwnd, &ps);
                    LRESULT(0)
                },
                None => unsafe {
                    let _ = ValidateRect(Some(hwnd), None);
                    LRESULT(0)
                },
            }
        }
        WM_DESTROY => {
            push_event(hwnd, OverlayEvent::Close);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn push_event(hwnd: HWND, ev: OverlayEvent) {
    if let Ok(mut map) = EVENT_QUEUES.lock() {
        let q = map.entry(hwnd.0 as isize).or_insert_with(VecDeque::new);
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

/// 取走指定窗口队列头事件 (poll_event 用; 测试直接调用做分流模拟)
fn drain_event(hwnd: HWND) -> Option<OverlayEvent> {
    EVENT_QUEUES
        .lock()
        .ok()
        .and_then(|mut map| map.get_mut(&(hwnd.0 as isize))?.pop_front())
}

/// 移除窗口的事件队列条目 (Drop 用; 滞留事件一并丢弃)
fn remove_queue(hwnd: HWND) {
    if let Ok(mut map) = EVENT_QUEUES.lock() {
        map.remove(&(hwnd.0 as isize));
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
        let _ = SetProcessDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
    }
}

pub fn create(cfg: WindowConfig) -> Result<WinOverlay, String> {
    set_dpi_awareness();

    unsafe {
        let hinstance = GetModuleHandleW(None).map_err(|e| format!("GetModuleHandleW: {}", e))?;
        let class_name = w!("VoidMeiOverlay");

        let wc = WNDCLASSW {
            // CS_DBLCLKS: 编辑面双击事件 (R5)
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
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
            // 多窗口: 第二个窗口起同类已注册 (ERROR_CLASS_ALREADY_EXISTS), 视为成功
            if windows::Win32::Foundation::GetLastError()
                != windows::Win32::Foundation::ERROR_CLASS_ALREADY_EXISTS
            {
                return Err("RegisterClassW 失败".into());
            }
        }

        let mut ex_style = match cfg.kind {
            WindowKind::Layered => {
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE
            }
            // Opaque 去 WS_EX_LAYERED: ULW 窗口内容全由 ULW 表面提供, WS_CHILD
            // 子控件不会被合成进来 (互斥是硬约束); 去 WS_EX_NOACTIVATE: 该样式
            // 阻止子控件获焦 (B 阶段 EDIT 依赖)。置顶/工具窗两形态共用
            WindowKind::Opaque => WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        };
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

        // 事件队列登记: 本 hwnd 的分流条目 (Drop 时移除)
        if let Ok(mut map) = EVENT_QUEUES.lock() {
            map.entry(hwnd.0 as isize).or_insert_with(VecDeque::new);
        }

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
        let dib = match CreateDIBSection(Some(hdc_screen), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
        {
            Ok(d) => d,
            Err(e) => {
                // 失败路径资源回收: 此时 hwnd 已创建且 EVENT_QUEUES 条目已登记,
                // 直接向上传播会泄漏僵尸窗口句柄 + 队列永久条目 (条目不移除则
                // 后续同槽位 key 复用时 push_event 仍会写入死队列)
                ReleaseDC(None, hdc_screen);
                let _ = DestroyWindow(hwnd);
                remove_queue(hwnd);
                return Err(format!("CreateDIBSection: {}", e));
            }
        };
        let memdc = CreateCompatibleDC(Some(hdc_screen));
        let old_obj = SelectObject(memdc, HGDIOBJ(dib.0));
        ReleaseDC(None, hdc_screen);

        // Opaque 登记 WM_PAINT 重画面 (Layered 走 ULW, 内容不依赖 WM_PAINT)
        if cfg.kind == WindowKind::Opaque {
            upsert_paint_surface(hwnd, bits as *const u8, cfg.width, cfg.height);
        }

        Ok(WinOverlay {
            hwnd,
            width: cfg.width,
            height: cfg.height,
            kind: cfg.kind,
            memdc,
            dib,
            old_obj,
            dib_bits: bits as *mut u8,
        })
    }
}

impl WinOverlay {
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
            // 两 kind 共用 DIB: buf 直拷入 (语义按 kind 分化 —
            // Layered = 预乘 BGRA / Opaque = 直通 BGRA, 调用方按 kind 喂)
            std::ptr::copy_nonoverlapping(buf.as_ptr(), self.dib_bits, expect);
            match self.kind {
                WindowKind::Layered => {
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
                WindowKind::Opaque => {
                    // 普通窗口: DIB 经 BitBlt 上客户区 (DC 即取即还);
                    // 失败与 ULW 同型向上 Err (诚实暴露, 不静默吞)
                    let hdc = GetDC(Some(self.hwnd));
                    let blit = BitBlt(
                        hdc,
                        0,
                        0,
                        self.width,
                        self.height,
                        Some(self.memdc),
                        0,
                        0,
                        SRCCOPY,
                    );
                    let _ = ReleaseDC(Some(self.hwnd), hdc);
                    if blit.is_err() {
                        return Err("BitBlt 失败".into());
                    }
                }
            }
        }
        Ok(())
    }

    fn set_position(&mut self, x: i32, y: i32) {
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_BOTTOM),
                x,
                y,
                0,
                0,
                windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                    | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
            );
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

    fn set_topmost(&mut self, on: bool) {
        // Java Window.setAlwaysOnTop — AlwaysOnTopCoordinator
        // suspendAll/restoreAll 的底层动作; 创建即 WS_EX_TOPMOST (POC 全窗口置顶), 此处运行时切换
        unsafe {
            let after = if on {
                Some(HWND_TOPMOST)
            } else {
                Some(HWND_NOTOPMOST)
            };
            let _ = SetWindowPos(
                self.hwnd,
                after,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    fn set_visible(&mut self, visible: bool) {
        // Java Window.setVisible — AlwaysOnTopCoordinator.hideAllOverlays/
        // showAllOverlays (FocusMonitor 游戏失焦自动隐藏) 的底层动作。
        // 显示用 SW_SHOWNOACTIVATE (配 WS_EX_NOACTIVATE): 恢复显示不抢焦点
        // (Java 侧 overlay setFocusable(false) 的等价防护)
        unsafe {
            let cmd = if visible { SW_SHOWNOACTIVATE } else { SW_HIDE };
            let _ = ShowWindow(self.hwnd, cmd);
        }
    }

    fn set_size(&mut self, w: i32, h: i32) {
        // PORT(WYSIWYG): Java reinitConfig → setBounds 的窗口几何面。两步:
        // ① DIB 重建 (present 缓冲 w*h*4 与新尺寸一致; UpdateLayeredWindow 的
        //   psize 参数也随 present 用新尺寸驱动分层窗口大小);
        // ② SetWindowPos 即时改窗几何 (present 前的空窗期不闪旧尺寸)。
        // 失败路径: 新 DIB 建不成则保持旧资源提前返回 — 后续 present 因缓冲
        // 尺寸不符报错 (诚实暴露, 不静默吞)
        if w == self.width && h == self.height {
            return;
        }
        unsafe {
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let hdc_screen = GetDC(None);
            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let new_dib = match CreateDIBSection(
                Some(hdc_screen),
                &bmi,
                DIB_RGB_COLORS,
                &mut bits,
                None,
                0,
            ) {
                Ok(d) => d,
                Err(_) => {
                    ReleaseDC(None, hdc_screen);
                    return; // 保持旧尺寸 (见方法头失败路径注)
                }
            };
            let new_memdc = CreateCompatibleDC(Some(hdc_screen));
            ReleaseDC(None, hdc_screen);
            // 旧资源释放 (select 回默认位图再删, 防 DC 持已删句柄)
            let _ = SelectObject(self.memdc, self.old_obj);
            let _ = DeleteDC(self.memdc);
            let _ = DeleteObject(HGDIOBJ(self.dib.0));
            self.dib = new_dib;
            self.memdc = new_memdc;
            self.old_obj = SelectObject(new_memdc, HGDIOBJ(new_dib.0));
            self.dib_bits = bits as *mut u8;
            // Opaque: 重画面换新 (旧 DIB 已释放, 残留旧指针 = WM_PAINT 悬垂读;
            // 须在 SetWindowPos 之前 — 改几何可能同步派发 WM_PAINT)
            if self.kind == WindowKind::Opaque {
                upsert_paint_surface(self.hwnd, bits as *const u8, w, h);
            }
            let _ = SetWindowPos(
                self.hwnd,
                None,
                0,
                0,
                w,
                h,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        self.width = w;
        self.height = h;
    }

    fn child_parent_handle(&self) -> usize {
        // Opaque 窗口的子控件接线面 (EditBox 父句柄); Layered 返回同值但
        // ULW 与 WS_CHILD 互斥, 不会被用于建子控件
        self.hwnd.0 as usize
    }

    fn poll_event(&mut self) -> Option<OverlayEvent> {
        unsafe {
            // 只泵本窗口消息 → WNDPROC → 本 hwnd 队列 (多窗口互不串扰;
            // 其他窗口的消息由各自实例的 poll_event 泵。POC 单窗口路径行为不变)
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, Some(self.hwnd), 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        // 只取本实例队列 (单实例时即原全局单队列语义)
        drain_event(self.hwnd)
    }

    fn screen_size(&self) -> (i32, i32) {
        unsafe {
            // 以窗口所在显示器为准 (多屏时位置归一化稳定)。
            // Java 侧统一除以启动主屏 Application.screenWidth/Height
            // (ConfigurationService), 此处为 MonitorFromWindow — 单屏
            // 行为一致; 多屏下跨显示器拖拽后存/取的归一化基准可能不同 (已知有意偏差,
            // 对接 Java 版迁移来的归一化配置时需注意语义差异)
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
                    windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
                ),
                windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                    windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN,
                ),
            )
        }
    }
}

impl Drop for WinOverlay {
    fn drop(&mut self) {
        // 先注销 Opaque 重画面再释放 DIB (销毁链可能触发 WM_PAINT,
        // 注册表残留已释放指针 = 悬垂读; Layered 无条目时幂等)
        remove_paint_surface(self.hwnd);
        unsafe {
            let _ = SelectObject(self.memdc, self.old_obj);
            let _ = DeleteDC(self.memdc);
            let _ = DeleteObject(HGDIOBJ(self.dib.0));
            // DestroyWindow 同步触发 WM_DESTROY → Close 事件入本 hwnd 队列 (条目移除前,
            // 与 Java Window.dispose → 子类 dispose 链等价: 销毁动作先于注销)
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.hwnd);
        }
        // 移除本 hwnd 的队列条目 (销毁后滞留事件一并丢弃)
        remove_queue(self.hwnd);
    }
}

