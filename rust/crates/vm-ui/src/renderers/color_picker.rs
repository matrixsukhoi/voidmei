//! ColorPickerPopup 的颜色数学层 (src/ui/layout/renderer/ColorPickerPopup.java)。
//!
//! **D9 变更**: 原 iced 弹层面板视图 (view_picker/PickerState, D1 期即为 dead_code
//! 预留) 已删 — 取色器 UI 归 vm-webui web 壳 (JS 侧 HSB 滑条)。本模块仅存
//! java.awt.Color 的 HSB↔RGB 逐位移植, 作为 web 壳取色器数值口径的 Rust 侧锚定。
//!
//! 颜色数学 = java.awt.Color.RGBtoHSB / HSBtoRGB 的 f32 逐位移植 (含 +0.5f
//! 取整), 锚点值经 Java 8 oracle 对拍 (2026-08-26, 见 tests)。
//! JColorChooser HSB 面板的三滑条量程同为 0-255, 与 Alpha 滑条一致。

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
        _ => [v255(brightness), v255(p), v255(q)],
    }
}

// =====================================================================
// Tests — 锚点值全部取自 Java 8 oracle (HsbOracle, 2026-08-26)
// =====================================================================
#[cfg(test)]
mod tests;
