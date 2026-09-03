use super::*;

const FONT: &str = "../../../fonts/sarasa-mono-sc-bold.ttf";

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

/// HUDAkbRow: AoA 条几何 (drawHRect 1px 环 + 内芯) + α 文字右置 + 主文字左置。
/// rightDraw=60, aoaY=30, lineWidth=2 → 条 (x+30, liney) 宽 30 高 5。
#[test]
fn akb_row_bar_and_text_geometry() {
    let f = main_font();
    let sf = small_font();
    let mut row = HUDAkbRow::new(0, 30, 60, 2);
    row.update("500", false, "12", 30, COLOR_YELLOW, COLOR_YELLOW);

    let mut cv = PixCanvas::new(140, 60).unwrap();
    let (x, y) = (10, 5);
    row.draw(&mut cv, x, y, &f, &sf, false);

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
    // 主文字: colorNum, 位于条左侧区域
    assert!(
        any_alpha_above(&cv, x, y, x + 29, y + 28, 200),
        "速度主文字在左侧"
    );
}

/// HUDAkbRow onDataUpdate 条长计算 (Java:69-72): 截断 + rightDraw 钳制。
#[test]
fn akb_row_aoa_ratio_clamp() {
    let mut row = HUDAkbRow::new(0, 30, 60, 2);
    row.set_style(60, 2, 100);
    row.set_aoa_from_ratio(0.255);
    assert_eq!(row.aoa_y, 25, "(int)(0.255*100) 截断");
    row.set_aoa_from_ratio(2.0);
    assert_eq!(row.aoa_y, 60, "钳到 rightDraw");
    row.set_aoa_from_ratio(-0.2);
    assert_eq!(row.aoa_y, -20, "负值不钳 (Java 仅上限钳制)");
}

/// HUDAkbRow 负宽分支 (UIBaseElements.java:106-109): aoaY<0 时条翻转到
/// x+rightDraw 右侧 (环自 x+rightDraw 起, 内芯 +1)。
#[test]
fn akb_row_negative_aoa_bar_flips_right() {
    let f = main_font();
    let sf = small_font();
    let mut row = HUDAkbRow::new(0, 30, 40, 2);
    row.update("500", false, "", -10, COLOR_YELLOW, COLOR_YELLOW);
    let mut cv = PixCanvas::new(120, 60).unwrap();
    let (x, y) = (10, 5);
    row.draw(&mut cv, x, y, &f, &sf, false);
    let liney = y + f.metrics().ascent + 1;
    // 环: drawRect(x+50-10, liney, 9, 4) → 列 x+40..x+49
    assert_eq!(a(&cv, x + 40, liney), 42, "负宽环左边");
    assert_eq!(a(&cv, x + 49, liney), 42, "负宽环右边");
    // 内芯: fillRect(x+50+1-10, liney+1, 8, 3) → 列 x+41..x+48
    assert_eq!(px(&cv, x + 41, liney + 1), COLOR_YELLOW, "负宽内芯");
    assert_eq!(a(&cv, x + 39, liney + 1), 0, "负宽条左侧无");
}

/// HUDAkbRow 组件级开关 (Java:38-40): 双关全闭无输出, 单开互不影响占位。
#[test]
fn akb_row_visibility_gates() {
    let f = main_font();
    let sf = small_font();
    let (x, y) = (10, 5);
    let liney = y + f.metrics().ascent + 1;

    // 仅 AoA: 左侧主文字区无笔画, 条存在
    let mut row = HUDAkbRow::new(0, 30, 60, 2);
    row.update("500", false, "12", 30, COLOR_YELLOW, COLOR_YELLOW);
    row.set_show_speed(false);
    let mut cv = PixCanvas::new(140, 60).unwrap();
    row.draw(&mut cv, x, y, &f, &sf, false);
    assert!(
        !any_alpha_above(&cv, x, y, x + 29, y + 28, 30),
        "主文字隐藏"
    );
    assert_eq!(px(&cv, x + 31, liney + 1), COLOR_YELLOW, "条仍在");

    // 仅速度: 条与 α 文字均无
    let mut row2 = HUDAkbRow::new(0, 30, 60, 2);
    row2.update("500", false, "12", 30, COLOR_YELLOW, COLOR_YELLOW);
    row2.set_show_aoa(false);
    let mut cv2 = PixCanvas::new(140, 60).unwrap();
    row2.draw(&mut cv2, x, y, &f, &sf, false);
    assert!(
        any_alpha_above(&cv2, x, y, x + 29, y + 28, 200),
        "主文字仍在"
    );
    assert!(
        cv2.pixmap().data()[((liney * cv2.width() + x + 45) * 4) as usize + 3] == 0,
        "条位置无"
    );
    assert!(
        !any_alpha_above(&cv2, x + 60, 0, 140, 60, 30),
        "α 文字区无输出"
    );
}

/// HUDAkbRow/HUDEnergyRow.getPreferredSize: 模板 + rightDraw 占位取大,
/// 隐藏开关不缩宽 (布局稳定, Java:102-112 / 78-88)。
#[test]
fn akb_energy_preferred_size_uses_templates() {
    let f = main_font();
    let sf = small_font();
    let mut akb = HUDAkbRow::new(0, 30, 60, 2);
    akb.update("888", false, "9", 30, COLOR_YELLOW, COLOR_YELLOW);
    akb.set_show_aoa(false); // 隐藏仍占位
    akb.set_template(Some("8888"), Some("88888"));
    let (w, h) = akb.preferred_size(&f, &sf);
    assert_eq!(w, (f.measure("8888")).max(60 + sf.measure("88888")));
    assert_eq!(h, 30);

    let mut en = HUDEnergyRow::new(1, 30, 50);
    en.update("8888", false, "9.9");
    en.set_show_energy(false);
    en.set_template(Some("88888"), Some("88.8"));
    let (w, _) = en.preferred_size(&f, &sf);
    assert_eq!(w, (f.measure("88888")).max(50 + sf.measure("88.8")));
    // 能量模板为 None 时回退实测文本 (Java:82)
    en.set_template(Some("88888"), None);
    let (w, _) = en.preferred_size(&f, &sf);
    assert_eq!(w, (f.measure("88888")).max(50 + sf.measure("9.9")));
}

/// HUDEnergyRow: 能量小字右置同基线 (Java:62-75), 双开关独立。
#[test]
fn energy_row_side_text_and_gates() {
    let f = main_font();
    let sf = small_font();
    let (x, y) = (10, 5);
    let base_y = y + f.metrics().ascent;

    let mut row = HUDEnergyRow::new(1, 30, 50);
    row.update("88", false, "12.3");
    let mut cv = PixCanvas::new(140, 60).unwrap();
    row.draw(&mut cv, x, y, &f, &sf, false);
    assert!(
        any_alpha_above(&cv, x, y, x + 40, y + 28, 200),
        "高度主文字"
    );
    assert!(
        any_alpha_above(&cv, x + 50, base_y - 20, x + 110, base_y + 4, 200),
        "能量小字在 x+rightDraw 右侧"
    );

    // 仅高度: 能量区无 (主文字 "88" 墨迹 ≤ x+27, 不入 x+50 起的右区)
    let mut row2 = HUDEnergyRow::new(1, 30, 50);
    row2.update("88", false, "12.3");
    row2.set_show_energy(false);
    let mut cv2 = PixCanvas::new(140, 60).unwrap();
    row2.draw(&mut cv2, x, y, &f, &sf, false);
    assert!(!any_alpha_above(&cv2, x + 45, 0, 140, 60, 30), "能量隐藏");

    // 仅能量: 主文字区无
    let mut row3 = HUDEnergyRow::new(1, 30, 50);
    row3.update("88", false, "12.3");
    row3.set_show_altitude(false);
    let mut cv3 = PixCanvas::new(140, 60).unwrap();
    row3.draw(&mut cv3, x, y, &f, &sf, false);
    assert!(!any_alpha_above(&cv3, x, 0, x + 45, 60, 30), "高度隐藏");
    assert!(any_alpha_above(&cv3, x + 50, 0, 140, 60, 200), "能量仍在");
}

/// HUDMechanizationRow 模板解析与占位宽 (Java:72-81 / 115-131):
/// 默认 W100/BRK/GEA; "    BRKGEAR" → 襟翼空段回退 F100; 占位宽 =
/// w("W100 ")+w("BRK ")+w("GEA") (getStringWidth 逐字符求和, Java 同口径;
/// 非等宽字符格 — 数字与空格 advance 不同, 见 font.rs charsWidth)。
#[test]
fn mech_row_template_parse_and_preferred_size() {
    let f = main_font();
    // Java getStringWidth(tpl + " ") 的拼接串直译 oracle
    let seg = |t: &str| f.measure(&format!("{t} "));

    let row = HUDMechanizationRow::new(2, 30);
    assert_eq!(row.base.id(), "row.2");
    assert_eq!(
        row.preferred_size(&f),
        (seg("W100") + seg("BRK") + f.measure("GEA"), 30)
    );

    let mut row = HUDMechanizationRow::new(2, 30);
    row.set_template(Some("    BRKGEAR")); // enableFlapAngleBar 预览串
    assert_eq!(row.flaps_template, "F100", "空襟翼段回退 F100 (Java:77)");
    assert_eq!(row.airbrake_template, "BRK");
    assert_eq!(row.gear_template, "GEA");
    // 基座模板同步锁宽 (super.setTemplate)
    assert_eq!(row.base.template.as_deref(), Some("    BRKGEAR"));
    assert_eq!(
        row.preferred_size(&f),
        (seg("F100") + seg("BRK") + f.measure("GEA"), 30)
    );

    // 短串 (<10) 不解析, 模板保持; None 不解析
    row.set_template(Some("F100BRK"));
    assert_eq!(row.flaps_template, "F100");
    row.set_template(None);
    assert_eq!(row.flaps_template, "F100");
    // 模板带 F100 前缀的解析 (襟翼条禁用预览串)
    row.set_template(Some("F100BRKGEA"));
    assert_eq!(
        (
            &row.flaps_template,
            &row.airbrake_template,
            &row.gear_template
        ),
        (&"F100".to_string(), &"BRK".to_string(), &"GEA".to_string())
    );
}

/// HUDMechanizationRow.update 合并串解析 (Java:48-61): ≥10 逐段 trim,
/// 短串三段全清; base.text 承载完整合并串。
#[test]
fn mech_row_update_parse() {
    let mut row = HUDMechanizationRow::new(2, 30);
    assert!(row.update("F100BRKGEA", false));
    assert_eq!(
        (&row.flaps_wing_str, &row.airbrake_str, &row.gear_str),
        (&"F100".to_string(), &"BRK".to_string(), &"GEA".to_string())
    );
    assert_eq!(row.base.text, "F100BRKGEA");

    assert!(row.update("    BRKGEAR", true), "内容与警告态均变");
    assert_eq!(row.flaps_wing_str, "", "4 空格段 trim 后为空");
    assert_eq!(
        (&row.airbrake_str, &row.gear_str),
        (&"BRK".to_string(), &"GEA".to_string())
    );
    assert!(row.base.is_warning);

    assert!(!row.update("    BRKGEAR", true), "同值无变化");
    assert!(row.update("    BRKGEAR", false), "仅警告态变化");
    assert!(row.update("W50", false), "仅主文字变化");
    assert_eq!(row.flaps_wing_str, "", "短串三段全清 (Java:56-59)");
    assert_eq!(row.airbrake_str, "");
    assert_eq!(row.gear_str, "");
}

/// HUDMechanizationRow.update_parts / on_data_update (Java:40-45 / 63-70):
/// 前者清主文字, 后者不动 base.text 直写 isWarning。
#[test]
fn mech_row_update_parts_and_on_data() {
    let mut row = HUDMechanizationRow::new(2, 30);
    row.update("F100BRKGEA", false);
    assert!(row.update_parts("F50", "BRK", "GEA", true));
    assert_eq!(row.base.text, "", "主文字清空 (Java:41)");
    assert!(row.base.is_warning);
    assert_eq!(row.flaps_wing_str, "F50");
    assert!(!row.update_parts("F50", "BRK", "GEA", true), "全同值无变化");
    assert!(row.update_parts("F60", "BRK", "GEA", true), "仅襟翼段变化");

    // on_data_update: base.text 保持, is_warning 直写 (Java:66-69)
    let mut b = vm_core::derived::hud_data::Builder::default();
    b.flaps_wing_str = "W 75".into();
    b.airbrake_str = "".into();
    b.gear_str = "GEA".into();
    b.warn_configuration = false;
    let data = b.build();
    assert!(row.on_data_update(&data));
    assert_eq!(
        (&row.flaps_wing_str, &row.airbrake_str, &row.gear_str),
        (&"W 75".to_string(), &String::new(), &"GEA".to_string())
    );
    assert!(!row.base.is_warning);
    assert_eq!(row.base.text, "", "onDataUpdate 不触 update (Java 原样)");
    assert!(!row.on_data_update(&data), "全同值无变化");
}

/// HUDMechanizationRow.draw 三段几何 (Java:83-113): 段起点 = 前段模板宽和
/// (含尾随空格), 隐藏/空数据段仍占位推进; 三开关独立。
#[test]
fn mech_row_draw_segments_and_gates() {
    let f = main_font();
    let (x, y) = (10, 5);
    let base_y = y + f.metrics().ascent;
    // 模板 F100/BRK/GEA 的段宽 (getStringWidth(tpl+" ") 直译; 逐字符求和)
    let seg = |t: &str| f.measure(&format!("{t} "));
    let flaps_seg = seg("F100");
    let brk_seg = seg("BRK");
    let gear_x = x + flaps_seg + brk_seg;
    let right_edge = gear_x + f.measure("GEA");

    // 单段点亮: 起落架 (起点 = 襟翼段宽 + 减速板段宽)
    let mut row = HUDMechanizationRow::new(2, 30);
    row.set_template(Some("F100BRKGEA"));
    row.update_parts("", "", "GEA", false);
    let mut cv = PixCanvas::new(200, 60).unwrap();
    row.draw(&mut cv, x, y, &f, false);
    assert!(
        !any_alpha_above(&cv, x, 0, gear_x, 60, 30),
        "前两段空 → 左侧无笔画"
    );
    assert!(
        any_alpha_above(&cv, gear_x, base_y - 25, right_edge, base_y + 5, 200),
        "起落架段起点 = 前两段占位宽之和"
    );

    // 隐藏段占位推进: 襟翼关而 BRK 仍从 x+襟翼段宽 起
    let mut row2 = HUDMechanizationRow::new(2, 30);
    row2.set_template(Some("F100BRKGEA"));
    row2.update_parts("F100", "BRK", "", false);
    row2.set_show_flaps(false);
    let mut cv2 = PixCanvas::new(200, 60).unwrap();
    row2.draw(&mut cv2, x, y, &f, false);
    assert!(
        !any_alpha_above(&cv2, x, 0, x + flaps_seg, 60, 30),
        "襟翼隐藏 → 占位区无笔画"
    );
    assert!(
        any_alpha_above(
            &cv2,
            x + flaps_seg,
            base_y - 25,
            x + flaps_seg + f.measure("BRK"),
            base_y + 5,
            200
        ),
        "减速板仍从占位推进处起"
    );

    // 全开: 三段首尾相接, 右缘 = 三段宽和; 警告态三段同色
    let mut row3 = HUDMechanizationRow::new(2, 30);
    row3.set_template(Some("F100BRKGEA"));
    row3.update_parts("F100", "BRK", "GEA", true);
    let mut cv3 = PixCanvas::new(200, 60).unwrap();
    row3.draw(&mut cv3, x, y, &f, false);
    assert!(
        any_alpha_above(&cv3, x, base_y - 25, x + flaps_seg, base_y + 5, 80),
        "襟翼段 (警告色)"
    );
    assert!(
        !any_alpha_above(&cv3, x, 0, right_edge, 60, 150),
        "警告色无 240 级像素"
    );
    assert!(
        !any_alpha_above(&cv3, right_edge, 0, 200, 60, 30),
        "右缘外无"
    );

    // 起落架段无尾随空格占位: gear_template 清空 → 段宽 0 (Java:109-112 无推进消费)
    let mut row4 = HUDMechanizationRow::new(2, 30);
    row4.set_template(Some("F100BRKGEA"));
    row4.gear_template.clear();
    row4.update_parts("", "", "GEA", false);
    let mut cv4 = PixCanvas::new(200, 60).unwrap();
    row4.draw(&mut cv4, x, y, &f, false);
    assert_eq!(
        row4.preferred_size(&f),
        (flaps_seg + brk_seg, 30),
        "空起落架模板不占宽"
    );
}

/// 对拍口径锁定: enableFlapAngleBar 预览串 "    BRKGEAR" (模板同源) →
/// 襟翼段空数据不绘制, BRK 从 x+襟翼段宽 / GEA 从前两段宽和起, 行宽三段和。
#[test]
fn mech_row_preview_placeholder_advance() {
    let f = main_font();
    let (x, y) = (10, 5);
    let base_y = y + f.metrics().ascent;
    let seg = |t: &str| f.measure(&format!("{t} "));
    let flaps_seg = seg("F100"); // 模板 "    " → 空段回退 "F100"
    let gear_x = x + flaps_seg + seg("BRK");

    let mut row = HUDMechanizationRow::new(2, 30);
    row.set_template(Some("    BRKGEAR"));
    row.update("    BRKGEAR", false);
    assert_eq!(row.flaps_wing_str, "");
    let mut cv = PixCanvas::new(200, 60).unwrap();
    row.draw(&mut cv, x, y, &f, false);
    assert!(
        !any_alpha_above(&cv, x, 0, x + flaps_seg, 60, 30),
        "襟翼段空占位"
    );
    assert!(
        any_alpha_above(
            &cv,
            x + flaps_seg,
            base_y - 25,
            x + flaps_seg + f.measure("BRK"),
            base_y + 5,
            200
        ),
        "BRK @ 襟翼段宽处"
    );
    assert!(
        any_alpha_above(
            &cv,
            gear_x,
            base_y - 25,
            gear_x + f.measure("GEA"),
            base_y + 5,
            200
        ),
        "GEA @ 前两段宽和处"
    );
    assert_eq!(
        row.preferred_size(&f),
        (flaps_seg + seg("BRK") + f.measure("GEA"), 30)
    );
}

/// HUDManeuverRow 刻度几何: len10 恒画, 0.1~0.4 阈值逐级点亮 (Java:87-102);
/// 列 = x+rightDraw-len, 行 = baseY+halfLine .. +halfLine+2*lineWidth (1px)。
#[test]
fn maneuver_row_tick_thresholds() {
    let f = main_font();
    let (x, y) = (10, 5);
    let (right_draw, half_line, line_width) = (60, 2, 2);
    let base_y = y + f.metrics().ascent;
    let ticks = TickScale {
        ticks: [10, 20, 30, 40, 50],
    };

    let mut row = HUDManeuverRow::new(4, 30, right_draw, half_line, line_width, 4.0, 2.0);
    // showGLoad=false: 排除主文字, 刻度列纯净 (色取主文字色规范语义)
    row.set_show_g_load(false);
    row.update("2.0", false, 0.35, 5, ticks);
    let mut cv = PixCanvas::new(100, 60).unwrap();
    row.draw(&mut cv, x, y, &f, false);

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
    let mut row2 = HUDManeuverRow::new(4, 30, right_draw, half_line, line_width, 4.0, 2.0);
    row2.set_show_g_load(false);
    row2.update("2.0", false, 0.4, 5, ticks);
    let mut cv2 = PixCanvas::new(100, 60).unwrap();
    row2.draw(&mut cv2, x, y, &f, false);
    assert_eq!(
        a(&cv2, x + right_draw - 50, tick_top + 2),
        240,
        "0.4 含等点亮"
    );
}

/// HUDManeuverRow 条线双层描边 (Java:104-114): thick shade 下层 + thin colorNum
/// 上层, y = baseY+halfLine+lineWidth; 行覆盖 = thick 半径外扩。
/// halfLine=2/lineWidth=2 → thin(2) 行 baseY+3..4, thick(4) 行 baseY+2..5。
#[test]
fn maneuver_row_bar_double_stroke_layers() {
    let f = main_font();
    let (x, y) = (10, 5);
    let (right_draw, half_line, line_width) = (60, 2, 2);
    let base_y = y + f.metrics().ascent;
    let line_y = base_y + half_line + line_width; // newY + lineWidth

    let mut row = HUDManeuverRow::new(4, 30, right_draw, half_line, line_width, 4.0, 2.0);
    row.set_show_g_load(false); // 排除文字, 条区纯净
    row.update(
        "2.0",
        false,
        0.35,
        30,
        TickScale {
            ticks: [10, 20, 30, 40, 50],
        },
    );
    let mut cv = PixCanvas::new(100, 60).unwrap();
    row.draw(&mut cv, x, y, &f, false);

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

/// HUDManeuverRow 开关与 preferred_size (Java:123-128):
/// max(主文字宽, rightDraw+5); 机动条关闭仅剩文字。
#[test]
fn maneuver_row_gates_and_preferred_size() {
    let f = main_font();
    let (x, y) = (10, 5);
    let base_y = y + f.metrics().ascent;
    let line_y = base_y + 2 + 2;

    let mut row = HUDManeuverRow::new(4, 30, 60, 2, 2, 4.0, 2.0);
    row.update(
        "2.0",
        false,
        0.35,
        30,
        TickScale {
            ticks: [10, 20, 30, 40, 50],
        },
    );
    let (w, h) = row.preferred_size(&f);
    assert_eq!(w, (f.measure("2.0")).max(60 + 5));
    assert_eq!(h, 30);

    // 机动条关: 条行无输出, 文字仍在
    let mut cv = PixCanvas::new(100, 60).unwrap();
    row.set_show_maneuver_bar(false);
    row.draw(&mut cv, x, y, &f, false);
    assert_eq!(a(&cv, x + 58, line_y), 0, "条关闭无条线");
    assert_eq!(a(&cv, x + 50, base_y + 4), 0, "条关闭无刻度");
    assert!(
        any_alpha_above(&cv, x, y, x + 40, y + 28, 200),
        "G 文字仍在"
    );

    // G 文字关: 仅条 (index=0.25 → len10/20/30 刻度点亮, 列 ≥ x+30 不入左区)
    let mut row2 = HUDManeuverRow::new(4, 30, 60, 2, 2, 4.0, 2.0);
    row2.update(
        "2.0",
        false,
        0.25,
        30,
        TickScale {
            ticks: [10, 20, 30, 40, 50],
        },
    );
    row2.set_show_g_load(false);
    let mut cv2 = PixCanvas::new(100, 60).unwrap();
    row2.draw(&mut cv2, x, y, &f, false);
    assert!(
        !any_alpha_above(&cv2, x, y, x + 25, y + 28, 30),
        "G 文字隐藏"
    );
    assert_a_close(a(&cv2, x + 58, line_y), src_over_a(240, 42), "条仍在");
}

/// 脏检查契约回归: update 返回值必须覆盖组件全部可变字段 (Java 原方法
/// 返回 void, bool 为 Rust 附加的组装侧重绘门控元数据)。HUDEnergyRow 的
/// energy_text 与 HUDManeuverRow 的 index/len 族均逐帧变化而 base 文字
/// 稳定, 漏比任一字段即冻结对应读数/条刻度。
#[test]
fn update_changed_covers_all_fields() {
    // HUDEnergyRow
    let mut en = HUDEnergyRow::new(1, 30, 50);
    en.update("1000", false, "E100");
    assert!(!en.update("1000", false, "E100"), "全同值无变化");
    assert!(en.update("1000", false, "E200"), "仅能量变化须报 changed");
    assert!(!en.update("1000", false, "E200"), "重复同能量无变化");
    assert!(
        en.update("1001", false, "E200"),
        "仅 base 文字变化仍报 changed"
    );
    assert!(en.update("1001", true, "E200"), "仅警告态变化仍报 changed");

    // HUDManeuverRow (刻度尺整体 + 单档距离均须参与比较)
    let t = |ticks: [i32; 5]| TickScale { ticks };
    let mut mn = HUDManeuverRow::new(4, 30, 60, 2, 2, 4.0, 2.0);
    mn.update("2.0", false, 0.1, 5, t([10, 20, 30, 40, 50]));
    assert!(
        !mn.update("2.0", false, 0.1, 5, t([10, 20, 30, 40, 50])),
        "全同值无变化"
    );
    assert!(
        mn.update("2.0", false, 0.2, 5, t([10, 20, 30, 40, 50])),
        "仅 index 变化"
    );
    assert!(
        mn.update("2.0", false, 0.2, 6, t([10, 20, 30, 40, 50])),
        "仅 len 变化"
    );
    assert!(
        mn.update("2.0", false, 0.2, 6, t([11, 20, 30, 40, 50])),
        "仅 len10 变化"
    );
    assert!(
        mn.update("2.0", false, 0.2, 6, t([11, 21, 30, 40, 50])),
        "仅 len20 变化"
    );
    assert!(
        mn.update("2.0", false, 0.2, 6, t([11, 21, 31, 40, 50])),
        "仅 len30 变化"
    );
    assert!(
        mn.update("2.0", false, 0.2, 6, t([11, 21, 31, 41, 50])),
        "仅 len40 变化"
    );
    assert!(
        mn.update("2.0", false, 0.2, 6, t([11, 21, 31, 41, 51])),
        "仅 len50 变化"
    );
    assert!(
        mn.update("2.1", false, 0.2, 6, t([11, 21, 31, 41, 51])),
        "仅文字变化"
    );
    assert!(
        mn.update("2.1", true, 0.2, 6, t([11, 21, 31, 41, 51])),
        "仅警告态变化"
    );
}
