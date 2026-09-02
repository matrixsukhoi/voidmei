//! 表单上下文契约 (Java: `RowRenderer.RenderContext` 嵌套接口, 唯一实现是
//! DynamicDataPage.java:126-175 的匿名类)。
//!
//! D9 后表单渲染归 vm-webui web 壳, 原 RowRenderer 策略接口与 RowRendererRegistry
//! 注册表 (占位翻译, 无生产消费者) 已随渲染责任移交退役。本文件只保留写链所需的
//! 上下文契约: 各渲染器 apply 写回链与 [`crate::renderer_config_helper`] 读写助手
//! 共用该单一类型 (生产实现 = `main_form::WriteContext`)。

/// Context object providing callbacks and state for rendering.
///
/// PORT: 方法取 `&self` (Java 实例方法语义; DynamicDataPage 的匿名实现经共享
/// 引用改外部状态, Rust 实现侧以内部可变性对位)。
pub trait RenderContext {
    /// Called when user changes a value and config should be saved
    fn on_save(&self);

    /// Syncs a boolean value to ConfigurationService (for overlay control)
    fn sync_to_config_service(&self, key: &str, value: bool);

    /// Gets a boolean value from ConfigurationService (for initial state)
    fn get_from_config_service(&self, key: &str, default_val: bool) -> bool;

    /// Syncs a string value to ConfigurationService
    fn sync_string_to_config_service(&self, key: &str, value: &str);

    /// Gets a string value from ConfigurationService (for initial state)
    fn get_string_from_config_service(&self, key: &str, default_val: &str) -> String;
}
