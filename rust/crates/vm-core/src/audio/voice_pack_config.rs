//! VoicePackConfig 的 Rust 移植 (src/prog/audio/VoicePackConfig.java)
//!
//! 语音包配置值对象，封装 "packName|enabled" 格式的解析和序列化
//!
//! 行为契约：
//! - parse("jarvis|true") → VoicePackConfig("jarvis", true)
//! - parse("jarvis") → VoicePackConfig("jarvis", true)  // 默认启用
//! - parse(null) → VoicePackConfig("default", true)
//! - parse("") → VoicePackConfig("default", true)
//! - toConfigString() → "packName|enabled"
//!
//! 这是一个不可变的值对象，线程安全。
//!
//! Java `final class` → Rust struct 天然无继承; pub final 字段 → pub 字段
//!。
//! Java String 入参可为 null → Option<&str>。
//! Java 手写 equals/hashCode → derive(PartialEq, Eq, Hash):
//! equals 语义逐字段一致 (enabled + packName 值相等); hashCode 的具体数值
//! (31*packName.hashCode()+...) 不可观测, 派生 Hash 保持 equals/hash 一致性契约。

use std::fmt;

/// 默认语音包名称
pub const DEFAULT_PACK: &str = "default";
/// 配置键前缀
pub const VOICE_PREFIX: &str = "voice_";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VoicePackConfig {
    /// 语音包名称
    pub pack_name: String,
    /// 是否启用
    pub enabled: bool,
}

/// 对应 Java `Boolean.parseBoolean(String)`:
/// 仅 "true" (忽略大小写) 为 true, 其余 (含 null/任意串) 一律 false。
// Java equalsIgnoreCase("true") — "true" 全 ASCII,
// eq_ignore_ascii_case 对任意 Unicode 输入行为一致。
fn parse_boolean(s: &str) -> bool {
    s.eq_ignore_ascii_case("true")
}

impl VoicePackConfig {
    /// 构造函数
    /// - `packName`: 语音包名称，null 或空字符串会被替换为 "default"
    /// - `enabled`: 是否启用
    pub fn new(pack_name: Option<&str>, enabled: bool) -> Self {
        let pack_name = match pack_name {
            Some(p) if !p.is_empty() => p.to_string(),
            _ => DEFAULT_PACK.to_string(),
        };
        VoicePackConfig { pack_name, enabled }
    }

    /// 解析配置字符串
    /// 格式: "packName|enabled" 或 "packName"
    ///
    /// - `configValue`: 配置值，可以为 null
    /// 返回: 解析后的配置对象
    pub fn parse(config_value: Option<&str>) -> VoicePackConfig {
        let mut pack_name = DEFAULT_PACK;
        let mut enabled = true;

        if let Some(v) = config_value {
            if !v.is_empty() {
                if v.contains('|') {
                    // (首个 '|' 处切一刀, 尾部空串保留) ↔ splitn(2, '|') 语义一致
                    let mut parts = v.splitn(2, '|');
                    pack_name = parts.next().unwrap(); // parts[0], splitn 必有首段
                    if let Some(part1) = parts.next() {
                        enabled = parse_boolean(part1);
                    }
                } else {
                    pack_name = v;
                }
            }
        }

        VoicePackConfig::new(Some(pack_name), enabled)
    }

    /// 序列化为配置字符串
    /// 返回: 格式: "packName|enabled"
    pub fn to_config_string(&self) -> String {
        format!("{}|{}", self.pack_name, self.enabled)
    }

    /// 创建启用状态变更后的新实例
    /// - `newEnabled`: 新的启用状态
    /// 返回: 新实例
    pub fn with_enabled(&self, new_enabled: bool) -> VoicePackConfig {
        VoicePackConfig::new(Some(&self.pack_name), new_enabled)
    }

    /// 创建包名变更后的新实例
    /// - `newPackName`: 新的包名
    /// 返回: 新实例
    pub fn with_pack_name(&self, new_pack_name: Option<&str>) -> VoicePackConfig {
        VoicePackConfig::new(new_pack_name, self.enabled)
    }

    /// 剥离 voice_ 前缀
    /// - `key`: 配置键
    /// 返回: 剥离前缀后的键
    // key 可为 null → Option, null 原样返回 None。
    pub fn strip_voice_prefix(key: Option<&str>) -> Option<String> {
        key.map(|k| {
            // starts_with 命中后 6 字节处必为 UTF-8 字符边界,
            // 字节切片与 Java UTF-16 码元切片等价。
            match k.strip_prefix(VOICE_PREFIX) {
                Some(rest) => rest.to_string(),
                None => k.to_string(),
            }
        })
    }

    /// 添加 voice_ 前缀
    /// - `key`: 告警键
    /// 返回: 带前缀的配置键
    pub fn with_voice_prefix(key: Option<&str>) -> Option<String> {
        key.map(|k| {
            if k.starts_with(VOICE_PREFIX) {
                k.to_string()
            } else {
                format!("{}{}", VOICE_PREFIX, k)
            }
        })
    }
}

/// 对应 Java toString() 覆写
impl fmt::Display for VoicePackConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VoicePackConfig{{packName='{}', enabled={}}}",
            self.pack_name, self.enabled
        )
    }
}

// =====================================================================
// Tests — 移植自 test/TestVoicePackConfig.java 的 VoicePackConfig 部分
// =====================================================================
#[cfg(test)]
mod tests;
