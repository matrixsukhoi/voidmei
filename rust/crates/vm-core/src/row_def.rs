//! cfg 驱动行定义 (W-D): ui_layout.cfg `:type data` 行 → 运行时 [`RowDef`]。
//! 取代原 fields.rs 静态表 — cfg 是行定义唯一来源 (公式设计 §8 裁决),
//! 显示元数据 (label/unit/precision/preview) 与取数表达式 (:target 短名)
//! 单点维护。编译在主线程完成 (RowConfig 持 Rc<SExp> 不可跨线程),
//! 产物 owned/Send, 经 ReinitParams 通道进 win32 线程。

use crate::config_loader::{GroupConfig, RowConfig};
use crate::formula::registry::FormulaView;
use crate::sexp_parser::{AtomType, SExp};

/// 受限条件 (:visible-when / :na-when 的编译产物; owned)。
/// `=`/`!=` 带 0.0001 容差 — 语义对齐 Java VisibilityExpressionEvaluator。
#[derive(Debug, Clone, PartialEq)]
pub enum Cond {
    // 值比较 (value 为字段当前值)
    NotEq(f64),
    Gte(f64),
    Gt(f64),
    Lt(f64),
    Lte(f64),
    Eq(f64),
    // 环境谓词 (经 var_value 短名取布尔量)
    IsJetEngine,
    IsPropEngine,
    IsPistonEngine,
    HasWep,
    HasBooster,
    Not(Box<Cond>),
    And(Box<Cond>, Box<Cond>),
    Or(Box<Cond>, Box<Cond>),
}

impl Cond {
    /// 求值; value 为字段当前值
    pub fn eval(&self, s: &dyn FormulaView, value: f64) -> bool {
        match self {
            Cond::NotEq(n) => (value - n).abs() >= 0.0001,
            Cond::Gte(n) => value >= *n,
            Cond::Gt(n) => value > *n,
            Cond::Lt(n) => value < *n,
            Cond::Lte(n) => value <= *n,
            Cond::Eq(n) => (value - n).abs() < 0.0001,
            Cond::IsJetEngine => s.var_value("is_jet_engine").unwrap_or(0.0) != 0.0,
            Cond::IsPropEngine => s.var_value("is_prop_engine").unwrap_or(0.0) != 0.0,
            Cond::IsPistonEngine => s.var_value("is_piston_engine").unwrap_or(0.0) != 0.0,
            Cond::HasWep => s.var_value("has_wep").unwrap_or(0.0) != 0.0,
            Cond::HasBooster => s.var_value("has_booster").unwrap_or(0.0) != 0.0,
            Cond::Not(e) => !e.eval(s, value),
            Cond::And(a, b) => a.eval(s, value) && b.eval(s, value),
            Cond::Or(a, b) => a.eval(s, value) || b.eval(s, value),
        }
    }
}

/// 输出格式 (:format)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatKind {
    Plain,
    /// TIME_MM_SS — "mm'ss" 分秒格式
    TimeMmSs,
}

/// 显示模式 (:unit-source/:precision-source 特例 — 全表仅进气压一条)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Metric,
    /// 英制切换: is_imperial 驱动 "P/x.x''"+1 位 / 公制 "Ata"+2 位
    ImperialManifold,
}

/// 单个数据行定义 (两面板统一形态)
#[derive(Debug, Clone, PartialEq)]
pub struct RowDef {
    /// 显示名 (:target-name 优先, 缺省 label — 全角/双空格对齐原样)
    pub label: String,
    pub unit: String,
    /// 预览模式的静态值 (原样字符串, 不经格式化)
    pub preview_value: String,
    /// 取数表达式 (:target): 变量短名 | 公式名 | "X * N" 乘数
    pub source: String,
    /// 小数位 (:precision, 缺省 0)
    pub precision: u8,
    pub format: FormatKind,
    pub display: DisplayMode,
    pub visible_when: Option<Cond>,
    pub na_when: Option<Cond>,
}

/// 组内 `:type data` 行 → RowDef 列表 (顺序保持; 非法表达式按无条件处理 —
/// cfg 用户容错, 语义 = Java 求值异常时的宽松回退)。
/// `disabled` = 行开关过滤 (value=false 的 data/switch 行不进面板 —
/// Java isFieldDisabled 语义, Rust 侧接线修复)。
pub fn rows_from_group(gc: &GroupConfig, disabled: &dyn Fn(&RowConfig) -> bool) -> Vec<RowDef> {
    fn walk(rows: &[RowConfig], disabled: &dyn Fn(&RowConfig) -> bool, out: &mut Vec<RowDef>) {
        for r in rows {
            if r.r#type.eq_ignore_ascii_case("DATA") {
                if !disabled(r) {
                    out.push(row_from_config(r));
                }
            } else {
                // 嵌套 (group ...) = HEADER 行, data 行藏在其 children
                walk(&r.children, disabled, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&gc.rows, disabled, &mut out);
    out
}

fn row_from_config(r: &RowConfig) -> RowDef {
    let target = r.property.clone().unwrap_or_else(|| r.label.clone());
    RowDef {
        label: r
            .target_name
            .clone()
            .unwrap_or_else(|| r.label.clone()),
        unit: r.unit.clone(),
        preview_value: r.preview_value.clone().unwrap_or_else(|| "0".to_string()),
        source: target,
        precision: r.precision.max(0) as u8,
        format: if r.format.eq_ignore_ascii_case("TIME_MM_SS") {
            FormatKind::TimeMmSs
        } else {
            FormatKind::Plain
        },
        display: if r.unit_source.is_some() || r.precision_source.is_some() {
            DisplayMode::ImperialManifold
        } else {
            DisplayMode::Metric
        },
        visible_when: r.visible_when.as_ref().and_then(|e| compile_cond(e)),
        na_when: r.na_when.as_ref().and_then(|e| compile_cond(e)),
    }
}

/// SExp → Cond。文法 (ui_layout.cfg :visible-when): `(op value N)` 值比较 /
/// `(not e)` / `(and a b ...)` / `(or a b ...)` / 裸谓词 `(isJetEngine)`。
fn compile_cond(e: &SExp) -> Option<Cond> {
    let SExp::List(list) = e else { return None };
    let mut it = list.children.iter();
    let head = it.next()?.as_atom().get_string();
    match head {
        // 值比较: 首参须为 value 符号, 次参数字字面量
        ">" | ">=" | "<" | "<=" | "=" | "!=" => {
            let sym = it.next()?.as_atom();
            if sym.get_string() != "value" {
                return None;
            }
            let num = it.next()?.as_atom();
            if num.r#type != AtomType::Number {
                return None;
            }
            let n = num.get_double();
            Some(match head {
                ">" => Cond::Gt(n),
                ">=" => Cond::Gte(n),
                "<" => Cond::Lt(n),
                "<=" => Cond::Lte(n),
                "=" => Cond::Eq(n),
                _ => Cond::NotEq(n),
            })
        }
        "not" | "!" => Some(Cond::Not(Box::new(compile_cond(it.next()?)?))),
        "and" => fold_cond(it, |a, b| Cond::And(Box::new(a), Box::new(b))),
        "or" => fold_cond(it, |a, b| Cond::Or(Box::new(a), Box::new(b))),
        "isJetEngine" => Some(Cond::IsJetEngine),
        "isPropEngine" => Some(Cond::IsPropEngine),
        "isPistonEngine" => Some(Cond::IsPistonEngine),
        "hasWep" => Some(Cond::HasWep),
        "hasBooster" => Some(Cond::HasBooster),
        _ => None,
    }
}

fn fold_cond<'a>(
    mut it: impl Iterator<Item = &'a std::rc::Rc<SExp>>,
    f: impl Fn(Cond, Cond) -> Cond,
) -> Option<Cond> {
    let mut acc = compile_cond(it.next()?)?;
    for e in it {
        acc = f(acc, compile_cond(e)?);
    }
    Some(acc)
}
