//! JSON 配置模型 (ui_layout.cfg S-expr 体系的接替者)。
//!
//! 结构: 出厂默认 `factory_default.json` (include_str! 内嵌, 代码版本即模板版本)
//! ⊕ 用户 delta 文件 `voidmei_config.json` (只存用户改过的量) → 运行树。
//! 合成/落盘见 `json_store`。
//!
//! 字段语义与旧 S-expr 版对齐, 差异 (去 Java 坏味道):
//! - `formula` 字段删除 — 旧版是 `property` 的镜像 (反射路径遗留);
//! - `hotkey` 组字段删除 — 全库死字段 (panel 级热键从未进 cfg);
//! - `hide_when_zero` 删除 — cfg 从未使用;
//! - `format` 与 `source` 拆分 — 旧版把 combo 下拉源塞进 format 字段复用;
//! - `visible_when`/`na_when` 由 S-expr 树 (`Rc<SExp>`) 改为中缀字符串
//!   (`"value > 0 && !isJetEngine"`), 编译见 ui_support::row_def。

use serde::{Deserialize, Serialize};

// =====================================================================
// 值类型
// =====================================================================

/// 行值 (JSON: bool / number / string 的自然映射;
/// 整数与非整数 number 分别落 Int/Double, round-trip 保型)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    Bool(bool),
    Int(i32),
    Double(f64),
    Str(String),
}

impl ConfigValue {
    /// 显示/配置交换串 (Java String.valueOf 语义: Double 走 Double.toString 位形)
    pub fn as_config_string(&self) -> String {
        match self {
            ConfigValue::Bool(b) => b.to_string(),
            ConfigValue::Int(i) => i.to_string(),
            ConfigValue::Double(d) => crate::base::java_compat::java_double_to_string(*d),
            ConfigValue::Str(s) => s.clone(),
        }
    }
}

// =====================================================================
// 行
// =====================================================================

/// 一行配置 (类型集合见 ui_layout 时代的 15 种 row type; children = HEADER 嵌套)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RowConfig {
    pub label: String,
    /// 大写类型名 ("SWITCH"/"SWITCH_INV"/"DATA"/"SLIDER"/"COMBO"/"COLOR"/
    /// "HOTKEY"/"BUTTON"/"INFO"/"VOICE"/"VOICE_GLOBAL"/"FMLIST"/"INPUT"…)
    #[serde(rename = "type")]
    pub r#type: String,
    /// 绑定键: 组字段名 (fontSize/fontName/panelColumns…) | 配置键 (crosshairSwitch…) |
    /// 数据短名 (data 行)。无键行 (info) 为 None — 消费方以 label 定位。
    pub property: Option<String>,
    /// data 行的显示名 (缺省用 label)
    pub target_name: Option<String>,
    pub value: Option<ConfigValue>,
    pub default_value: Option<ConfigValue>,
    pub unit: String,
    /// 小数位 (data 行)
    pub precision: i32,
    /// 输出格式 ("TIME_MM_SS" 等; 缺省 "%s")
    pub format: String,
    /// 下拉选项源 (combo/fmlist 行: "_FONTS_"/"_CROSSHAIRS_"/逗号字面量/目录)
    pub source: Option<String>,
    /// slider 区间
    pub min_val: i32,
    pub max_val: i32,
    /// HEADER 行的组内列数
    pub group_columns: i32,
    pub desc: Option<String>,
    pub desc_img: Option<String>,
    /// preview 模式的静态值 (data 行)
    pub preview_value: Option<String>,
    pub fg_color: Option<String>,
    /// 动态单位/精度源 (公式名, 全表仅进气压一条使用)
    pub unit_source: Option<String>,
    pub precision_source: Option<String>,
    /// 显示条件 (中缀: "value > 0 && !isJetEngine"; `value` 为行当前值)
    pub visible_when: Option<String>,
    /// NA 条件 (满足时显示 "-" 而非隐藏)
    pub na_when: Option<String>,
    pub children: Vec<RowConfig>,
}

impl Default for RowConfig {
    fn default() -> Self {
        RowConfig {
            label: String::new(),
            r#type: "DATA".to_string(),
            property: None,
            target_name: None,
            value: None,
            default_value: None,
            unit: String::new(),
            precision: 0,
            format: "%s".to_string(),
            source: None,
            min_val: 0,
            max_val: 100,
            group_columns: 0,
            desc: None,
            desc_img: None,
            preview_value: None,
            fg_color: None,
            unit_source: None,
            precision_source: None,
            visible_when: None,
            na_when: None,
            children: Vec::new(),
        }
    }
}

impl RowConfig {
    pub fn get_int(&self) -> i32 {
        match &self.value {
            Some(ConfigValue::Int(i)) => *i,
            Some(ConfigValue::Double(d)) => *d as i32,
            // 值缺失/越型 → 0 (旧版 null 走 Java NPE-catch 兜底的等价收敛)
            _ => self
                .value
                .as_ref()
                .and_then(|v| v.as_config_string().parse::<i32>().ok())
                .unwrap_or(0),
        }
    }

    /// 值缺失 → false (旧版 panic 的安全化: cfg 为用户可编辑输入)
    pub fn get_bool(&self) -> bool {
        match &self.value {
            Some(ConfigValue::Bool(b)) => *b,
            Some(v) => crate::base::java_compat::java_parse_boolean(&v.as_config_string()),
            None => false,
        }
    }

    pub fn get_str(&self) -> String {
        match &self.value {
            None => "null".to_string(),
            Some(v) => v.as_config_string(),
        }
    }
}

// =====================================================================
// Panel
// =====================================================================

/// 一个设置 panel (= 旧 GroupConfig; HUD 页面化后仍是设置面板的容器)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GroupConfig {
    pub title: String,
    /// 窗口归一化位置 (0..1)
    pub x: f64,
    pub y: f64,
    pub alpha: i32,
    /// switch_key 的开态 (panel 可见性)
    pub visible: bool,
    pub font_name: Option<String>,
    /// 字号微调 (-6..+20)
    pub font_size: i32,
    /// overlay 列数
    pub columns: i32,
    /// 设置面板列数
    pub panel_columns: i32,
    /// panel 可见性开关键 ("flightInfoSwitch" 等)
    pub switch_key: Option<String>,
    pub rows: Vec<RowConfig>,
}

impl Default for GroupConfig {
    fn default() -> Self {
        GroupConfig {
            title: String::new(),
            x: 0.1,
            y: 0.1,
            alpha: 150,
            visible: false,
            font_name: None,
            font_size: 0,
            columns: 2,
            panel_columns: 2,
            switch_key: None,
            rows: Vec::new(),
        }
    }
}

impl GroupConfig {
    pub fn new(title: String) -> GroupConfig {
        GroupConfig {
            title,
            ..GroupConfig::default()
        }
    }
}

// =====================================================================
// HUD 页面 (画布即窗口; 出厂页与用户页同构)
// =====================================================================

/// 页面级字体语义 (坐标 = line_height 倍数的换算基)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PageFont {
    pub family: String,
    /// 字号微调 (cfg fontSize 行同源)
    pub size_add: i32,
    /// 整页缩放源键 ("crosshairScale"; 缺省 100 = 不缩放)
    pub scale_source: String,
}

/// 一个组件实例 (页面 components 数组序 = 拓扑建树序 = z 序)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ComponentDoc {
    /// 页内唯一 id (= 布局节点 id = 链式挂载的父引用键)
    pub id: String,
    /// 注册表类型名 ("core.minihud.row0" / "core.data.field" …)
    #[serde(rename = "type")]
    pub r#type: String,
    /// 坐标 (line_height 倍数; 相对父锚点偏移)
    pub pos: [f64; 2],
    /// [self, parent] 锚点名 ("TopLeft"…; 缺省 ["TopLeft","TopLeft"])
    pub anchor: [String; 2],
    /// 父组件 id (None = 根, 挂页面虚拟画布)
    pub parent: Option<String>,
    /// 显示条件 (中缀; 求值变量 = 配置 bool 快照 → 遥测短名)
    pub visible_when: Option<String>,
    /// 硬开关 (false = 组件不建)
    pub enabled: bool,
    /// 类型静态属性 (注册表 props_schema 校验)
    pub props: serde_json::Value,
}

impl Default for ComponentDoc {
    fn default() -> Self {
        ComponentDoc {
            id: String::new(),
            r#type: String::new(),
            pos: [0.0, 0.0],
            anchor: [String::new(), String::new()],
            parent: None,
            visible_when: None,
            enabled: true,
            props: serde_json::Value::Null,
        }
    }
}

/// 一个 HUD 页面 (= 一个 overlay 窗口)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PageDoc {
    /// 稳定 id (= host entry id = 位置存档键)
    pub id: String,
    pub name: String,
    /// 激活开关键 (None = 恒显调试页)
    pub switch_key: Option<String>,
    /// host 条目键 (默认 = switchKey; 推力曲线 = "thrustdFS" —
    /// 激活策略 config(switchKey)∧jetOnly 经 strategy_extra 表达)
    pub entry_key: Option<String>,
    /// 激活策略扩展 ("jetOnly" 等; 渲染线程 strategy_for 特判)
    pub strategy_extra: Option<String>,
    /// 归一化窗口位置 [x, y] (拖拽存档写回)
    pub pos: Option<[f64; 2]>,
    /// 包围盒 padding (窗口 = 内容包围盒 + 2×padding)
    pub padding: i32,
    /// 画布语义 (None/"free" = 4096 自由画布; "minihud" = ctx.width×2 派生画布 —
    /// crosshair 的 MiddleRight 右半区锚定依赖此语义, 真窗由 minihud 编排器承载)
    pub canvas: Option<String>,
    pub font: PageFont,
    /// 出厂页内容版本戳 (升级提示比对; 用户页恒 0)
    pub content_version: u32,
    pub components: Vec<ComponentDoc>,
}

#[allow(clippy::derivable_impls)] // version=1 语义
impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            version: 1,
            panels: Vec::new(),
            pages: Vec::new(),
        }
    }
}

impl Default for PageDoc {
    fn default() -> Self {
        PageDoc {
            id: String::new(),
            name: String::new(),
            switch_key: None,
            entry_key: None,
            strategy_extra: None,
            pos: None,
            padding: 45,
            canvas: None,
            font: PageFont::default(),
            content_version: 0,
            components: Vec::new(),
        }
    }
}

// =====================================================================
// 出厂文件顶层
// =====================================================================

/// factory_default.json 顶层 (用户 delta 见 json_store::UserDelta)
/// (Default 手写: version 缺省 1 非零值, 不可派生)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub version: u32,
    pub panels: Vec<GroupConfig>,
    /// HUD 出厂页面 (W2+: 逐 overlay 复刻; 旧 overlay 迁完前二者并存)
    pub pages: Vec<PageDoc>,
}


