use super::*;

// ---------- DPI 纯计算 (跨平台) ----------

#[test]
fn dpi_200pct_logical_and_scale_math() {
    let d = DpiHelper::from_detection(3840, 2160, 2.0, 2.0);
    assert_eq!(d.get_scale_x(), 2.0);
    assert_eq!(d.get_scale_y(), 2.0);
    assert_eq!(d.get_scale(), 2.0, "主因子 = 水平 (Java getScale)");
    assert_eq!(d.get_logical_screen_width(), 1920);
    assert_eq!(d.get_logical_screen_height(), 1080);
    assert_eq!(d.get_physical_screen_width(), 3840);
    assert_eq!(d.get_physical_screen_height(), 2160);
    assert_eq!(d.scale(800), 1600);
    assert_eq!(d.scale(24), 48);
    assert_eq!(d.unscale(48), 24);
    assert!(d.is_high_dpi());
}

#[test]
fn dpi_150pct() {
    let d = DpiHelper::from_detection(3000, 2000, 1.5, 1.5);
    assert_eq!(d.get_logical_screen_width(), 2000);
    assert_eq!(d.get_logical_screen_height(), 1333); // 2000/1.5=1333.33 → round 1333
    assert_eq!(d.scale(100), 150);
    assert!(d.is_high_dpi());
}

#[test]
fn dpi_scale_half_up_rounding_125pct() {
    // §2.3 钉子: Java Math.round(12.5)=13 (floor(x+0.5));
    // Rust f64::round 半偶舍入会给 12, 不可用
    let d = DpiHelper::from_detection(2400, 1350, 1.25, 1.25);
    assert_eq!(d.scale(10), 13);
    assert_eq!(d.unscale(13), 10); // 13/1.25=10.4 → 10
    assert_eq!(d.scale_f64(10.0), 12.5); // double 版不取整
}

#[test]
fn dpi_zero_scale_guard() {
    // Java `if (scaleX > 0)` else 分支: logical=physical;
    // unscale 的 `if (scaleX == 0) return scaledValue` 卫语句
    let d = DpiHelper::from_detection(1920, 1080, 0.0, 0.0);
    assert_eq!(d.get_logical_screen_width(), 1920);
    assert_eq!(d.get_logical_screen_height(), 1080);
    assert!(!d.is_high_dpi());
    assert_eq!(d.unscale(55), 55);
}

#[test]
fn dpi_is_high_dpi_boundary() {
    assert!(!DpiHelper::from_detection(100, 100, 1.0, 1.0).is_high_dpi());
    assert!(
        !DpiHelper::from_detection(100, 100, 1.01, 1.01).is_high_dpi(),
        "严格 > 1.01"
    );
    assert!(DpiHelper::from_detection(100, 100, 1.011, 1.0).is_high_dpi());
    assert!(
        DpiHelper::from_detection(100, 100, 1.0, 1.02).is_high_dpi(),
        "y 单独超标即高 DPI"
    );
}

#[test]
fn dpi_fallback_defaults() {
    let d = DpiHelper::fallback(1920, 1080, "GetDpiForMonitor: test");
    assert_eq!(d.get_scale_x(), 1.0);
    assert_eq!(d.get_scale_y(), 1.0);
    assert_eq!(d.get_logical_screen_width(), 1920);
    assert_eq!(d.get_logical_screen_height(), 1080);
    assert!(!d.is_high_dpi());
}

// ---------- Windows 平台腿 (真实 API) ----------
#[cfg(target_os = "windows")]
mod win_tests {
    use super::DpiHelper;
    use windows::Win32::Foundation::HWND;

    #[test]
    fn dpi_init_real_detection() {
        let d = DpiHelper::init();
        let (pw, ph) = (
            d.get_physical_screen_width(),
            d.get_physical_screen_height(),
        );
        assert!(
            pw > 0 && ph > 0,
            "桌面会话主屏物理尺寸应为正, got {}x{}",
            pw,
            ph
        );
        let (sx, sy) = (d.get_scale_x(), d.get_scale_y());
        assert!(sx > 0.0 && sy > 0.0);
        // logical 语义钉子: Java (int) Math.round(physical / scale) (scale>0 分支)
        assert_eq!(
            d.get_logical_screen_width(),
            ((pw as f64 / sx) + 0.5).floor() as i32
        );
        assert_eq!(
            d.get_logical_screen_height(),
            ((ph as f64 / sy) + 0.5).floor() as i32
        );
        assert_eq!(d.is_high_dpi(), sx > 1.01 || sy > 1.01);
    }

    #[test]
    fn dpi_for_window_invalid_falls_back() {
        // 无效句柄: GetDpiForWindow 返回 0 → Java catch 同款回退 (scale 1.0)
        let d = DpiHelper::for_window(HWND(std::ptr::null_mut()));
        assert_eq!(d.get_scale_x(), 1.0);
        assert_eq!(d.get_scale_y(), 1.0);
        assert_eq!(d.get_logical_screen_width(), d.get_physical_screen_width());
    }
}
