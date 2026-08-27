//! prog.config 依赖桩 —— **不是** prog.config 的翻译。
//!
//! `crate::config_api` 是 P2 批二占位空模块, 而 ui.model 的
//! DefaultFieldManager / FlightInfoConfig / EngineInfoConfig 依赖
//! `prog.config.ConfigProvider` 与 `prog.config.ConfigLoader` 的内部类
//! GroupConfig / RowConfig。按 `fm::handle::BlkxPlaceholder` 先例, 此处以
//! "ui.model 消费面子集"桩顶住编译期类型缺口。
//!
//! TODO(port): config_api 批次落地后删除本文件, 引用切换到
//! `crate::config_api` (trait 实现归属、RowConfig 全字段含 `Object value`
//! 的类型选型由该批次裁决 —— 本桩刻意只含消费面字段, 不预判)。

/// Interface for configuration access.
/// Abstracts away the specific configuration storage mechanism.
/// (对应 Java `prog.config.ConfigProvider` 接口的消费面签名)
// PORT: Java getConfig 返回 null 表示未设置 → Option<String>; setConfig 变更
// 状态 → &mut self (Java 隐式 this 可变)。
pub trait ConfigProvider {
    /// Get a configuration value by key.
    ///
    /// @param key Configuration key
    /// @return Value or null/empty if not set
    fn get_config(&self, key: &str) -> Option<String>;

    /// Set a configuration value.
    ///
    /// @param key   Configuration key
    /// @param value Value to set
    fn set_config(&mut self, key: &str, value: &str);

    /// Check if a field is disabled by configuration.
    ///
    /// @param key Configuration key
    /// @return true if disabled, false if enabled
    fn is_field_disabled(&self, key: &str) -> bool;
}

/// `prog.config.ConfigLoader.RowConfig` 消费面子集桩。
/// 字段与默认值取 Java 声明原样 (仅保留 ui.model 读取的 8 个字段)。
/// Java RowConfig 共 23 个声明字段, 以下 15 个不在本桩: formula/format (构造器
/// 参数, 反射路径用)/value/defaultValue/fgColor/desc/descImg/precision/
/// unitSource/precisionSource/visibleWhen/naWhen (SExp)/minVal/maxVal/
/// groupColumns —— config_api 批次落地时消费方须整体切换到全字段版本,
/// 遗漏即静默语义漂移 (如 naWhen 丢失 → DataField.na_when_evaluator 装配链断)。
pub struct RowConfig {
    pub label: String,
    /// Display name for overlay if different from label (Java 默认 null)
    pub target_name: Option<String>,
    /// Unit string (e.g., "Hp") — Java 默认 "" (非 null)
    pub unit: String,
    /// Default value for UI preview/placeholder (Java 默认 null)
    pub preview_value: Option<String>,
    /// Hide if value is zero (Java 默认 false)
    pub hide_when_zero: bool,
    /// DATA, HEADER, SLIDER, COMBO, SWITCH, BUTTON (Java 默认 "DATA";
    /// 字段名 `type` 是 Rust 关键字, r#type 保名)
    pub r#type: String,
    /// Bound GroupConfig property (e.g., "fontSize") (Java 默认 null)
    pub property: Option<String>,
    /// 子行 (Java 默认 `new ArrayList<>()`, 非 null)
    pub children: Vec<RowConfig>,
}

impl RowConfig {
    /// Java 构造器 `RowConfig(label, formula, format)` 中 ui.model 消费面只读 label;
    /// formula/format 参数被丢弃 (见结构体注释的字段清单), 桩构造器只收 label。
    pub fn new(label: &str) -> Self {
        RowConfig {
            label: label.to_string(),
            target_name: None,
            unit: String::new(),
            preview_value: None,
            hide_when_zero: false,
            r#type: "DATA".to_string(),
            property: None,
            children: Vec::new(),
        }
    }
}

/// `prog.config.ConfigLoader.GroupConfig` 消费面子集桩 (title + rows;
/// x/y/alpha/hotkey 等字段未被 ui.model 消费, 不在本桩)。
pub struct GroupConfig {
    pub title: String,
    pub rows: Vec<RowConfig>,
}

impl GroupConfig {
    /// Java 构造器 `GroupConfig(title)`, 其余字段走声明默认值。
    pub fn new(title: &str) -> Self {
        GroupConfig {
            title: title.to_string(),
            rows: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests;
