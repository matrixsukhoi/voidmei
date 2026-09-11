//! ui 表单数据层黑盒场景 (ui::main_form): Message → update 写链 →
//! ConfigurationService 树/delta → UIStateBus 广播。只经 crate pub 面
//! (MainFormState 观测口 + 总线录制订阅), 不触碰渲染。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


use std::sync::{Arc, Mutex};

use expect_test::expect;

use kernel::base::bus::ui_state_bus::{UiStateEvent, UIStateBus};
use kernel::base::event::ui_state_events;
use kernel::config::configuration_service::ConfigurationService;
use kernel::config::json_model::{ConfigValue, GroupConfig, RowConfig};

use ui::main_form::{update, Message, MainFormState};

// ---------------------------------------------------------------------
// 构造速记
// ---------------------------------------------------------------------

/// 行构造
fn row(label: &str, ty: &str, target: Option<&str>, value: Option<ConfigValue>) -> RowConfig {
    RowConfig {
        label: label.to_string(),
        r#type: ty.to_string(),
        property: target.map(str::to_string),
        value: value.clone(),
        default_value: value,
        ..RowConfig::default()
    }
}

fn panel(title: &str, rows: Vec<RowConfig>) -> GroupConfig {
    GroupConfig {
        title: title.to_string(),
        rows,
        ..GroupConfig::default()
    }
}

/// 测试树: 面板A (组1 嵌套) + 面板B (跨 panel 同 key)
fn test_panels() -> Vec<GroupConfig> {
    vec![
        GroupConfig {
            panel_columns: 2,
            rows: vec![RowConfig {
                label: "组1".to_string(),
                r#type: "HEADER".to_string(),
                children: vec![
                    row("开关", "SWITCH", Some("k1"), Some(ConfigValue::Bool(true))),
                    row("反相", "SWITCH_INV", Some("k2"), Some(ConfigValue::Bool(false))),
                    row("滑条", "SLIDER", Some("fontSize"), Some(ConfigValue::Int(0))),
                    RowConfig {
                        source: Some("A,B,C".to_string()),
                        ..row("下拉", "COMBO", Some("style"), Some(ConfigValue::Str("A".into())))
                    },
                ],
                ..RowConfig::default()
            }],
            ..panel("面板A", vec![])
        },
        panel("面板B", vec![row("开关B", "SWITCH", Some("k1"), Some(ConfigValue::Bool(true)))]),
    ]
}

/// 临时持久化路径 (掺 PID 防并发测试进程同名互踩)
fn tmp_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("vm_ui_main_form_{}_{name}.json", std::process::id()))
        .to_str()
        .unwrap()
        .to_string()
}

/// 真实链路环境: 测试树注入 (install_for_test) + CONFIG_CHANGED 总线录制。
/// 返回订阅句柄 — 调用方须绑定保活 (`_sub`), RAII Drop 即注销。
fn mk_state(
    name: &str,
    persist: Option<String>,
) -> (
    MainFormState,
    Arc<Mutex<Vec<UiStateEvent>>>,
    kernel::base::bus::Subscription<UiStateEvent>,
) {
    let p = persist.unwrap_or_else(|| tmp_path(name));
    let _ = std::fs::remove_file(&p);
    let bus = Arc::new(UIStateBus::new());
    let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let s2 = Arc::clone(&seen);
    let sub = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |m: &UiStateEvent| {
        s2.lock().unwrap().push(m.clone());
    });
    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.install_for_test(test_panels(), &p);
    (MainFormState::new(config, bus), seen, sub)
}

/// 事件流 → (type, data) 对
fn events_of(seen: &Arc<Mutex<Vec<UiStateEvent>>>) -> Vec<(String, String)> {
    seen.lock()
        .unwrap()
        .iter()
        .map(|e| (e.event_type.clone(), e.data.clone().unwrap_or_default()))
        .collect()
}

/// Toggle 全链: 服务树 + 快照 (含跨 panel 同 key 全局更新) + CONFIG_CHANGED(key) 广播
#[test]
fn toggle_写链落库与广播() {
    let (mut state, seen, _sub) = mk_state("toggle", None);
    update(
        &mut state,
        Message::Toggle {
            panel: "面板A".into(),
            key: "k1".into(),
            value: false,
        },
    );

    expect!["false"].assert_eq(&state.service_string("k1"));
    // set_config 递归更新全部同 key 行 (面板B 的 k1 一并落库)
    expect![[r#"
        面板A: Some(Bool(false))
        面板B: Some(Bool(false))"#]]
    .assert_eq(&format!(
        "面板A: {:?}\n面板B: {:?}",
        state.snapshot_row("面板A", "k1").unwrap().value,
        state.snapshot_row("面板B", "k1").unwrap().value,
    ));
    let evs = events_of(&seen);
    assert!(
        evs.contains(&(ui_state_events::CONFIG_CHANGED.into(), "k1".into())),
        "应广播 CONFIG_CHANGED(k1), 实际 {evs:?}"
    );
}

/// SWITCH_INV 反相链: 显示 true → 服务存 false + row.value 存显示值
#[test]
fn toggle_switch_inv_取反落库() {
    let (mut state, _seen, _sub) = mk_state("inv", None);
    update(
        &mut state,
        Message::Toggle {
            panel: "面板A".into(),
            key: "k2".into(),
            value: true,
        },
    );
    // 服务 get_config 对 SWITCH_INV 返回 !row.get_bool() (存 true → 读 false)
    expect!["false"].assert_eq(&state.service_string("k2"));
    expect!["row.value = Some(Bool(true)) (显示值)"]
    .assert_eq(&format!(
        "row.value = {:?} (显示值)",
        state.snapshot_row("面板A", "k2").unwrap().value
    ));
}

/// Slider 拖拽语义: 内存链即时更新不落盘; Save 后 delta 落盘 (组字段持久真相)
#[test]
fn slider_内存链与save落盘() {
    let persist = tmp_path("slider_user");
    let _ = std::fs::remove_file(&persist);
    let (mut state, seen, _sub) = mk_state("slider", Some(persist.clone()));

    update(
        &mut state,
        Message::Slider {
            panel: "面板A".into(),
            key: "fontSize".into(),
            value: 7,
        },
    );
    // 实时链: 快照行值 + 组字段
    expect!["7"].assert_eq(&state.snapshot_row("面板A", "fontSize").unwrap().get_int().to_string());
    let group_a = state
        .config()
        .get_layout_configs()
        .unwrap()
        .into_iter()
        .find(|g| g.title == "面板A")
        .unwrap();
    assert_eq!(group_a.font_size, 7, "组字段 fontSize 即时更新");
    // 拖拽期不落盘 (on_release 前文件不存在)
    assert!(
        !std::path::Path::new(&persist).exists(),
        "Slider 拖拽期不得落盘"
    );
    assert!(events_of(&seen).contains(&(ui_state_events::CONFIG_CHANGED.into(), "fontSize".into())));

    // Save: delta 落盘 (组字段进 delta.fields)
    update(&mut state, Message::Save);
    assert!(std::path::Path::new(&persist).exists());
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&persist).unwrap()).unwrap();
    expect!["7"].assert_eq(&saved["panels"]["面板A"]["fields"]["fontSize"].to_string());
    let _ = std::fs::remove_file(&persist);
}

/// Combo 选中链: row.value Str + 服务 + 即时落盘
#[test]
fn combo_即存() {
    let persist = tmp_path("combo_user");
    let _ = std::fs::remove_file(&persist);
    let (mut state, _seen, _sub) = mk_state("combo", Some(persist.clone()));

    update(
        &mut state,
        Message::Combo {
            panel: "面板A".into(),
            key: "style".into(),
            value: "B".into(),
        },
    );
    expect!["B"].assert_eq(&state.service_string("style"));
    expect![[r#"Some(Str("B"))"#]]
    .assert_eq(&format!("{:?}", state.snapshot_row("面板A", "style").unwrap().value));
    // Java ComboRowRenderer 每次选中即 onSave → 落盘
    assert!(std::path::Path::new(&persist).exists());
    let _ = std::fs::remove_file(&persist);
}

/// ColorPicked 全链: 主键十进制 + row.value + CONFIG_CHANGED(key) + 即时落盘
#[test]
fn color_picked_十进制落库() {
    let persist = tmp_path("color_user");
    let _ = std::fs::remove_file(&persist);
    let bus = Arc::new(UIStateBus::new());
    let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let s2 = Arc::clone(&seen);
    let _sub = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |m: &UiStateEvent| {
        s2.lock().unwrap().push(m.clone());
    });
    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.install_for_test(
        vec![panel(
            "P",
            vec![row(
                "告警色",
                "COLOR",
                Some("fontWarn"),
                Some(ConfigValue::Str("#FF2400FF".into())),
            )],
        )],
        &persist,
    );
    let mut state = MainFormState::new(config, bus);

    update(
        &mut state,
        Message::ColorPicked {
            panel: "P".into(),
            key: "fontWarn".into(),
            value: [255, 36, 0, 128],
        },
    );
    // 服务: 主键十进制 (向后兼容存储格式)
    expect!["255, 36, 0, 128"].assert_eq(&state.service_string("fontWarn"));
    expect![[r#"Some(Str("255, 36, 0, 128"))"#]]
    .assert_eq(&format!(
        "{:?}",
        state.snapshot_row("P", "fontWarn").unwrap().value
    ));
    let evs = events_of(&seen);
    assert!(
        evs.contains(&(ui_state_events::CONFIG_CHANGED.into(), "fontWarn".into())),
        "应广播 CONFIG_CHANGED(fontWarn)"
    );
    // 即时落盘 + 服务树收敛
    assert!(std::path::Path::new(&persist).exists());
    expect!["255, 36, 0, 128"].assert_eq(&state.config().get_layout_configs().unwrap()[0].rows[0].get_str());
    let _ = std::fs::remove_file(&persist);
}

/// RefreshPreviews: 精确广播一条 CONFIG_CHANGED("ui_layout.cfg")
#[test]
fn refresh_previews_广播() {
    let (mut state, seen, _sub) = mk_state("refresh", None);
    seen.lock().unwrap().clear();
    update(&mut state, Message::RefreshPreviews);
    expect![[r#"
        [("configChanged", "ui_layout.cfg")]"#]]
    .assert_eq(&format!("{:?}", events_of(&seen)));
}

/// 域外面板消息: 无副作用无 panic (行不存在忽略)
#[test]
fn 未知面板忽略() {
    let (mut state, seen, _sub) = mk_state("unknown_panel", None);
    update(
        &mut state,
        Message::Toggle {
            panel: "不存在".into(),
            key: "k1".into(),
            value: false,
        },
    );
    expect!["true (服务值不变)"].assert_eq(&format!("{} (服务值不变)", state.service_string("k1")));
    assert!(events_of(&seen).is_empty(), "不得产生事件");
}

/// StartGame/EndGame = 保存语义 (Java MainForm.confirm / mCancel):
/// 挂起的 Slider 内存值经两者落盘 + 广播全局键
#[test]
fn start_end_game_保存语义() {
    let persist = tmp_path("game_user");
    let _ = std::fs::remove_file(&persist);
    let (mut state, seen, _sub) = mk_state("game", Some(persist.clone()));

    // Slider 拖拽期不落盘 → StartGame 承担保存
    update(
        &mut state,
        Message::Slider {
            panel: "面板A".into(),
            key: "fontSize".into(),
            value: 9,
        },
    );
    assert!(!std::path::Path::new(&persist).exists());
    update(&mut state, Message::StartGame);
    assert!(std::path::Path::new(&persist).exists(), "StartGame 应落盘");
    assert!(
        events_of(&seen).contains(&(ui_state_events::CONFIG_CHANGED.into(), "ui_layout.cfg".into())),
        "StartGame 应广播全局键"
    );

    // EndGame 同款保存语义: 新改动再落一次
    let _ = std::fs::remove_file(&persist);
    update(
        &mut state,
        Message::Slider {
            panel: "面板A".into(),
            key: "fontSize".into(),
            value: 12,
        },
    );
    assert!(!std::path::Path::new(&persist).exists());
    update(&mut state, Message::EndGame);
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&persist).unwrap()).unwrap();
    expect!["12"].assert_eq(&saved["panels"]["面板A"]["fields"]["fontSize"].to_string());
    let _ = std::fs::remove_file(&persist);
}

/// enableFMPrint 特例: 写链额外广播 FM_PRINT_SWITCH_CHANGED
/// (Java DynamicDataPage.java:148-151)
#[test]
fn toggle_fmprint_特例双广播() {
    let bus = Arc::new(UIStateBus::new());
    // 两类事件各挂一探针, 共享 seen (送达序 = publish 序)
    let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let s2 = Arc::clone(&seen);
    let _sub_cfg = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |m: &UiStateEvent| {
        s2.lock().unwrap().push(m.clone());
    });
    let s3 = Arc::clone(&seen);
    let _sub_fm = bus.subscribe(ui_state_events::FM_PRINT_SWITCH_CHANGED, move |m: &UiStateEvent| {
        s3.lock().unwrap().push(m.clone());
    });
    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.install_for_test(
        vec![panel(
            "p",
            vec![row("fm", "SWITCH", Some("enableFMPrint"), Some(ConfigValue::Bool(true)))],
        )],
        &tmp_path("fmp"),
    );
    let mut state = MainFormState::new(config, Arc::clone(&bus));

    update(
        &mut state,
        Message::Toggle {
            panel: "p".into(),
            key: "enableFMPrint".into(),
            value: false,
        },
    );
    expect![[r#"
        [("configChanged", "enableFMPrint"), ("fmPrintSwitchChanged", "false")]"#]]
    .assert_eq(&format!("{:?}", events_of(&seen)));
}

/// ButtonAction 确认链: factoryReset 挂起 → 确认执行 → 行值回注入树原值
/// (模态态私有, 经可观察的配置值收敛断言)
#[test]
fn button_action_确认后重置() {
    let persist = tmp_path("btn");
    let _ = std::fs::remove_file(&persist);
    let bus = Arc::new(UIStateBus::new());
    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.install_for_test(
        vec![panel(
            "P",
            vec![row("a", "SWITCH", Some("k1"), Some(ConfigValue::Bool(false)))],
        )],
        &persist,
    );
    let mut state = MainFormState::new(config, bus);

    // 改动 → 登记 delta
    update(
        &mut state,
        Message::Toggle {
            panel: "P".into(),
            key: "k1".into(),
            value: true,
        },
    );
    expect!["true"].assert_eq(&state.service_string("k1"));

    // 按下 + 确认 → reset 执行 (delta 清空 → 注入树原值)
    update(&mut state, Message::ButtonAction { action: "factoryReset".into() });
    // 未确认前不重置: 中途 Cancel 后值保持
    update(&mut state, Message::CancelPending);
    expect!["true (取消后不得重置)"].assert_eq(&format!(
        "{} (取消后不得重置)",
        state.service_string("k1")
    ));
    update(&mut state, Message::ButtonAction { action: "factoryReset".into() });
    update(&mut state, Message::ConfirmPending);
    expect!["false (确认后回注入树原值)"].assert_eq(&format!(
        "{} (确认后回注入树原值)",
        state.service_string("k1")
    ));
    expect!["1 (快照随重置收敛)"].assert_eq(&format!("{} (快照随重置收敛)", state.panel_count()));
    let _ = std::fs::remove_file(&persist);
}

/// 计数与首行定位 (headless 驱动的基础面)
#[test]
fn 计数与首行定位() {
    let (state, _seen, _sub) = mk_state("counts", None);
    expect!["panels 2 rows 6"].assert_eq(&format!(
        "panels {} rows {}",
        state.panel_count(),
        state.row_count()
    ));
    expect![[r#"
        SWITCH -> Some(("面板A", "k1"))
        SLIDER -> Some(("面板A", "fontSize"))
        COMBO -> Some(("面板A", "style"))
        COLOR -> None"#]]
    .assert_eq(&format!(
        "SWITCH -> {:?}\nSLIDER -> {:?}\nCOMBO -> {:?}\nCOLOR -> {:?}",
        state.first_row_of_type("SWITCH"),
        state.first_row_of_type("SLIDER"),
        state.first_row_of_type("COMBO"),
        state.first_row_of_type("COLOR"),
    ));
}
