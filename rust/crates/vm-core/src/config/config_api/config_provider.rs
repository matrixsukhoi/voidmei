//! 对应 Java: `src/prog/config/ConfigProvider.java`

/// Interface for configuration access.
/// Abstracts away the specific configuration storage mechanism.
///
/// Java interface → Rust trait。
pub trait ConfigProvider {
    /// Get a configuration value by key.
    ///
    /// - `key`: Configuration key
    /// 返回: Value or null/empty if not set
    ///
    /// Java 可空 String 返回 → Option<String>; 契约允许 null(None) 与
    /// 空串(Some("")) 两种"未设置"形态, 调用方需一并处理 (Java 端 ConfigurationService
    /// 实现固定返回空串, 但接口契约按源文件保留两种)。
    fn get_config(&self, key: &str) -> Option<String>;

    /// Set a configuration value.
    ///
    /// - `key`: Configuration key
    /// - `value`: Value to set
    ///
    /// Java 写方法 → &self (非 &mut)。Java 中 ConfigurationService 单实例
    /// implements ConfigProvider, 经 getConfigProvider() 以共享引用分发给全部消费方
    ///;
    /// 且 setConfig 在变更中途同步 publish CONFIG_CHANGED (ConfigurationService),
    /// UIStateBus.publish 内联执行 handler (UIStateBus) 会重入读配置 —
    /// 调用侧包 Mutex/RefCell 的 &mut 方案在同线程重入下死锁/panic,
    /// 故共享与重入安全由实现侧内部可变性承担: 短锁改值、放锁后再广播。
    fn set_config(&self, key: &str, value: &str);

    /// Check if a field is disabled by configuration.
    ///
    /// - `key`: Configuration key
    /// 返回: true if disabled, false if enabled
    fn is_field_disabled(&self, key: &str) -> bool;
}

#[cfg(test)]
mod tests;
