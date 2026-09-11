//! L2 变量动作规则: 阈值表达式 + 持续(hold)/冷却(cooldown) → 动作事件。
//! 设计: doc/formula_system_design.md §9。
//!
//! 定位: 引擎产出触发**事件流** (数据面), 消费面 (toast/语音播放) 由 voidmei
//! 侧接线 — 本模块零 UI 依赖, 全单测覆盖 hold/cooldown 时序。
//!
//! 与 VoiceWarning 的关系: VoiceWarning 的 ~17 条 check_* 判定 (动态阈值/
//! 多条件/Service 状态联动) 回归面大, 其外置为出厂规则归后续批次
//! (设计 §13 遗留); 本引擎先承载用户自定义规则。

use super::ast::RExpr;
use super::definition::{FormulaResults, VarLookup};
use super::eval::{eval, EvalCtx};
use super::registry::VarSnapshot;

/// 动作 (触发事件载荷; 消费方按 kind 分派)
#[derive(Debug, Clone, PartialEq)]
pub enum RuleAction {
    /// 语音资源 key (VoiceWarning 的 wav key)
    Voice(String),
    /// toast 文案
    Toast(String),
    /// 具名标志 (供 overlay 变色引用, 并存语义 — 设计 §9 D4)
    Flag(String),
}

/// 规则定义 (持久化形态, formulas.cfg 的 (rule ...) 段)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RuleDef {
    pub name: String,
    /// 触发条件 (公式语言表达式, 可引用变量与公式)
    pub when: String,
    /// 条件持续 hold_ms 才触发 (0 = 立即)
    pub hold_ms: f64,
    /// 触发后冷却冷却秒数 (冷却期条件再真不重复触发)
    pub cooldown_s: f64,
    pub actions: Vec<RuleAction>,
    pub disabled: bool,
}

/// 触发事件 (formula_step 产出, ServiceData.rule_triggers 落地)
#[derive(Debug, Clone, PartialEq)]
pub struct RuleTriggered {
    pub rule: String,
    pub action: RuleAction,
    pub at_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
enum RuleError {
    Parse(String),
}

struct CompiledRule {
    def: RuleDef,
    cond: Result<RExpr, RuleError>,
}

#[derive(Debug, Clone, PartialEq)]
enum RuleState {
    /// 未触发 (held_ms 累计中)
    Idle { held_ms: f64 },
    /// 已触发: 冷却截止 + 条件持续累计 (出冷却即判, 不丢持续语义)
    Cooling { until_ms: u64, held_ms: f64 },
}

/// 规则引擎: 编译 (复用公式语言) + 帧求值 (hold/cooldown 状态机)
pub struct RuleEngine {
    rules: Vec<CompiledRule>,
    states: Vec<RuleState>,
}

impl RuleEngine {
    pub fn new() -> Self {
        RuleEngine {
            rules: Vec::new(),
            states: Vec::new(),
        }
    }

    /// 编译规则集 (when 表达式走公式编译链; 坏规则隔离不阻断)
    pub fn install(&mut self, defs: &[RuleDef], reg: &dyn VarLookup) {
        self.rules = defs
            .iter()
            .filter(|d| !d.disabled && !d.name.is_empty())
            .map(|d| CompiledRule {
                def: d.clone(),
                cond: compile_when(&d.when, reg),
            })
            .collect();
        self.states = vec![RuleState::Idle { held_ms: 0.0 }; self.rules.len()];
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// 冷却/持_invalid 状态全清 (换机/会话重置, 对齐公式状态原语重置语义)
    pub fn reset(&mut self) {
        self.states = vec![RuleState::Idle { held_ms: 0.0 }; self.rules.len()];
    }

    /// 帧求值: 快照+公式结果 → 触发事件 (formula_step 内, 公式求值之后)
    pub fn eval(
        &mut self,
        snap: &VarSnapshot,
        results: &FormulaResults,
        now_ms: u64,
        interval_ms: f64,
    ) -> Vec<RuleTriggered> {
        let mut out = Vec::new();
        for (i, r) in self.rules.iter().enumerate() {
            let cond = match &r.cond {
                Ok(c) => c,
                Err(_) => continue, // 坏规则隔离
            };
            // 规则条件暂不含 FM 查表函数 (阶段 5 后续), fm_blkx=None
            let ctx = EvalCtx {
                snap,
                results,
                now_ms,
                interval_ms,
                fm_data: None,
            };
            let v = eval(cond, &ctx, &mut super::eval::StateStore::new()).num();
            // 条件 NaN = 不可判定, 视为假 (不累计不触发)
            let active = !v.is_nan() && v != 0.0;
            let state = self.states[i].clone();
            let cd_until = now_ms + (r.def.cooldown_s * 1000.0) as u64;
            let next = match state {
                RuleState::Cooling { until_ms, held_ms } => {
                    let h = if active { held_ms + interval_ms } else { 0.0 };
                    if now_ms < until_ms {
                        // 冷却期: 条件持续累计 (出冷却即判), 不触发
                        RuleState::Cooling {
                            until_ms,
                            held_ms: h,
                        }
                    } else if active && h >= r.def.hold_ms {
                        self.fire(r, now_ms, &mut out);
                        RuleState::Cooling {
                            until_ms: cd_until,
                            held_ms: 0.0,
                        }
                    } else {
                        RuleState::Idle { held_ms: h }
                    }
                }
                RuleState::Idle { held_ms } => {
                    if !active {
                        RuleState::Idle { held_ms: 0.0 }
                    } else {
                        let held = held_ms + interval_ms;
                        if held >= r.def.hold_ms {
                            self.fire(r, now_ms, &mut out);
                            RuleState::Cooling {
                                until_ms: cd_until,
                                held_ms: 0.0,
                            }
                        } else {
                            RuleState::Idle { held_ms: held }
                        }
                    }
                }
            };
            self.states[i] = next;
        }
        out
    }

    fn fire(&self, r: &CompiledRule, now_ms: u64, out: &mut Vec<RuleTriggered>) {
        for a in &r.def.actions {
            out.push(RuleTriggered {
                rule: r.def.name.clone(),
                action: a.clone(),
                at_ms: now_ms,
            });
        }
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// when 表达式编译 (复用公式 parser/resolve; 公式引用不可用 — 单表达式命名空间)
fn compile_when(src: &str, reg: &dyn VarLookup) -> Result<RExpr, RuleError> {
    let mut next_site = 0u32;
    let mut sites = Vec::new();
    // 空公式槽表: when 里只能引用注册表变量 (公式引用属阶段 5 后续 — 规则间/
    // 公式间引用需统一 DAG, 现阶段语义边界: 变量阈值规则)
    let slots = std::collections::HashMap::new();
    let (expr, _) = super::parser::parse(src).map_err(RuleError::Parse)?;
    super::definition::resolve_rexpr_public(&expr, reg, &slots, &mut next_site, &mut sites)
        .map_err(|e| RuleError::Parse(e.to_string()))
}
