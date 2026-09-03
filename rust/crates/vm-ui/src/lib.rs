//! vm-ui lib 入口 (D9 后职责 = MainForm **表单数据层**)。
//!
//! D9 决策: 设置窗 (MainForm) 换 Tauri 2 web 壳 (vm-webui + vm-app dispatcher 粘合),
//! 原 iced view 层已整体删除。本 crate 只保留纯数据面:
//! - [`main_form`] — Message/MainFormState/update 写回链 + persist_and_notify 落盘
//!   (无窗口验收工具在 main_form::headless, E10 提离核心)
//! - [`renderers`] — 各行类型的 apply 写回链、纯数据函数与行定位助手
//! - [`render_context`] — 写链上下文契约 (RenderContext trait, 生产实现 =
//!   main_form::WriteContext)
//! - [`renderer_config_helper`] — 配置读写助手 (PropertyBinder 注册表 + 服务同步)
//!
//! 表单渲染 (HTML/JS) 归 vm-webui web 壳; 依赖方向不变: vm-app → vm-ui (数据层)。

pub mod main_form;
pub mod render_context;
pub mod renderers;
// 重构波2 自 vm-core 下沉 (本 crate 唯一消费的设置面板支撑面)
pub mod renderer_config_helper;
