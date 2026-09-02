//! 三件平台小件合一文件 (P4 平台件收官批的任务边界裁决, §0.6 一文件一模块
//! 在此让位于批内合一; 待波16 按域拆分):
//! 1. DPI 检测 — Java `src/prog/util/DPIHelper.java` (scale/logical 尺寸语义保真)
//! 2. Windows 前台焦点检测 — Java `src/prog/util/WindowsFocusDetector.java`,
//!    实现 vm-core `focus_monitor::FocusDetector` trait (Windows 腿落地)
//! 3. winmm 声音播放 — javax.sound.sampled Clip/AudioSystem 使用面,
//!    实现 vm-core `voice_resource_manager::{SoundPlayer, SoundClip}` trait
//!    (PORTING §3 库映射裁决: winmm PlaySound, "语音是整文件播放, 够用")

use vm_core::base::format::java_round;
use vm_core::base::logger;
use vm_core::audio::voice_resource_manager::SoundError;

// =====================================================================
// 1. DPI — 对应 Java: src/prog/util/DPIHelper.java (一比一翻译)
//    跨平台纯计算腿 (可测) + cfg(windows) Win32 检测腿
// =====================================================================

/// 对应 Java: `public final class DPIHelper`
///
/// PORT: Java 是静态字段 + `synchronized init()` 幂等 + `ensureInitialized()`
/// 惰性初始化 —— 三者都服务于"静态可变全局"这一载体 (LIFETIMES §1.2 DPI/Screen
/// 组归 Env, 启动后只读); Rust 改值语义: 构造一次由调用方持有, 幂等由
/// "只构造一次"承担, 懒初始化模式随之消解。
#[derive(Debug, Clone)]
pub struct DpiHelper {
    /// DPI scale factors (1.0 = 100%, 2.0 = 200%)
    scale_x: f64,
    scale_y: f64,
    /// Logical screen dimensions (what Swing sees)
    logical_screen_width: i32,
    logical_screen_height: i32,
    /// Physical screen dimensions (actual monitor pixels)
    physical_screen_width: i32,
    physical_screen_height: i32,
}

impl DpiHelper {
    /// 对应 Java: `init()` try 块成功路径 — 检测值注入 (跨平台纯构造,
    /// 供测试与非 Windows 集成方使用)。
    /// PORT: 成功日志在此发出 (Java init 尾部的 Logger.info)。
    pub fn from_detection(
        physical_width: i32,
        physical_height: i32,
        scale_x: f64,
        scale_y: f64,
    ) -> Self {
        // Calculate logical screen dimensions
        // Physical pixels / scale factor = logical pixels
        // PORT: Java `if (scaleX > 0)` 分支顺序原样; `(int) Math.round(...)` 的
        // long→int 截断以 `as i32` 复刻 (屏幕像素域, 无溢出面)
        let logical_w = if scale_x > 0.0 {
            java_round(physical_width as f64 / scale_x) as i32
        } else {
            physical_width
        };
        let logical_h = if scale_y > 0.0 {
            java_round(physical_height as f64 / scale_y) as i32
        } else {
            physical_height
        };
        // Log DPI detection results
        // PORT: Java String.format("%.2fx%.2f, %dx%d ...") → {:.2}/{}
        logger::info(
            "DPIHelper",
            &format!(
                "DPI Detection: Scale={:.2}x{:.2}, Physical={}x{}, Logical={}x{}",
                scale_x, scale_y, physical_width, physical_height, logical_w, logical_h
            ),
        );
        DpiHelper {
            scale_x,
            scale_y,
            logical_screen_width: logical_w,
            logical_screen_height: logical_h,
            physical_screen_width: physical_width,
            physical_screen_height: physical_height,
        }
    }

    /// 对应 Java: `init()` catch 块 — 检测失败回退 100% 缩放。
    /// PORT: Java catch 的 `e.getMessage()` → reason 参数。
    pub fn fallback(physical_width: i32, physical_height: i32, reason: &str) -> Self {
        // Fallback to 100% scaling if detection fails
        logger::warn(
            "DPIHelper",
            &format!("DPI detection failed, using defaults: {}", reason),
        );
        DpiHelper {
            scale_x: 1.0,
            scale_y: 1.0,
            logical_screen_width: physical_width,
            logical_screen_height: physical_height,
            physical_screen_width: physical_width,
            physical_screen_height: physical_height,
        }
    }

    /// Returns the horizontal DPI scale factor. 1.0 = 100%, 1.5 = 150%, 2.0 = 200%
    pub fn get_scale_x(&self) -> f64 {
        self.scale_x
    }

    /// Returns the vertical DPI scale factor. Usually equals get_scale_x().
    pub fn get_scale_y(&self) -> f64 {
        self.scale_y
    }

    /// Returns the primary DPI scale factor (horizontal).
    pub fn get_scale(&self) -> f64 {
        self.scale_x
    }

    /// Returns the logical screen width in pixels (what Swing sees / window positioning)
    pub fn get_logical_screen_width(&self) -> i32 {
        self.logical_screen_width
    }

    /// Returns the logical screen height in pixels
    pub fn get_logical_screen_height(&self) -> i32 {
        self.logical_screen_height
    }

    /// Returns the physical screen width in actual monitor pixels
    pub fn get_physical_screen_width(&self) -> i32 {
        self.physical_screen_width
    }

    /// Returns the physical screen height in actual monitor pixels
    pub fn get_physical_screen_height(&self) -> i32 {
        self.physical_screen_height
    }

    /// Scales a base value by the DPI scale factor (int 版本, Java 重载 1/2)。
    pub fn scale(&self, base_value: i32) -> i32 {
        java_round(base_value as f64 * self.scale_x) as i32
    }

    /// Scales a base value by the DPI scale factor (double version, 不取整)
    pub fn scale_f64(&self, base_value: f64) -> f64 {
        base_value * self.scale_x
    }

    /// Inverse scale — converts a scaled value back to base value.
    pub fn unscale(&self, scaled_value: i32) -> i32 {
        // Java `if (scaleX == 0)` 精确比较原样保持
        if self.scale_x == 0.0 {
            return scaled_value;
        }
        java_round(scaled_value as f64 / self.scale_x) as i32
    }

    /// Returns true if the system is using high-DPI scaling (> 100%).
    pub fn is_high_dpi(&self) -> bool {
        self.scale_x > 1.01 || self.scale_y > 1.01
    }
}

// =====================================================================
// WAV 头解析 — SoundPlayer winmm 腿的文件验收面 (跨平台纯函数)
// =====================================================================

/// 校验 RIFF/WAVE 容器并返回时长 (秒)。
/// 对应 Java `AudioSystem.getAudioInputStream` 的验收面: 文件不可读/非受支持
/// 音频格式抛异常 → 本函数以 Err 收敛 (open_clip 的失败合同)。
/// PORT: 只认 RIFF/WAVE 容器 (VoidMei 语音包全为 wav); Java 另支持 AU/AIFF
/// 等, 项目内无使用面。fmt 只取 byteRate (时长计算用), 不校验压缩格式 —
/// PlaySound 能否播放某编码由其自身决定, 与 Java 解码器集的差异属已知取舍。
/// 又: 多 fmt/data 块取最后出现者、顶层 RIFF 尺寸字段不校验 (Java AudioSystem
/// 取首个且严格校验容器) — 规范语音包单块 PCM, 该分歧不可达, 备案。
pub fn parse_wav_duration(bytes: &[u8]) -> Result<f64, SoundError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let mut byte_rate: Option<u32> = None;
    let mut data_size: Option<u32> = None;
    // RIFF 块遍历: [4B id][4B LE size][body]; size 奇数时后随 1 字节 pad
    let mut i = 12usize;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let size =
            u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        let body = i + 8;
        if id == b"fmt " {
            if size < 16 || body + 16 > bytes.len() {
                return Err("malformed fmt chunk".into());
            }
            // fmt 布局: format(2) channels(2) sampleRate(4) byteRate(4) ...
            byte_rate = Some(u32::from_le_bytes([
                bytes[body + 8],
                bytes[body + 9],
                bytes[body + 10],
                bytes[body + 11],
            ]));
        } else if id == b"data" {
            // 尺寸字段可能虚标超出文件尾, 钳到实际可读长度
            data_size = Some(size.min(bytes.len() - body) as u32);
        }
        // 奇数尺寸的 pad 字节一并跳过; PORT: 32 位目标上 body+size (u32 虚标)
        // 可使 usize 回绕 → 理论死循环, checked 加法, 溢出视为越过文件尾结束
        // 遍历 (64 位目标无此面, 防御性)
        match body.checked_add(size).and_then(|n| n.checked_add(size & 1)) {
            Some(n) => i = n,
            None => break,
        }
    }
    match (byte_rate, data_size) {
        (Some(br), Some(ds)) if br > 0 => Ok(ds as f64 / br as f64),
        _ => Err("missing fmt/data chunk or zero byteRate".into()),
    }
}

// =====================================================================
// Windows 平台腿: DPI 检测 / 焦点检测 / winmm 播放
// =====================================================================
#[cfg(target_os = "windows")]
mod win {
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;
    use std::time::Instant;

    use vm_core::platform::focus_monitor::FocusDetector;
    use vm_core::base::logger;
    use vm_core::audio::voice_resource_manager::{SoundClip, SoundPlayer, SoundError};

    use super::{parse_wav_duration, DpiHelper};

    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, POINT};
    use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FILENAME, SND_FLAGS, SND_NODEFAULT};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::HiDpi::{
        GetDpiForMonitor, GetDpiForWindow, SetProcessDpiAwarenessContext,
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, MDT_EFFECTIVE_DPI,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetSystemMetrics, GetWindowThreadProcessId, SM_CXSCREEN, SM_CYSCREEN,
    };

    // ---------------- DPI 检测腿 ----------------

    impl DpiHelper {
        /// 对应 Java: `DPIHelper.init()` 的 Windows 检测腿 (Per-Monitor V2)。
        /// Java 语义 = Toolkit.getScreenSize() (主屏物理像素) +
        /// GraphicsConfiguration.getDefaultTransform() (缩放系数); Rust 侧以
        /// GetSystemMetrics + GetDpiForMonitor(主屏 effective DPI) 对应。
        ///
        /// PORT (uiScale 语义, 任务指定注明): Java 发行版 JVM flag
        /// `-Dsun.java2d.uiScale=1` (voidmeil4j.xml) 强制 Java2D 缩放为 1 →
        /// `getDefaultTransform()` 恒等 → DPIHelper 检测得 scale=1.0、
        /// logical==physical (即 exe 启动下 Application.dpiScale 恒 1,
        /// 100% 缩放屏上全部计算与旧代码一致)。Rust 侧无 JVM 位图缩放可关:
        /// win.rs 的 Per-Monitor V2 已使全部 API 物理像素直读, 渲染天然 1:1
        /// 物理像素 (uiScale=1 的"清晰字体"目标由自身绘制达成), 故本实现报告
        /// 真实 OS 缩放; vm-app 若要逐字复刻 exe 行为 (dpiScale 恒 1),
        /// 以 `DpiHelper::from_detection(w, h, 1.0, 1.0)` 构造即可 —
        /// w/h 必须传**逻辑尺寸** (取 init() 结果的 get_logical_screen_width()
        /// /height()): exe 下 Java 所见的 physical==logical==逻辑像素
        /// (200% 屏 3840 物理时 Java 两值均 1920), 误传物理尺寸则
        /// 高 DPI 屏上复刻失败。
        pub fn init() -> DpiHelper {
            unsafe {
                // Per-Monitor V2 感知: 未设时 GetDpiForMonitor 只会拿到被虚拟化的
                // 96 DPI; win.rs create() 同款调用 (幂等, 进程已设其他级别时
                // 失败即忽略, 与窗口路径不冲突)
                let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
                // Get physical screen size from Toolkit → 主屏物理像素
                // (PMv2 下 GetSystemMetrics 不被系统虚拟化)
                let physical_width = GetSystemMetrics(SM_CXSCREEN);
                let physical_height = GetSystemMetrics(SM_CYSCREEN);
                // Detect DPI scale using GraphicsConfiguration → 主屏 effective DPI
                match primary_monitor_dpi() {
                    Ok((dx, dy)) => DpiHelper::from_detection(
                        physical_width,
                        physical_height,
                        dx as f64 / 96.0,
                        dy as f64 / 96.0,
                    ),
                    // Fallback to 100% scaling if detection fails (Java catch 块)
                    // PORT: Java catch 重读 Toolkit.getScreenSize() (可再抛 →
                    // initialized 保持 false); 此处复用已读的 GetSystemMetrics 值 —
                    // 该 API 无失败再抛面, 差异仅在无显示器退化会话 (不可达环境)
                    Err(e) => DpiHelper::fallback(
                        physical_width,
                        physical_height,
                        &format!("GetDpiForMonitor: {}", e),
                    ),
                }
            }
        }

        /// 指定窗口的 DPI (Per-Monitor V2: 跨显示器迁移后的实时值)。
        /// GetDpiForWindow 失败 (无效句柄) 返回 0 → Java catch 同款回退。
        /// PORT: 窗口 DPI 无 x/y 之分, scaleX=scaleY (Java transform 理论可
        /// 分离, 实践恒等); 物理尺寸仍取主屏, 对齐 Java Toolkit.getScreenSize()
        /// 的主屏语义 (Java Application.screenWidth/Height 同源)。
        ///
        /// PORT (超出 Java 面的新增 API): Java DPIHelper 仅启动期主屏一次性检测
        /// (LIFETIMES §1.2 DPI 归 Env, 启动后只读), 无每窗口变体 — 本构造器仅供
        /// 运行时 DPI 语义的扩展需求; Java 对拍/移植路径只允许
        /// `from_detection(w, h, 1.0, 1.0)` 复刻 exe 行为 (见 init() PORT 注)。
        /// 又: 混合缩放多屏下"窗口 DPI × 主屏物理尺寸"的组合语义错位
        /// (logical = 主屏物理/窗口 scale, 窗口在他屏时无意义) — vm-app
        /// 接线前不应采纳本构造器, 或改取窗口所在监视器 (MonitorFromWindow)
        /// 的尺寸。
        /// 又: from_detection 尾部每次构造都打一条 INFO 日志 (Java init 幂等仅
        /// 一条), 勿在高频路径反复调用。
        pub fn for_window(hwnd: HWND) -> DpiHelper {
            unsafe {
                let dpi = GetDpiForWindow(hwnd);
                let w = GetSystemMetrics(SM_CXSCREEN);
                let h = GetSystemMetrics(SM_CYSCREEN);
                if dpi > 0 {
                    let s = dpi as f64 / 96.0;
                    DpiHelper::from_detection(w, h, s, s)
                } else {
                    DpiHelper::fallback(w, h, "GetDpiForWindow: 0")
                }
            }
        }
    }

    /// 主屏 (POINT{0,0} 所在监视器) 的 effective DPI。
    /// Java 侧对应 GraphicsEnvironment.getDefaultScreenDevice() — 主屏。
    fn primary_monitor_dpi() -> windows::core::Result<(u32, u32)> {
        unsafe {
            let mon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTONEAREST);
            let mut dx = 0u32;
            let mut dy = 0u32;
            GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy).map(|_| (dx, dy))
        }
    }

    // ---------------- 焦点检测 ----------------

    /// 对应 Java: `src/prog/util/WindowsFocusDetector.java` (C 类/P4,
    /// windows crate 绑定)。Windows 平台专用焦点检测器。
    /// 跨平台分派腿 (FocusDetector.java 的 os.name 判定, A 类) 不在本文件,
    /// 见 vm-core focus_monitor.rs trait 注 (FocusDetector trait 归属维持在那侧)。
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

    // ---------------- winmm 声音播放 ----------------

    /// javax.sound.sampled.AudioSystem 工厂面的 winmm 实现 (D7 注入面的平台腿)。
    pub struct WinMmSoundPlayer;

    impl SoundPlayer for WinMmSoundPlayer {
        /// Java: `AudioSystem.getAudioInputStream(file)` + `getLine(info)` +
        /// `clip.open(audioStream)` 的合成; 读取/格式失败 → Err。
        /// AudioInputStream 的开关生命周期 (trait 文档注) 在本腿无对应物 —
        /// PlaySound 直接按路径播文件, open 时仅读头校验, 流即读即弃。
        fn open_clip(&self, path: &Path) -> Result<Box<dyn SoundClip>, SoundError> {
            let bytes = std::fs::read(path)?;
            let duration_secs = parse_wav_duration(&bytes)?;
            Ok(Box::new(WinMmClip {
                path_wide: path_as_wide(path),
                duration_secs,
                playing: AtomicBool::new(false),
                start_at: Mutex::new(None),
                closed: AtomicBool::new(false),
            }))
        }
    }

    /// javax.sound.sampled.Clip 的 winmm PlaySound 实现 (D7 SoundClip 平台腿)。
    ///
    /// PORT (PlaySound 语义边界, 平台件已知取舍):
    /// - 单通道: PlaySound 全进程同一时刻只播一路, 后 start() 打断前一路;
    ///   stop()/close() 的 PlaySound(NULL) 同为进程全局停而非仅本句柄
    ///   (Java stop/close 只作用于自身 line, 对非运行中的 line 是 no-op) —
    ///   已收窄为仅在本句柄 is_running() 时发送。判据不可用粘滞 playing:
    ///   自然播完不回写, 按它发送会使已播完 clip 的 close() 误杀他路刚
    ///   起播的新声 (消费端 reload() 对旧 clip 无条件 close 即该场景);
    ///   is_running = playing ∧ 未超时长, 严格窄于 playing, 恰好复刻
    ///   Java no-op 语义;
    ///   残余不可消除 (两路): (a) 时长近似下被 start 打断且未超自身时长窗
    ///   的 clip 仍 is_running=true, 其 stop/close 仍会误伤当前实际在播者;
    ///   (b) 跨 clip 无锁并发 — 判得 is_running=true 后、发出全局停前,
    ///   他路 start() 刚起播则被误杀 (消费端逐 alert 各持 Mutex, 不同
    ///   alert 锁不同, 窗口真实存在; 判据改 is_running 后已收窄到
    ///   is_running 为真的时段);
    ///   (Java 每 VoiceAlert 独立 Clip, 多告警可并发 — MainForm 试听按钮
    ///   (VoiceRowRenderer) 与告警并发播放即此取舍的实际受影响面;
    ///   PORTING §3 裁决 "语音是整文件播放, 够用" 即接受此限);
    /// - 无查询接口: isRunning 以 start 时刻 + WAV 时长近似 (自然播完判 false,
    ///   误差为轮询粒度; playOnce 的冷却节拍与之兼容);
    /// - 无音量面: master_gain_range() 恒 None → applyVolume 走
    ///   "Control not supported" 空 catch 分支 (§2.7 同款), 音量旋钮本腿暂失;
    /// - 异步文件生存期: PlaySound(NULL) 返回时 winmm 后台线程可能仍在读源
    ///   文件, drop 后立即删/覆盖源文件存在竞态 (install_pack 覆盖语音包
    ///   同理) — SND_ASYNC 路径固有, 已知取舍;
    /// - 每次播放天然从头: setFramePosition(0) 的回绕语义自动满足,
    ///   非零帧 seek 不支持 (全库调用点两处: VoiceWarning.playOnce 与
    ///   VoiceRowRenderer 试听按钮, 均只传 0)。
    pub struct WinMmClip {
        /// UTF-16 NUL 结尾路径 (PlaySoundW 直用; SND_ASYNC 下 winmm 后台线程
        /// 播放期间仍会读文件, 句柄存续期勿删源文件)
        path_wide: Vec<u16>,
        /// WAV 时长 (秒), is_running 的近似基准
        duration_secs: f64,
        /// start() 置 true; stop/close 置 false。自然播完不回写 — is_running()
        /// 按时长判 false; stop()/close() 的全局停发送判据用 is_running()
        /// 而非本标志 (粘滞值会误伤他路在播声音, 见 struct 文档"单通道"注)
        playing: AtomicBool,
        /// 本次 start 时刻 (is_running 时长判据)
        start_at: Mutex<Option<Instant>>,
        /// close 幂等标志 (Java line close 状态机: 已 close 再 close 无副作用)
        closed: AtomicBool,
    }

    impl WinMmClip {
        /// 停止异步播放的 MSDN 指定方式: PlaySound(pszSound=NULL)。
        /// (SND_PURGE 是 16 位遗留, Vista+ 不支持)
        fn winmm_stop() {
            unsafe {
                let _ = PlaySoundW(PCWSTR::null(), None, SND_FLAGS(0));
            }
        }
    }

    impl SoundClip for WinMmClip {
        /// Java: `clip.start()` — 非阻塞; SND_ASYNC 同为即发即忘。
        /// SND_NODEFAULT: 文件不可播时禁掉系统默认提示音 (Java 失败路径无声)。
        fn start(&self) {
            // playOnce 的 catch 吞掉 → debug 日志无声跳过; closed 守卫恢复该语义
            if self.closed.load(Ordering::SeqCst) {
                logger::debug("VoiceAlert", "PlaySoundW 播放失败: line closed");
                return;
            }
            let ok = unsafe {
                PlaySoundW(
                    PCWSTR::from_raw(self.path_wide.as_ptr()),
                    None,
                    SND_ASYNC | SND_FILENAME | SND_NODEFAULT,
                )
            };
            if ok.as_bool() {
                // 先锁写 start_at 再置 playing: 并发 is_running 不会读到
                // playing=true + start_at=None 的瞬态假阴性 (Java Clip 内部
                // 同步, start 返回后即恒 true)
                *self.start_at.lock().unwrap() = Some(Instant::now());
                self.playing.store(true, Ordering::SeqCst);
            } else {
                // ("播放失败: {key} - {msg}", key 属 VoiceAlert 层上下文,
                // clip 层不可得); trait 无 Result 返回面, 就地落同语义 debug
                // 日志, 拼 GetLastError 便于现场排障 (PlaySoundW 返回 BOOL,
                // winmm 不保证失败时设置 last error, 0/脏值仅作参考)
                let err = unsafe { windows::Win32::Foundation::GetLastError() };
                logger::debug(
                    "VoiceAlert",
                    &format!("PlaySoundW 播放失败: GetLastError={}", err.0),
                );
            }
        }

        /// Java: `clip.stop()` — 停止播放但行保留, 可再 start。
        fn stop(&self) {
            // PlaySound(NULL) 是进程全局停: 发送判据用 is_running() 而非粘滞
            // playing — Java stop() 对非运行中的 line 是 no-op, 绝不互扰;
            // playing 在自然播完后仍为 true, 按它发送会让"已播完 clip 的
            // stop"误伤他路正在播的声音 (is_running 严格窄于 playing,
            // 见 struct 文档"单通道"注)
            if self.is_running() {
                WinMmClip::winmm_stop();
            }
            self.playing.store(false, Ordering::SeqCst);
            *self.start_at.lock().unwrap() = None;
        }

        /// Java: `clip.isRunning()` — 播放进行中 true, 自然结束/停止后 false。
        /// PlaySound 无查询面: 以 start 时刻 + 时长近似; 过期后不回写 playing
        /// (无 &mut), 下次调用同样判 false, 语义等价。
        fn is_running(&self) -> bool {
            if !self.playing.load(Ordering::SeqCst) {
                return false;
            }
            match *self.start_at.lock().unwrap() {
                None => false,
                Some(t) => t.elapsed().as_secs_f64() < self.duration_secs,
            }
        }

        /// Java: `clip.setFramePosition(int)` — 见 struct 文档 (PlaySound 每次
        /// 从头播放, 全库两处调用点均传 0, 语义自动满足)。
        fn set_frame_position(&self, _frame: i32) {}

        /// Java: `clip.close()` — 释放行资源并停止播放; 幂等 (Drop 兜底再调)。
        fn close(&self) {
            if self.closed.swap(true, Ordering::SeqCst) {
                return;
            }
            // 同 stop(): 判据 is_running() — 消费端 reload() 对旧 clip 无条件
            // close (voice_warning.rs), 已自然播完的旧 clip 若按粘滞 playing
            // 发送全局停, 会切断他路刚起播的新告警 (B1 修复点)
            if self.is_running() {
                WinMmClip::winmm_stop();
            }
            self.playing.store(false, Ordering::SeqCst);
            *self.start_at.lock().unwrap() = None;
        }

        /// PlaySound 无每句柄增益控制 → None (Java FloatControl 缺失同款,
        /// applyVolume 的空 catch 分支吞掉, §2.7)。
        fn master_gain_range(&self) -> Option<(f32, f32)> {
            None
        }

        /// master_gain_range() 恒 None ⇒ applyVolume 不会调到这里; 空实现保留接口。
        fn set_master_gain(&self, _value: f32) {}
    }

    impl Drop for WinMmClip {
        fn drop(&mut self) {
            // RAII 兜底契约 (trait 文档): Drop 等价 close(), 显式 close 后幂等
            self.close();
        }
    }

    /// 路径 → UTF-16 NUL 结尾 (PCWSTR 要求; 非法单元照搬, Win32 路径语义)
    fn path_as_wide(path: &Path) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);
        wide
    }
}

#[cfg(target_os = "windows")]
pub use win::{WinMmClip, WinMmSoundPlayer, WindowsFocusDetector};

// =====================================================================
// Tests — Java 侧无独立测试; 按"公共项写边界测试"规则:
// DPI 纯腿/WAV 解析跨平台单测, Win32 腿以真实 API 冒烟 (win.rs 真实窗口
// 测试同款先例), 全部断言真实行为, 不做条件跳过。
// =====================================================================
#[cfg(test)]
mod tests;
