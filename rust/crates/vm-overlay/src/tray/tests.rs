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
