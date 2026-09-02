//! vm-core: VoidMei 纯逻辑层。
//! 重构波2 起按域分组 (原 47 个平铺模块 → 8 域 + 4 根留);
//! 根部 `pub use` shim 保 `crate::x` / `vm_core::x` 旧路径全部有效。
//! UI 面 (renderer_config_helper/row_renderer_registry → vm-ui;
//! ui_model/hud_layout_node/layout/ui_constants → vm-overlay) 已随本波下沉消费 crate。

// PhysicsConstants.g 的转发 (单一来源在 base::physics_constants, 试运行审查裁决)
pub use base::physics_constants::{g, G};

// ---- 域模块 ----
pub mod base;      // 总线/事件/日志/通用工具/插值/物理常量
pub mod config;    // 配置栈: 装载/S 表达式/合并迁移/监视/门面/总线
pub mod telemetry; // 8111 HTTP 客户端 + 遥测解析器
pub mod fm;        // FM 管理栈 + FM 数据 (fmdata) + 功率模型族
pub mod formula;   // 公式系统 (L0 registry/L1 编译/L2 规则引擎)
pub mod derived;   // HUD 派生量/飞行分析/日志/事件总线/慢速轮询
pub mod audio;     // 语音告警判定/资源管理/告警类型
pub mod uisupport; // 双消费 UI 支撑 (行定义/机型对比)
pub mod platform;  // 平台检测 (游戏失焦)

// ---- 根留 (无域归属的顶层小件) ----
pub mod activation_strategy;
pub mod atmosphere_model;
pub mod controller_state;
pub mod lang;
pub mod overlay_context;

// ---- 根 re-export shim (旧 crate::x 路径的兼容面 = 有策展的公共 API) ----
pub use audio::{voice_resource_manager, voice_warning};
pub use base::{
    bus, calc_helper, event, exception_helper, file_utils, format, interpolation, logger,
    physics_constants, string_helper,
};
pub use config::{
    app_state, config_api, config_loader, config_manager, config_watcher,
    configuration_service, sexp_parser, ui_state_bus,
};
pub use derived::{
    flight_analyzer, flight_data_bus, flight_log, hud_calculator, hud_data, other_service,
};
pub use fm::{fm_power_extractor, fmdata, piston_power_model, power_curve_helper};
pub use platform::focus_monitor;
pub use telemetry::{http_helper, parser};
pub use uisupport::{comparison, row_def};
