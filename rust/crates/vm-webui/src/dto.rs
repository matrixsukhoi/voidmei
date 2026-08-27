//! cfg 树 DTO (D9 阶段②): `GroupConfig/RowConfig` (vm-core, 无 serde) → 前端 JSON。
//! vm-core 零改动 — 映射集中在本模块。`r#type` 序列化为 "type"。

use serde::{Deserialize, Serialize};

use vm_core::config_loader::{ConfigValue, GroupConfig, RowConfig};

/// 一个设置 panel (= Java WebTabbedPane 一页)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelDto {
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub alpha: i32,
    pub hotkey: i32,
    pub visible: bool,
    pub font_name: Option<String>,
    pub font_size: i32,
    pub columns: i32,
    pub panel_columns: i32,
    pub switch_key: Option<String>,
    pub rows: Vec<RowDto>,
}

/// 一行配置 (15 种 row type; children = HEADER 嵌套组)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowDto {
    pub label: String,
    #[serde(rename = "type")]
    pub row_type: String,
    /// 绑定键 (无 :target 的行为 None — 前端以 label 为键)
    pub property: Option<String>,
    pub value: Option<serde_json::Value>,
    pub default_value: Option<serde_json::Value>,
    pub unit: String,
    pub format: String,
    pub desc: Option<String>,
    pub desc_img: Option<String>,
    pub min_val: i32,
    pub max_val: i32,
    pub group_columns: i32,
    pub children: Vec<RowDto>,
}

impl From<&GroupConfig> for PanelDto {
    fn from(g: &GroupConfig) -> Self {
        PanelDto {
            title: g.title.clone(),
            x: g.x,
            y: g.y,
            alpha: g.alpha,
            hotkey: g.hotkey,
            visible: g.visible,
            font_name: g.font_name.clone(),
            font_size: g.font_size,
            columns: g.columns,
            panel_columns: g.panel_columns,
            switch_key: g.switch_key.clone(),
            rows: g.rows.iter().map(Into::into).collect(),
        }
    }
}

impl From<&RowConfig> for RowDto {
    fn from(r: &RowConfig) -> Self {
        RowDto {
            label: r.label.clone(),
            row_type: r.r#type.clone(),
            property: r.property.clone(),
            value: r.value.as_ref().map(config_value_to_json),
            default_value: r.default_value.as_ref().map(config_value_to_json),
            unit: r.unit.clone(),
            format: r.format.clone(),
            desc: r.desc.clone(),
            desc_img: r.desc_img.clone(),
            min_val: r.min_val,
            max_val: r.max_val,
            group_columns: r.group_columns,
            children: r.children.iter().map(Into::into).collect(),
        }
    }
}

/// ConfigValue → JSON (Bool→bool, Int→number, Double→number, Str→string)
fn config_value_to_json(v: &ConfigValue) -> serde_json::Value {
    match v {
        ConfigValue::Bool(b) => serde_json::Value::Bool(*b),
        ConfigValue::Int(i) => serde_json::json!(i),
        ConfigValue::Double(d) => serde_json::json!(d),
        ConfigValue::Str(s) => serde_json::Value::String(s.clone()),
    }
}

/// 前端表单消息 (与 vm-ui main_form::Message 一一对应; 转换在 vm-app dispatcher —
/// vm-webui 不依赖 vm-ui, 组装层单点粘合)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind")]
pub enum FormMessageDto {
    /// 开关翻转 (value = 显示值, SWITCH_INV 落库取反)
    Toggle { panel: String, key: String, value: bool },
    /// 滑条值 (拖拽期实时, 不落盘)
    Slider { panel: String, key: String, value: i32 },
    /// 下拉选中
    Combo { panel: String, key: String, value: String },
    /// 颜色 (RGBA 字节; 落库 = 主键十进制串 + legacy 分键)
    ColorPicked { panel: String, key: String, value: [u8; 4] },
    Save,
    StartGame,
    EndGame,
    RefreshPreviews,
    ButtonAction { action: String },
    ConfirmPending,
    CancelPending,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row() -> RowConfig {
        RowConfig {
            label: "显示hud数据".into(),
            target_name: None,
            formula: None,
            format: String::new(),
            unit: "px".into(),
            value: Some(ConfigValue::Bool(true)),
            default_value: Some(ConfigValue::Int(50)),
            fg_color: None,
            desc: Some("帮助".into()),
            desc_img: Some("img.png".into()),
            preview_value: None,
            hide_when_zero: false,
            precision: 2,
            unit_source: None,
            precision_source: None,
            visible_when: None,
            na_when: None,
            r#type: "SWITCH".into(),
            property: Some("drawHUDtext".into()),
            min_val: 1,
            max_val: 100,
            group_columns: 0,
            children: Vec::new(),
        }
    }

    #[test]
    fn row_dto_字段映射_含_type_改名() {
        let dto = RowDto::from(&sample_row());
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["type"], "SWITCH"); // r#type → type (camelCase 化)
        assert_eq!(json["property"], "drawHUDtext");
        assert_eq!(json["value"], true);
        assert_eq!(json["defaultValue"], 50);
        assert_eq!(json["unit"], "px");
        assert_eq!(json["desc"], "帮助");
        assert_eq!(json["descImg"], "img.png");
        assert_eq!(json["minVal"], 1);
        assert_eq!(json["groupColumns"], 0);
    }

    #[test]
    fn panel_dto_树递归含子行() {
        let mut parent = sample_row();
        parent.r#type = "HEADER".into();
        parent.property = None;
        parent.children = vec![sample_row()];
        let g = GroupConfig {
            title: "hud面板设置".into(),
            rows: vec![parent],
            ..GroupConfig::new("tmp".into())
        };
        let dto = PanelDto::from(&g);
        assert_eq!(dto.title, "hud面板设置");
        assert_eq!(dto.rows.len(), 1);
        assert_eq!(dto.rows[0].children.len(), 1);
        assert_eq!(dto.rows[0].children[0].row_type, "SWITCH");
    }

    #[test]
    fn form_message_dto_反序列化() {
        let m: FormMessageDto =
            serde_json::from_str(r#"{"kind":"Toggle","panel":"p","key":"k","value":true}"#)
                .unwrap();
        assert!(matches!(m, FormMessageDto::Toggle { value: true, .. }));
        let u: FormMessageDto = serde_json::from_str(r#"{"kind":"Save"}"#).unwrap();
        assert!(matches!(u, FormMessageDto::Save));
        let c: FormMessageDto =
            serde_json::from_str(r#"{"kind":"ColorPicked","panel":"p","key":"k","value":[1,2,3,4]}"#)
                .unwrap();
        assert!(matches!(c, FormMessageDto::ColorPicked { value: [1, 2, 3, 4], .. }));
    }
}
