use super::*;

/// 读预乘 RGBA 像素 (与 gauges_bars/render2d 测试同约定)
fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
    let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
    [d[0], d[1], d[2], d[3]]
}

fn any_nonzero(c: &PixCanvas) -> bool {
    c.pixmap().data().iter().any(|&b| b != 0)
}

/// 每通道 ±2 容差 (tiny-skia 预乘取整路径的 LSB 系统差, render2d 头注口径)
fn assert_px_close(actual: [u8; 4], expected: [u8; 4], what: &str) {
    for i in 0..4 {
        assert!(
            (actual[i] as i32 - expected[i] as i32).abs() <= 2,
            "{what}: 通道{i} = {actual:?} 期望 ~{expected:?} (±2 LSB)"
        );
    }
}

/// Java:263-265 周期公式: (1000/interval)>>3, 0 钳 1 (long 整除)
#[test]
fn blink_ticks_formula() {
    assert_eq!(WarningBlinkHost::new(100).blink_ticks(), 1, "10>>3=1");
    assert_eq!(WarningBlinkHost::new(50).blink_ticks(), 2, "20>>3=2");
    assert_eq!(WarningBlinkHost::new(40).blink_ticks(), 3, "25>>3=3");
    assert_eq!(WarningBlinkHost::new(10).blink_ticks(), 12, "100>>3=12");
    assert_eq!(
        WarningBlinkHost::new(16).blink_ticks(),
        7,
        "1000/16=62 (截断)>>3=7"
    );
    assert_eq!(WarningBlinkHost::new(125).blink_ticks(), 1, "8>>3=1");
    assert_eq!(WarningBlinkHost::new(200).blink_ticks(), 1, "5>>3=0 → 钳 1");
    assert_eq!(
        WarningBlinkHost::new(1000).blink_ticks(),
        1,
        "1>>3=0 → 钳 1"
    );
}

/// interval=0: Java long 除零抛 ArithmeticException → Rust panic (同致命)
#[test]
#[should_panic]
fn blink_ticks_zero_interval_panics() {
    let _ = WarningBlinkHost::new(0);
}

/// 节奏 blink_ticks=1 (interval=100): 每帧翻转 — 帧序列 亮/灭/亮/灭
/// (Java:77-85 — 先绘制后计数翻转, 首帧 blinkActing=false 即亮)
#[test]
fn blink_rhythm_toggle_every_frame() {
    let mut host = WarningBlinkHost::new(100);
    host.set_blink_x(true);
    let mut frames = Vec::new();
    for _ in 0..4 {
        let mut c = PixCanvas::new(60, 40).unwrap();
        host.draw_blink_x(&mut c, 60, 40, false);
        frames.push(any_nonzero(&c));
    }
    assert_eq!(frames, vec![true, false, true, false], "1 帧周期交替");
}

/// 节奏 blink_ticks=2 (interval=50): 亮亮灭灭 — 半周期 2 帧
#[test]
fn blink_rhythm_two_frame_half_period() {
    let mut host = WarningBlinkHost::new(50);
    host.set_blink_x(true);
    let mut frames = Vec::new();
    for _ in 0..6 {
        let mut c = PixCanvas::new(60, 40).unwrap();
        host.draw_blink_x(&mut c, 60, 40, false);
        frames.push(any_nonzero(&c));
    }
    assert_eq!(frames, vec![true, true, false, false, true, true]);
}

/// blinkX=false: 不绘制且节奏冻结 (Java:77 门卫整体短路 — 计数/相位不推进);
/// 恢复后从冻结相位继续 (首帧仍亮)
#[test]
fn blink_disabled_freezes_rhythm() {
    let mut host = WarningBlinkHost::new(100);
    for _ in 0..5 {
        let mut c = PixCanvas::new(60, 40).unwrap();
        host.draw_blink_x(&mut c, 60, 40, false);
        assert!(!any_nonzero(&c), "blinkX=false 无输出");
    }
    assert!(!host.is_blink_acting(), "相位冻结在初始 false");
    host.set_blink_x(true);
    let mut c = PixCanvas::new(60, 40).unwrap();
    host.draw_blink_x(&mut c, 60, 40, false);
    assert!(any_nonzero(&c), "恢复后首帧即亮 (计数从 0 续)");
}

/// is_blink_off=true: Java:37-39 整体不绘制 (off 相位走同一路径)
#[test]
fn warning_off_phase_draws_nothing() {
    let mut w = WarningOverlay::new();
    let mut c = PixCanvas::new(60, 40).unwrap();
    w.draw(&mut c, 0, 0, 60, 40, true, false);
    assert!(c.pixmap().data().iter().all(|&b| b == 0));
}

/// X 几何 (x=5,y=5,w=50,h=30, aa=false 像素中心采样):
/// - 影线1 (7,7)→(53,33), 前景线1 (6,6)→(54,34) (:51/:57 内缩 2/1);
/// - (17,15) 距前景线1 2.41px (>1.5) 距影线1 2.23px (≤2.5) → 纯影
///   [0,0,0,42] (黑影预乘仅 alpha);
/// - 中心 (30,20) = 两影线 + 两前景线四层交叠: 影+影 a=77 → 前景 a=244 →
///   前景 a≈254, 预乘 ≈ [27,254,128,254];
/// - (18,27) 在副对角线 (54,6)→(6,34) 上 (距 0.68), 距主线 12.3 → 单侧
///   影(42)+前景(240) 两层 ≈ [25,240,120,242];
/// - (4,4)/(5,20) 远离全部线段 (含圆帽) → 空。
#[test]
fn warning_x_geometry_layers() {
    let mut w = WarningOverlay::new();
    let mut c = PixCanvas::new(60, 40).unwrap();
    w.draw(&mut c, 5, 5, 50, 30, false, false);
    // 预乘存储: shadowColor(0,0,0,42) → [0,0,0,42] 精确
    assert_eq!(px(&c, 17, 15), [0, 0, 0, 42], "纯影层轮廓 (主线垂距 2.23)");
    // 中心四层交叠 (srcOver 链: 42→77→244→254)
    assert_px_close(
        px(&c, 30, 20),
        [27, 254, 128, 254],
        "中心交点 = 双影+双前景",
    );
    // 单线三层: colorNum(27,255,128,240) SrcOver 影(0,0,0,42):
    // out_a = 240+42·15/255 ≈ 242.5, 通道 ≈ c·out_a/255 → [25.4,240,120.5]
    assert_px_close(px(&c, 18, 27), [25, 240, 120, 242], "副对角线单线叠色");
    assert_eq!(
        px(&c, 4, 4),
        [0, 0, 0, 0],
        "端点圆帽外 (距 (6,6) 帽缘 2.12 > 1.5)"
    );
    assert_eq!(px(&c, 5, 20), [0, 0, 0, 0], "远离线身");
}

/// aa=true (生产 graphAASetting 恒 ON) 冒烟: 线身核心覆盖率 1, 与非 AA 同值
/// (中心距两对角线 0.21/0.79 < half−0.5 = 1.0, AA 柔边不触及)
#[test]
fn warning_aa_smoke() {
    let mut w = WarningOverlay::new();
    let mut c = PixCanvas::new(60, 40).unwrap();
    w.draw(&mut c, 5, 5, 50, 30, false, true);
    assert!(any_nonzero(&c), "AA 开启时 X 有输出");
    assert_px_close(
        px(&c, 30, 20),
        [27, 254, 128, 254],
        "AA 中心与非 AA 同层叠色",
    );
}
