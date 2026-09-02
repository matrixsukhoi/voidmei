//! vm-core: VoidMei 纯逻辑层, 11 域分组 (波8 定稿, 根留清零; 波9 shim 退役,
//! 全库统一 `crate::<域>::<模块>` / `vm_core::<域>::<模块>` 唯一路径):
//! base(总线家族/事件/日志/工具/物理/大气) / config(配置栈) / telemetry(HTTP+解析)
//! / fm(管理栈+data+功率族) / formula(公式系统) / derived(HUD 派生) / audio(语音)
//! / ui_support(行定义+机型对比) / platform(平台检测) / lang(i18n) / activation(激活)。

// ---- 域模块 (11) ----
pub mod activation; // overlay 激活: 谓词组合 (strategy) + 渲染上下文 (context)
pub mod audio;      // 语音告警判定/资源管理/告警类型
pub mod base;       // 总线家族/事件类型/日志/通用工具/插值/物理常量/标准大气
pub mod config;     // 配置栈: 装载/S 表达式/合并迁移/监视/门面
pub mod derived;    // HUD 派生量/飞行分析/日志/地图消息慢速轮询
pub mod fm;         // FM 管理栈 + FM 数据 (data) + 功率模型族
pub mod formula;    // 公式系统 (L0 registry/L1 编译/L2 规则引擎/manager)
pub mod lang;       // i18n 文本表 (波8 自根留升域)
pub mod platform;   // 平台检测 (游戏失焦)
pub mod telemetry;  // 8111 HTTP 客户端 + 遥测解析器
pub mod ui_support; // 双消费 UI 支撑 (行定义/机型对比; 波8 uisupport 更名)
