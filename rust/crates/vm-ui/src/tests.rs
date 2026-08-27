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
