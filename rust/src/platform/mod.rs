//! 平台窗口抽象: 透明/置顶/穿透 overlay 窗口的跨平台接口
//! Windows = UpdateLayeredWindow, Linux = X11 depth-32 visual

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
pub enum OverlayEvent {
    Close,
    MousePress { root_x: i32, root_y: i32 },
    MouseMove { root_x: i32, root_y: i32, left_down: bool },
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
