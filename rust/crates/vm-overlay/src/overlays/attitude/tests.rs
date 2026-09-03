use super::*;

const FONT: &str = "../../../fonts/sarasa-mono-sc-bold.ttf";

fn font() -> LoadedFont {
    LoadedFont::new(std::path::Path::new(FONT), 24).unwrap()
}

/// 读预乘 RGBA 像素 (与 gauges_bars/render2d 测试同约定)
fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
    let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
    [d[0], d[1], d[2], d[3]]
}

fn a(c: &PixCanvas, x: i32, y: i32) -> u8 {
    px(c, x, y)[3]
}

fn hud(pitch: f64, roll: f64, slip: f64, valid: bool) -> vm_core::derived::hud_data::HUDData {
    vm_core::derived::hud_data::Builder {
        pitch,
        roll,
        slip,
        pitch_valid: valid,
        ..Default::default()
    }
    .build()
}

/// 样式上下文: cd=20, cr=10, inner=12, lw=2, half=1 (MinimalHUDContext 典型比例)
fn gauge() -> AttitudeIndicatorGauge {
    let mut g = AttitudeIndicatorGauge::new();
    g.set_style_context(20, 10, 12, 2, 1, 24);
    g
}

// -----------------------------------------------------------------------
// 格式化 (Java String.format 语义)
// -----------------------------------------------------------------------

/// %3d: 右对齐宽 3 空格补; 负号占宽; 超宽原样
#[test]
fn fmt_d3_padding() {
    assert_eq!(fmt_d3(0), "  0");
    assert_eq!(fmt_d3(5), "  5");
    assert_eq!(fmt_d3(-7), " -7");
    assert_eq!(fmt_d3(123), "123");
    assert_eq!(fmt_d3(1234), "1234");
}

/// %-4.1f: 左对齐宽 4, HALF_UP 1 位小数
#[test]
fn fmt_f41_rounding_and_padding() {
    assert_eq!(fmt_f41(0.0), "0.0 ");
    assert_eq!(fmt_f41(3.25), "3.3 ", "精确 .25 二进制值 HALF_UP 进位");
    assert_eq!(fmt_f41(2.5), "2.5 ");
    assert_eq!(fmt_f41(9.96), "10.0", "进位自然超宽");
    assert_eq!(fmt_f41(15.0), "15.0");
    assert_eq!(fmt_f41(100.3), "100.3");
    assert_eq!(fmt_f41(f64::NAN), "NaN ");
    assert_eq!(fmt_f41(f64::INFINITY), "Infinity");
}

// -----------------------------------------------------------------------
// AttitudeIndicatorGauge — 双模式目标点与数据换算 (Java 公式手算)
// -----------------------------------------------------------------------

/// 双模式符号表 (Java:112-125 代码值):
/// cd=20 → radius=10, center=(40,50) (x=30,y=40)。
/// pitch=10 slip=5 → aosX=(int)(−5·96/30)=−16;
/// body (−1,−1): target=(40+(−1)(−16)·3/2, 50−(int)(10/2)) = (64,45);
/// earth (+1,+1): target=(40−24, 50+5) = (16,55)。
#[test]
fn indicator_gauge_dual_mode_target_points() {
    let mut g = gauge();
    assert!(g.on_data_update(&hud(10.0, 0.0, 5.0, true)));
    assert_eq!(g.aos_x, -16, "aosX = (int)(−slip·4·fontSize/30)");
    g.set_inertial_mode(false);
    assert_eq!(
        g.target_point(30, 40),
        (64, 45),
        "body: horizon 随 pitch 上移/侧滑右移"
    );
    g.set_inertial_mode(true);
    assert_eq!(g.target_point(30, 40), (16, 55), "earth: 符号全翻");
    // roll 符号: body=+1, earth=−1 (roll_theta)
    let mut zb = gauge();
    zb.on_data_update(&hud(0.0, 90.0, 0.0, true));
    assert!(
        (zb.roll_theta() - std::f64::consts::FRAC_PI_2).abs() < 1e-12,
        "body θ=+90°"
    );
    zb.set_inertial_mode(true);
    assert!(
        (zb.roll_theta() + std::f64::consts::FRAC_PI_2).abs() < 1e-12,
        "earth θ=−90°"
    );
}

/// onDataUpdate 的文本族 (Java:210-223):
/// - pitchValid=false → sAttitude 空 / roundHorizon=0
/// - pitchValid=true → "%3d" of |round(pitch)|
/// - slip → "%-4.1f" of |round(slip·10)/10|, roundSlip 仅保符号
#[test]
fn indicator_gauge_on_data_update_texts() {
    let mut g = gauge();
    g.on_data_update(&hud(7.6, 0.0, -2.44, true));
    assert_eq!(g.round_horizon, 8, "round(7.6)=8");
    assert_eq!(g.s_attitude, "  8");
    // slipValue = round(−24.4)/10 = −24/10 = −2.4 → 符号 −1, 文本 "2.4 "
    assert_eq!(g.round_slip, -1);
    assert_eq!(g.s_sideslip, "2.4 ");
    // pitch 无效
    g.on_data_update(&hud(7.6, 0.0, 0.0, false));
    assert_eq!(g.s_attitude, "");
    assert_eq!(g.round_horizon, 0);
    assert_eq!(g.s_sideslip, "0.0 ");
    // pitch 为负 → 文本色走 colorUnit 分支的 roundHorizon<0
    g.on_data_update(&hud(-3.0, 0.0, 0.0, true));
    assert_eq!(g.round_horizon, -3);
    assert_eq!(g.s_attitude, "  3");
}

// -----------------------------------------------------------------------
// AttitudeIndicatorGauge — marks 几何 (像素断言, 手算坐标)
// -----------------------------------------------------------------------

/// roll=0 (无旋转): 弧盒 (32,42,20,20) → 圆心 (42,52) r=10, 下半圆;
/// 细 w=2 带 r∈[9,11], 粗 w=4 带 r∈[8,12]; 顶部竖刻度 x=42 y∈[40,47];
/// 左右横刻度 y=52 x∈[30,32]/[52,54]。
#[test]
fn indicator_gauge_marks_geometry_roll0() {
    let mut g = gauge();
    g.on_data_update(&hud(0.0, 0.0, 0.0, true));
    let mut cv = PixCanvas::new(200, 200).unwrap();
    g.draw(&mut cv, 30, 40, None, false);

    // 弧底细带内 (d=10.5): colorNum(240) 叠 shade(42) ≈ 242
    assert!(
        (235..=250).contains(&a(&cv, 42, 62)),
        "弧底细带 d=10.5, a={}",
        a(&cv, 42, 62)
    );
    // 外侧粗独占环 (d=11.5 ∈ [11,12]): 单层 shade 精确 42
    assert_eq!(a(&cv, 42, 63), 42, "外粗独占环 d=11.5");
    // 粗带外 (d=12.6): 透明
    assert_eq!(a(&cv, 42, 64), 0, "粗带外 d=12.6");
    // 上半圆无弧 (d=9.9 但角度 +135° 不在跨度): 顶部竖刻度在 x=42 不在 (49,45)
    assert_eq!(a(&cv, 49, 45), 0, "上半圆无弧");
    // 顶部竖刻度 (x=42, y∈[40,47] 带)
    assert!(a(&cv, 42, 43) > 200, "顶部竖刻度");
    // 左横刻度 (x∈[30,32]+圆帽, y=52)
    assert!(a(&cv, 30, 52) > 200, "左横刻度");
    // 右横刻度
    assert!(a(&cv, 53, 52) > 200, "右横刻度");
    // 牵引线零长 → 圆点: 粗 r=2 + 细 r=1 的 colorLabel(166) 叠 shade(42) ≈ 181
    let dot = a(&cv, 40, 50);
    assert!((170..=195).contains(&dot), "牵引线圆点 a={dot}");
    // 弧圆心处无图形 (下半圆弧是环带)
    assert_eq!(a(&cv, 42, 52), 0, "弧圆心透明");
}

/// 滚转旋转 (body roll=90, θ=+90° 视觉顺时针):
/// 弧心 (42,52) 绕 target(40,50) 旋转 → (38,52), 角度跨度 [−180,0]−90 = [−270,−90]
/// → 左半圆 (6点→9点→12点); 顶部竖刻度旋到右侧水平 (43,52)→(50,52);
/// 右横刻度旋到竖直向下; 左横刻度旋到竖直向上。
#[test]
fn indicator_gauge_marks_rotation_body_roll90() {
    let mut g = gauge();
    g.on_data_update(&hud(0.0, 90.0, 0.0, true));
    let mut cv = PixCanvas::new(200, 200).unwrap();
    g.draw(&mut cv, 30, 40, None, false);

    // 左半圆 9 点 (d=9.5 from (38,52))
    assert!(a(&cv, 28, 52) > 200, "左半圆 9 点 (旋转后弧)");
    // 右下 45° (d=10.6) 不在跨度 → 无弧
    assert_eq!(a(&cv, 45, 59), 0, "右下象限无弧");
    // 顶部刻度旋到右侧水平 (43..50, 52)
    assert!(a(&cv, 46, 52) > 200, "顶刻度旋至右侧");
    // 右横刻度旋到 (38,62)-(38,64) 竖直向下
    assert!(a(&cv, 38, 63) > 200, "右刻度旋至向下");
    // 左横刻度旋到 (38,40)-(38,42) 竖直向上
    assert!(a(&cv, 38, 41) > 200, "左刻度旋至向上");
}

/// 惯性模式 roll=90 (θ=−90° 视觉逆时针): 弧心 → (42,48), 跨度 [−90,90]
/// → 右半圆 — 与 body 模式镜像。
#[test]
fn indicator_gauge_marks_rotation_inertial_roll90() {
    let mut g = gauge();
    g.set_inertial_mode(true);
    g.on_data_update(&hud(0.0, 90.0, 0.0, true));
    let mut cv = PixCanvas::new(200, 200).unwrap();
    g.draw(&mut cv, 30, 40, None, false);

    // 右半圆 3 点 (d=10.6 from (42,48), φ=−45° 在跨度)
    assert!(a(&cv, 49, 55) > 200, "右半圆右下 45° (惯性模式)");
    // 左上 135° 不在跨度 → 无弧 (d=10.6)
    assert_eq!(a(&cv, 34, 40), 0, "左上象限无弧");
}

/// 文本带 (Java:153-166): pitch 文本在 target 右侧 gap 起, slip 文本以
/// "888" 模板宽锁定左缘在 target 左侧; 基线 target_y−1。
/// x=100,y=80 → center (110,90); slip=−3.4 → aosX=10 → target=(95,84)。
#[test]
fn indicator_gauge_text_zones() {
    let mut g = gauge();
    g.on_data_update(&hud(12.0, 0.0, -3.4, true));
    let f = font();
    let mut cv = PixCanvas::new(220, 220).unwrap();
    g.draw(&mut cv, 100, 80, Some(&f), false);

    let (tx, ty) = (95, 84);
    // marks 最远到达 target+cr+hbs+粗帽 ≈ 110 → x≥115 的非零像素必为 pitch 文本
    let mut right = false;
    for yy in (ty - 24 - 2)..(ty + 8) {
        for xx in 115..160 {
            if a(&cv, xx, yy) > 0 {
                right = true;
            }
        }
    }
    assert!(right, "pitch 文本出现在 target 右侧带 (target_x={tx})");
    // marks 左侧最远 target−cr−hbs−粗帽 ≈ 80 → x≤78 的非零像素必为 slip 文本
    let mut left = false;
    for yy in (ty - 24 - 2)..(ty + 8) {
        for xx in 20..78 {
            if a(&cv, xx, yy) > 0 {
                left = true;
            }
        }
    }
    assert!(left, "slip 文本出现在 target 左侧带");
}

/// 脏检查 (W3 契约): 同值不脏, 变化置脏, draw 清脏, 模式切换置脏
#[test]
fn indicator_gauge_dirty_checking() {
    let mut g = gauge();
    assert!(g.on_data_update(&hud(1.0, 2.0, 3.0, true)));
    assert!(!g.on_data_update(&hud(1.0, 2.0, 3.0, true)), "同值不脏");
    assert!(g.is_dirty());
    let mut cv = PixCanvas::new(80, 80).unwrap();
    g.draw(&mut cv, 10, 10, None, false);
    assert!(!g.is_dirty(), "draw 后清脏");
    g.set_inertial_mode(true);
    assert!(g.is_dirty(), "模式切换置脏");
}

// -----------------------------------------------------------------------
// AttitudeOverlay — 点集几何 (Java drawTick 公式手算)
// -----------------------------------------------------------------------

/// w=150 h=300 pitch=0 roll=0: Pitch=150 → 地面多边形角点
/// (±225/±375, 150/1950); 刻度 4 条 y∈{−150,450,0,300} (±60°/±30°对)。
#[test]
fn overlay_polygon_and_ticks_roll0_pitch0() {
    let mut o = AttitudeOverlay::new();
    o.reinit(150, 300, false, false);
    o.update_telemetry(0.0, 0.0, 0.0, 0.0, 0.0, None);
    assert_eq!(o.pitch_y, 150, "Pitch = round((−0+30)·300/60)");
    assert_eq!(o.p_t[0], (-225, 150), "多边形左上 (−2w+w/2, 0+Pitch)");
    assert_eq!(o.p_t[1], (375, 150), "多边形右上");
    assert_eq!(o.p_t[2], (375, 1950), "多边形右下 (6h)");
    assert_eq!(o.p_t[3], (-225, 1950), "多边形左下");
    // 刻度对: i=0 → ±60° (y=∓300+150), i=1 → ±30° (y=∓150+150)
    assert_eq!(o.p_t[4], (-75, -150), "60° 刻度左端");
    assert_eq!(o.p_t[5], (225, -150), "60° 刻度右端");
    assert_eq!(o.p_t[6], (-75, 450), "−60° 刻度左端");
    assert_eq!(o.p_t[7], (225, 450));
    assert_eq!(o.p_t[8], (-75, 0), "30° 刻度左端");
    assert_eq!(o.p_t[9], (225, 0));
    assert_eq!(o.p_t[10], (-75, 300), "−30° 刻度左端");
    assert_eq!(o.p_t[11], (225, 300));
}

/// roll=90 (绕 (75,150) 视觉顺时针): 手算旋转+floor(x+0.5) 取整端点 —
/// 地平线竖直化 (75,−150)-(75,450), 地面在左 (x≤75)。
#[test]
fn overlay_polygon_roll90_endpoints() {
    let mut o = AttitudeOverlay::new();
    o.reinit(150, 300, false, false);
    o.update_telemetry(0.0, 0.0, 0.0, 90.0, 0.0, None);
    assert_eq!(o.p_t[0], (75, -150), "地平线左端点旋至正上方");
    assert_eq!(o.p_t[1], (75, 450), "地平线右端点旋至正下方");
    assert_eq!(o.p_t[2], (-1725, 450), "深地面角点");
    assert_eq!(o.p_t[3], (-1725, -150));
    // 60° 刻度线旋成竖直 x=375
    assert_eq!(o.p_t[4], (375, 0));
    assert_eq!(o.p_t[5], (375, 300));
}

/// pitch=+10: Pitch=round(20·5)=100 → 地平线上移到 y=100 (天地分界随俯仰);
/// aoa/aos 映射与航向分量。
#[test]
fn overlay_pitch_offset_and_mappings() {
    let mut o = AttitudeOverlay::new();
    o.reinit(150, 300, true, true);
    o.update_telemetry(5.0, -3.0, 10.0, 0.0, 180.0, Some((18.0, -4.0)));
    assert_eq!(o.pitch_y, 100, "Pitch = round((−10+30)·300/60) = 100");
    assert_eq!(o.p_t[0], (-225, 100), "地平线随 pitch 上移");
    assert_eq!(o.p_t[1], (375, 100));
    assert_eq!(o.aoa_y, 175, "AoA = round((5+30)·300/60)");
    assert_eq!(o.aos_x, 90, "AoS = round((3+15)·150/30)");
    // compass=180°: sin(π)≈1.2e−16 → compassX=0; cos(π)=−1 → compassY=−37 (w/4=37)
    assert_eq!(o.compass_x, 0);
    assert_eq!(o.compass_y, -37);
    // 攻角极限: U=round(48·5)=240, D=round(26·5)=130
    assert_eq!(o.aoa_limit_u, 240);
    assert_eq!(o.aoa_limit_d, 130);

    // 关闭开关 → 哨兵 −10 (画在窗口外)
    o.reinit(150, 300, true, false);
    o.update_telemetry(5.0, -3.0, 10.0, 0.0, 180.0, Some((18.0, -4.0)));
    assert_eq!(o.aoa_limit_u, AOA_LIMIT_OFF);
    assert_eq!(o.aoa_limit_d, AOA_LIMIT_OFF);
    // 无 FM 同哨兵
    o.reinit(150, 300, true, true);
    o.update_telemetry(5.0, -3.0, 10.0, 0.0, 180.0, None);
    assert_eq!(o.aoa_limit_u, AOA_LIMIT_OFF);
}

// -----------------------------------------------------------------------
// AttitudeOverlay — 像素级图层 (locater 绘制序)
// -----------------------------------------------------------------------

/// 图层与裁剪: 地面 colorUnit 下半、上半透明; 中线行; 侧滑球十字;
/// 攻角极限线 (开/关); 航向指针对; 窗口裁剪天然成立。
#[test]
fn overlay_draw_layers_pixels() {
    let mut o = AttitudeOverlay::new();
    o.reinit(150, 300, true, true);
    o.update_telemetry(5.0, 0.0, 0.0, 0.0, 0.0, Some((18.0, -4.0)));
    let mut cv = PixCanvas::new(150, 300).unwrap();
    o.draw(&mut cv, false);

    // 地面: (75,200) colorUnit a=220 预乘 RGB≈143
    let g = px(&cv, 75, 200);
    assert_eq!(g[3], 220, "地面 alpha=colorUnit 220");
    assert!(
        (g[0] as i32 - 143).abs() <= 1 && g[0] == g[1] && g[1] == g[2],
        "预乘灰 {g:?}"
    );
    // 天空: (75,100) 透明 (刻度不在该行, 中线在 149)
    assert_eq!(a(&cv, 75, 100), 0, "地平线上方透明");
    // 左外中线段 (0..18, 149) colorNum 240
    assert!(
        (230..=255).contains(&a(&cv, 5, 148)),
        "中线行 a={}",
        a(&cv, 5, 148)
    );
    // 侧滑球十字 (AoS=75, AoA=175): colorNum(240) 叠地面(220) ≈ 247
    assert!(a(&cv, 75, 174) > 240, "十字横臂叠地面");
    // 攻角极限 U=240 (warning 100 叠地面 220 ≈ 234), D=130 (叠天空 = 100)
    let u = a(&cv, 120, 240) as i32;
    assert!((225..=245).contains(&u), "上限线叠地面 a={u}");
    let d = a(&cv, 120, 130) as i32;
    assert!((90..=110).contains(&d), "下限线叠天空 a=100");
    // 航向指针对 (compass=0 → 竖直): 正向 colorNum 叠地面 / 反向 warning 叠天空
    assert!(a(&cv, 75, 180) > 240, "航向正向臂");
    let m = a(&cv, 75, 120) as i32;
    assert!((90..=110).contains(&m), "航向反向臂");
    // 地面延伸被窗口裁剪: 近右缘仍纯地面 (x=149 列有边框 shade 叠加, 避开)
    assert_eq!(a(&cv, 140, 200), 220, "多边形 ±2w 宽被画布裁剪");
    // 边框存在 (shade 弱 alpha; 取天空段避开地平线行的地面叠色)
    assert_eq!(a(&cv, 0, 140), 42, "左边框 shade");
    // pitch 刻度行 (60° 对旋平后 y=0) 与 30° 对 (y=300 界外被裁, y=0 可见)
    assert!(
        a(&cv, 120, 0) > 0,
        "30° 刻度行 y=0 (60° 对在 y=−150 已出界)"
    );
}

/// 极限线关闭 (哨兵 −10): 窗口内无 warning 横线
#[test]
fn overlay_aoa_limits_off_invisible() {
    let mut o = AttitudeOverlay::new();
    o.reinit(150, 300, false, false);
    o.update_telemetry(0.0, 0.0, 0.0, 0.0, 0.0, Some((18.0, -4.0)));
    let mut cv = PixCanvas::new(150, 300).unwrap();
    o.draw(&mut cv, false);
    assert_eq!(a(&cv, 120, 130), 0, "哨兵 −10 → 极限线不可见");
    assert_eq!(a(&cv, 120, 5), 0, "窗口上部无其他图形");
}

// -----------------------------------------------------------------------
// Blocker 回归: CAP_ROUND 端帽绕弧端点 (非弧心) — 孔内/弧心上方无杂散填充
// -----------------------------------------------------------------------

/// roll=0: 弧心 (42,52) r=10, 粗带 [8,12] 细带 [9,11]。
/// 孔内点 (35,52)/(49,52): 距弧心 <8 (带内缘内)、距弧端帽 (32,52)/(52,52) >2、
/// 距牵引圆点 (40,50) r=2 与三刻度线均 >2 —— 修复前端帽绕弧心的自相交轮廓
/// 在孔内产出杂散填充 (Java 该处为空), 修复后应为 0。
#[test]
fn indicator_gauge_arc_caps_hole_interior_clean() {
    let mut g = gauge();
    g.on_data_update(&hud(0.0, 0.0, 0.0, true));
    let mut cv = PixCanvas::new(200, 200).unwrap();
    g.draw(&mut cv, 30, 40, None, false);
    assert_eq!(a(&cv, 35, 52), 0, "孔内左 (距弧心 6.5 < 内缘 8)");
    assert_eq!(
        a(&cv, 49, 52),
        0,
        "孔内右 (距弧心 7.5 < 内缘 8, 距弧端帽 2.55>2)"
    );
}

/// roll=45: 弧心旋至 (40,52.83), 孔内深处的像素距弧带 (≥8)/牵引圆点 (r=2)/
/// 三条旋转刻度线 (粗帽 r=2) 均有余量 —— 修复前孔内出现 19 个杂散像素
/// (alpha 42~243, 距弧心最近 0.84px), 修复后全空。
#[test]
fn indicator_gauge_arc_caps_hole_interior_clean_rolled() {
    let mut g = gauge();
    g.on_data_update(&hud(0.0, 45.0, 0.0, true));
    let mut cv = PixCanvas::new(200, 200).unwrap();
    g.draw(&mut cv, 30, 40, None, false);
    for (x, y) in [(40, 53), (39, 52), (41, 54), (42, 52)] {
        assert_eq!(a(&cv, x, y), 0, "滚转 45° 弧孔内 ({x},{y})");
    }
}

// 注: 弧端帽的【外伸】无法用独立像素断言 —— Java 几何里弧两端点 (roll=0 时
// (32,52)/(52,52)) 与左右横刻度端点重合, 同为 CAP_ROUND 同宽的刻度 stadium 帽
// 与弧帽同圆心同半径, 弧帽永远被刻度帽完全覆盖 (实测 roll=90 时 (38,43) =
// 弧帽+刻度帽两遍 shade=77, 与 Java 合成一致)。端帽修复的可见差异仅在孔内
// 杂散清除, 由上面两个 hole_interior 测试钉死。

/// Blocker 回归: 中心下半圆弧心 = drawArc 盒角+半径 = (w/2−1, h/2−1) r=6
/// (stroke_arc 收圆心入参, 非盒角)。w=150 h=300 pitch=0 roll=0:
/// - 弧底 (74,155): 距正确圆心 6.5 ∈ 描边带 [4.5,7.5], colorNum 叠地面 ≈253
/// - (62,143): 距错误圆心 (68,143) 5.5 曾为弧体, 距正确圆心 13.2 带外 → 空
/// - (79,149): 弧体桥接中线右内段 (方帽起 x=81) 前的 2px —— 修复前错位弧
///   留出 Java 没有的断口
#[test]
fn overlay_center_arc_true_center() {
    let mut o = AttitudeOverlay::new();
    o.reinit(150, 300, false, false);
    o.update_telemetry(0.0, 0.0, 0.0, 0.0, 0.0, None);
    let mut cv = PixCanvas::new(150, 300).unwrap();
    o.draw(&mut cv, false);
    assert!(
        a(&cv, 74, 155) > 230,
        "弧底在正确圆心 (74,149) 下方, a={}",
        a(&cv, 74, 155)
    );
    assert!(a(&cv, 74, 154) > 230, "弧体径向 5.5 带内");
    assert_eq!(a(&cv, 62, 143), 0, "旧错误圆心 (68,143) 的弧体带内点已无弧");
    assert!(
        a(&cv, 79, 149) > 230,
        "弧端桥接中线右内段, a={}",
        a(&cv, 79, 149)
    );
}

/// live 工厂: DPI 缩放尺寸 (150%→round(150·1.5)=225/round(300·1.5)=450) +
/// 句柄喂入后 render 闭包画到新值 (共享 state 生效) + 注册键
#[test]
fn attitude_overlay_spec_dpi_and_shared_state() {
    let cell = Rc::new(RefCell::new(ReinitParams {
        dpi_scale: 1.5,
        ..Default::default()
    }));
    let (h, mut spec) = attitude_overlay_spec(&cell).unwrap();
    assert_eq!((spec.width, spec.height), (225, 450));
    assert_eq!(
        (spec.id.as_str(), spec.config_key.as_str()),
        ("enableAttitudeIndicator", "enableAttitudeIndicator")
    );
    // 喂入: aoa=10 → AoA = round((10+30)·450/60) = 300
    h.borrow_mut()
        .update_telemetry(10.0, 0.0, 0.0, 0.0, 0.0, None);
    assert_eq!(h.borrow().aoa_y, 300);
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv);
    // 侧滑球十字在 y=300 (colorNum 线体, BasicStroke(2) 行 299..300)
    assert!(
        a(&cv, 110, 299) > 0 || a(&cv, 110, 300) > 0,
        "十字随 aoa 喂入下移"
    );

    // WYSIWYG reinit: 宽 150→200 (150%) → 新尺寸 300×450 (setBounds 面)
    cell.borrow_mut().attitude.width = 200;
    let (w1, h1) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert_eq!((w1, h1), (300, 450));
    assert_eq!(
        (h.borrow().x_width, h.borrow().x_height),
        (300, 450),
        "state 已换新几何"
    );
}

/// CloseAllOverlays 数据面重置 (app_shell reset_handles_preview_values 调用面):
/// live 残留姿态点集/极限线 → reset_preview → 构造器数据初值, 几何保留。
/// 场景: 托盘 live→preview 后重开的预览窗地平仪不得冻结在上次 live 姿态
#[test]
fn attitude_reset_preview_clears_telemetry_state() {
    let cell = Rc::new(RefCell::new(ReinitParams::default()));
    let (h, _spec) = attitude_overlay_spec(&cell).unwrap();
    // live 残留: aoa/pitch/roll/极限线全量喂入 (非构造态)
    h.borrow_mut()
        .update_telemetry(10.0, 5.0, -20.0, 30.0, 90.0, Some((20.0, -8.0)));
    {
        let g = h.borrow();
        assert_ne!(g.aoa_y, 0, "aoa 喂入已离开构造态");
        assert_ne!(g.pitch_y, 0, "pitch 喂入已离开构造态");
        assert!(g.p_t.iter().any(|&p| p != (0, 0)), "姿态点集已生成");
    }
    let geo_before = {
        let g = h.borrow();
        (g.x_width, g.x_height, g.show_direction, g.show_aoa_limits)
    };
    // 重置 → 构造器数据值; 几何不动 (reinit 闭包职责)
    h.borrow_mut().reset_preview();
    let att = h.borrow();
    assert_eq!(
        (
            att.aos_x,
            att.aoa_y,
            att.pitch_y,
            att.compass_x,
            att.compass_y
        ),
        (0, 0, 0, 0, 0)
    );
    assert_eq!(
        (att.aoa_limit_u, att.aoa_limit_d),
        (AOA_LIMIT_OFF, AOA_LIMIT_OFF)
    );
    assert!(att.p_t.iter().all(|&p| p == (0, 0)), "姿态点集清空");
    assert!(att.is_dirty(), "重置标脏 (强制下一帧重绘)");
    let geo_after = (
        att.x_width,
        att.x_height,
        att.show_direction,
        att.show_aoa_limits,
    );
    assert_eq!(geo_before, geo_after, "几何保留 (reinit 面不动)");
}
