//! 对比窗口数据面黑盒场景 (commands_comparison 的纯函数族, 已放宽 pub):
//! fm1 归一化 / 结构合并 (独有键并集) / 胜负规则 / COPY 文案生成。
//! 零文件零 UI — build_structure 输入是 fmdata 行文本的纯字符串面。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


use expect_test::expect;

use webui::commands_comparison::{
    build_copy_text, build_structure, comparison_title, normalize_secondary, row_win,
};

/// fm1 归一化: None/空串/同名 → 单机; 异名 → Some
#[test]
fn normalize_secondary_归一化() {
    let cases: Vec<(Option<&str>, Option<&str>)> = vec![
        (None, None),
        (Some(""), None),
        (Some("  "), Some("  ")), // 原样保留 (trim 归调用方; 仅空串判None)
        (Some("bf109"), None),    // 与 fm0 同名 → 单机
        (Some("fw190"), Some("fw190")),
    ];
    let actual = cases
        .iter()
        .map(|&(fm1, _)| {
            format!(
                "{fm1:?} vs fm0=bf109 -> {:?}",
                normalize_secondary("bf109", fm1)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        None vs fm0=bf109 -> None
        Some("") vs fm0=bf109 -> None
        Some("  ") vs fm0=bf109 -> Some("  ")
        Some("bf109") vs fm0=bf109 -> None
        Some("fw190") vs fm0=bf109 -> Some("fw190")"#]]
    .assert_eq(&actual);
}

/// 窗口标题: 单机/双机两形态 (DTO title 与 web_windows 窗口 title 同源)
#[test]
fn comparison_title_两形态() {
    expect!["Aircraft Data: bf109"].assert_eq(&comparison_title("bf109", None));
    expect!["Comparison: bf109 vs fw190"].assert_eq(&comparison_title("bf109", Some("fw190")));
}

/// 结构合并: lines0 建结构 (含分节头), lines1 独有键插入最近命中位之后
#[test]
fn build_structure_独有键并集() {
    let lines0: Vec<String> = vec![
        "------ 速度 ------".into(), // 分节头 (6 连字符; 结构行, 不进 map)
        "空重(kg): 4644.0".into(),
        "  ".into(),                  // 空行跳过
        "最大燃油重量(kg): 1500".into(),
    ];
    let lines1: Vec<String> = vec![
        "------ 速度 ------".into(), // 合并段头行跳过 (防重复)
        "空重(kg): 5000.0".into(),   // 共有键: 只进 map1, 不动结构
        "独有指标: 42".into(),        // 独有键 → 插在最近命中 (空重) 之后
        "尾部键: 7".into(),           // 插入位 = 尾部 push (lastMatch 之后)
    ];
    let (structure, map0, map1) = build_structure(&lines0, &lines1);

    let struct_lines = structure
        .iter()
        .map(|i| {
            let tag = if i.is_header { "H" } else { "P" };
            format!("{tag}[{}]", i.text)
        })
        .collect::<Vec<_>>()
        .join(" | ");
    expect!["H[速度] | P[空重(kg)] | P[独有指标] | P[尾部键] | P[最大燃油重量(kg)]"].assert_eq(&struct_lines);

    // 值映射: map0 只含 fm0 键; map1 全量含 fm1 键
    expect!["4644.0"].assert_eq(map0.get("空重(kg)").unwrap());
    expect!["1500"].assert_eq(map0.get("最大燃油重量(kg)").unwrap());
    assert!(!map0.contains_key("独有指标"), "fm0 无独有键");
    expect!["5000.0"].assert_eq(map1.get("空重(kg)").unwrap());
    expect!["42"].assert_eq(map1.get("独有指标").unwrap());
}

/// fm0 空: lines1 全量追加进结构 (插入位 0 起, 头部行照跳)
#[test]
fn build_structure_fm0空全量追加() {
    let lines1: Vec<String> = vec!["翼载荷(kg/m²): 200.5".into(), "无值行".into()];
    let (structure, map0, map1) = build_structure(&[], &lines1);
    // 全部键来自 fm1; fm0 map 空
    let actual = structure
        .iter()
        .map(|i| {
            let tag = if i.is_header { "H" } else { "P" };
            format!("{tag}[{}]", i.text)
        })
        .collect::<Vec<_>>()
        .join(" | ");
    // "无值行" 无冒号 → 不建行 (解析段过滤)
    expect!["P[翼载荷(kg/m²)]"].assert_eq(&actual);
    assert!(map0.is_empty());
    expect!["200.5"].assert_eq(map1.get("翼载荷(kg/m²)").unwrap());
}

/// 胜负规则: 规则键按 lower/higher 判向; 无规则键/同值/单机 → Draw
#[test]
fn row_win_胜负规则() {
    // 空重: 轻好 (lower is better) — 左小 → Left
    expect!["空重(kg) 4644 vs 5000 -> Left"].assert_eq(&format!(
        "空重(kg) 4644 vs 5000 -> {:?}",
        row_win("空重(kg)", "4644", "5000", false)
    ));
    // 反向: 左大 → Right
    expect!["空重(kg) 5000 vs 4644 -> Right"].assert_eq(&format!(
        "空重(kg) 5000 vs 4644 -> {:?}",
        row_win("空重(kg)", "5000", "4644", false)
    ));
    // 最大燃油: 重好 (higher is better) — 左大 → Left
    expect!["最大燃油重量(kg) 1500 vs 1200 -> Left"].assert_eq(&format!(
        "最大燃油重量(kg) 1500 vs 1200 -> {:?}",
        row_win("最大燃油重量(kg)", "1500", "1200", false)
    ));
    // 同值 (差 <0.001) → Draw
    expect!["空重(kg) 4644.0 vs 4644.0005 -> Draw"].assert_eq(&format!(
        "空重(kg) 4644.0 vs 4644.0005 -> {:?}",
        row_win("空重(kg)", "4644.0", "4644.0005", false)
    ));
    // 无规则键 → Draw (灰)
    expect!["未知指标 1 vs 2 -> Draw"].assert_eq(&format!(
        "未知指标 1 vs 2 -> {:?}",
        row_win("未知指标", "1", "2", false)
    ));
    // 缺键补 "-": 无数字可提 → Draw
    expect!["空重(kg) - vs 5000 -> Draw"].assert_eq(&format!(
        "空重(kg) - vs 5000 -> {:?}",
        row_win("空重(kg)", "-", "5000", false)
    ));
    // 单机模式恒 Draw
    expect!["空重(kg) 4644 vs 5000 单机 -> Draw"].assert_eq(&format!(
        "空重(kg) 4644 vs 5000 单机 -> {:?}",
        row_win("空重(kg)", "4644", "5000", true)
    ));
    // 临界速度 [min, max]: 取第二数 (vne), 大好
    expect!["临界速度(km/h) [144, 1167] vs [150, 1100] -> Left"].assert_eq(&format!(
        "临界速度(km/h) [144, 1167] vs [150, 1100] -> {:?}",
        row_win("临界速度(km/h)", "[144, 1167]", "[150, 1100]", false)
    ));
}

/// COPY 文案: 单机/对比两形态 (含胜负方名标注)
#[test]
fn build_copy_text_文案生成() {
    let lines0: Vec<String> = vec!["------ 基本 ------".into(), "空重(kg): 4644.0".into()];
    let lines1: Vec<String> = vec!["空重(kg): 5000.0".into(), "独有指标: 42".into()];
    let (structure, map0, map1) = build_structure(&lines0, &lines1);

    // 对比模式: 分节头线 + 双值 + 胜负方名 [bf109 +] (trim_end: 尾部换行不进快照)
    let dual = build_copy_text("bf109", "fw190", false, &structure, &map0, &map1);
    expect![[r#"
        ========== Comparison: bf109 vs fw190 ==========

        ---------- 基本 ----------
        空重(kg): 4644.0 vs 5000.0  [bf109 +]
        独有指标: - vs 42"#]]
    .assert_eq(dual.trim_end());

    // 单机模式: 单值列 (fm1 值不可见)
    let solo = build_copy_text("bf109", "", true, &structure, &map0, &map1);
    expect![[r#"
        ========== Aircraft Data: bf109 ==========

        ---------- 基本 ----------
        空重(kg): 4644.0
        独有指标: -"#]]
    .assert_eq(solo.trim_end());
}
