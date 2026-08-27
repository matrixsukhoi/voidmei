use super::*;

const FONTS: &str = "../../../fonts";

/// 预乘 RGBA alpha 通道读取
fn a(cv: &PixCanvas, x: i32, y: i32) -> u8 {
    cv.pixmap().data()[((y * cv.width() + x) * 4 + 3) as usize]
}

/// 对拍形态锁定: ctx 推导整数 (crossScale=113 → hudFontSize=28, width=254,
/// height=267 — Java 手算 oracle, MinimalHUDContext.java:96-153) + sizing 计划
/// 非零。与 minihud.rs ctx_metrics_match_java_math 互补 (此处经 parity 组装入口
/// 验证); 默认开关的组件可见性互斥组 (罗盘↔姿态仪/速度条↔油门条) 由
/// minihud.rs visibility_switches_from_settings 以同值设置锁定, 组件字段模块外
/// 不可见不重复断言。
#[test]
fn parity_scenario_ctx_and_sizing() {
    let overlay = MiniHudOverlay::init(
        false,
        SERVICE_LOOP_INTERVAL_MS,
        &ParitySettings,
        1.0,
        &std::path::Path::new(FONTS).join("sarasa-mono-sc-bold.ttf"),
    )
    .unwrap();
    let ctx = overlay.ctx();
    assert_eq!((ctx.cross_scale, ctx.hud_font_size), (113, 28));
    assert_eq!((ctx.width, ctx.height), (254, 267));
    assert_eq!(ctx.fonts.draw.size, 28);
    assert_eq!(ctx.fonts.small.size, 21);
    assert_eq!(ctx.fonts.s_small.size, 14);
    let plan = overlay.sizing().unwrap();
    assert!(plan.new_width > 0 && plan.new_height > 0, "applyAutoSizing 计划非空");
}

/// 画布 = sizing 计划 (Java applyAutoSizing 的 window.setSize 同式); 帧非空;
/// aa on/off 双口径均可渲染 (对拍脚本 --aa 注入路径)
#[test]
fn canvas_matches_sizing_and_renders_both_aa() {
    let fonts = std::path::Path::new(FONTS);
    for aa in [true, false] {
        let cv = render_minihud(fonts, aa).unwrap();
        let overlay = MiniHudOverlay::init(
            false,
            SERVICE_LOOP_INTERVAL_MS,
            &ParitySettings,
            1.0,
            &fonts.join("sarasa-mono-sc-bold.ttf"),
        )
        .unwrap();
        let plan = overlay.sizing().unwrap();
        assert_eq!(
            (cv.width(), cv.height()),
            (plan.new_width, plan.new_height),
            "aa={} 画布 = applyAutoSizing 计划",
            aa
        );
        assert!(
            cv.pixmap().data().iter().any(|&b| b != 0),
            "aa={} HUD 帧有内容",
            aa
        );
    }
}

/// 预览串注入的渲染锚点: 帧内有内容 + 准星十字竖线列存在
/// (canvas MIDDLE_RIGHT 锚, 右半区的整列亮像素; 精确像素位归 rust_compare.sh 对拍)
#[test]
fn preview_rows_and_crosshair_anchors() {
    let fonts = std::path::Path::new(FONTS);
    let cv = render_minihud(fonts, true).unwrap();
    let top_band = (0..cv.height()).any(|y| (0..cv.width()).any(|x| a(&cv, x, y) > 0));
    assert!(top_band, "帧内有非透明内容");
    // 准星可见 → 十字竖线在画布右半存在一整列亮像素 (colorNum alpha=240)
    let right_half = cv.width() / 2;
    let crosshair_col = (right_half..cv.width()).any(|x| {
        (0..cv.height()).filter(|&y| a(&cv, x, y) > 200).count() > (cv.height() / 4) as usize
    });
    assert!(crosshair_col, "准星竖线列存在 (右半区)");
}

/// 确定性: 同参数两次渲染逐字节一致 (对拍基线的可复现前提)
#[test]
fn renders_are_deterministic() {
    let fonts = std::path::Path::new(FONTS);
    let a = render_minihud(fonts, true).unwrap();
    let b = render_minihud(fonts, true).unwrap();
    assert_eq!((a.width(), a.height()), (b.width(), b.height()));
    assert_eq!(
        a.pixmap().data(),
        b.pixmap().data(),
        "同参数两帧必须逐字节一致"
    );
}
