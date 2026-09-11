//! cfg 树 DTO (D9 阶段② → JSON 化 Phase 1): 设置面板树由 kernel 的
//! serde 模型 (`json_model::GroupConfig/RowConfig`, camelCase) 直接序列化 —
//! 本模块的 PanelDto/RowDto From 映射层已退役。保留: web 窗口域 DTO
//! (对比/功率曲线) 与 FormMessageDto。

use serde::{Deserialize, Serialize};

pub use kernel::config::json_model::{GroupConfig as PanelDto, RowConfig as RowDto};

// =====================================================================
// P6 web 窗口域 DTO: 对比 / 功率曲线 两窗口的数据命令返回面
// (commands_comparison.rs 构造, 前端 AntD Table + SVG 消费)。
// Java 侧对应 JDialog/WebFrame 窗口类的字段区; 颜色/像素字段 (Color/坐标) 是
// 展示域, 换语义标 (kind/color 字符串) 不进数据面。
// =====================================================================

/// 对比行胜负 (Java win 的 -1/0/1): Left=左胜(v0) Draw=平 Right=右胜(v1)。
/// 序列化保持整数 (经 `into = i32`), 前端按数值消费
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(into = "i32")]
pub enum Win {
    Left,
    Draw,
    Right,
}

impl From<Win> for i32 {
    fn from(w: Win) -> i32 {
        match w {
            Win::Left => -1,
            Win::Draw => 0,
            Win::Right => 1,
        }
    }
}

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
    /// 胜负 (Java win): Left=左胜(v0) Draw=平 Right=右胜(v1)
    pub win: Win,
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

/// 拐点族 (Java 以 marker Color 区分三族, web 侧换语义标着色)
/// 序列化输出小写字符串, 与前端着色映射一致
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum InflectionKind {
    /// 峰/临界高度 (金)
    #[serde(rename = "peak")]
    Peak,
    /// 级间过渡谷 (蓝)
    #[serde(rename = "valley")]
    Valley,
    /// 斜率拐点 (紫)
    #[serde(rename = "kink")]
    Kink,
}

/// 功率曲线拐点标注 (Java PowerCurveWindow.InflectionPoint; Color 换语义标)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InflectionPointDto {
    /// 拐点族 (序列化 "peak"/"valley"/"kink") — 三族语义见 [`InflectionKind`]
    pub kind: InflectionKind,
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

/// 前端表单消息 (与 ui main_form::Message 一一对应; 转换在 voidmei dispatcher —
/// webui 不依赖 ui, 组装层单点粘合)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind")]
pub enum FormMessageDto {
    /// 开关翻转 (value = 显示值, SWITCH_INV 落库取反)
    Toggle {
        panel: String,
        key: String,
        value: bool,
    },
    /// 滑条值 (拖拽期实时, 不落盘)
    Slider {
        panel: String,
        key: String,
        value: i32,
    },
    /// 下拉选中
    Combo {
        panel: String,
        key: String,
        value: String,
    },
    /// 颜色 (RGBA 字节; 落库 = 主键十进制串 + legacy 分键)
    ColorPicked {
        panel: String,
        key: String,
        value: [u8; 4],
    },
    Save,
    StartGame,
    EndGame,
    RefreshPreviews,
    ButtonAction {
        action: String,
    },
    ConfirmPending,
    CancelPending,
}

