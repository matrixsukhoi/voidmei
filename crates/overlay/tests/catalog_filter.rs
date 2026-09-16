//! 组件库目录黑盒场景 (D4b 下沉面): 全量分组 / 搜索过滤 / 空组剔除 /
//! 组序稳定 / 未知查询空集。纯数据断言, 零窗口零字体。
#![allow(non_snake_case)] // 中文场景命名是项目惯例

use expect_test::expect;

use overlay::widgets::{catalog_groups, catalog_groups_filtered, widget_registry};

/// 分组快照文本: 每组 `分类: 名字, 名字, ...` (idx → display_zh, 可读性优先)
fn dump(groups: &[(&'static str, Vec<usize>)]) -> String {
    groups
        .iter()
        .map(|(cat, idxs)| {
            let names: Vec<&str> = idxs
                .iter()
                .map(|&i| widget_registry()[i].display_zh)
                .collect();
            format!("{cat}: {}", names.join(", "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 空查询 = 全量 (快照 + 逐组相等双断言)
#[test]
fn catalog_空查询等于全量() {
    expect![[r#"
        文本: 速度读数, AoA 指示, 高度读数, 能量读数, 襟翼/可变翼, 减速板, 起落架, SEP 读数, G 读数, 机动刻度条, 数据字段, 起落架告警, FM字段, FM文本行
        仪表: 智能襟翼条, 速度条, 油门条, 姿态指示器, 罗盘, 引擎仪表, 操纵面十字, 地平仪窗, 襟翼竖条, 方向舵横条
        装饰: 准星
        列表: FM数据列表
        图表: 推力-真空速曲线
        布局容器: 列表容器"#]]
    .assert_eq(&dump(&catalog_groups()));
    assert_eq!(
        dump(&catalog_groups_filtered("")),
        dump(&catalog_groups()),
        "空查询应返回全量分组"
    );
}

/// 中文子串命中: "字段" 只命中文本组的数据字段, 其余组整体剔除
#[test]
fn catalog_中文子串命中与空组剔除() {
    expect!["文本: 数据字段, FM字段"]
    .assert_eq(&dump(&catalog_groups_filtered("字段")));
}

/// 多组命中的组序稳定: 过滤后组序 = 全量聚簇序的子序列
#[test]
fn catalog_多组命中组序稳定() {
    let filtered = catalog_groups_filtered("表");
    expect![[r#"
        仪表: 引擎仪表
        列表: FM数据列表
        布局容器: 列表容器"#]]
    .assert_eq(&dump(&filtered));
    // 程序断言: 命中组相对序与全量一致 (子序列)
    let full_order: Vec<&str> = catalog_groups().iter().map(|(c, _)| *c).collect();
    let hit_order: Vec<&str> = filtered.iter().map(|(c, _)| *c).collect();
    let mut it = full_order.into_iter();
    let stable = hit_order.iter().all(|h| it.any(|f| f == *h));
    assert!(stable, "过滤后组序应是全量组序的子序列");
}

/// 未知查询 = 空集
#[test]
fn catalog_未知查询空集() {
    assert!(catalog_groups_filtered("不存在的词条zzz").is_empty());
}
