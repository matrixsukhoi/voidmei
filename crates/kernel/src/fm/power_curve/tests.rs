use super::*;

fn stage() -> CompressorStageParams {
    CompressorStageParams::default()
}

// hasConstRpm: constRpmAlt=0 是合法值(海平面 ConstRPM), 只看 constRpmPower
#[test]
fn has_const_rpm_boundary() {
    let mut p = stage();
    // 未定义 ConstRPM: constRpmAlt/constRpmPower 双双默认 0 → false
    assert!(!has_const_rpm(&p));
    // constRpmPower=0 边界: 0 > 0 为 false
    p.const_rpm_power = 0.0;
    assert!(!has_const_rpm(&p));
    // 正值: 即便 constRpmAlt=0 (海平面弯点) 也算有 ConstRPM
    p.const_rpm_alt = 0.0;
    p.const_rpm_power = 0.1;
    assert!(has_const_rpm(&p));
    // 负值
    p.const_rpm_power = -5.0;
    assert!(!has_const_rpm(&p));
    // NaN > 0 与 Java 一致为 false
    p.const_rpm_power = f64::NAN;
    assert!(!has_const_rpm(&p));
}

// constRpmBelowCritAlt: 差值严格 < -1, 恰好 -1 不算
#[test]
fn const_rpm_below_crit_alt_boundary() {
    let mut p = stage();
    p.const_rpm_power = 100.0;
    p.crit_alt = 5000.0;
    // 无 ConstRPM 时即便差值再大也为 false (短路)
    p.const_rpm_power = 0.0;
    assert!(!const_rpm_below_crit_alt(&p));
    p.const_rpm_power = 100.0;
    // 恰好 -1: 4999 - 5000 = -1, 严格小于不成立
    p.const_rpm_alt = 4999.0;
    assert!(!const_rpm_below_crit_alt(&p));
    // -1.5 成立
    p.const_rpm_alt = 4998.5;
    assert!(const_rpm_below_crit_alt(&p));
    // -0.5 不成立
    p.const_rpm_alt = 4999.5;
    assert!(!const_rpm_below_crit_alt(&p));
    // 弯点在临界高度之上不成立
    p.const_rpm_alt = 6000.0;
    assert!(!const_rpm_below_crit_alt(&p));
    // NaN 差值: 比较为 false
    p.const_rpm_alt = f64::NAN;
    assert!(!const_rpm_below_crit_alt(&p));
}

// constRpmBelowOldCritAlt: 参照物是调整前的 oldAltitude 而非 critAlt
#[test]
fn const_rpm_below_old_crit_alt_boundary() {
    let mut p = stage();
    p.const_rpm_power = 100.0;
    // oldAltitude=5000, critAlt=4000 (调整压低了临界高度):
    // 对 oldAltitude 差 -2 < -1, 对 critAlt 差 +998 → 两函数在此分叉
    p.old_altitude = 5000.0;
    p.crit_alt = 4000.0;
    p.const_rpm_alt = 4998.0;
    assert!(!const_rpm_below_crit_alt(&p)); // 对 critAlt 不成立
    assert!(const_rpm_below_old_crit_alt(&p)); // 对 oldAltitude 成立
                                               // 恰好 -1 边界 (相对 oldAltitude)
    p.const_rpm_alt = 4999.0;
    assert!(!const_rpm_below_old_crit_alt(&p));
    // 无 ConstRPM 短路
    p.const_rpm_power = 0.0;
    assert!(!const_rpm_below_old_crit_alt(&p));
}

// constRpmBelowWepCritAlt: 参照物是 WEP 临界高度
#[test]
fn const_rpm_below_wep_crit_alt_boundary() {
    let mut p = stage();
    p.const_rpm_power = 100.0;
    p.wep_crit_alt = 6000.0;
    p.const_rpm_alt = 5998.0;
    assert!(const_rpm_below_wep_crit_alt(&p));
    // 恰好 -1
    p.const_rpm_alt = 5999.0;
    assert!(!const_rpm_below_wep_crit_alt(&p));
    // 高于 WEP 临界高度
    p.const_rpm_alt = 6500.0;
    assert!(!const_rpm_below_wep_crit_alt(&p));
    // 无 ConstRPM 短路
    p.const_rpm_power = 0.0;
    assert!(!const_rpm_below_wep_crit_alt(&p));
}

// constRpmAboveCritAlt: 四条件合取, 各条件单独失效
#[test]
fn const_rpm_above_crit_alt_boundary() {
    let mut p = stage();
    p.const_rpm_power = 100.0;
    p.const_rpm_alt = 5000.0;
    p.crit_alt = 5000.0;
    p.crit_power = 2000.0;
    p.ceiling_power = 1500.0; // critPower - ceilingPower = 500 > 1
    p.curvature = 1.5;
    assert!(const_rpm_above_crit_alt(&p));
    // 条件 1: 无 ConstRPM
    p.const_rpm_power = 0.0;
    assert!(!const_rpm_above_crit_alt(&p));
    p.const_rpm_power = 100.0;
    // 条件 2: 弯点高度不等于临界高度
    p.const_rpm_alt = 4999.0;
    assert!(!const_rpm_above_crit_alt(&p));
    p.const_rpm_alt = 5000.0;
    // 条件 3: 功率差恰好 1, 严格大于不成立
    p.ceiling_power = 1999.0;
    assert!(!const_rpm_above_crit_alt(&p));
    p.ceiling_power = 1998.5; // 差 1.5 > 1
    assert!(const_rpm_above_crit_alt(&p));
    // 条件 4: 曲率恰为 1 (默认值), 严格大于不成立
    p.curvature = 1.0;
    assert!(!const_rpm_above_crit_alt(&p));
    // NaN == critAlt 为 false, 与 Java 一致
    p.const_rpm_alt = f64::NAN;
    p.curvature = 1.5;
    assert!(!const_rpm_above_crit_alt(&p));
}

// constRpmBelowDeck: 海平面(0)及以下算 deck 以下, constRpmAlt=0 合法
#[test]
fn const_rpm_below_deck_boundary() {
    let mut p = stage();
    p.const_rpm_power = 100.0;
    // 0 恰在边界: <= 0 成立 (海平面 ConstRPM)
    p.const_rpm_alt = 0.0;
    assert!(const_rpm_below_deck(&p));
    // 负值
    p.const_rpm_alt = -0.5;
    assert!(const_rpm_below_deck(&p));
    // 正小值不成立
    p.const_rpm_alt = 0.1;
    assert!(!const_rpm_below_deck(&p));
    // 无 ConstRPM 短路 (即便 alt=0)
    p.const_rpm_power = 0.0;
    p.const_rpm_alt = 0.0;
    assert!(!const_rpm_below_deck(&p));
}

// hasCeiling: 两个参数都须为正
#[test]
fn has_ceiling_boundary() {
    let mut p = stage();
    p.ceiling_alt = 10000.0;
    p.ceiling_power = 1000.0;
    assert!(has_ceiling(&p));
    // ceilingAlt = 0
    p.ceiling_alt = 0.0;
    assert!(!has_ceiling(&p));
    p.ceiling_alt = 10000.0;
    // ceilingPower = 0
    p.ceiling_power = 0.0;
    assert!(!has_ceiling(&p));
    // 负值
    p.ceiling_power = -1.0;
    assert!(!has_ceiling(&p));
}

// ceilingIsUseful: 参照高度/功率取 oldAltitude/oldPower (>0 时), 否则回退 critAlt/critPower; 差值 >= 2
#[test]
fn ceiling_is_useful_boundary() {
    let mut p = stage();
    // 场景 1: oldAltitude/oldPower 均有效, 作为参照
    p.old_altitude = 5000.0;
    p.old_power = 2000.0;
    p.ceiling_alt = 10000.0;
    p.ceiling_power = 1500.0;
    // 高度差 5000 >= 2, 功率差 500 >= 2
    assert!(ceiling_is_useful(&p));
    // 高度差恰好 2 (相对 oldAltitude, 而非 critAlt)
    p.crit_alt = 7000.0; // 若误用 critAlt 差仅 3000 仍成立, 换个对照:
    p.ceiling_alt = 5002.0;
    assert!(ceiling_is_useful(&p));
    // 高度差 1.99 < 2
    p.ceiling_alt = 5001.99;
    assert!(!ceiling_is_useful(&p));
    // 功率差恰好 2 成立
    p.ceiling_alt = 10000.0;
    p.ceiling_power = 1998.0;
    assert!(ceiling_is_useful(&p));
    // 功率差 1.99 不成立
    p.ceiling_power = 1998.01;
    assert!(!ceiling_is_useful(&p));
    // 场景 2: oldAltitude<=0 回退 critAlt, oldPower<=0 回退 critPower
    p.old_altitude = 0.0;
    p.old_power = 0.0;
    p.crit_alt = 4000.0;
    p.crit_power = 1800.0;
    p.ceiling_alt = 8000.0; // 对 critAlt 差 4000
    p.ceiling_power = 1000.0; // 对 critPower 差 800
    assert!(ceiling_is_useful(&p));
    // 对回退参照 critAlt 差不足: ceiling_alt = 4001 (差 1 < 2)
    p.ceiling_alt = 4001.0;
    assert!(!ceiling_is_useful(&p));
    // oldAltitude 为负也回退
    p.old_altitude = -100.0;
    assert!(!ceiling_is_useful(&p));
    // 场景 3: 无 ceiling 参数短路 (高度差/功率差再大也 false)
    p.old_altitude = 5000.0;
    p.old_power = 2000.0;
    p.ceiling_alt = 0.0;
    p.ceiling_power = 0.0;
    assert!(!ceiling_is_useful(&p));
}

// powerIsDeckPower: |critAlt - deckAlt| < 1
#[test]
fn power_is_deck_power_boundary() {
    let mut p = stage();
    // 默认 FM: critAlt = deckAlt = 0 → 平直曲线, 成立
    assert!(power_is_deck_power(&p));
    p.crit_alt = 1000.0;
    p.deck_alt = 1000.0;
    assert!(power_is_deck_power(&p));
    // 差 0.999 成立
    p.deck_alt = 999.001;
    assert!(power_is_deck_power(&p));
    // 差恰好 1 不成立
    p.deck_alt = 999.0;
    assert!(!power_is_deck_power(&p));
    // 负方向对称
    p.deck_alt = 1000.999;
    assert!(power_is_deck_power(&p));
    p.deck_alt = 1001.0;
    assert!(!power_is_deck_power(&p));
    // NaN: abs(NaN) < 1 为 false, 与 Java Math.abs 一致
    p.deck_alt = f64::NAN;
    assert!(!power_is_deck_power(&p));
}
