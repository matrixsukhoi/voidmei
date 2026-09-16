//! 组件库目录 (D4b 自 voidmei edit_chrome 下沉): 注册表序 → 分类分组 +
//! 搜索过滤。纯函数 (零窗口零字体), 黑盒直测。

use super::registry::{widget_registry, WidgetCategory};

/// 分类中文名 (侧栏分组标题)
fn category_zh(c: &WidgetCategory) -> &'static str {
    match c {
        WidgetCategory::Text => "文本",
        WidgetCategory::Gauge => "仪表",
        WidgetCategory::Chart => "图表",
        WidgetCategory::List => "列表",
        WidgetCategory::Composite => "复合 (黑盒)",
        WidgetCategory::Decor => "装饰",
        WidgetCategory::Layout => "布局容器",
    }
}

/// 组件库全量分组 (注册表序聚簇; 组间顺序 = 首次出现序)
pub fn catalog_groups() -> Vec<(&'static str, Vec<usize>)> {
    let mut out: Vec<(&'static str, Vec<usize>)> = Vec::new();
    for (i, m) in widget_registry().iter().enumerate() {
        let cat = category_zh(&m.category);
        match out.iter_mut().find(|(c, _)| *c == cat) {
            Some((_, v)) => v.push(i),
            None => out.push((cat, vec![i])),
        }
    }
    out
}

/// 组件库搜索过滤: query 非空时按 display_zh 包含匹配过滤条目, 空组剔除;
/// 组间顺序保持聚簇序 (空查询 = 全量)
pub fn catalog_groups_filtered(query: &str) -> Vec<(&'static str, Vec<usize>)> {
    let groups = catalog_groups();
    if query.is_empty() {
        return groups;
    }
    groups
        .into_iter()
        .filter_map(|(cat, idxs)| {
            let hits: Vec<usize> = idxs
                .into_iter()
                .filter(|&i| widget_registry()[i].display_zh.contains(query))
                .collect();
            (!hits.is_empty()).then_some((cat, hits))
        })
        .collect()
}
