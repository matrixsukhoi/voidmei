//! 公式系统: 内置+自定义数学公式的统一承载 (设计: doc/formula_system_design.md)。
//!
//! 三层: registry(L0 原子变量) → definition/eval(L1 公式 DAG 求值) → rules(L2, 阶段5)。
//! 求值收敛 Service 线程单点 (裁决 A1); 引擎全静态注册, 零反射零新依赖。

pub mod ast;
pub mod definition;
pub mod eval;
pub mod functions;
pub mod lexer;
pub mod manager;
pub mod parser;
pub mod persistence;
pub mod registry;
pub mod rules;

pub use definition::{CompiledFormulaSet, FormulaDef, FormulaResults, VarLookup};
pub use eval::{EvalCtx, StateStore};
pub use functions::{FnId, Value};
pub use manager::{resolve_target, target_value, FormulaManager, TargetVar};
pub use registry::{assemble_snapshot, registry, MetaInputs, Registry, VarSnapshot};

#[cfg(test)]
mod tests;
