//! 对应 Java: `src/ui/window/comparison/logic/ComparisonCalculator.java` (一比一翻译)
//!
//! 对比计算 (差值/百分比/胜负判定)。
//! PORT: 残留 import `java.awt.Color` / `ui.window.comparison.model.ComparisonData`
//! 均未被类体使用 (CLASSIFY.md §19 亦注 "Color 为残留 import"), 不产生依赖。

/// 对应 Java 嵌套枚举 `ComparisonCalculator.WinState`。
// PORT: Java 枚举常量全大写 (WIN/LOSS/DRAW/UNKNOWN) → Rust 驼峰
// (controller_state.rs 同款先例), 语义不变。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinState {
    Win,
    Loss,
    Draw,
    Unknown,
}

/// 对应 Java 嵌套类 `ComparisonCalculator.DiffResult` (公共字段直读, 不造 getter)。
#[derive(Debug, Clone, Copy)]
pub struct DiffResult {
    pub diff: f64,
    pub percent: f64,
    pub win: WinState,
}

impl DiffResult {
    // Java 构造器 `DiffResult(double diff, double percent, WinState win)`
    pub fn new(diff: f64, percent: f64, win: WinState) -> Self {
        Self { diff, percent, win }
    }
}

/// 对应 Java 全静态类 `ComparisonCalculator` (单元结构体承载关联函数)。
pub struct ComparisonCalculator;

impl ComparisonCalculator {
    //
    // PORT: `val0 == 0` 的浮点相等比较 (含 -0.0/±0.0 相等、NaN 恒不等) 原样保留,
    // 不改写为 is_zero()/abs < eps —— NaN 传入时走 LOSS 分支是 Java 实测行为。
    #[allow(clippy::float_cmp)]
    pub fn compare(val0: f64, val1: f64, higher_is_better: bool) -> DiffResult {
        if val0 == 0.0 || val1 == 0.0 {
            return DiffResult::new(0.0, 0.0, WinState::Unknown);
        }

        let diff = val1 - val0;
        let percent = (diff / val0) * 100.0;

        // Epsilon check
        if diff.abs() < 0.001 {
            return DiffResult::new(0.0, 0.0, WinState::Draw);
        }

        let win = if higher_is_better {
            if diff > 0.0 {
                WinState::Win
            } else {
                WinState::Loss
            }
        } else {
            if diff < 0.0 {
                WinState::Win
            } else {
                WinState::Loss
            }
        };

        DiffResult::new(diff, percent, win)
    }
}

// =====================================================================
// Tests — Java 侧无独立测试文件; 期望值全部取自 Java 8 oracle 实测
// (原类直跑, Double.doubleToLongBits 逐位对拍)。
#[cfg(test)]
mod tests;
