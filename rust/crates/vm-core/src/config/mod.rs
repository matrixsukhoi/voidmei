//! 配置域: ui_layout.cfg 装载 (config_loader + key_text 键码映射)/S 表达式解析/
//! 双文件合并迁移 (config_manager + md5 + ui_state_storage 桩)/文件监视/门面服务。

pub mod app_state;
pub mod config_api;
pub mod config_loader;
pub mod config_manager;
pub mod config_watcher;
pub mod configuration_service;
pub mod key_text;
pub mod md5;
pub mod sexp_parser;
pub mod ui_state_storage;
