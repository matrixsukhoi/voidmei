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
    // Java: public static DiffResult compare(double val0, double val1, boolean higherIsBetter)
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
            if diff > 0.0 { WinState::Win } else { WinState::Loss }
        } else {
            if diff < 0.0 { WinState::Win } else { WinState::Loss }
        };

        DiffResult::new(diff, percent, win)
    }
}

// =====================================================================
// Tests — Java 侧无独立测试文件; 期望值全部取自 Java 8 oracle 实测
// (原类直跑, Double.doubleToLongBits 逐位对拍)。
#[cfg(test)]
mod tests {
    use super::*;

    fn bits(x: f64) -> u64 {
        x.to_bits()
    }

    #[test]
    fn compare_basic_win_loss() {
        // oracle: CMP 100.0 120.0 true → diff=percent=20.0, WIN
        let r = ComparisonCalculator::compare(100.0, 120.0, true);
        assert_eq!(r.win, WinState::Win);
        assert_eq!(bits(r.diff), 4626322717216342016);
        assert_eq!(bits(r.percent), 4626322717216342016);

        // oracle: higherIsBetter=false → LOSS
        let r = ComparisonCalculator::compare(100.0, 120.0, false);
        assert_eq!(r.win, WinState::Loss);

        // oracle: CMP 4644.0 4093.0 → diff=-551.0, percent≈-11.86 (val1<val0)
        let r = ComparisonCalculator::compare(4644.0, 4093.0, true);
        assert_eq!(r.win, WinState::Loss);
        assert_eq!(bits(r.diff), (-551.0f64).to_bits());
        let r = ComparisonCalculator::compare(4644.0, 4093.0, false);
        assert_eq!(r.win, WinState::Win);
        // percent = -551/4644*100, Java oracle 逐位 (long 有符号打印 -4600503146096848956)
        assert_eq!(bits(r.percent), (-4600503146096848956_i64) as u64);
    }

    #[test]
    fn compare_epsilon_draw() {
        // oracle: CMP 100.0 100.0009 false → |diff|<0.001 → DRAW, diff=percent=0
        let r = ComparisonCalculator::compare(100.0, 100.0009, false);
        assert_eq!(r.win, WinState::Draw);
        assert_eq!(bits(r.diff), 0);
        assert_eq!(bits(r.percent), 0);
    }

    #[test]
    fn compare_epsilon_boundary_not_draw() {
        // oracle: CMP 100.0 100.001 false → diff=0.0010000000000047748 不小于 0.001 → LOSS
        let r = ComparisonCalculator::compare(100.0, 100.001, false);
        assert_eq!(r.win, WinState::Loss);
        assert_eq!(bits(r.diff), 4562254508917391360);
        assert_eq!(bits(r.percent), 4562254508917391360);
        // oracle: CMP 100.0 100.0015 false → LOSS, diff 位型不同
        let r = ComparisonCalculator::compare(100.0, 100.0015, false);
        assert_eq!(r.win, WinState::Loss);
        assert_eq!(bits(r.diff), 4564560351926550528);
    }

    #[test]
    fn compare_zero_operand_unknown() {
        // oracle: 任一侧为 0 (含 -0.0, Java == 视 -0.0==0.0) → UNKNOWN, 全 0
        for (v0, v1) in [(0.0, 5.0), (5.0, 0.0), (0.0, 0.0), (-0.0, 5.0)] {
            let r = ComparisonCalculator::compare(v0, v1, true);
            assert_eq!(r.win, WinState::Unknown, "({v0}, {v1})");
            assert_eq!(bits(r.diff), 0);
            assert_eq!(bits(r.percent), 0);
        }
    }

    #[test]
    fn compare_negative_base() {
        // oracle: CMP -10.0 -5.0 → diff=5.0, percent=5/-10*100=-50.0
        let r = ComparisonCalculator::compare(-10.0, -5.0, true);
        assert_eq!(r.win, WinState::Win);
        assert_eq!(bits(r.diff), 4617315517961601024);
        // oracle percent 位型 (Java long 有符号打印 -4591138345127510016)
        assert_eq!(bits(r.percent), (-4591138345127510016_i64) as u64);
        let r = ComparisonCalculator::compare(-10.0, -5.0, false);
        assert_eq!(r.win, WinState::Loss);

        // oracle: CMP 3.0 1.0 true → diff=-2.0, percent=-66.666…, LOSS
        let r = ComparisonCalculator::compare(3.0, 1.0, true);
        assert_eq!(r.win, WinState::Loss);
        assert_eq!(bits(r.diff), (-2.0f64).to_bits());
        // oracle percent 位型 (Java long 有符号打印 -4588980370306061654)
        assert_eq!(bits(r.percent), (-4588980370306061654_i64) as u64);
    }

    #[test]
    fn compare_nan_propagates_to_loss() {
        // oracle: NaN 与 0 比较恒 false → 不走 UNKNOWN; diff=NaN 不小于 0.001;
        // NaN>0 / NaN<0 均 false → LOSS (两分支同)
        let r = ComparisonCalculator::compare(f64::NAN, 1.0, true);
        assert_eq!(r.win, WinState::Loss);
        assert_eq!(bits(r.diff), 9221120237041090560);
        let r = ComparisonCalculator::compare(f64::NAN, 1.0, false);
        assert_eq!(r.win, WinState::Loss);
        let r = ComparisonCalculator::compare(1.0, f64::NAN, false);
        assert_eq!(r.win, WinState::Loss);
        assert_eq!(bits(r.percent), 9221120237041090560);
    }

    #[test]
    fn compare_infinity() {
        // oracle: CMP ∞ 1.0 true → diff=1-∞=-∞, percent=-∞/∞*100=NaN, LOSS
        let r = ComparisonCalculator::compare(f64::INFINITY, 1.0, true);
        assert_eq!(r.win, WinState::Loss);
        // oracle diff 位型 = -inf (Java long 有符号打印 -4503599627370496)
        assert_eq!(bits(r.diff), (-4503599627370496_i64) as u64);
        assert!(r.percent.is_nan());
    }

    #[test]
    fn compare_sign_flip_changes_winner() {
        // oracle: CMP 100.0 -50.0 false → diff=percent=-150.0, WIN (lower better);
        // 位型按 Java long 有符号打印 -4583890364477210624
        let r = ComparisonCalculator::compare(100.0, -50.0, false);
        assert_eq!(r.win, WinState::Win);
        assert_eq!(bits(r.diff), (-4583890364477210624_i64) as u64);
        assert_eq!(bits(r.percent), (-4583890364477210624_i64) as u64);
    }

    #[test]
    fn diff_result_fields_public() {
        // Java 公共字段直读语义
        let r = DiffResult::new(1.5, 2.5, WinState::Draw);
        assert_eq!(r.diff, 1.5);
        assert_eq!(r.percent, 2.5);
        assert_eq!(r.win, WinState::Draw);
    }
}
