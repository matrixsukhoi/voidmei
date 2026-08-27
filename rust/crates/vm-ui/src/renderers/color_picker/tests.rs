use super::*;

/// oracle 按 %.7f 打印 (舍入 ≤5e-8), f32 表示再引入微小误差 — 容差覆盖打印
/// 舍入, 非阈值放宽
const EPS: f32 = 1e-6;

fn assert_hsb(rgb: [u8; 3], want: [f64; 3]) {
    let got = rgb_to_hsb(rgb);
    for (g, w) in got.iter().zip(want.iter()) {
        assert!(
            (g - *w as f32).abs() < EPS,
            "rgb{rgb:?} hsb {got:?} vs oracle {want:?}"
        );
    }
}

// RGBtoHSB 锚点 (oracle 15 色)
#[test]
fn rgb_to_hsb_anchors() {
    assert_hsb([255, 0, 0], [0.0, 1.0, 1.0]);
    assert_hsb([0, 255, 0], [0.3333333, 1.0, 1.0]);
    assert_hsb([0, 0, 255], [0.6666667, 1.0, 1.0]);
    assert_hsb([255, 255, 255], [0.0, 0.0, 1.0]);
    assert_hsb([0, 0, 0], [0.0, 0.0, 0.0]);
    assert_hsb([255, 128, 0], [0.0836601, 1.0, 1.0]);
    assert_hsb([128, 255, 0], [0.2496732, 1.0, 1.0]);
    assert_hsb([0, 128, 255], [0.5830066, 1.0, 1.0]);
    assert_hsb([232, 147, 50], [0.0888278, 0.7844828, 0.9098039]);
    assert_hsb([255, 85, 0], [0.0555556, 1.0, 1.0]);
    assert_hsb([100, 150, 200], [0.5833333, 0.5, 0.7843137]);
    assert_hsb([127, 127, 127], [0.0, 0.0, 0.4980392]);
    assert_hsb([1, 2, 3], [0.5833333, 0.6666667, 0.0117647]);
    assert_hsb([254, 253, 252], [0.0833333, 0.0078740, 0.9960784]);
    assert_hsb([51, 102, 153], [0.5833333, 0.6666667, 0.6]);
}

// HSBtoRGB 锚点 (oracle 11 组; 输入按 Java 源的 f32 除法重构)
#[test]
fn hsb_to_rgb_anchors() {
    assert_eq!(hsb_to_rgb([0.0, 1.0, 1.0]), [255, 0, 0]);
    assert_eq!(hsb_to_rgb([0.5, 1.0, 1.0]), [0, 255, 255]);
    assert_eq!(hsb_to_rgb([2.0 / 6.0, 1.0, 1.0]), [0, 255, 0]);
    assert_eq!(hsb_to_rgb([0.0, 0.0, 1.0]), [255, 255, 255]);
    assert_eq!(hsb_to_rgb([0.0, 0.0, 0.0]), [0, 0, 0]);
    assert_eq!(hsb_to_rgb([0.111111, 0.5, 0.75]), [191, 159, 96]);
    assert_eq!(hsb_to_rgb([0.9, 0.9, 0.9]), [230, 23, 147]);
    assert_eq!(hsb_to_rgb([1.0 / 3.0, 1.0, 0.5]), [0, 128, 0]);
    assert_eq!(hsb_to_rgb([2.0 / 3.0, 0.25, 1.0]), [191, 191, 255]);
    assert_eq!(hsb_to_rgb([0.05, 0.95, 0.85]), [217, 73, 11]);
    assert_eq!(hsb_to_rgb([1.0, 1.0, 1.0]), [255, 0, 0]); // hue ≥1 归一化回绕
}

// RGB→HSB→RGB 往返: oracle 15 色全部逐位相等
#[test]
fn hsb_round_trip_exact() {
    let rgbs: [[u8; 3]; 15] = [
        [255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255], [0, 0, 0],
        [255, 128, 0], [128, 255, 0], [0, 128, 255], [232, 147, 50], [255, 85, 0],
        [100, 150, 200], [127, 127, 127], [1, 2, 3], [254, 253, 252], [51, 102, 153],
    ];
    for rgb in rgbs {
        assert_eq!(hsb_to_rgb(rgb_to_hsb(rgb)), rgb, "往返失配: {rgb:?}");
    }
}

// 视图构建冒烟 (含 alpha=0 边缘色)
#[test]
fn view_picker_builds() {
    let st = PickerState::new([232, 147, 50, 128]);
    let _el = view_picker(&st, "面板", "fontWarn");
    let st2 = PickerState::new([0, 0, 0, 0]);
    let _el2 = view_picker(&st2, "面板", "fontWarn");
}
