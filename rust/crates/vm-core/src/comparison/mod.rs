//! ui.window.comparison.logic 包 (A 类, CLASSIFY.md §19): FM 属性对比的
//! 规则提取 (ComparisonRule/ComparisonRules + rules/*) 与胜负计算
//! (ComparisonCalculator)。
//! 窗口类 (CompactComparisonWindow 等) 属 C 类/后续批次, 不在本模块。
//! 同包 model/ComparisonData.java **亦为 A 类** (CLASSIFY.md §19, 36 行, 纯数据
//! 结构), 只是不在本批 logic/ 翻译范围 —— 待 A 类放量批次补译, 勿漏。
//!
//! PORT: Java 包名 ui.window.comparison.logic → crate::comparison (PORTING.md
//! §0.6 子域目录); 包内类型在 mod 根 re-export 镜像 (ui_model 同款先例)。

pub mod comparison_calculator;
pub mod comparison_rule;
pub mod comparison_rules;
pub mod rules;

pub use comparison_calculator::{ComparisonCalculator, DiffResult, WinState};
pub use comparison_rule::ComparisonRule;
pub use comparison_rules::ComparisonRules;
pub use rules::{LambdaRule, ListIndexRule, MultiListIndexRule, SimpleRule};
