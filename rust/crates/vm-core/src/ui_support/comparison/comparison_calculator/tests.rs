use super::*;

fn bits(x: f64) -> u64 {
    x.to_bits()
}

#[test]
fn compare_basic_win_loss() {
    // 基线: CMP 100.0 120.0 true → diff=percent=20.0, WIN
    let r = ComparisonCalculator::compare(100.0, 120.0, true);
    assert_eq!(r.win, WinState::Win);
    assert_eq!(bits(r.diff), 4626322717216342016);
    assert_eq!(bits(r.percent), 4626322717216342016);

    // 基线: higherIsBetter=false → LOSS
    let r = ComparisonCalculator::compare(100.0, 120.0, false);
    assert_eq!(r.win, WinState::Loss);

    // 基线: CMP 4644.0 4093.0 → diff=-551.0, percent≈-11.86 (val1<val0)
    let r = ComparisonCalculator::compare(4644.0, 4093.0, true);
    assert_eq!(r.win, WinState::Loss);
    assert_eq!(bits(r.diff), (-551.0f64).to_bits());
    let r = ComparisonCalculator::compare(4644.0, 4093.0, false);
    assert_eq!(r.win, WinState::Win);
    // percent = -551/4644*100, 历史基线 逐位 (long 有符号打印 -4600503146096848956)
    assert_eq!(bits(r.percent), (-4600503146096848956_i64) as u64);
}

#[test]
fn compare_epsilon_draw() {
    // 基线: CMP 100.0 100.0009 false → |diff|<0.001 → DRAW, diff=percent=0
    let r = ComparisonCalculator::compare(100.0, 100.0009, false);
    assert_eq!(r.win, WinState::Draw);
    assert_eq!(bits(r.diff), 0);
    assert_eq!(bits(r.percent), 0);
}

#[test]
fn compare_epsilon_boundary_not_draw() {
    // 基线: CMP 100.0 100.001 false → diff=0.0010000000000047748 不小于 0.001 → LOSS
    let r = ComparisonCalculator::compare(100.0, 100.001, false);
    assert_eq!(r.win, WinState::Loss);
    assert_eq!(bits(r.diff), 4562254508917391360);
    assert_eq!(bits(r.percent), 4562254508917391360);
    // 基线: CMP 100.0 100.0015 false → LOSS, diff 位型不同
    let r = ComparisonCalculator::compare(100.0, 100.0015, false);
    assert_eq!(r.win, WinState::Loss);
    assert_eq!(bits(r.diff), 4564560351926550528);
}

#[test]
fn compare_zero_operand_unknown() {
    // 基线: 任一侧为 0 (含 -0.0, Java == 视 -0.0==0.0) → UNKNOWN, 全 0
    for (v0, v1) in [(0.0, 5.0), (5.0, 0.0), (0.0, 0.0), (-0.0, 5.0)] {
        let r = ComparisonCalculator::compare(v0, v1, true);
        assert_eq!(r.win, WinState::Unknown, "({v0}, {v1})");
        assert_eq!(bits(r.diff), 0);
        assert_eq!(bits(r.percent), 0);
    }
}

#[test]
fn compare_negative_base() {
    // 基线: CMP -10.0 -5.0 → diff=5.0, percent=5/-10*100=-50.0
    let r = ComparisonCalculator::compare(-10.0, -5.0, true);
    assert_eq!(r.win, WinState::Win);
    assert_eq!(bits(r.diff), 4617315517961601024);
    // 基线 percent 位型 (Java long 有符号打印 -4591138345127510016)
    assert_eq!(bits(r.percent), (-4591138345127510016_i64) as u64);
    let r = ComparisonCalculator::compare(-10.0, -5.0, false);
    assert_eq!(r.win, WinState::Loss);

    // 基线: CMP 3.0 1.0 true → diff=-2.0, percent=-66.666…, LOSS
    let r = ComparisonCalculator::compare(3.0, 1.0, true);
    assert_eq!(r.win, WinState::Loss);
    assert_eq!(bits(r.diff), (-2.0f64).to_bits());
    // 基线 percent 位型 (Java long 有符号打印 -4588980370306061654)
    assert_eq!(bits(r.percent), (-4588980370306061654_i64) as u64);
}

#[test]
fn compare_nan_propagates_to_loss() {
    // 基线: NaN 与 0 比较恒 false → 不走 UNKNOWN; diff=NaN 不小于 0.001;
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
    // 基线: CMP ∞ 1.0 true → diff=1-∞=-∞, percent=-∞/∞*100=NaN, LOSS
    let r = ComparisonCalculator::compare(f64::INFINITY, 1.0, true);
    assert_eq!(r.win, WinState::Loss);
    // 基线 diff 位型 = -inf (Java long 有符号打印 -4503599627370496)
    assert_eq!(bits(r.diff), (-4503599627370496_i64) as u64);
    assert!(r.percent.is_nan());
}

#[test]
fn compare_sign_flip_changes_winner() {
    // 基线: CMP 100.0 -50.0 false → diff=percent=-150.0, WIN (lower better);
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
