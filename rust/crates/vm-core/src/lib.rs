//! vm-core: VoidMei 纯逻辑层 (Java src/prog/util + src/parser 的一比一翻译目标)
//! POC 期已移植的模块在此汇合; A 类翻译流水线产物落在本 crate。

// PhysicsConstants.g 的转发 (单一来源在 physics_constants, 试运行审查裁决)
pub use physics_constants::{g, G};

pub mod activation_strategy;
pub mod atmosphere_model;
pub mod voice_warning;
pub mod voice_resource_manager;
pub mod row_renderer_registry;
pub mod renderer_config_helper;
pub mod reflect_binder;
pub mod hud_layout_node;
pub mod overlay_context;
pub mod other_service;
pub mod hud_calculator;
pub mod http_helper;
pub mod focus_monitor;
pub mod flight_log;
pub mod flight_analyzer;
pub mod ui_state_bus;
pub mod flight_data_bus;
pub mod config_loader;
pub mod config_manager;
pub mod config_watcher;
pub mod configuration_service;
pub mod exception_helper;
pub mod logger;
pub mod bus;
pub mod blkx;
pub mod audio;
pub mod comparison;
pub mod config_api;
pub mod controller_state;
pub mod hud_data;
pub mod lang;
pub mod parser;
pub mod ui_constants;
pub mod ui_model;
pub mod visibility_expression;
pub mod calc_helper;
pub mod event;
pub mod file_utils;
pub mod fm;
pub mod fm_power_extractor;
pub mod power_curve_helper;
pub mod piston_power_model;
pub mod sexp_parser;
pub mod fields;
pub mod format;
pub mod interpolation;
pub mod layout;
pub mod physics_constants;
pub mod string_helper;
