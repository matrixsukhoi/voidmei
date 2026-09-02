//! DPI 检测 — Java `src/prog/util/DPIHelper.java` (一比一翻译)。
//! 跨平台纯计算腿 (可测) + cfg(windows) Win32 检测腿。
//! 波16 自 extras.rs 按域拆出 (原三合一文件备案的拆分落地)。

use vm_core::base::format::java_round;
use vm_core::base::logger;

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
// Windows 平台腿: DPI 检测 (Win32)
// =====================================================================
#[cfg(target_os = "windows")]
mod win {
    use windows::Win32::Foundation::{HWND, POINT};
    use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::HiDpi::{
        GetDpiForMonitor, GetDpiForWindow, SetProcessDpiAwarenessContext,
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, MDT_EFFECTIVE_DPI,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    use super::DpiHelper;

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
}

// Tests — DPI 纯腿跨平台单测, Win32 腿以真实 API 冒烟 (win.rs 真实窗口
// 测试同款先例), 全部断言真实行为, 不做条件跳过。
#[cfg(test)]
mod tests;
