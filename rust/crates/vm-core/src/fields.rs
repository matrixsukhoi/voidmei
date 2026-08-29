//! 16 个数据字段定义常量表
//! 值取自 ui_layout.cfg (panel "飞行信息") L114-129, 与 Java FlightInfoConfig.createDefault 等价

/// 字段取值来源 (对应 Java TelemetrySource getter)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldSource {
    Ias,
    Tas,
    Mach,
    Compass,
    Altitude,
    Vario,
    Sep,
    Acceleration,
    RollRate,
    Ny,
    TurnRate,
    TurnRadius,
    AoA,
    AoS,
    /// "getWingSweep * 100"
    WingSweepMul100,
    RadioAltitude,
}

impl FieldSource {
    /// 变量短名 (公式槽/registry 统一名 — 内核取数唯一键, 单名制)
    pub fn target(&self) -> &'static str {
        match self {
            FieldSource::Ias => "ias",
            FieldSource::Tas => "tas",
            FieldSource::Mach => "mach",
            FieldSource::Compass => "compass",
            FieldSource::Altitude => "altitude",
            FieldSource::Vario => "vario",
            FieldSource::Sep => "sep",
            FieldSource::Acceleration => "acceleration",
            FieldSource::RollRate => "roll_rate",
            FieldSource::Ny => "ny",
            FieldSource::TurnRate => "turn_rate",
            FieldSource::TurnRadius => "turn_rds",
            FieldSource::AoA => "aoa",
            FieldSource::AoS => "aos",
            FieldSource::WingSweepMul100 => "wing_sweep",
            FieldSource::RadioAltitude => "radio_altitude",
        }
    }

    /// Java getter 名 (**仅对拍文件边界** — values.txt 跨 Java/Rust 回灌格式,
    /// 内核取数禁用; 曾作为 :target 双名制主键致 live 显示断链, W10 收口)
    pub fn getter(&self) -> &'static str {
        match self {
            FieldSource::Ias => "getIAS",
            FieldSource::Tas => "getTAS",
            FieldSource::Mach => "getMach",
            FieldSource::Compass => "getCompass",
            FieldSource::Altitude => "getAltitude",
            FieldSource::Vario => "getVario",
            FieldSource::Sep => "getSEP",
            FieldSource::Acceleration => "getAcceleration",
            FieldSource::RollRate => "getRollRate",
            FieldSource::Ny => "getNy",
            FieldSource::TurnRate => "getTurnRate",
            FieldSource::TurnRadius => "getTurnRadius",
            FieldSource::AoA => "getAoA",
            FieldSource::AoS => "getAoS",
            FieldSource::WingSweepMul100 => "getWingSweep",
            FieldSource::RadioAltitude => "getRadioAltitude",
        }
    }
}

/// 受限条件求值器: 当前 16 行仅用到三种比较 (value 为字段当前值)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cond {
    /// (!= value N)
    NotEq(f64),
    /// (>= value N)
    Gte(f64),
    /// (> value N)
    Gt(f64),
}

impl Cond {
    pub fn eval(&self, value: f64) -> bool {
        match *self {
            // Java VisibilityExpressionEvaluator 的 = / != 带 0.0001 容差
            Cond::NotEq(n) => (value - n).abs() > 0.0001,
            Cond::Gte(n) => value >= n,
            Cond::Gt(n) => value > n,
        }
    }
}

/// 单个数据行定义
pub struct FieldDef {
    /// 显示名 (全角空格对齐, 原样保留)
    pub label: &'static str,
    pub unit: &'static str,
    /// 预览模式的静态值 (原样字符串, 不经格式化)
    pub preview_value: &'static str,
    pub source: FieldSource,
    /// 小数位 (对应 :precision, 缺省 0)
    pub precision: u8,
    pub visible_when: Option<Cond>,
    pub na_when: Option<Cond>,
}

impl FieldDef {
    /// preview 值恒可见 (Java 端 preview 不订阅事件, visible-when 不求值)
    pub fn preview_text(&self) -> &'static str {
        self.preview_value
    }
}

/// 与 ui_layout.cfg 数据开关组顺序一致
pub const FIELDS: &[FieldDef] = &[
    FieldDef { label: "表  速", unit: "Km/h", preview_value: "500", source: FieldSource::Ias, precision: 0, visible_when: None, na_when: None },
    FieldDef { label: "真空速", unit: "Km/h", preview_value: "550", source: FieldSource::Tas, precision: 0, visible_when: None, na_when: None },
    FieldDef { label: "马赫数", unit: "Ma", preview_value: "0.45", source: FieldSource::Mach, precision: 2, visible_when: None, na_when: None },
    FieldDef { label: "航  向", unit: "Deg", preview_value: "270", source: FieldSource::Compass, precision: 0, visible_when: None, na_when: None },
    FieldDef { label: "高  度", unit: "M", preview_value: "1500", source: FieldSource::Altitude, precision: 0, visible_when: None, na_when: None },
    FieldDef { label: "爬升率", unit: "M/s", preview_value: "10", source: FieldSource::Vario, precision: 1, visible_when: None, na_when: None },
    FieldDef { label: "S E P", unit: "M/s", preview_value: "15", source: FieldSource::Sep, precision: 0, visible_when: None, na_when: None },
    FieldDef { label: "加速度", unit: "M/s²", preview_value: "1.2", source: FieldSource::Acceleration, precision: 1, visible_when: None, na_when: None },
    FieldDef { label: "滚转率", unit: "Deg/s", preview_value: "5.0", source: FieldSource::RollRate, precision: 0, visible_when: None, na_when: None },
    FieldDef { label: "过  载", unit: "G", preview_value: "1.0", source: FieldSource::Ny, precision: 1, visible_when: None, na_when: None },
    FieldDef { label: "转弯率", unit: "Deg/s", preview_value: "2.5", source: FieldSource::TurnRate, precision: 1, visible_when: None, na_when: None },
    FieldDef { label: "转半径", unit: "M", preview_value: "800", source: FieldSource::TurnRadius, precision: 0, visible_when: None, na_when: Some(Cond::Gt(9999.0)) },
    FieldDef { label: "攻  角", unit: "Deg", preview_value: "4.2", source: FieldSource::AoA, precision: 1, visible_when: None, na_when: None },
    FieldDef { label: "侧滑角", unit: "Deg", preview_value: "0.5", source: FieldSource::AoS, precision: 1, visible_when: None, na_when: None },
    FieldDef { label: "可变翼", unit: "%", preview_value: "15", source: FieldSource::WingSweepMul100, precision: 0, visible_when: Some(Cond::NotEq(0.0)), na_when: None },
    FieldDef { label: "测距高", unit: "M", preview_value: "325", source: FieldSource::RadioAltitude, precision: 0, visible_when: Some(Cond::Gte(0.0)), na_when: None },
];
