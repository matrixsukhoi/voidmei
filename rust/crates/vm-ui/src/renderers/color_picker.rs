//! ColorPickerPopup 的 iced 语义复刻 (src/ui/layout/renderer/ColorPickerPopup.java)。
//!
//! Java: WebPopOver 弹层 = JColorChooser(仅 HSB 面板) + Alpha 滑条 (0-255) +
//! Hex 输入 + 预览色块 + [取消][确定] (L100-216)。iced: 无弹层窗口原语
//! (features tiny-skia/system), 以内嵌面板视图交付 — 接线批十三将
//! [`PickerState`] 挂 MainFormState (或独立 pane) 并把本视图挂到 COLOR 行下方。
//!
//! 颜色数学 = java.awt.Color.RGBtoHSB / HSBtoRGB 的 f32 逐位移植 (含 +0.5f
//! 取整), 锚点值经 Java 8 oracle 对拍 (2026-08-26, 见 tests)。
//! JColorChooser HSB 面板的三滑条量程同为 0-255, 与 Alpha 滑条一致。
//!
//! PORT(交互分歧备案):
//! - Java 拖动滑条只改弹层 currentColor, 点"确定"才回调落库 (L201-208); 冻结的
//!   消息枚举无弹层内部消息 → 本视图滑条/hex 直发 Message::ColorPicked (预览即
//!   写), 取消 = Ignore。接线批若需确认门控, 应为弹层增加内部消息变体后把
//!   on_change 改发内部态。
//! - Java hexField 允许中间非法文本 (失焦才解析回弹, L257-266); 本视图 hex 输入
//!   即时门控 — 非法输入静默 + 视图回弹规范串 (无 draft 状态可存)。

use iced::widget::{button, column, container, row, slider, text, text_input, Space};
use iced::{Color, Element, Length};

use super::color::{to_hex_string, try_parse_color};
use crate::main_form::Message;

// =====================================================================
// HSB ↔ RGB (java.awt.Color 逐位移植)
// =====================================================================

/// Java Color.RGBtoHSB: rgb → [h, s, b] (各 ∈ [0,1] 分数)。
pub fn rgb_to_hsb(rgb: [u8; 3]) -> [f32; 3] {
    let (r, g, b) = (rgb[0] as i32, rgb[1] as i32, rgb[2] as i32);
    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let brightness = max as f32 / 255.0;
    let saturation = if max == 0 { 0.0 } else { (max - min) as f32 / max as f32 };
    let hue = if max == min {
        0.0
    } else {
        // Java: 各通道对 (max-min) 的归一差
        let rc = (max - r) as f32 / (max - min) as f32;
        let gc = (max - g) as f32 / (max - min) as f32;
        let bc = (max - b) as f32 / (max - min) as f32;
        let h = if r == max {
            bc - gc
        } else if g == max {
            2.0 + rc - bc
        } else {
            4.0 + gc - rc
        };
        let mut h = h / 6.0;
        if h < 0.0 {
            h += 1.0;
        }
        h
    };
    [hue, saturation, brightness]
}

/// Java Color.HSBtoRGB: [h, s, b] 分数 → rgb (通道经 `*255+0.5f` 取整)。
pub fn hsb_to_rgb(hsb: [f32; 3]) -> [u8; 3] {
    let (hue, saturation, brightness) = (hsb[0], hsb[1], hsb[2]);
    if saturation == 0.0 {
        let v = (brightness * 255.0 + 0.5) as i32;
        return [v as u8, v as u8, v as u8];
    }
    // Java: h = (hue - floor(hue)) * 6 — hue >= 1 或 < 0 归一化到 [0,1)
    let h = (hue - hue.floor()) * 6.0;
    let f = h - h.floor();
    let p = brightness * (1.0 - saturation);
    let q = brightness * (1.0 - saturation * f);
    let t = brightness * (1.0 - saturation * (1.0 - f));
    let v255 = |x: f32| (x * 255.0 + 0.5) as u8;
    match h as i32 {
        0 => [v255(brightness), v255(t), v255(p)],
        1 => [v255(q), v255(brightness), v255(p)],
        2 => [v255(p), v255(brightness), v255(t)],
        3 => [v255(p), v255(q), v255(brightness)],
        4 => [v255(t), v255(p), v255(brightness)],
        _ => [v255(brightness), v255(p), v255(q)], // Java case 5 + 越界落 default
    }
}

// =====================================================================
// 弹层状态 + 视图
// =====================================================================

/// 弹层状态 (Java currentColor; hex 文本由 current 规范化派生, 见模块文档)。
/// PORT(dead_code): 接线批十三挂 MainFormState; 本批仅测试可达。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickerState {
    pub current: [u8; 4],
}

#[allow(dead_code)]
impl PickerState {
    /// Java L71: initialColor null → Color.WHITE
    pub fn new(initial: [u8; 4]) -> Self {
        PickerState { current: initial }
    }
}

/// 弹层面板视图 (Java mainPanel 布局: HSB 区 / Alpha 行 / Hex+预览行 / 按钮行)。
/// panel_title/key = COLOR 行的定位键 (滑条/确定按钮发 ColorPicked 时携带)。
/// PORT(dead_code): 接线批十三; 视图本体与消息形状已定, 挂载即用。
#[allow(dead_code)]
pub fn view_picker<'a>(
    state: &PickerState,
    panel_title: &'a str,
    key: &'a str,
) -> Element<'a, Message> {
    let cur = state.current;
    let [h, s, b] = rgb_to_hsb([cur[0], cur[1], cur[2]]);
    let f255 = |f: f32| (f * 255.0 + 0.5) as i32; // 滑条反量化 (JColorChooser 同 0-255 量程)

    // H/S/B 滑条: 变更即以新分量重建 rgb (alpha 保留) — Java chooser 监听 (L118-124)
    let mk_hs = move |v: i32, which: u8| {
        let (h2, s2, b2) = match which {
            0 => (v as f32 / 255.0, s, b),
            1 => (h, v as f32 / 255.0, b),
            _ => (h, s, v as f32 / 255.0),
        };
        let rgb = hsb_to_rgb([h2, s2, b2]);
        Message::ColorPicked {
            panel: panel_title.to_string(),
            key: key.to_string(),
            value: [rgb[0], rgb[1], rgb[2], cur[3]],
        }
    };
    let hue_sl = slider(0..=255, f255(h), move |v| mk_hs(v, 0));
    let sat_sl = slider(0..=255, f255(s), move |v| mk_hs(v, 1));
    let bri_sl = slider(0..=255, f255(b), move |v| mk_hs(v, 2));
    // Alpha 滑条 (Java L141-151): 只改 alpha 通道
    let alpha_sl = slider(0..=255, cur[3] as i32, move |v| Message::ColorPicked {
        panel: panel_title.to_string(),
        key: key.to_string(),
        value: [cur[0], cur[1], cur[2], v as u8],
    });

    // Hex 输入 + 预览 (Java hexRow L162-189): 合法完整色串才提交
    let hex_cur = to_hex_string(&cur, true);
    let hex_field = text_input("", &hex_cur).on_input(move |t| match try_parse_color(&t) {
        Some(c) => Message::ColorPicked {
            panel: panel_title.to_string(),
            key: key.to_string(),
            value: c,
        },
        None => Message::Ignore,
    });
    let preview = container(Space::new(Length::Fixed(40.0), Length::Fixed(24.0)))
        .style(move |_| container::Style {
            background: Some(Color::from_rgba8(cur[0], cur[1], cur[2], cur[3] as f32 / 255.0).into()),
            ..Default::default()
        });

    // 按钮行 (Java L193-209): 取消 = 不应用 (关闭状态归接线层, 此处 Ignore);
    // 确定 = 以当前色回调 (= ColorPicked 最终值)
    let cancel = button("取消").on_press(Message::Ignore);
    let confirm = button("确定").on_press(Message::ColorPicked {
        panel: panel_title.to_string(),
        key: key.to_string(),
        value: cur,
    });

    column![
        row![text("色相 H"), hue_sl, text(f255(h).to_string())].spacing(6),
        row![text("饱和 S"), sat_sl, text(f255(s).to_string())].spacing(6),
        row![text("明度 B"), bri_sl, text(f255(b).to_string())].spacing(6),
        row![text("Alpha:"), alpha_sl, text(cur[3].to_string())].spacing(6),
        row![text("Hex:"), hex_field, preview].spacing(6),
        row![cancel, confirm].spacing(6),
    ]
    .spacing(5)
    .padding(10)
    .into()
}

// =====================================================================
// Tests — 锚点值全部取自 Java 8 oracle (HsbOracle, 2026-08-26)
// =====================================================================
#[cfg(test)]
mod tests {
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
}
