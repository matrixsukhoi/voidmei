//! vm-ui: P5 MainForm (iced + tiny-skia 软件渲染, D1 决策)。
//!
//! 入口: 读真实 ui_layout.cfg 构建数据驱动设置表单; `--headless` 跳过窗口只做
//! 状态机测试 (无落盘副作用)。窗口打开由人工验收, 运行日志确认表单构建成功。
//! 注: view/update 用具名函数 (D1 坑位: 闭包触发高阶生命周期推断失败)。

mod main_form;
mod renderers;

use std::sync::Arc;

use vm_core::bus::EventBus;
use vm_core::config_manager;
use vm_core::configuration_service::ConfigurationService;

use crate::main_form::{MainFormState, Message};

fn main() -> iced::Result {
    if std::env::args().any(|a| a == "--headless") {
        std::process::exit(main_form::run_headless());
    }

    // 生产链 (对位 Java Controller: configService.initConfig → MainForm(c)):
    // config_manager 首次运行拷模板/模板变更合并 — CWD=仓库根时完整生效
    let bus = Arc::new(EventBus::new());
    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.init_config();
    if config.get_layout_configs().is_none_or(|g| g.is_empty()) {
        // CWD 无 ./ui_layout.cfg (如自 rust/ 启动) → 回退直读仓库模板。
        // PORT(分歧备案): Java 此时显示 "Data (Empty)" 占位页不落盘; 本侧以模板
        // 自愈表单, 且 persist_path 仍指向用户路径 → 首次保存会把模板内容覆写到
        // 空/损坏的用户配置 (有意的恢复路径, Java 无此行为)
        if let Some(p) = main_form::locate_template_cfg() {
            eprintln!("vm-ui: CWD 无 ./ui_layout.cfg, 回退加载 {p}");
            config.load_layout(p);
        } else {
            eprintln!("vm-ui: 未找到 ui_layout.cfg, 表单为空 (Data (Empty) 占位)");
        }
    }

    let state = MainFormState::new(
        config,
        bus,
        Some(config_manager::get_user_config_path().to_string()),
    );
    println!(
        "vm-ui: 表单构建成功: {} panels / {} rows — 打开窗口 (人工验收)",
        state.panel_count(),
        state.row_count()
    );

    // D1 坑位: 完整 turbofish + 具名 view/update 函数; run_with 注入构造完的状态
    // (run() 要求 State: Default — MainForm 构造必带配置服务, 不造伪默认)
    iced::application::<MainFormState, Message, iced::Theme, iced::Renderer>(
        "VoidMei 设置 — iced MainForm",
        main_form::update,
        main_form::view,
    )
    .theme(|_| iced::Theme::default())
    .window(iced::window::Settings {
        // Java: width = min(800, logicalWidth - 40) (MainForm.java:294); 屏幕探测属
        // 组装层 (DPI 批次), 此处取固定近似 800 (Java 上限) x 620
        size: iced::Size::new(800.0, 620.0),
        ..Default::default()
    })
    .run_with(move || (state, iced::Task::none()))
}
