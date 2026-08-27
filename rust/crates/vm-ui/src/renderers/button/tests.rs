use super::*;

fn button_row(prop: Option<&str>, fgcolor: Option<&str>) -> RowConfig {
    let mut r = RowConfig::new("恢复出厂设置".into(), None, "%s".into());
    r.r#type = "BUTTON".into();
    r.property = prop.map(str::to_string);
    r.fg_color = fgcolor.map(str::to_string);
    r
}

// :fgcolor 解析 (Java parseColor 语义: 逐段 trim / 越界不钳位 / 无 alpha)
#[test]
fn parse_fg_color_semantics() {
    // cfg 实况 "255, 100, 100" (factoryReset)
    assert_eq!(parse_fg_color("255, 100, 100"), Some([255, 100, 100, 255]));
    // 逐段 trim (Java p[i].trim())
    assert_eq!(parse_fg_color(" 255 , 100 , 100 "), Some([255, 100, 100, 255]));
    // 尾逗号: split 丢尾部空串 → 3 段仍成立
    assert_eq!(parse_fg_color("255, 100, 100,"), Some([255, 100, 100, 255]));
    // 第 4 段忽略 (Color(r,g,b) 三参构造)
    assert_eq!(parse_fg_color("255, 100, 100, 99"), Some([255, 100, 100, 255]));
    // 不足 3 段 / 非法 / 越界 (无钳位, Java 构造器抛异常) → None
    assert_eq!(parse_fg_color("255,100"), None);
    assert_eq!(parse_fg_color("a,b,c"), None);
    assert_eq!(parse_fg_color("300, 0, 0"), None);
    assert_eq!(parse_fg_color("-1, 0, 0"), None);
    assert_eq!(parse_fg_color("1.5,2,3"), None);
    assert_eq!(parse_fg_color(""), None);
    // Java trim 口径 (码点 <=0x20): 控制符首尾可删 → 解析成功
    assert_eq!(parse_fg_color("\u{0001}255, 100, 100\u{001F}"), Some([255, 100, 100, 255]));
    // nbsp (>0x20): Java trim 不删 → parseInt 失败 → 不着色 (Unicode trim 会误删)
    assert_eq!(parse_fg_color("\u{00A0}255, 100, 100"), None);
}

// 已知动作键集合 (Java 分派键, 接线批的动作注册面)
#[test]
fn known_actions_cover_java_dispatch() {
    for k in ["resetConfig", "openComparison", "openPowerCurve", "importConfig", "factoryReset"] {
        assert!(KNOWN_ACTIONS.contains(&k), "缺 Java 动作键 {k}");
    }
    assert!(!KNOWN_ACTIONS.contains(&"unknown"));
}

// 视图构建冒烟: 已知动作/未知键/无键/前景色四形态
#[test]
fn view_row_builds() {
    let r1 = button_row(Some("factoryReset"), Some("255, 100, 100"));
    let _el = view_row(&r1);
    let r2 = button_row(Some("unknownAction"), None);
    let _el2 = view_row(&r2);
    let r3 = button_row(None, None);
    let _el3 = view_row(&r3);
    let r4 = button_row(Some("resetConfig"), Some("bad,fg"));
    let _el4 = view_row(&r4);
}
