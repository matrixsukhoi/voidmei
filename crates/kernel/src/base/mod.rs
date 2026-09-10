//! 基座域: 总线家族/事件类型/日志/异常助手/文件工具/插值/物理常量/标准大气模型/
//! JDK 标准库语义复刻 (java_compat) / Java printf 引擎 (format::java_printf)。
//! (波20 serde 化: string_helper 手写 JSON 取数层已随 parser 重写退役)

pub mod atmosphere_model;
pub mod bus;
pub mod calc_helper;
pub mod engine_type;
pub mod event;
pub mod exception_helper;
pub mod file_utils;
pub mod format;
pub mod interpolation;
pub mod java_compat;
pub mod logger;
pub mod physics_constants;
pub mod ports;
