//! 平台域 (波10 分域): 跨平台窗口抽象 (win/x11) + 宿主 (host) + 托盘 (tray)
//! + 热键 (hotkey) + WYSIWYG reinit 参数包 (reinit) + DPI 检测 (dpi) + 前台焦点
//! 检测 (focus) + winmm 声音播放 (sound; 原三合一 extras 波16 按域拆出)。
//! 窗口坐标持久化经 host 的 PositionStore trait (组装层注入)。

pub struct WindowConfig {
    pub width: i32,
    pub height: i32,
    /// 初始位置 (物理像素)
    pub x: i32,
    pub y: i32,
    /// 鼠标穿透 (游戏模式); preview 模式 false 可拖拽
    pub click_through: bool,
}

/// 主循环消费的事件 (拖拽状态机输入)
/// derive PartialEq: 事件分流单测断言用 (Java 无对应, Rust 测试面)
#[derive(Debug, PartialEq)]
pub enum OverlayEvent {
    Close,
    MousePress {
        root_x: i32,
        root_y: i32,
    },
    MouseMove {
        root_x: i32,
        root_y: i32,
        left_down: bool,
    },
    MouseRelease,
}

pub trait OverlayWindow {
    /// 提交预乘 BGRA 缓冲 (len = w*h*4, 行主序)
    fn present(&mut self, buf: &[u8]) -> Result<(), String>;
    fn set_position(&mut self, x: i32, y: i32);
    fn position(&self) -> (i32, i32);
    /// 运行时切换穿透 (预留: 目前创建时按模式一次定型)
    #[allow(dead_code)]
    fn set_click_through(&mut self, on: bool);
    /// 运行时切换置顶 (Java Window.setAlwaysOnTop — AlwaysOnTopCoordinator
    /// suspendAll/restoreAll 的底层动作; POC 全窗口恒 TOPMOST, 无对话框阶段不会被调用)
    fn set_topmost(&mut self, _on: bool) {}
    /// 运行时切换可见性 (Java Window.setVisible — AlwaysOnTopCoordinator
    /// hideAllOverlays/showAllOverlays (FocusMonitor 游戏失焦自动隐藏) 的底层动作;
    /// Java isDisplayable 守卫由所有权天然保证: 槽位存在 = 窗口未销毁,
    /// 已销毁窗口不存在"复活"路径 — 僵尸窗口防护)
    fn set_visible(&mut self, _visible: bool) {}
    /// 运行时改窗口尺寸 (Java Window.setSize/setBounds — WYSIWYG
    /// reinitConfig 重算布局后 setBounds 的底层动作; x11 波次前缺省空实现)
    fn set_size(&mut self, _w: i32, _h: i32) {}
    /// 非阻塞取事件, 无事件返回 None
    fn poll_event(&mut self) -> Option<OverlayEvent>;
    /// 屏幕物理尺寸 (位置归一化用)
    fn screen_size(&self) -> (i32, i32);
}

#[cfg(target_os = "windows")]
mod win;

#[cfg(not(target_os = "windows"))]
mod x11;

#[cfg(target_os = "windows")]
pub use win::create;

#[cfg(not(target_os = "windows"))]
pub use x11::create;

// ---- 波10 迁入的域成员 (原顶层平铺) ----
pub mod dpi;
pub mod focus;
pub mod host;
pub mod hotkey;
pub mod reinit;
#[cfg(target_os = "windows")]
pub mod tray;
