//! JSON 配置存储: 出厂默认 (内嵌) ⊕ 用户 delta 的合成/持久化。
//!
//! 模式对标 Emacs defcustom / VSCode settings — 用户文件只存改过的量
//! (delta), 未提及的部分永远跟随出厂新默认 (升级时新组件/新面板自动出现);
//! 弃全量快照模式 (首次保存后冻结一切出厂值, 遮蔽后续版本的默认演化)。
//!
//! 文件: `voidmei_config.json` (工作区根)。损坏文件改名 `.corrupt` 隔离 +
//! 回退出厂; 落盘走 `.tmp` → rename 原子替换, 滚动保留一份 `.bak`。

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::base::logger;
use crate::config::json_model::{AppConfig, ConfigValue, GroupConfig, RowConfig};

/// 出厂默认 (构建期内嵌 — 代码版本即模板版本)
const FACTORY_DEFAULT_JSON: &str = include_str!("factory_default.json");

/// 用户 delta 文件路径 (工作区根, 对齐旧 ui_layout.user.cfg 的位置习惯)
pub const USER_CONFIG_PATH: &str = "./voidmei_config.json";

// =====================================================================
// Delta 模型
// =====================================================================

/// 用户 delta: 只存与出厂默认的差异。
/// 行值以 (panel 标题, 行绑定键) 定位; 组字段以字段名定位; 页面区随
/// Phase 2 引入 (userPages / ownedFactoryPages)。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserDelta {
    pub version: u32,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub panels: HashMap<String, PanelDelta>,
}

/// 单个 panel 的 delta
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PanelDelta {
    /// switch_key 开态 (panel 可见性)
    pub visible: Option<bool>,
    /// 归一化窗口位置 [x, y]
    pub pos: Option<[f64; 2]>,
    /// 组字段名 → 值 (fontSize/fontName/panelColumns…)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub fields: HashMap<String, ConfigValue>,
    /// 行绑定键 → 行值 (panel 内所有同名 target 行一起覆盖,
    /// 对齐 set_config 的全局更新语义)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub rows: HashMap<String, ConfigValue>,
}

// =====================================================================
// 出厂装载
// =====================================================================

/// 解析内嵌出厂默认 (损坏 = 编译期资产错误, 直接 panic 暴露)
pub fn factory_default() -> AppConfig {
    serde_json::from_str(FACTORY_DEFAULT_JSON).expect("factory_default.json 损坏 (编译期资产)")
}

// =====================================================================
// 合成
// =====================================================================

/// 基树 ⊕ delta → 运行树 (panels 深拷贝后按 delta 覆盖)。
/// delta 中基树不存在的 panel/键 → warn 丢弃 (模板演化或手编残留)。
pub fn synthesize(base: &[GroupConfig], delta: &UserDelta) -> Vec<GroupConfig> {
    let mut panels = base.to_vec();
    let titles: Vec<String> = panels.iter().map(|p| p.title.clone()).collect();
    for panel in panels.iter_mut() {
        if let Some(pd) = delta.panels.get(&panel.title) {
            apply_panel_delta(panel, pd);
        }
    }
    for title in delta.panels.keys() {
        if !titles.iter().any(|t| t == title) {
            logger::warn("JsonStore", &format!("delta 引用不存在的 panel: {title} (已丢弃)"));
        }
    }
    panels
}

fn apply_panel_delta(panel: &mut GroupConfig, pd: &PanelDelta) {
    if let Some(v) = pd.visible {
        panel.visible = v;
    }
    if let Some([x, y]) = pd.pos {
        panel.x = x;
        panel.y = y;
    }
    for (field, value) in &pd.fields {
        set_panel_field(panel, field, value.clone());
    }
    for (key, value) in &pd.rows {
        apply_row_value(&mut panel.rows, key, value.clone());
    }
}

/// 组字段名 → GroupConfig 字段 (PropertyBinder 反射的显式接替者)。
/// 返回 false = 非组字段名 (调用方回落行值通道)。
pub fn set_panel_field(panel: &mut GroupConfig, field: &str, value: ConfigValue) -> bool {
    match field {
        "title" => as_str(&value).map(|s| panel.title = s).is_some(),
        "x" => as_f64(&value).map(|v| panel.x = v).is_some(),
        "y" => as_f64(&value).map(|v| panel.y = v).is_some(),
        "alpha" => as_i32(&value).map(|v| panel.alpha = v).is_some(),
        "visible" => as_bool_val(&value).map(|v| panel.visible = v).is_some(),
        "fontName" => {
            panel.font_name = as_str(&value);
            true
        }
        "fontSize" => as_i32(&value).map(|v| panel.font_size = v).is_some(),
        "columns" => as_i32(&value).map(|v| panel.columns = v).is_some(),
        "panelColumns" => as_i32(&value).map(|v| panel.panel_columns = v).is_some(),
        "switchKey" => {
            panel.switch_key = as_str(&value);
            true
        }
        _ => false,
    }
}

/// 组字段名判定 (写链二分: 组字段 vs 行值)
pub fn is_panel_field(field: &str) -> bool {
    matches!(
        field,
        "title" | "x" | "y" | "alpha" | "visible" | "fontName" | "fontSize" | "columns"
            | "panelColumns" | "switchKey"
    )
}

fn as_str(v: &ConfigValue) -> Option<String> {
    match v {
        ConfigValue::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn as_i32(v: &ConfigValue) -> Option<i32> {
    match v {
        ConfigValue::Int(i) => Some(*i),
        ConfigValue::Double(d) => Some(*d as i32),
        _ => None,
    }
}

fn as_f64(v: &ConfigValue) -> Option<f64> {
    match v {
        ConfigValue::Int(i) => Some(f64::from(*i)),
        ConfigValue::Double(d) => Some(*d),
        _ => None,
    }
}

fn as_bool_val(v: &ConfigValue) -> Option<bool> {
    match v {
        ConfigValue::Bool(b) => Some(*b),
        _ => None,
    }
}

/// 行值覆盖: panel 内所有绑定键命中行 (含嵌套 children) 一起写 —
/// 对齐 set_config 的全局更新语义。
fn apply_row_value(rows: &mut [RowConfig], key: &str, value: ConfigValue) -> bool {
    let mut hit = false;
    for r in rows.iter_mut() {
        if r.property.as_deref() == Some(key) {
            r.value = Some(value.clone());
            hit = true;
        }
        if !r.children.is_empty() && apply_row_value(&mut r.children, key, value.clone()) {
            hit = true;
        }
    }
    hit
}

// =====================================================================
// 持久化
// =====================================================================

/// 读用户 delta; 文件不存在 → 空 delta (首次运行);
/// 损坏 → 改名 `.corrupt` 隔离 + 空 delta 回退出厂。
pub fn load_delta(path: &str) -> UserDelta {
    if !Path::new(path).exists() {
        return UserDelta::default();
    }
    match std::fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(delta) => delta,
            Err(e) => {
                let corrupt = format!("{path}.corrupt");
                let _ = std::fs::rename(path, &corrupt);
                logger::warn(
                    "JsonStore",
                    &format!("用户配置损坏 ({e}), 已隔离到 {corrupt} 并回退出厂默认"),
                );
                UserDelta::default()
            }
        },
        Err(e) => {
            logger::warn("JsonStore", &format!("用户配置读取失败 ({e}), 回退出厂默认"));
            UserDelta::default()
        }
    }
}

/// delta 落盘: `.bak` 滚动 → `.tmp` 写 → rename 原子替换。
pub fn save_delta(path: &str, delta: &UserDelta) -> Result<(), String> {
    let bak = format!("{path}.bak");
    if Path::new(path).exists() {
        let _ = std::fs::rename(path, bak);
    }
    let tmp = format!("{path}.tmp");
    let json = serde_json::to_string_pretty(delta).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

/// 读外部 delta 文件 (导入配置用); 损坏 → Err (导入不吞错)。
pub fn read_delta_file(path: &str) -> Result<UserDelta, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读取失败: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("解析失败: {e}"))
}
