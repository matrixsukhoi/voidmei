//! 数字显示格式化。
//!
//! 波21 显示引擎退役: 原 FastNumberFormatter + Java printf 语义复刻
//! (HALF_UP 舍入/最短十进制串展开/printf 宽度) → Rust `format!` 原生语义。
//! 行为差异备案 (用户裁决接受): x.xx5 边界从 Java HALF_UP 变 Rust
//! nearest-even (2.675 → "2.67" 而非 "2.68"), 显示末位偶发漂移;
//! ±∞ 输出 "inf" 而非 "Infinity" (HUD 值域不可达)。
//! 保留的域契约: NaN → "N/A" (UI 文案), 负零抑制 (-0.04 → "0.0")。
//!
//! java_printf 子模块保留: Lang i18n 模板域 (数据驱动的格式串, 非显示引擎)。
//! java_round 族保留: 像素几何舍入 (floor(x+0.5), 与显示格式化不同脉)。

mod java_printf;

pub use java_printf::{java_string_format, FmtArg};

/// Java Math.round(double) = floor(x + 0.5) — 像素几何域舍入
/// (与显示格式化不同脉: 该语义服务于渲染布局, 非 printf 舍入)
pub fn java_round(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

/// Java Math.round(float)→int 窄版 (重构波3: 全仓 12 处私有副本收敛于此)
pub fn java_round_f32(x: f32) -> i32 {
    (x + 0.5).floor() as i32
}

/// Java Math.round(double) 经 (int) 窄化的常用形态
pub fn java_round_f64(x: f64) -> i32 {
    (x + 0.5).floor() as i32
}

/// 数值格式化 (原 FastNumberFormatter 的域契约保留版):
/// - NaN → "N/A" (UI 缺数据文案);
/// - 负零抑制: 舍入到零的负数不显示负号 (-0.04 两位精度 → "0.0");
/// - 其余 = Rust `{:.prec}` 语义。
pub fn format(value: f64, precision: u8) -> String {
    if value.is_nan() {
        return "N/A".to_string();
    }
    let mut s = format!("{:.*}", precision as usize, value);
    // 负零抑制: "-0"/"-0.0"/"-0.00" 形态剥负号
    if s.starts_with('-') && s.trim_start_matches(['-', '0', '.']).is_empty() {
        s.remove(0);
    }
    s
}

/// 定点小数显示 (原 java_f 的 Rust 语义版, 波21 更名):
/// Rust `{:.prec}` nearest-even 舍入 (HALF_UP 复刻退役), NaN/±∞ 原生输出。
/// 保留为函数是显示语义单点 (HUD 全部数字列的格式化出口)。
pub fn fmt_f(d: f64, prec: usize) -> String {
    format!("{:.*}", prec, d)
}

/// printf 宽度语义的薄包装: 不足补空格 (默认右对齐), 超宽不截断。
/// (波21: 原 Java printf 宽度实现退役, Rust format! 宽度同语义)
pub fn pad_width(s: String, width: usize, left_align: bool) -> String {
    if left_align {
        format!("{s:<width$}")
    } else {
        format!("{s:>width$}")
    }
}

/// `String.format("%0Nd", v)` 的薄包装: Rust `{:0N}` 即符号感知零填充。
pub fn java_d0(v: i64, width: usize) -> String {
    format!("{v:0width$}")
}

/// `String.format("%+.Nf", d)`: 非负值强制带 '+'。
/// NaN 特判保留 (Rust `{:+}` 会输出 "+NaN", 显示文案用 "NaN")。
pub fn java_f_plus(d: f64, prec: usize) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    format!("{d:+.prec$}")
}

/// `String.format("%.0f", d)` 的整数化: Rust `{:.0}` nearest-even 舍入
/// (原精确二进制 HALF_UP 两脉并存退役), 负零保留, NaN/±∞ 原生输出。
pub fn java_f0_exact(d: f64) -> String {
    format!("{d:.0}")
}

/// MM'SS 时间格式化 (power_info 油温耐热时间; 分钟 999 封顶)
pub fn format_time(value: f64) -> String {
    if value.is_nan() || value < 0.0 {
        return "--'--".to_string();
    }
    let total_seconds = value as i32;
    let minutes = (total_seconds / 60).min(999);
    let seconds = total_seconds % 60;
    format!("{minutes:02}'{seconds:02}")
}

#[cfg(test)]
mod tests;
