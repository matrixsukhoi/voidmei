//! `prog.i18n` 包的 Rust 移植 (src/prog/i18n/Lang.java)。
//!
//! PORT: Java 经 `java.util.Properties` 运行时加载 `lang/cur.properties`;
//! 本移植固化为静态表 [`table::LANGUAGE_PROPERTIES`] (键值 = Java 加载后的实际值,
//! Java 8 oracle 实测生成), 字符串字段为 `&'static str`。

// PORT: Java 保真 — 模块名 lang 对应 Java 类名前缀 (Lang/lang 同名嵌套),
// 重命名会破坏与 Java 源的路径对应
#[allow(clippy::module_inception)]
pub mod lang;
pub mod table;

pub use lang::{Config, Lang};
