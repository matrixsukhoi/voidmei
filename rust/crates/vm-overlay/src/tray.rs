//! 系统托盘 (win32 家族): Shell_NotifyIconW + 隐藏消息窗口 NIF_MESSAGE 回调。
//! 语义对齐 Java `Application.initSystemTray` (SystemTray/TrayIcon 的 C 类复刻):
//! - 左键点击 → CAS 防重入 → handler.activate()
//!   (Java: `ctr.stop(); ctr = new Controller()` — 重建应用核并弹设置窗)
//! - 右键 → 上下文菜单 设置/开始/退出 (Java 菜单仅 about/close 两项;
//!   "设置/开始"是把 Java 左键"一键重建(开设置窗+重启服务)"拆成菜单独立入口,
//!   "退出"对应 Java close 菜单项。Java 的 about 项是 UI 层 toast 通知,
//!   归组装层在 handler 侧挂接, 托盘本体不依赖 vm-ui)
//! - Drop → NIM_DELETE 删图标 + 销毁菜单/图标/窗口 (Java: close 项的 tray.remove(icon))
//!
//! 消息泵: 托盘回调经隐藏窗口 WNDPROC 到达, 消息在创建线程排队 —
//! 组装层主循环每帧调 [`TrayIcon::pump`] 分派 (对齐 AWT EDT 泵, 同 host.rs pump_events 模式)。
//!
//! explorer 重启: 监听 "TaskbarCreated" 广播, 到达时以原 (hWnd,uID) 重发 NIM_ADD
//! (Java: AWT SystemTray 内部自带该机制; win32 手工复刻必须自带, 否则 explorer
//! 崩溃重启后图标永久消失、菜单/退出入口不可达)。
//!
//! **线程亲和**: `pump()` 与 `Drop` 都必须在创建线程调用 — `PeekMessageW` 只搜调用
//! 线程的消息队列 (跨线程泵则托盘回调永不分派); `DestroyWindow` 不能销毁其他线程
//! 创建的窗口 (跨线程 Drop 则窗口+WNDPROC 可达性泄漏)。D8 拓扑下托盘归单 win32 泵线程拥有。

#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, DeleteObject, GetDC, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyIcon, DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, HICON, LoadIconW,
    PeekMessageW, PostMessageW, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow,
    TrackPopupMenu, TranslateMessage, CS_HREDRAW, CS_VREDRAW, HMENU, ICONINFO, IDI_APPLICATION,
    MF_STRING, MSG, PM_REMOVE, TPM_RIGHTBUTTON, WM_APP, WM_COMMAND, WM_CONTEXTMENU, WM_LBUTTONUP,
    WM_NULL, WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPED,
};

/// NIF_MESSAGE 回调消息号 (Java: AWT 内部把托盘鼠标事件转给 TrayIcon 的
/// MouseListener; win32 手工等价: 托盘区鼠标事件以本消息发往注册窗口)
const TRAY_CALLBACK_MSG: u32 = WM_APP + 1;

/// NOTIFYICONDATAW.uID — 单图标进程恒 1 (Java: AWT 内部 icon id, 无多图标场景)
const TRAY_ID: u32 = 1;

/// 菜单项 id (WM_COMMAND 的 LOWORD(wParam))
const MENU_ID_SETTINGS: usize = 1001; // Java 菜单无此项 (左键语义拆分)
const MENU_ID_START: usize = 1002; // Java 菜单无此项
const MENU_ID_EXIT: usize = 1003; // PORT: Java close 菜单项 (Lang.close)

/// 托盘触发的 UI 动作回调 (组装层注入; 托盘不依赖具体 UI 实现)
pub trait TrayHandler: Send {
    /// 左键点击 / 菜单"设置": 重建应用核并弹出设置窗体。
    /// PORT: Application.java:251-273 mouseClicked BUTTON1 →
    /// `ctr.stop(); ctr = new Controller()` (MainForm 随 Controller 重建显示)。
    /// 快速重复点击由托盘层 CAS 防重入拦截 (见 [`dispatch_activate`]),
    /// handler 侧只会串行收到。
    fn activate(&mut self);

    /// 菜单"开始": 启动/重启遥测 (Java 左键语义中 Controller 重建的服务启动部分,
    /// Java 菜单无对应项 — P5 组装层的拆分入口)
    fn start(&mut self);

    /// 菜单"退出": PORT: Application.java:229-235 close MenuItem →
    /// `tray.remove(icon); System.exit(0)` — 图标移除由 [`TrayIcon`] 的 Drop 完成,
    /// 进程退出由本回调完成 (Java System.exit 的归属方)。
    ///
    /// **退出序契约 (组装层实现必须遵守)**: 退出进程前必须先 drop 本托盘的
    /// [`TrayIcon`] — `std::process::exit` 不运行 Drop, 直接 exit 会丢 NIM_DELETE
    /// 留下僵尸托盘图标 (Java close 项是 `tray.remove(icon)` 显式在 `System.exit(0)` 之前)
    fn exit(&mut self);
}

/// 托盘配置 (标签/图标由组装层注入; Java 侧对应 Lang.close/Lang.mStart/appName
/// 与 "image/16x16.png" 这些运行时读取点)
pub struct TrayConfig {
    /// 悬停提示 (Java: icon.setToolTip(appName))
    pub tooltip: String,
    /// 图标 PNG 路径 (Java: Toolkit.getImage("image/16x16.png"));
    /// 仅支持 RGBA8 PNG, 读取失败/格式不符回退系统默认图标
    /// (Java AWT 画空图的更差表现, 有意加强)
    pub icon_path: PathBuf,
    /// 菜单"设置"标签 (Java 无对应键)
    pub settings_label: String,
    /// 菜单"开始"标签 (Java: Lang.mStart)
    pub start_label: String,
    /// 菜单"退出"标签 (Java: Lang.close)
    pub exit_label: String,
}

impl Default for TrayConfig {
    /// 默认标签取 vm-core Lang 静态表 — Java: initSystemTray 运行时读 Lang 字段
    fn default() -> Self {
        let lang = vm_core::lang::Lang::init_lang();
        Self {
            tooltip: lang.app_name.to_string(),
            icon_path: PathBuf::from("image/16x16.png"),
            settings_label: "设置".to_string(),
            start_label: lang.m_start.to_string(),
            exit_label: lang.close.to_string(),
        }
    }
}

/// WNDPROC 侧共享态: 菜单句柄 + 窗口 + 重挂数据 (右键弹菜单/explorer 重启要用)。
/// 进程内单实例 (Java initSystemTray 仅 main 调用一次)
static TRAY_SHARED: LazyLock<Mutex<Option<TrayShared>>> = LazyLock::new(|| Mutex::new(None));

#[derive(Clone, Copy)]
struct TrayShared {
    hwnd: HWND,
    menu: HMENU,
    /// "TaskbarCreated" 重挂用的 NIM_ADD 数据副本 (原样重发)
    nid: NOTIFYICONDATAW,
    /// 初始 NIM_ADD 是否成功 (失败不参与重挂 — Java: AWT add 失败后无图标可重挂)
    added: bool,
}

/// explorer (重)启动时向全部顶层窗口广播 "TaskbarCreated" 的注册消息号。
/// Java: AWT SystemTray 内部监听该广播自动重加 TrayIcon — win32 手工等价。
/// RegisterWindowMessageW 失败返回 0 (非合法消息号, 永不匹配即安全降级)
static TASKBAR_CREATED_MSG: LazyLock<u32> =
    LazyLock::new(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) });

// HWND/HMENU 是整数值句柄, 跨线程传递安全 (win.rs unsafe impl Send 同先例)
unsafe impl Send for TrayShared {}

/// 回调 handler 槽位 (WNDPROC 无法携带 Rust 上下文, 经静态槽中转 —
/// Java 对应: TrayIcon 监听器持有外部闭包, 由 AWT 代理窗口托管)。
/// 携带世代号: Drop/重装交错时凭 gen 做身份守卫, 不误清/复活他人 handler
/// (D8 单泵线程下本不可达, 防御跨线程组装错误)
static TRAY_HANDLER: LazyLock<Mutex<Option<HandlerSlot>>> = LazyLock::new(|| Mutex::new(None));

struct HandlerSlot {
    gen: u64,
    handler: Box<dyn TrayHandler>,
}

/// handler 世代分配器 (每次 install +1); Drop 作废时也 +1,
/// 使在途执行 (with_handler 摘槽期间) 的回填权失效
static TRAY_HANDLER_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 在途执行世代的标记 (0 = 空闲)。写入/读取均在 TRAY_HANDLER 锁内,
/// 与 Drop 的作废判定互斥 — 回调仅在泵线程串行 (WNDPROC 唯一入口), 单值够用
static TRAY_HANDLER_INFLIGHT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// PORT: Application.java:134 `trayClickProcessing = new AtomicBoolean(false)` —
/// 托盘点击 CAS 防重入标志 (Java static → 进程级静态, 跨 Controller 重建存活)
static TRAY_CLICK_PROCESSING: AtomicBool = AtomicBool::new(false);

/// 安装 handler (TrayIcon::new 与测试注入共用), 返回分配的世代号
fn install_handler(h: Box<dyn TrayHandler>) -> u64 {
    let gen = TRAY_HANDLER_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    *TRAY_HANDLER.lock().unwrap() = Some(HandlerSlot { gen, handler: h });
    gen
}

/// 摘除指定世代的 handler (TrayIcon Drop 与构造失败路径共用):
/// - 槽内是本代 → 清槽 + 世代+1 (作废在途回填)
/// - 槽内是他代 (新实例已重装) → 身份守卫不动
/// - 槽空但本代在途执行 (with_handler 摘走中) → 仅世代+1, Box 由 with_handler 释放
fn uninstall_handler(gen: u64) {
    let mut slot = TRAY_HANDLER.lock().unwrap();
    match slot.as_ref().map(|s| s.gen) {
        Some(g) if g == gen => {
            *slot = None;
            TRAY_HANDLER_GEN.fetch_add(1, Ordering::SeqCst);
        }
        Some(_) => {}
        None => {
            if TRAY_HANDLER_INFLIGHT.load(Ordering::SeqCst) == gen {
                TRAY_HANDLER_GEN.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
}

/// 取出 handler 锁外执行回调。
/// 锁纪律 (LIFETIMES §3.3): Mutex 不可重入, handler 若再进托盘层二次 lock 会死锁
/// (Java synchronized 可重入无此问题) — 锁内只摘槽位, 回调整体锁外;
/// 嵌套进入时槽位为空 → 安全空转。
/// PORT: Java AWT EDT 对监听器异常的兜底 (捕获打印不中断) → catch_unwind,
/// 回调 panic 不允许穿越 FFI 边界 (extern "system" 内 unwind = UB)
fn with_handler(f: impl FnOnce(&mut dyn TrayHandler)) {
    let mut slot = TRAY_HANDLER.lock().unwrap();
    let Some(mut s) = slot.take() else { return };
    // 锁内标记在途: 与 Drop 的作废判定互斥, 杜绝"摘槽后、标记前"的清空缝隙
    TRAY_HANDLER_INFLIGHT.store(s.gen, Ordering::SeqCst);
    drop(slot);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(s.handler.as_mut())));
    // 回填守卫: 世代未被作废 (Drop/重装会 +1) 且槽位仍空才回填;
    // 否则 handler 随本 Box drop 释放 — 防"已下线实例的 handler 复活滞留"
    let mut slot = TRAY_HANDLER.lock().unwrap();
    if slot.is_none() && s.gen == TRAY_HANDLER_GEN.load(Ordering::SeqCst) {
        *slot = Some(s);
    }
    TRAY_HANDLER_INFLIGHT.store(0, Ordering::SeqCst);
}

/// 左键/菜单"设置"分发: CAS 防重入 + finally 语义复位, 原样照搬 Java。
/// PORT: Application.java:253-271 mouseClicked BUTTON1:
/// `compareAndSet(false,true)` 失败 → 记日志忽略; 成功 → 重建 → finally `set(false)`
fn dispatch_activate() {
    // 使用CAS操作防止快速重复点击导致多次创建Controller
    // compareAndSet: 如果当前值为false则设为true并返回true，否则返回false
    if TRAY_CLICK_PROCESSING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        vm_core::logger::info("Application", "Ignoring duplicate tray click");
        return;
    }
    // (Java 在 try 内打点三行日志, e2e 断言标记 — 原文保留)
    vm_core::logger::info("Application", "--------------------------------------------------");
    vm_core::logger::info("Application", "ACTION: Tray Icon Clicked. Restoring MainForm...");
    vm_core::logger::info("Application", "--------------------------------------------------");
    with_handler(|h| h.activate());
    // 无论成功或异常都重置标志，允许下一次点击
    // (Java finally — handler panic 已被 with_handler 捕获, 此处必达)
    TRAY_CLICK_PROCESSING.store(false, Ordering::SeqCst);
}

/// 菜单"开始"分发 (Java 菜单项无 CAS 守卫, 直调)
fn dispatch_start() {
    with_handler(|h| h.start());
}

/// 菜单"退出"分发 (Java: tray.remove(icon); System.exit(0) — remove 在 Drop, exit 在 handler)
fn dispatch_exit() {
    with_handler(|h| h.exit());
}

/// WM_COMMAND 分发: 返回是否为已知菜单项 (测试断言用)
fn on_menu_command(cmd_id: usize) -> bool {
    match cmd_id {
        MENU_ID_SETTINGS => {
            dispatch_activate();
            true
        }
        MENU_ID_START => {
            dispatch_start();
            true
        }
        MENU_ID_EXIT => {
            dispatch_exit();
            true
        }
        _ => false,
    }
}

/// 托盘 WNDPROC: FFI 边界 + panic 墙, 逻辑在 [`tray_dispatch`]。
/// catch_unwind 覆盖 WNDPROC 可达路径整体 (handler 回调/日志/锁 unwrap 任一 panic
/// 面), 不允许 unwind 穿越 extern "system" (现代 Rust 为 abort 而非 UB, 但会杀死
/// 整个进程)。panic 时兜底 DefWindowProc 保持消息链存活
/// (对齐 AWT EDT 对监听器异常"捕获打印不中断"的兜底语义)
unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // 句柄/参数均为 Copy 整数, AssertUnwindSafe 无实际逃逸面
    let handled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        unsafe { tray_dispatch(hwnd, msg, wparam, lparam) }
    }));
    match handled {
        Ok(r) => r,
        // panic 已吞, 不再调 logger (logger 自身可能就是 panic 源, 二次 unwind 无人捕获)
        Err(_) => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// WNDPROC 消息逻辑: 处理 NIF_MESSAGE 回调/菜单命令/explorer 重启广播, 其余走 DefWindowProc
unsafe fn tray_dispatch(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        TRAY_CALLBACK_MSG => {
            // 回调约定: LOWORD(lParam) = 鼠标消息 (uID 在 HIWORD, 单图标无分流需求)
            let mouse_msg = (lparam.0 & 0xFFFF) as u32;
            match mouse_msg {
                // Java mouseClicked = 按下+抬起成对完成 → 取抬起沿
                WM_LBUTTONUP => dispatch_activate(),
                // 新旧 shell 每代只发其一 (旧 WM_RBUTTONUP / 新 WM_CONTEXTMENU), 实际
                // 不会双到达 — 若真先后到达, 第二条会在第一条 TrackPopupMenu 的模态
                // 循环内再入弹第二份菜单 (行为依赖"仅其一"的 shell 约定)
                WM_RBUTTONUP | WM_CONTEXTMENU => show_context_menu(hwnd),
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            // 菜单选择: LOWORD(wParam) = 菜单项 id; 未知 id 交默认处理不吞消息
            if on_menu_command(wparam.0 & 0xFFFF) {
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        // explorer 重启: 广播到达即原样重发 NIM_ADD (图标记录随旧 explorer 死亡清空)
        m if m == *TASKBAR_CREATED_MSG => {
            readd_icon();
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// explorer 重启后重挂托盘图标: 以共享槽保存的 NIM_ADD 数据副本重发。
/// 锁内只 Copy 数据, Shell_NotifyIconW 是外部调用不得持锁执行
unsafe fn readd_icon() {
    let shared = TRAY_SHARED.lock().unwrap().filter(|s| s.added).map(|s| s.nid);
    let Some(nid) = shared else { return };
    // 广播迟到 (Drop 后 NIM_DELETE 已清) 或 (hWnd,uID) 仍存在时失败无害
    if Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
        vm_core::logger::info("Application", "任务栏重启, 托盘图标已重挂");
    }
}

/// 右键弹出上下文菜单 (Java: icon.setPopupMenu(p) 的 AWT 内部机制)
unsafe fn show_context_menu(hwnd: HWND) {
    // 锁内只取菜单句柄 (Copy), TrackPopupMenu 是外部调用不得持锁执行
    let menu = TRAY_SHARED.lock().unwrap().map(|s| s.menu);
    let Some(menu) = menu else { return };

    // MSDN 任务栏图标菜单规范: 弹出前 SetForegroundWindow + 关闭后补 WM_NULL,
    // 否则点击菜单外区域菜单不消失 (AWT setPopupMenu 内部同机制)
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, Some(0), hwnd, None);
    let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
}

/// NUL 结尾 UTF-16 (win32 宽字符串)
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 解码 PNG (仅 RGBA8) → 32bpp HICON。
/// Java: Toolkit.getImage("image/16x16.png") 交 AWT 内部转图标;
/// win32 手工等价: png 解码 → CreateDIBSection(BGRA top-down) + 单色全零掩码
/// → CreateIconIndirect (alpha 通道生效, 掩码被忽略, 按惯例仍需提供)。
/// icon_path 是 pub 注入面: 非 RGBA PNG (灰度/RGB/调色板) 显式报错走调用方
/// IDI_APPLICATION 回退 — chunks_exact_mut(4) 按 4 字节/像素解析, 通道数
/// 不符会静默错位 (花屏), 不猜测展开
fn load_icon(path: &Path) -> Result<HICON, String> {
    let data = std::fs::read(path).map_err(|e| format!("读 {} 失败: {}", path.display(), e))?;
    // png::Decoder 要求 Read+Seek (compare.rs 的 BufReader<File> 同先例)
    let mut decoder = png::Decoder::new(std::io::Cursor::new(data));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("PNG info 失败: {}", e))?;
    let mut buf = vec![0u8; reader.output_buffer_size().unwrap_or(0)];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("PNG 解码失败: {}", e))?;
    // 通道数守卫: normalize_to_color8 只做 16→8bit 归一, 不改 color type —
    // 灰度(1B/px)/RGB(3B/px) 输入若放行, 后续按 4B/px 解析必错位
    if info.color_type != png::ColorType::Rgba {
        return Err(format!("非 RGBA PNG (color_type={:?}), 不做错位解析", info.color_type));
    }
    let (w, h) = (info.width as i32, info.height as i32);
    if w <= 0 || h <= 0 {
        return Err(format!("图标尺寸非法: {}x{}", w, h));
    }
    buf.truncate((w * h * 4) as usize);

    unsafe {
        let hdc_screen = GetDC(None);
        // RGBA → BGRA (win32 DIB 字节序; HICON 色位图用直通 alpha, 不预乘)
        let mut bgra = buf.clone();
        for px in bgra.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
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
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let color = match CreateDIBSection(Some(hdc_screen), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
        {
            Ok(d) => d,
            Err(e) => {
                ReleaseDC(None, hdc_screen);
                return Err(format!("CreateDIBSection: {}", e));
            }
        };
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());
        // 单色全零掩码 (AND=0: 每像素取色位图; 32bpp 下实际由 alpha 决定)
        let mask_row = ((w + 31) / 32) * 4; // DWORD 对齐
        let mask_bits = vec![0u8; (mask_row * h) as usize];
        let mask = CreateBitmap(w, h, 1, 1, Some(mask_bits.as_ptr() as *const _));
        ReleaseDC(None, hdc_screen);

        let ii = ICONINFO {
            fIcon: windows::core::BOOL(1),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: color,
        };
        let hicon = CreateIconIndirect(&ii);
        // CreateIconIndirect 复制位模, 两张源位图需自删
        let _ = DeleteObject(HGDIOBJ(color.0));
        let _ = DeleteObject(HGDIOBJ(mask.0));
        hicon.map_err(|e| format!("CreateIconIndirect: {}", e))
    }
}

/// 系统托盘句柄: 隐藏消息窗口 + Shell_NotifyIcon 注册 + 上下文菜单。
/// 进程内单实例 (Java initSystemTray 仅 main 调一次; 静态共享槽不允许第二个)
pub struct TrayIcon {
    hwnd: HWND,
    menu: HMENU,
    icon: HICON,
    /// 本实例 handler 的世代号 (Drop 摘槽身份守卫)
    handler_gen: u64,
    /// CreateIconIndirect 产物才需要 DestroyIcon (LoadIconW 共享图标禁删)
    owns_icon: bool,
    /// NIM_ADD 成功才置位 (Drop 时据此决定是否 NIM_DELETE)
    added: bool,
}

// Send 允许跨线程移动, 但线程亲和仍在: pump()/Drop 必须回到创建线程调用
// (见模块文档; D8 拓扑下托盘全程留在单 win32 泵线程)
unsafe impl Send for TrayIcon {}

impl TrayIcon {
    /// 创建托盘: 隐藏窗口 → 菜单 → 图标 → NIM_ADD。
    /// NIM_ADD 失败不致命: PORT: Java `tray.add(icon)` 抛 AWTException →
    /// ExceptionHelper.logAndContinue("系统托盘") + debugPrint(Lang.failaddtoTray)
    /// — 程序无托盘继续运行, 返回 Ok(added=false)
    pub fn new(handler: Box<dyn TrayHandler>, cfg: TrayConfig) -> Result<TrayIcon, String> {
        // 单实例守卫: 静态共享槽 (WNDPROC 中转) 只有一个, **构造全程持锁** —
        // check-then-create 存在 TOCTOU 缝隙 (两线程并发 new 都能过检, 后者
        // install_handler 覆盖槽位使首个托盘的点击落到第二个 handler);
        // D8 单泵线程拓扑下不可达, 持锁把"不允许第二个"从注释约定变成硬保证
        let mut shared = TRAY_SHARED.lock().unwrap();
        if let Some(s) = shared.as_ref() {
            return Err(format!("托盘已存在 (hwnd={:?}), 进程内单实例", s.hwnd.0));
        }

        unsafe {
            // 先装 handler: 窗口一旦可达, WNDPROC 回调就要有受体
            let handler_gen = install_handler(handler);

            let hinstance = GetModuleHandleW(None)
                .map_err(|e| format!("GetModuleHandleW: {}", e))?;
            let class_name = w!("VoidMeiTrayWnd");
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(tray_wnd_proc),
                hInstance: hinstance.into(),
                lpszClassName: class_name,
                cbClsExtra: 0,
                cbWndExtra: 0,
                hIcon: Default::default(),
                hCursor: Default::default(),
                hbrBackground: Default::default(),
                lpszMenuName: Default::default(),
            };
            let atom = RegisterClassW(&wc);
            if atom == 0 {
                // 多次注册同类: 已存在视为成功 (对齐 win.rs 先例)
                if windows::Win32::Foundation::GetLastError()
                    != windows::Win32::Foundation::ERROR_CLASS_ALREADY_EXISTS
                {
                    // 失败路径回收已装 handler (窗口不可达, 槽位无意义)
                    uninstall_handler(handler_gen);
                    return Err("RegisterClassW 失败".into());
                }
            }

            // 隐藏消息窗口: WS_OVERLAPPED 且不 ShowWindow (Java: AWT 内部辅助窗口,
            // 用户不可见; 不用 HWND_MESSAGE 消息专用窗口 — TrackPopupMenu
            // 需可前台化的属主窗口, 否则菜单点外部不关闭)
            let hwnd = CreateWindowExW(
                Default::default(),
                class_name,
                w!("VoidMei Tray"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
            .map_err(|e| {
                uninstall_handler(handler_gen);
                format!("CreateWindowExW: {}", e)
            })?;

            // 菜单: 设置 / 开始 / 退出 (Java: PopupMenu + about/close 两项)
            let menu = match CreatePopupMenu() {
                Ok(m) => m,
                Err(e) => {
                    uninstall_handler(handler_gen);
                    let _ = DestroyWindow(hwnd);
                    return Err(format!("CreatePopupMenu: {}", e));
                }
            };
            for (id, label) in [
                (MENU_ID_SETTINGS, &cfg.settings_label),
                (MENU_ID_START, &cfg.start_label),
                (MENU_ID_EXIT, &cfg.exit_label),
            ] {
                let wide = to_wide(label);
                if let Err(e) = AppendMenuW(
                    menu,
                    MF_STRING,
                    id,
                    windows::core::PCWSTR(wide.as_ptr()),
                ) {
                    uninstall_handler(handler_gen);
                    let _ = DestroyMenu(menu);
                    let _ = DestroyWindow(hwnd);
                    return Err(format!("AppendMenuW: {}", e));
                }
            }

            // 图标: PNG 优先, 失败回退系统默认 (Java getImage 失败画空图, 有意加强)。
            // 在共享槽登记前加载 — 彻底失败时槽未登记, 回收路径无需回滚共享槽
            let (icon, owns_icon) = match load_icon(&cfg.icon_path) {
                Ok(h) => (h, true),
                Err(e) => {
                    vm_core::logger::warn_default(&format!("图标加载失败回退默认: {}", e));
                    match LoadIconW(None, IDI_APPLICATION) {
                        Ok(h) => (h, false),
                        // 失败路径资源回收 (win.rs create 同纪律): 窗口/菜单/handler
                        Err(e) => {
                            uninstall_handler(handler_gen);
                            let _ = DestroyMenu(menu);
                            let _ = DestroyWindow(hwnd);
                            return Err(format!("LoadIconW: {}", e));
                        }
                    }
                }
            };

            // NIM_ADD: NIF_MESSAGE(回调) | NIF_ICON | NIF_TIP(悬停提示)
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: TRAY_ID,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: TRAY_CALLBACK_MSG,
                hIcon: icon,
                ..Default::default()
            };
            // szTip 上限 128 UTF-16 码元 (含 NUL)
            let tip: Vec<u16> = cfg.tooltip.encode_utf16().take(127).collect();
            nid.szTip[..tip.len()].copy_from_slice(&tip);

            // 共享槽登记 (右键菜单/explorer 重挂要用; Drop 时摘除)。
            // nid 副本随槽保存, TaskbarCreated 到达时原样重发 NIM_ADD
            *shared = Some(TrayShared {
                hwnd,
                menu,
                nid,
                added: false,
            });

            let added = Shell_NotifyIconW(NIM_ADD, &nid).as_bool();
            if !added {
                // PORT: Java 两条失败日志 — `ExceptionHelper.logAndContinue(e1, "系统托盘")`
                // (WARN, 组件"系统托盘"; Shell_NotifyIconW 只回 BOOL 无异常对象, 以描述代)
                vm_core::logger::warn("系统托盘", "托盘加入失败 (NIM_ADD), 程序继续运行");
                // PORT: Java `debugPrint(Lang.failaddtoTray)` → Logger.info("Legacy", ...)
                vm_core::logger::info(
                    "Legacy",
                    vm_core::lang::Lang::init_lang().failaddto_tray,
                );
            }
            // 重挂资格随 NIM_ADD 结果落槽 (失败不参与重挂, 对齐 Java 无图标可重挂)
            if let Some(s) = shared.as_mut() {
                s.added = added;
            }

            Ok(TrayIcon {
                hwnd,
                menu,
                icon,
                handler_gen,
                owns_icon,
                added,
            })
        }
    }

    /// 一轮消息泵: 分派本线程队列里发往托盘窗口的消息 (回调在调用线程执行)。
    /// PORT: Java AWT EDT 泵的等价物 — 组装层主循环每帧调用
    /// (对齐 host.rs pump_events 的"调用方驱动"模式)
    pub fn pump(&mut self) {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, Some(self.hwnd), 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    /// NIM_ADD 是否成功 (测试/组装层诊断用)
    pub fn is_added(&self) -> bool {
        self.added
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe {
            // PORT: Java close MenuItem 的 tray.remove(icon) — 退出前删图标防僵尸占位
            if self.added {
                let nid = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: self.hwnd,
                    uID: TRAY_ID,
                    ..Default::default()
                };
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            }
            let _ = DestroyMenu(self.menu);
            let _ = DestroyWindow(self.hwnd);
            if self.owns_icon {
                let _ = DestroyIcon(self.icon);
            }
        }
        // 共享槽摘除: 仅当仍指向本实例 (防御被后建实例替换的极端错序)
        let mut shared = TRAY_SHARED.lock().unwrap();
        if shared.map(|s| s.hwnd == self.hwnd).unwrap_or(false) {
            *shared = None;
        }
        // handler 槽位摘除: 世代身份守卫 (新实例已重装则不动其 handler;
        // 本代在途执行则作废其回填权, 防"已下线 handler 复活")
        uninstall_handler(self.handler_gen);
        // PORT: Java trayClickProcessing 是 static 且不复位 (JVM 即退无需复位) —
        // Drop 不碰 CAS 标志; 若在途点击尚未走完 finally, 标志由 dispatch_activate 复位
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// 串行化本模块测试: 共享静态槽 (TRAY_HANDLER/TRAY_SHARED/CAS 标志),
    /// cargo 默认并行会互相清场 (对齐 win.rs TEST_LOCK 先例)
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// 录音 handler: 计数器经 Arc 与测试句柄共享 (handler 装进静态槽后仍可观测)
    #[derive(Default, Clone)]
    struct Counts {
        activate: Arc<std::sync::atomic::AtomicUsize>,
        start: Arc<std::sync::atomic::AtomicUsize>,
        exit: Arc<std::sync::atomic::AtomicUsize>,
    }

    struct Recorder {
        counts: Counts,
        /// activate 执行期间嵌套触发一次 dispatch_activate (模拟处理中又来点击)
        nested_click_in_activate: bool,
    }

    impl TrayHandler for Recorder {
        fn activate(&mut self) {
            self.counts.activate.fetch_add(1, Ordering::SeqCst);
            if self.nested_click_in_activate {
                // 嵌套点击: CAS 必须拦截 (标志仍为 true)
                dispatch_activate();
            }
        }
        fn start(&mut self) {
            self.counts.start.fetch_add(1, Ordering::SeqCst);
        }
        fn exit(&mut self) {
            self.counts.exit.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl Recorder {
        fn default_counts() -> Self {
            Self {
                counts: Counts::default(),
                nested_click_in_activate: false,
            }
        }
    }

    /// 仓库根的 16x16.png (git 资产; Java 同路径 image/16x16.png)。
    /// 上溯三级到仓库根 (realtests.rs 同惯例: vm-overlay → crates → rust → voidmei)
    fn repo_icon_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../image/16x16.png")
    }

    /// CAS 防重入: 处理中(标志 true)的第二次点击被忽略 + 计数不增
    #[test]
    fn cas_guard_blocks_click_while_processing() {
        let _guard = TEST_LOCK.lock().unwrap();
        TRAY_CLICK_PROCESSING.store(false, Ordering::SeqCst);
        let counts = Counts::default();
        install_handler(Box::new(Recorder {
            counts: counts.clone(),
            nested_click_in_activate: false,
        }));
        // 模拟"上一次点击尚未处理完" (标志被占)
        TRAY_CLICK_PROCESSING.store(true, Ordering::SeqCst);
        dispatch_activate();
        assert_eq!(counts.activate.load(Ordering::SeqCst), 0, "占用期点击必须被忽略");
        // 释放后恢复受理
        TRAY_CLICK_PROCESSING.store(false, Ordering::SeqCst);
        dispatch_activate();
        assert_eq!(counts.activate.load(Ordering::SeqCst), 1);
        // finally 语义: 分发完成标志复位
        assert!(!TRAY_CLICK_PROCESSING.load(Ordering::SeqCst));
        *TRAY_HANDLER.lock().unwrap() = None;
    }

    /// 嵌套点击 (handler 执行中重入): 内层被 CAS 拦截, 外层恰好一次
    #[test]
    fn nested_click_during_handler_is_ignored() {
        let _guard = TEST_LOCK.lock().unwrap();
        TRAY_CLICK_PROCESSING.store(false, Ordering::SeqCst);
        let counts = Counts::default();
        install_handler(Box::new(Recorder {
            counts: counts.clone(),
            nested_click_in_activate: true,
        }));
        dispatch_activate();
        // 外层 1 次; 嵌套 1 次被拦 (若未拦会是 2 次)
        assert_eq!(counts.activate.load(Ordering::SeqCst), 1);
        // 标志最终复位 (允许下一次点击)
        assert!(!TRAY_CLICK_PROCESSING.load(Ordering::SeqCst));
        dispatch_activate();
        assert_eq!(counts.activate.load(Ordering::SeqCst), 2);
        *TRAY_HANDLER.lock().unwrap() = None;
    }

    /// 菜单命令分发: 三个 id 各达其位, 未知 id 无副作用;
    /// "设置"走 CAS 守卫路径 (与左键同语义), 开始/退出直调 (Java 菜单项无守卫)
    #[test]
    fn menu_command_dispatch_routes_to_handler() {
        let _guard = TEST_LOCK.lock().unwrap();
        TRAY_CLICK_PROCESSING.store(false, Ordering::SeqCst);
        let counts = Counts::default();
        install_handler(Box::new(Recorder {
            counts: counts.clone(),
            nested_click_in_activate: false,
        }));
        assert!(on_menu_command(MENU_ID_START));
        assert!(on_menu_command(MENU_ID_EXIT));
        assert!(on_menu_command(MENU_ID_SETTINGS));
        assert!(!on_menu_command(9999), "未知 id 必须报告未处理");
        assert_eq!(counts.start.load(Ordering::SeqCst), 1);
        assert_eq!(counts.exit.load(Ordering::SeqCst), 1);
        assert_eq!(counts.activate.load(Ordering::SeqCst), 1);
        *TRAY_HANDLER.lock().unwrap() = None;
    }

    /// handler 卸载后的分发是安全空转 (Drop 后残留消息路径不得 panic)
    #[test]
    fn dispatch_without_handler_is_noop() {
        let _guard = TEST_LOCK.lock().unwrap();
        TRAY_CLICK_PROCESSING.store(false, Ordering::SeqCst);
        *TRAY_HANDLER.lock().unwrap() = None;
        dispatch_activate();
        dispatch_start();
        dispatch_exit();
        assert!(!TRAY_CLICK_PROCESSING.load(Ordering::SeqCst));
    }

    /// 真实 PNG → HICON: 解码项目根 image/16x16.png 并生成有效图标句柄
    #[test]
    fn icon_built_from_repo_png() {
        let path = repo_icon_path();
        if !path.exists() {
            // git 资产缺失时显式跳过 (对齐 data/ 缺失跳过惯例, 非假通过)
            eprintln!("跳过: {} 缺失", path.display());
            return;
        }
        let hicon = load_icon(&path).expect("16x16.png 应能生成 HICON");
        assert!(!hicon.is_invalid());
        unsafe {
            let _ = DestroyIcon(hicon);
        }
    }

    /// 缺失文件: load_icon 报错不 panic (运行时回退默认图标路径的前提)
    #[test]
    fn icon_load_missing_file_errs() {
        let r = load_icon(Path::new("Z:/voidmei/不存在.png"));
        assert!(r.is_err());
    }

    /// 非 RGBA PNG (RGB8): load_icon 必须显式报错 — icon_path 是 pub 注入面,
    /// 通道数不符若放行会按 4B/px 错位解析 (花屏), 报错走 IDI_APPLICATION 回退
    #[test]
    fn icon_rejects_non_rgba_png() {
        let path =
            std::env::temp_dir().join(format!("voidmei_tray_rgb8_{}.png", std::process::id()));
        let file = std::fs::File::create(&path).expect("建临时 PNG");
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), 4, 4);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().expect("PNG header");
        w.write_image_data(&[128u8; 4 * 4 * 3]).expect("PNG 数据");
        drop(w); // Writer Drop 收尾 IEND (compare.rs 编码同源)
        let r = load_icon(&path);
        let _ = std::fs::remove_file(&path);
        assert!(r.is_err(), "RGB8 PNG 必须被拒绝 (走默认图标回退)");
        let msg = r.unwrap_err();
        assert!(msg.contains("非 RGBA"), "错误应说明通道数不符: {}", msg);
    }

    /// 真实托盘全链路: 创建(NIM_ADD) → pump 空转 → Drop(NIM_DELETE + 槽位清理)
    #[test]
    fn tray_add_pump_drop_roundtrip() {
        let _guard = TEST_LOCK.lock().unwrap();
        TRAY_CLICK_PROCESSING.store(false, Ordering::SeqCst);
        let counts = Counts::default();
        let mut tray = TrayIcon::new(
            Box::new(Recorder {
                counts: counts.clone(),
                nested_click_in_activate: false,
            }),
            TrayConfig {
                icon_path: repo_icon_path(),
                ..Default::default()
            },
        )
        .expect("托盘创建失败");
        // 本机有任务栏, NIM_ADD 应成功 (CI 无桌面环境会在此暴露, 不做假通过)
        assert!(tray.is_added());
        // 共享槽已登记
        assert!(TRAY_SHARED.lock().unwrap().is_some());
        // 无消息时泵空转
        tray.pump();
        drop(tray);
        // Drop 后: 图标删了 (无法直接观测, 以槽位清理为代理), 静态槽复位
        assert!(TRAY_SHARED.lock().unwrap().is_none());
        assert!(TRAY_HANDLER.lock().unwrap().is_none());
        // CAS 标志对齐 Java static 语义不由 Drop 复位 — 本测试无点击, 恒 false
        assert!(!TRAY_CLICK_PROCESSING.load(Ordering::SeqCst));
    }

    /// 单实例守卫: 存活期间二次创建被拒
    #[test]
    fn second_tray_instance_rejected() {
        let _guard = TEST_LOCK.lock().unwrap();
        let t1 = TrayIcon::new(
            Box::new(Recorder::default_counts()),
            TrayConfig {
                icon_path: repo_icon_path(),
                ..Default::default()
            },
        )
        .expect("首个托盘创建失败");
        let r2 = TrayIcon::new(
            Box::new(Recorder::default_counts()),
            TrayConfig::default(),
        );
        assert!(r2.is_err(), "进程内第二个托盘必须被拒绝");
        drop(t1);
        // 首个 Drop 后可重建
        let t3 = TrayIcon::new(
            Box::new(Recorder::default_counts()),
            TrayConfig {
                icon_path: repo_icon_path(),
                ..Default::default()
            },
        );
        assert!(t3.is_ok(), "Drop 后应可重建托盘");
    }

    /// explorer 重启链路: TaskbarCreated 广播 → WNDPROC → 重发 NIM_ADD, 不 panic
    /// (Java: AWT SystemTray 内部自动重挂的手工等价, 广播号经 RegisterWindowMessageW 注册)
    #[test]
    fn taskbar_created_broadcast_readd_no_panic() {
        let _guard = TEST_LOCK.lock().unwrap();
        let mut tray = TrayIcon::new(
            Box::new(Recorder::default_counts()),
            TrayConfig {
                icon_path: repo_icon_path(),
                ..Default::default()
            },
        )
        .expect("托盘创建失败");
        // 广播消息号注册成功 (失败返回 0 = 永不匹配的安全降级)
        let taskbar_msg = *TASKBAR_CREATED_MSG;
        assert_ne!(taskbar_msg, 0, "TaskbarCreated 消息号注册失败");
        // 重挂数据已随 NIM_ADD 结果落槽
        {
            let shared = TRAY_SHARED.lock().unwrap();
            let s = shared.as_ref().expect("共享槽已登记");
            assert_eq!(s.nid.uID, TRAY_ID, "重挂副本应携带原 uID");
            assert_eq!(s.hwnd, tray.hwnd);
            assert!(s.added, "NIM_ADD 成功后重挂资格应置位");
        }
        // 模拟 explorer 重启广播 → 泵分派 → 重挂路径不 panic
        unsafe {
            let _ = PostMessageW(Some(tray.hwnd), taskbar_msg, WPARAM(0), LPARAM(0));
        }
        tray.pump();
        drop(tray);
    }

    /// activate 必 panic 的 handler (panic 墙测试用)
    struct PanickingActivator;

    impl TrayHandler for PanickingActivator {
        fn activate(&mut self) {
            panic!("handler 故意 panic (panic 墙测试)");
        }
        fn start(&mut self) {}
        fn exit(&mut self) {}
    }

    /// panic 墙: handler panic 被 WNDPROC 的 catch_unwind 吞, 不穿越 FFI
    /// (穿出会 abort 测试进程); finally 语义下 CAS 标志仍复位
    #[test]
    fn panic_in_handler_contained_by_pump() {
        let _guard = TEST_LOCK.lock().unwrap();
        TRAY_CLICK_PROCESSING.store(false, Ordering::SeqCst);
        let mut tray = TrayIcon::new(
            Box::new(PanickingActivator),
            TrayConfig {
                icon_path: repo_icon_path(),
                ..Default::default()
            },
        )
        .expect("托盘创建失败");
        // 投递左键抬起回调 (lParam LOWORD = WM_LBUTTONUP)
        unsafe {
            let _ = PostMessageW(
                Some(tray.hwnd),
                TRAY_CALLBACK_MSG,
                WPARAM(0),
                LPARAM(WM_LBUTTONUP as isize),
            );
        }
        tray.pump();
        // panic 后标志复位 (Java finally 语义), 下次点击可受理
        assert!(!TRAY_CLICK_PROCESSING.load(Ordering::SeqCst));
        drop(tray);
    }
}
