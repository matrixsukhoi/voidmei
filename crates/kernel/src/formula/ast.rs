//! 公式系统 AST: 未解析(名字形态, parser 产物)与已解析(编号形态, 编译产物)。
//! 设计: doc/formula_system_design.md §3

/// 一元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

/// 二元运算符 (含比较与逻辑; 逻辑语义见 eval.rs 短路说明)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

/// 未解析表达式 — 变量与函数以字符串表示 (parser 直接产物)
#[derive(Debug, Clone)]
pub enum Expr {
    Num(f64),
    /// 变量引用 (原子变量名 / 公式名 / 常量名, 编译期裁决)
    Name(String),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Ternary {
        cond: Box<Expr>,
        then: Box<Expr>,
        els: Box<Expr>,
    },
}

/// 已解析表达式 — 名字全部替换为编号:
/// - 变量 → 注册表 VarId (registry.rs)
/// - 其他公式 → 公式结果槽编号 (definition.rs 拓扑排序后回填)
/// - 函数 → FnId (functions.rs); site 为状态原语调用点编号
///   (编译期按 AST 遍历序分配, 求值期作 StateStore 键, 同一公式内唯一)
#[derive(Debug, Clone)]
pub enum RExpr {
    Num(f64),
    /// 注册表原子变量
    Var(u16),
    /// 其他公式的结果槽 (拓扑序保证被引用者先算)
    Formula(u16),
    Call {
        fid: u16,
        args: Vec<RExpr>,
        site: u32,
    },
    Unary {
        op: UnOp,
        expr: Box<RExpr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<RExpr>,
        rhs: Box<RExpr>,
    },
    Ternary {
        cond: Box<RExpr>,
        then: Box<RExpr>,
        els: Box<RExpr>,
    },
}

/// 收集 RExpr 中对其他公式的引用槽 (编译期建依赖图用)
pub fn collect_formula_refs(expr: &RExpr, out: &mut Vec<u16>) {
    match expr {
        RExpr::Formula(slot) => {
            if !out.contains(slot) {
                out.push(*slot);
            }
        }
        RExpr::Num(_) | RExpr::Var(_) => {}
        RExpr::Call { args, .. } => {
            for a in args {
                collect_formula_refs(a, out);
            }
        }
        RExpr::Unary { expr, .. } => collect_formula_refs(expr, out),
        RExpr::Binary { lhs, rhs, .. } => {
            collect_formula_refs(lhs, out);
            collect_formula_refs(rhs, out);
        }
        RExpr::Ternary { cond, then, els } => {
            collect_formula_refs(cond, out);
            collect_formula_refs(then, out);
            collect_formula_refs(els, out);
        }
    }
}
