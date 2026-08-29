//! 数据字段定义常量表 (批3 统一): FlightInfo 16 行 + PowerInfo 19 行共用
//! 一套 FieldDef/Cond/取数/格式化骨架, 渲染层各自保留。
//! 值取自 ui_layout.cfg (panel "飞行信息" L114-129 / "性能数据" L151-170),
//! 与 Java FlightInfoConfig/EngineInfoConfig 的 createDefault 等价。

use crate::formula::registry::FormulaView;

/// 受限条件 (ui_layout.cfg :visible-when / :na-when 表达式的常量表快照)。
/// 两面板曾各自持 Cond(值比较) 与 VisExpr(含谓词), 批3 合一。
#[derive(Debug, PartialEq)]
pub enum Cond {
    // 值比较 (value 为字段当前值); = / != 带 0.0001 容差 (Java 求值器同款)
    NotEq(f64),
    Gte(f64),
    Gt(f64),
    Lt(f64),
    Lte(f64),
    Eq(f64),
    // 环境谓词 (经 var_value 短名取布尔量)
    IsJetEngine,
    IsPistonEngine,
    HasWep,
    HasBooster,
    /// (not e) — 子树为 const 提升的静态引用 (常量表可构造; Box 非 const)
    Not(&'static Cond),
    /// (and a b)
    And(&'static Cond, &'static Cond),
}

impl Cond {
    /// 求值; value 为字段当前值 (对应 Java evaluator.evaluate(value))
    pub fn eval(&self, s: &dyn FormulaView, value: f64) -> bool {
        match self {
            Cond::NotEq(n) => (value - n).abs() >= 0.0001,
            Cond::Gte(n) => value >= *n,
            Cond::Gt(n) => value > *n,
            Cond::Lt(n) => value < *n,
            Cond::Lte(n) => value <= *n,
            Cond::Eq(n) => (value - n).abs() < 0.0001,
            Cond::IsJetEngine => s.var_value("is_jet_engine").unwrap_or(0.0) != 0.0,
            Cond::IsPistonEngine => s.var_value("is_piston_engine").unwrap_or(0.0) != 0.0,
            Cond::HasWep => s.var_value("has_wep").unwrap_or(0.0) != 0.0,
            Cond::HasBooster => s.var_value("has_booster").unwrap_or(0.0) != 0.0,
            Cond::Not(e) => !e.eval(s, value),
            Cond::And(a, b) => a.eval(s, value) && b.eval(s, value),
        }
    }
}

/// 单个数据行定义 (两面板统一形态)
pub struct FieldDef {
    /// 显示名 (全角/双空格对齐, 原样保留)
    pub label: &'static str,
    pub unit: &'static str,
    /// 预览模式的静态值 (原样字符串, 不经格式化)
    pub preview_value: &'static str,
    /// 取数表达式: 变量短名 | 公式名 | "X * N" 乘数 (W10 单名制)
    pub source: &'static str,
    /// Java getter 名 (**仅对拍文件边界** — values.txt 跨 Java/Rust 回灌格式,
    /// 内核取数禁用; 动力表无对拍需求, 填短名占位)
    pub getter: &'static str,
    /// 小数位 (对应 :precision, 缺省 0)
    pub precision: u8,
    /// :format TIME_MM_SS (仅动力表余油时间一条)
    pub time_mm_ss: bool,
    pub visible_when: Option<Cond>,
    pub na_when: Option<Cond>,
    /// :unit-source/:precision-source (cfg 全表仅进气压一条: is_imperial 驱动
    /// 英制 "P/x.x''"+1 位 / 公制 "Ata"+2 位)
    pub imperial_display: bool,
}

impl FieldDef {
    /// preview 值恒可见 (Java 端 preview 不订阅事件, visible-when 不求值)
    pub fn preview_text(&self) -> &'static str {
        self.preview_value
    }
}

/// 静态 Cond 引用 (常量表构造用; And 需静态地址)
const NOT_JET: Cond = Cond::Not(&Cond::IsJetEngine);
const MANIFOLD_VIS: Cond = Cond::And(&Cond::IsPistonEngine, &Cond::NotEq(1.0));
const WEP_TIME_VIS: Cond = Cond::And(&Cond::HasWep, &Cond::Gt(0.0));

/// 与 ui_layout.cfg "飞行信息" 数据开关组顺序一致 (16 项)
pub const FIELDS: &[FieldDef] = &[
    FieldDef { label: "表  速", unit: "Km/h", preview_value: "500", source: "ias", getter: "getIAS", precision: 0, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "真空速", unit: "Km/h", preview_value: "550", source: "tas", getter: "getTAS", precision: 0, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "马赫数", unit: "Ma", preview_value: "0.45", source: "mach", getter: "getMach", precision: 2, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "航  向", unit: "Deg", preview_value: "270", source: "compass", getter: "getCompass", precision: 0, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "高  度", unit: "M", preview_value: "1500", source: "altitude", getter: "getAltitude", precision: 0, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "爬升率", unit: "M/s", preview_value: "10", source: "vario", getter: "getVario", precision: 1, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "S E P", unit: "M/s", preview_value: "15", source: "sep", getter: "getSEP", precision: 0, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "加速度", unit: "M/s²", preview_value: "1.2", source: "acceleration", getter: "getAcceleration", precision: 1, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "滚转率", unit: "Deg/s", preview_value: "5.0", source: "roll_rate", getter: "getRollRate", precision: 0, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "过  载", unit: "G", preview_value: "1.0", source: "ny", getter: "getNy", precision: 1, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "转弯率", unit: "Deg/s", preview_value: "2.5", source: "turn_rate", getter: "getTurnRate", precision: 1, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "转半径", unit: "M", preview_value: "800", source: "turn_rds", getter: "getTurnRadius", precision: 0, time_mm_ss: false, visible_when: None, na_when: Some(Cond::Gt(9999.0)), imperial_display: false },
    FieldDef { label: "攻  角", unit: "Deg", preview_value: "4.2", source: "aoa", getter: "getAoA", precision: 1, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "侧滑角", unit: "Deg", preview_value: "0.5", source: "aos", getter: "getAoS", precision: 1, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    // cfg 原样表达式 "getWingSweep * 100" — 取数层乘数语法
    FieldDef { label: "可变翼", unit: "%", preview_value: "15", source: "wing_sweep * 100", getter: "getWingSweep", precision: 0, time_mm_ss: false, visible_when: Some(Cond::NotEq(0.0)), na_when: None, imperial_display: false },
    FieldDef { label: "测距高", unit: "M", preview_value: "325", source: "radio_altitude", getter: "getRadioAltitude", precision: 0, time_mm_ss: false, visible_when: Some(Cond::Gte(0.0)), na_when: None, imperial_display: false },
];

/// ui_layout.cfg "性能数据" 组逐行快照 (19 项, 顺序一致; 值照抄原 PowerFieldDef 表)
pub const POWER_FIELD_DEFS: &[FieldDef] = &[
    FieldDef { label: "功  率", unit: "Hp", preview_value: "1200", source: "horse_power", getter: "horse_power", precision: 0, time_mm_ss: false, visible_when: Some(NOT_JET), na_when: Some(Cond::Lte(0.0)), imperial_display: false },
    FieldDef { label: "推  力", unit: "Kgf", preview_value: "1000", source: "thrust", getter: "thrust", precision: 0, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "转  速", unit: "Rpm", preview_value: "2400", source: "rpm", getter: "rpm", precision: 0, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "桨距角", unit: "Deg", preview_value: "55", source: "prop_pitch", getter: "prop_pitch", precision: 1, time_mm_ss: false, visible_when: Some(NOT_JET), na_when: Some(Cond::Eq(-65535.0)), imperial_display: false },
    FieldDef { label: "桨效率", unit: "%", preview_value: "85", source: "prop_efficiency", getter: "prop_efficiency", precision: 1, time_mm_ss: false, visible_when: Some(NOT_JET), na_when: Some(Cond::Lte(0.0)), imperial_display: false },
    FieldDef { label: "实功率", unit: "Hp", preview_value: "1100", source: "eff_hp", getter: "eff_hp", precision: 0, time_mm_ss: false, visible_when: Some(NOT_JET), na_when: Some(Cond::Lte(0.0)), imperial_display: false },
    FieldDef { label: "进气压", unit: "Ata", preview_value: "1.2", source: "manifold_pressure_display", getter: "manifold_pressure_display", precision: 2, time_mm_ss: false, visible_when: Some(MANIFOLD_VIS), na_when: None, imperial_display: true },
    FieldDef { label: "动力量", unit: "%", preview_value: "95", source: "power_percent", getter: "power_percent", precision: 0, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "燃油量", unit: "Kg", preview_value: "500", source: "mass_fuel", getter: "mass_fuel", precision: 0, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "总  重", unit: "Kg", preview_value: "3500", source: "total_weight", getter: "total_weight", precision: 0, time_mm_ss: false, visible_when: Some(Cond::Gt(0.0)), na_when: None, imperial_display: false },
    // cfg 原样表达式 "getFuelTimeMili * 0.001" (ms→s) + :format TIME_MM_SS
    FieldDef { label: "燃油时", unit: "s", preview_value: "45", source: "fuel_time_mili * 0.001", getter: "fuel_time_mili", precision: 0, time_mm_ss: true, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "加力量", unit: "Kg", preview_value: "50", source: "wep_kg", getter: "wep_kg", precision: 0, time_mm_ss: false, visible_when: Some(Cond::HasWep), na_when: None, imperial_display: false },
    FieldDef { label: "加力时", unit: "s", preview_value: "300", source: "wep_time", getter: "wep_time", precision: 0, time_mm_ss: true, visible_when: Some(WEP_TIME_VIS), na_when: None, imperial_display: false },
    FieldDef { label: "助推燃料", unit: "Kg", preview_value: "850", source: "booster_fuel_kg", getter: "booster_fuel_kg", precision: 1, time_mm_ss: false, visible_when: Some(Cond::HasBooster), na_when: None, imperial_display: false },
    FieldDef { label: "助推余量", unit: "%", preview_value: "100", source: "booster_fuel_percent", getter: "booster_fuel_percent", precision: 0, time_mm_ss: false, visible_when: Some(Cond::HasBooster), na_when: None, imperial_display: false },
    FieldDef { label: "温  度", unit: "C", preview_value: "90", source: "water_temp", getter: "water_temp", precision: 0, time_mm_ss: false, visible_when: None, na_when: Some(Cond::Lte(-65535.0)), imperial_display: false },
    FieldDef { label: "油  温", unit: "C", preview_value: "80", source: "oil_temp", getter: "oil_temp", precision: 0, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
    FieldDef { label: "耐热时", unit: "S", preview_value: "60", source: "heat_tolerance", getter: "heat_tolerance", precision: 0, time_mm_ss: false, visible_when: None, na_when: Some(Cond::Gt(90000.0)), imperial_display: false },
    FieldDef { label: "响应速", unit: "%/s", preview_value: "10", source: "engine_response", getter: "engine_response", precision: 0, time_mm_ss: false, visible_when: None, na_when: None, imperial_display: false },
];
