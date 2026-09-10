//! 激活域: overlay 激活条件谓词组合 (strategy) + 渲染上下文聚合 (context)。
//! 波8 自根留组建 — 二者同主题: context 是 strategy 消费的渲染期聚合载体。
//! (context 生产消费面当前为空, 属迁移预留的休眠模块。)

pub mod context;
pub mod strategy;

pub use strategy::ActivationStrategy;
