use super::*;

const REGULAR: &str = "../../fonts/sarasa-mono-sc-regular.ttf";

fn font(size: i32) -> LoadedFont {
    LoadedFont::new(std::path::Path::new(REGULAR), size).unwrap()
}

/// 读预乘 RGBA 像素 (overlays_field2 测试同约定)
fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
    let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
    [d[0], d[1], d[2], d[3]]
}

/// 直通色 → tiny-skia 预乘取整 ((c*a+127)/255), 断言基准用
fn premul(c: [u8; 4]) -> [u8; 4] {
    [
        ((c[0] as u32 * c[3] as u32 + 127) / 255) as u8,
        ((c[1] as u32 * c[3] as u32 + 127) / 255) as u8,
        ((c[2] as u32 * c[3] as u32 + 127) / 255) as u8,
        c[3],
    ]
}

/// 测试用喷气机 FM (3 高度档 × 3 速度档推力表; Java paintComponent 消费面:
/// velocityThr/maxThrAft/altitudeThr + alt/velThrNum)
fn jet_fmdata() -> FmData {
    let mut b = FmData::default();
    b.is_jet = true;
    b.vel_thr_num = 3;
    b.alt_thr_num = 3;
    let mut vt = [0.0; 30];
    vt[..3].copy_from_slice(&[100.0, 200.0, 400.0]);
    b.velocity_thr = Some(vt);
    let mut at = [0.0; 30];
    at[..3].copy_from_slice(&[0.0, 2000.0, 4000.0]);
    b.altitude_thr = Some(at);
    b.max_thr_aft = Some(vec![
        vec![3000.0, 2800.0, 2000.0],
        vec![2500.0, 2300.0, 1600.0],
        vec![2000.0, 1800.0, 1200.0],
    ]);
    b
}

// ---- chart_geometry: Java :557-592 公式逐式 基线 ----

/// 手算期望 (jet_blkx 输入): xn=[100,200,400] → xmin/xmax=100/400;
/// ymin=findMin(row2)=1200, ymax=findMax(row0)=3000 (对齐 10 后不变);
/// xgap=round(((401-100)/5)/5.0)*5 = round(12.0)*5 = 60;
/// ygap=round(((3001-1200)/5)/5.0)*5 = round(72.0)*5 = 360;
/// pxmin=100, pxmax=460, pymin=1200, pymax=3360;
/// ggx4=800/(460-100), ggy4=400/(3360-1200); rgbx=(int)(255/4)=63
#[test]
fn chart_geometry_oracle() {
    let g = chart_geometry(&jet_fmdata());
    assert_eq!((g.dwidth, g.dheight), (800, 400));
    assert_eq!(
        (g.xmin, g.xmax, g.ymin, g.ymax),
        (100.0, 400.0, 1200.0, 3000.0)
    );
    assert_eq!((g.xgap, g.ygap), (60, 360));
    assert_eq!((g.pxmin, g.pymin), (100, 1200));
    assert_eq!(g.ggx4, 800.0 / 360.0, "dwidth/(pxmax-pxmin)");
    assert_eq!(g.ggy4, 400.0 / 2160.0, "dheight/(pymax-pymin)");
    assert_eq!(g.rgbx, 63, "(int)(255.0f/4) 截断");
}

/// findMin/findMax 初值保真: 空数组返回 Java 初值 (Float.MAX_VALUE /
/// Float.MIN_VALUE=1.4e-45, 非 f32::MIN)
#[test]
fn find_min_max_empty_slice_returns_java_sentinels() {
    assert_eq!(find_min(&[]), f32::MAX as f64);
    assert_eq!(find_max(&[]), f64::from(f32::from_bits(1)));
    assert_eq!(find_min(&[5.0, -3.0, 9.0]), -3.0);
    assert_eq!(find_max(&[5.0, -3.0, 9.0]), 9.0);
}

/// java Math.round(float) 半-up 语义
#[test]
fn java_round_i32_half_up() {
    assert_eq!(java_round_f32(11.5), 12);
    assert_eq!(java_round_f32(11.4), 11);
    assert_eq!(java_round_f32(-11.5), -11, "floor(-11.5+0.5)=floor(-11)");
}

// ---- draw: 空缓存跳过 + 像素墨迹 (aa=false 精确断言) ----

/// 测试字体组 (12/16/18 三档, 同一 regular 文件)
fn dfs_fonts() -> (LoadedFont, LoadedFont, LoadedFont) {
    (font(12), font(16), font(18))
}

/// 无 FM / velThrNum==0 → paintComponent 直接 return (Java :554-555 null 守卫)
#[test]
fn draw_blank_without_fm_data() {
    let (f12, f16, f18) = dfs_fonts();
    let fonts = DfsFonts {
        num12: &f12,
        text16: &f16,
        text18: &f18,
        text12: &f12,
    };
    let mut cv = PixCanvas::new(900, 500).unwrap();
    DrawFrameSimpl::new().draw(&mut cv, &fonts, false);
    assert!(cv.pixmap().data().iter().all(|&b| b == 0), "无句柄全空");
    let mut b0 = jet_fmdata();
    b0.vel_thr_num = 0;
    let mut d = DrawFrameSimpl::new();
    d.reload_fm(Some(Arc::new(b0)));
    d.draw(&mut cv, &fonts, false);
    assert!(
        cv.pixmap().data().iter().all(|&b| b == 0),
        "velThrNum=0 跳过"
    );
}

/// 坐标系/数据点/图例的像素落点 (aa=false):
/// - x 轴 (50..850, y=460, 宽 3) 与 y 刻度 ii=0 (y=460) 同色叠 → 纯黑;
/// - 行1 首点 (v=100→px=50, thr=2500→py=219): dot fill_rect(49,218,2,2) 覆盖
///   y 轴黑底 (premul r=62 + 黑底 SrcOver 保 r);
/// - 图例行0 线段 (760..780, y=100, 宽 1, 灰 63) 无叠 → premul 精确
#[test]
fn draw_curve_pixels() {
    let (f12, f16, f18) = dfs_fonts();
    let fonts = DfsFonts {
        num12: &f12,
        text16: &f16,
        text18: &f18,
        text12: &f12,
    };
    let mut d = DrawFrameSimpl::new();
    d.reload_fm(Some(Arc::new(jet_fmdata())));
    let mut cv = PixCanvas::new(900, 500).unwrap();
    d.draw(&mut cv, &fonts, false);

    // x 轴内点: 黑 250 (与 y 刻度 ii=0 同色 SrcOver 叠加, alpha 只增不减)
    let axis = px(&cv, 200, 460);
    assert_eq!((axis[0], axis[1], axis[2]), (0, 0, 0), "x 轴纯黑");
    assert!(axis[3] >= 250, "alpha ≥ 250 (实测 {})", axis[3]);
    // y 轴 (x=50, y∈[60,460], 宽 3 → 列 49-51)
    let yaxis = px(&cv, 50, 300);
    assert_eq!((yaxis[0], yaxis[1], yaxis[2]), (0, 0, 0), "y 轴纯黑");
    assert!(yaxis[3] >= 250);

    // 行1 首点 dot (49,218): 灰 (1+1)*63=126 over 黑轴 → r 保持 premul 124
    let dot = px(&cv, 49, 218);
    assert_eq!(
        (dot[0], dot[1], dot[2]),
        (124, 124, 124),
        "数据点灰 126 直通的预乘"
    );
    assert!(dot[3] >= 250);
    // dot 下缘外一格仍是轴黑 (dot 恰 2×2)
    assert_eq!(px(&cv, 49, 220)[0], 0, "dot 外回轴黑");

    // 图例行0 线段 (760..780, y=100, 灰 63/α250, 透明底单覆盖)
    assert_eq!(px(&cv, 765, 100), premul([63, 63, 63, 250]), "图例线段");
    // 图例文本带 ("高度0m" @ 785 基线 105)
    assert!(
        (785..830).any(|x| px(&cv, x, 95)[3] > 0 || px(&cv, x, 104)[3] > 0),
        "图例文本墨迹"
    );
    // 标题带 ("推力-真空速曲线" @ x=450 基线 y=50, 字号 18)
    assert!(
        (300..620).any(|x| (30..50).any(|y| px(&cv, x, y)[3] > 0)),
        "标题墨迹"
    );
    // 画布全域有量级墨迹 (曲线 3 行 × 3 点 + 网格)
    let ink = cv
        .pixmap()
        .data()
        .chunks_exact(4)
        .filter(|p| p[3] > 0)
        .count();
    assert!(ink > 2000, "非零像素量级 (实测 {ink})");
}

// ---- spec 工厂 / DrawFrameSimplFeed 泵测试已随 W3 组件化退役 ----
// (挂载面 = widgets::fm_sidecar ThrustChartWidget 的 sidecar tick:
//  1000ms 节流 / displayFmKey==0 收腿退场 / 10s 自动 close / 会话脉冲,
//  数据推进链断言见 voidmei render_feeds; 本文件保留 DrawFrameSimpl 本体
//  的几何基线与像素墨迹测试)