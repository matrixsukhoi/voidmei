//! vm-ui lib 入口 (D9 后职责 = MainForm **表单数据层**)。
//!
//! D9 决策: 设置窗 (MainForm) 换 Tauri 2 web 壳 (vm-webui + vm-app dispatcher 粘合),
//! 原 D1 批次的 iced 0.13 view 层 (run_shell_form/MainFormHooks/update_app/view_app
//! 及各渲染器 view_row) 已整体删除。本 crate 只保留纯数据面:
//! - [`main_form`]: Message/MainFormState/update 写回链 + persist_and_notify 落盘
//!   + run_headless 无窗口状态机驱动
//! - [`renderers`]: 各行类型的 apply 写回链、纯数据函数与行定位助手
//!
//! 表单渲染 (HTML/JS) 归 vm-webui web 壳; 依赖方向不变: vm-app → vm-ui (数据层)。

pub mod main_form;
pub mod renderers;
