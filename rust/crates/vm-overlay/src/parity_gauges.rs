//! parity_gauges: gauge 像素对拍基线 — Java `OverlayPngExport --gauge` 的 Rust 等效实现
//!
//! D7 验收路线: 三 gauge 组件以与 Java 端 (src/ui/debug/OverlayPngExport.java
//! exportGauge*) **同源快照的常量表**最小实例化, 画到 preferred size + 2×pad 的
//! PixCanvas, 双 PNG 走 compare 热力图对拍。改一处常量必须同步另一处。
//!
//! | gauge | 组件 (批八产物) | 风格参数 (固定对拍基线, 见下注) | 默认数据 |
//! |---|---|---|---|
//! | linear | gauges_bars::LinearGauge (竖向 tick 左) | length=120 thickness=8 font=24 | value=55 |
//! | compass | gauge_compass::CompassGauge | r=25 lw=3 big=24 small=12 | heading=123.4 loc=C4 |
//! | attitude | gauge_attitude::AttitudeIndicatorGauge | cd=30 cr=15 inner=19 lw=2 half=1 font=18 | pitch=12.5 roll=25 slip=-3.4 valid=1 |
//!
//! 风格参数是**固定对拍基线**, 不是 MinimalHUDContext 生产推导值: 生产侧
//! MiniHUDOverlay.applyStyleToComponents 按 hudFontSize 动态换算 (默认
//! crosshairScale=113 → hudFontSize=28, 生产实为 linear(134,7,f14) /
//! compass(22,2,28,21,f21) / attitude(35,18,22,2,1,f21)); 仅 attitude 基线
//! 恰与 hudFontSize=24 推导巧合一致 (cd=round(2·24·0.618)=30 / cr=15 /
//! inner=19)。基线取易读整数并覆盖目标渲染分支, 双端同源即可对拍,
//! 勿把本表当生产默认形态。
//! 画布 = preferred size + 2×pad (pad: linear/compass 20, attitude 40 — 容纳北三角
//! tip (1.35r) 与 attitude 双值文本伸出 preferred 界的部分; 两侧同 pad 同裁剪语义,
//! Swing BufferedImage 边界裁剪 ↔ Pixmap 光栅化界天然对齐)。
//!
//! 数据注入 (--data 文件): 每行 "key=value" (# 注释), 与 Java readPairs 同格式;
//! 未提供的键走默认值 — 默认数据选非基数角/非零侧滑, 覆盖 sin/cos 路径与格式化分支。
//! 数值键走 f64 域 (Java dval = Double.parseDouble(v.trim())), 字符串键原样不
//! trim (Java sval); 数值串解析失败时本端回退默认而 Java 端抛异常中止导出 —
//! 差异仅在 CLI 失败时机, 双端产物不同会被 compare 放大暴露, 不产生假通过。

use crate::font::LoadedFont;
use crate::gauge_attitude::AttitudeIndicatorGauge;
use crate::gauge_compass::CompassGauge;
use crate::gauges_bars::LinearGauge;
use crate::render2d::PixCanvas;

/// gauge 对拍数据 (各键 Option = 未注入走默认)
#[derive(Debug, Clone, Default)]
pub struct GaugeData {
    /// linear: 条值 (display 缺省 = value 的十进制串)
    pub value: Option<i32>,
    pub display: Option<String>,
    /// compass: 航向 (度) / 地图网格
    pub heading: Option<f64>,
    pub loc: Option<String>,
    /// attitude: 俯仰/滚转/侧滑 (度) / 俯仰数据有效
    pub pitch: Option<f64>,
    pub roll: Option<f64>,
    pub slip: Option<f64>,
    pub valid: Option<bool>,
}

impl GaugeData {
    /// "key=value" 单项注入; 未知 key 返回 false (调用方预检报错用)。
    /// 数值键复刻 Java dval (Double.parseDouble(v.trim())); 字符串键复刻 Java
    /// sval (原样不 trim, 带空白进 FontMetrics 宽度)。
    pub fn apply_pair(&mut self, key: &str, value: &str) -> bool {
        match key {
            // Java exportLinearGauge L250: (int) dval(...) — f64 域解析后截断,
            // "value=60.5"→60; as i32 与 Java (int) 同为向零+饱和+NaN→0
            "value" => self.value = value.trim().parse::<f64>().ok().map(|x| x as i32),
            "display" => self.display = Some(value.to_string()),
            "heading" => self.heading = value.trim().parse().ok(),
            "loc" => self.loc = Some(value.to_string()),
            "pitch" => self.pitch = value.trim().parse().ok(),
            "roll" => self.roll = value.trim().parse().ok(),
            "slip" => self.slip = value.trim().parse().ok(),
            // Java exportAttitudeGauge L296: dval(data,"valid",1.0) != 0.0 — f64 域:
            // "0.0"/"0"/"-0.0"→false, "0.5"/NaN→true (非仅 1/0 整数语义)
            "valid" => self.valid = value.trim().parse::<f64>().ok().map(|x| x != 0.0),
            _ => return false,
        }
        true
    }

    /// 对拍数据文件解析 (与 OverlayPngExport.readPairs 同式: 行 trim, # 注释,
    /// 空行与 '=' 打头行跳过; key 与值均不二次 trim; 未知键静默忽略 — Java 端
    /// map.put 后无人查询即走默认, 本端 apply_pair 返回值在此不当作错误)
    pub fn parse_file(path: &str) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("读取 {} 失败: {}", path, e))?;
        let mut d = GaugeData::default();
        for line in content.lines() {
            // Java String.trim 只裁 ≤U+0020; Rust str::trim 是 Unicode 空白集 (更宽),
            // 此处按 Java 语义精确复刻
            let line = line.trim_matches(|c: char| (c as u32) <= 0x20);
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(eq) = line.find('=') {
                if eq > 0 {
                    let k = &line[..eq]; // 行已 trim, key 保持 substring 原样
                    let v = &line[eq + 1..]; // 值不 trim (Java sval 原样, dval 内自 trim)
                    d.apply_pair(k, v);
                }
            }
        }
        Ok(d)
    }
}

/// 画布尺寸 (preferred size + 2×pad) — Java exportGauge* 同式, 对拍尺寸硬断言依据
pub fn gauge_canvas_size(name: &str) -> Result<(i32, i32), String> {
    match name {
        // LinearGauge.getPreferredSize 竖向: (int)(24*2.0)+8 = 56 宽, 120 高 (L61-77)
        "linear" => Ok((24 * 2 + 8 + 2 * 20, 120 + 2 * 20)),
        // CompassGauge.getPreferredSize: 2r × 2r = 50 (L57-60)
        "compass" => Ok((25 * 2 + 2 * 20, 25 * 2 + 2 * 20)),
        // AttitudeIndicatorGauge.getPreferredSize: cd × cd = 30 (L63-66)
        "attitude" => Ok((30 + 2 * 40, 30 + 2 * 40)),
        other => Err(format!("未知 gauge: {} (linear|compass|attitude)", other)),
    }
}

/// 渲染对拍基线画布 (与 Java `--gauge <name>` 输出逐像素对拍)。
/// fonts_dir 需含 sarasa-mono-sc-bold.ttf (Java 端注册的 BOLD 族同源文件)。
pub fn render_gauge(
    name: &str,
    data: &GaugeData,
    fonts_dir: &std::path::Path,
    aa: bool,
) -> Result<PixCanvas, String> {
    let bold = fonts_dir.join("sarasa-mono-sc-bold.ttf");
    match name {
        "linear" => render_linear(data, &bold, aa),
        "compass" => render_compass(data, &bold, aa),
        "attitude" => render_attitude(data, &bold, aa),
        other => Err(format!("未知 gauge: {} (linear|compass|attitude)", other)),
    }
}

/// linear: 竖向油门条典型形态 (Java exportLinearGauge 同参)
fn render_linear(data: &GaugeData, bold: &std::path::Path, aa: bool) -> Result<PixCanvas, String> {
    const LENGTH: i32 = 120;
    const THICKNESS: i32 = 8;
    const PAD: i32 = 20;
    let font = LoadedFont::new(bold, 24)?;
    let mut g = LinearGauge::new("THR", 110, true);
    g.set_style_context(LENGTH, THICKNESS);
    let value = data.value.unwrap_or(55);
    let display = data
        .display
        .clone()
        .unwrap_or_else(|| value.to_string());
    g.update(value, &display);

    // preferred size 公式 (Java L70: textMetric + 5 - 5 + thicknessCache)
    let text_metric = (font.size as f64 * 2.0) as i32; // (int)(size*2.0) 截断
    let mut cv = PixCanvas::new(text_metric + THICKNESS + 2 * PAD, LENGTH + 2 * PAD)?;
    g.draw(&mut cv, PAD, PAD, &font, aa);
    Ok(cv)
}

/// compass: 非基数航向角覆盖指针旋转 + 航向/网格双文本 (Java exportCompassGauge 同参)
fn render_compass(data: &GaugeData, bold: &std::path::Path, aa: bool) -> Result<PixCanvas, String> {
    const R: i32 = 25;
    const LW: i32 = 3;
    const BIG: i32 = 24;
    const SMALL: i32 = 12;
    const PAD: i32 = 20;
    let font_small = LoadedFont::new(bold, SMALL)?;
    let mut g = CompassGauge::new(R);
    g.set_style_context(R, LW, BIG, SMALL);
    let heading = data.heading.unwrap_or(123.4);
    let loc = data.loc.clone().unwrap_or_else(|| "C4".to_string());
    g.update(heading, &loc);

    let mut cv = PixCanvas::new(R * 2 + 2 * PAD, R * 2 + 2 * PAD)?;
    g.draw(&mut cv, PAD, PAD, Some(&font_small), aa);
    Ok(cv)
}

/// attitude: 俯仰+滚转+侧滑+双值文本全路径 (Java exportAttitudeGauge 同参)。
/// style→data 顺序保真 (aosX 换算消费 font size, 生产同序)
fn render_attitude(data: &GaugeData, bold: &std::path::Path, aa: bool) -> Result<PixCanvas, String> {
    const CD: i32 = 30;
    const CR: i32 = 15;
    const INNER: i32 = 19;
    const LW: i32 = 2;
    const HALF: i32 = 1;
    const PAD: i32 = 40;
    let font = LoadedFont::new(bold, 18)?; // hudFontSizeSmall = 24·0.75
    let mut g = AttitudeIndicatorGauge::new();
    g.set_style_context(CD, CR, INNER, LW, HALF, 18);

    let hud = vm_core::hud_data::Builder {
        pitch: data.pitch.unwrap_or(12.5),
        roll: data.roll.unwrap_or(25.0),
        slip: data.slip.unwrap_or(-3.4),
        pitch_valid: data.valid.unwrap_or(true),
        ..Default::default()
    }
    .build();
    g.on_data_update(&hud);

    let mut cv = PixCanvas::new(CD + 2 * PAD, CD + 2 * PAD)?;
    g.draw(&mut cv, PAD, PAD, Some(&font), aa);
    Ok(cv)
}

#[cfg(test)]
mod tests {
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
}
