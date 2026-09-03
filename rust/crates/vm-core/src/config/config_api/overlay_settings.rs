//! 对应 Java: `src/prog/config/OverlaySettings.java`

/// Generalized interface for overlay settings.
/// Provides relative-to-absolute coordinate mapping and supports write-back for
/// dragging.
///
/// PORT: Java interface → Rust trait (§1 多实现接口, 实现 ConfigurationService.GenericOverlaySettingsImpl)。
pub trait OverlaySettings {

    /// PORT: Java `getGroupConfig()` 返回 `ConfigLoader.GroupConfig` — ConfigLoader
    /// 属 B 类尚未翻译, 以关联类型占位不引入前向依赖; ConfigurationService 实现
    /// 时指定 `type GroupConfig = crate::config::config_loader::GroupConfig`。
    type GroupConfig;

    /// Get absolute X coordinate in pixels.
    ///
    /// @param width Window width for centering fallback (if applicable)
    fn get_window_x(&self, width: i32) -> i32;

    /// Get absolute Y coordinate in pixels.
    ///
    /// @param height Window height for centering fallback (if applicable)
    fn get_window_y(&self, height: i32) -> i32;

    /// Save absolute pixel coordinates back to the relative coordinate system.
    ///
    /// PORT: Java 写方法 → &self (非 &mut): Java 实现是 ConfigurationService 的内部类
    /// 视图, 写回目标 gc.x/gc.y 位于共享的 layoutConfigs (ConfigurationService),
    /// 视图自身无独占状态; Rust 侧视图持共享句柄, 与 ConfigProvider::set_config 同方向
    /// (LIFETIMES §7 Arc<ConfigStore>), 写回由实现侧内部可变性完成。
    fn save_window_position(&self, x: f64, y: f64);

    /// Get the font name for this overlay.
    fn get_font_name(&self) -> String;

    /// Get the numeric font name for this overlay.
    fn get_num_font_name(&self) -> String;

    /// Get the font size adjustment for this overlay.
    fn get_font_size_add(&self) -> i32;

    /// Generic property getters
    ///
    /// PORT: Java 可空入参契约未声明, def 按非空 `&str` 处理; 返回 String (Java 返回引用, 值语义等价)。
    fn get_bool(&self, key: &str, def: bool) -> bool;

    fn get_int(&self, key: &str, def: i32) -> i32;

    fn get_string(&self, key: &str, def: &str) -> String;

    /// Get the underlying GroupConfig for advanced configuration access.
    ///
    /// PORT: Java 实现可返回 null (GenericOverlaySettingsImpl 找不到分组时) → Option;
    /// 借用仅当次有效 — Java 返回的是活对象引用 (EngineInfoConfig 保留该引用为字段,
    /// setConfig/import 后重读可见新值), Rust 借用无法跨 &self 长期持有, 有持有需求
    /// 的调用方须每次重取或存快照; 写回: 位置 x/y 走 save_window_position,
    /// 其余字段走 ConfigProvider::set_config。
    fn get_group_config(&self) -> Option<&Self::GroupConfig>;

    /// 获取是否启用游戏失焦时自动隐藏overlay功能。
    ///
    /// @return true如果启用自动隐藏
    fn auto_hide_on_focus_loss(&self) -> bool;
}

#[cfg(test)]
mod tests;
