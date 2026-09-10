//! prog.audio 的 A 类部分 (P2 批二): 语音告警类型枚举 + 语音包配置值对象。
//! 同包其余两文件 (VoiceResourceManager/VoiceWarning) 属 B 类后续批次, 不在本模块。
//! 类型顶层 re-export 镜像 Java `import prog.audio.VoicePackConfig` 的扁平引用
//! (event/mod.rs 先例)。

pub mod voice_alert_type;
pub mod voice_pack_config;
// 重构波2 吸收: 原顶层 voice_warning / voice_resource_manager 归入本域
pub mod voice_resource_manager;
pub mod voice_warning;

pub use voice_alert_type::VoiceAlertType;
pub use voice_pack_config::VoicePackConfig;
