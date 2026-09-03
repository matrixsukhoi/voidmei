//! 对应 Java: `src/prog/config/ConfigProvider.java`

/// Interface for configuration access.
/// Abstracts away the specific configuration storage mechanism.
///
/// PORT: Java interface → Rust trait (§1 多实现接口, 主要实现 ConfigurationService)。
pub trait ConfigProvider {

    /// Get a configuration value by key.
    ///
    /// @param key Configuration key
    /// @return Value or null/empty if not set
    ///
    /// PORT: Java 可空 String 返回 → Option<String>; 契约允许 null(None) 与
    /// 空串(Some("")) 两种"未设置"形态, 调用方需一并处理 (Java 端 ConfigurationService
    /// 实现固定返回空串, 但接口契约按源文件保留两种)。
    fn get_config(&self, key: &str) -> Option<String>;

    /// Set a configuration value.
    ///
    /// @param key   Configuration key
    /// @param value Value to set
    ///
    /// PORT: Java 写方法 → &self (非 &mut)。Java 中 ConfigurationService 单实例
    /// implements ConfigProvider, 经 getConfigProvider() 以共享引用分发给全部消费方
    /// (LIFETIMES §7 目标形态 `config: Arc<ConfigStore>` + 内部 RwLock — Arc 只能给 `&`);
    /// 且 setConfig 在变更中途同步 publish CONFIG_CHANGED (ConfigurationService),
    /// UIStateBus.publish 内联执行 handler (UIStateBus) 会重入读配置 —
    /// 调用侧包 Mutex/RefCell 的 &mut 方案在同线程重入下死锁/panic (§2.8),
    /// 故共享与重入安全由实现侧内部可变性承担: 短锁改值、放锁后再广播。
    fn set_config(&self, key: &str, value: &str);

    /// Check if a field is disabled by configuration.
    ///
    /// @param key Configuration key
    /// @return true if disabled, false if enabled
    fn is_field_disabled(&self, key: &str) -> bool;
}

#[cfg(test)]
mod tests;
