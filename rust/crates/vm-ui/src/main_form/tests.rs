use super::*;
use vm_core::config::config_loader::ConfigValue;

const TEST_CFG: &str = r#"(panel "面板A" :panel-columns 2
  (group "组1"
    (item "开关" :type switch :target "k1" :value true)
    (item "反相" :type switch-inv :target "k2" :value false)
    (item "滑条" :type slider :target "fontSize" :min -10 :max 10 :value 0)
    (item "下拉" :type combo :target "style" :source "A,B,C" :value "A")
  )
)
(panel "面板B"
  (item "开关B" :type switch :target "k1" :value true)
)"#;

fn tmp_path(name: &str) -> String {
    // 掺 PID: 防两个测试进程并发跑时同名临时文件 truncate/read 竞争
    std::env::temp_dir()
        .join(format!("vm_ui_main_form_{}_{name}.cfg", std::process::id()))
        .to_str()
        .unwrap()
        .to_string()
}

/// 真实链路环境: 模板落 tmp → ConfigurationService 装载 + 总线录制订阅。
/// 返回订阅句柄 — 调用方须绑定保活 (`_sub`), RAII Drop 即注销。
fn mk_state(
    name: &str,
    persist: Option<String>,
) -> (
    MainFormState,
    Arc<Mutex<Vec<UiStateEvent>>>,
    vm_core::base::bus::Subscription<UiStateEvent>,
) {
    let p = tmp_path(name);
    std::fs::write(&p, TEST_CFG).unwrap();
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
    config.load_layout(&p);
    (MainFormState::new(config, bus, persist), seen, sub)
}

fn events_of(seen: &Arc<Mutex<Vec<UiStateEvent>>>) -> Vec<(String, String)> {
    seen.lock()
        .unwrap()
        .iter()
        .map(|e| (e.event_type.clone(), e.data.clone().unwrap_or_default()))
        .collect()
}

// Toggle 全链: 服务树 + 快照 (含跨 panel 同 key 全局更新, 对位 setConfig 的
// update_rows_recursive 全实例语义) + CONFIG_CHANGED(key) + 保存链广播
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
    // Java setConfig 递归更新全部同 key 行 (面板B 的 k1 一并落库)
    assert_eq!(
        state.snapshot_row("面板B", "k1").unwrap().value,
        Some(ConfigValue::Bool(false))
    );
    let evs = events_of(&seen);
    assert!(evs.contains(&(ui_state_events::CONFIG_CHANGED.into(), "k1".into())));
    assert!(evs.contains(&(
        ui_state_events::CONFIG_CHANGED.into(),
        "ui_layout.cfg".into()
    )));
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

// Slider 实时链不落盘; Save 落盘后服务树收敛 (组字段 font_size 经 load_layout 回服务)
#[test]
fn slider_live_then_save_persists_and_converges() {
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
    // 实时链: 快照行值 + 组字段 + 服务值
    assert_eq!(
        state.snapshot_row("面板A", "fontSize").unwrap().get_int(),
        7
    );
    assert_eq!(state.service_string("fontSize"), "7");
    // 拖拽语义: 不落盘 (on_release 前文件不存在)
    assert!(
        !std::path::Path::new(&persist).exists(),
        "Slider 拖拽期不得落盘"
    );
    assert!(events_of(&seen).contains(&(ui_state_events::CONFIG_CHANGED.into(), "fontSize".into())));

    // Save: 落盘 + 服务树重读收敛 (组字段 font_size 回到服务侧 — clone-split 收敛)
    update(&mut state, Message::Save);
    assert!(std::path::Path::new(&persist).exists());
    let group_a = state
        .config
        .get_layout_configs()
        .unwrap()
        .into_iter()
        .find(|g| g.title == "面板A")
        .unwrap();
    assert_eq!(
        group_a.font_size, 7,
        "组字段经落盘→load_layout 收敛回服务树"
    );
    let _ = std::fs::remove_file(&persist);
}

// Combo 选中链: row.value Str + 服务 + on_save 即落盘 (Java 每次交互即存)
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

// ColorPicked 全链: 主键十进制 + row.value + CONFIG_CHANGED(key) + on_save
// 即落盘 + 保存链广播 (Java applyColorChange L110-136 → onSave)
#[test]
fn color_picked_writes_decimal_bus_and_persists() {
    let persist = tmp_path("color_user");
    let _ = std::fs::remove_file(&persist);
    let p = tmp_path("color_src");
    std::fs::write(
        &p,
        r##"(panel "P" (item "告警色" :type color :target "fontWarn" :value "#FF2400FF"))"##,
    )
    .unwrap();
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
    config.load_layout(&p);
    let mut state = MainFormState::new(config, bus, Some(persist.clone()));

    update(
        &mut state,
        Message::ColorPicked {
            panel: "P".into(),
            key: "fontWarn".into(),
            value: [255, 36, 0, 128],
        },
    );
    // 服务: 主键十进制 (Java L124 向后兼容存储格式)
    assert_eq!(state.service_string("fontWarn"), "255, 36, 0, 128");
    // 快照行值 = 十进制串 (mirror_key_from_service 收敛)
    assert_eq!(
        state.snapshot_row("P", "fontWarn").unwrap().value,
        Some(ConfigValue::Str("255, 36, 0, 128".into()))
    );
    // WYSIWYG 链: set→publish(key) + 保存链 publish("ui_layout.cfg")
    let evs = events_of(&seen);
    assert!(evs.contains(&(ui_state_events::CONFIG_CHANGED.into(), "fontWarn".into())));
    assert!(evs.contains(&(
        ui_state_events::CONFIG_CHANGED.into(),
        "ui_layout.cfg".into()
    )));
    // Java L135 onSave: 即时落盘 + 服务树收敛
    assert!(std::path::Path::new(&persist).exists());
    assert_eq!(
        state.config.get_layout_configs().unwrap()[0].rows[0].get_str(),
        "255, 36, 0, 128"
    );
    let _ = std::fs::remove_file(&persist);
}

/// 单段 cfg 的状态工厂 (无总线断言用例)
fn solo_state(name: &str, cfg: &str, persist: Option<String>) -> MainFormState {
    let p = tmp_path(name);
    std::fs::write(&p, cfg).unwrap();
    let bus = Arc::new(vm_core::base::bus::ui_state_bus::UIStateBus::new());
    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.load_layout(&p);
    MainFormState::new(config, bus, persist)
}

// 无 :target 开关: label 为消息键 → write_bool(None) 回落 row.value + 即时落盘
// (Java SwitchRowRenderer L64-67: writeBool(null) 失败回落 + onSave)
#[test]
fn toggle_without_target_falls_back_to_row_value() {
    let persist = tmp_path("notgt_sw_user");
    let _ = std::fs::remove_file(&persist);
    let mut state = solo_state(
        "notgt_sw",
        r#"(panel "P" (item "裸开关" :type switch :value true))"#,
        Some(persist.clone()),
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
    // Java 每次交互 onSave 即落盘; row.value 经挂起重放 → 服务树收敛
    assert!(std::path::Path::new(&persist).exists());
    assert_eq!(
        state.config.get_layout_configs().unwrap()[0].rows[0].value,
        Some(ConfigValue::Bool(false))
    );
    let _ = std::fs::remove_file(&persist);
}

// 无 :target 滑条: 内存链不落盘 (valueIsAdjusting), Save 落盘并收敛服务树
#[test]
fn slider_without_target_memory_then_save() {
    let persist = tmp_path("notgt_sl_user");
    let _ = std::fs::remove_file(&persist);
    let mut state = solo_state(
        "notgt_sl",
        r#"(panel "P" (item "裸滑条" :type slider :min 0 :max 10 :value 3))"#,
        Some(persist.clone()),
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
        state.config.get_layout_configs().unwrap()[0].rows[0].get_int(),
        7
    );
    let _ = std::fs::remove_file(&persist);
}

// 无 :target 下拉: row.value + 即时落盘 (Java ComboRowRenderer L52-61)
#[test]
fn combo_without_target_persists_row_value() {
    let persist = tmp_path("notgt_cb_user");
    let _ = std::fs::remove_file(&persist);
    let mut state = solo_state(
        "notgt_cb",
        r#"(panel "P" (item "裸下拉" :type combo :source "X,Y" :value "X"))"#,
        Some(persist.clone()),
    );

    update(
        &mut state,
        Message::Combo {
            panel: "P".into(),
            key: "裸下拉".into(),
            value: "Y".into(),
        },
    );
    assert_eq!(
        state.snapshot_row("P", "裸下拉").unwrap().value,
        Some(ConfigValue::Str("Y".into()))
    );
    assert!(std::path::Path::new(&persist).exists());
    assert_eq!(
        state.config.get_layout_configs().unwrap()[0].rows[0].value,
        Some(ConfigValue::Str("Y".into()))
    );
    let _ = std::fs::remove_file(&persist);
}

// 外部整树替换 (import/reset/watcher 模拟): 服务树被 load_layout 重载后, 后续
// 交互的保存不得用陈旧快照覆盖外部值 (对位 DynamicDataPage.rebuild L94-100
// findGroupByTitle 取最新树); 快照随保存重建
#[test]
fn persist_after_external_reload_keeps_external_values() {
    let persist = tmp_path("ext_user");
    let _ = std::fs::remove_file(&persist);
    let cfg = r#"(panel "P"
  (item "开关1" :type switch :target "e1" :value true)
  (item "开关2" :type switch :target "e2" :value true)
)"#;
    let mut state = solo_state("ext", cfg, Some(persist.clone()));

    // 交互 1: e1=false 交互即存
    update(
        &mut state,
        Message::Toggle {
            panel: "P".into(),
            key: "e1".into(),
            value: false,
        },
    );
    assert!(std::path::Path::new(&persist).exists());

    // 外部替换: 持久化路径被外部重写 (e1=true) 且服务树重载 — 快照变陈旧
    std::fs::write(&persist, cfg).unwrap();
    state.config.load_layout(&persist);

    // 交互 2: e2=false → 落盘必须保留外部 e1=true (旧实现写陈旧快照会回滚 e1)
    update(
        &mut state,
        Message::Toggle {
            panel: "P".into(),
            key: "e2".into(),
            value: false,
        },
    );
    let reread = vm_core::config::config_loader::load_config(&persist);
    let vals: Vec<(String, bool)> = reread[0]
        .rows
        .iter()
        .map(|r| (r.property.clone().unwrap(), r.get_bool()))
        .collect();
    assert_eq!(
        vals,
        vec![("e1".to_string(), true), ("e2".to_string(), false)],
        "外部 e1=true 保留, 本交互 e2=false 落盘"
    );
    // 快照已随保存重建 (rebuild 语义)
    assert!(state.snapshot_row("P", "e1").unwrap().get_bool());
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

// enableFMPrint 特例: sync_to_config_service 额外广播 FM_PRINT_SWITCH_CHANGED
// (Java DynamicDataPage.java:148-151)
#[test]
fn write_context_fmprint_special_publishes() {
    let p = tmp_path("fmp");
    std::fs::write(
        &p,
        r#"(panel "p" (item "fm" :type switch :target "enableFMPrint" :value true))"#,
    )
    .unwrap();
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
    config.load_layout(&p);

    let ctx = WriteContext::new(&config, &bus);
    ctx.sync_to_config_service("enableFMPrint", false);
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

/// 动作按钮执行链 (审查轮 2-D 接线): ButtonAction 挂模态 → ConfirmPending
/// 执行 reset + 整树收敛。
/// reset 链操作 config_manager 全局路径 (CWD 相对) → tmp 沙箱 + 专用锁
/// (进程级 CWD, 对齐 vm-core CWD_LOCK 纪律)。
/// ⚠ 沙箱守卫纪律 (事故教训): Drop 里 **chdir 回 orig、remove 的必须是
/// tmp dir** — 两目标分离存储, 清理对象写错会删工作区
#[test]
fn button_action_confirm_executes_reset() {
    use std::sync::Mutex as TestMutex;
    static CWD_LOCK: TestMutex<()> = TestMutex::new(());
    let _cwd_guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let dir = std::env::temp_dir().join(format!("vm_ui_btn_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // 沙箱放模板 (resetToFactory 读 ./ui_layout.cfg 覆盖用户配置)
    let tpl = std::fs::read_to_string("../../../ui_layout.cfg").unwrap();
    std::fs::write(dir.join("ui_layout.cfg"), &tpl).unwrap();
    let orig = std::env::current_dir().unwrap();
    std::env::set_current_dir(&dir).unwrap();
    struct Sandbox {
        orig: std::path::PathBuf, // chdir 回这里
        dir: std::path::PathBuf,  // 只删这里 (tmp)
    }
    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.orig);
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
    let _sandbox = Sandbox {
        orig,
        dir: dir.clone(),
    };

    let bus = Arc::new(vm_core::base::bus::ui_state_bus::UIStateBus::new());
    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.load_layout("ui_layout.cfg");
    let mut state = MainFormState::new(
        config,
        Arc::clone(&bus),
        Some(
            dir.join("ui_layout.user.cfg")
                .to_string_lossy()
                .into_owned(),
        ),
    );
    let n_before = state.groups.len();

    // ① 按下 factoryReset → 挂起确认模态 (不执行)
    update(
        &mut state,
        Message::ButtonAction {
            action: "factoryReset".into(),
        },
    );
    assert!(state.pending_action.is_some(), "确认模态应挂起");
    assert_eq!(state.groups.len(), n_before, "未确认前不得重置");

    // ② 取消 → 无副作用
    update(&mut state, Message::CancelPending);
    assert!(state.pending_action.is_none());

    // ③ 再按 + 确认 → reset 执行 + 整树收敛 (模板组数回归)
    update(
        &mut state,
        Message::ButtonAction {
            action: "factoryReset".into(),
        },
    );
    update(&mut state, Message::ConfirmPending);
    assert!(state.pending_action.is_none(), "执行后模态关闭");
    assert!(
        state.groups.len() >= 10,
        "整树应从模板收敛 (实得 {} 组)",
        state.groups.len()
    );
}
