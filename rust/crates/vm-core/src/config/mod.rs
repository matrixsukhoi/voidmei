//! 配置域: JSON 模型 (json_model) + 出厂/用户 delta 存储 (json_store) +
//! S 表达式解析 (sexp_parser — formulas.cfg 外壳的唯一解析器, 保留) +
//! 应用态桩 (app_state) + 门面服务。
//!
//! 历史: ui_layout.cfg S-expr 体系 (config_loader/config_manager/
//! ui_state_storage/config_watcher/key_text) 已整体迁移至 JSON (Phase 1) —
//! 出厂默认内嵌 factory_default.json, 用户差异落 voidmei_config.json。

pub mod app_state;
pub mod config_api;
pub mod configuration_service;
pub mod json_model;
pub mod json_store;
pub mod sexp_parser;
