//! 颜色串解析 (ColorHelper 移植, src/prog/util/ColorHelper.java): hex
//! `#RRGGBB` / `#RRGGBBAA` 与十进制 `"R, G, B[, A]"` 双格式, 失败回落默认色
//! (cfg `:value` 为 hex, 用户编辑后存十进制 — 双格式互通的原因)。
//!
//! 重构波13 收敛点: vm-ui/renderers/color.rs 与 config/configuration_service
//! 的两份同族实现归一至此。以 vm-ui 侧实现为准 (Java 8 oracle 测试锚定,
//! 2026-08-26 对拍用例现锚于 vm-ui renderers/color tests 与 configuration_service
//! tests 的 color_parse_matrix); 两份实现比对语义等价, 差异仅在 Option 形态
//! (vm-core 旧版直接回落默认) 与切片写法, 无行为分歧。
//!
//! 边缘语义:
//! - 外层 trim = Java `String.trim` (<= U+0020, 不含 NBSP/U+3000) → java_compat;
//! - 内部空白剔除 = Java `replaceAll("\\s+")` 字符集 [ \t\n\x0B\f\r];
//! - `split(",")` 尾部空串丢弃; 越界值钳位 [0,255] 不回默认。

use crate::base::java_compat::java_trim;

/// Java `ColorHelper.parseColor(String, Color)`: 双格式解析, 失败回落 default。
pub fn parse_color(text: &str, default: [u8; 4]) -> [u8; 4] {
    try_parse_color(text).unwrap_or(default)
}

/// 解析的 Option 形态 (None = 非法色串)。
pub fn try_parse_color(text: &str) -> Option<[u8; 4]> {
    let trimmed = java_trim(text);
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('#') {
        parse_hex_color(trimmed)
    } else {
        parse_decimal_color(trimmed)
    }
}

/// Java `parseHexColor`: "#RRGGBB" (alpha=255) / "#RRGGBBAA";
/// 其他长度/非法数字落穿 → None。
fn parse_hex_color(hex: &str) -> Option<[u8; 4]> {
    let h = &hex[1..]; // '#' 恒 ASCII 单字节, [1..] 是合法切点
    // 非 ASCII 多字节字符在 Java parseInt 必失败 → None (顺带规避切包边界 panic)
    if !h.is_ascii() {
        return None;
    }
    let b = h.as_bytes();
    let byte = |r: std::ops::Range<usize>| u8::from_str_radix(&h[r], 16).ok();
    match b.len() {
        6 => Some([byte(0..2)?, byte(2..4)?, byte(4..6)?, 255]),
        8 => Some([byte(0..2)?, byte(2..4)?, byte(4..6)?, byte(6..8)?]),
        _ => None,
    }
}

/// Java `parseDecimalColor`: "R, G, B[, A]", 全空白剔除 + 钳位 [0,255]。
fn parse_decimal_color(decimal: &str) -> Option<[u8; 4]> {
    // replaceAll("\\s+", "") — 内部空白一并剔除
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
    // Java parseInt: 十进制, 非法串抛异常 → None
    let r = parts[0].parse::<i32>().ok()?;
    let g = parts[1].parse::<i32>().ok()?;
    let b = parts[2].parse::<i32>().ok()?;
    let a = if parts.len() >= 4 { parts[3].parse::<i32>().ok()? } else { 255 };
    Some([clamp_u8(r), clamp_u8(g), clamp_u8(b), clamp_u8(a)])
}

/// Java `replaceAll("\\s+")` 的字符集 [ \t\n\x0B\f\r]。
/// PORT: Rust is_ascii_whitespace 不含 \x0B, 显式对齐 (oracle vt-internal 用例)
fn is_java_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{000B}' | '\u{000C}' | '\r')
}

/// Java `clamp`
fn clamp_u8(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}
