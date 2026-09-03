//! 对应 Java 包: `ui.window.comparison.logic.rules`。
//! 内置规则实现: SimpleRule / ListIndexRule / MultiListIndexRule / LambdaRule。

pub mod lambda_rule;
pub mod list_index_rule;
pub mod multi_list_index_rule;
pub mod simple_rule;

pub use lambda_rule::LambdaRule;
pub use list_index_rule::ListIndexRule;
pub use multi_list_index_rule::MultiListIndexRule;
pub use simple_rule::SimpleRule;

// ---- java.util.regex 模式 (regex crate) ----
// 波21: 原手写等价扫描器 (迁移期 "无权改 Cargo.toml" 的产物) 退役, 模式
// 原样落 regex crate。Java SimpleRule / ListIndexRule / MultiListIndexRule
// 三类各自重复定义同一常量, Rust 侧集中于此。
// 字符类显式 ASCII ([0-9] 而非 \d): 对齐 java.util.regex 无
// UNICODE_CHARACTER_CLASS 标志的语义 (regex 默认 \d 是 Unicode 数字)。

use std::sync::LazyLock;

use regex::Regex;

/// Java 正则 `\s`: [ \t\n\x0B\x0C\r] (ASCII 定义)
pub(crate) const JAVA_WS: &str = r" \t\n\x0B\x0C\r";

/// NUMBER_PATTERN `(-?\d+(\.\d+)?)` — find() 首次匹配的组1。
pub(super) fn find_number(s: &str) -> Option<&str> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(-?[0-9]+(\.[0-9]+)?)").unwrap());
    RE.captures(s).map(|c| c.get(1).unwrap().as_str())
}

// Matches numbers (including negative and decimal) within brackets
// (ListIndexRule.java LIST_PATTERN 常量原注释, 逐字保留 — PORTING.md §0.2)

/// LIST_PATTERN `\[([^\]]+)\]` 的 find() 首次匹配, 返回组1 (括号内文本)。
pub(super) fn find_bracket_list(s: &str) -> Option<&str> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]+)\]").unwrap());
    RE.captures(s).map(|c| c.get(1).unwrap().as_str())
}

// Matches each bracketed list
// (MultiListIndexRule.java LIST_PATTERN 常量原注释, 逐字保留 — PORTING.md §0.2)

/// LIST_PATTERN 的 `while (m.find())` 循环: captures_iter 不重叠, 收集全部组1。
pub(super) fn find_all_bracket_lists(s: &str) -> Vec<&str> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]+)\]").unwrap());
    RE.captures_iter(s)
        .map(|c| c.get(1).unwrap().as_str())
        .collect()
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

// Java String.trim 复刻收敛于 base::java_compat; 子模块经 `super::java_trim` 引用
// (父模块私有 use 对子模块可见)。
use crate::base::java_compat::java_trim;

#[cfg(test)]
mod tests;
