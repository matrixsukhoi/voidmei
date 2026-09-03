use super::*;

// ---- vk_to_vc: 值域锚定 javap 提取的 NativeKeyEvent 常量 (Java oracle) ----

/// 字母/数字/F 键/编辑键 (VK 域 → VC 域) 与 jar 常量逐一相等
#[test]
fn vk_to_vc_letters_digits_fnkeys() {
    // 字母 A..Z (VK 0x41..0x5A): jar VC_A=30 .. VC_Z=44 (非连续, 逐个钉)
    let expect: [(u32, u16); 26] = [
        (0x41, 30),
        (0x42, 48),
        (0x43, 46),
        (0x44, 32),
        (0x45, 18),
        (0x46, 33),
        (0x47, 34),
        (0x48, 35),
        (0x49, 23),
        (0x4A, 36),
        (0x4B, 37),
        (0x4C, 38),
        (0x4D, 50),
        (0x4E, 49),
        (0x4F, 24),
        (0x50, 25),
        (0x51, 16),
        (0x52, 19),
        (0x53, 31),
        (0x54, 20),
        (0x55, 22),
        (0x56, 47),
        (0x57, 17),
        (0x58, 45),
        (0x59, 21),
        (0x5A, 44),
    ];
    for (vk, vc) in expect {
        assert_eq!(vk_to_vc(vk, false), vc, "vk {:#x}", vk);
    }
    // 数字排 0..9 (VK 0x30..0x39 → VC_0=11 顺序错位: 0 在 0x30, 1 在 0x31..)
    let digits = [
        (0x30u32, 11u16),
        (0x31, 2),
        (0x32, 3),
        (0x33, 4),
        (0x34, 5),
        (0x35, 6),
        (0x36, 7),
        (0x37, 8),
        (0x38, 9),
        (0x39, 10),
    ];
    for (vk, vc) in digits {
        assert_eq!(vk_to_vc(vk, false), vc);
    }
    // F1..F12 (VK_F1=0x70..VK_F10=0x79, VK_F11=0x7A, VK_F12=0x7B;
    // jar VC_F1=59..VC_F10=68, VC_F11=87, VC_F12=88)
    let fkeys = [
        (0x70u32, 59u16),
        (0x71, 60),
        (0x79, 68),
        (0x7A, 87),
        (0x7B, 88),
    ];
    for (vk, vc) in fkeys {
        assert_eq!(vk_to_vc(vk, false), vc);
    }
    // 常用编辑/控制键
    assert_eq!(vk_to_vc(0x1B, false), 1, "ESC"); // VC_ESCAPE
    assert_eq!(vk_to_vc(0x0D, false), 28, "ENTER"); // VC_ENTER
    assert_eq!(vk_to_vc(0x08, false), 14, "BACKSPACE");
    assert_eq!(vk_to_vc(0x09, false), 15, "TAB");
    assert_eq!(vk_to_vc(0x20, false), 57, "SPACE");
    assert_eq!(vk_to_vc(0x10, false), 42, "VK_SHIFT→VC_SHIFT_L");
    assert_eq!(vk_to_vc(0xA0, false), 42, "L-SHIFT");
    assert_eq!(vk_to_vc(0xA1, false), 54, "R-SHIFT");
    assert_eq!(vk_to_vc(0x11, false), 29, "VK_CONTROL→VC_CTRL_L");
    assert_eq!(vk_to_vc(0x12, false), 56, "VK_MENU→VC_ALT_L");
    assert_eq!(vk_to_vc(0x5B, false), 3675, "LWIN→VC_META");
    assert_eq!(vk_to_vc(0x5D, false), 3677, "APPS→VC_CONTEXT_MENU");
    assert_eq!(vk_to_vc(0x13, false), 3653, "PAUSE");
    // OEM 标点 (HotkeyRowRenderer 可绑域)
    assert_eq!(vk_to_vc(0xBA, false), 39, "SEMICOLON");
    assert_eq!(vk_to_vc(0xBB, false), 13, "OEM_PLUS→VC_EQUALS");
    assert_eq!(vk_to_vc(0xBC, false), 51, "COMMA");
    assert_eq!(vk_to_vc(0xBD, false), 12, "MINUS");
    assert_eq!(vk_to_vc(0xBE, false), 52, "PERIOD");
    assert_eq!(vk_to_vc(0xBF, false), 53, "SLASH");
    assert_eq!(vk_to_vc(0xC0, false), 41, "BACKQUOTE");
    assert_eq!(vk_to_vc(0xDB, false), 26, "OPEN_BRACKET");
    assert_eq!(vk_to_vc(0xDC, false), 43, "BACK_SLASH");
    assert_eq!(vk_to_vc(0xDD, false), 27, "CLOSE_BRACKET");
    assert_eq!(vk_to_vc(0xDE, false), 40, "QUOTE");
    // 小键盘 (jar VC_NUMPAD0=82..9=73 顺序错位钉三个)
    assert_eq!(vk_to_vc(0x60, false), 82, "NUMPAD0");
    assert_eq!(vk_to_vc(0x61, false), 79, "NUMPAD1");
    assert_eq!(vk_to_vc(0x69, false), 73, "NUMPAD9");
}

/// 三个 Lock 键 + 导航/编辑键 (HotkeyManager 过滤与用户可绑域的键)
#[test]
fn vk_to_vc_locks_and_navigation() {
    assert_eq!(vk_to_vc(0x14, false), VC_CAPS_LOCK as u16);
    assert_eq!(vk_to_vc(0x91, false), VC_SCROLL_LOCK as u16);
    // NumLock 硬件带 E0 前缀 (LLKHF_EXTENDED), 但 VK_NUMLOCK 不参与扩展 OR,
    // 转换后仍为 69 — HotkeyManager 的伪事件过滤依赖这一不变量
    assert_eq!(vk_to_vc(0x90, true), VC_NUM_LOCK as u16);
    // 导航键表值 (jar: VC_INSERT=3666 VC_HOME=3655 VC_END=3663
    // VC_PAGE_UP=3657 VC_PAGE_DOWN=3665 VC_UP=57416 VC_LEFT=57419
    // VC_RIGHT=57421 VC_DOWN=57424, 均 0x0E/0xE0 前缀形态)
    assert_eq!(vk_to_vc(0x2D, false), 3666);
    assert_eq!(vk_to_vc(0x24, false), 3655);
    assert_eq!(vk_to_vc(0x23, false), 3663);
    assert_eq!(vk_to_vc(0x21, false), 3657);
    assert_eq!(vk_to_vc(0x22, false), 3665);
    assert_eq!(vk_to_vc(0x26, false), 57416);
    assert_eq!(vk_to_vc(0x25, false), 57419);
    assert_eq!(vk_to_vc(0x27, false), 57421);
    assert_eq!(vk_to_vc(0x28, false), 57424);
    assert_eq!(vk_to_vc(0x2E, false), 3667, "DELETE");
    assert_eq!(vk_to_vc(0x2C, false), 3639, "PRINTSCREEN");
    assert_eq!(vk_to_vc(0x0C, false), 57420, "VK_CLEAR→VC_CLEAR");
}

/// 扩展键 OR 逻辑 = DLL 反汇编跳转表的逐分支复刻 (含历史怪癖, 保真不修正)
#[test]
fn vk_to_vc_extended_or_quirks() {
    // 小键盘回车: E0 1C → 表值 VK_RETURN=0x1C | 0x0E00 = 0xE1C
    // (锚定: DLL 表项 VK_RETURN→0x1C 逐字节验证 + OR 0x0E00 分支反汇编;
    // jar VC_ 常量无 KP_* 系, 0xE1C 不在常量表内 — Java 侧同样运行时算出)
    assert_eq!(vk_to_vc(0x0D, true), 0x1C | 0x0E00);
    // 扩展导航: 表值已带 0x0E/0xE0 前缀, 再 OR 0xEE00 是 jnativehook 的
    // 实机行为 (Java 侧收到的即 0xFE48 之类) — 用户配置存的就是这些值
    assert_eq!(vk_to_vc(0x26, true), 0xE048 | 0xEE00, "UP");
    assert_eq!(vk_to_vc(0x2D, true), 0x0E52 | 0xEE00, "INSERT");
    assert_eq!(vk_to_vc(0x21, true), 0x0E49 | 0xEE00, "PRIOR");
    // 扩展标志不影响不在跳转表里的键 (无 OR)
    assert_eq!(vk_to_vc(0x50, true), 25, "P 带 E0 也不变");
    assert_eq!(vk_to_vc(0x90, true), 69, "NUMLOCK");
}

/// 越界 VK (>0xFF) → VC_UNDEFINED (DLL 边界检查: `vk < 0x100`)
#[test]
fn vk_to_vc_out_of_range() {
    assert_eq!(vk_to_vc(0x100, false), VC_UNDEFINED as u16);
    assert_eq!(vk_to_vc(u32::MAX, true), VC_UNDEFINED as u16);
}

// ---- 绑定面 (HotkeyManager.java 各方法, 纯逻辑跨平台) ----

fn mgr_with_tx() -> (HotkeyManager, Receiver<HotkeyEvent>) {
    HotkeyManager::with_channel()
}









/// 未 init 时 shutdown 直接返回 (Java `if (!initialized) return;`) —
/// 绑定表不清; init 后 shutdown 清表 (keyBindings.clear)
#[test]
fn shutdown_clears_bindings_only_when_initialized() {
    let (mut m, _rx) = mgr_with_tx();
    m.bind(VC_P, "fmOverlayToggle");
    m.shutdown(); // 未 init: 早退, 表保留
    assert!(m.is_bound(VC_P), "未初始化时 shutdown 不得清表");
    if let Ok(()) = m.init() {
        assert!(m.is_bound(VC_P), "init 不清绑定 (Java 绑定表跨 init 存活)");
        m.shutdown();
        assert!(!m.is_bound(VC_P), "初始化后的 shutdown 必须清表");
        // shutdown 后可重新 init + 绑定 (Java 可重入)
        m.init().expect("re-init after shutdown");
        m.bind(VC_P, "fmOverlayToggle");
        assert!(m.is_bound(VC_P));
    }
    // 非 Windows: init Err — 上面的 init 块自然跳过, 断言路径不假通过
}

// ---- 钩子生命周期 + 派发路径 (Windows) ----

#[cfg(target_os = "windows")]
mod win {
    use super::super::*;
    use windows::Win32::Foundation::{LPARAM, WPARAM};

    /// 真实 SetWindowsHookExW 冒烟: init → 双重 init 早退 → shutdown →
    /// 再 init (不注入键盘, 只验安装/卸载链路与日志路径不炸)
    #[test]
    fn hook_install_lifecycle() {
        let (mut m, _rx) = HotkeyManager::with_channel();
        m.init().expect("钩子安装失败");
        // Safe to call multiple times - only initializes once
        m.init().expect("重复 init 必须早退 Ok");
        m.shutdown();
        m.init().expect("shutdown 后重装钩子失败");
        // Drop 再走一遍卸钩 (join 自身线程外的正常路径)
    }

    /// 测试线程上直接装填回调上下文并调 keyboard_proc —
    /// 完整覆盖 转换→过滤→查表→sink 派发 链 (无 OS 键盘注入, 确定性)
    fn dispatch(
        bindings: Arc<Mutex<HashMap<i32, String>>>,
        sink: Arc<dyn HotkeyEventSink>,
        vk: u32,
        flags: u32,
        msg: u32,
    ) -> LRESULT {
        HOOK_CTX.with(|c| {
            *c.borrow_mut() = Some(HookCtx { bindings, sink });
        });
        let kb = KBDLLHOOKSTRUCT {
            vkCode: vk,
            scanCode: 0,
            flags: windows::Win32::UI::WindowsAndMessaging::KBDLLHOOKSTRUCT_FLAGS(flags),
            time: 0,
            dwExtraInfo: 0,
        };
        let r = unsafe { keyboard_proc(0, WPARAM(msg as usize), LPARAM(&kb as *const _ as isize)) };
        HOOK_CTX.with(|c| *c.borrow_mut() = None);
        r
    }

    fn chan_sink() -> (Arc<dyn HotkeyEventSink>, Receiver<HotkeyEvent>) {
        let (sink, rx) = ChannelHotkeySink::new();
        (sink, rx)
    }

    fn empty_map() -> Arc<Mutex<HashMap<i32, String>>> {
        Arc::new(Mutex::new(HashMap::new()))
    }

    /// 绑定键按下 → 事件经 sink 送出, key_code = VC 域 (Java 键码语义)
    #[test]
    fn dispatch_bound_key_sends_vc_domain_event() {
        let map = empty_map();
        map.lock()
            .unwrap()
            .insert(VC_P, "fmOverlayToggle".to_string());
        let (sink, rx) = chan_sink();
        // VK_P = 0x50, 无扩展标志 → VC 25
        dispatch(map, sink, 0x50, 0, WM_KEYDOWN);
        let ev = rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("绑定键必须派发");
        assert_eq!(
            ev,
            HotkeyEvent {
                event_type: "fmOverlayToggle".into(),
                key_code: VC_P
            }
        );
    }

    /// NumLock 伪事件过滤 (HotkeyManager.java:63-66): 硬件 NumLock 带 E0
    #[test]
    fn dispatch_numlock_filtered() {
        let map = empty_map();
        map.lock()
            .unwrap()
            .insert(VC_NUM_LOCK, "shouldNotFire".into());
        let (sink, rx) = chan_sink();
        dispatch(map, sink, 0x90, LLKHF_EXTENDED.0, WM_KEYDOWN);
        assert!(
            rx.recv_timeout(std::time::Duration::from_millis(150))
                .is_err(),
            "NumLock 即使已绑定也不得派发"
        );
    }

    /// 未绑定键按下 → 静默 (keyBindings.get == null 路径)
    #[test]
    fn dispatch_unbound_key_silent() {
        let (sink, rx) = chan_sink();
        dispatch(empty_map(), sink, 0x51 /* Q */, 0, WM_KEYDOWN);
        assert!(rx
            .recv_timeout(std::time::Duration::from_millis(150))
            .is_err());
    }

    /// 自动重复: 同键持续按住逐次 WM_KEYDOWN → 逐次派发 (jnativehook 同形)
    #[test]
    fn dispatch_autorepeat_fires_each_press() {
        let map = empty_map();
        map.lock().unwrap().insert(VC_P, "fmOverlayToggle".into());
        let (sink, rx) = chan_sink();
        for _ in 0..3 {
            dispatch(map.clone(), sink.clone(), 0x50, 0, WM_KEYDOWN);
        }
        for i in 0..3 {
            assert!(
                rx.recv_timeout(std::time::Duration::from_secs(2)).is_ok(),
                "第 {} 次重复未派发",
                i + 1
            );
        }
    }

    /// WM_SYSKEYDOWN (Alt 组合) 同样进入按下派发
    #[test]
    fn dispatch_syskeydown_also_fires() {
        let map = empty_map();
        map.lock().unwrap().insert(VC_P, "fmOverlayToggle".into());
        let (sink, rx) = chan_sink();
        dispatch(map, sink, 0x50, 0, WM_SYSKEYDOWN);
        assert!(rx.recv_timeout(std::time::Duration::from_secs(2)).is_ok());
    }

    /// code < 0 直通 (低级钩子契约), 不派发不 panic
    #[test]
    fn dispatch_negative_code_passthrough() {
        let (_sink, rx) = chan_sink();
        let kb = KBDLLHOOKSTRUCT {
            vkCode: 0x50,
            scanCode: 0,
            flags: windows::Win32::UI::WindowsAndMessaging::KBDLLHOOKSTRUCT_FLAGS(0),
            time: 0,
            dwExtraInfo: 0,
        };
        let r = unsafe {
            keyboard_proc(
                -1,
                WPARAM(WM_KEYDOWN as usize),
                LPARAM(&kb as *const _ as isize),
            )
        };
        // CallNextHookEx(None, ...) 的返回值取决于进程内其他 LL 键盘钩子
        // (输入法/录屏工具), 非本回调契约 — 只断言不派发、不 panic
        let _ = r;
        assert!(rx
            .recv_timeout(std::time::Duration::from_millis(150))
            .is_err());
    }

    /// sink 回调 panic 不得 abort 进程 (panic 跨 extern "system" 边界
    /// unwind 即 abort) 也不得中断钩子链 — 对齐 Java UIStateBus 逐个
    /// catch 不中断 (LIFETIMES §2.2); 本测试不崩 = catch_unwind 生效
    struct PanickingSink;
    impl HotkeyEventSink for PanickingSink {
        fn on_hotkey(&self, _event: &HotkeyEvent) {
            panic!("sink boom (测试预期)");
        }
    }

    #[test]
    fn dispatch_sink_panic_is_contained() {
        let map = empty_map();
        map.lock()
            .unwrap()
            .insert(VC_P, "fmOverlayToggle".to_string());
        // 正常返回即证明 panic 被捕获且派发后代码路径继续
        let _ = dispatch(map, Arc::new(PanickingSink), 0x50, 0, WM_KEYDOWN);
    }

    /// 抬键/其他消息不进按下路径 (WM_KEYUP 不派发 — 监听器只实现 nativeKeyPressed)
    #[test]
    fn dispatch_keyup_ignored() {
        let map = empty_map();
        map.lock().unwrap().insert(VC_P, "fmOverlayToggle".into());
        let (sink, rx) = chan_sink();
        dispatch(map, sink, 0x50, 0, 0x0101 /* WM_KEYUP */);
        assert!(rx
            .recv_timeout(std::time::Duration::from_millis(150))
            .is_err());
    }
}
