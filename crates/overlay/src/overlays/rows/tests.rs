use super::*;

const FONT: &str = "../../fonts/sarasa-mono-sc-bold.ttf";

fn main_font() -> LoadedFont {
    LoadedFont::new(std::path::Path::new(FONT), 24).unwrap()
}

/// MinimalHUDContext.java:152 hudFontSizeSmall = 0.75 × 主字号
fn small_font() -> LoadedFont {
    LoadedFont::new(std::path::Path::new(FONT), 18).unwrap()
}

fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
    let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
    [d[0], d[1], d[2], d[3]]
}

fn a(c: &PixCanvas, x: i32, y: i32) -> u8 {
    px(c, x, y)[3]
}

/// 区域内是否存在 alpha 达阈值的像素 (文本笔画的稳健判据)
fn any_alpha_above(c: &PixCanvas, x0: i32, y0: i32, x1: i32, y1: i32, thr: u8) -> bool {
    for y in y0..y1 {
        for x in x0..x1 {
            if a(c, x, y) > thr {
                return true;
            }
        }
    }
    false
}

/// Java2D SrcOver 直通域合成后的 alpha (双层叠色期望值, tiny-skia ±2 LSB)
fn src_over_a(fg: u8, bg: u8) -> u8 {
    let fa = fg as f32 / 255.0;
    let fda = bg as f32 / 255.0;
    ((fa + fda * (1.0 - fa)) * 255.0 + 0.5) as u8
}

fn assert_a_close(actual: u8, expected: u8, what: &str) {
    assert!(
        (actual as i32 - expected as i32).abs() <= 2,
        "{what}: alpha {actual} 期望 ~{expected}"
    );
}

/// HUDTextRow: 警告/常态双色 + 基线平移不变性 (draw 输出仅依赖 (x,y) 相对几何)。
#[test]
fn text_row_colors_and_translation() {
    let f = main_font();
    let mut row = HUDTextRow::new(2, 30);
    assert_eq!(row.id(), "row.2");

    // 常态 colorNum (a=240)
    assert!(row.update("875", false));
    let mut cv = PixCanvas::new(120, 60).unwrap();
    row.draw(&mut cv, 10, 10, &f, false);
    assert!(any_alpha_above(&cv, 5, 5, 60, 45, 200), "常态笔画存在");
    assert_eq!(a(&cv, 10, 9), 0, "行顶 y-1 之上无笔画 (小字号上探之外)");

    // 警告 colorWarning (a=100)
    assert!(row.update("875", true));
    let mut cvw = PixCanvas::new(120, 60).unwrap();
    row.draw(&mut cvw, 10, 10, &f, false);
    assert!(any_alpha_above(&cvw, 5, 5, 60, 45, 80), "警告笔画存在");
    assert!(
        !any_alpha_above(&cvw, 5, 5, 60, 45, 150),
        "警告色无 240 级像素"
    );

    // 平移不变性: y+10 的输出 = y 输出整体下移 10 行
    let mut cv2 = PixCanvas::new(120, 70).unwrap();
    row.update("875", false);
    row.draw(&mut cv2, 10, 20, &f, false);
    for y in 0..60 {
        for x in 5..60 {
            let p0 = px(&cv, x, y);
            let p1 = px(&cv2, x, y + 10);
            assert_eq!(p0, p1, "平移像素 ({x},{y})");
        }
    }
}

/// HUDTextRow.getPreferredSize (HUDTextRow.java:66-83): 模板优先 / 空文本宽 0。
#[test]
fn text_row_template_width() {
    let f = main_font();
    let mut row = HUDTextRow::new(0, 30);
    // 无模板空文本: Java getStringWidth("")=0 → w=0 (非默认 200)
    assert_eq!(row.preferred_size(&f), (0, 30));
    row.update("1", false);
    assert_eq!(row.preferred_size(&f), (f.measure("1"), 30));
    row.set_template(Some("88888"));
    assert_eq!(row.preferred_size(&f), (f.measure("88888"), 30));
    assert!(f.measure("88888") > f.measure("1"), "等宽字体前提");
    // 空模板视为未设 (Java:69 !templateText.isEmpty() 条件)
    row.set_template(Some(""));
    assert_eq!(row.preferred_size(&f), (f.measure("1"), 30));
    assert!(!row.update("1", false), "同值 update 无变化");
}

/// 等宽 advance 钉子: 出厂页机械化段间距 pos 换算的字体事实依据。
/// 字母/空格 advance = 0.5em ("BRK " 4 字符 → 2.0 行高, 精确);
/// 数字 advance 像素化略宽 ("F100 " ≈ 2.6 行高, ±1px)。换字体即醒目失败。
#[test]
fn monospace_advance_facts() {
    let f = main_font();
    assert_eq!(f.measure("BRK "), f.size * 2, "字母+空格段 = 2.0 行高 (精确)");
    let flaps_seg = f.measure("F100 ") as f64 / f.size as f64;
    assert!(
        (flaps_seg - 2.6).abs() <= 0.05,
        "F100 段 ≈ 2.6 行高 (实得 {flaps_seg})"
    );
}

/// AoaGauge: AoA 条几何 (drawHRect 1px 环 + 内芯) + α 文字右置。
/// rightDraw=60, aoaY=30, lineWidth=2 → 条 (x+30, liney) 宽 30 高 5。
#[test]
fn aoa_gauge_bar_and_text_geometry() {
    let f = main_font();
    let sf = small_font();
    let mut g = AoaGauge::new(30, 60, 2);
    g.update("12", 30, COLOR_YELLOW, COLOR_YELLOW);

    let mut cv = PixCanvas::new(140, 60).unwrap();
    let (x, y) = (10, 5);
    g.draw(&mut cv, x, y, &f, &sf, false);

    let ascent = f.metrics().ascent;
    let liney = y + ascent + 1;
    // 环 (shade): 上边行 liney / 下边行 liney+4, 列 x+30..x+59
    assert_eq!(a(&cv, x + 30, liney), 42, "条环上边 shade");
    assert_eq!(a(&cv, x + 59, liney), 42, "条环上边右端");
    assert_eq!(a(&cv, x + 45, liney + 4), 42, "条环下边 shade");
    // 内芯 (aoaBarColor=不透明黄): 列 x+31..x+58, 行 liney+1..liney+3
    assert_eq!(px(&cv, x + 31, liney + 1), COLOR_YELLOW, "条内芯左上");
    assert_eq!(px(&cv, x + 58, liney + 3), COLOR_YELLOW, "条内芯右下");
    assert_eq!(a(&cv, x + 29, liney + 1), 0, "条左侧无");
    assert_eq!(a(&cv, x + 60, liney + 2), 0, "条右侧无 (α 文字区行不重叠)");
    // α 文字: 基线 liney-1, 左缘 x+60 (数字无降部, 不触条区行)
    assert!(
        any_alpha_above(&cv, x + 60, liney - 20, x + 110, liney, 100),
        "α 文字在 x+rightDraw 右侧"
    );
}

/// AoaGauge 条长计算 (Java:69-72): 截断 + rightDraw 钳制。
#[test]
fn aoa_gauge_ratio_clamp() {
    let mut g = AoaGauge::new(30, 60, 2);
    g.set_style(60, 2, 100);
    g.set_aoa_from_ratio(0.255);
    assert_eq!(g.aoa_y, 25, "(int)(0.255*100) 截断");
    g.set_aoa_from_ratio(2.0);
    assert_eq!(g.aoa_y, 60, "钳到 rightDraw");
    g.set_aoa_from_ratio(-0.2);
    assert_eq!(g.aoa_y, -20, "负值不钳 (Java 仅上限钳制)");
}

/// AoaGauge 负宽分支 (UIBaseElements.java:106-109): aoaY<0 时条翻转到
/// x+rightDraw 右侧 (环自 x+rightDraw 起, 内芯 +1)。
#[test]
fn aoa_gauge_negative_aoa_bar_flips_right() {
    let f = main_font();
    let sf = small_font();
    let mut g = AoaGauge::new(30, 40, 2);
    g.update("", -10, COLOR_YELLOW, COLOR_YELLOW);
    let mut cv = PixCanvas::new(120, 60).unwrap();
    let (x, y) = (10, 5);
    g.draw(&mut cv, x, y, &f, &sf, false);
    let liney = y + f.metrics().ascent + 1;
    // 环: drawRect(x+50-10, liney, 9, 4) → 列 x+40..x+49
    assert_eq!(a(&cv, x + 40, liney), 42, "负宽环左边");
    assert_eq!(a(&cv, x + 49, liney), 42, "负宽环右边");
    // 内芯: fillRect(x+50+1-10, liney+1, 8, 3) → 列 x+41..x+48
    assert_eq!(px(&cv, x + 41, liney + 1), COLOR_YELLOW, "负宽内芯");
    assert_eq!(a(&cv, x + 39, liney + 1), 0, "负宽条左侧无");
}

/// AoaGauge/EnergyReadout.getPreferredSize: rightDraw + 模板宽恒占位
/// (布局稳定, 原 Java:102-112 / 78-88)。
#[test]
fn aoa_energy_preferred_size_uses_templates() {
    let sf = small_font();
    let mut g = AoaGauge::new(30, 60, 2);
    g.set_template(Some("88888"));
    assert_eq!(g.preferred_size(&sf), (60 + sf.measure("88888"), 30));
    // 模板 None 时回退实测文本 (Java:102)
    g.set_template(None);
    g.update("9", 30, COLOR_YELLOW, COLOR_YELLOW);
    assert_eq!(g.preferred_size(&sf), (60 + sf.measure("9"), 30));

    let mut en = EnergyReadout::new(30, 50);
    en.update("9.9");
    en.set_template(Some("88.8"));
    assert_eq!(en.preferred_size(&sf), (50 + sf.measure("88.8"), 30));
    // 能量模板为 None 时回退实测文本 (Java:82)
    en.set_template(None);
    assert_eq!(en.preferred_size(&sf), (50 + sf.measure("9.9"), 30));
}

/// EnergyReadout: 能量小字右置同基线 (Java:62-75); 纯辅件无左侧输出。
#[test]
fn energy_readout_side_text() {
    let f = main_font();
    let sf = small_font();
    let (x, y) = (10, 5);
    let base_y = y + f.metrics().ascent;

    let mut en = EnergyReadout::new(30, 50);
    en.update("12.3");
    let mut cv = PixCanvas::new(140, 60).unwrap();
    en.draw(&mut cv, x, y, &f, &sf, false);
    assert!(
        any_alpha_above(&cv, x + 50, base_y - 20, x + 110, base_y + 4, 200),
        "能量小字在 x+rightDraw 右侧"
    );
    assert!(
        !any_alpha_above(&cv, 0, 0, x + 45, 60, 30),
        "纯辅件: 右置区左侧无输出"
    );
}

/// split_trim3 三段切分 (原 HUDMechanizationRow /75-80; 行族拆解后归
/// MechPart 的 push_templates 解析原语)。
#[test]
fn split_trim3_segments() {
    assert_eq!(
        split_trim3("F100BRKGEA").unwrap(),
        (
            "F100".to_string(),
            "BRK".to_string(),
            "GEA".to_string()
        )
    );
    assert_eq!(
        split_trim3("    BRKGEAR").unwrap(),
        (
            String::new(),
            "BRK".to_string(),
            "GEA".to_string()
        ),
        "4 空格段 trim 后为空 (GEAR 第 10 字符后截断, Java substring 同口径)"
    );
    assert_eq!(
        split_trim3("W 75BRKGEA").unwrap(),
        ("W 75".to_string(), "BRK".to_string(), "GEA".to_string())
    );
    assert!(split_trim3("F100BRK").is_none(), "短串 (<10) 不解析");
    assert!(split_trim3("").is_none());
}

/// MechPart: 模板锁宽占位 (空数据不缩宽) + 空数据不绘制 + 警告色 + 脏检查。
#[test]
fn mech_part_template_and_draw() {
    let f = main_font();
    let (x, y) = (10, 5);
    let base_y = y + f.metrics().ascent;

    // 占位: 模板宽; 襟翼空段回退 F100 (Java:77), airbrake/gear 不回退
    let mut p = MechPart::new(MechKind::Flaps, 30);
    assert_eq!(p.preferred_size(&f), (f.measure("W100"), 30));
    p.set_template("");
    assert_eq!(p.template, "F100", "空襟翼段回退 F100");
    assert_eq!(p.preferred_size(&f), (f.measure("F100"), 30));
    let mut ab = MechPart::new(MechKind::Airbrake, 30);
    ab.set_template("");
    assert_eq!(ab.template, "", "减速板空段不回退");
    assert_eq!(ab.preferred_size(&f).0, 0, "空模板宽 0");

    // 空数据不绘制 (模板仍占位)
    let mut cv = PixCanvas::new(120, 60).unwrap();
    p.draw(&mut cv, x, y, &f, false);
    assert!(!any_alpha_above(&cv, 0, 0, 120, 60, 1), "空数据无输出");

    // 数据绘制 + 警告色三段同源
    assert!(p.update("F100", true));
    let mut cv2 = PixCanvas::new(120, 60).unwrap();
    p.draw(&mut cv2, x, y, &f, false);
    assert!(
        any_alpha_above(
            &cv2,
            x,
            base_y - 25,
            x + f.measure("F100"),
            base_y + 5,
            80
        ),
        "警告色段"
    );
    assert!(
        !any_alpha_above(&cv2, x, 0, x + f.measure("F100"), 60, 150),
        "警告色无 240 级像素"
    );
    assert!(
        !any_alpha_above(&cv2, x + f.measure("F100"), 0, 120, 60, 30),
        "段宽外无"
    );
    // 脏检查
    assert!(!p.update("F100", true), "同值无变化");
    assert!(p.update("F50", true), "仅文本变化");
    assert!(p.update("F50", false), "仅警告态变化");
}

/// ManeuverBar 刻度几何: len10 恒画, 0.1~0.4 阈值逐级点亮 (Java:87-102);
/// 列 = x+rightDraw-len, 行 = baseY+halfLine .. +halfLine+2*lineWidth (1px)。
#[test]
fn maneuver_bar_tick_thresholds() {
    let f = main_font();
    let (x, y) = (10, 5);
    let (right_draw, half_line, line_width) = (60, 2, 2);
    let base_y = y + f.metrics().ascent;
    let ticks = TickScale {
        ticks: [10, 20, 30, 40, 50],
    };

    let mut bar = ManeuverBar::new(30, right_draw, half_line, line_width, 4.0, 2.0);
    bar.update(0.35, 5, ticks);
    let mut cv = PixCanvas::new(100, 60).unwrap();
    bar.draw(&mut cv, x, y, &f, false);

    let tick_top = base_y + half_line;
    let tick_bot = base_y + half_line + 2 * line_width;
    // len10 (恒画), len20 (0.35>=0.1), len30 (>=0.2), len40 (>=0.3) 点亮
    for len in [10, 20, 30, 40] {
        let col = x + right_draw - len;
        assert_eq!(a(&cv, col, tick_top), 240, "刻度 len={len} 顶行");
        assert_eq!(a(&cv, col, tick_bot), 240, "刻度 len={len} 底行");
        assert_eq!(a(&cv, col - 1, tick_top + 2), 0, "刻度 len={len} 左邻");
    }
    // len50 (0.35<0.4) 不点亮
    assert_eq!(a(&cv, x + right_draw - 50, tick_top + 2), 0, "len50 未点亮");
    // 刻度行范围外无 (竖刻度 1px 精确盒)
    assert_eq!(a(&cv, x + right_draw - 10, tick_top - 1), 0, "刻度上方无");
    assert_eq!(a(&cv, x + right_draw - 10, tick_bot + 1), 0, "刻度下方无");

    // 阈值边界: index=0.4 → len50 点亮 (>= 含等)
    let mut bar2 = ManeuverBar::new(30, right_draw, half_line, line_width, 4.0, 2.0);
    bar2.update(0.4, 5, ticks);
    let mut cv2 = PixCanvas::new(100, 60).unwrap();
    bar2.draw(&mut cv2, x, y, &f, false);
    assert_eq!(
        a(&cv2, x + right_draw - 50, tick_top + 2),
        240,
        "0.4 含等点亮"
    );
}

/// ManeuverBar 条线双层描边 (Java:104-114): thick shade 下层 + thin colorNum
/// 上层, y = baseY+halfLine+lineWidth; 行覆盖 = thick 半径外扩。
/// halfLine=2/lineWidth=2 → thin(2) 行 baseY+3..4, thick(4) 行 baseY+2..5。
#[test]
fn maneuver_bar_double_stroke_layers() {
    let f = main_font();
    let (x, y) = (10, 5);
    let (right_draw, half_line, line_width) = (60, 2, 2);
    let base_y = y + f.metrics().ascent;
    let line_y = base_y + half_line + line_width; // newY + lineWidth

    let mut bar = ManeuverBar::new(30, right_draw, half_line, line_width, 4.0, 2.0);
    bar.update(
        0.35,
        30,
        TickScale {
            ticks: [10, 20, 30, 40, 50],
        },
    );
    let mut cv = PixCanvas::new(100, 60).unwrap();
    bar.draw(&mut cv, x, y, &f, false);

    // 条横跨 x+30..x+60 (len=30), 采样列 x+58 (条体内, 非刻度列)
    let col = x + 58;
    // thin(宽2, 圆帽) 行 line_y-1..line_y = baseY+3..4: thin 叠 thick
    assert_a_close(
        a(&cv, col, line_y),
        src_over_a(240, 42),
        "主线行 (thin over thick)",
    );
    assert_a_close(a(&cv, col, line_y - 1), src_over_a(240, 42), "主线行上");
    // thick(宽4) 独占行 baseY+2 / baseY+5 (band 边界为整, 像素中心 .5 无歧义)
    assert_eq!(a(&cv, col, line_y - 2), 42, "影线单独行上 (thick only)");
    assert_eq!(a(&cv, col, line_y + 1), 42, "影线单独行下 (thick only)");
    // thick band 外
    assert_eq!(a(&cv, col, line_y - 3), 0, "条上方 2px");
    assert_eq!(a(&cv, col, line_y + 2), 0, "条下方 2px");
    // 条长: 左端 x+30 内侧 (x+32), 条外 (x+26)
    assert_a_close(a(&cv, x + 32, line_y), src_over_a(240, 42), "条左端内侧");
    assert_eq!(a(&cv, x + 26, line_y), 0, "条长之外");
}

/// ManeuverBar.getPreferredSize: rightDraw+5 (条右端占位)。
#[test]
fn maneuver_bar_preferred_size() {
    let bar = ManeuverBar::new(30, 60, 2, 2, 4.0, 2.0);
    assert_eq!(bar.preferred_size(), (65, 30));
}

/// 脏检查契约回归: update 返回值必须覆盖组件全部可变字段 (Java 原方法
/// 返回 void, bool 为 Rust 附加的组装侧重绘门控元数据)。AoaGauge 的
/// 文本/条长/双色, EnergyReadout 的文本, ManeuverBar 的 index/len/刻度族
/// 均逐帧变化, 漏比任一字段即冻结对应读数/条刻度。
#[test]
fn update_changed_covers_all_fields() {
    // AoaGauge (文本/条长/双色全参与)
    let mut g = AoaGauge::new(30, 60, 2);
    g.set_style(60, 2, 100);
    g.set_aoa_from_ratio(0.1);
    let y = g.aoa_y;
    g.update("12", y, COLOR_YELLOW, COLOR_YELLOW); // 初写 (空→"12")
    assert!(!g.update("12", y, COLOR_YELLOW, COLOR_YELLOW), "全同值无变化");
    assert!(g.update("13", y, COLOR_YELLOW, COLOR_YELLOW), "仅文本变化");
    assert!(g.update("13", y + 1, COLOR_YELLOW, COLOR_YELLOW), "仅条长变化");
    assert!(
        g.update("13", y + 1, [1, 2, 3, 4], COLOR_YELLOW),
        "仅文字色变化"
    );
    assert!(
        g.update("13", y + 1, [1, 2, 3, 4], [5, 6, 7, 8]),
        "仅条色变化"
    );

    // EnergyReadout
    let mut en = EnergyReadout::new(30, 50);
    en.update("E100"); // 初写 (空→"E100")
    assert!(!en.update("E100"), "全同值无变化");
    assert!(en.update("E200"), "仅能量变化须报 changed");
    assert!(!en.update("E200"), "重复同能量无变化");

    // ManeuverBar (刻度尺整体 + 单档距离均须参与比较)
    let t = |ticks: [i32; 5]| TickScale { ticks };
    let mut mn = ManeuverBar::new(30, 60, 2, 2, 4.0, 2.0);
    mn.update(0.1, 5, t([10, 20, 30, 40, 50])); // 初写 (default 刻度尺 → 非零)
    assert!(
        !mn.update(0.1, 5, t([10, 20, 30, 40, 50])),
        "全同值无变化"
    );
    assert!(mn.update(0.2, 5, t([10, 20, 30, 40, 50])), "仅 index 变化");
    assert!(mn.update(0.2, 6, t([10, 20, 30, 40, 50])), "仅 len 变化");
    assert!(mn.update(0.2, 6, t([11, 20, 30, 40, 50])), "仅 len10 变化");
    assert!(mn.update(0.2, 6, t([11, 21, 30, 40, 50])), "仅 len20 变化");
    assert!(mn.update(0.2, 6, t([11, 21, 31, 40, 50])), "仅 len30 变化");
    assert!(mn.update(0.2, 6, t([11, 21, 31, 41, 50])), "仅 len40 变化");
    assert!(mn.update(0.2, 6, t([11, 21, 31, 41, 51])), "仅 len50 变化");
}
