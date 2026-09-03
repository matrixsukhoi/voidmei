//! 公式求值器: RExpr + VarSnapshot → Value;含状态原语 StateStore。
//! 状态原语语义逐项对齐现有实现 (位级对拍目标, 见各 match 臂注释)。
//! 设计: doc/formula_system_design.md §3.5/§3.6

use super::ast::{BinOp, RExpr, UnOp};
use super::definition::FormulaResults;
use super::functions::{eval_pure, fid_from_u16, is_ctx_fn, is_stateful, FnId, Value};
use super::registry::VarSnapshot;
use std::collections::HashMap;

/// 单帧求值上下文 (Service 线程组装)
pub struct EvalCtx<'a> {
    pub snap: &'a VarSnapshot,
    /// 本帧各公式已算结果 (拓扑序保证被引用者先算)
    pub results: &'a FormulaResults,
    pub now_ms: u64,
    /// 本帧实际间隔 (ms) — blend/learn_max 的隐含 ratio = interval_ms/1000,
    /// 对齐 service_fields 的 `ratio=freq/1000f`
    pub interval_ms: f64,
    /// 当前 FM (FM 查表函数族 fm_vne 等的表源; None → NaN 隔离)
    pub fm_data: Option<&'a crate::fm::data::FmData>,
}

/// 状态原语的私有状态 (键 = (公式槽, 调用点 site))
enum PrimState {
    /// sma: 渐进均值, 对齐 calc_helper.rs SimpleMovingAverage
    Sma {
        data: Vec<f64>,
        cnt: usize,
        avg: f64,
    },
    /// prev / blend / deriv 的上一帧记忆
    Prev(f64),
    PrevT {
        prev: f64,
        t: u64,
    },
    /// vote: ±n 冻结投票, 对齐 service_loop 的 check_engine_jet
    Vote {
        cnt: i64,
        frozen: Option<f64>,
    },
    /// stable: 值持续不变计时, 对齐 check_flap 的"维持1秒稳定"
    Stable {
        prev: f64,
        held_ms: f64,
        has_prev: bool,
    },
    /// learn_max: 门控内软逼近最大值+超时锁定,
    /// 对齐 methods_engine 的 get_maximum_rpm_learn
    LearnMax {
        cur: f64,
        elapsed_ms: f64,
        locked: bool,
    },
}

/// 状态原语仓: Service 线程私有 (求值单线程, 无竞争)。
/// 键 = 状态原语调用点 site — 编译集全局唯一 (definition.rs 分配)
#[derive(Default)]
pub struct StateStore {
    map: HashMap<u32, PrimState>,
}

impl StateStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 重置全部状态 (FM_CHANGED / 会话重置)
    pub fn reset_all(&mut self) {
        self.map.clear();
    }

    /// 热更新差集清理: 只保留仍存活公式持有的调用点
    pub fn retain_sites(&mut self, alive: &std::collections::HashSet<u32>) {
        self.map.retain(|site, _| alive.contains(site));
    }
}

/// 求值一个已解析表达式
pub fn eval(expr: &RExpr, ctx: &EvalCtx, store: &mut StateStore) -> Value {
    match expr {
        RExpr::Num(v) => Value::Num(*v),
        // 直接索引 (W1c, §6.1): VarId 编译期由 registry 分配恒 < 快照长度
        // (empty 快照也按 registry().len() 构造), 越界不可达 — 免 Option 分支
        RExpr::Var(vid) => Value::Num(ctx.snap.values[*vid as usize]),
        RExpr::Formula(slot) => Value::Num(ctx.results.get(*slot)),
        RExpr::Unary { op, expr } => {
            let v = eval(expr, ctx, store).num();
            Value::Num(match op {
                UnOp::Neg => -v,
                // NaN 在逻辑上下文传播 (NaN != 0.0 在 IEEE 为 true, 不传播会吞错)
                UnOp::Not => {
                    if v.is_nan() {
                        f64::NAN
                    } else {
                        (!(v != 0.0)) as u8 as f64
                    }
                }
            })
        }
        RExpr::Binary { op, lhs, rhs } => eval_binary(*op, lhs, rhs, ctx, store),
        RExpr::Ternary { cond, then, els } => {
            let c = eval(cond, ctx, store).num();
            if c.is_nan() {
                Value::Num(f64::NAN)
            } else if c != 0.0 {
                eval(then, ctx, store)
            } else {
                eval(els, ctx, store)
            }
        }
        RExpr::Call { fid, args, site } => {
            let Some(fid) = fid_from_u16(*fid) else {
                return Value::Num(f64::NAN);
            };
            if fid == FnId::Latch {
                // latch(cond, x) 惰性原语 (W2): cond 真 → 输出 x 并记忆;
                // cond 假 → 输出上帧值, x **不求值** (内部 sma/prev 状态不污染) —
                // 对齐 Deriver 的 "if (an != 0) 才更新" 条件更新语义
                let c = eval(&args[0], ctx, store).num();
                if c.is_nan() {
                    return Value::Num(f64::NAN);
                }
                if c != 0.0 {
                    let x = eval(&args[1], ctx, store).num();
                    if x.is_nan() {
                        return Value::Num(f64::NAN); // NaN 不污染记忆
                    }
                    let st = store.map.entry(*site).or_insert(PrimState::Prev(0.0));
                    if let PrimState::Prev(p) = st {
                        *p = x;
                    }
                    return Value::Num(x);
                }
                let st = store.map.entry(*site).or_insert(PrimState::Prev(0.0));
                let out = match st {
                    PrimState::Prev(p) => *p,
                    _ => f64::NAN,
                };
                return Value::Num(out);
            }
            if is_stateful(fid) {
                // 状态原语: 先求实参, NaN 输入不污染状态 (设计 §3.6 隔离)
                let vals: Vec<f64> = args.iter().map(|a| eval(a, ctx, store).num()).collect();
                if vals.iter().any(|v| v.is_nan()) {
                    return Value::Num(f64::NAN);
                }
                Value::Num(eval_stateful(fid, &vals, *site, ctx, store))
            } else if is_ctx_fn(fid) {
                // FM 查表族: 需上下文携带的当前 blkx (无 FM → NaN 隔离)
                let vals: Vec<f64> = args.iter().map(|a| eval(a, ctx, store).num()).collect();
                if vals.iter().any(|v| v.is_nan()) {
                    return Value::Num(f64::NAN);
                }
                Value::Num(eval_ctx_fn(fid, &vals, ctx.fm_data))
            } else {
                let vals: Vec<Value> = args.iter().map(|a| eval(a, ctx, store)).collect();
                eval_pure(fid, &vals)
            }
        }
    }
}

fn eval_binary(
    op: BinOp,
    lhs: &RExpr,
    rhs: &RExpr,
    ctx: &EvalCtx,
    store: &mut StateStore,
) -> Value {
    // 逻辑短路: 假短路 && / 真短路 ||; NaN 操作数在非短路路径传播 (错误显形)
    match op {
        BinOp::And => {
            let l = eval(lhs, ctx, store).num();
            if l == 0.0 {
                return Value::Num(0.0);
            }
            let r = eval(rhs, ctx, store).num();
            if l.is_nan() || r.is_nan() {
                return Value::Num(f64::NAN);
            }
            Value::Num((r != 0.0) as u8 as f64)
        }
        BinOp::Or => {
            let l = eval(lhs, ctx, store).num();
            if l != 0.0 && !l.is_nan() {
                return Value::Num(1.0);
            }
            let r = eval(rhs, ctx, store).num();
            if l.is_nan() || r.is_nan() {
                return Value::Num(f64::NAN);
            }
            Value::Num((r != 0.0) as u8 as f64)
        }
        _ => {
            let l = eval(lhs, ctx, store).num();
            let r = eval(rhs, ctx, store).num();
            let v = match op {
                BinOp::Add => l + r,
                BinOp::Sub => l - r,
                BinOp::Mul => l * r,
                BinOp::Div => l / r, // 除零 → IEEE inf/NaN, 不 panic
                BinOp::Mod => l % r,
                BinOp::Pow => l.powf(r),
                BinOp::Eq => (l == r) as u8 as f64,
                BinOp::Ne => (l != r) as u8 as f64,
                BinOp::Lt => (l < r) as u8 as f64,
                BinOp::Le => (l <= r) as u8 as f64,
                BinOp::Gt => (l > r) as u8 as f64,
                BinOp::Ge => (l >= r) as u8 as f64,
                BinOp::And | BinOp::Or => unreachable!("已在上文短路分支处理"),
            };
            Value::Num(v)
        }
    }
}

/// 状态原语求值 (vals 已保证无 NaN)
fn eval_stateful(fid: FnId, vals: &[f64], site: u32, ctx: &EvalCtx, store: &mut StateStore) -> f64 {
    let key = site;
    match fid {
        FnId::Sma => {
            // sma(x, n): 渐进均值 (窗口未满按已有点平均), 位级对齐 SimpleMovingAverage
            let (x, n) = (vals[0], vals[1].max(1.0) as usize);
            let st = store.map.entry(key).or_insert_with(|| PrimState::Sma {
                data: vec![0.0; n],
                cnt: 0,
                avg: 0.0,
            });
            match st {
                PrimState::Sma { data, cnt, avg } => {
                    if *cnt < n {
                        data[*cnt] = x;
                        *cnt += 1;
                        *avg = data[..*cnt].iter().sum::<f64>() / *cnt as f64;
                    } else {
                        let ridx = *cnt % n;
                        *avg += (x - data[ridx]) / n as f64;
                        data[ridx] = x;
                        *cnt += 1;
                    }
                    *avg
                }
                _ => f64::NAN,
            }
        }
        FnId::Prev => {
            // prev(x): 上一帧输入, 初值 0 (对齐 speedvp 语义)
            let st = store.map.entry(key).or_insert(PrimState::Prev(0.0));
            match st {
                PrimState::Prev(p) => {
                    let out = *p;
                    *p = vals[0];
                    out
                }
                _ => f64::NAN,
            }
        }
        FnId::Blend => {
            // blend(x, ratio): out = (1-ratio)*prev + ratio*x, prev 初值 0
            // 对齐 service_loop 的 `ratio_1*x_prev + ratio*x`
            let (x, ratio) = (vals[0], vals[1]);
            let st = store.map.entry(key).or_insert(PrimState::Prev(0.0));
            match st {
                PrimState::Prev(p) => {
                    let out = (1.0 - ratio) * *p + ratio * x;
                    *p = out;
                    out
                }
                _ => f64::NAN,
            }
        }
        FnId::Deriv => {
            // deriv(x): 每秒变化率 (x-prev)*1000/interval_ms
            let st = store.map.entry(key).or_insert(PrimState::PrevT {
                prev: vals[0],
                t: ctx.now_ms,
            });
            match st {
                PrimState::PrevT { prev, t } => {
                    let dt = ctx.now_ms.saturating_sub(*t) as f64;
                    let out = if dt > 0.0 {
                        (vals[0] - *prev) * 1000.0 / dt
                    } else {
                        0.0
                    };
                    *prev = vals[0];
                    *t = ctx.now_ms;
                    out
                }
                _ => f64::NAN,
            }
        }
        FnId::Vote => {
            // vote(up, down, n): 每帧 cnt += up - down; |cnt| >= n 冻结输出 ±1
            // 对齐 check_engine_jet: magenato<0 → -1, 否则 +1, ±100 收敛
            let (up, down, n) = (vals[0] != 0.0, vals[1] != 0.0, vals[2] as i64);
            let st = store.map.entry(key).or_insert(PrimState::Vote {
                cnt: 0,
                frozen: None,
            });
            match st {
                PrimState::Vote { cnt, frozen } => {
                    if let Some(f) = frozen {
                        return *f;
                    }
                    *cnt += (up as i64) - (down as i64);
                    if n > 0 && cnt.abs() >= n {
                        let out = if *cnt >= 0 { 1.0 } else { -1.0 };
                        *frozen = Some(out);
                        return out;
                    }
                    0.0
                }
                _ => f64::NAN,
            }
        }
        FnId::Stable => {
            // stable(x, ms): x 持续不变达 ms → 1 (持续输出), 变化清零
            // 对齐 check_flap "维持1秒稳定" 的计时语义
            let (x, ms) = (vals[0], vals[1]);
            let st = store.map.entry(key).or_insert(PrimState::Stable {
                prev: x,
                held_ms: 0.0,
                has_prev: false,
            });
            match st {
                PrimState::Stable {
                    prev,
                    held_ms,
                    has_prev,
                } => {
                    if *has_prev && x == *prev {
                        *held_ms += ctx.interval_ms;
                    } else {
                        *held_ms = 0.0;
                    }
                    *prev = x;
                    *has_prev = true;
                    (*held_ms >= ms) as u8 as f64
                }
                _ => f64::NAN,
            }
        }
        FnId::LearnMax => {
            // learn_max(x, gate, timeout_ms): gate 真且 x>=cur 时
            // cur = (1-ratio)*cur + ratio*x (ratio=interval_ms/1000, 对齐 service_fields 的 ratio 定义);
            // gate 有效时长累计, 超 timeout_ms 锁定。初值 cur=0 (resetvaria maximumThrRPM 语义位级对拍阶段4校准)
            let (x, gate, timeout_ms) = (vals[0], vals[1] != 0.0, vals[2]);
            let ratio = (ctx.interval_ms / 1000.0).clamp(0.0, 1.0);
            let st = store.map.entry(key).or_insert(PrimState::LearnMax {
                cur: 0.0,
                elapsed_ms: 0.0,
                locked: false,
            });
            match st {
                PrimState::LearnMax {
                    cur,
                    elapsed_ms,
                    locked,
                } => {
                    if !*locked && gate {
                        if x >= *cur {
                            *cur = (1.0 - ratio) * *cur + ratio * x;
                        }
                        *elapsed_ms += ctx.interval_ms;
                        if *elapsed_ms >= timeout_ms {
                            *locked = true;
                        }
                    }
                    *cur
                }
                _ => f64::NAN,
            }
        }
        _ => f64::NAN, // 非状态原语防御兜底
    }
}

/// FM 查表族求值 (当前 blkx 由上下文携带)。
/// 语义: vne/mne/aoa_high 无 FM → NaN 隔离; flap 两函数走共享实现的业务默认
/// (0 级/无 FM → MAX/125), 与被替代代码位级一致。
/// 共享实现里的 unwrap panic (Java NPE 保真) 以 catch_unwind 收敛为 NaN —
/// 生产 READY 链表恒 Some 不可达, 防御仅为公式用户免崩 Service 线程。
fn eval_ctx_fn(fid: FnId, vals: &[f64], fmdata: Option<&crate::fm::data::FmData>) -> f64 {
    use FnId as F;
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match fid {
        F::FmFlapAllowSpeed => {
            crate::fm::data::get_flap_allow_speed(vals[0] as i32, vals[1] != 0.0, fmdata)
        }
        F::FmFlapAllowAngle => {
            crate::fm::data::get_flap_allow_angle(vals[0], vals[1] != 0.0, fmdata)
        }
        _ => {
            let Some(b) = fmdata else { return f64::NAN };
            match fid {
                F::FmVne => b.get_vne_v_wing(vals[0]),
                F::FmMne => b.get_mne_v_wing(vals[0]),
                F::FmAoaHigh => b.get_aoa_high_v_wing(vals[0], vals[1] as i32),
                _ => f64::NAN,
            }
        }
    }));
    r.unwrap_or(f64::NAN)
}
