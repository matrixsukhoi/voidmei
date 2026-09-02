//! 对应 Java 包: `ui.window.comparison.logic.rules` (一比一翻译)。
//! 内置规则实现: SimpleRule / ListIndexRule / MultiListIndexRule / LambdaRule。

pub mod lambda_rule;
pub mod list_index_rule;
pub mod multi_list_index_rule;
pub mod simple_rule;

pub use lambda_rule::LambdaRule;
pub use list_index_rule::ListIndexRule;
pub use multi_list_index_rule::MultiListIndexRule;
pub use simple_rule::SimpleRule;

// ---- java.util.regex 手写等价扫描器 ----
// PORT: SimpleRule / ListIndexRule / MultiListIndexRule 三个 Java 类各自重复定义
// 同一 NUMBER_PATTERN / LIST_PATTERN 常量 (逐字符相同); Rust 侧集中为本模块共享
// 函数, 匹配语义按 java.util.regex (ASCII \d、贪婪+回溯、leftmost-find) 复刻,
// parser/map_obj.rs 同款先例 (vm-core 依赖清单不含 regex crate)。
// comparison_rules.rs (SLASH_*) 亦复用 try_number_ends。
// 原注释归属 (PORTING.md §0.2 逐字保留): Java 三处 NUMBER_PATTERN 定义行上方
// 均无注释; LIST_PATTERN 各有一条, 见 find_bracket_list /
// find_all_bracket_lists 处。

use crate::telemetry::parser::char_len_at;
use crate::base::java_compat::java_trim;

/// Java 正则 `\d`: [0-9] (无 UNICODE_CHARACTER_CLASS 标志的 ASCII 定义)。
/// UTF-8 多字节字符的首/续字节均 ≥ 0x80, 不会被误判为数字。
fn is_ascii_digit(b: u8) -> bool {
    b.is_ascii_digit()
}

/// NUMBER_PATTERN `(-?\d+(\.\d+)?)` 在 pos 处的贪婪匹配尝试。
/// 返回 (int_end, full_end): int_end = 整数段末尾 (小数未吞), full_end = 整体末尾;
/// 两者相等表示无小数部分。pos 必须是字符边界。
///
/// 回溯无关性 (与 java.util.regex 结果一致的论证):
/// - `-?` 回退 (不吞 '-') 后 `\d+` 顶在 '-' 上必失败;
/// - `\d+` 让出尾位后, 下一个原子 (可选 `.` 段或后继) 顶在数字上必失败;
///
/// 故唯一有效的选择点是 `(\.\d+)?` 吞/不吞 —— 由调用方 (find_slash_both) 按贪婪
/// 优先序自行回溯。
pub(super) fn try_number_ends(s: &str, pos: usize) -> Option<(usize, usize)> {
    let b = s.as_bytes();
    let mut p = pos;
    if p < b.len() && b[p] == b'-' {
        p += 1;
    }
    let dstart = p;
    while p < b.len() && is_ascii_digit(b[p]) {
        p += 1;
    }
    if p == dstart {
        return None; // \d+ 至少 1 位
    }
    let int_end = p;
    let mut end = p;
    if b.get(p) == Some(&b'.') {
        let mut q = p + 1;
        while q < b.len() && is_ascii_digit(b[q]) {
            q += 1;
        }
        if q > p + 1 {
            end = q; // (\.\d+)? 贪婪: 点后至少 1 位才吞
        }
    }
    Some((int_end, end))
}

/// NUMBER_PATTERN `(-?\d+(\.\d+)?)` 的 `Matcher.find()` (首次匹配), 返回组1文本。
/// 组1 包裹整个模式, 故组1 = 整匹配文本。
pub(super) fn find_number(s: &str) -> Option<&str> {
    let mut i = 0usize;
    while i < s.len() {
        if let Some((_, end)) = try_number_ends(s, i) {
            return Some(&s[i..end]);
        }
        i += char_len_at(s, i);
    }
    None
}

// Matches numbers (including negative and decimal) within brackets
// (ListIndexRule.java LIST_PATTERN 常量原注释, 逐字保留 — PORTING.md §0.2)

/// LIST_PATTERN `\[([^\]]+)\]` 的 `Matcher.find()` (首次匹配), 返回组1 (括号内文本)。
/// `[^\]]` 贪婪到首个 ']' 或串尾; 至少 1 个非 ']' 字符且以 ']' 收尾才命中。
pub(super) fn find_bracket_list(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    let mut i = 0usize;
    while i < s.len() {
        if b[i] == b'[' {
            let mut j = i + 1;
            while j < s.len() && b[j] != b']' {
                j += char_len_at(s, j);
            }
            if j > i + 1 && j < s.len() && b[j] == b']' {
                return Some(&s[i + 1..j]);
            }
        }
        i += char_len_at(s, i);
    }
    None
}

// Matches each bracketed list
// (MultiListIndexRule.java LIST_PATTERN 常量原注释, 逐字保留 — PORTING.md §0.2)

/// LIST_PATTERN 的 `while (m.find())` 循环: 从左到右不重叠匹配, 收集全部组1。
pub(super) fn find_all_bracket_lists(s: &str) -> Vec<&str> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < s.len() {
        if b[i] == b'[' {
            let mut j = i + 1;
            while j < s.len() && b[j] != b']' {
                j += char_len_at(s, j);
            }
            if j > i + 1 && j < s.len() && b[j] == b']' {
                out.push(&s[i + 1..j]);
                i = j + 1; // 下一 find 从上次整匹配末尾起 (不重叠)
                continue;
            }
        }
        i += char_len_at(s, i);
    }
    out
}

/// Java `String.split(",")` (limit=0):
/// - 无 ',' 时原样返回 `[input]` (空串 → `[""]`, Java oracle 实测);
/// - 有 ',' 时移除尾部空段 (可全部移空, `",,,"` → 长度 0)。
///
/// ListIndexRule/MultiListIndexRule 的 `index < parts.length` 边界依赖此语义。
pub(super) fn java_split_comma(s: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 1 {
        return parts; // 无匹配 — Java "no match" 路径
    }
    while parts.last().map(|p| p.is_empty()).unwrap_or(false) {
        parts.pop();
    }
    parts
}

// Java String.trim 复刻收敛于 base::java_compat (本模块原字节版私有副本已删,
// 语义等价: ≤ U+0020 的字符在 UTF-8 中均为单字节, 按字符与按字节裁剪等价);
// 子模块经 `super::java_trim` 引用上述导入。

#[cfg(test)]
mod tests;
