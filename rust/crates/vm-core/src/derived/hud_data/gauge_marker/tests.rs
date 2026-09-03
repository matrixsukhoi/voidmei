use super::*;

/// 历史基线 `GMdefault`: builder() 不设任何字段的产物。
#[test]
fn builder_defaults_match_java() {
    let d = GaugeMarker::builder().build();
    assert_eq!(d.id, "");
    assert_eq!(d.r#type, MarkerType::LineFull);
    assert_eq!(d.ratio, -1.0);
    assert_eq!(d.color, [255, 0, 0, 255]);
    assert_eq!(d.label, "");
    assert_eq!(d.width_ratio, 0.5);
    assert_eq!(d.side, 0);
    assert!(!d.is_visible(), "ratio=-1 默认隐藏");
}

/// Java 类文档示例的 fluent 构造路径。
#[test]
fn fluent_build_sets_all_fields() {
    let m = GaugeMarker::builder()
        .id("optimal".to_string())
        .r#type(MarkerType::TickLabeled)
        .ratio(0.5)
        .color([255, 200, 64, 255])
        .label("WEP".to_string())
        .width_ratio(0.25)
        .side(-1)
        .build();
    assert_eq!(m.id, "optimal");
    assert_eq!(m.r#type, MarkerType::TickLabeled);
    assert_eq!(m.ratio, 0.5);
    assert_eq!(m.color, [255, 200, 64, 255]);
    assert_eq!(m.label, "WEP");
    assert_eq!(m.width_ratio, 0.25);
    assert_eq!(m.side, -1);
}

/// 历史基线 `visible(...)` 边界表: 含 NaN 两端开区间行为。
#[test]
fn is_visible_boundaries_match_java() {
    let cases = [
        (0.0, true),
        (1.0, true),
        (-0.0001, false),
        (1.0001, false),
        (-1.0, false),
        (f64::NAN, false),
        (0.5, true),
    ];
    for (r, expect) in cases {
        assert_eq!(
            GaugeMarker::builder().ratio(r).build().is_visible(),
            expect,
            "visible({r}) 失配"
        );
    }
}

/// 历史基线: `with_same=true`、`with_plus_5e5=true`
/// (|0.5+0.00005 - 0.5| = 4.999999999999449e-5 < 1e-4 → 同实例)。
#[test]
fn with_ratio_returns_self_when_delta_below_epsilon() {
    let m = GaugeMarker::builder().ratio(0.5).build();
    assert!(matches!(m.with_ratio(0.5), Cow::Borrowed(_)));
    assert!(matches!(m.with_ratio(0.5 + 0.00005), Cow::Borrowed(_)));
    // 9e-5 < 1e-4 → 同实例 (基线 with_9e5=true)
    let m0 = GaugeMarker::builder().ratio(0.0).build();
    assert!(matches!(m0.with_ratio(0.00009), Cow::Borrowed(_)));
}

/// 历史基线 `with_exact_1e4=false`: 严格小于判定, delta == 1e-4 时已换新实例。
#[test]
fn with_ratio_new_instance_at_exact_epsilon() {
    let m0 = GaugeMarker::builder().ratio(0.0).build();
    // 0.0 + 0.0001: delta 即 1e-4 的最近双精度值, `< 0.0001` 不成立 → Owned
    assert_eq!(0.0001 - 0.0, 0.0001);
    assert!(matches!(m0.with_ratio(0.0001), Cow::Owned(_)));
}

/// 历史基线 `new_ratio=0.75 ...`: 换 ratio 后其余字段原样保留
/// (走 Builder 重建路径, 未显式设置处为 Builder 默认的拷贝源值)。
#[test]
fn with_ratio_copies_other_fields() {
    let m = GaugeMarker::builder()
        .id("stall".to_string())
        .r#type(MarkerType::Zone)
        .ratio(0.5)
        .color([255, 0, 0, 200])
        .label("STALL".to_string())
        .width_ratio(0.3)
        .side(1)
        .build();
    let m2 = m.with_ratio(0.75);
    let owned = match m2 {
        Cow::Owned(ref g) => g.clone(),
        Cow::Borrowed(_) => panic!("0.75 vs 0.5 应生成新实例"),
    };
    assert_eq!(owned.ratio, 0.75);
    assert_eq!(owned.id, "stall");
    assert_eq!(owned.r#type, MarkerType::Zone);
    assert_eq!(owned.color, [255, 0, 0, 200]);
    assert_eq!(owned.label, "STALL");
    assert_eq!(owned.width_ratio, 0.3);
    assert_eq!(owned.side, 1);
    // 原实例不受影响 (Java 不可变类语义)
    assert_eq!(m.ratio, 0.5);
}

/// 边界: NaN 与任何 ratio 的差都是 NaN, `< 0.0001` 为 false → 必然 Owned。
#[test]
fn with_ratio_nan_always_new_instance() {
    let m = GaugeMarker::builder().ratio(0.5).build();
    assert!(matches!(m.with_ratio(f64::NAN), Cow::Owned(_)));
}
