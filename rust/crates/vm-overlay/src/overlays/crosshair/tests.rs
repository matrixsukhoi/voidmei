use super::*;

/// 读预乘 RGBA 像素 (与 gauges_bars/render2d 测试同约定)
fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
    let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
    [d[0], d[1], d[2], d[3]]
}

const RED: [u8; 4] = [255, 0, 0, 255]; // 影层测试色 (不透明 → 影专属像素可精确断言)
const GREEN: [u8; 4] = [0, 255, 0, 255]; // 前景层测试色

/// 标准被测件: width=40 @ (10,10) → 中心 (30,30), halfW=20, quarterW=10,
/// lineLen=40, strokeWidth=2 (影 3/前景 2), 自定义不透明双色
fn subject() -> (PixCanvas, CrosshairGauge) {
    let mut g = CrosshairGauge::new();
    g.set_style_context(40);
    g.set_colors(RED, GREEN);
    (PixCanvas::new(80, 80).unwrap(), g)
}

/// Java:91 strokeWidth 公式 (int 除法 + 下限 2)
#[test]
fn stroke_width_formula() {
    assert_eq!(CrosshairGauge::stroke_width(40), 2, "40/30=1 → max(1,2)=2");
    assert_eq!(CrosshairGauge::stroke_width(59), 2, "59/30=1 → 2");
    assert_eq!(CrosshairGauge::stroke_width(60), 2, "60/30=2 → 2");
    assert_eq!(CrosshairGauge::stroke_width(90), 3, "90/30=3");
    assert_eq!(CrosshairGauge::stroke_width(150), 5);
    assert_eq!(CrosshairGauge::stroke_width(600), 20);
    assert_eq!(CrosshairGauge::stroke_width(0), 2, "宽度 0 仍钳 2");
    assert_eq!(
        CrosshairGauge::stroke_width(-30),
        2,
        "负宽 Java max(-1,2)=2"
    );
}

/// preferred_size = width×width (:38-44 软件分支)
#[test]
fn preferred_size_software() {
    let mut g = CrosshairGauge::new();
    assert_eq!(g.preferred_size(), (0, 0), "构造默认 width=0");
    g.set_style_context(40);
    assert_eq!(g.preferred_size(), (40, 40));
    assert_eq!(g.id(), "gauge.crosshair");
}

/// 线臂几何: 行/列 30 上的臂本体为前景色; 中心与四向 quarter 间隙为空。
/// 像素中心距几何线 ≤1 (前景 stroke 2) / ≤1.5 (影 stroke 3) 判覆盖。
#[test]
fn crosshair_arms_and_center_gap() {
    let (mut c, mut g) = subject();
    g.draw(&mut c, 10, 10, false);
    // 中心: 四臂的 quarterW=10 间隙内, 距圆心 0.7 — 无任何图元
    assert_eq!(px(&c, 30, 30), [0, 0, 0, 0], "中心间隙为空");
    // 臂本体 (左/右/上/下, 均在 lineLen 覆盖内)
    assert_eq!(px(&c, 5, 30), GREEN, "左臂 (行30, x∈[-10,20])");
    assert_eq!(px(&c, 55, 30), GREEN, "右臂 (行30, x∈[40,70])");
    assert_eq!(px(&c, 30, 5), GREEN, "上臂 (列30, y∈[-10,20])");
    assert_eq!(px(&c, 30, 65), GREEN, "下臂 (列30, y∈[40,70])");
    // 水平/垂直两向的间隙 (quarterW=10 → [20,40] 空带, 圆帽覆盖不到)
    assert_eq!(px(&c, 25, 30), [0, 0, 0, 0], "水平间隙");
    assert_eq!(px(&c, 30, 25), [0, 0, 0, 0], "垂直间隙");
    assert_eq!(px(&c, 35, 35), [0, 0, 0, 0], "非臂非圆区");
}

/// 圆环几何: 半径 20 双层环 — 前景带 [19,21] / 影带 [18.5,21.5]。
/// 45° 方向 (44,44) 距心 20.51 = 前景; (45,44) 距心 21.23 = 仅影
/// (前景 3px 宽不足以覆盖, 1px 影轮廓透出); (45,45) 距心 21.92 = 环外空。
#[test]
fn crosshair_circle_two_layers() {
    let (mut c, mut g) = subject();
    g.draw(&mut c, 10, 10, false);
    assert_eq!(px(&c, 44, 44), GREEN, "环带前景 (径向 20.51)");
    assert_eq!(px(&c, 45, 44), RED, "影层轮廓 (径向 21.23, 前景外/影内)");
    assert_eq!(px(&c, 45, 45), [0, 0, 0, 0], "环外 (径向 21.92 > 21.5)");
    assert_eq!(px(&c, 9, 30), GREEN, "9 点方向环带 (径向 20.51)");
}

/// CAP_ROUND 圆帽: 端点沿方向外伸 stroke/2 — 右臂起点 (40,30) 的圆帽伸入
/// quarter 间隙 (39,30); 左臂终点 (20,30) 同理; 再外 1px (38/21) 为空。
#[test]
fn crosshair_round_caps_poke_into_gap() {
    let (mut c, mut g) = subject();
    g.draw(&mut c, 10, 10, false);
    assert_eq!(px(&c, 39, 30), GREEN, "右臂起点圆帽伸入间隙 (距端点 0.71)");
    assert_eq!(px(&c, 38, 30), [0, 0, 0, 0], "圆帽外 (距端点 1.58 > 1.5)");
    assert_eq!(px(&c, 20, 30), GREEN, "左臂终点圆帽 (距端点 0.71)");
    assert_eq!(px(&c, 21, 30), [0, 0, 0, 0], "圆帽外 (距端点 1.58 > 1.5)");
}

/// 奇数 width 的 int 截断 (:98-100): width=41 → halfW=20 (圆同 40 口径),
/// lineLen=41·4/4=41 → 右臂外端 cx+41=71 (width=40 时为 70)。
#[test]
fn crosshair_odd_width_int_truncation() {
    let mut g = CrosshairGauge::new();
    g.set_style_context(41);
    g.set_colors(RED, GREEN);
    let mut c = PixCanvas::new(80, 80).unwrap();
    g.draw(&mut c, 10, 10, false);
    assert_eq!(g.preferred_size(), (41, 41));
    // 中心 x+41/2 = 30 同 width=40; 圆 halfW=20 → (45,44) 仍为影轮廓
    assert_eq!(px(&c, 45, 44), RED, "圆半径仍取 41/2=20");
    assert_eq!(px(&c, 71, 30), GREEN, "右臂外端 cx+41=71 圆帽");
    assert_eq!(px(&c, 72, 30), [0, 0, 0, 0], "外端外 (距端点 1.58)");
}

/// 默认色 (:27-28): 前景不透明金黄盖影 → 纯 [255,215,8,255];
/// 影专属像素 = 预乘 [0,0,0,75] (黑色影预乘仅 alpha 通道)。
#[test]
fn crosshair_default_colors() {
    let mut g = CrosshairGauge::new();
    g.set_style_context(40);
    let mut c = PixCanvas::new(80, 80).unwrap();
    g.draw(&mut c, 10, 10, false);
    assert_eq!(px(&c, 5, 30), CROSSHAIR_FOREGROUND, "臂 = 前景金黄");
    assert_eq!(px(&c, 45, 44), [0, 0, 0, 75], "影轮廓 = Color(0,0,0,75)");
}

/// 退化 width=0: halfW=0 → stroke_circle r≤0 不绘制; 四臂退化为零长线。
/// 仅守护不 panic (Java 零长 CAP_ROUND 线画点 vs tiny-skia 行为未钉,
/// 生产中 0 尺寸组件不进布局, 不构成保真对象)。
#[test]
fn crosshair_zero_width_no_panic() {
    let mut g = CrosshairGauge::new();
    let mut c = PixCanvas::new(16, 16).unwrap();
    g.draw(&mut c, 0, 0, false);
}

/// aa=true (生产 graphAASetting 恒 ON) 冒烟: 几何仍在, 像素非空
#[test]
fn crosshair_aa_smoke() {
    let (mut c, mut g) = subject();
    g.draw(&mut c, 10, 10, true);
    assert!(
        c.pixmap().data().iter().any(|&b| b != 0),
        "AA 开启时准星有输出"
    );
    // 臂中线深核心仍为纯前景 (覆盖率 1 处 SrcOver 不透明源 = 本色)
    let p = px(&c, 55, 30);
    assert_eq!((p[0], p[1], p[2]), (0, 255, 0), "AA 臂核心仍纯绿");
}
