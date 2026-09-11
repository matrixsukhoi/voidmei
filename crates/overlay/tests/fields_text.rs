//! overlays 域非渲染逻辑黑盒场景: warning 闪烁节奏 / compass 方位文本 /
//! bars 分档文本 / rows 行值与刻度选择。零像素零字体 — 不触碰 draw,
//! 只经 update/set_* 的纯数据面观察 (渲染质量归人工验收)。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


use expect_test::expect;

use overlay::overlays::bars::{FlapAngleBar, LinearGauge};
use overlay::overlays::compass::CompassGauge;
use overlay::overlays::rows::{AoaGauge, MechKind, MechPart, TickScale, MANEUVER_TICK_STEPS};
use overlay::overlays::warning::WarningBlinkHost;
use overlay::render::canvas::PixCanvas;

/// 1x1 离屏画布 (推进闪烁状态机的载体; 不做像素断言)
fn cv() -> PixCanvas {
    PixCanvas::new(1, 1).expect("1x1 画布构造")
}

// ---------------------------------------------------------------------
// warning: 闪烁节奏宿主
// ---------------------------------------------------------------------

/// 周期推导: (1000/interval)>>3, 0 钳 1 (long 整除 + >>3 + 截断链)
#[test]
fn warning_blink_周期推导() {
    let cases: Vec<(i64, i32)> = vec![
        (50, 2),   // 20>>3
        (100, 1),  // 10>>3
        (125, 1),  // 8>>3
        (124, 1),  // 1000/124=8>>3=1
        (500, 1),  // 2>>3=0 → 钳 1
        (2000, 1), // 0>>3=0 → 钳 1
    ];
    let actual = cases
        .iter()
        .map(|&(ms, _)| format!("{ms}ms -> {}", WarningBlinkHost::new(ms).blink_ticks()))
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        50ms -> 2
        100ms -> 1
        125ms -> 1
        124ms -> 1
        500ms -> 1
        2000ms -> 1"#]]
    .assert_eq(&actual);
}

/// 节奏状态机: blinkX=false 计数/相位冻结; true 每 blinkTicks 帧翻转
#[test]
fn warning_blink_节奏状态机() {
    // interval=50 → blinkTicks=2: 每 2 帧翻转一相
    let mut host = WarningBlinkHost::new(50);
    host.set_blink_x(false);
    let mut c = cv();
    for _ in 0..5 {
        host.draw_blink_x(&mut c, 10, 10, false);
    }
    expect!["off_phase=false (blinkX=false 冻结)"].assert_eq(&format!(
        "off_phase={} (blinkX=false 冻结)",
        host.is_blink_acting()
    ));

    // 使能后: 帧 1,2 推进计数; 第 2 帧末翻转 → 第 3 帧起 off 相位
    host.set_blink_x(true);
    host.draw_blink_x(&mut c, 10, 10, false); // tick=1
    expect!["tick1: off=false"].assert_eq(&format!("tick1: off={}", host.is_blink_acting()));
    host.draw_blink_x(&mut c, 10, 10, false); // tick=2 → 帧内翻转 (draw 先用旧相位)
    expect!["tick2: off=true (翻转发生在帧内)"].assert_eq(&format!(
        "tick2: off={} (翻转发生在帧内)",
        host.is_blink_acting()
    ));
    host.draw_blink_x(&mut c, 10, 10, false); // tick=3, off 相位 (只推进不画)
    expect!["tick3: off=true"].assert_eq(&format!("tick3: off={}", host.is_blink_acting()));
    host.draw_blink_x(&mut c, 10, 10, false); // tick=4 → 再翻转
    expect!["tick4: off=false"].assert_eq(&format!("tick4: off={}", host.is_blink_acting()));
    host.draw_blink_x(&mut c, 10, 10, false); // tick=5
    expect!["tick5: off=false"].assert_eq(&format!("tick5: off={}", host.is_blink_acting()));
}

// ---------------------------------------------------------------------
// compass: 方位文本换算 (%3.0f 右对齐) + 脏检查
// ---------------------------------------------------------------------

/// 航向读数 → " 35" 形态 (String.format("%3.0f")); mapGrid 直通行
#[test]
fn compass_方位文本换算() {
    let mut g = CompassGauge::new(20);
    let headings = [0.0, 5.4, 35.0, 90.0, 359.9, 360.0, 720.0];
    let actual = headings
        .iter()
        .map(|&h| {
            g.update(h, "A1");
            format!("{h} -> [{}]", g.line_compass())
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        0 -> [  0]
        5.4 -> [  5]
        35 -> [ 35]
        90 -> [ 90]
        359.9 -> [360]
        360 -> [360]
        720 -> [720]"#]]
    .assert_eq(&actual);

    // mapGrid 直通
    g.update(0.0, "B7");
    expect!["B7"].assert_eq(g.line_loc());

    // 脏检查: 同值重复 update 返回 false, 变化返回 true
    assert!(!g.update(0.0, "B7"), "同值不得置变化");
    assert!(g.update(1.0, "B7"), "航向变化应上报");

    // 坐标系模式: 离体默认, 可切随体 (视觉分支切换置脏)
    assert!(!g.inertial_mode());
    g.set_inertial_mode(true);
    assert!(g.inertial_mode());
    expect!["true dirty"].assert_eq(&format!("{} dirty", g.is_dirty()));
}

// ---------------------------------------------------------------------
// bars: 分档文本映射
// ---------------------------------------------------------------------

/// 襟翼角度对 → "%3.0f/%3.0f" 显示文本 (三色分档的数据面)
#[test]
fn flapbar_分档文本映射() {
    let mut bar = FlapAngleBar::new();
    // (当前角, 安全角) → 显示文本
    let cases: Vec<(f64, f64)> = vec![
        (0.0, 100.0),
        (19.6, 100.0),  // HALF_UP → 20
        (33.0, 100.0),
        (100.0, 100.0), // 顶格: 裕度 0
        (119.5, 100.0), // 超限: 显示原值
        (0.0, 90.0),
    ];
    let actual = cases
        .iter()
        .map(|&(cur, safe)| {
            bar.update(cur, safe);
            format!("{cur}/{safe} -> [{}]", bar.display_text())
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        0/100 -> [  0/100]
        19.6/100 -> [ 20/100]
        33/100 -> [ 33/100]
        100/100 -> [100/100]
        119.5/100 -> [120/100]
        0/90 -> [  0/ 90]"#]]
    .assert_eq(&actual);
    // 脏检查: 同值不置变化
    assert!(!bar.update(0.0, 90.0));
    assert!(bar.update(1.0, 90.0));
}

/// LinearGauge 更新链: 值/显示串变化上报, setter 族置脏
#[test]
fn linear_gauge_更新链() {
    let mut g = LinearGauge::new("THR", 100, false);
    assert!(g.update(50, "50%"), "初值后首次变化");
    assert!(!g.update(50, "50%"), "同值不得重复上报");
    assert!(g.update(50, "51%"), "显示串单独变化也上报");
    // max<=0 → pix 0 分支不 panic (draw 才消费, 数据面仅验证 update 容错)
    g.set_max_value(0);
    assert!(g.is_dirty(), "set_max_value 应置脏");

    // 风格缓存读取口
    g.set_style_context(200, 12);
    expect!["len200 th12"].assert_eq(&format!("len{} th{}", g.length_cache(), g.thickness_cache()));
}

// ---------------------------------------------------------------------
// rows: 行值/模板/刻度选择
// ---------------------------------------------------------------------

/// MechPart 模板槽: 襟翼空模板回退 "F100", 其余段空即空
#[test]
fn mech_part_模板回退() {
    let mut flaps = MechPart::new(MechKind::Flaps, 20);
    flaps.set_template("");
    expect!["F100"].assert_eq(&flaps.template);
    let mut brk = MechPart::new(MechKind::Airbrake, 20);
    brk.set_template("");
    expect![""].assert_eq(&brk.template);

    // update 脏检查: 文本与警告位全参与
    let mut gear = MechPart::new(MechKind::Gear, 20);
    assert!(gear.update("GEA", false));
    assert!(!gear.update("GEA", false), "同值不得重复上报");
    assert!(gear.update("GEA", true), "警告位单独变化也上报");
}

/// 机动条刻度档位点亮: 档 0 恒亮, 档 i 在 index ≥ 前档阈值时亮
#[test]
fn tick_scale_点亮档位表() {
    let scale = TickScale {
        ticks: [10, 20, 30, 40, 50],
    };
    let cases = [0.0, 0.05, 0.1, 0.15, 0.2, 0.35, 0.4, 0.9];
    let actual = cases
        .iter()
        .map(|&idx| {
            let lit: Vec<i32> = scale.lit_lens(idx).collect();
            format!("idx {idx} ({}): {:?}", format_steps(idx), lit)
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        idx 0 (≥0 阈值): [10]
        idx 0.05 (≥0 阈值): [10]
        idx 0.1 (≥1 阈值): [10, 20]
        idx 0.15 (≥1 阈值): [10, 20]
        idx 0.2 (≥2 阈值): [10, 20, 30]
        idx 0.35 (≥3 阈值): [10, 20, 30, 40]
        idx 0.4 (≥4 阈值): [10, 20, 30, 40, 50]
        idx 0.9 (≥5 阈值): [10, 20, 30, 40, 50]"#]]
    .assert_eq(&actual);
}

/// 阈值表即数据 (0.1~0.5 五档) — 防漂移钉死
#[test]
fn tick_scale_档位常量() {
    expect!["[0.1, 0.2, 0.3, 0.4, 0.5]"].assert_eq(&format!("{:?}", MANEUVER_TICK_STEPS));
    expect!["[0, 0, 0, 0, 0]"].assert_eq(&format!("{:?}", TickScale::default().ticks));
}

/// AoA 条长: ratio×length 截断 + 钳 rightDraw (负值不钳)
#[test]
fn aoa_条长计算与钳制() {
    let mut a = AoaGauge::new(20, 80, 2);
    let cases = [0.0, 0.5, 0.79, 0.85, 1.2, -0.5];
    let actual = cases
        .iter()
        .map(|&r| {
            a.set_aoa_from_ratio(r);
            format!("{r} -> aoaY {}", a.aoa_y)
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        0 -> aoaY 0
        0.5 -> aoaY 50
        0.79 -> aoaY 79
        0.85 -> aoaY 80
        1.2 -> aoaY 80
        -0.5 -> aoaY -50"#]]
    .assert_eq(&actual);
}

/// fmt 辅助: lit_lens 的阈值步进说明 (紧凑注释用)
fn format_steps(idx: f64) -> String {
    let n = MANEUVER_TICK_STEPS
        .iter()
        .filter(|&&t| idx >= t)
        .count();
    format!("≥{n} 阈值")
}
