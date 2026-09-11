//! voidmei 集成测试公共夹具: AppShell 无窗装配 + UI 事件驱动 + live 帧手造。
//! 黑盒原则: 经 pub 面 (with_parts/pump/ui_bus/FrameStore) 驱动;
//! 不 spawn 渲染线程/真窗口 (run_supervisor_phase 会自动补启真 Win32 窗口, 禁用)。
#![allow(dead_code)]

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use kernel::base::bus::flight_data_bus::FlightDataBus;
use kernel::base::bus::ui_state_bus::UIStateBus;
use kernel::config::configuration_service::ConfigurationService;
use kernel::config::json_model::{ConfigValue, GroupConfig, RowConfig};
use kernel::fm::FMManager;
use kernel::lang::Lang;
use overlay::platform::hotkey::HotkeyManager;
use voidmei::{AppShell, ControllerState, Env, ShellParts};

static CFG_N: AtomicUsize = AtomicUsize::new(0);

fn tmp_cfg(name: &str) -> String {
    let n = CFG_N.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir()
        .join(format!("vm_it_{name}_{}_{n}.json", std::process::id()))
        .to_str()
        .unwrap()
        .to_string()
}

fn trow(target: &str, value: bool) -> RowConfig {
    RowConfig {
        label: target.to_string(),
        r#type: "SWITCH".to_string(),
        property: Some(target.to_string()),
        value: Some(ConfigValue::Bool(value)),
        default_value: Some(ConfigValue::Bool(value)),
        ..RowConfig::default()
    }
}

fn tpanel(title: &str, rows: Vec<RowConfig>) -> GroupConfig {
    GroupConfig {
        title: title.to_string(),
        visible: true,
        rows,
        ..GroupConfig::default()
    }
}

/// fixture 树 (crosshairSwitch=true / enableEngineControl=false / autoStart=false)
fn test_panels() -> Vec<GroupConfig> {
    vec![tpanel(
        "T",
        vec![
            trow("crosshairSwitch", true),
            trow("enableEngineControl", false),
            trow("autoStartGameMode", false),
        ],
    )]
}

/// AppShell 无窗装配: tmp cfg (无写盘副作用) + 30ms 短防抖 + 网络隔离
/// (Service 指向 9 号死端口 — 连接立即拒绝; FM-Detect 探测关闭)。
/// 不起渲染线程 — ui_cmd 接收端留在 shell 内。
pub fn fixture() -> AppShell {
    let ui_bus = Arc::new(UIStateBus::new());
    let config = ConfigurationService::new(Some(Arc::clone(&ui_bus)));
    config.install_for_test(test_panels(), &tmp_cfg("base"));
    let (hotkey, hotkey_rx) = HotkeyManager::with_channel();
    let mut env = Env::probe(&Lang::init_lang(), false);
    env.app_port = 9; // discard 端口: 无监听, connect 立即 RST
    let mut shell = AppShell::with_parts(ShellParts {
        env,
        config,
        ui_bus: Arc::clone(&ui_bus),
        flight_bus: Arc::new(FlightDataBus::new()),
        fm: Arc::new(FMManager::new(Arc::new(kernel::base::bus::EventBus::new()))),
        hotkey,
        hotkey_rx,
        debounce_delay: Duration::from_millis(30),
    });
    shell.probe_network_for_test(false);
    shell.rebuild_controller(true);
    shell
}

/// UI 事件发布 + 泵 (pump 排空监督事件 + drive_from_live — 生产主循环同款)
pub fn send_ui_event_and_pump(shell: &mut AppShell, event_type: &str, data: &str) {
    shell
        .ui_bus
        .publish(event_type, Some("MainForm"), Some(data));
    shell.pump();
}

/// live ServiceData → 帧仓 (发布一帧; 真机由 Service 线程写, 测试手造)
pub fn live_store_of(d: &data::service_fields::ServiceData) -> Arc<data::frame::FrameStore> {
    let store = Arc::new(data::frame::FrameStore::new());
    store.publish(data::frame::Frame::from_service_data(d));
    store
}

/// drive_from_live 判定所需的 live 快照 (flags/type/playerLive)
pub fn live_service_data(plane: &str) -> data::service_fields::ServiceData {
    let mut st = kernel::game_api::parser::State::new();
    st.flag = true;
    let mut ind = kernel::game_api::parser::Indicators::new();
    ind.flag = true;
    ind.r#type = Some(plane.to_string());
    let mut d = data::service_fields::ServiceData::default();
    d.s_state = Some(st);
    d.s_indic = Some(ind);
    d.player_live = true;
    d
}

/// 装填 live 帧仓 (controller 读面)
pub fn set_live(shell: &AppShell, store: Arc<data::frame::FrameStore>) {
    *shell.shared.live.write().unwrap() = Some(store);
}

pub fn state_of(shell: &AppShell) -> ControllerState {
    shell.controller.as_ref().unwrap().state()
}

/// 等待谓词 (5s 上限; CI 慢机防御)
pub fn poll_until(desc: &str, f: impl Fn() -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("等待超时: {desc} (5s)");
}

/// 仓库根 (字体等资源定位)
pub fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
