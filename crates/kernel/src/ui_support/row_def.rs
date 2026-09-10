//! visibleWhen/naWhen 中缀条件 → [`Cond`] 编译与求值。
//! 消费方 = widgets::data_field (原子数据字段组件, factory_default.json
//! 的 `:type data` 行属性直读 — 行编译中间层已随 fields.grid 退役)。

use crate::formula::registry::FormulaView;

/// 受限条件 (visibleWhen / naWhen 的编译产物; owned)。
/// `==`/`!=` 带 0.0001 容差 — 语义对齐旧 VisibilityExpressionEvaluator。
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

// =====================================================================
// 中缀条件编译器 (visibleWhen/naWhen: "value > 0 && !isJetEngine")
// =====================================================================

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    And,
    Or,
    Not,
    LParen,
    RParen,
    Ident(String),
    Num(f64),
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
    Neq,
}

/// 词法分析 (错误 → None, 调用方按无条件处理)
fn tokenize(s: &str) -> Option<Vec<Tok>> {
    let b = s.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        match b[i] {
            b' ' | b'\t' => i += 1,
            b'&' => {
                if i + 1 < b.len() && b[i + 1] == b'&' {
                    out.push(Tok::And);
                    i += 2;
                } else {
                    return None;
                }
            }
            b'|' => {
                if i + 1 < b.len() && b[i + 1] == b'|' {
                    out.push(Tok::Or);
                    i += 2;
                } else {
                    return None;
                }
            }
            b'!' => {
                if i + 1 < b.len() && b[i + 1] == b'=' {
                    out.push(Tok::Neq);
                    i += 2;
                } else {
                    out.push(Tok::Not);
                    i += 1;
                }
            }
            b'(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            b')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            b'>' => {
                if i + 1 < b.len() && b[i + 1] == b'=' {
                    out.push(Tok::Gte);
                    i += 2;
                } else {
                    out.push(Tok::Gt);
                    i += 1;
                }
            }
            b'<' => {
                if i + 1 < b.len() && b[i + 1] == b'=' {
                    out.push(Tok::Lte);
                    i += 2;
                } else {
                    out.push(Tok::Lt);
                    i += 1;
                }
            }
            b'=' => {
                if i + 1 < b.len() && b[i + 1] == b'=' {
                    out.push(Tok::Eq);
                    i += 2;
                } else {
                    return None; // 单 = 不是合法比较符
                }
            }
            c if c.is_ascii_digit() || c == b'-' || c == b'+' || c == b'.' => {
                // 数字字面量 (支持负数/小数)
                let start = i;
                i += 1;
                while i < b.len()
                    && (b[i].is_ascii_digit() || b[i] == b'.' || b[i] == b'e' || b[i] == b'E'
                        || ((b[i] == b'-' || b[i] == b'+')
                            && (b[i - 1] == b'e' || b[i - 1] == b'E')))
                {
                    i += 1;
                }
                let n: f64 = s[start..i].parse().ok()?;
                out.push(Tok::Num(n));
            }
            c if c.is_ascii_alphanumeric() || c == b'_' => {
                let start = i;
                i += 1;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                out.push(Tok::Ident(s[start..i].to_string()));
            }
            _ => return None,
        }
    }
    Some(out)
}

/// 中缀条件 → Cond。
/// 文法: or := and ('||' and)*; and := unary ('&&' unary)*;
///       unary := '!' unary | primary;
///       primary := '(' or ')' | 谓词 | 'value' relop 数字
/// (visibleWhen/naWhen 共用; 原子数据字段组件同源消费)
pub fn compile_cond(expr: &str) -> Option<Cond> {
    let toks = tokenize(expr)?;
    let mut p = Parser { toks: &toks, pos: 0 };
    let c = p.parse_or()?;
    if p.pos != p.toks.len() {
        return None; // 尾部残留 = 语法错误
    }
    Some(c)
}

struct Parser<'a> {
    toks: &'a [Tok],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos)?.clone();
        self.pos += 1;
        Some(t)
    }

    fn parse_or(&mut self) -> Option<Cond> {
        let mut acc = self.parse_and()?;
        while self.peek() == Some(&Tok::Or) {
            self.pos += 1;
            acc = Cond::Or(Box::new(acc), Box::new(self.parse_and()?));
        }
        Some(acc)
    }

    fn parse_and(&mut self) -> Option<Cond> {
        let mut acc = self.parse_unary()?;
        while self.peek() == Some(&Tok::And) {
            self.pos += 1;
            acc = Cond::And(Box::new(acc), Box::new(self.parse_unary()?));
        }
        Some(acc)
    }

    fn parse_unary(&mut self) -> Option<Cond> {
        if self.peek() == Some(&Tok::Not) {
            self.pos += 1;
            return Some(Cond::Not(Box::new(self.parse_unary()?)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<Cond> {
        match self.next()? {
            Tok::LParen => {
                let c = self.parse_or()?;
                if self.next()? != Tok::RParen {
                    return None;
                }
                Some(c)
            }
            Tok::Ident(name) => self.ident_or_compare(name),
            _ => None, // 数字/运算符开头 = 语法错误
        }
    }

    /// 标识符: 谓词 | 'value' relop 数字
    fn ident_or_compare(&mut self, name: String) -> Option<Cond> {
        let pred = |n: &str| -> Option<Cond> {
            match n {
                "isJetEngine" => Some(Cond::IsJetEngine),
                "isPropEngine" => Some(Cond::IsPropEngine),
                "isPistonEngine" => Some(Cond::IsPistonEngine),
                "hasWep" => Some(Cond::HasWep),
                "hasBooster" => Some(Cond::HasBooster),
                _ => None,
            }
        };
        if name != "value" {
            return pred(&name);
        }
        // value 后必须跟比较符 + 数字
        let op = self.next()?;
        let Tok::Num(n) = self.next()? else { return None };
        Some(match op {
            Tok::Gt => Cond::Gt(n),
            Tok::Gte => Cond::Gte(n),
            Tok::Lt => Cond::Lt(n),
            Tok::Lte => Cond::Lte(n),
            Tok::Eq => Cond::Eq(n),
            Tok::Neq => Cond::NotEq(n),
            _ => return None,
        })
    }
}
