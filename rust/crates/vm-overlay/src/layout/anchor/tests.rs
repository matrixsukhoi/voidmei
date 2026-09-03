use super::*;

/// 历史基线: 9 变体 × 6 谓词全真值表
/// (输出顺序 isLeft isRight isTop isBottom isCenterHorizontal isCenterVertical)。
#[test]
fn predicates_truth_table_matches_java() {
    let table: [(Anchor, [bool; 6]); 9] = [
        // TOP_LEFT      true false true  false false false
        (Anchor::TopLeft, [true, false, true, false, false, false]),
        // TOP_CENTER    false false true  false true  false
        (Anchor::TopCenter, [false, false, true, false, true, false]),
        // TOP_RIGHT     false true  true  false false false
        (Anchor::TopRight, [false, true, true, false, false, false]),
        // MIDDLE_LEFT   true  false false false false true
        (Anchor::MiddleLeft, [true, false, false, false, false, true]),
        // CENTER        false false false false true  true
        (Anchor::Center, [false, false, false, false, true, true]),
        // MIDDLE_RIGHT  false true  false false false true
        (
            Anchor::MiddleRight,
            [false, true, false, false, false, true],
        ),
        // BOTTOM_LEFT   true  false false true  false false
        (Anchor::BottomLeft, [true, false, false, true, false, false]),
        // BOTTOM_CENTER false false false true  true  false
        (
            Anchor::BottomCenter,
            [false, false, false, true, true, false],
        ),
        // BOTTOM_RIGHT  false true  false true  false false
        (
            Anchor::BottomRight,
            [false, true, false, true, false, false],
        ),
    ];
    for (a, [left, right, top, bottom, ch, cv]) in &table {
        assert_eq!(a.is_left(), *left, "{a:?}.is_left");
        assert_eq!(a.is_right(), *right, "{a:?}.is_right");
        assert_eq!(a.is_top(), *top, "{a:?}.is_top");
        assert_eq!(a.is_bottom(), *bottom, "{a:?}.is_bottom");
        assert_eq!(a.is_center_horizontal(), *ch, "{a:?}.is_center_horizontal");
        assert_eq!(a.is_center_vertical(), *cv, "{a:?}.is_center_vertical");
    }
}

/// 结构不变量 (派生自真值表, 与 Java 一致):
/// 每行/每列恰好 3 个真; 角点是 isLeft/isRight 与 isTop/isBottom 的组合。
#[test]
fn structural_invariants() {
    let all = [
        Anchor::TopLeft,
        Anchor::TopCenter,
        Anchor::TopRight,
        Anchor::MiddleLeft,
        Anchor::Center,
        Anchor::MiddleRight,
        Anchor::BottomLeft,
        Anchor::BottomCenter,
        Anchor::BottomRight,
    ];
    for p in [
        Anchor::is_left,
        Anchor::is_right,
        Anchor::is_top,
        Anchor::is_bottom,
        Anchor::is_center_horizontal,
        Anchor::is_center_vertical,
    ] {
        assert_eq!(all.iter().filter(|a| p(a)).count(), 3);
    }
    // 角点
    assert!(Anchor::TopLeft.is_left() && Anchor::TopLeft.is_top());
    assert!(Anchor::TopRight.is_right() && Anchor::TopRight.is_top());
    assert!(Anchor::BottomLeft.is_left() && Anchor::BottomLeft.is_bottom());
    assert!(Anchor::BottomRight.is_right() && Anchor::BottomRight.is_bottom());
    // 中心: 六谓词中命中 isCenterHorizontal + isCenterVertical 两个
    assert!(Anchor::Center.is_center_horizontal() && Anchor::Center.is_center_vertical());
    assert!(!Anchor::Center.is_left() && !Anchor::Center.is_right());
    assert!(!Anchor::Center.is_top() && !Anchor::Center.is_bottom());
}
