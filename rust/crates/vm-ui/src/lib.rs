//! vm-ui lib 入口 (组装批): MainForm 运行面对 vm-app 暴露。
//!
//! vm-app (组装 bin) 经 [`run_shell_form`] 在主线程拉起 iced MainForm 窗口 (D8 线程
//! 拓扑: 主线程 = iced 消息循环), 并以 [`MainFormHooks`] 注入 shell 侧回调 —
//! vm-ui 不依赖 vm-app (依赖方向单向: vm-app → vm-ui)。
//!
//! **周期泵** (对位 Java EDT 常驻事件线程): iced 的 subscription 需要 iced_futures
//! 执行器, 默认 (无 tokio/async-std/smol feature) 是 null 后端 — spawn 直接丢
//! future、`iced::time` 为空模块。因此本 crate 的 iced 依赖启用 `tokio` feature
//! (rt+time 拉起 every 流), 50ms `Tick` 消息驱动 [`MainFormHooks::on_tick`] —
//! vm-app 在其中执行 `AppShell::pump` (监督事件 + drive_from_live 状态机)。
//!
//! **窗口生命周期相位** (对位 Java):
//! - MainForm.confirm (MainForm.java:265-278) = `setVisible(false)` + `tc.start()`
//!   → 本侧 StartGame 消息处理后 `iced::exit()` 关窗, 后续 (游戏模式监督循环) 归
//!   vm-app 主循环。
//! - 底部 mCancel (:92-98) = saveConfig + System.exit(0) → EndGame 消息 → 回调
//!   dispatch(EndGame) (置退出请求) + exit。
//! - 窗口 X (Java setDefaultCloseOperation(3)=DISPOSE) → iced run 返回, 应用继续
//!   (托盘存活), 相位切换同样归 vm-app 主循环。

pub mod main_form;
pub mod renderers;

use std::time::Duration;

use iced::{Element, Task};

pub use main_form::{MainFormState, Message};

/// 组装层注入的 shell 回调 (全部在 iced 主线程调用; 闭包由 vm-app 持有 AppShell)
pub struct MainFormHooks {
    /// 构造期一次 (对位 Java MainForm 首次显示 → UIStateBus.publish(UI_READY),
    /// Controller.uiReadyHandler → Preview 的触发面)。
    pub on_ready: Box<dyn FnMut()>,
    /// "开始游戏" (MainForm.confirm 的 tc 侧序列归属; 本侧已先落保存链)
    pub on_start_game: Box<dyn FnMut()>,
    /// "结束游戏" (mCancel; 本侧已先落保存链)
    pub on_end_game: Box<dyn FnMut()>,
    /// 50ms 周期泵 (AppShell::pump); 返回 true = 请求关窗
    /// (托盘重建/退出请求 — 相位切换归 vm-app 主循环)
    pub on_tick: Box<dyn FnMut() -> bool>,
}

/// 包装消息: 表单原生消息 + 周期泵
#[derive(Debug, Clone)]
pub enum AppMessage {
    Form(Message),
    Tick,
}

/// iced 状态 = 表单状态 + shell 回调 (State 仅需 'static — ConfigurationService
/// 含 Rc 树 !Send, iced::application 无 Send 约束, 恒留主线程)
pub struct MainFormApp {
    form: MainFormState,
    hooks: MainFormHooks,
}

impl MainFormApp {
    pub fn new(form: MainFormState, mut hooks: MainFormHooks) -> Self {
        // UI_READY 在构造点发布 (窗口即将首显; shell.pump 消费该事件进 Preview)
        (hooks.on_ready)();
        MainFormApp { form, hooks }
    }

    /// 表单状态只读访问 (组装层构建期诊断)
    pub fn form(&self) -> &MainFormState {
        &self.form
    }
}

/// iced update (D1: 具名函数, 闭包触发高阶生命周期推断失败)
pub fn update_app(state: &mut MainFormApp, message: AppMessage) -> Task<AppMessage> {
    match message {
        // Java MainForm.confirm: 保存链在 MainForm 侧 (main_form::update),
        // tc 侧序列 (endPreview/saveConfig/loadFromConfig/start) 归 on_start_game
        AppMessage::Form(Message::StartGame) => {
            main_form::update(&mut state.form, Message::StartGame);
            (state.hooks.on_start_game)();
            iced::exit() // 对位 confirm 的 setVisible(false): 关窗, 相位归主循环
        }
        // Java mCancel: saveConfig + tc.saveConfig + System.exit(0) — 进程退出由
        // shell.is_exit_requested → 主循环 shutdown 承担 (比裸 exit 多线程收尾)
        AppMessage::Form(Message::EndGame) => {
            main_form::update(&mut state.form, Message::EndGame);
            (state.hooks.on_end_game)();
            iced::exit()
        }
        AppMessage::Form(m) => {
            main_form::update(&mut state.form, m);
            Task::none()
        }
        AppMessage::Tick => {
            let close = (state.hooks.on_tick)();
            if close { iced::exit() } else { Task::none() }
        }
    }
}

/// iced view (透传 main_form::view, 消息映射回包装层)
pub fn view_app(state: &MainFormApp) -> Element<'_, AppMessage> {
    main_form::view(&state.form).map(AppMessage::Form)
}

/// MainForm 默认字体家族 (按平台选系统 CJK 字体)。
///
/// 对位 Java: Swing/WebLaF 用逻辑字体 (Dialog/SansSerif), JDK fontconfig 自动
/// 映射到系统微软雅黑 → 中文从不缺字。**cfg 里的 Sarasa Mono SC 属 overlay/HUD
/// 渲染** (vm-overlay 经 swash 从 fonts/ 加载), 与 MainForm 表单字体无关 (Java
/// MainForm 同样不读 cfg 字体)。
///
/// tofu 根因 (三层叠加, 修复前从未设置任何字体):
/// 1. iced Font::DEFAULT = "Fira Sans", 本 crate default-features=false 未打包
///    Fira 字节 → fontdb 查无此家族, 命中任意默认字体 (通常无 CJK);
/// 2. iced 0.13 widget 文本默认 Shaping::Basic — cosmic-text Basic 路径**不做
///    字体回退** (Advanced 才做, 且只能逐 widget 显式开, 无全局开关);
/// 3. 系统字体其实已进 fontdb (load_system_fonts 无条件调用), Basic 到不了。
/// → 显式指定自身含 CJK 的系统字体, Basic shaping 直接命中, 无需回退。
const PLATFORM_CJK_FONT: &str = {
    // cosmic-text 自身 Windows 简中回退同款 (fallback/windows.rs han_unification)
    #[cfg(windows)]
    {
        "Microsoft YaHei UI"
    }
    #[cfg(target_os = "linux")]
    {
        "Noto Sans CJK SC"
    }
    #[cfg(target_os = "macos")]
    {
        "PingFang SC"
    }
};

/// MainForm 默认字体 (拉丁+CJK 单字体覆盖; Java 为 Segoe UI+系统回退, 视觉近似)
pub fn platform_default_font() -> iced::Font {
    iced::Font::with_name(PLATFORM_CJK_FONT)
}

/// 组装层入口: 构造完成的表单状态 + shell 回调 → iced 主循环 (阻塞至关窗/退出)。
///
/// PORT(winit 复跑备案): run 返回后同进程可再次调用 (托盘 Activate → 重开设置窗
/// 的相位循环); winit 0.30 在 Windows/Linux 支持顺序重建事件循环 (macOS 不支持,
/// 非 P5 目标平台)。失败返回 Err, 由调用方降级托盘模式。
pub fn run_shell_form(form: MainFormState, hooks: MainFormHooks) -> iced::Result {
    iced::application::<MainFormApp, AppMessage, iced::Theme, iced::Renderer>(
        "VoidMei 设置 — iced MainForm",
        update_app,
        view_app,
    )
    .theme(|_| iced::Theme::default())
    // 系统中文字体 (见 PLATFORM_CJK_FONT 头注 — 不设则 Basic shaping 中文全 tofu)
    .default_font(platform_default_font())
    .window(iced::window::Settings {
        // Java: width = min(800, logicalWidth - 40) (MainForm.java:294); 与 bin
        // 入口同款固定近似 800 (Java 上限) x 620
        size: iced::Size::new(800.0, 620.0),
        ..Default::default()
    })
    // 50ms 泵 (Java EDT 事件驱动 + Service 10Hz 轮询的合成节拍; 见模块头注)
    .subscription(|_| {
        iced::time::every(Duration::from_millis(50)).map(|_| AppMessage::Tick)
    })
    .run_with(move || (MainFormApp::new(form, hooks), Task::none()))
}

// =====================================================================
// Tests — hooks 接线协议 (无需窗口: 直接驱动 update_app)
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn form_state() -> MainFormState {
        // tmp 配置 (无落盘副作用): 单开关面板, 覆盖消息链最小面
        let bus = std::sync::Arc::new(vm_core::bus::EventBus::new());
        let config = vm_core::configuration_service::ConfigurationService::new(Some(bus));
        let dir = std::env::temp_dir().join(format!("vm_ui_lib_{}.cfg", std::process::id()));
        std::fs::write(
            &dir,
            "(panel \"T\" :visible true\n (item \"hud\" :type switch :target \"crosshairSwitch\" :value true))\n",
        )
        .unwrap();
        config.load_layout(dir.to_str().unwrap());
        MainFormState::new(config, std::sync::Arc::new(vm_core::bus::EventBus::new()), None)
    }

    fn counting_hooks() -> (MainFormHooks, Arc<Mutex<Vec<&'static str>>>) {
        let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
        let hooks = MainFormHooks {
            on_ready: {
                let l = Arc::clone(&log);
                Box::new(move || l.lock().unwrap().push("ready"))
            },
            on_start_game: {
                let l = Arc::clone(&log);
                Box::new(move || l.lock().unwrap().push("start"))
            },
            on_end_game: {
                let l = Arc::clone(&log);
                Box::new(move || l.lock().unwrap().push("end"))
            },
            on_tick: {
                let l = Arc::clone(&log);
                Box::new(move || {
                    l.lock().unwrap().push("tick");
                    false
                })
            },
        };
        (hooks, log)
    }

    /// 构造即发 on_ready (UI_READY 对位); StartGame/EndGame 先表单更新链再回调
    #[test]
    fn hooks_fire_in_java_order() {
        let (hooks, log) = counting_hooks();
        let mut app = MainFormApp::new(form_state(), hooks);
        assert_eq!(*log.lock().unwrap(), vec!["ready"], "构造期 UI_READY");

        let _ = update_app(&mut app, AppMessage::Form(Message::StartGame));
        assert_eq!(*log.lock().unwrap(), vec!["ready", "start"]);

        let _ = update_app(&mut app, AppMessage::Form(Message::EndGame));
        assert_eq!(*log.lock().unwrap(), vec!["ready", "start", "end"]);
    }

    /// Tick 泵驱动 + on_tick 返回 true 语义 (关窗请求由 Task 表达, 不 panic 即通)
    #[test]
    fn tick_pump_closes_when_requested() {
        let (hooks, log) = counting_hooks();
        let mut app = MainFormApp::new(form_state(), hooks);
        let _ = update_app(&mut app, AppMessage::Tick);
        assert_eq!(log.lock().unwrap().last().copied(), Some("tick"));

        // on_tick=true 路径 (Task 形态无断言面, 不 panic + 日志顺序即可)
        let flag = Arc::new(Mutex::new(true));
        let f2 = Arc::clone(&flag);
        app.hooks.on_tick = Box::new(move || *f2.lock().unwrap());
        let _ = update_app(&mut app, AppMessage::Tick);
    }

    /// 普通表单消息透传 main_form::update (Toggle 落服务树, WYSIWYG 链不因包装断)
    #[test]
    fn form_messages_delegate_to_main_form() {
        let (hooks, _log) = counting_hooks();
        let mut app = MainFormApp::new(form_state(), hooks);
        // panel 名以解析树为准 (组标题经 Lang 解析, 不能按 cfg 原串假设)
        let (panel, key) = app
            .form()
            .first_row_of_type("SWITCH")
            .expect("cfg 应含 SWITCH 行");
        let before = app.form().service_string(&key);
        let _ = update_app(
            &mut app,
            AppMessage::Form(Message::Toggle {
                panel,
                key: key.clone(),
                value: !before.eq_ignore_ascii_case("true"),
            }),
        );
        let after = app.form().service_string(&key);
        assert_ne!(before, after, "Toggle 应经透传落服务树");
    }

    /// 默认字体必须真实命中系统字体库且含汉字 — 否则 Basic shaping 中文全 tofu
    /// (fontdb 由 iced 全局 FontSystem 持有, load_system_fonts 无条件执行, 可直接查)。
    /// 仅 Windows 硬断言: 当前唯一已接线平台 (X11 属 D8 遗留, 无 CJK 字体的
    /// headless Linux 失败属环境事实而非代码缺陷)。
    #[cfg(windows)]
    #[test]
    fn platform_default_font_resolves_with_cjk() {
        use iced_graphics::text::{cosmic_text::fontdb, font_system};

        let mut system = font_system().write().unwrap();
        let id = system.raw().db().query(&fontdb::Query {
            families: &[fontdb::Family::Name(crate::PLATFORM_CJK_FONT)],
            weight: fontdb::Weight::default(),
            style: fontdb::Style::default(),
            stretch: fontdb::Stretch::default(),
        });
        let id = id.unwrap_or_else(|| {
            panic!(
                "平台默认字体 {} 未在系统字体库命中 — MainForm 中文将 tofu",
                crate::PLATFORM_CJK_FONT
            )
        });
        // 命中的字体必须同时覆盖汉字与数字 (表单标签/数值两态);
        // fontdb 回调给原始字节, 自行解析 Face (同 vm-overlay font.rs 惯用法)
        let covered = system.raw().db().with_face_data(id, |data, index| {
            ttf_parser::Face::parse(data, index)
                .map(|face| face.glyph_index('中').is_some() && face.glyph_index('0').is_some())
                .unwrap_or(false)
        });
        assert_eq!(
            covered,
            Some(true),
            "字体 {} 命中但缺汉字/数字字形",
            crate::PLATFORM_CJK_FONT
        );
    }
}
