//! 公式定义与编译器: FormulaDef[] → CompiledFormulaSet (拓扑序) → 每帧求值。
//! 编译管线: parse → resolve(名字→编号, site 全局分配) → 依赖图 → 环检测 → 拓扑序。
//! 设计: doc/formula_system_design.md §6

use super::ast::{collect_formula_refs, Expr, RExpr};
use super::eval::{eval, EvalCtx, StateStore};
use super::functions::{arity, fid_to_u16, is_stateful, resolve_fn};
use super::registry::VarSnapshot;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// 公式定义 (持久化形态, formulas.cfg 条目)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormulaDef {
    pub name: String,
    pub expr: String,
    pub unit: String,
    pub precision: u8,
    pub desc: String,
    pub disabled: bool,
    /// 内置(出厂)公式 — 编辑器只读/另存副本
    pub builtin: bool,
}

/// 编译错误 (诊断/编辑器标注)
#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    Parse(String),
    UnknownName(String),
    UnknownFn(String),
    BadArity {
        f: String,
        got: usize,
    },
    DuplicateName(String),
    Cycle(Vec<String>),
    /// 接管型公式 (与系统变量同名) 的表达式引用了自身 — 会隐式变成
    /// "引用自己上一帧的值" (prev 语义), 混乱且难排查, 编译期拒绝
    SelfOverride(String),
    DisabledByUser,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Parse(m) => write!(f, "语法错误: {m}"),
            CompileError::UnknownName(n) => write!(f, "未知变量或公式: {n}"),
            CompileError::UnknownFn(n) => write!(f, "未知函数: {n}"),
            CompileError::BadArity { f: name, got } => write!(f, "函数 {name} 参数个数不符: {got}"),
            CompileError::DuplicateName(n) => write!(f, "公式重名: {n}"),
            CompileError::Cycle(chain) => write!(f, "循环依赖: {}", chain.join(" → ")),
            CompileError::SelfOverride(n) => {
                write!(f, "公式 {n} 与系统变量同名 (接管型), 表达式内不能引用自身 (需上一帧值请用 prev/状态原语)")
            }
            CompileError::DisabledByUser => write!(f, "已禁用"),
        }
    }
}

/// 名字→变量编号解析 (registry 实现; 测试可 stub)
pub trait VarLookup {
    fn lookup(&self, name: &str) -> Option<u16>;
    fn version(&self) -> u32;
}

/// 编译产物中的单个公式
#[derive(Debug, Clone)]
pub struct CompiledFormula {
    pub def: FormulaDef,
    /// resolve 产物 (None = 编译失败)
    pub rexpr: Option<RExpr>,
    pub err: Option<CompileError>,
    /// 依赖的其他公式槽
    pub deps: Vec<u16>,
    /// 本公式持有的状态原语调用点 (热更新差集清理用)
    pub sites: Vec<u32>,
}

/// 编译集: 槽号 = formulas 下标
#[derive(Debug, Clone)]
pub struct CompiledFormulaSet {
    pub formulas: Vec<CompiledFormula>,
    /// 拓扑序的公式槽序列 (valid 且非 disabled)
    pub order: Vec<u16>,
    /// 公式名 → 槽 (Arc 形态: 求值线程每帧零拷贝挂到 ServiceData 供 overlay 绑定)
    pub slots: Arc<HashMap<String, u16>>,
    pub registry_version: u32,
}

impl CompiledFormulaSet {
    /// 名字表的共享句柄 (formula_step 写回 ServiceData.formula_slots)
    pub fn slots_arc(&self) -> Arc<HashMap<String, u16>> {
        Arc::clone(&self.slots)
    }
}

/// 一帧求值结果 (槽号索引; invalid/死公式 = NaN)
#[derive(Debug, Clone, Default)]
pub struct FormulaResults {
    pub values: Vec<f64>,
}

impl FormulaResults {
    pub fn get(&self, slot: u16) -> f64 {
        self.values.get(slot as usize).copied().unwrap_or(f64::NAN)
    }
}

impl CompiledFormulaSet {
    /// 编译入口 (W-C: 公式恒活 — 原 external_refs/liveness 机制因"全名塞入=永不
    /// 生效"已删, 公式量小成本可忽略; disabled/invalid 仍排除出求值序)。
    pub fn compile(defs: &[FormulaDef], reg: &dyn VarLookup) -> Self {
        // 1. 槽预分配 + 重名检查 (后者 invalid)
        let mut slots: HashMap<String, u16> = HashMap::new();
        let mut formulas: Vec<CompiledFormula> = Vec::with_capacity(defs.len());
        let mut occupied: HashSet<String> = HashSet::new();
        for (i, def) in defs.iter().enumerate() {
            let dup = def.name.is_empty() || !occupied.insert(def.name.clone());
            slots.insert(def.name.clone(), i as u16);
            formulas.push(CompiledFormula {
                def: def.clone(),
                rexpr: None,
                err: if dup {
                    Some(CompileError::DuplicateName(def.name.clone()))
                } else if def.disabled {
                    Some(CompileError::DisabledByUser)
                } else {
                    None
                },
                deps: Vec::new(),
                sites: Vec::new(),
            });
        }

        // 2. parse + resolve (site 全局唯一分配)
        let mut next_site: u32 = 0;
        for slot in 0..formulas.len() as u16 {
            if formulas[slot as usize].err.is_some() {
                continue;
            }
            let expr_src = formulas[slot as usize].def.expr.clone();
            let own_name = formulas[slot as usize].def.name.clone();
            match resolve_formula(&expr_src, reg, &slots, &own_name, &mut next_site) {
                Ok((rexpr, sites)) => {
                    let mut deps = Vec::new();
                    collect_formula_refs(&rexpr, &mut deps);
                    let f = &mut formulas[slot as usize];
                    f.rexpr = Some(rexpr);
                    f.sites = sites;
                    f.deps = deps;
                }
                Err(e) => {
                    formulas[slot as usize].err = Some(e);
                }
            }
        }

        // 3. 环检测 (DFS 三色染色, 环上公式全部标 Cycle)
        detect_cycles(&mut formulas);

        // 4. 拓扑排序 (Kahn; 有环/invalid 公式已被排除出 deps 有效性,
        //    被 invalid 公式依赖者求值自然 NaN, 不阻断拓扑)
        let order = topo_sort(&formulas);

        CompiledFormulaSet {
            formulas,
            order,
            slots: Arc::new(slots),
            registry_version: reg.version(),
        }
    }

    /// 帧求值 (Service 线程单点; 死公式跳过)
    pub fn eval_frame(
        &self,
        snap: &VarSnapshot,
        store: &mut StateStore,
        now_ms: u64,
        interval_ms: f64,
        fm_data: Option<&crate::fm::data::FmData>,
    ) -> FormulaResults {
        let mut results = FormulaResults {
            values: vec![f64::NAN; self.formulas.len()],
        };
        for &slot in &self.order {
            let f = &self.formulas[slot as usize];
            if let Some(rexpr) = &f.rexpr {
                let ctx = EvalCtx {
                    snap,
                    results: &results,
                    now_ms,
                    interval_ms,
                    fm_data,
                };
                let v = eval(rexpr, &ctx, store).num();
                results.values[slot as usize] = v;
            }
        }
        results
    }

    /// 全部存活公式的状态原语调用点并集 (热更新差集清理)
    pub fn alive_sites(&self) -> HashSet<u32> {
        let mut s = HashSet::new();
        for f in &self.formulas {
            if f.err.is_none() && !f.def.disabled {
                s.extend(f.sites.iter().copied());
            }
        }
        s
    }

}

/// resolve_expr 的公开包装 (rules.rs 复用公式编译链编译 when 表达式)
pub fn resolve_rexpr_public(
    e: &Expr,
    reg: &dyn VarLookup,
    slots: &HashMap<String, u16>,
    next_site: &mut u32,
    sites: &mut Vec<u32>,
) -> Result<RExpr, CompileError> {
    resolve_expr(e, reg, slots, "", next_site, sites)
}

/// 单公式 resolve: parse + 名字编号化 + arity 检查 + site 分配;
/// own_name 用于接管型自引用检查 (与系统变量同名的公式, 其表达式内引用
/// 自身 = 隐式"上一帧值"语义, 编译期拒绝)
fn resolve_formula(
    src: &str,
    reg: &dyn VarLookup,
    slots: &HashMap<String, u16>,
    own_name: &str,
    next_site: &mut u32,
) -> Result<(RExpr, Vec<u32>), CompileError> {
    let (expr, _site_count) = super::parser::parse(src).map_err(CompileError::Parse)?;
    let mut sites = Vec::new();
    let r = resolve_expr(&expr, reg, slots, own_name, next_site, &mut sites)?;
    // 常量折叠 pass (W1c): 纯运算符 Num-Num 子树折为 Num, 不折函数调用
    Ok((fold_consts(r), sites))
}

fn resolve_expr(
    e: &Expr,
    reg: &dyn VarLookup,
    slots: &HashMap<String, u16>,
    own_name: &str,
    next_site: &mut u32,
    sites: &mut Vec<u32>,
) -> Result<RExpr, CompileError> {
    Ok(match e {
        Expr::Num(v) => RExpr::Num(*v),
        Expr::Name(n) => {
            if !own_name.is_empty() && n == own_name && reg.lookup(n).is_some() {
                return Err(CompileError::SelfOverride(n.clone()));
            }
            if let Some(vid) = reg.lookup(n) {
                RExpr::Var(vid)
            } else if let Some(&slot) = slots.get(n) {
                // 自引用由环检测裁决
                RExpr::Formula(slot)
            } else {
                return Err(CompileError::UnknownName(n.clone()));
            }
        }
        Expr::Call { name, args } => {
            let fid = resolve_fn(name).ok_or_else(|| CompileError::UnknownFn(name.clone()))?;
            let (lo, hi) = arity(fid);
            if args.len() < lo || args.len() > hi {
                return Err(CompileError::BadArity {
                    f: name.clone(),
                    got: args.len(),
                });
            }
            let mut rargs = Vec::with_capacity(args.len());
            for a in args {
                rargs.push(resolve_expr(a, reg, slots, own_name, next_site, sites)?);
            }
            // site 全局分配 (每调用点一份, StateStore 键)
            let site = *next_site;
            *next_site += 1;
            if is_stateful(fid) {
                sites.push(site);
            }
            RExpr::Call {
                fid: fid_to_u16(fid),
                args: rargs,
                site,
            }
        }
        Expr::Unary { op, expr } => RExpr::Unary {
            op: *op,
            expr: Box::new(resolve_expr(expr, reg, slots, own_name, next_site, sites)?),
        },
        Expr::Binary { op, lhs, rhs } => RExpr::Binary {
            op: *op,
            lhs: Box::new(resolve_expr(lhs, reg, slots, own_name, next_site, sites)?),
            rhs: Box::new(resolve_expr(rhs, reg, slots, own_name, next_site, sites)?),
        },
        Expr::Ternary { cond, then, els } => RExpr::Ternary {
            cond: Box::new(resolve_expr(cond, reg, slots, own_name, next_site, sites)?),
            then: Box::new(resolve_expr(then, reg, slots, own_name, next_site, sites)?),
            els: Box::new(resolve_expr(els, reg, slots, own_name, next_site, sites)?),
        },
    })
}

/// DFS 三色染色环检测: 环上公式标 Cycle(完整环链) 并断开其依赖
fn detect_cycles(formulas: &mut [CompiledFormula]) {
    let n = formulas.len();
    // 有效公式才算图节点 (invalid 已带 err)
    let mut color = vec![0u8; n]; // 0=white 1=gray 2=black
    let mut stack: Vec<usize> = Vec::new();

    // 深度递归改显式栈 (公式依赖链理论可深, 防栈溢)
    fn dfs(u: usize, formulas: &mut [CompiledFormula], color: &mut [u8], stack: &mut Vec<usize>) {
        if color[u] != 0 {
            return;
        }
        color[u] = 1;
        stack.push(u);
        let deps = formulas[u].deps.clone();
        let mut i = 0;
        loop {
            if i >= deps.len() {
                break;
            }
            let v = deps[i] as usize;
            match color[v] {
                0 => {
                    dfs(v, formulas, color, stack);
                    // 回到本层后继续处理剩余 dep
                }
                1 => {
                    // 发现环: stack 中 v..=u 是环链
                    let chain: Vec<String> = stack
                        .iter()
                        .skip_while(|&&s| s != v)
                        .map(|&s| formulas[s].def.name.clone())
                        .chain(std::iter::once(formulas[v].def.name.clone()))
                        .collect();
                    // 环上所有公式标 invalid (仍在 stack gray 区间 v..)
                    let cyc: HashSet<usize> =
                        stack.iter().skip_while(|&&s| s != v).cloned().collect();
                    for &cu in &cyc {
                        if color[cu] == 1 {
                            color[cu] = 2; // 视作处理完, 防重复标
                            formulas[cu].err = Some(CompileError::Cycle(chain.clone()));
                            formulas[cu].rexpr = None;
                            formulas[cu].deps.clear();
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
        // 弹栈
        while let Some(top) = stack.pop() {
            if top == u {
                break;
            }
            color[top] = 2;
        }
        color[u] = 2;
    }

    let mut u = 0;
    while u < n {
        if formulas[u].err.is_none() {
            dfs(u, formulas, &mut color, &mut stack);
        } else {
            color[u] = 2;
        }
        u += 1;
    }
}

/// Kahn 拓扑排序 (err 公式不进 order)
fn topo_sort(formulas: &[CompiledFormula]) -> Vec<u16> {
    let n = formulas.len();
    let mut indeg = vec![0usize; n];
    let mut rev: Vec<Vec<u16>> = vec![Vec::new(); n];
    for (i, f) in formulas.iter().enumerate() {
        if f.err.is_some() {
            continue;
        }
        for &d in &f.deps {
            // 依赖 invalid 公式: 求值 NaN, 但不阻断拓扑 — 不计边
            if formulas[d as usize].err.is_none() && d as usize != i {
                indeg[i] += 1;
                rev[d as usize].push(i as u16);
            }
        }
    }
    let mut q: VecDeque<usize> = (0..n)
        .filter(|&i| indeg[i] == 0 && formulas[i].err.is_none())
        .collect();
    let mut order = Vec::with_capacity(n);
    while let Some(u) = q.pop_front() {
        order.push(u as u16);
        for &v in &rev[u] {
            indeg[v as usize] -= 1;
            if indeg[v as usize] == 0 {
                q.push_back(v as usize);
            }
        }
    }
    order
}

/// 单公式试算 (编辑器 TryPanel 用: 无持久化/无依赖公式时逐个验证)
pub fn try_eval_single(
    expr_src: &str,
    reg: &dyn VarLookup,
    snap: &VarSnapshot,
    store: &mut StateStore,
    now_ms: u64,
    interval_ms: f64,
    fm_data: Option<&crate::fm::data::FmData>,
) -> Result<f64, CompileError> {
    // 单公式命名空间: 空 slots (无公式间引用), site 从 0 起
    let slots = HashMap::new();
    let mut next_site = 0u32;
    let (rexpr, _sites) = resolve_formula(expr_src, reg, &slots, "", &mut next_site)?;
    let empty = FormulaResults { values: Vec::new() };
    let ctx = EvalCtx {
        snap,
        results: &empty,
        now_ms,
        interval_ms,
        fm_data,
    };
    Ok(eval(&rexpr, &ctx, store).num())
}

/// 编译期常量折叠 (W1c, 设计 §6.1): 纯运算符两端皆 Num 的子树折为单 Num,
/// 运算式与 eval.rs 运行时语义逐项一致 (含除零→IEEE、比较→0/1、逻辑短路)。
/// 不折函数调用 (语义/NaN 陷阱保守); site 已在折叠前收集, 不受影响。
fn fold_consts(r: RExpr) -> RExpr {
    use super::ast::BinOp::{self, *};
    use super::ast::UnOp;
    let fold_bin = |op: BinOp, l: f64, rr: f64| -> f64 {
        match op {
            Add => l + rr,
            Sub => l - rr,
            Mul => l * rr,
            Div => l / rr,
            Mod => l % rr,
            Pow => l.powf(rr),
            Eq => (l == rr) as u8 as f64,
            Ne => (l != rr) as u8 as f64,
            Lt => (l < rr) as u8 as f64,
            Le => (l <= rr) as u8 as f64,
            Gt => (l > rr) as u8 as f64,
            Ge => (l >= rr) as u8 as f64,
            And => {
                if l == 0.0 {
                    0.0
                } else if l.is_nan() || rr.is_nan() {
                    f64::NAN
                } else {
                    (rr != 0.0) as u8 as f64
                }
            }
            Or => {
                if l != 0.0 && !l.is_nan() {
                    1.0
                } else if l.is_nan() || rr.is_nan() {
                    f64::NAN
                } else {
                    (rr != 0.0) as u8 as f64
                }
            }
        }
    };
    match r {
        RExpr::Unary { op, expr } => {
            let e = fold_consts(*expr);
            if let (RExpr::Num(v), UnOp::Neg) = (&e, op) {
                RExpr::Num(-v)
            } else {
                RExpr::Unary {
                    op,
                    expr: Box::new(e),
                }
            }
        }
        RExpr::Binary { op, lhs, rhs } => {
            let l = fold_consts(*lhs);
            let rr = fold_consts(*rhs);
            // 逻辑短路常量先判 (另一支不约束, 折叠即短路)
            if let RExpr::Num(a) = &l {
                if op == BinOp::And && *a == 0.0 {
                    return RExpr::Num(0.0);
                }
                if op == BinOp::Or && *a != 0.0 && !a.is_nan() {
                    return RExpr::Num(1.0);
                }
            }
            match (&l, &rr) {
                (RExpr::Num(a), RExpr::Num(b)) => RExpr::Num(fold_bin(op, *a, *b)),
                _ => RExpr::Binary {
                    op,
                    lhs: Box::new(l),
                    rhs: Box::new(rr),
                },
            }
        }
        RExpr::Ternary { cond, then, els } => {
            let c = fold_consts(*cond);
            match &c {
                RExpr::Num(v) if !v.is_nan() => {
                    if *v != 0.0 {
                        fold_consts(*then)
                    } else {
                        fold_consts(*els)
                    }
                }
                _ => RExpr::Ternary {
                    cond: Box::new(c),
                    then: Box::new(fold_consts(*then)),
                    els: Box::new(fold_consts(*els)),
                },
            }
        }
        RExpr::Call { fid, args, site } => RExpr::Call {
            fid,
            args: args.into_iter().map(fold_consts).collect(),
            site,
        },
        other => other,
    }
}
