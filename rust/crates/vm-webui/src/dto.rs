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

// =====================================================================
// P6 web 窗口域 DTO: 对比 / 功率曲线 / 飞行记录曲线 三窗口的数据命令返回面
// (commands_windows.rs 构造, 前端 AntD Table + SVG 消费)。
// Java 侧对应 JDialog/WebFrame 窗口类的字段区; 颜色/像素字段 (Color/坐标) 是
// 展示域, 换语义标 (kind/color 字符串) 不进数据面。
// =====================================================================

/// 对比窗口一行 (Java CompactComparisonWindow.DisplayItem + addComparisonRow
/// 的展示面: 行类型/属性名/两侧值/胜负)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonRowDto {
    /// true = 分节标题行 (Java "------" 行), 此时 text = 标题文本
    pub is_header: bool,
    /// 行文本 (标题或属性名)
    pub text: String,
    /// FM0 侧值 (缺键时前端显示 "-" 已由后端补齐: Java `v0 == null ? "-" : v0`)
    pub value0: Option<String>,
    /// FM1 侧值 (单机模式恒 None)
    pub value1: Option<String>,
    /// 胜负: -1=左胜(v0) 0=平 1=右胜(v1) (Java win)
    pub win: i32,
    /// 符号列: "-" / "▶" / "◀" (Java addComparisonRow)
    pub symbol: String,
}

/// 对比窗口全量数据 (displayStructure + dataMap0/1 + COPY 文本)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonDataDto {
    pub fm0_name: String,
    /// None = 单机数据视图 (Java fm1Name == null || isEmpty)
    pub fm1_name: Option<String>,
    pub single_mode: bool,
    /// 窗口标题 (Java: "Aircraft Data: x" / "Comparison: x vs y")
    pub title: String,
    pub rows: Vec<ComparisonRowDto>,
    /// COPY 按钮文本 (Java buildCopyText, 含胜负方名)
    pub copy_text: String,
}

/// 功率曲线拐点标注 (Java PowerCurveWindow.InflectionPoint; Color 换语义标)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InflectionPointDto {
    /// "peak" (峰/临界高度, 金) | "valley" (级间过渡, 蓝) | "kink" (斜率拐点, 紫)
    /// — Java 以 marker Color 区分三族, web 侧换语义标着色
    pub kind: String,
    /// 标注文本 (Java label: "1档" / "1→2档" / "Kink")
    pub label: String,
    pub altitude_m: i32,
    pub power: f64,
}

/// 单条功率曲线 (Java PowerCurveWindow.CurveData 的数据面)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerCurveDto {
    pub fm_name: String,
    /// errorMessage == null && powerCurve != null (Java isValid)
    pub valid: bool,
    /// 高度格点功率 (hp); 索引 i ↔ 高度 i × altStep
    pub power_curve: Vec<f64>,
    pub alt_step: i32,
    pub max_display_alt: i32,
    pub max_power: f64,
    pub min_power: f64,
    pub peak_altitude: i32,
    pub inflection_points: Vec<InflectionPointDto>,
    /// None = 成功; Some = 错误信息 (Java errorMessage)
    pub error_message: Option<String>,
}

/// 功率曲线窗口全量数据 (loadPowerCurves + calculateDisplayRange + 错误汇总)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerCurveDataDto {
    pub fm0_name: String,
    /// None = 单曲线模式 (空或 == fm0 都归一, Java 构造器裁决)
    pub fm1_name: Option<String>,
    /// Java isDualMode (fm1Name != null; 与曲线有效无关)
    pub dual_mode: bool,
    pub speed_kmh: i32,
    pub wep_mode: bool,
    pub curve0: PowerCurveDto,
    pub curve1: Option<PowerCurveDto>,
    /// 合并显示域上限 (ceil 到百 hp; Java calculateDisplayRange)
    pub display_max_power: f64,
    /// 合并显示域下限 (floor 到百 hp)
    pub display_min_power: f64,
    /// 双失败合并/单侧失败提示 (Java buildErrorMessage; 全成功 None)
    pub error_message: Option<String>,
}

/// 飞行记录曲线页的一条数据序列 (drawPoint/drawCoordinates 的点列)
///
/// 前端连线契约 (阶段④ SVG/canvas 任务书): DrawFrame 页 0-3 (index 0/1/2/3)
/// 的连线带毛刺抑制 — Java drawCoordinates 连线循环 (DrawFrame.java:392-395)
/// `if (Math.abs(X[ii] - X[ii+1]) > 100) X[ii+1] = X[ii+2];` 对相邻点跳变
/// >100 的段落做替换后连线。该规则属绘图数据变换, 未随本 DTO 输出 (快照不可
/// 变 — Java 的就地改数组 + 重绘跨页累积形态不适用); 前端连线时按只读规则
/// 逐段复刻判断 (不回写 x), 绘点不受此规则影响。页 5/6/7 无此过滤。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlightSeriesDto {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    /// 语义色标 "black"/"red"/"blue" (Java Color 的 web 对位, 前端映射)
    pub color: String,
    /// 图例文本 (无图例页 None)
    pub legend: Option<String>,
}

/// 飞行记录曲线窗口一页 (Java pixIndex 页; 页号 4 无页面)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlightPageDto {
    /// Java pixIndex (0/1/2/3/5/6/7)
    pub index: i32,
    pub title: String,
    pub x_name: String,
    pub y_name: String,
    pub x_unit: String,
    pub y_unit: String,
    pub series: Vec<FlightSeriesDto>,
    /// 页内占位提示 (如 page6 FM 未加载时的 "FM数据未加载")
    pub error_text: Option<String>,
}

/// 飞行记录曲线窗口全量数据 (DrawFrame.init(c, A) 后 paintAction 读取的子集)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlightRecordDataDto {
    /// 机型名 (Java fA.type; 缺失 None)
    pub plane_type: Option<String>,
    /// Java fA.engineType (0=活塞 1=喷气 2=涡桨)
    pub engine_type: i32,
    pub initalt_stage: i32,
    pub curalt_stage: i32,
    pub pages: Vec<FlightPageDto>,
    /// getdata(fA.type) 的 FM 加载成败 (page6 守卫)
    pub fm_loaded: bool,
    /// 窗口标题 (Java: fA.type + Lang.dFTitleHZ)
    pub window_title: String,
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
