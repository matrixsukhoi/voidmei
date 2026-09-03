//! vm-ui lib 入口 (D9 后职责 = MainForm **表单数据层**)。
//!
//! D9 决策: 设置窗 (MainForm) 换 Tauri 2 web 壳 (vm-webui + vm-app dispatcher 粘合),
//! 原 iced view 层已整体删除。本 crate 只保留纯数据面:
//! - [`main_form`] — Message/MainFormState/update 写链 (JSON 配置变更后直调
//!   ConfigurationService, 无窗口验收工具在 main_form::headless)
//! - [`renderers`] — 行定位助手 + combo 选项解析 (写链与 PropertyBinder 已随
//!   JSON 配置化收敛至 main_form)
//!
//! 表单渲染 (HTML/JS) 归 vm-webui web 壳; 依赖方向不变: vm-app → vm-ui (数据层)。

pub mod main_form;
pub mod renderers;
