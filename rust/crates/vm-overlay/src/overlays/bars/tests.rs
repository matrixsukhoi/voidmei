use super::*;

const FONT: &str = "../../../fonts/sarasa-mono-sc-bold.ttf";

fn font() -> LoadedFont {
    LoadedFont::new(std::path::Path::new(FONT), 24).unwrap()
}

/// 读预乘 RGBA 像素 (与 render2d 测试同约定; 断言走 alpha 通道 —
/// 同族色 RGB 相同仅 alpha 不同: num=240 label=166 warning=100 shade=42)
fn px(c: &PixCanvas, x: i32, y: i32) -> [u8; 4] {
    let d = &c.pixmap().data()[((y * c.width() + x) * 4) as usize..][..4];
    [d[0], d[1], d[2], d[3]]
}

fn a(c: &PixCanvas, x: i32, y: i32) -> u8 {
    px(c, x, y)[3]
}

/// Java2D SrcOver 直通域合成后的 alpha (两层叠色的期望值)。
/// 半透明层叠在 Java 同样加深 alpha (shade 叠 shade 等), 此处按同式计算;
/// tiny-skia 预乘取整路径存在 ±1-2 LSB 系统差 (render2d 头注), 用容差比较。
fn src_over_a(fg: u8, bg: u8) -> u8 {
    let fa = fg as f32 / 255.0;
    let fda = bg as f32 / 255.0;
    ((fa + fda * (1.0 - fa)) * 255.0 + 0.5) as u8
}

fn assert_a_close(actual: u8, expected: u8, what: &str) {
    assert!(
        (actual as i32 - expected as i32).abs() <= 2,
        "{what}: alpha {actual} 期望 ~{expected} (SrcOver 叠色 ±2 LSB)"
    );
}

/// 竖向 LinearGauge (默认 tick 在左): 条右置、填充自底向上、随值分隔线。
/// pixVal = round(55*120/110) = 60; 填充行 = y+h-1-60 .. y+h-2 (valH 行)。
#[test]
fn linear_gauge_vertical_fill_and_separator() {
    let f = font();
    let mut g = LinearGauge::new("T", 110, true);
    g.set_style_context(120, 8);
    assert!(g.update(55, "55"));

    let mut cv = PixCanvas::new(140, 160).unwrap();
    g.draw(&mut cv, 20, 10, &f, false);

    let text_w = f.measure("55");
    let bar_x = 20 + text_w + 2;
    // 填充: 列 bar_x+1..bar_x+6 (w-2), 行 69..128; 129 为底边框行 (shade)
    assert_eq!(a(&cv, bar_x + 1, 72), 240, "填充体 (分隔线覆盖区之下)");
    assert_eq!(a(&cv, bar_x + 1, 128), 240, "填充底");
    assert_eq!(a(&cv, bar_x + 1, 129), 42, "底边框行 shade");
    assert_eq!(a(&cv, bar_x + 1, 68), 0, "填充上方透明");
    // 边框环: 列 bar_x / bar_x+7
    assert_eq!(a(&cv, bar_x, 10), 42, "左上边框 shade");
    assert_eq!(a(&cv, bar_x + 7, 129), 42, "右下边框 shade");
    // 分隔线 3px 环 + 1px 值色内芯: sepY = y+length-1-pixVal = 69
    let sep_y = 10 + 120 - 1 - 60;
    let total_w = text_w + 2 + 8;
    assert_eq!(a(&cv, 20, sep_y), 42, "分隔线环上边 shade");
    // 环右下角叠在条右边框列上 (shade 叠 shade → SrcOver 加深, Java 同)
    assert_a_close(
        a(&cv, 20 + total_w - 1, sep_y + 2),
        src_over_a(42, 42),
        "分隔线环右下",
    );
    assert_eq!(a(&cv, 21, sep_y + 1), 240, "分隔线内芯值色");
    assert_eq!(a(&cv, 45, sep_y - 1), 0, "分隔线上方 (文本影与条间隙列)");
    assert_eq!(a(&cv, 45, sep_y + 3), 0, "分隔线下方");
}

/// 竖向越界钳制: curValue > maxValue → valH 钳到 h, 填充自条上方 1px 起
/// (Java:202 valH = min(val,h) 后 fillRect 上探 y+h-1-valH, 行为如此)。
#[test]
fn linear_gauge_vertical_clamp_over_range() {
    let f = font();
    let mut g = LinearGauge::new("T", 100, true);
    g.set_style_context(80, 8);
    g.update(250, "250");
    let mut cv = PixCanvas::new(140, 120).unwrap();
    g.draw(&mut cv, 20, 10, &f, false);
    let bar_x = 20 + f.measure("250") + 2;
    // pixVal=200 → valH 钳 80 → 填充行 9..88 (9 = y+h-1-80, 高出条顶 1px)
    assert_eq!(a(&cv, bar_x + 1, 9), 240, "填充顶上探 1px (Java 行为)");
    assert_eq!(a(&cv, bar_x + 1, 11), 240, "条内填充");
    assert_eq!(a(&cv, bar_x + 1, 8), 0, "上探之外透明");
}

/// 横向 LinearGauge: 填充列截断 (valW-2)、flip 分隔线【不渲染】(Java
/// drawRect 负宽 oracle 0 像素)、文本在条下方 — 只剩条 + 条下文本。
#[test]
fn linear_gauge_horizontal_separator_invisible() {
    let f = font();
    let mut g = LinearGauge::new("T", 100, false);
    g.set_style_context(100, 6);
    g.update(50, "50");
    let mut cv = PixCanvas::new(140, 100).unwrap();
    g.draw(&mut cv, 10, 20, &f, false);

    // pixVal=50 → 填充列 11..58 (valW-2=48), 行 21..24 (h-2)
    assert_eq!(a(&cv, 11, 22), 240, "填充左端");
    assert_eq!(a(&cv, 58, 24), 240, "填充右端");
    assert_eq!(a(&cv, 60, 22), 0, "填充右外 (valW-2 截断)");
    // flip 分隔线 (LinearGauge.java:176): drawRect(x+pixVal-2, y, 3, -thickness-fontSize)
    // = drawRect(58, 20, -4, ·) 负宽 → Java 整体不绘制 (oracle 实测 0 像素)。
    // 条顶边框行 y=20 只有 shade 单层, 无 shade 叠 shade 加深, 分隔线区域
    // (列 57..61) 与条内无异。
    assert_eq!(a(&cv, 57, 20), 42, "条顶边框单层 shade (无分隔线叠色)");
    assert_eq!(a(&cv, 61, 20), 42, "条顶边框单层 shade");
    assert_eq!(a(&cv, 59, 20), 42, "条顶边框单层 shade");
    assert_eq!(a(&cv, 59, 21), 0, "分隔线位置无向上延伸 (Java 不可见)");
    assert_eq!(a(&cv, 50, 21), 240, "条顶内填充体");
    // 文本基线 y+thickness+fontSize = 50 → 笔画像素存在 (alpha 240)
    assert!(
        cv.pixmap().data().chunks_exact(4).any(|p| p[3] == 240),
        "文本笔画存在"
    );
}

/// LabeledLinearGauge 横向: 主线列恰 x+pixVal / 影线列 x+pixVal+1,
/// 自条顶 y 延伸到 y+sepHeight (thickness+fontSize+2), 1px 精确列。
#[test]
fn labeled_linear_gauge_horizontal_separator_lines() {
    let f = font();
    let mut g = LabeledLinearGauge::new("RPM", 100, false);
    g.gauge.update(40, "88");
    let mut cv = PixCanvas::new(160, 100).unwrap();
    g.draw(&mut cv, 10, 20, 100, 6, &f, false);

    let pix_val = 40;
    let sep_height = 6 + 24 + 2;
    // 条顶行 (20) 是条边框 shade: 主线 colorNum 叠 shade / 影线 shade 叠 shade
    assert_a_close(a(&cv, 10 + pix_val, 20), src_over_a(240, 42), "主线条顶");
    assert_eq!(a(&cv, 10 + pix_val, 20 + sep_height), 240, "主线尾端");
    assert_a_close(a(&cv, 10 + pix_val + 1, 20), src_over_a(42, 42), "影线条顶");
    assert_eq!(a(&cv, 10 + pix_val - 1, 22), 0, "主线左侧");
    assert_eq!(a(&cv, 10 + pix_val + 2, 22), 0, "影线右侧");
    // 填充: valW=40 → 列 11..48
    assert_eq!(a(&cv, 11, 22), 240, "填充左端");
    assert_eq!(a(&cv, 48, 24), 240, "填充右端");
    assert_eq!(a(&cv, 49, 22), 0, "填充右外");
}

/// LabeledLinearGauge 竖向: 标签参与总宽 → 条相对无标签版右移 labelW。
#[test]
fn labeled_linear_gauge_vertical_label_offsets_bar() {
    let f = font();
    let mut plain = LinearGauge::new("", 100, true);
    plain.set_style_context(100, 8);
    plain.update(50, "88");
    let mut cv_plain = PixCanvas::new(200, 140).unwrap();
    plain.draw(&mut cv_plain, 20, 10, &f, false);

    let mut lab = LabeledLinearGauge::new("油", 100, true);
    lab.gauge.set_style_context(100, 8);
    lab.gauge.update(50, "88");
    let mut cv_lab = PixCanvas::new(200, 140).unwrap();
    lab.draw(&mut cv_lab, 20, 10, 100, 8, &f, false);

    // 填充行 = y+h-1-50 .. y+h-2 = 59..108, 行 100 在内
    let plain_bar = 20 + f.measure("88") + 2;
    assert_eq!(a(&cv_plain, plain_bar + 1, 100), 240, "plain 填充存在");
    let label_w = f.measure("油");
    assert_eq!(
        a(&cv_lab, plain_bar + label_w + 1, 100),
        240,
        "labeled 填充右移 labelW"
    );
    assert_eq!(
        a(&cv_lab, plain_bar + 1, 100),
        0,
        "原位置无条 (行 100 在文本区下方)"
    );
}

/// SpeedRatioBar 分区/边界分支:
/// - 背景 colorNum 全条, 速度比 shade 自底向上
/// - 失速红区右半宽; 锁舵/马赫线在对应行 (butt2 行 y-1..y), r<=0 或 >=1 不画
#[test]
fn speed_ratio_bar_zones_and_boundary_branches() {
    let f = font();
    let mut b = SpeedRatioBar::new();
    b.set_style_context(10, 100);

    // 分支 1: 常态 speed=0.5 stall=0.25 mach=0.8 ail=0.3 rud=0.6
    // (x=45 使速度刻度左端 x-6-templateW("888")=3 落在画布内)
    b.update(0.5, 0.25, 0.8, 0.3, 0.6);
    let mut cv = PixCanvas::new(100, 130).unwrap();
    let (x, y) = (45, 10);
    b.draw(&mut cv, x, y, Some(&f), false);

    // 背景 colorNum: 顶部未被覆盖
    assert_eq!(a(&cv, x, y), 240, "顶部背景 colorNum");
    // shade 速度区 = shade 叠在 colorNum 背景上 (greenH=50 → 行 60..109; 行 60 被刻度覆盖 → 查 61)
    assert_a_close(
        a(&cv, x, y + 51),
        src_over_a(42, 240),
        "shade 区顶 (刻度行下)",
    );
    assert_a_close(a(&cv, x, y + 99), src_over_a(42, 240), "shade 区底");
    // 红区: stallH=25, stallW=5 → 列 x+5..x+9, 行 y+75..y+99 (warning 叠 shade 栈)
    assert_a_close(
        a(&cv, x + 9, y + 99),
        src_over_a(100, src_over_a(42, 240)),
        "红区右下",
    );
    assert_a_close(
        a(&cv, x + 4, y + 99),
        src_over_a(42, 240),
        "红区左邻仍是 shade",
    );
    // 红区上方 (行 84) 仍在 shade 速度区内 (shade 行 60..109, 红区行 85..109)
    assert_a_close(
        a(&cv, x + 9, y + 74),
        src_over_a(42, 240),
        "红区上方仍是 shade 区",
    );
    // 马赫线: machY = y+100-80 = y+20, 行 y+19..y+20, 列 x..x+10 (warning 叠背景)
    assert_a_close(a(&cv, x + 5, y + 20), src_over_a(100, 240), "马赫线行");
    assert_eq!(a(&cv, x + 5, y + 21), 240, "马赫线下方无");
    // 副翼刻度: lockY = y+100-30, 列 x-4..x+w/2-1 (butt 右端列不点亮), 行 lockY-1..lockY
    assert_eq!(a(&cv, x - 4, y + 70), 240, "副翼刻度左端 colorNum");
    assert_eq!(a(&cv, x - 5, y + 70), 0, "刻度左外");
    // 方向舵刻度: lockY = y+100-60 = y+40, 列 x+5..x+w+3 (右端列 x+w+4 不点亮)
    assert_eq!(a(&cv, x + 13, y + 40), 240, "方向舵刻度右端");
    assert_eq!(a(&cv, x + 14, y + 40), 0, "方向舵刻度右外");
    // 速度刻度: tickY = y+50, 左起 x-6-templateW("888"), 右端 x+8 (butt 右端列不点亮)
    let tw = f.measure("888");
    assert_eq!(a(&cv, x - 6 - tw, y + 50), 240, "速度刻度左端");
    // 右端在条上: tick(colorNum) 叠 shade 栈 (行 60 = shade 区顶)
    assert_a_close(
        a(&cv, x + 8, y + 50),
        src_over_a(240, src_over_a(42, 240)),
        "速度刻度右端 (条右缘-1)",
    );
    // x+9 (条右缘列) 无刻度像素: 仅 shade 叠背景栈
    assert_a_close(a(&cv, x + 9, y + 50), src_over_a(42, 240), "条右缘列无刻度");

    // 分支 2: 全零/越界开关 — 无 shade/红区/刻度, 纯背景
    let mut b2 = SpeedRatioBar::new();
    b2.set_style_context(10, 100);
    b2.update(0.0, 0.0, 1.0, 0.0, 1.5);
    let mut cv2 = PixCanvas::new(100, 130).unwrap();
    b2.draw(&mut cv2, x, y, None, false);
    // 速度刻度恒画 (行 y+99..y+100), 取避开刻度行的采样
    for yy in [y, y + 50, y + 98] {
        assert_eq!(a(&cv2, x + 5, yy), 240, "纯背景行 {yy}");
    }
    assert_eq!(a(&cv2, x - 4, y + 70), 0, "ail=0 无左刻度");
    assert_eq!(a(&cv2, x + 14, y + 40), 0, "rud=1.5 无右刻度");
    assert_eq!(a(&cv2, x + 5, y + 20), 240, "mach=1 无红线 (背景)");

    // 分支 3: clamp 越界: speed/stall > 1 → 满条 shade + 满高红区 (刻度行 y 除外)
    let mut b3 = SpeedRatioBar::new();
    b3.set_style_context(10, 100);
    b3.update(1.5, 1.5, 0.0, 0.0, 0.0);
    let mut cv3 = PixCanvas::new(100, 130).unwrap();
    b3.draw(&mut cv3, x, y, None, false);
    assert_a_close(a(&cv3, x, y + 5), src_over_a(42, 240), "speed>1 满条 shade");
    assert_a_close(
        a(&cv3, x + 9, y + 5),
        src_over_a(100, src_over_a(42, 240)),
        "stall>1 满高红区",
    );
    assert_a_close(a(&cv3, x + 4, y + 5), src_over_a(42, 240), "红区左半 shade");
}

/// SpeedRatioBar 数值文本右对齐到刻度左缘, 基线 tickY-3 (Java:141-157)。
/// 扫描速度刻度行 (y+62) 以上、条左侧区域: 只有 "47"+阴影, 右缘不越 tick 缘。
#[test]
fn speed_ratio_bar_value_text_right_aligned() {
    let f = font();
    let mut b = SpeedRatioBar::new();
    b.set_style_context(10, 100);
    b.update(0.47, 0.0, 0.0, 0.0, 0.0);
    let mut cv = PixCanvas::new(80, 130).unwrap();
    let (x, y) = (40, 10);
    b.draw(&mut cv, x, y, Some(&f), false);
    // displayValue = round(47)=47, 右缘 = x-6, 基线 = tickY-3 = y+100-47-3 = y+50
    let text_x = x - 6 - f.measure("47");
    assert!(text_x >= 0, "测试几何: 文本起点在画布内");
    let (mut min_col, mut max_col, mut count) = (i32::MAX, i32::MIN, 0);
    for yy in 0..(y + 50) {
        for xx in text_x..x {
            if a(&cv, xx, yy) > 0 {
                min_col = min_col.min(xx);
                max_col = max_col.max(xx);
                count += 1;
            }
        }
    }
    assert!(count > 0, "存在文本像素");
    assert!(
        (text_x..=text_x + 2).contains(&min_col),
        "文本左缘 ≈ 右对齐起点 ({min_col} vs {text_x})"
    );
    assert!(max_col <= x - 5, "文本右缘含阴影不越 tick+1 缘 ({max_col})");
}

/// FlapAngleBar 正常分支: 三区宽度划分 (used=截断值, margin, overspeed)。
/// total=250/current=25/maxSafe=65 选位使边界列避开固定刻度 (纯色可等值断言):
/// used=50 (列 20..69), margin=80 (70..149), overspeed=120 (150..269);
/// 刻度列 t=20+2·tick ∈ {59..60, 85..86, 139..140, 219..220} 均不在边界。
#[test]
fn flap_angle_bar_normal_split() {
    let f = font();
    let mut b = FlapAngleBar::new();
    b.set_style_context(250, 8);
    assert!(b.update(25.0, 65.0));
    assert_eq!(b.display_text(), " 25/ 65", "%3.0f 格式");
    let mut cv = PixCanvas::new(300, 80).unwrap();
    let (x, y) = (20, 5);
    b.draw(&mut cv, x, y, Some(&f), false);

    let bar_y = y + f.size + 2;
    assert_eq!(
        a(&cv, x + 30, bar_y),
        42,
        "used 区 shade (列 50, 避开刻度 59..60)"
    );
    assert_eq!(a(&cv, x + 49, bar_y), 42, "used 末列 (=50-1)");
    assert_eq!(a(&cv, x + 50, bar_y), 240, "margin 首列 colorNum");
    assert_eq!(a(&cv, x + 129, bar_y), 240, "margin 末列");
    assert_eq!(a(&cv, x + 130, bar_y), 100, "overspeed 首列 warning");
    assert_eq!(a(&cv, x + 249, bar_y), 100, "最右仍是 overspeed");
}

/// FlapAngleBar 刻度几何: 整除定位 tx、100 全高/其余 1/4、方帽上伸 1px
/// (AA OFF 中心规则: 行 y0-1..y1, 下端行 y1+1 不点亮)、列 tx-1..tx。
/// 宽 400 使文本居中后不与所测刻度列重叠 (alpha 才是纯 label 166)。
#[test]
fn flap_angle_bar_tick_geometry() {
    let f = font();
    let mut b = FlapAngleBar::new();
    b.set_style_context(400, 8);
    b.update(0.0, 100.0);
    let mut cv = PixCanvas::new(460, 80).unwrap();
    let (x, y) = (20, 5);
    b.draw(&mut cv, x, y, Some(&f), false);
    let bar_y = y + f.size + 2;
    // tx = x + tick*400/125 (int 除): 20→84? 20*400/125=64 → 84; 33→125+20=125... 计算见断言
    let t33 = x + 33 * 400 / 125; // 20 + 105 = 125
    let t100 = x + 100 * 400 / 125; // 20 + 320 = 340
                                    // 1/4 高刻度 (ext=8/4=2): 行 barY-2-3 .. barY = barY-5..barY (下端行不外伸)
    assert_eq!(a(&cv, t33, bar_y - 5), 166, "1/4 刻度顶部");
    assert_eq!(a(&cv, t33 - 1, bar_y - 5), 166, "刻度左列 (tx-1)");
    assert_eq!(a(&cv, t33, bar_y - 6), 0, "1/4 刻度上方无");
    assert_eq!(a(&cv, t33 - 2, bar_y - 5), 0, "刻度左列外");
    // 100 刻度全高 (ext=8): 行 barY-8-3 .. barY
    assert_eq!(a(&cv, t100, bar_y - 11), 166, "100 刻度方帽上伸 1px");
    assert_eq!(a(&cv, t100, bar_y - 12), 0, "100 刻度上方无");
}

/// FlapAngleBar 超限分支: current > maxSafe → margin=0, 红色紧接 used;
/// 负角/NaN 边界保护不产生负宽 ((int)NaN=0 与 Java 一致)。
/// used=160 → 列 20..179; 刻度 t100 列 179..180 恰压边界 → 断言取清洁列。
#[test]
fn flap_angle_bar_overspeed_and_guard() {
    let f = font();
    let mut b = FlapAngleBar::new();
    b.set_style_context(200, 8);
    b.update(100.0, 50.0);
    let mut cv = PixCanvas::new(240, 80).unwrap();
    let (x, y) = (20, 5);
    b.draw(&mut cv, x, y, Some(&f), false);
    let bar_y = y + f.size + 2;
    // used = 160; margin = 0; overspeed = 40 (首列 180 在刻度上 → 查 181)
    assert_eq!(a(&cv, x + 138, bar_y), 42, "used 区内清洁列");
    assert_eq!(a(&cv, x + 161, bar_y), 100, "超限红色 (清洁列)");
    assert_eq!(a(&cv, x + 199, bar_y), 100, "红色到最右");

    // 负角: used=max(0,-48)=0 → 全条 margin+overspeed
    let mut b2 = FlapAngleBar::new();
    b2.set_style_context(200, 8);
    b2.update(-30.0, 60.0);
    let mut cv2 = PixCanvas::new(240, 80).unwrap();
    b2.draw(&mut cv2, x, y, Some(&f), false);
    assert_eq!(a(&cv2, x, bar_y), 240, "used=0 → margin 起点");

    // NaN 角: 文本 "NaN/NaN"; Java NaN<=NaN 恒 false → 走超限分支, 全条红
    // ((int)NaN=0 → used=0, margin=0, overspeed=total)
    let mut b3 = FlapAngleBar::new();
    b3.set_style_context(200, 8);
    b3.update(f64::NAN, f64::NAN);
    let mut cv3 = PixCanvas::new(240, 80).unwrap();
    b3.draw(&mut cv3, x, y, Some(&f), false);
    assert_eq!(b3.display_text(), "NaN/NaN");
    assert_eq!(
        a(&cv3, x, bar_y),
        100,
        "NaN → 超限分支全红 (Java NaN 比较恒 false)"
    );
}

/// 线基元几何 (期望值 = Java 8 oracle 实测像素盒):
/// - butt2 AA OFF: drawLine(10,·,20,·) 覆盖列 10..19 — 右端列不点亮
///   (宽>1 的 strokedShape 中心规则光栅, 端点含像素仅 1px Bresenham 快速路径)
/// - square2 AA OFF: drawLine(·,10,·,25) 覆盖列 tx-1..tx / 行 9..25 —
///   方帽下端行不点亮
/// - AA ON: 宽 2 线 3 行/列柔边, oracle 不透明色 a=128/255/128 角点 a=64,
///   此处按 cov_color 同式 (colors().num a=240 → 120/60; LABEL 166 → 83/42)
#[test]
fn line_primitive_pixel_boxes() {
    // butt2 AA OFF: 列 10..19, 行 14..15
    let mut cv = PixCanvas::new(40, 40).unwrap();
    butt_line(&mut cv, 10, 15, 20, 15, 2, colors().num, false);
    assert_eq!(a(&cv, 10, 14), 240, "butt2 左端列");
    assert_eq!(a(&cv, 19, 15), 240, "butt2 右端列 (oracle: 端点列不点亮)");
    assert_eq!(a(&cv, 20, 15), 0, "butt2 右端外");
    assert_eq!(a(&cv, 10, 13), 0, "butt2 上行外");
    assert_eq!(a(&cv, 10, 16), 0, "butt2 下行外");
    assert_eq!(a(&cv, 9, 15), 0, "butt2 左外");

    // butt2 AA ON: 覆盖盒 [10.5,20.5]×[14.5,16.5] → 3 行柔边 + 端点列半覆盖
    // (oracle: 21 列×3 行 = 63 非零像素)
    let mut cvs = PixCanvas::new(40, 40).unwrap();
    butt_line(&mut cvs, 10, 15, 20, 15, 2, colors().num, true);
    assert_eq!(a(&cvs, 15, 15), 240, "AA 中行全值");
    assert_eq!(a(&cvs, 15, 14), 120, "AA 上柔边行 a=round(240·0.5)");
    assert_eq!(a(&cvs, 15, 16), 120, "AA 下柔边行");
    assert_eq!(a(&cvs, 15, 13), 0, "AA 柔边外");
    assert_eq!(a(&cvs, 10, 15), 120, "AA 左端点列半覆盖");
    assert_eq!(a(&cvs, 20, 15), 120, "AA 右端点列 (AA ON 才点亮)");
    assert_eq!(a(&cvs, 10, 14), 60, "AA 角点 1/4 覆盖 (oracle a=64 同式)");
    assert_eq!(a(&cvs, 20, 16), 60, "AA 右下角点");
    assert_eq!(a(&cvs, 9, 15), 0, "AA 左外");
    assert_eq!(a(&cvs, 21, 15), 0, "AA 右外");

    // square2 AA OFF: 列 29..30, 行 9..25 (方帽上端外伸 1 行、下端行不点亮)
    let mut cv2 = PixCanvas::new(40, 40).unwrap();
    vline_square2(&mut cv2, 30, 10, 25, colors().label, false);
    assert_eq!(a(&cv2, 29, 9), 166, "square2 左列方帽上伸");
    assert_eq!(
        a(&cv2, 30, 25),
        166,
        "square2 下端行 y1 (oracle: y1+1 行不点亮)"
    );
    assert_eq!(a(&cv2, 30, 26), 0, "square2 下端行外");
    assert_eq!(a(&cv2, 30, 8), 0, "square2 上外");
    assert_eq!(a(&cv2, 28, 15), 0, "square2 左外");
    assert_eq!(a(&cv2, 31, 15), 0, "square2 右外");

    // square2 AA ON: 覆盖盒 [29.5,31.5]×[9.5,26.5] → 3 列柔边, 端行半透明
    // (oracle: 行 4..26 端行半透明, 列 a=128/255/128)
    let mut cv2s = PixCanvas::new(40, 40).unwrap();
    vline_square2(&mut cv2s, 30, 10, 25, colors().label, true);
    assert_eq!(a(&cv2s, 30, 15), 166, "AA 中列全值");
    assert_eq!(a(&cv2s, 29, 15), 83, "AA 左柔边列 a=round(166·0.5)");
    assert_eq!(a(&cv2s, 31, 15), 83, "AA 右柔边列");
    assert_eq!(a(&cv2s, 30, 9), 83, "AA 上端行半透明");
    assert_eq!(a(&cv2s, 30, 26), 83, "AA 下端行 (AA ON 才点亮)");
    assert_eq!(a(&cv2s, 29, 9), 42, "AA 角点 1/4 覆盖");
    assert_eq!(a(&cv2s, 30, 8), 0, "AA 上外");
    assert_eq!(a(&cv2s, 32, 15), 0, "AA 右外");

    // 1px: 列恰 x, 行 y0..y1 (AA 开关输出一致)
    let mut cv3 = PixCanvas::new(40, 40).unwrap();
    vline_1px(&mut cv3, 12, 5, 15, colors().warning);
    vline_1px(&mut cv3, 14, 5, 15, colors().warning);
    assert_eq!(a(&cv3, 12, 5), 100, "1px 线顶");
    assert_eq!(a(&cv3, 12, 15), 100, "1px 线底");
    assert_eq!(a(&cv3, 11, 10), 0, "1px 线左外");
    assert_eq!(a(&cv3, 13, 10), 0, "1px 线右外");
    assert_eq!(a(&cv3, 14, 10), 100, "1px AA ON 同盒");
    assert_eq!(a(&cv3, 15, 10), 0, "1px AA ON 右外");
}

/// drawRect 负/零尺寸语义 (Java 8 oracle): 负宽/负高整体不绘制;
/// 零宽退化 1px 竖线 (列 x, 行 y..y+h)、零高退化 1px 横线、双零无输出
#[test]
fn ring_negative_and_degenerate() {
    let mut cv = PixCanvas::new(40, 40).unwrap();
    primitives::ring1px(&mut cv, 10, 10, -4, 20, colors().num);
    primitives::ring1px(&mut cv, 10, 10, 20, -4, colors().num);
    primitives::ring1px(&mut cv, 10, 10, -4, -9, colors().num);
    assert!(
        cv.pixmap().data().iter().all(|&b| b == 0),
        "负宽/负高 0 像素"
    );

    // 零宽: oracle drawRect(50,10,0,20) = 列 50 行 10..30 的 1px 竖线
    let mut cv2 = PixCanvas::new(40, 40).unwrap();
    primitives::ring1px(&mut cv2, 20, 5, 0, 15, colors().num);
    assert_eq!(a(&cv2, 20, 5), 240, "零宽退化竖线顶");
    assert_eq!(a(&cv2, 20, 20), 240, "零宽退化竖线底 (行 y..y+h)");
    assert_eq!(a(&cv2, 20, 21), 0, "竖线底外");
    assert_eq!(a(&cv2, 19, 10), 0, "竖线左外");

    // 零高: 1px 横线 列 x..x+w
    let mut cv3 = PixCanvas::new(40, 40).unwrap();
    primitives::ring1px(&mut cv3, 5, 20, 15, 0, colors().num);
    assert_eq!(a(&cv3, 5, 20), 240, "零高退化横线左端");
    assert_eq!(a(&cv3, 20, 20), 240, "零高退化横线右端");
    assert_eq!(a(&cv3, 21, 20), 0, "横线右外");
    assert_eq!(a(&cv3, 10, 19), 0, "横线上外");

    // 双零: drawRect 的 4 条边线全为零长度段, 无输出
    let mut cv4 = PixCanvas::new(40, 40).unwrap();
    primitives::ring1px(&mut cv4, 10, 10, 0, 0, colors().num);
    assert!(cv4.pixmap().data().iter().all(|&b| b == 0), "双零无输出");
}

/// fmt_pct3 边界: HALF_UP 舍入、宽度补齐、负数 (含 -0.0 保号)、
/// 0.5 的 f64 前驱 (精确十进制舍入)、NaN
#[test]
fn fmt_pct3_rounding_and_padding() {
    assert_eq!(fmt_pct3(0.0), "  0");
    assert_eq!(fmt_pct3(99.5), "100", "HALF_UP 进位且自然超宽");
    assert_eq!(fmt_pct3(0.4), "  0");
    assert_eq!(fmt_pct3(0.5), "  0", "波21: 精确半点 nearest-even 取偶");
    assert_eq!(fmt_pct3(-0.5), " -0", "波21: nearest-even (Rust 舍入)");
    assert_eq!(fmt_pct3(-2.4), " -2");
    // oracle: String.format("%3.0f", -0.0) 数值部分 "-0" (负零保号), 宽 3 补成 " -0"
    assert_eq!(fmt_pct3(-0.0), " -0", "负零保号 (Java oracle, 宽 3 含符号)");
    assert_eq!(fmt_pct3(-0.4), " -0", "负值舍到零保负号");
    // oracle: 0.5 的 f64 前驱按精确十进制 HALF_UP 舍到 0
    // (v+0.5 在 f64 中进到 1.0 的舍入路径已修正)
    assert_eq!(fmt_pct3(0.49999999999999994), "  0");
    assert_eq!(fmt_pct3(f64::NAN), "NaN");
}

/// 脏检查: 同值重复 update 返回 false, 变化置脏, draw 清脏
#[test]
fn dirty_checking_semantics() {
    let mut g = LinearGauge::new("T", 100, true);
    assert!(g.update(50, "50"));
    assert!(!g.update(50, "50"), "同值不脏");
    assert!(g.is_dirty());
    let f = font();
    let mut cv = PixCanvas::new(100, 140).unwrap();
    g.draw(&mut cv, 5, 5, &f, false);
    assert!(!g.is_dirty(), "draw 后清脏");
    assert!(g.update(60, "60"));

    let mut b = FlapAngleBar::new();
    assert!(b.update(10.0, 100.0));
    assert!(!b.update(10.0, 100.0));

    let mut s = SpeedRatioBar::new();
    assert!(s.update(0.5, 0.1, 0.1, 0.1, 0.1));
    assert!(!s.update(0.5, 0.1, 0.1, 0.1, 0.1));
}

/// W3 契约: 风格/值色 setter 置脏 (同值不置脏) — 按 is_dirty() 门控 draw
/// 的组装层 (MiniHUD ThrottleBar 的 valueColor 注入) 必须经 setter 改字段
#[test]
fn linear_gauge_style_setters_mark_dirty() {
    let f = font();
    let mut g = LinearGauge::new("T", 100, true);
    g.set_style_context(40, 6);
    let mut cv = PixCanvas::new(120, 140).unwrap();
    g.update(50, "50");
    g.draw(&mut cv, 5, 5, &f, false);
    assert!(!g.is_dirty(), "draw 后清脏");

    g.set_value_color(Some(colors().warning));
    assert!(g.is_dirty(), "值色注入置脏");
    g.set_vertical(false);
    g.set_tick_on_right(true);
    g.set_max_value(200);
    g.set_label("U");
    g.draw(&mut cv, 5, 5, &f, false);
    assert!(!g.is_dirty(), "再次 draw 清脏");

    g.set_vertical(false);
    g.set_value_color(Some(colors().warning));
    assert!(!g.is_dirty(), "同值 setter 不置脏");
}
