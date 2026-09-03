//! 配置域: ui_layout.cfg 装载 (config_loader + key_text 键码映射)/S 表达式解析/
//! 双文件合并迁移 (config_manager + ui_state_storage 桩)/文件监视/门面服务。 (波21: 手写 md5 已换 md-5 crate)

pub mod app_state;
pub mod config_api;
pub mod config_loader;
pub mod config_manager;
pub mod config_watcher;
pub mod configuration_service;
pub mod key_text;
pub mod sexp_parser;
pub mod ui_state_storage;
