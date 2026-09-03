use super::*;
use vm_core::config::json_model::{ConfigValue, GroupConfig, RowConfig};

/// 行构造速记
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

fn tmp_path(name: &str) -> String {
    // 掺 PID: 防两个测试进程并发跑时同名临时文件 truncate/read 竞争
    std::env::temp_dir()
        .join(format!("vm_ui_main_form_{}_{name}.json", std::process::id()))
        .to_str()
        .unwrap()
        .to_string()
}

/// 真实链路环境: 小树注入 (install_for_test) + 总线录制订阅。
/// 返回订阅句柄 — 调用方须绑定保活 (`_sub`), RAII Drop 即注销。
fn mk_state(
    name: &str,
    persist: Option<String>,
) -> (
    MainFormState,
    Arc<Mutex<Vec<UiStateEvent>>>,
    vm_core::base::bus::Subscription<UiStateEvent>,
) {
    let p = persist.unwrap_or_else(|| tmp_path(name));
    let _ = std::fs::remove_file(&p);
    let bus = Arc::new(vm_core::base::bus::ui_state_bus::UIStateBus::new());
    let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let s2 = Arc::clone(&seen);
    let sub = bus.subscribe(
        vm_core::base::event::ui_state_events::CONFIG_CHANGED,
        move |m: &UiStateEvent| {
            s2.lock().unwrap().push(m.clone());
        },
    );
    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.install_for_test(test_panels(), &p);
    (MainFormState::new(config, bus), seen, sub)
}

fn events_of(seen: &Arc<Mutex<Vec<UiStateEvent>>>) -> Vec<(String, String)> {
    seen.lock()
        .unwrap()
        .iter()
        .map(|e| (e.event_type.clone(), e.data.clone().unwrap_or_default()))
        .collect()
}

// Toggle 全链: 服务树 + 快照 (含跨 panel 同 key 全局更新) + CONFIG_CHANGED(key)
#[test]
fn toggle_updates_service_snapshot_and_bus() {
    let (mut state, seen, _sub) = mk_state("toggle", None);
    update(
        &mut state,
        Message::Toggle {
            panel: "面板A".into(),
            key: "k1".into(),
            value: false,
        },
    );

    assert_eq!(state.service_string("k1"), "false");
    assert_eq!(
        state.snapshot_row("面板A", "k1").unwrap().value,
        Some(ConfigValue::Bool(false))
    );
    // set_config 递归更新全部同 key 行 (面板B 的 k1 一并落库)
    assert_eq!(
        state.snapshot_row("面板B", "k1").unwrap().value,
        Some(ConfigValue::Bool(false))
    );
    let evs = events_of(&seen);
    assert!(evs.contains(&(ui_state_events::CONFIG_CHANGED.into(), "k1".into())));
}

// SwitchInv 反相链: 显示 true → 服务存 false + row.value 存显示值
#[test]
fn toggle_switch_inv_inverts_on_write() {
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
    assert_eq!(state.service_string("k2"), "false");
    assert_eq!(
        state.snapshot_row("面板A", "k2").unwrap().value,
        Some(ConfigValue::Bool(true)),
        "row.value 存显示值"
    );
}

// Slider 实时链不落盘; Save 落盘 (组字段 fontSize 的 delta 持久化)
#[test]
fn slider_live_then_save_persists() {
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
    assert_eq!(
        state.snapshot_row("面板A", "fontSize").unwrap().get_int(),
        7
    );
    let group_a = state
        .config()
        .get_layout_configs()
        .unwrap()
        .into_iter()
        .find(|g| g.title == "面板A")
        .unwrap();
    assert_eq!(group_a.font_size, 7, "组字段 fontSize 即时更新");
    // 拖拽语义: 不落盘 (on_release 前文件不存在)
    assert!(
        !std::path::Path::new(&persist).exists(),
        "Slider 拖拽期不得落盘"
    );
    assert!(events_of(&seen).contains(&(ui_state_events::CONFIG_CHANGED.into(), "fontSize".into())));

    // Save: delta 落盘 (组字段进 delta.fields — 持久真相)
    update(&mut state, Message::Save);
    assert!(std::path::Path::new(&persist).exists());
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&persist).unwrap()).unwrap();
    assert_eq!(saved["panels"]["面板A"]["fields"]["fontSize"], 7);
    let _ = std::fs::remove_file(&persist);
}

// Combo 选中链: row.value Str + 服务 + 即时落盘
#[test]
fn combo_pick_persists_immediately() {
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
    assert_eq!(state.service_string("style"), "B");
    assert_eq!(
        state.snapshot_row("面板A", "style").unwrap().value,
        Some(ConfigValue::Str("B".into()))
    );
    // Java ComboRowRenderer 每次选中即 onSave → 落盘
    assert!(std::path::Path::new(&persist).exists());
    let _ = std::fs::remove_file(&persist);
}

// ColorPicked 全链: 主键十进制 + row.value + CONFIG_CHANGED(key) + 即时落盘
#[test]
fn color_picked_writes_decimal_bus_and_persists() {
    let persist = tmp_path("color_user");
    let _ = std::fs::remove_file(&persist);
    let bus = Arc::new(vm_core::base::bus::ui_state_bus::UIStateBus::new());
    let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let s2 = Arc::clone(&seen);
    let _sub = bus.subscribe(
        vm_core::base::event::ui_state_events::CONFIG_CHANGED,
        move |m: &UiStateEvent| {
            s2.lock().unwrap().push(m.clone());
        },
    );
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
    assert_eq!(state.service_string("fontWarn"), "255, 36, 0, 128");
    // 快照行值 = 十进制串
    assert_eq!(
        state.snapshot_row("P", "fontWarn").unwrap().value,
        Some(ConfigValue::Str("255, 36, 0, 128".into()))
    );
    let evs = events_of(&seen);
    assert!(evs.contains(&(ui_state_events::CONFIG_CHANGED.into(), "fontWarn".into())));
    // 即时落盘 + 服务树收敛
    assert!(std::path::Path::new(&persist).exists());
    assert_eq!(
        state.config().get_layout_configs().unwrap()[0].rows[0].get_str(),
        "255, 36, 0, 128"
    );
    let _ = std::fs::remove_file(&persist);
}

/// 单树状态工厂 (无总线断言用例)
fn solo_state(name: &str, panels: Vec<GroupConfig>) -> (MainFormState, String) {
    let p = tmp_path(name);
    let _ = std::fs::remove_file(&p);
    let bus = Arc::new(vm_core::base::bus::ui_state_bus::UIStateBus::new());
    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.install_for_test(panels, &p);
    (MainFormState::new(config, bus), p)
}

// 无 :target 开关: label 为消息键 → set_config 的 label 命中臂 + 即时落盘
#[test]
fn toggle_without_target_falls_back_to_row_value() {
    let (mut state, persist) = solo_state(
        "notgt_sw",
        vec![panel("P", vec![row("裸开关", "SWITCH", None, Some(ConfigValue::Bool(true)))])],
    );

    update(
        &mut state,
        Message::Toggle {
            panel: "P".into(),
            key: "裸开关".into(),
            value: false,
        },
    );
    assert_eq!(
        state.snapshot_row("P", "裸开关").unwrap().value,
        Some(ConfigValue::Bool(false))
    );
    // 即时落盘 + 服务树收敛
    assert!(std::path::Path::new(&persist).exists());
    assert_eq!(
        state.config().get_layout_configs().unwrap()[0].rows[0].value,
        Some(ConfigValue::Bool(false))
    );
    let _ = std::fs::remove_file(&persist);
}

// 无 :target 滑条: 内存链不落盘 (valueIsAdjusting), Save 落盘
#[test]
fn slider_without_target_memory_then_save() {
    let (mut state, persist) = solo_state(
        "notgt_sl",
        vec![panel("P", vec![row("裸滑条", "SLIDER", None, Some(ConfigValue::Int(3)))])],
    );

    update(
        &mut state,
        Message::Slider {
            panel: "P".into(),
            key: "裸滑条".into(),
            value: 7,
        },
    );
    assert!(!std::path::Path::new(&persist).exists(), "拖拽期不落盘");
    assert_eq!(state.snapshot_row("P", "裸滑条").unwrap().get_int(), 7);

    update(&mut state, Message::Save);
    assert!(std::path::Path::new(&persist).exists());
    assert_eq!(
        state.config().get_layout_configs().unwrap()[0].rows[0].get_int(),
        7
    );
    let _ = std::fs::remove_file(&persist);
}

// RefreshPreviews: 精确广播一条 CONFIG_CHANGED("ui_layout.cfg")
#[test]
fn refresh_previews_publishes_exactly() {
    let (mut state, seen, _sub) = mk_state("refresh", None);
    seen.lock().unwrap().clear();
    update(&mut state, Message::RefreshPreviews);
    assert_eq!(
        events_of(&seen),
        vec![(
            ui_state_events::CONFIG_CHANGED.into(),
            "ui_layout.cfg".into()
        )]
    );
}

// 域外面板消息: 无副作用无 panic
#[test]
fn unknown_panel_message_is_ignored() {
    let (mut state, seen, _sub) = mk_state("unknown_panel", None);
    update(
        &mut state,
        Message::Toggle {
            panel: "不存在".into(),
            key: "k1".into(),
            value: false,
        },
    );
    assert_eq!(state.service_string("k1"), "true", "服务值不变");
    assert!(events_of(&seen).is_empty(), "不得产生事件");
}

// 计数与首行定位 (headless 驱动的基础)
#[test]
fn counts_and_first_row_of_type() {
    let (state, _seen, _sub) = mk_state("counts", None);
    assert_eq!(state.panel_count(), 2);
    // 面板A: HEADER(组1) + 4 项; 面板B: 1 项
    assert_eq!(state.row_count(), 6);
    assert_eq!(
        state.first_row_of_type("SWITCH"),
        Some(("面板A".to_string(), "k1".to_string()))
    );
    assert_eq!(
        state.first_row_of_type("SLIDER"),
        Some(("面板A".to_string(), "fontSize".to_string()))
    );
    assert_eq!(
        state.first_row_of_type("COMBO"),
        Some(("面板A".to_string(), "style".to_string()))
    );
    assert_eq!(state.first_row_of_type("COLOR"), None);
}

// enableFMPrint 特例: 写链额外广播 FM_PRINT_SWITCH_CHANGED
// (Java DynamicDataPage.java:148-151)
#[test]
fn toggle_fmprint_special_publishes() {
    let bus = Arc::new(vm_core::base::bus::ui_state_bus::UIStateBus::new());
    // 路由总线: 两类事件各挂一探针, 共享 seen (实际送达序 = publish 序)
    let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let s2 = Arc::clone(&seen);
    let _sub_cfg = bus.subscribe(
        vm_core::base::event::ui_state_events::CONFIG_CHANGED,
        move |m: &UiStateEvent| {
            s2.lock().unwrap().push(m.clone());
        },
    );
    let s3 = Arc::clone(&seen);
    let _sub_fm = bus.subscribe(
        vm_core::base::event::ui_state_events::FM_PRINT_SWITCH_CHANGED,
        move |m: &UiStateEvent| {
            s3.lock().unwrap().push(m.clone());
        },
    );
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
    let evs = events_of(&seen);
    assert_eq!(
        evs,
        vec![
            (
                ui_state_events::CONFIG_CHANGED.into(),
                "enableFMPrint".into()
            ),
            (
                ui_state_events::FM_PRINT_SWITCH_CHANGED.into(),
                "false".into()
            ),
        ]
    );
}

/// 动作按钮执行链: ButtonAction 挂模态 → ConfirmPending 执行 reset + 快照收敛。
/// (JSON 化: 无 CWD 沙箱 — reset 不读磁盘模板, 树注入即基)
#[test]
fn button_action_confirm_executes_reset() {
    let (mut state, _p) = solo_state(
        "btn",
        vec![panel(
            "P",
            vec![row("a", "SWITCH", Some("k1"), Some(ConfigValue::Bool(false)))],
        )],
    );

    // 改动 → delta 登记
    update(
        &mut state,
        Message::Toggle {
            panel: "P".into(),
            key: "k1".into(),
            value: true,
        },
    );
    assert_eq!(state.service_string("k1"), "true");

    // ① 按下 factoryReset → 挂起确认模态 (不执行)
    update(
        &mut state,
        Message::ButtonAction {
            action: "factoryReset".into(),
        },
    );
    assert!(state.pending_action.is_some(), "确认模态应挂起");
    assert_eq!(state.service_string("k1"), "true", "未确认前不得重置");

    // ② 取消 → 无副作用
    update(&mut state, Message::CancelPending);
    assert!(state.pending_action.is_none());

    // ③ 再按 + 确认 → reset 执行 (delta 清空 → 注入树原值)
    update(
        &mut state,
        Message::ButtonAction {
            action: "factoryReset".into(),
        },
    );
    update(&mut state, Message::ConfirmPending);
    assert!(state.pending_action.is_none(), "执行后模态关闭");
    assert_eq!(state.service_string("k1"), "false", "行值回注入树原值");
    assert_eq!(state.panel_count(), 1, "快照随重置收敛");
}

/// resetConfig (行值重置) 分支: 只清行值 delta, 不动组字段
#[test]
fn button_action_reset_config_clears_row_values_only() {
    let (mut state, _p) = solo_state(
        "btn2",
        vec![GroupConfig {
            font_size: 0,
            rows: vec![
                row("a", "SWITCH", Some("k1"), Some(ConfigValue::Bool(false))),
                row("fs", "SLIDER", Some("fontSize"), Some(ConfigValue::Int(0))),
            ],
            ..panel("P", vec![])
        }],
    );
    update(
        &mut state,
        Message::Slider {
            panel: "P".into(),
            key: "fontSize".into(),
            value: 5,
        },
    );
    update(
        &mut state,
        Message::Toggle {
            panel: "P".into(),
            key: "k1".into(),
            value: true,
        },
    );
    update(
        &mut state,
        Message::ButtonAction {
            action: "resetConfig".into(),
        },
    );
    update(&mut state, Message::ConfirmPending);
    // 行值回出厂, 组字段保留 (Java resetAllLayoutDefaults 语义)
    assert_eq!(state.service_string("k1"), "false");
    let g = &state.groups()[0];
    assert_eq!(g.font_size, 5, "组字段不在行值重置范围");
}
