//! vm-app 集成测试 (波11 分片: 共享夹具/助手在本文件, 用例按主题分片 —
//! lifecycle / debounce_config / render_feeds / voice; 分片经 `use super::*`
//! 取共享面, 先例 vm-overlay overlays/fields_tests)。

mod debounce_config;
mod lifecycle;
mod render_feeds;
mod voice;

use super::*;

use std::sync::atomic::AtomicUsize;

use vm_core::base::bus::EventBus; // FmChangedBus 底座 (EventBus<FMHandle>) 的构造面

use vm_core::fm::FMHandle;

static CFG_N: AtomicUsize = AtomicUsize::new(0);

/// 测试助手 (波4): ServiceData → 帧仓 (发布一帧)。原 "手造 RwLock<ServiceData>
/// 塞 live" 的测试形态改为帧仓; 需要中途改数据的用 update_live_frame 重发布。
fn frame_store_of(d: &ServiceData) -> Arc<vm_data::frame::FrameStore> {
    let store = Arc::new(vm_data::frame::FrameStore::new());
    store.publish(vm_data::frame::Frame::from_service_data(d));
    store
}

/// 改 ServiceData 后重发布帧 (测试观测写点的等价物)
fn update_live_frame(store: &vm_data::frame::FrameStore, d: &ServiceData) {
    store.publish(vm_data::frame::Frame::from_service_data(d));
}

/// tmp 配置文件 (vm-core configuration_service 测试同款惯例)
fn tmp_cfg(name: &str) -> String {
    let n = CFG_N.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir()
        .join(format!("vm_app_shell_{name}_{}_{n}.json", std::process::id()))
        .to_str()
        .unwrap()
        .to_string()
}

/// 行/panel 构造速记
fn trow(target: &str, value: bool) -> vm_core::config::json_model::RowConfig {
    vm_core::config::json_model::RowConfig {
        label: target.to_string(),
        r#type: "SWITCH".to_string(),
        property: Some(target.to_string()),
        value: Some(vm_core::config::json_model::ConfigValue::Bool(value)),
        default_value: Some(vm_core::config::json_model::ConfigValue::Bool(value)),
        ..vm_core::config::json_model::RowConfig::default()
    }
}

fn tpanel(
    title: &str,
    rows: Vec<vm_core::config::json_model::RowConfig>,
) -> vm_core::config::json_model::GroupConfig {
    vm_core::config::json_model::GroupConfig {
        title: title.to_string(),
        visible: true,
        rows,
        ..vm_core::config::json_model::GroupConfig::default()
    }
}

/// 数值行 (slider 语义)
fn tint(target: &str, value: i32) -> vm_core::config::json_model::RowConfig {
    vm_core::config::json_model::RowConfig {
        label: target.to_string(),
        r#type: "SLIDER".to_string(),
        property: Some(target.to_string()),
        value: Some(vm_core::config::json_model::ConfigValue::Int(value)),
        default_value: Some(vm_core::config::json_model::ConfigValue::Int(value)),
        ..vm_core::config::json_model::RowConfig::default()
    }
}

/// 字符串行 (combo 语义)
fn tstr(target: &str, value: &str) -> vm_core::config::json_model::RowConfig {
    vm_core::config::json_model::RowConfig {
        label: target.to_string(),
        r#type: "COMBO".to_string(),
        property: Some(target.to_string()),
        value: Some(vm_core::config::json_model::ConfigValue::Str(value.to_string())),
        default_value: Some(vm_core::config::json_model::ConfigValue::Str(value.to_string())),
        ..vm_core::config::json_model::RowConfig::default()
    }
}

/// fixture 树 (crosshairSwitch=true / enableEngineControl=false / autoStart=false)
fn test_panels() -> Vec<vm_core::config::json_model::GroupConfig> {
    vec![tpanel(
        "T",
        vec![
            trow("crosshairSwitch", true),
            trow("enableEngineControl", false),
            trow("autoStartGameMode", false),
        ],
    )]
}

fn auto_start_panels() -> Vec<vm_core::config::json_model::GroupConfig> {
    vec![tpanel("T", vec![trow("autoStartGameMode", true)])]
}

/// AppShell 测试装配: tmp cfg (无 init_config 写盘副作用) + 30ms 短防抖 +
/// **网络隔离** (Service 指向 9 号死端口 — 连接立即拒绝; FM-Detect 探测关闭
/// — 8111 可能被 mock/游戏占用, 项目惯例端口占用即隔离, 不做假通过);
/// 不起渲染线程 — ui_cmd 接收端留在 shell 内供测试观察。
fn fixture() -> AppShell {
    fixture_with_debounce(30)
}

fn fixture_with_debounce(ms: u64) -> AppShell {
    fixture_full(ms, test_panels())
}

/// 全参 fixture (自定义树; 见 fixture_with_debounce 注)
fn fixture_full(ms: u64, panels: Vec<vm_core::config::json_model::GroupConfig>) -> AppShell {
    let ui_bus = Arc::new(vm_core::base::bus::ui_state_bus::UIStateBus::new());
    let config = ConfigurationService::new(Some(Arc::clone(&ui_bus)));
    config.install_for_test(panels, &tmp_cfg("full"));
    let (hotkey, hotkey_rx) = HotkeyManager::with_channel();
    let mut env = Env::probe(&Lang::init_lang(), false);
    env.app_port = 9; // discard 端口: 无服务监听, connect 立即 RST
                      // 字体目录钉在仓库根 (cargo 测试 CWD=crate 根, CWD 相对探测不稳;
                      // 渲染线程注册面的 spec 工厂需要真实字体文件)
    env.fonts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fonts");
    let mut shell = AppShell::with_parts(ShellParts {
        env,
        config,
        ui_bus,
        flight_bus: Arc::new(FlightDataBus::new()),
        fm: Arc::new(FMManager::new(
            Arc::new(vm_core::base::bus::EventBus::new()),
        )),
        hotkey,
        hotkey_rx,
        debounce_delay: Duration::from_millis(ms),
    });
    // 网络探测关闭 (见 fixture 注): FM-Detect/Preview 只走 selectedFM0 兜底
    shell.probe_network_for_test(false);
    shell.rebuild_controller(true);
    shell
}

/// 泵监督事件 (订阅转发 → 主线程处理链), 返回是否处理过事件
fn pump_events(shell: &mut AppShell) -> bool {
    let mut handled = false;
    while let Ok(ev) = shell.main_event_rx.try_recv() {
        shell.handle_main_event(ev);
        handled = true;
    }
    handled
}

fn publish_ui_event(
    bus: &vm_core::base::bus::ui_state_bus::UIStateBus,
    event_type: &str,
    data: &str,
) {
    bus.publish(event_type, Some("MainForm"), Some(data));
}

// ------------------------------------------------------------------
// 状态机: preview → game → (drive) → stop
// ------------------------------------------------------------------

/// 构造 drive_from_live 判定所需的 live ServiceData 快照
/// (真机由 Service.update 写; 测试直填公开字段 — flags/type/playerLive)
fn live_service_data(plane: &str) -> ServiceData {
    let mut st = vm_core::game_api::parser::State::new();
    st.flag = true;
    let mut ind = vm_core::game_api::parser::Indicators::new();
    ind.flag = true;
    ind.r#type = Some(plane.to_string());
    let mut d = ServiceData::default();
    // s_state/s_indic 构造期由 Service::new 建立 (service_fields Default 注);
    // 测试直填 Some
    d.s_state = Some(st);
    d.s_indic = Some(ind);
    d.player_live = true;
    d
}

// ------------------------------------------------------------------
// P6 收口: 注册面 6 窗口条目 / live 喂数全链 / 注册键 ↔ ui_layout.cfg 核对
// ------------------------------------------------------------------

/// 测试用空窗口 (field2 测试 MiniWin 同款; 免真窗依赖)
struct NullWin;

impl vm_overlay::platform::OverlayWindow for NullWin {
    fn present(&mut self, _buf: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn set_position(&mut self, _x: i32, _y: i32) {}
    fn position(&self) -> (i32, i32) {
        (0, 0)
    }
    fn set_click_through(&mut self, _on: bool) {}
    fn poll_event(&mut self) -> Option<vm_overlay::platform::OverlayEvent> {
        None
    }
    fn screen_size(&self) -> (i32, i32) {
        (1920, 1080)
    }
}

fn test_overlay_inputs() -> OverlayInputs {
    OverlayInputs {
        dpi_scale: 1.0,
        hud: HudSettingsSnapshot::build(
            &fixture()
                .controller
                .as_ref()
                .unwrap()
                .config
                .get_hud_settings(),
        ),
        pages: vm_core::config::json_store::factory_pages_arc(),
        font_add_engine: 0,
        font_add_power: 0,
        font_add_flight: 0,
        font_add_gear: 0,
        gear_show_edge: false,
        font_add_axis: 0,
        axis_show_edge: false,
        font_add_fm: 0,
        attitude_width: 150,
        attitude_height: 300,
        attitude_freq_ms: 40,
        attitude_show_direction: false,
        attitude_show_aoa_limits: true,
        service_loop_interval_ms: 50,
        colors: GlobalColors::JAVA_DEFAULT,
        aa: true,
    }
}

// (FM拆包数据 init 几何期望助手 spec_fm_size 已随旧 spec 工厂退役删除
//  — W3: 尺寸面迁 widgets::fm_sidecar 组件 preferred_size)

/// 计数 mock 播放器: start() 按告警键 (wav 文件名去扩展) 计数 — 装配面
/// "告警键触发 play 路径" 的观测探针 (vm-core voice_warning tests 的
/// starts() 同款手法; 此处钉的是 open_voice_warning 装配链:
/// reload→load_clip→play_once→start 的端到端贯通)
struct CountingPlayer {
    plays: Arc<Mutex<HashMap<String, usize>>>,
}
