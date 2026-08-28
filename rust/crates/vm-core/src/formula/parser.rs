//! 公式语言递归下降解析器: Token 流 → Expr(未解析 AST)。
//! 文法层次: doc/formula_system_design.md §3.1

use super::ast::{BinOp, Expr, UnOp};
use super::lexer::Tok;

/// 语法错误 (位置 = 出错 token 序号)
#[derive(Debug, Clone)]
pub struct ParseError {
    pub pos: usize,
    pub msg: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "位置 {}: {}", self.pos, self.msg)
    }
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    /// 状态原语调用点编号 (编译期供 RExpr::Call.site)
    next_site: u32,
}

/// 解析入口: 返回 (AST, 状态原语调用点总数)
pub fn parse(src: &str) -> Result<(Expr, u32), String> {
    let toks = super::lexer::lex(src).map_err(|e| e.to_string())?;
    let mut p = Parser { toks, pos: 0, next_site: 0 };
    let expr = p.parse_ternary().map_err(|e| e.to_string())?;
    if p.pos < p.toks.len() {
        return Err(format!("表达式末尾有多余内容: {:?}", p.toks[p.pos]));
    }
    Ok((expr, p.next_site))
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == Some(t) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, t: &Tok) -> Result<(), ParseError> {
        if self.eat(t) {
            Ok(())
        } else {
            Err(ParseError {
                pos: self.pos,
                msg: format!("期望 {:?}, 实际 {:?}", t, self.peek()),
            })
        }
    }

    /// ident 是否为逻辑关键字 (and/or/not 与符号形态等价)
    fn kw_ident(&self) -> Option<&'static str> {
        match self.peek() {
            Some(Tok::Ident(s)) if s == "and" || s == "or" || s == "not" => {
                Some(match s.as_str() {
                    "and" => "and",
                    "or" => "or",
                    _ => "not",
                })
            }
            _ => None,
        }
    }

    fn parse_ternary(&mut self) -> Result<Expr, ParseError> {
        let cond = self.parse_or()?;
        if self.eat(&Tok::Question) {
            let then = self.parse_ternary()?;
            self.expect(&Tok::Colon)?;
            let els = self.parse_ternary()?;
            return Ok(Expr::Ternary {
                cond: Box::new(cond),
                then: Box::new(then),
                els: Box::new(els),
            });
        }
        Ok(cond)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_and()?;
        loop {
            let is_or = self.peek() == Some(&Tok::OrOr) || self.kw_ident() == Some("or");
            if !is_or {
                break;
            }
            self.bump(); // 消费 || 或 or
            let rhs = self.parse_and()?;
            lhs = Expr::Binary { op: BinOp::Or, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_cmp()?;
        loop {
            let is_and = self.peek() == Some(&Tok::AndAnd) || self.kw_ident() == Some("and");
            if !is_and {
                break;
            }
            self.bump();
            let rhs = self.parse_cmp()?;
            lhs = Expr::Binary { op: BinOp::And, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_cmp(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_add()?;
        let op = match self.peek() {
            Some(Tok::EqEq) => Some(BinOp::Eq),
            Some(Tok::NotEq) => Some(BinOp::Ne),
            Some(Tok::Lt) => Some(BinOp::Lt),
            Some(Tok::Le) => Some(BinOp::Le),
            Some(Tok::Gt) => Some(BinOp::Gt),
            Some(Tok::Ge) => Some(BinOp::Ge),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let rhs = self.parse_add()?;
            return Ok(Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) });
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Plus) => BinOp::Add,
                Some(Tok::Minus) => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_mul()?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Star) => BinOp::Mul,
                Some(Tok::Slash) => BinOp::Div,
                Some(Tok::Percent) => BinOp::Mod,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_unary()?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Some(Tok::Minus) => {
                self.bump();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary { op: UnOp::Neg, expr: Box::new(expr) })
            }
            Some(Tok::Not) => {
                self.bump();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary { op: UnOp::Not, expr: Box::new(expr) })
            }
            Some(Tok::Ident(s)) if s == "not" => {
                self.bump();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary { op: UnOp::Not, expr: Box::new(expr) })
            }
            _ => self.parse_pow(),
        }
    }

    /// 幂右结合: primary ["^" unary] — 右结合由递归到 unary 实现
    fn parse_pow(&mut self) -> Result<Expr, ParseError> {
        let base = self.parse_primary()?;
        if self.eat(&Tok::Caret) {
            let exp = self.parse_unary()?;
            return Ok(Expr::Binary {
                op: BinOp::Pow,
                lhs: Box::new(base),
                rhs: Box::new(exp),
            });
        }
        Ok(base)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.bump() {
            Some(Tok::Num(v)) => Ok(Expr::Num(v)),
            Some(Tok::Ident(name)) => {
                // 函数调用 vs 变量引用
                if self.peek() == Some(&Tok::LParen) {
                    self.bump();
                    let mut args = Vec::new();
                    if self.peek() != Some(&Tok::RParen) {
                        loop {
                            args.push(self.parse_ternary()?);
                            if !self.eat(&Tok::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(&Tok::RParen)?;
                    self.next_site += 1;
                    return Ok(Expr::Call { name, args });
                }
                Ok(Expr::Name(name))
            }
            Some(Tok::LParen) => {
                let inner = self.parse_ternary()?;
                self.expect(&Tok::RParen)?;
                Ok(inner)
            }
            other => Err(ParseError {
                pos: self.pos.saturating_sub(1),
                msg: format!("意外 token: {other:?}"),
            }),
        }
    }
}
