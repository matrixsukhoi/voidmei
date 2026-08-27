//! ColorRowRenderer 的写回链语义复刻 (src/ui/layout/renderer/ColorRowRenderer.java)
//! + ColorHelper 的解析/格式化移植 (src/prog/util/ColorHelper.java, vm-core 未译,
//! 就近落地本文件供 color_picker 共用)。
//!
//! **D9 变更**: 原 iced view_row/swatch 已删 (渲染归 vm-webui web 壳), 本模块仅存
//! ColorHelper 解析/格式化 + 读链 (read_current) + 写链 (apply)。
//!
//! ColorHelper 语义 (边缘行为经 Java 8 oracle 对拍, 2026-08-26, 用例值见 tests):
//! - parse_color: hex (#RRGGBB / #RRGGBBAA) 与十进制 ("R, G, B[, A]") 双格式,
//!   失败回落默认色 (cfg :value 为 hex, 用户编辑后存十进制 — 双格式互通的原因)。
//! - to_decimal_string: 配置存储格式 (向后兼容); to_hex_string: 显示格式 (大写)。
//!
//! 交互语义保真 (Java L30-136):
//! - 读: ctx.getStringFromConfigService(key, "255, 255, 255, 255") — 直读服务,
//!   无 PropertyBinder 分支 (ColorRowRenderer.java:34)。
//! - 写 (apply): 主键存十进制 (Java L124) + legacy 分键 keyR/G/B/A (Java L127-130,
//!   全库无读取方, 保真写入) + row.value=十进制串 (L133) + onSave (L135)。
//!
//! PORT(提交时机备案): Java hex 输入 Enter/失焦提交 (L55-63); D1 期 iced 无失焦
//! 消息 → 仅在解析出合法完整色串时发 ColorPicked (部分输入静默)。D9 后提交时机
//! 归 web 壳 (JS 输入框), Message::ColorPicked 消息形状不变。

use vm_core::config_loader::{ConfigValue, GroupConfig, RowConfig};
use vm_core::row_renderer_registry::RenderContext;

use super::{find_row_path, row_by_path, row_by_path_mut};

/// Java Color.WHITE (ColorRowRenderer.java:35 解析回落的默认白)
pub const WHITE: [u8; 4] = [255, 255, 255, 255];

/// Java ColorHelper.isHexFormat (L160-162): trim 后以 '#' 开头。
/// PORT(dead_code): ColorHelper 全量移植的 API 面 (Java 侧同样无生产调用方)。
#[allow(dead_code)]
pub fn is_hex_format(text: &str) -> bool {
    java_trim(text).starts_with('#')
}

/// Java String.trim(): 去两端码点 <= U+0020 的字符。
/// PORT: Rust str::trim 是 Unicode 空白集, 对 nbsp/U+3000 会多删 (oracle nbsp-hex
/// 用例: Java 不删 → 十进制路径解析失败 → 默认色), 必须按 Java 语义实现。
/// crate 内共享: button.rs 的 :fgcolor 逐段 trim 同为 Java String.trim 语义。
pub(crate) fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c <= '\u{0020}')
}

/// Java replaceAll("\\s+") 的字符集 [ \t\n\x0B\f\r]。
/// PORT: Rust is_ascii_whitespace 不含 \x0B, 显式对齐 (oracle vt-internal 用例)
fn is_java_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{000B}' | '\u{000C}' | '\r')
}

/// Java ColorHelper.parseColor (L41-55): 双格式解析, 失败回落 default。
pub fn parse_color(text: &str, default: [u8; 4]) -> [u8; 4] {
    try_parse_color(text).unwrap_or(default)
}

/// 解析的 Option 形态 (hex 输入框的"合法完整色串"门控用, 见 view_row)。
pub fn try_parse_color(text: &str) -> Option<[u8; 4]> {
    let trimmed = java_trim(text);
    if trimmed.is_empty() {
        return None; // Java L42-44: null/trim 后空 → default
    }
    if trimmed.starts_with('#') {
        parse_hex_color(trimmed)
    } else {
        parse_decimal_color(trimmed)
    }
}

/// Java parseHexColor (L64-86): "#RRGGBB" (alpha=255) / "#RRGGBBAA";
/// 其他长度/非法数字落穿 → default。
fn parse_hex_color(hex: &str) -> Option<[u8; 4]> {
    let h = &hex[1..]; // '#' 恒 ASCII 单字节, [1..] 是合法切点
    // 非 ASCII 多字节字符在 Java parseInt 必失败 → default (顺带规避切包边界 panic)
    if !h.is_ascii() {
        return None;
    }
    let b = h.as_bytes();
    let byte = |r: std::ops::Range<usize>| u8::from_str_radix(&h[r], 16).ok();
    match b.len() {
        // Java: new Color(r, g, b) — 2 位十六进制恒 0-255, 无需钳位
        6 => Some([byte(0..2)?, byte(2..4)?, byte(4..6)?, 255]),
        8 => Some([byte(0..2)?, byte(2..4)?, byte(4..6)?, byte(6..8)?]),
        _ => None,
    }
}

/// Java parseDecimalColor (L95-118): "R, G, B[, A]", 全空白剔除 + 钳位 [0,255]。
fn parse_decimal_color(decimal: &str) -> Option<[u8; 4]> {
    // Java L97: replaceAll("\\s+", "") — 内部空白一并剔除
    let cleaned: String = decimal.chars().filter(|c| !is_java_ws(*c)).collect();
    // Java String.split(","): 尾部空串丢弃 (oracle "255, 85, 0," → 3 段 → a=255;
    // Rust split 原样保留 → 需模拟, 否则尾部逗号串解析失败偏离 Java)
    let mut parts: Vec<&str> = cleaned.split(',').collect();
    while parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    if parts.len() < 3 {
        return None;
    }
    // Java parseInt: 十进制, 非法串抛异常 → default
    let r = parts[0].parse::<i32>().ok()?;
    let g = parts[1].parse::<i32>().ok()?;
    let b = parts[2].parse::<i32>().ok()?;
    let a = if parts.len() >= 4 { parts[3].parse::<i32>().ok()? } else { 255 };
    Some([clamp_u8(r), clamp_u8(g), clamp_u8(b), clamp_u8(a)])
}

/// Java clamp (L167-169)
fn clamp_u8(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// Java toDecimalString (L126-131): 存储格式 "R, G, B, A"。
/// PORT: Java null 分支 ("255, 255, 255, 255") 在 Rust 类型下不可达。
pub fn to_decimal_string(c: &[u8; 4]) -> String {
    format!("{}, {}, {}, {}", c[0], c[1], c[2], c[3])
}

/// Java toHexString (L140-152): 显示格式, 大写十六进制 ("%02X")。
pub fn to_hex_string(c: &[u8; 4], include_alpha: bool) -> String {
    if include_alpha {
        format!("#{:02X}{:02X}{:02X}{:02X}", c[0], c[1], c[2], c[3])
    } else {
        format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
    }
}

/// 读链 (Java L33-35): 服务串 (cfg 内 hex / 编辑后十进制) 双格式解析,
/// 缺省 "255, 255, 255, 255"。
pub fn read_current(row: &RowConfig, ctx: &dyn RenderContext) -> [u8; 4] {
    match row.property.as_deref() {
        Some(key) => parse_color(&ctx.get_string_from_config_service(key, "255, 255, 255, 255"), WHITE),
        // Java key=null → getConfig 对 null key NPE (域内不可达: COLOR 行恒带
        // :target), 折叠为行值解析 — 与 switch.rs 的同类折叠一致
        None => parse_color(&row.get_str(), WHITE),
    }
}

/// 颜色变更写回 (Java applyColorChange L110-136)。经 main_form::update 的
/// ColorPicked 臂接线 (with_panel 模式, 与 switch/slider/combo 同构)。
pub fn apply(panel: &mut GroupConfig, key: &str, rgba: [u8; 4], ctx: &dyn RenderContext) {
    let Some(path) = find_row_path(&panel.rows, key) else {
        return;
    };
    let prop = row_by_path(&panel.rows, &path)
        .expect("find_row_path 已定位")
        .property
        .clone();
    let unified = to_decimal_string(&rgba);
    if let Some(p) = prop.as_deref() {
        // Java L124: 主键十进制存储 (向后兼容)
        ctx.sync_string_to_config_service(p, &unified);
        // Java L127-130: legacy 分键 (拆通道写; 全库无读取方, 纯兼容面, 保真写入)
        ctx.sync_string_to_config_service(&format!("{p}R"), &rgba[0].to_string());
        ctx.sync_string_to_config_service(&format!("{p}G"), &rgba[1].to_string());
        ctx.sync_string_to_config_service(&format!("{p}B"), &rgba[2].to_string());
        ctx.sync_string_to_config_service(&format!("{p}A"), &rgba[3].to_string());
    }
    // Java L133: row.value = unified (内存模型)
    row_by_path_mut(&mut panel.rows, &path)
        .expect("find_row_path 已定位")
        .value = Some(ConfigValue::Str(unified));
    // Java L135: onSave
    ctx.on_save();
}

// =====================================================================
// Tests — ColorHelper 边缘语义全部取自 Java 8 oracle 对拍 (默认色 1,2,3,4)
// =====================================================================
#[cfg(test)]
mod tests;
