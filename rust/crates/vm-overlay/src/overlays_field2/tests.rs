use super::*;
use std::cell::RefCell;
use std::collections::HashMap;

const BOLD: &str = "../../../fonts/sarasa-mono-sc-bold.ttf";
const REGULAR: &str = "../../../fonts/sarasa-mono-sc-regular.ttf";

fn font(path: &str, size: i32) -> LoadedFont {
    LoadedFont::new(std::path::Path::new(path), size).unwrap()
}

/// 读预乘 RGBA 像素 (与 overlay_list/render2d 测试同约定)
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

// ---- java_format_f / java_string_format: Java 8 oracle 对拍 ----

/// HALF_UP on 最短往返十进制 (Java Formatter 语义) vs Rust 半偶的判别值
#[test]
fn java_format_f_half_up_oracle() {
    // String.format("%.1f", 5.25) = "5.3" (Rust {:.1} 半偶 → "5.2")
    assert_eq!(java_format_f(5.25, 1), "5.3");
    // String.format("%.2f", 2.675) = "2.68" (Rust → "2.67")
    assert_eq!(java_format_f(2.675, 2), "2.68");
    // String.format("%.0f", 0.5) = "1" / (2.5) = "3" (Rust → 0 / 2)
    assert_eq!(java_format_f(0.5, 0), "1");
    assert_eq!(java_format_f(2.5, 0), "3");
    // 最短表示 2.675 的 %.1f = "2.7" (vm-core java_format_f1 文档 oracle)
    assert_eq!(java_format_f(2.675, 1), "2.7");
}

/// 常规/负数/补零/NaN/-0.0/整域
#[test]
fn java_format_f_domains() {
    assert_eq!(java_format_f(3050.0, 1), "3050.0");
    assert_eq!(java_format_f(-8.4, 1), "-8.4");
    assert_eq!(java_format_f(9.0, 2), "9.00");
    assert_eq!(java_format_f(0.105, 3), "0.105");
    assert_eq!(java_format_f(-0.04, 1), "-0.0", "负号保留 (Java Formatter)");
    assert_eq!(java_format_f(f64::NAN, 1), "NaN");
    assert_eq!(java_format_f(f64::INFINITY, 0), "Infinity");
    // 巨整数域: 1e26 → 全整数 + ".0"
    assert_eq!(java_format_f(1e26, 1), "100000000000000000000000000.0");
    // 小数 |x|<1 的 prec=0
    assert_eq!(java_format_f(0.49999999999999994, 0), "0");
}

/// java_string_format: %s/%d/%.Nf 顺序展开 + %% 字面 (bFlapRestrict 模板)
#[test]
fn java_string_format_engine() {
    let t = "襟翼限速(km/h)%d: %.0f%% / %.0f\n";
    assert_eq!(
        java_string_format(t, &[FmtArg::D(1), FmtArg::F(95.0), FmtArg::F(640.0)]),
        "襟翼限速(km/h)1: 95% / 640\n"
    );
    assert_eq!(
        java_string_format("FM文件: %s - %s", &[FmtArg::S("a"), FmtArg::S("b")]),
        "FM文件: a - b"
    );
    // %s 收 null 字段 → "null"
    assert_eq!(java_string_format("V: %s", &[FmtArg::S("null")]), "V: null");
}

/// 模板/实参错配 → panic (Java UnknownFormatConversionException /
/// MissingFormatArgumentException 的崩溃语义)
#[test]
#[should_panic]
fn java_string_format_missing_arg_panics() {
    let _ = java_string_format("%s %s", &[FmtArg::S("a")]);
}

/// %d 位点收浮点实参 → panic (Java IllegalFormatConversionException;
/// 曾静默 `v as i64` 输出与 doc "两语言同为崩溃语义" 矛盾, 已对齐)
#[test]
#[should_panic(expected = "IllegalFormatConversionException")]
fn java_string_format_f_at_d_panics() {
    let _ = java_string_format("%d", &[FmtArg::F(1.5)]);
}

/// %d 位点收字符串实参 → panic (Java 同抛 IllegalFormatConversionException)
#[test]
#[should_panic(expected = "IllegalFormatConversionException")]
fn java_string_format_s_at_d_panics() {
    let _ = java_string_format("%d", &[FmtArg::S("x")]);
}

/// addLines 的 Java trim 语义: 只剥 ≤ U+0020, 全角空格 U+3000 保留
/// (Rust `str::trim` 会多剥一层 — 域内不可达, 本测试锁定复刻边界)
#[test]
fn add_lines_java_trim_semantics() {
    let mut lines = Vec::new();
    add_lines(&mut lines, "a\u{3000}  \nb\u{3000}\n  \t\n");
    assert_eq!(lines, vec!["a\u{3000}".to_string(), "b\u{3000}".to_string()]);
}

// ---- ControlSurfacesOverlay ----

/// init/reinitConfig 几何公式 (Java :225-271, :107-111):
/// fontSize=24 → width=144, rudderValPix=108, twidth=240, theight=180,
/// locate=4, stroke=2; enableAxisEdge 加 sw=10
#[test]
fn control_surfaces_geometry() {
    let mut ov = ControlSurfacesOverlay::new();
    ov.init(0, 1.0, false, 30, 40, true);
    assert_eq!(ov.font_size, 24);
    assert_eq!(ov.label_font_size, 12, "Math.round(24/2.0f)");
    assert_eq!((ov.width, ov.height), (144, 144));
    assert_eq!(ov.rudder_val_pix, 108, "(50+100)*144/200 初值");
    assert_eq!((ov.content_width, ov.content_height), (240, 180), "(int)(144+96)/(int)(144+36)");
    assert_eq!(ov.shade_width, 0);
    assert_eq!((ov.total_width, ov.total_height), (240, 180));
    assert_eq!((ov.px, ov.py), (72, 72));
    assert_eq!((ov.locate_size, ov.stroke_size), (4, 2));
    assert_eq!((ov.lx, ov.ly), (30, 40), "OverlaySettings 坐标透传");
    assert!(ov.has_service, "游戏模式");
    // 初值 50 (Java :91-94)
    assert_eq!(ov.elevator_num, "50");
    assert_eq!(ov.wing_sweep_num, "50");

    // enableAxisEdge: sw=10 外扩 (Java :250-256)
    let mut ov2 = ControlSurfacesOverlay::new();
    ov2.init(0, 1.0, true, 0, 0, false);
    assert_eq!(ov2.shade_width, 10);
    assert_eq!((ov2.total_width, ov2.total_height), (260, 200));
    assert!(!ov2.has_service, "preview: s == null");

    // fontadd=-6 → fontSize 18, width 108, twidth (int)(108+72)=180
    let mut ov3 = ControlSurfacesOverlay::new();
    ov3.init(-6, 1.0, false, 0, 0, false);
    assert_eq!(ov3.font_size, 18);
    assert_eq!(ov3.width, 108);
    assert_eq!(ov3.content_width, 180);
    assert_eq!(ov3.content_height, (108.0 + 27.0) as i32, "135");

    // 奇数字号: fontSize 25 (dpi 校准) → label = Math.round(12.5f) = 13
    let mut ov4 = ControlSurfacesOverlay::new();
    ov4.reinit_config(1, 1.0, false, 0, 0);
    assert_eq!(ov4.font_size, 25);
    assert_eq!(ov4.label_font_size, 13);
}

/// onFlightData: 50ms 节流 + preview 不更新数据 + 游标/条位换算 (Java :280-312)
#[test]
fn control_surfaces_throttle_and_mapping() {
    let mut ov = ControlSurfacesOverlay::new();
    ov.init(0, 1.0, false, 0, 0, true);

    // 首事件: lastRefreshTime=0 → 0-0 < 50 恒真 → 被跳过? Java 同:
    // 初值 0, now=0 时 0-0=0 < 50 → skip。用 now=100 起测
    assert!(!ov.on_flight_data(0, 0.0, 0.0, 0.0, 0.0, false), "0-0 < 50 跳过 (Java 同)");
    assert!(ov.on_flight_data(100, -100.0, 100.0, 0.0, 0.85, true));
    assert_eq!((ov.px, ov.py), (0, 144), "副翼 -100 → 左缘; 升降舵 100 → 底缘");
    assert_eq!(ov.rudder_val_pix, 72, "方向舵 0 → 中位");
    assert_eq!(ov.wing_sweep_num, "85", "可变翼 0.85 → 85 (isWingSweepValid)");
    assert_eq!(ov.elevator_num, "100");
    assert_eq!(ov.aileron_num, "-100");

    // 节流: +30ms 跳过, +50ms 放行
    assert!(!ov.on_flight_data(130, 0.0, 0.0, 0.0, 0.0, false));
    assert!(ov.on_flight_data(150, 50.7, -25.9, 100.0, -65535.0, false));
    // (int) 截断向零: 50.7→50, -25.9→-25
    assert_eq!((ov.px, ov.py), ((100 + 50) * 144 / 200, (100 - 25) * 144 / 200));
    assert_eq!(ov.rudder_val_pix, 144, "满舵 → 全宽");
    assert_eq!(ov.aileron_num, "50");
    assert_eq!(ov.elevator_num, "-25");
    assert_eq!(ov.wing_sweep_num, "0", "wsweep -65535 无效标记 → 0");

    // preview (s == null): 返回 true (repaint 恒调度) 但数据保持
    let mut pv = ControlSurfacesOverlay::new();
    pv.init_preview(0, 1.0, false, 0, 0);
    assert!(pv.on_flight_data(100, -100.0, -100.0, -100.0, 0.5, true));
    assert_eq!((pv.px, pv.py), (72, 72), "初值 50 → 中心");
    assert_eq!(pv.rudder_val_pix, 108, "初值条位");
    assert_eq!(pv.rudder_num, "50");
}

/// draw 像素: 边框/十字影子+主十字/横条+游标 (alpha=255 语义区, 预乘=直通)
#[test]
fn control_surfaces_draw_pixels() {
    let mut ov = ControlSurfacesOverlay::new();
    ov.init(0, 1.0, false, 0, 0, true);
    let f_num = font(BOLD, 24);
    let f_label = font(BOLD, 12);
    let f_unit = font(REGULAR, 12);
    let fonts = CsFonts { num: &f_num, label: &f_label, unit: &f_unit };
    let mut cv = PixCanvas::new(240, 180).unwrap();
    ov.draw(&mut cv, &fonts, false);

    // 边框 (BasicStroke(1), colorShadeShape): 四角 — Java 4 条 drawLine 各自独立
    // 描边, 角点被两条线 SrcOver 叠两次: 42 + 213·42/255 ≈ 77 (Java 同值)
    let corner_blend = [0u8, 0, 0, 77];
    for (x, y) in [(0, 0), (143, 0), (0, 143), (143, 143)] {
        assert_eq!(px(&cv, x, y), corner_blend, "边框角双叠 ({x},{y})");
    }
    assert_eq!(px(&cv, 0, 72), colors().shade_shape, "左边框中点");
    // 边框外无字
    assert_eq!(px(&cv, 60, 60), [0, 0, 0, 0], "十字区中心空");

    // 主十字 (colorNum, 中心 (72,72) 偏移 -1, 线宽 2): 六条独立 drawLine 的
    // 描边互相交叠 — 断言取**单笔画覆盖**点 (Java 同样叠出混合 alpha):
    // 主横线 y=71 (行 70/71 实心, 臂 x∈[68,73]); 主竖线 x=71 (列 70/71 实心)
    assert_eq!(px(&cv, 69, 70), premul(colors().num), "主横线单覆盖点 (69,70)");
    assert_eq!(px(&cv, 70, 69), premul(colors().num), "主竖线单覆盖点 (70,69)");
    // 主线交叠中心 (行70/71 × 列70/71): 240+240·15/255 → 饱和 255
    assert_eq!(px(&cv, 70, 70)[3], 255, "主十字中心核心双叠饱和");
    // 影子十字 (colorShadeShape, 轴 y=72/x=72, 偏移 +1): 在主线臂端外侧露出 —
    // 影横臂延至 x=74 (主横臂 x≤73), 影竖臂延至 y=74 (主竖臂 y≤73) → 单覆盖点
    assert_eq!(px(&cv, 74, 71), colors().shade_shape, "影横臂右尖端 (74,71)");
    assert_eq!(px(&cv, 71, 74), colors().shade_shape, "影竖臂下尖端 (71,74)");
    // 影子自身交点 (72,72) 双叠: 42+213·42/255 ≈ 77 (Java 同)
    assert_eq!(px(&cv, 72, 72), [0, 0, 0, 77], "影子交点双叠");

    // 底部方向舵横条 (y=height=144 起, 高 12): 外框阴影 + 内填 colorNum。
    // 条顶左角 (0,144) 与 locater 左边框线端点 (drawLine(0,0,0,r), r=144
    // 含端点) 重叠 → SrcOver 双叠 77 (Java 同序同叠); 条底右角单覆盖
    assert_eq!(px(&cv, 0, 144), [0, 0, 0, 77], "条顶左角 (与边框线端点双叠)");
    assert_eq!(px(&cv, 143, 155), colors().shade_shape, "条底边框右角 (144+12-1)");
    assert_eq!(px(&cv, 2, 150), premul(colors().num), "条内填充 (初值 108 宽)");
    assert_eq!(px(&cv, 105, 150), premul(colors().num), "条内填充右段 (x ≤ 106)");
    assert_eq!(px(&cv, 109, 150), [0, 0, 0, 0], "游标右缘外空 (x=109)");

    // 游标竖线 (x=106..108, y=144..167): 阴影框 + colorLabel 中心 1px。
    // 顶行与条顶边框重叠 → 双叠 77; 中心列 (x=107) 从 y=145 起, 底段无条遮挡
    assert_eq!(px(&cv, 106, 144), [0, 0, 0, 77], "游标左上角 (与条顶边框双叠)");
    assert_eq!(px(&cv, 106, 160), colors().shade_shape, "游标左框单覆盖 (条外段)");
    assert_eq!(px(&cv, 107, 160), premul(colors().label), "游标中心 colorLabel (条外段)");
    assert_eq!(px(&cv, 107, 166), premul(colors().label), "游标下端 (144+24-2)");
}

/// draw 文本带: 4 行 BOS 标签 (数字 x=width 基线 24; 标签/单位 x=width+54)
/// 与方向舵数字 (x=rudderValPix, 基线 168) 有字形像素落点
#[test]
fn control_surfaces_draw_text_zones() {
    let mut ov = ControlSurfacesOverlay::new();
    ov.init(0, 1.0, false, 0, 0, true);
    let f_num = font(BOLD, 24);
    let f_label = font(BOLD, 12);
    let f_unit = font(REGULAR, 12);
    let fonts = CsFonts { num: &f_num, label: &f_label, unit: &f_unit };
    let mut cv = PixCanvas::new(240, 180).unwrap();
    ov.draw(&mut cv, &fonts, false);

    let has_ink = |x0: i32, x1: i32, y0: i32, y1: i32| -> bool {
        (x0..x1).any(|x| (y0..y1).any(|y| px(&cv, x, y)[3] > 0))
    };
    // 数字 "50" @ (144, 24 基线), fontNum 24 — lwidth=(9*24)>>2=54
    assert!(has_ink(144, 180, 4, 26), "首行数字带 (升降舵 50)");
    // 标签名 "升降舵" @ (198, 12 基线) + 单位 "%" @ (198, 24 基线)
    assert!(has_ink(198, 240, 2, 14), "首行标签名带");
    assert!(has_ink(198, 216, 14, 26), "首行单位带");
    // 第四行 (可变翼) dy = 12 + 3*36 = 120 基线
    assert!(has_ink(198, 240, 110, 132), "第四行标签带 (dy=120)");
    // 方向舵数字 "50" @ (108, 168 基线) fontLabel 12
    assert!(has_ink(108, 132, 156, 170), "条值数字带");
}

// ---- FmUnpackedDataOverlay ----

/// 测试用 ConfigProvider stub (HashMap + RefCell, 与 vm-core config_provider 测试同式)
struct MapConfig {
    values: RefCell<HashMap<String, String>>,
}

impl MapConfig {
    fn new() -> Self {
        MapConfig { values: RefCell::new(HashMap::new()) }
    }
    fn set(&self, k: &str, v: &str) {
        self.values.borrow_mut().insert(k.to_string(), v.to_string());
    }
}

impl ConfigProvider for MapConfig {
    fn get_config(&self, key: &str) -> Option<String> {
        self.values.borrow().get(key).cloned()
    }
    fn set_config(&self, key: &str, value: &str) {
        self.values.borrow_mut().insert(key.to_string(), value.to_string());
    }
    fn is_field_disabled(&self, _key: &str) -> bool {
        false
    }
}

/// 全字段齐备的测试 blkx (期望值 = Java 8 oracle 手算, HALF_UP 判别值混入)
fn full_fmdata() -> FmData {
    let mut b = FmData::default();
    b.read_file_name = Some("spitfire_mk24".to_string());
    b.version = Some("2.35.0.9".to_string());
    b.emptyweight = 3050.0;
    b.maxfuelweight = 780.45; // %.1f HALF_UP → "780.5"
    b.critical_speed = 230.0; // ×3.6 = 828.000...01 → "828"
    b.vne = 1050.0;
    b.raw_wing_crit_overload = Some([-196000.0, 441000.0]);
    b.grossweight = 5000.0; // full: 1.2·(2·raw/(g·w)∓1) → (-8.4, 20.4)
    b.halfweight = 4000.0; // half → (-10.8, 25.8)
    b.flaps_destruction_num = 2;
    let mut flaps = [[0.0; 2]; 6];
    flaps[0] = [0.0, 640.0];
    flaps[1] = [0.95, 520.0]; // ×100 = 94.99... → %.0f → "95"
    b.flaps_destruction_ind_speed = Some(flaps);
    b.elav_eff = 580.0;
    b.aileron_eff = 640.0;
    b.rudder_eff = 700.0;
    b.elav_power_loss = 0.25; // %.1f HALF_UP → "0.3"
    b.aileron_power_loss = 0.35; // → "0.4"
    b.rudder_power_loss = 0.45; // → "0.5"
    b.nitro = 120.0;
    b.nitro_decr = 2.0; // 120/(2·60) = 1.0
    b.avg_eng_recovery_rate = 3.25; // %.1f HALF_UP → "3.3"
    b.no_flap_wll = 9.0; // (9+1)/2 = 5.0
    b.full_flap_wll = 13.0; // 7.0
    b.moment_of_inertia = Some([12000.0, 25000.0, 8000.0]); // [P:m[2], R:m[0], Y:m[1]]
    b.a_wing = 25.8;
    b.a_fuselage = 5.4;
    b.oswalds_efficiency_number = 0.75;
    b.aspect_ratio = 6.0;
    b.swept_wing_angle = 0.0;
    b.cd_s = 0.42;
    b.ind_cd_f = 0.003; // 4000·0.003 ≈ 12.000...002 → "12"
    b.radiator_cd = 0.021;
    b.oil_radiator_cd = 0.017;
    let mut wing = FmParts::default();
    wing.name = Some("机翼 无襟翼".to_string());
    wing.cd_min = 0.0285; // %.3f HALF_UP → "0.029"
    wing.cl0 = 0.05;
    wing.aoa_crit_low = -14.4;
    wing.aoa_crit_high = 18.6;
    wing.cl_crit_low = -1.15;
    wing.cl_crit_high = 1.55;
    b.no_flaps_wing = Some(wing.clone());
    let mut ff = FmParts::default();
    ff.name = Some("机翼 全襟翼".to_string());
    ff.cd_min = 0.0331;
    ff.cl0 = 0.12;
    ff.aoa_crit_low = -13.1;
    ff.aoa_crit_high = 20.2;
    ff.cl_crit_low = -1.35;
    ff.cl_crit_high = 1.85;
    b.full_flaps_wing = Some(ff);
    let mut fuse = FmParts::default();
    fuse.name = Some("机身".to_string());
    fuse.cd_min = 0.0151;
    fuse.cl0 = 0.02;
    fuse.aoa_crit_low = -27.9;
    fuse.aoa_crit_high = 27.9;
    fuse.cl_crit_low = -0.41;
    fuse.cl_crit_high = 0.49;
    b.fuselage = Some(fuse);
    let mut fin = FmParts::default();
    fin.name = Some("垂尾".to_string());
    fin.cd_min = 0.0081;
    fin.cl0 = 0.0;
    fin.aoa_crit_low = -16.2;
    fin.aoa_crit_high = 16.2;
    fin.cl_crit_low = -0.62;
    fin.cl_crit_high = 0.62;
    b.fin = Some(fin);
    let mut stab = FmParts::default();
    stab.name = Some("平尾".to_string());
    stab.cd_min = 0.0062;
    stab.cl0 = -0.06;
    stab.aoa_crit_low = -15.5;
    stab.aoa_crit_high = 15.5;
    stab.cl_crit_low = -0.55;
    stab.cl_crit_high = 0.55;
    b.stab = Some(stab);
    b
}

/// generateLines 全量 (config None → 全启用) 的逐行 oracle
#[test]
fn generate_lines_full_field_list() {
    let lines = generate_lines(Some(&full_fmdata()), None);
    let expect_prefix = [
        "FM文件: spitfire_mk24 - 2.35.0.9",
        "空重(kg): 3050.0",
        "最大燃油重量(kg): 780.5", // %.1f HALF_UP 判别
        "临界速度(km/h): [828, 1050]",
        "允许过载(满/半油): [-8.4, 20.4], [-10.8, 25.8]",
        "襟翼限速(km/h)0: 0% / 640",
        "襟翼限速(km/h)1: 95% / 520",
        "三舵有效速度(km/h): [ 升降580, 副翼640, 方向700 ]",
        "三舵锁舵因数: [ 升降0.3, 副翼0.4, 方向0.5 ]", // %.1f HALF_UP ×3
        "加力(kg)/时限(分钟): 120.0 / 1.0",
        "平均耐热条恢复速率: 3.3", // %.1f HALF_UP 判别
        "千米最大升力过载: 5.0 / 7.0(襟) @ 350IAS",
        "三轴转动惯量: [ P: 8000, R: 12000, Y: 25000 ]",
        "主升力面积: 25.8机翼, 5.4机身",
        "主升力面积因数载荷: 9.00 / 13.00(襟)",
        "翼展效率: 0.75 展弦比: 6.0 后掠角: 0.0",
        "主阻力面积因数及加速度系数: 0.42 / 0.105",
        "诱导阻力因数及加速度系数: 0.003 / 12",
        "散热/油冷器阻力系数: 0.021 / 0.017",
    ];
    assert!(lines.len() >= expect_prefix.len() + 25, "全字段行数 ≥ 44, 实 {}", lines.len());
    for (i, want) in expect_prefix.iter().enumerate() {
        assert_eq!(&lines[i], want, "第 {i} 行");
    }
    // FM 器件段 (addFmParts ×5 段, 每段表头+4 行)
    assert_eq!(lines[19], "------fm器件 机翼 无襟翼------");
    assert_eq!(lines[20], "零升阻力系数: 0.029", "%.3f HALF_UP 判别");
    assert_eq!(lines[21], "零攻角升力: 0.050");
    assert_eq!(lines[22], "临界攻角: [-14.4, 18.6]");
    assert_eq!(lines[23], "临界攻角升力系数: [-1.15, 1.55]");
    let idx = lines
        .iter()
        .position(|l| l == "------fm器件 平尾------")
        .expect("第五段 (Stab)");
    assert_eq!(&lines[idx + 1..idx + 5], [
        "零升阻力系数: 0.006",
        "零攻角升力: -0.060",
        "临界攻角: [-15.5, 15.5]",
        "临界攻角升力系数: [-0.55, 0.55]",
    ]);
}

/// 无数据 / null 字段 ("null" 文本) / 空白模板行裁剪
#[test]
fn generate_lines_no_data_and_null_fields() {
    assert_eq!(
        generate_lines(None, None),
        vec!["FM Data Preview".to_string(), "[No Data Loaded]".to_string()]
    );
    // readFileName/version 为 null → %s 打 "null" (Java Formatter 行为)
    let mut b = FmData::default();
    b.emptyweight = 1.0;
    let lines = generate_lines(Some(&b), None);
    assert_eq!(lines[0], "FM文件: null - null");
}

/// 字段开关: false 关 / 空串与缺失默认开 / parseBoolean 仅 "true" (忽略大小写)
#[test]
fn generate_lines_field_switches() {
    let cfg = MapConfig::new();
    cfg.set("showWeight", "false");
    cfg.set("showCritSpeed", "FALSE"); // parseBoolean 忽略大小写 → false
    cfg.set("showLift", ""); // 空串 → 默认启用
    cfg.set("showDrag", "yes"); // 非 "true" → false
    let lines = generate_lines(Some(&full_fmdata()), Some(&cfg));
    assert!(!lines.iter().any(|l| l.starts_with("空重")), "showWeight=false 关");
    assert!(!lines.iter().any(|l| l.starts_with("临界速度")), "FALSE (忽略大小写) 关");
    assert!(lines.iter().any(|l| l.starts_with("主升力面积")), "空串默认开");
    assert!(!lines.iter().any(|l| l.starts_with("主阻力面积")), "yes → false");
    assert!(lines.iter().any(|l| l.starts_with("加力")), "其余段不受影响");
    // fmVersion 恒显 → "[No Fields Enabled]" 占位不可达 (Java 同)
    assert!(lines.iter().any(|l| l.starts_with("FM文件")));
}

/// nitro ≤ 0 段隐藏 (Java :212 blkx.nitro > 0 门控)
#[test]
fn generate_lines_nitro_gate() {
    let mut b = full_fmdata();
    b.nitro = 0.0;
    let lines = generate_lines(Some(&b), None);
    assert!(!lines.iter().any(|l| l.contains("加力")));
    b.nitro = 60.0;
    b.nitro_decr = 1.0;
    let lines = generate_lines(Some(&b), None);
    assert!(lines.iter().any(|l| l == "加力(kg)/时限(分钟): 60.0 / 1.0"));
}

/// 表头谓词 (Java :87/:118 startsWith 覆盖默认 contains) + 斑马交互
#[test]
fn fm_overlay_header_matcher() {
    let f = font(REGULAR, 14);
    let mut ov = FmUnpackedDataOverlay::new(1440, 1.0, 12);
    ov.init(None, &f);
    assert!(ov.base.zebra.is_header("FM文件: x"));
    assert!(ov.base.zebra.is_header("------fm器件: 机翼"));
    assert!(!ov.base.zebra.is_header("prefix FM文件"), "startsWith 不含中缀");
    assert!(!ov.base.zebra.is_header("含 fm器件 中缀的行"), "默认 contains 已被覆盖");
}

/// 游戏模式门控: 初始隐藏不取数; toggle 后取数并脏; 同数据不脏 (Java :67/:318)
#[test]
fn fm_overlay_toggle_visibility_gating() {
    let f = font(REGULAR, 14);
    let mut ov = FmUnpackedDataOverlay::new(1440, 1.0, 12);
    ov.init(None, &f);
    assert!(!ov.is_visible_now(), "游戏模式初始隐藏");
    assert!(!ov.tick(), "隐藏分支不取数不显示");
    assert!(!ov.base.window_visible);

    ov.toggle();
    assert!(ov.is_visible_now());
    ov.reload_fm_data(Some(Arc::new(full_fmdata())));
    assert!(ov.tick(), "首帧脏 (lastData=null → 行清单入基座)");
    assert!(ov.base.window_visible);
    assert!(!ov.tick(), "同数据 equals → 不脏");

    ov.toggle();
    assert!(!ov.tick(), "再隐藏 → 不取数");
    assert!(!ov.base.window_visible);
}

/// reload/reinit 换 blkx → 行清单随脏检查刷新; None → 占位 (Java :130-151)
#[test]
fn fm_overlay_reload_and_reinit() {
    let f = font(REGULAR, 14);
    let mut ov = FmUnpackedDataOverlay::new(1440, 1.0, 12);
    ov.init(None, &f);
    ov.toggle(); // 可见化以走取数分支

    // last_data 为基座私有字段, 内容经 generate_lines() 断言、刷新经脏标志断言
    ov.reload_fm_data(Some(Arc::new(full_fmdata())));
    assert!(ov.tick());
    assert!(ov.generate_lines()[0].starts_with("FM文件: spitfire"));

    ov.reload_fm_data(None);
    assert!(ov.tick(), "清单变化 ([No Data Loaded]) → 脏");
    assert_eq!(
        ov.generate_lines(),
        vec!["FM Data Preview".to_string(), "[No Data Loaded]".to_string()]
    );
    assert!(!ov.tick(), "同清单 → 不脏");

    // reinit_config: FMManager.current() 快照注入 (Java :146-147)
    let mut b = FmData::default();
    b.read_file_name = Some("tempest_mk5".to_string());
    ov.reinit_config(Some(Arc::new(b)), &f);
    assert!(ov.tick(), "reinit 换机 → 清单变化 → 脏");
    assert!(ov.generate_lines()[0].starts_with("FM文件: tempest_mk5"));
    // 预览模式绕过可见门控 (BaseOverlay.run:235 isPreview ||)
    let mut pv = FmUnpackedDataOverlay::new(1440, 1.0, 12);
    pv.init_preview(None, &f);
    assert!(pv.is_visible_now());
    assert!(pv.base.is_preview);
    assert!(pv.tick(), "preview 隐藏语义下仍取数");
}

/// QA 批十终检: 五个 overlay (field1 三件 + 本文件两件) 的内容渲染函数经
/// OverlaySpec 装入 OverlayHost 走全链 (register → open_all → render_tick →
/// present → close_all)。field2 两组件的完整组装 (动态窗口高/逐条目可见性/
/// 预览闭包工厂) 按模块头 PORT 注留组装层, 此处只证 host 的 render 闭包通道
/// (RenderFn) 对二者同样可用 — Java 侧五件同经 OverlayManager 注册装载。
/// 窗口生命周期语义 (销毁序/分流/拖拽) 由 host.rs 自有测试覆盖, 此处 mock 只记
/// present 次数并断言缓冲尺寸。
#[test]
fn five_overlays_mount_into_overlay_host() {
    use crate::host::{OverlayHost, OverlaySpec};
    use crate::platform::{OverlayEvent, OverlayWindow, WindowConfig};
    use std::cell::Cell;
    use std::rc::Rc;

    struct MiniWin {
        presents: Rc<Cell<u32>>,
        size: (i32, i32),
    }
    impl OverlayWindow for MiniWin {
        fn present(&mut self, buf: &[u8]) -> Result<(), String> {
            assert_eq!(buf.len(), (self.size.0 * self.size.1 * 4) as usize);
            self.presents.set(self.presents.get() + 1);
            Ok(())
        }
        fn set_position(&mut self, _x: i32, _y: i32) {}
        fn position(&self) -> (i32, i32) {
            (0, 0)
        }
        fn set_click_through(&mut self, _on: bool) {}
        fn poll_event(&mut self) -> Option<OverlayEvent> {
            None
        }
        fn screen_size(&self) -> (i32, i32) {
            (1920, 1080)
        }
    }

    let presents = Rc::new(Cell::new(0u32));
    let p_counter = Rc::clone(&presents);
    let mut host = OverlayHost::with_factory(Box::new(move |cfg: WindowConfig| {
        let size = (cfg.width, cfg.height);
        Ok(Box::new(MiniWin { presents: Rc::clone(&p_counter), size }) as Box<dyn OverlayWindow>)
    }));

    // ①~③ field1 三键 (engineInfoSwitch/enableEngineControl/enablegearAndFlaps):
    // POC 预览工厂已随重构波2 退役, 此处以最小手工 spec 顶位 (host 通道语义
    // 与内容函数无关, 真实内容渲染由 ④⑤ + field1 自有测试覆盖)
    for key in ["engineInfoSwitch", "enableEngineControl", "enablegearAndFlaps"] {
        host.register(OverlaySpec {
            id: key.into(),
            config_key: key.into(),
            width: 40,
            height: 12,
            render: Box::new(|_cv| {}),
            reinit: None,
        });
    }
    // ④ ControlSurfaces (Java 键 enableAxis): draw 内容函数手工包进 render 闭包
    //    (P5 组装契约 (c) 预览工厂留组装层, 此处同形态验证)
    let mut cs = ControlSurfacesOverlay::new();
    cs.init_preview(0, 1.0, false, 0, 0);
    let f_num = font(BOLD, cs.font_size);
    let f_label = font(BOLD, cs.label_font_size);
    let f_unit = font(REGULAR, cs.label_font_size);
    let (cw, ch) = (cs.total_width, cs.total_height);
    host.register(OverlaySpec {
        id: "enableAxis".into(),
        config_key: "enableAxis".into(),
        width: cw,
        height: ch,
        render: Box::new(move |cv| {
            let fonts = CsFonts { num: &f_num, label: &f_label, unit: &f_unit };
            cs.draw(cv, &fonts, aa());
        }),
        reinit: None,
    });
    // ⑤ FMUnpackedData (Java 键 enableFMPrint): render(&mut) 同通道
    let f_list = font(REGULAR, 14);
    let mut fm = FmUnpackedDataOverlay::new(1440, 1.0, 12);
    fm.init_preview(None, &f_list);
    assert!(fm.tick(), "preview 首帧取数 (占位两行清单)");
    let (fw, fh) = (fm.base.width, fm.base.height);
    assert!(fw > 0 && fh > 0);
    host.register(OverlaySpec {
        id: "enableFMPrint".into(),
        config_key: "enableFMPrint".into(),
        width: fw,
        height: fh,
        render: Box::new(move |cv| {
            fm.render(cv, &f_list, aa());
        }),
        reinit: None,
    });

    // 全链: 开 → 首帧五窗各 present 一次 (尺寸逐窗断言) → 静态内容脏检查抑制
    // → close_all 后槽位全空不再渲染
    host.open_all().unwrap();
    assert_eq!(host.active_ids().len(), 5, "五个 overlay 全部装载打开");
    host.render_tick().unwrap();
    assert_eq!(presents.get(), 5, "首帧五窗各一次 present");
    host.render_tick().unwrap();
    assert_eq!(presents.get(), 5, "静态预览内容: 脏检查抑制");
    host.close_all();
    host.render_tick().unwrap();
    assert_eq!(presents.get(), 5, "槽位全空: 不再 present");
    assert!(host.active_ids().is_empty());
}

/// live 工厂: 尺寸 = 内容区 (fontAdd 0/dpi 1 → fs=24, w=144, twidth=240,
/// theight=180), has_service 初值 false (init_preview), 喂入侧置 true 后
/// on_flight_data 才推数据; render 闭包共享句柄画到新值
#[test]
fn control_surfaces_overlay_spec_shared_state() {
    let fonts_dir = std::path::Path::new("../../../fonts");
    let cell = Rc::new(RefCell::new(ReinitParams::default()));
    let (h, mut spec) = control_surfaces_overlay_spec(fonts_dir, &cell).unwrap();
    assert_eq!((spec.width, spec.height), (240, 180), "内容区尺寸 (无 sw 边框)");
    assert_eq!((spec.id.as_str(), spec.config_key.as_str()), ("enableAxis", "enableAxis"));
    // 初值 px = width/2 = 72 (游标居中, Java init :108)
    assert_eq!(h.borrow().px, 72);
    // has_service=false: 数据不更新 (preview 形态)
    assert!(h.borrow_mut().on_flight_data(100, 100.0, 0.0, 0.0, 0.0, false));
    assert_eq!(h.borrow().px, 72, "preview 门控: 数据保持");
    // 游戏形态 (喂入方切换 has_service, app_shell 承载): aileron=100 → px=144
    h.borrow_mut().has_service = true;
    assert!(h.borrow_mut().on_flight_data(200, 100.0, 0.0, 0.0, 0.0, false));
    assert_eq!(h.borrow().px, 144);
    assert_eq!(h.borrow().aileron_num, "100");
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0));

    // WYSIWYG reinit: fontAdd 0→6 → fs=30 → w=180, twidth=300, theight=225
    cell.borrow_mut().font_add_axis = 6;
    let (w1, h1) = (spec.reinit.as_mut().unwrap())().expect("reinit 应成功");
    assert_eq!((w1, h1), (300, 225), "字号 6 的内容区 (fs=30)");
    assert_eq!(h.borrow().font_size, 30, "state 已换新几何");
    // reinit 后 render 闭包可画 (共享字体单元已更新, 不 panic)
    let mut cv2 = PixCanvas::new(w1, h1).unwrap();
    (spec.render)(&mut cv2);
    assert!(cv2.pixmap().data().iter().any(|&b| b != 0));
}

/// CloseAllOverlays 数据面重置 (app_shell reset_handles_preview_values 调用面):
/// live 残留 (num 串 + 游标/舵条) → reset_preview → initPreview 的
/// "Initial Values (50)" + 游标居中。场景: 托盘 live→preview 后重开的
/// 预览窗不得显示上次 live 舵面值
#[test]
fn control_surfaces_reset_preview_restores_initial_values() {
    let fonts_dir = std::path::Path::new("../../../fonts");
    let cell = Rc::new(RefCell::new(ReinitParams::default()));
    let (h, _spec) = control_surfaces_overlay_spec(fonts_dir, &cell).unwrap();
    // live 残留: has_service=true 喂非 50 值 (副翼 100/升降 -80/舵 60/翼扫 40)
    h.borrow_mut().has_service = true;
    assert!(h.borrow_mut().on_flight_data(200, 100.0, -80.0, 60.0, 40.0, true));
    assert_eq!(h.borrow().aileron_num, "100");
    // 重置 → 初值段: 四 num 串 "50" + 游标/舵条回几何中心 (init :91-94/:108-111)
    h.borrow_mut().reset_preview();
    let cs = h.borrow();
    assert_eq!(
        (
            cs.elevator_num.as_str(),
            cs.aileron_num.as_str(),
            cs.rudder_num.as_str(),
            cs.wing_sweep_num.as_str()
        ),
        ("50", "50", "50", "50")
    );
    assert_eq!(
        (cs.px, cs.py, cs.rudder_val_pix),
        (cs.width / 2, cs.height / 2, (50 + 100) * cs.width / 200),
        "游标居中 + 舵条半量程"
    );
}

// ---- FmUnpackedData spec 工厂 + FmUnpackedFeed (P5 组装契约 (a)(b)(c) 销号面) ----

/// 最小 mock 窗口: 只记 set_visible/set_size 调用序 (host/tests.rs MockWindow 同款形态)
struct FeedMockWin {
    log: Rc<RefCell<Vec<String>>>,
}

impl crate::platform::OverlayWindow for FeedMockWin {
    fn present(&mut self, _buf: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn set_position(&mut self, _x: i32, _y: i32) {}
    fn position(&self) -> (i32, i32) {
        (60, 100)
    }
    fn set_click_through(&mut self, _on: bool) {}
    fn set_topmost(&mut self, _on: bool) {}
    fn set_visible(&mut self, visible: bool) {
        self.log.borrow_mut().push(format!("set_visible:{visible}"));
    }
    fn set_size(&mut self, w: i32, h: i32) {
        self.log.borrow_mut().push(format!("set_size:{w},{h}"));
    }
    fn poll_event(&mut self) -> Option<crate::platform::OverlayEvent> {
        None
    }
    fn screen_size(&self) -> (i32, i32) {
        (1920, 1080)
    }
}

fn feed_host(log: &Rc<RefCell<Vec<String>>>) -> OverlayHost {
    let log = Rc::clone(log);
    OverlayHost::with_factory(Box::new(move |_cfg| {
        Ok(Box::new(FeedMockWin { log: Rc::clone(&log) })
            as Box<dyn crate::platform::OverlayWindow>)
    }))
}

fn feed_fm() -> Arc<FMManager> {
    Arc::new(FMManager::new(Arc::new(vm_core::bus::EventBus::new())))
}

/// 工厂初态 = initPreview 形态 (恒可见 + 空数据 — 注册期 = Java 无实例形态;
/// 数据装载见 [`fm_unpacked_preview_session_pumps_data`] — Java 预览实例的
/// run 线程同样在跑, 审查 B2-2); spec 尺寸 = init 几何
/// (logicalHeight 1080/dpi 1 → scaleFactor 0.75 → 324×864, BaseOverlay.java:94-95)
#[test]
fn fm_unpacked_spec_preview_shape_and_render() {
    let (h, mut spec) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        None,
        &feed_fm(),
    )
    .unwrap();
    assert_eq!(
        (spec.id.as_str(), spec.config_key.as_str()),
        ("enableFMPrint", "enableFMPrint")
    );
    assert_eq!((spec.width, spec.height), (324, 864), "init 几何 (round(12·36·0.75) × 12·72)");
    {
        let fm = h.borrow();
        assert!(fm.visible, "preview: always visible (:113)");
        assert!(fm.base.is_preview, "preview: isPreview=true (:110)");
        assert_eq!(fm.base.width, 324);
    }
    // 空数据渲染: dataPanel 底色铺满 (非零像素), 无文本行
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0), "panel 底色");
}

/// 预览会话数据装载 (审查 B2-2 回归锚): Java needsThread=true — 预览实例同样
/// 起 run() 线程 (OverlayManager.refreshPreview :326-331), isPreview 分支每
/// 200ms generateLines → 预览窗显示 FM 字段行 (非空面板)。Rust 对位 = 泵不做
/// 会话门控: preview 形态 tick 取数 → dirty → adjustPosition 高度自适应。
#[test]
fn fm_unpacked_preview_session_pumps_data() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut host = feed_host(&log);
    let (h, spec) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        None,
        &feed_fm(),
    )
    .unwrap();
    host.register(spec);
    // 预览物化 (Java refreshPreview: 工厂 initPreview + 起线程)
    host.refresh_preview().unwrap();
    // 预览期的 FM 装载面 (Java previewInitializer 的 setBlkx(current) /
    // reinitConfig 直读 — 事件订阅仅游戏 init, reload 不走)
    h.borrow_mut().reinit_config(Some(Arc::new(full_fmdata())), &font(REGULAR, 14));
    let mut feed = FmUnpackedFeed::new();
    log.borrow_mut().clear();
    // 泵 (无会话门控): preview 取数 → 高度自适应 resize + 拉起 (幂等可见)
    feed.pump(&mut host, "enableFMPrint", &h, 1_000);
    let row_h = crate::overlay_list::ZebraList::row_height(&font(REGULAR, 14));
    let lines = h.borrow().generate_lines().len() as i32;
    assert!(lines >= 44, "预览装载 FM 行清单 (实测 {lines})");
    assert_eq!(
        h.borrow().base.height,
        lines * row_h,
        "preview 首轮高度自适应 (非 864 初始空面板)"
    );
    assert!(h.borrow().base.window_visible, "preview isPreview 绕过可见门控");
    // 数据稳定零冗余
    feed.pump(&mut host, "enableFMPrint", &h, 1_300);
    assert_eq!(log.borrow().len(), 1, "稳定期仅首帧 resize 一次");
}

/// 游戏会话全链 (Java run() 循环 + FM_OVERLAY_TOGGLE/FM_CHANGED 的组装面驱动):
/// 隐藏起步 → FM_CHANGED 重载 + 热键切换 → tick 取数 → 高度自适应落 resize +
/// 可见拉起 → 数据稳定零冗余调用 (脏检查/幂等守卫) → 再切换隐藏
#[test]
fn fm_unpacked_feed_game_flow() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut host = feed_host(&log);
    let (h, spec) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        None,
        &feed_fm(),
    )
    .unwrap();
    host.register(spec);
    host.open_all().unwrap();
    // 游戏形态 (win32 OpenAllOverlays 处理点同款): isPreview=false + 隐藏起步
    {
        let mut fm = h.borrow_mut();
        fm.base.is_preview = false;
        fm.visible = false;
    }
    host.set_entry_visible("enableFMPrint", false);
    let mut feed = FmUnpackedFeed::new();
    log.borrow_mut().clear();
    // ① 隐藏态 tick (else 分支): 不取数, 窗口保持隐藏, 高度不动
    feed.pump(&mut host, "enableFMPrint", &h, 1_000);
    assert_eq!(h.borrow().base.height, 864, "隐藏分支不取数, 高度保持 init 值");
    assert!(log.borrow().is_empty(), "无窗口动作 (幂等守卫)");
    // ② FM_CHANGED reload + 热键切换可见
    h.borrow_mut().reload_fm_data(Some(Arc::new(full_fmdata())));
    h.borrow_mut().toggle();
    // ③ 可见分支首 tick: 取数 → dirty → adjustPosition → resize + 拉起窗口
    feed.pump(&mut host, "enableFMPrint", &h, 1_300);
    let row_h = crate::overlay_list::ZebraList::row_height(&font(REGULAR, 14));
    let lines = h.borrow().generate_lines().len() as i32;
    assert!(lines >= 44, "全字段行数 (实测 {lines})");
    assert_eq!(
        h.borrow().base.height,
        lines * row_h,
        "高度 = 行数×行高 (adjustPosition, 未触 1040 钳制)"
    );
    assert_eq!(
        *log.borrow(),
        vec!["set_visible:true".to_string(), format!("set_size:324,{}", lines * row_h)],
        "拉起 + resize 各恰一次"
    );
    // ④ 数据稳定: 脏检查 + 幂等 → 零窗口动作
    feed.pump(&mut host, "enableFMPrint", &h, 1_600);
    assert_eq!(log.borrow().len(), 2, "稳定期零冗余调用 (Issue #54 防抖)");
    // ⑤ 再切换: 隐藏 (else 分支 setVisible(false), 幂等记录拦重复)
    h.borrow_mut().toggle();
    feed.pump(&mut host, "enableFMPrint", &h, 1_900);
    assert_eq!(log.borrow().last().unwrap(), "set_visible:false");
}

/// show* 开关实效 (engine_disables 实效测试先例): config 全关 → 仅 FM 版本行
/// (最小面) vs 全开 (None = 默认启用) → 显著更高
// PORT(allow): MapConfig 含 RefCell (!Sync) — 工厂签名的 Arc<dyn ConfigProvider>
// 无 Send 约束 (Rc 句柄恒留本线程), 与 Java 引用共享同构
#[test]
#[allow(clippy::arc_with_non_send_sync)]
fn fm_unpacked_field_switches_change_height() {
    let fm = feed_fm();
    let row_h = crate::overlay_list::ZebraList::row_height(&font(REGULAR, 14));
    // 全关 (16 键 "false" → 仅 fmVersion 恒显行)
    let cfg_off = MapConfig::new();
    for key in [
        "showWeight",
        "showCritSpeed",
        "showGLoadLimits",
        "showFlapLimits",
        "showControlEffectiveness",
        "showNitro",
        "showHeatRecovery",
        "showMaxLiftLoad",
        "showInertia",
        "showLift",
        "showDrag",
        "showNoFlapsWing",
        "showFullFlapsWing",
        "showFuselage",
        "showFin",
        "showStab",
    ] {
        cfg_off.set(key, "false");
    }
    let (h_off, _) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        Some(Arc::new(cfg_off)),
        &fm,
    )
    .unwrap();
    h_off.borrow_mut().reload_fm_data(Some(Arc::new(full_fmdata())));
    h_off.borrow_mut().tick();
    assert_eq!(h_off.borrow().base.height, row_h, "全关 = 仅 FM 版本一行的高度");
    // 全开 (config None → isFieldEnabled 默认启用)
    let (h_on, _) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        None,
        &fm,
    )
    .unwrap();
    h_on.borrow_mut().reload_fm_data(Some(Arc::new(full_fmdata())));
    h_on.borrow_mut().tick();
    assert!(
        h_on.borrow().base.height > 20 * row_h,
        "全开显著更高 (实测 {} vs 最小 {})",
        h_on.borrow().base.height,
        row_h
    );
}

/// reset_preview (win32 CloseAllOverlays → reset_handles_preview_values 调用面):
/// live 行残留 → 预览重开为空面板 (Java closeAll 销毁实例 + 预览工厂新建)
#[test]
fn fm_unpacked_reset_preview_clears_live_lines() {
    let (h, mut spec) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        None,
        &feed_fm(),
    )
    .unwrap();
    // live 会话残留: 游戏形态 + FM 数据 + 可见
    {
        let mut fm = h.borrow_mut();
        fm.base.is_preview = false;
        fm.visible = true;
        fm.reload_fm_data(Some(Arc::new(full_fmdata())));
        assert!(fm.tick(), "数据到达 (dirty)");
    }
    // 行内容入画: 文本带存在白色墨迹 (斑马行白字)
    let (w0, h0) = (spec.width, spec.height.min(200));
    let has_ink = |c: &PixCanvas| {
        c.pixmap()
            .data()
            .chunks_exact(4)
            .any(|p| p[3] > 200 && p[0] > 200 && p[1] > 200 && p[2] > 200)
    };
    let mut cv = PixCanvas::new(w0, h0).unwrap();
    (spec.render)(&mut cv);
    assert!(has_ink(&cv), "live 行文本墨迹");
    // 重置: 可见/预览态/lastData 清空 → 空面板
    h.borrow_mut().reset_preview();
    {
        let fm = h.borrow();
        assert!(fm.visible && fm.base.is_preview, "preview 形态");
    }
    let mut cv2 = PixCanvas::new(w0, h0).unwrap();
    (spec.render)(&mut cv2);
    assert!(!has_ink(&cv2), "重置后无文本行 (Java 新实例空面板)");
}

/// reinit 闭包 (Java reinitConfig): setBlkx(FMManager.current().blkx) — 未就绪
/// 句柄 blkx=None → 清空 (占位容忍); 返回 None (无 setBounds, 高度待下次数据
/// 变更自纠); 清指纹后 render 通道可用
#[test]
fn fm_unpacked_reinit_clears_fmdata_and_keeps_render() {
    let (h, mut spec) = fm_unpacked_data_overlay_spec(
        std::path::Path::new("../../../fonts"),
        1080,
        &Rc::new(RefCell::new(ReinitParams::default())),
        None,
        &feed_fm(),
    )
    .unwrap();
    h.borrow_mut().reload_fm_data(Some(Arc::new(full_fmdata())));
    assert!(h.borrow().generate_lines().len() >= 44, "重载后有数据");
    assert!(
        (spec.reinit.as_mut().unwrap())().is_none(),
        "reinitConfig 无 setBounds (Java 同 — 返回 None 仅清指纹)"
    );
    assert_eq!(
        h.borrow().generate_lines(),
        vec!["FM Data Preview".to_string(), "[No Data Loaded]".to_string()],
        "setBlkx(current=None) 清空 → 占位清单"
    );
    let mut cv = PixCanvas::new(spec.width, spec.height).unwrap();
    (spec.render)(&mut cv);
    assert!(cv.pixmap().data().iter().any(|&b| b != 0));
}
