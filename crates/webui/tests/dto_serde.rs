//! webui::dto 序列化形状黑盒场景: 行/panel 树的 camelCase 契约 (前端
//! AntD 表单消费面)、FormMessageDto 反序列化 (dispatcher 入口的消息形状)、
//! web 窗口域 DTO 的语义标序列化 (win 整数 / 拐点族小写串)。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


use expect_test::expect;
use serde_json::json;

use kernel::config::json_model::{ConfigValue, GroupConfig, RowConfig};
use webui::dto::{
    ComparisonRowDto, FormMessageDto, InflectionKind, InflectionPointDto, Win,
};

/// 行 DTO 序列化: camelCase + `type` 字段 rename + slider 区间字段名
#[test]
fn row_dto_形状camel_case() {
    let r = RowConfig {
        label: "字号".into(),
        r#type: "SLIDER".into(),
        property: Some("fontSize".into()),
        value: Some(ConfigValue::Int(7)),
        default_value: Some(ConfigValue::Int(0)),
        min_val: -6,
        max_val: 20,
        group_columns: 3,
        visible_when: Some("value > 0".into()),
        children: vec![RowConfig {
            label: "子行".into(),
            r#type: "HEADER".into(),
            ..RowConfig::default()
        }],
        ..RowConfig::default()
    };
    let v = serde_json::to_value(&r).unwrap();
    expect![[r#"
        {
          "label": "字号",
          "type": "SLIDER",
          "property": "fontSize",
          "targetName": null,
          "value": 7,
          "defaultValue": 0,
          "unit": "",
          "precision": 0,
          "format": "%s",
          "source": null,
          "minVal": -6,
          "maxVal": 20,
          "groupColumns": 3,
          "desc": null,
          "descImg": null,
          "previewValue": null,
          "fgColor": null,
          "unitSource": null,
          "precisionSource": null,
          "visibleWhen": "value > 0",
          "naWhen": null,
          "children": [
            {
              "label": "子行",
              "type": "HEADER",
              "property": null,
              "targetName": null,
              "value": null,
              "defaultValue": null,
              "unit": "",
              "precision": 0,
              "format": "%s",
              "source": null,
              "minVal": 0,
              "maxVal": 100,
              "groupColumns": 0,
              "desc": null,
              "descImg": null,
              "previewValue": null,
              "fgColor": null,
              "unitSource": null,
              "precisionSource": null,
              "visibleWhen": null,
              "naWhen": null,
              "children": []
            }
          ]
        }"#]]
    .assert_eq(&serde_json::to_string_pretty(&v).unwrap());
}

/// panel 树序列化: GroupConfig 全字段 camelCase (GetLayoutTree 的 wire 形状)
#[test]
fn panel_树形状camel_case() {
    let g = GroupConfig {
        title: "面板A".into(),
        visible: true,
        font_size: 7,
        columns: 2,
        panel_columns: 4,
        switch_key: Some("flightInfoSwitch".into()),
        rows: vec![RowConfig {
            label: "开关".into(),
            r#type: "SWITCH".into(),
            property: Some("k1".into()),
            value: Some(ConfigValue::Bool(true)),
            ..RowConfig::default()
        }],
        ..GroupConfig::default()
    };
    let v = serde_json::to_value(&g).unwrap();
    // alpha/fontName 等缺省字段一并钉死 (前端按整形状消费)
    expect![[r#"
        {
          "title": "面板A",
          "alpha": 150,
          "visible": true,
          "fontName": null,
          "fontSize": 7,
          "columns": 2,
          "panelColumns": 4,
          "switchKey": "flightInfoSwitch",
          "rows": [
            {
              "label": "开关",
              "type": "SWITCH",
              "property": "k1",
              "targetName": null,
              "value": true,
              "defaultValue": null,
              "unit": "",
              "precision": 0,
              "format": "%s",
              "source": null,
              "minVal": 0,
              "maxVal": 100,
              "groupColumns": 0,
              "desc": null,
              "descImg": null,
              "previewValue": null,
              "fgColor": null,
              "unitSource": null,
              "precisionSource": null,
              "visibleWhen": null,
              "naWhen": null,
              "children": []
            }
          ]
        }"#]]
    .assert_eq(&serde_json::to_string_pretty(&v).unwrap());
}

/// FormMessageDto 反序列化: 每变体一条 JSON → 消息 (前端 → dispatcher 入口形状)
#[test]
fn form_message_反序列化各变体() {
    let cases: Vec<(&str, serde_json::Value)> = vec![
        ("Toggle", json!({"kind": "Toggle", "panel": "P", "key": "k1", "value": true})),
        ("Slider", json!({"kind": "Slider", "panel": "P", "key": "fontSize", "value": 7})),
        ("Combo", json!({"kind": "Combo", "panel": "P", "key": "style", "value": "B"})),
        (
            "ColorPicked",
            json!({"kind": "ColorPicked", "panel": "P", "key": "fontWarn", "value": [255, 36, 0, 128]}),
        ),
        ("Save", json!({"kind": "Save"})),
        ("StartGame", json!({"kind": "StartGame"})),
        ("EndGame", json!({"kind": "EndGame"})),
        ("RefreshPreviews", json!({"kind": "RefreshPreviews"})),
        ("ButtonAction", json!({"kind": "ButtonAction", "action": "factoryReset"})),
        ("ConfirmPending", json!({"kind": "ConfirmPending"})),
        ("CancelPending", json!({"kind": "CancelPending"})),
    ];
    let actual = cases
        .iter()
        .map(|(name, v)| {
            let m: FormMessageDto = serde_json::from_value(v.clone()).unwrap();
            format!("{name}: {m:?}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        Toggle: Toggle { panel: "P", key: "k1", value: true }
        Slider: Slider { panel: "P", key: "fontSize", value: 7 }
        Combo: Combo { panel: "P", key: "style", value: "B" }
        ColorPicked: ColorPicked { panel: "P", key: "fontWarn", value: [255, 36, 0, 128] }
        Save: Save
        StartGame: StartGame
        EndGame: EndGame
        RefreshPreviews: RefreshPreviews
        ButtonAction: ButtonAction { action: "factoryReset" }
        ConfirmPending: ConfirmPending
        CancelPending: CancelPending"#]]
    .assert_eq(&actual);

    // 未知 kind / 缺字段 → 反序列化失败 (前端拼错不得静默)
    let bad: Result<FormMessageDto, _> = serde_json::from_value(json!({"kind": "Nope"}));
    assert!(bad.is_err());
}

/// web 窗口域语义标: Win → -1/0/1 整数; 拐点族 → 小写字符串; 对比行 camelCase
#[test]
fn win与拐点族序列化() {
    let wins = [Win::Left, Win::Draw, Win::Right]
        .iter()
        .map(|w| serde_json::to_value(w).unwrap().to_string())
        .collect::<Vec<_>>()
        .join(",");
    expect!["-1,0,1"].assert_eq(&wins);

    let kinds = [InflectionKind::Peak, InflectionKind::Valley, InflectionKind::Kink]
        .iter()
        .map(|k| serde_json::to_value(k).unwrap().to_string())
        .collect::<Vec<_>>()
        .join(",");
    expect!["\"peak\",\"valley\",\"kink\""].assert_eq(&kinds);

    // 拐点标注 (前端 SVG 消费面): kind/label/altitudeM/power
    let p = InflectionPointDto {
        kind: InflectionKind::Valley,
        label: "1→2档".into(),
        altitude_m: 1250,
        power: 312.5,
    };
    expect![[r#"{"kind":"valley","label":"1→2档","altitudeM":1250,"power":312.5}"#]]
    .assert_eq(&serde_json::to_value(&p).unwrap().to_string());

    // 对比行: isHeader/value0/value1/win/symbol 全 camelCase
    let row = ComparisonRowDto {
        is_header: false,
        text: "空重(kg)".into(),
        value0: Some("4644.0".into()),
        value1: Some("5000.0".into()),
        win: Win::Left,
        symbol: "▶".into(),
    };
    expect![[r#"{"isHeader":false,"text":"空重(kg)","value0":"4644.0","value1":"5000.0","win":-1,"symbol":"▶"}"#]]
    .assert_eq(&serde_json::to_value(&row).unwrap().to_string());
}
