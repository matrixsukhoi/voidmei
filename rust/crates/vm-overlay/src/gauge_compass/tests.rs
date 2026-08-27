use super::*;

/// 北三角基数象限几何 (CompassGauge.java:185-213):
/// r=25 → height=(int)8.75=8, halfBase=(int)7.499…=7, tipDist=33;
/// 角 0: tip 正上 (60,27), 底边坐圆顶 (60,35) 切向展开 ±7;
/// 角 π/2: tip 正右 (93,60), 底边 (85,60) 竖直展开 ±7
#[test]
fn north_triangle_geometry_cardinal() {
    let pts = north_triangle(60, 60, 25, 0.0);
    assert_eq!(pts[0], (60, 27), "tip 在圆外 33px 处 (北)");
    assert_eq!(pts[1], (67, 35), "corner1 切向 +7 (圆顶)");
    assert_eq!(pts[2], (53, 35), "corner2 切向 -7");

    let pts = north_triangle(60, 60, 25, std::f64::consts::FRAC_PI_2);
    assert_eq!(pts[0], (93, 60), "tip 正右 (东)");
    assert_eq!(pts[1], (85, 67), "corner1 竖直向下");
    assert_eq!(pts[2], (85, 53), "corner2 竖直向上");
    // cos(π/2)=6.12e-17 → (int)(33·6.12e-17)=0, 三角形纯轴向
    assert_eq!(pts[0].1, 60);
}

/// 三角尺寸的 (int) 截断 (Java:187-188): f64 字面量乘积的舍入必须逐值核对 —
/// r=20: 20×0.35 的精确积恰在 7.0 半 ulp 平局点, IEEE 取偶舍到 7.0 (Java 同)
/// → height=7; 20×0.30 舍到 6.0 → halfBase=6; tipDist=27。
/// r=25: 25×0.35 → 8.749…→8, 25×0.30 → 7.499…→7
#[test]
fn north_triangle_size_truncation() {
    let pts = north_triangle(0, 0, 20, 0.0);
    assert_eq!(pts[0], (0, -27), "r=20 tipDist = 20+7 = 27 (height 舍入为 7)");
    assert_eq!(pts[1], (6, -20), "halfBase 6");
    assert_eq!(pts[2], (-6, -20));

    let pts = north_triangle(0, 0, 25, 0.0);
    assert_eq!(pts[0], (0, -33), "r=25 tipDist=33 (height 8)");
    assert_eq!(pts[1], (7, -25), "halfBase 7");
}

/// Java (int) 向零截断 (非 floor): 微小负 sin 分量截为 0 而非 -1
/// (tip.x 与 corner 的切向 y 分量均含 (int)负微小量 = 0)
#[test]
fn int_cast_truncates_toward_zero() {
    let pts = north_triangle(100, 100, 20, -1e-9);
    assert_eq!(pts[0].0, 100, "tip.x: (int)(27·sin(-1e-9)) = (int)(-2.7e-8) = 0");
    assert_eq!(pts[0].1, 73, "tip.y: 100 - 27");
    assert_eq!(pts[1].1, 80, "corner1.y: baseY + (int)(6·sin(-1e-9)) = 80 + 0");
    assert_eq!(pts[2].0, 94, "corner2.x: base - 切向半宽 6");
}

/// 指针几何 (update 派生 + Java:151-154), r=25:
/// 0°: tip (60,45)/end (60,28); 90°: tip (75,60)/end (92,60); 180°: tip (60,75)/end (60,92)。
/// 90°/180° 验证 f32 化 compassRads 后 sin 误差 (≤1e-7) 不翻越整型边界,
/// 以及 (1.3f·25)=32.499…→32 / (0.618·25)=15.45→15 / 微小量向零截 0
#[test]
fn update_pointer_geometry_cardinals() {
    let mut g = CompassGauge::new(25);
    g.set_style_context(25, 3, 24, 12);

    g.update(0.0, "C4");
    assert_eq!((g.compass_dx, g.compass_dy), (0, 32));
    assert_eq!(pointer_tip(60, 60, 25, g.compass_rads), (60, 45));
    assert_eq!((60 + g.compass_dx, 60 - g.compass_dy), (60, 28), "指针朝北");

    g.update(90.0, "C4");
    assert_eq!((g.compass_dx, g.compass_dy), (32, 0), "sin(1.5707964f32)≈1 → 32, cos≈4.4e-8 → 0");
    assert_eq!(pointer_tip(60, 60, 25, g.compass_rads), (75, 60), "指针朝东");

    g.update(180.0, "C4");
    assert_eq!((g.compass_dx, g.compass_dy), (0, -32), "sin(πf32)=-8.7e-8 → 0, cos→-32");
    assert_eq!(pointer_tip(60, 60, 25, g.compass_rads), (60, 75), "指针朝南");
    assert_eq!((60 + g.compass_dx, 60 - g.compass_dy), (60, 92));
}

/// 随体模式固定机标线 (Java:138-139): (int)(0.618·25)=15, (int)(1.3·25)=32
/// (此处 1.3 是 double 字面量, 32.5 截 32)
#[test]
fn fixed_segment_geometry() {
    let ((tx, ty), (ex, ey)) = fixed_segment(60, 60, 25);
    assert_eq!((tx, ty), (60, 45), "tip = cy - 15");
    assert_eq!((ex, ey), (60, 28), "end = cy - 32");
    // 长度与离体 0° 指针一致 (15→32 同区间)
    let mut g = CompassGauge::new(25);
    g.update(0.0, "");
    assert_eq!((tx, ty), pointer_tip(60, 60, 25, g.compass_rads));
}

/// 文本基线位置 (Java:167-171) 含 int 除法向零截断:
/// r=25, big=24 → (r-big)/2 = 0; r=37 → 13/2 = 6
#[test]
fn label_positions_int_division() {
    let (compass, loc) = label_positions(30, 10, 25, 2, 24, 12);
    assert_eq!(compass, (35, 34), "y = 10+24-(25-24)/2 = 34");
    assert_eq!(loc, (35, 65), "y = 10+25+12/2+24 = 65");

    let (compass, _) = label_positions(30, 10, 37, 2, 24, 12);
    assert_eq!(compass, (35, 28), "(37-24)/2 = 6 (向零截断)");
}

/// %3.0f 航向格式 (Java:95): HALF_UP / 宽 3 右对齐 / 负零保号 / NaN
#[test]
fn fmt_heading3_rounding() {
    assert_eq!(fmt_heading3(5.0), "  5");
    assert_eq!(fmt_heading3(359.6), "360", "HALF_UP 进位自然超宽");
    assert_eq!(fmt_heading3(0.5), "  1");
    assert_eq!(fmt_heading3(-0.4), " -0", "负值舍到零保负号");
    assert_eq!(fmt_heading3(-0.0), " -0");
    assert_eq!(fmt_heading3(f64::NAN), "NaN");
    assert_eq!(fmt_heading3(0.49999999999999994), "  0", "精确十进制舍入");
}

/// %3.0f 非有限与超 i64 域值 (畸形遥测 org.json "1e999"→inf / "1e19" 路径):
/// Formatter 输出 "Infinity"/"-Infinity"/完整十进制, 不得出现 as i64 饱和串。
/// 1e19/2^63 均为整值 double, 精确十进制展开无舍入分歧
#[test]
fn fmt_heading3_infinite_and_huge() {
    assert_eq!(fmt_heading3(f64::INFINITY), "Infinity");
    assert_eq!(fmt_heading3(f64::NEG_INFINITY), "-Infinity");
    assert_eq!(fmt_heading3(1e19), "10000000000000000000");
    assert_eq!(fmt_heading3(-1e19), "-10000000000000000000");
    assert_eq!(fmt_heading3(9_223_372_036_854_775_808.0), "9223372036854775808");
}

/// 双模式语义 (Java:117-123 / 34-37): 离体北三角角恒 0, 随体 = -compassRads
#[test]
fn mode_semantics_north_angle() {
    let mut g = CompassGauge::new(25);
    g.update(90.0, "");
    assert_eq!(north_angle(false, g.compass_rads), 0.0, "离体: 北固定 12 点钟");
    assert!(
        (north_angle(true, g.compass_rads) + g.compass_rads as f64).abs() < 1e-12,
        "随体: 北三角转 -compassRads"
    );
    assert!(!g.inertial_mode(), "默认离体 (Java:37)");
    g.set_inertial_mode(true);
    assert!(g.inertial_mode());
    assert!(g.is_dirty(), "模式切换置脏");
    assert_eq!(g.id(), "gauge.compass");
    assert_eq!(g.preferred_size(), (50, 50), "preferred = 2r×2r (Java:58-60)");
}

/// NaN 航向 (地图方向无效时 0/0 → NaN): Java (int)NaN=0 → dx/dy 归 0,
/// 指针退化为 tip(0.618r 方向亦 NaN→0)=(cx,cy) 到 (cx,cy) 的零长度线;
/// 文本 "NaN"
#[test]
fn nan_heading_semantics() {
    let mut g = CompassGauge::new(25);
    assert!(g.update(f64::NAN, ""));
    assert_eq!((g.compass_dx, g.compass_dy), (0, 0), "(int)NaN = 0");
    assert!(g.compass_rads.is_nan());
    assert_eq!(g.line_compass(), "NaN");
    assert_eq!(pointer_tip(60, 60, 25, g.compass_rads), (60, 60));
}

/// 脏检查: 同值不脏, 变化置脏, draw 清脏; set_style_context 同值不置脏
#[test]
fn dirty_checking_semantics() {
    let mut g = CompassGauge::new(25);
    assert!(g.update(123.4, "B2"));
    assert!(!g.update(123.4, "B2"), "同航向同网格不脏");
    assert!(g.is_dirty());
    let mut cv = PixCanvas::new(120, 120).unwrap();
    g.draw(&mut cv, 10, 10, None, true);
    assert!(!g.is_dirty(), "draw 后清脏");
    g.update(124.0, "B2");
    assert!(g.is_dirty(), "航向变化置脏");
    g.draw(&mut cv, 10, 10, None, true);
    g.set_style_context(25, 3, 24, 12);
    assert!(g.is_dirty(), "风格变化置脏");
    g.draw(&mut cv, 10, 10, None, true);
    g.set_style_context(25, 3, 24, 12);
    assert!(!g.is_dirty(), "同值风格不置脏");
}

/// 渲染冒烟: 图层序像素采样 (heading=180 → 指针在下半, 不污染北三角采样)。
/// 预乘存储期望: colorUnit [166·220/255≈143]³;
/// 圆环/指针处 shade 底层在下 (双层 stroke 同心), alpha = SrcOver(240,42)≈242
#[test]
fn render_smoke_layer_order() {
    /// Java2D SrcOver 直通域合成后的 alpha (同 gauges_bars 测试式)
    fn src_over_a(fg: u8, bg: u8) -> u8 {
        let fa = fg as f32 / 255.0;
        let fda = bg as f32 / 255.0;
        ((fa + fda * (1.0 - fa)) * 255.0 + 0.5) as u8
    }
    let mut g = CompassGauge::new(25);
    g.set_style_context(25, 3, 24, 12);
    g.update(180.0, "C4");
    let mut cv = PixCanvas::new(120, 120).unwrap();
    g.draw(&mut cv, 10, 10, None, false);
    // 圆心 (35,35): 北三角内部 (35,6) — 距心 28.5 > 外环外缘 27.5, 纯 colorUnit 填充
    let d = |x: i32, y: i32| {
        let i = ((y * cv.width() + x) * 4) as usize;
        cv.pixmap().data()[i..i + 4].to_vec()
    };
    let tri = d(35, 6);
    for (got, want) in tri.iter().zip([143u8, 143, 143, 220]) {
        assert!(
            (i32::from(*got) - i32::from(want)).abs() <= 2,
            "北三角填充 ≈{:?} (期望 ~{want})",
            tri
        );
    }
    // 圆环右点 (60,35): 距心 25.5 ∈ num 环 [23.5,26.5] 内部, shade 外环垫底
    let ring_alpha = src_over_a(240, 42);
    let ring = d(60, 35);
    for (got, want) in ring.iter().zip([25u8, 240, 120, ring_alpha]) {
        assert!(
            (i32::from(*got) - i32::from(want)).abs() <= 2,
            "圆环 num 层 ≈{:?} (期望 ~{want}, num 叠 shade)",
            ring
        );
    }
    // 指针列 (35,55): 180° 指针 (35,50)-(35,67), shade 宽 5 垫底 + num 宽 3 在上
    let ptr = d(35, 55);
    for (got, want) in ptr.iter().zip([25u8, 240, 120, ring_alpha]) {
        assert!(
            (i32::from(*got) - i32::from(want)).abs() <= 2,
            "指针 num 层 ≈{:?} (期望 ~{want}, num 叠 shade)",
            ptr
        );
    }
    // 图层序: (35,9) 在北三角内部 (距心 25.5, 同时被圆环双层覆盖) —
    // 绿色 num 环压在灰色三角上 → g 通道远大于 r 通道, 且 alpha 高于纯三角 220
    let over = d(35, 9);
    assert!(
        i32::from(over[1]) - i32::from(over[0]) > 100,
        "圆环 (绿) 盖住三角 (灰): {:?}",
        over
    );
    assert!(over[3] > 220, "叠层后 alpha 高于纯三角 220: {:?}", over);
}
