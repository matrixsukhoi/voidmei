use super::*;

const FONT: &str = "../../../fonts/sarasa-mono-sc-bold.ttf";

/// 预乘 RGBA 像素 (与 gauges_bars/render2d 测试同约定)
fn a(cv: &PixCanvas, x: i32, y: i32) -> u8 {
    cv.pixmap().data()[((y * cv.width() + x) * 4 + 3) as usize]
}

/// 画布尺寸 = 共享常量表 (对拍尺寸是整数运算, Java/Rust 必须逐像素同界)
#[test]
fn canvas_sizes_match_shared_spec() {
    let fonts = std::path::Path::new("../../../fonts");
    for name in ["linear", "compass", "attitude"] {
        let cv = render_gauge(name, &GaugeData::default(), fonts, true).unwrap();
        assert_eq!(
            (cv.width(), cv.height()),
            gauge_canvas_size(name).unwrap(),
            "{} 画布尺寸",
            name
        );
    }
    // Java 实测基线 (OverlayPngExport --gauge 输出尺寸钉死)
    assert_eq!(gauge_canvas_size("linear").unwrap(), (96, 160));
    assert_eq!(gauge_canvas_size("compass").unwrap(), (90, 90));
    assert_eq!(gauge_canvas_size("attitude").unwrap(), (110, 110));
    assert!(gauge_canvas_size("nope").is_err());
}

/// 默认数据渲染锚点: 各组件核心可视元素存在 (非空 + 关键几何位置着色)。
/// linear: pixVal=round(55·120/110)=60 → 填充行 79..138 (y+h-1-pixVal 起),
/// 条列 = PAD+measure("55")+2+1 (tick 左布局)。
#[test]
fn defaults_render_anchor_pixels() {
    let fonts = std::path::Path::new("../../../fonts");
    let f = LoadedFont::new(std::path::Path::new(FONT), 24).unwrap();
    let cv = render_gauge("linear", &GaugeData::default(), fonts, true).unwrap();
    let bar_x = 20 + f.measure("55") + 2;
    assert_eq!(a(&cv, bar_x + 1, 130), 240, "linear 填充体 colorNum");
    assert_eq!(a(&cv, bar_x + 1, 78), 0, "填充上方透明 (填充顶行 79 之上)");
    // compass: 圆环右点 (中心 (45,45), r=25) — num 环叠 shade 外环 ≈242
    let cv = render_gauge("compass", &GaugeData::default(), fonts, true).unwrap();
    assert!(a(&cv, 69, 45) > 230, "compass 圆环 num 层");
    // attitude: 牵引线 (center (55,55) → target (43,49)) 途中像素存在
    let cv = render_gauge("attitude", &GaugeData::default(), fonts, true).unwrap();
    assert!(a(&cv, 50, 53) > 150, "attitude 牵引线带");
}

/// 数据注入: apply_pair 键域 + Java dval/sval 域语义 + 默认值回退
#[test]
fn data_injection_and_unknown_key() {
    let mut d = GaugeData::default();
    assert!(d.apply_pair("heading", " 270.5 "));
    assert_eq!(d.heading, Some(270.5), "数值键 dval 自 trim");
    assert!(d.apply_pair("loc", " B2"));
    assert_eq!(d.loc.as_deref(), Some(" B2"), "字符串键 sval 原样不 trim");
    // value: Java (int) dval — f64 域解析后向零截断, 非 i32 域解析
    assert!(d.apply_pair("value", "60.5"));
    assert_eq!(d.value, Some(60), "(int)(double)60.5 = 60");
    assert!(d.apply_pair("value", "-2.9"));
    assert_eq!(d.value, Some(-2), "负数向零截断");
    // valid: Java dval(...)!=0.0 — f64 域判定
    assert!(d.apply_pair("valid", "0.0"));
    assert_eq!(d.valid, Some(false), "\"0.0\" 双端均 false");
    assert!(d.apply_pair("valid", "-0.0"));
    assert_eq!(d.valid, Some(false), "-0.0 == 0.0");
    assert!(d.apply_pair("valid", "0.5"));
    assert_eq!(d.valid, Some(true), "非零 double → true");
    assert!(!d.apply_pair("bogus", "1"), "未知键拒绝 (apply_pair 层)");
    // 数值串解析失败 → None → 渲染走默认 (Java 端 Double.parseDouble 抛异常
    // 中止导出 — 差异仅在 CLI 失败时机, 不产生同参数下的静默渲染分歧)
    assert!(d.apply_pair("value", "nan!"));
    assert_eq!(d.value, None);
}

/// parse_file 行级语义: 行 trim/# 注释/'=' 打头跳过/键值不二次 trim/未知键静默忽略
#[test]
fn parse_file_matches_java_read_pairs() {
    let path = std::env::temp_dir().join("vm_overlay_parity_gauges_data_test.txt");
    std::fs::write(
        &path,
        "# 注释行\r\n  heading= 90.0  \r\n=C4\r\nloc= C4\r\nbogus=1\r\nvalue=60.5\r\npitch = 5\r\n",
    )
    .unwrap();
    let d = GaugeData::parse_file(path.to_str().unwrap()).unwrap();
    assert_eq!(d.heading, Some(90.0), "行 trim + 数值键 dval 自 trim");
    assert_eq!(d.loc.as_deref(), Some(" C4"), "值 substring 原样 (前导空格保留)");
    assert_eq!(d.value, Some(60), "f64 域注入 60.5 → 60");
    assert_eq!(
        d.pitch, None,
        "键 \"pitch \" 带尾随空格 / 未知键 bogus — 两端同样静默忽略"
    );
    let _ = std::fs::remove_file(&path);
}
