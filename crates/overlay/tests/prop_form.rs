//! 属性表单行模型黑盒场景 (D4b 下沉面): schema→行映射 / 值三兜底
//! (实值 → default_props → 类型零值) / EnumCycle 分型 / 未注册类型 /
//! 页面属性。构造 doc 直接测, 零窗口零字体不经编辑会话。
#![allow(non_snake_case)] // 中文场景命名是项目惯例

use expect_test::expect;

use kernel::config::json_model::{ComponentDoc, PageDoc};
use overlay::widgets::{component_prop_rows, lookup_widget, page_prop_rows, PropCtrlKind, PropRowDef};

/// 行集快照文本: 每行 `label | key | kind(枚举候选) | 值文本`
/// (空值显示 <空> — 避免快照行尾空白)
fn dump(rows: &[PropRowDef]) -> String {
    rows.iter()
        .map(|r| {
            let kind = if r.kind == PropCtrlKind::EnumCycle {
                format!("{:?}{:?}", r.kind, r.enum_values)
            } else {
                format!("{:?}", r.kind)
            };
            let value = if r.value_text.is_empty() {
                "<空>"
            } else {
                &r.value_text
            };
            format!("{} | {} | {} | {}", r.label, r.key, kind, value)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// core.data.field 测试组件 (schema 10 行; props 由场景覆写)
fn field_comp(props: serde_json::Value) -> ComponentDoc {
    ComponentDoc {
        id: "v1".into(),
        r#type: "core.data.field".into(),
        props,
        ..Default::default()
    }
}

/// 选中 data.field 的全行集: 通用 3 行 + schema 10 行 (顺序与分型);
/// props 空 → 值走 default_props 兜底, default 缺项 → 类型零值
/// (Int=0 / Bool=false / 其余空串)
#[test]
fn propform_data_field全行集_空props走default与零值兜底() {
    let comp = field_comp(serde_json::json!({}));
    let meta = lookup_widget("core.data.field");
    expect![[r#"
        标识 | __id | Text | v1
        显示 | __enabled | Toggle | true
        显示条件 | __visible_when | Text | <空>
        数据绑定 | target | Text | ias
        描述 | label | Text | 表  速
        单位 | unit | Text | Km/h
        小数位 | precision | IntStepper | 0
        格式 | format | EnumCycle["", "TIME_MM_SS"] | <空>
        预览值 | previewValue | Text | 500
        显示条件 | visibleWhen | Text | <空>
        NA 条件 | naWhen | Text | <空>
        英制切换 | imperial | Toggle | false
        字号增量 | fontAdd | IntStepper | 0"#]]
    .assert_eq(&dump(&component_prop_rows(&comp, meta)));
}

/// 值三兜底之实值层: props 实值优先于 default_props
/// (覆盖 label/precision/format/imperial/visibleWhen, 其余仍走兜底)
#[test]
fn propform_实值优先于default兜底() {
    let comp = field_comp(serde_json::json!({
        "label": "高度",
        "precision": 2,
        "format": "TIME_MM_SS",
        "imperial": true,
        "visibleWhen": "jet",
    }));
    let meta = lookup_widget("core.data.field");
    expect![[r#"
        标识 | __id | Text | v1
        显示 | __enabled | Toggle | true
        显示条件 | __visible_when | Text | <空>
        数据绑定 | target | Text | ias
        描述 | label | Text | 高度
        单位 | unit | Text | Km/h
        小数位 | precision | IntStepper | 2
        格式 | format | EnumCycle["", "TIME_MM_SS"] | TIME_MM_SS
        预览值 | previewValue | Text | 500
        显示条件 | visibleWhen | Text | jet
        NA 条件 | naWhen | Text | <空>
        英制切换 | imperial | Toggle | true
        字号增量 | fontAdd | IntStepper | 0"#]]
    .assert_eq(&dump(&component_prop_rows(&comp, meta)));
}

/// EnumCycle 分型: format 行带 schema 候选值 (点击循环的数据源)
#[test]
fn propform_枚举行分型携带候选值() {
    let comp = field_comp(serde_json::json!({}));
    let meta = lookup_widget("core.data.field");
    let row = component_prop_rows(&comp, meta)
        .into_iter()
        .find(|r| r.key == "format")
        .expect("format 行在场");
    expect!["EnumCycle [\"\", \"TIME_MM_SS\"]"].assert_eq(&format!(
        "{:?} {:?}",
        row.kind, row.enum_values
    ));
}

/// 未注册类型 (meta = None): 仅通用 3 行 (标识/显示/显示条件)
#[test]
fn propform_未注册类型仅通用行() {
    let comp = ComponentDoc {
        id: "ghost".into(),
        r#type: "core.ghost".into(),
        ..Default::default()
    };
    expect![[r#"
        标识 | __id | Text | ghost
        显示 | __enabled | Toggle | true
        显示条件 | __visible_when | Text | <空>"#]]
    .assert_eq(&dump(&component_prop_rows(&comp, lookup_widget("core.ghost"))));
}

/// 页面属性 3 行: 页名(Text) / 内边距(IntStepper) / 字号增量(IntStepper)
#[test]
fn propform_页面属性三行() {
    let mut page = PageDoc {
        name: "试驾页".into(),
        padding: 6,
        ..Default::default()
    };
    page.font.size_add = 2; // Default 0, 显式给值便于快照区分
    expect![[r#"
        页名 | __page_name | Text | 试驾页
        内边距 | __page_padding | IntStepper | 6
        字号增量 | __page_font_add | IntStepper | 2"#]]
    .assert_eq(&dump(&page_prop_rows(&page)));
}
