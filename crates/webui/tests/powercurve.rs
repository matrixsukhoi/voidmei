//! 功率曲线拐点检测黑盒场景 (identify_inflection_points_for_curve):
//! 双峰一谷的峰/谷/kink 标注 + 短曲线守卫。输入是纯 f64 曲线 (高度格点),
//! 零文件零 UI — ALT_STEP=25m 的几何语义直接驱动期望。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


use expect_test::expect;

use webui::commands_powercurve::identify_inflection_points_for_curve;

/// 曲线构造: 关键高度锚点线性插值 (峰/谷/平段)
/// 锚点 = (高度索引, 功率)
fn curve(anchors: &[(usize, f64)], len: usize) -> Vec<f64> {
    let mut v = vec![0.0f64; len];
    for w in anchors.windows(2) {
        let (i0, p0) = (w[0].0, w[0].1);
        let (i1, p1) = (w[1].0, w[1].1);
        for (i, slot) in v.iter_mut().enumerate().take(i1 + 1).skip(i0) {
            let t = (i - i0) as f64 / (i1 - i0) as f64;
            *slot = p0 + (p1 - p0) * t;
        }
    }
    v
}

/// 标注串速记: kind/label/altitude/power
fn marks(pts: &[webui::dto::InflectionPointDto]) -> String {
    pts.iter()
        .map(|p| {
            let kind = match p.kind {
                webui::dto::InflectionKind::Peak => "P",
                webui::dto::InflectionKind::Valley => "V",
                webui::dto::InflectionKind::Kink => "K",
            };
            // 功率取整 (f64 比较容噪; 标注位置是断言核心)
            format!("{kind} {} @{}m", p.label, p.altitude_m)
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// 双峰一谷: 谷 = 级间过渡 (1→2档), 双峰 = 临界高度档;
/// 生成序 = 谷 (Phase2) 先于峰 (Phase3), kink 兜底其余斜率突变
#[test]
fn 拐点_双峰一谷() {
    // 0m 爬升至峰1 (500m) → 落入谷 (1250m) → 二段爬升至峰2 (2000m) → 衰减
    let anchors: Vec<(usize, f64)> = vec![
        (0, 1100.0),
        (20, 1200.0), // 峰1 @ 500m
        (50, 300.0),  // 谷 @ 1250m
        (80, 1000.0), // 峰2 @ 2000m
        (100, 600.0),
    ];
    let pc = curve(&anchors, 101);
    let pts = identify_inflection_points_for_curve(&pc, 1200.0);
    let actual = marks(&pts);
    expect!["V 1→2档 @1250m | P 1档 @500m | P 2档 @2000m"]
    .assert_eq(&actual);
}

/// 短曲线守卫: max_idx < 6 直接空 (扫描窗不足, Java 同位守卫)
#[test]
fn 拐点_短曲线守卫() {
    // len=7 → max_idx = min(400, 6) = 6 → 不触发守卫; len=6 → 5 < 6 触发
    let long_enough = curve(&[(0, 100.0), (6, 90.0)], 7);
    // 该曲线单调 (无峰谷), kink 也无 → 空但已过守卫 (max_idx==6 进扫描)
    let pts = identify_inflection_points_for_curve(&long_enough, 100.0);
    expect!["(len=7 单调曲线无拐点) 0"].assert_eq(&format!("(len=7 单调曲线无拐点) {}", pts.len()));

    let too_short = curve(&[(0, 100.0), (5, 90.0)], 6);
    let pts2 = identify_inflection_points_for_curve(&too_short, 100.0);
    expect!["(len=6 守卫) 0"].assert_eq(&format!("(len=6 守卫) {}", pts2.len()));

    let empty = identify_inflection_points_for_curve(&[], 100.0);
    expect!["(空曲线) 0"].assert_eq(&format!("(空曲线) {}", empty.len()));
}
