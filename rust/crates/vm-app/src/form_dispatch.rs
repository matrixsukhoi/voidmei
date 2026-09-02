//! D9 表单 IPC dispatcher (主线程执行体): tauri command (async 线程) → mpsc →
//! [`ShellForm::pump_once`] 内 drain → 本模块 → MainFormState 写链 / AppShell 命令。
//! 组装层单点粘合 `FormMessageDto ↔ Message` (vm-webui 不依赖 vm-ui)。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use vm_core::config_manager;
use vm_core::configuration_service::ConfigurationService;
use vm_ui::main_form::{self, MainFormState, Message};
use vm_app::{AppShell, UiCommand};
use vm_webui::dto::{FormMessageDto, PanelDto};
use vm_webui::ipc::{self, FormRuntime, IpcReply, RequestKind};

/// 主线程共享的表单态 cell (Rc 单线程: dispatcher 与主循环同在主线程;
/// 托盘 rebuild 后由主循环整体替换 — 对位原相 A 每次构造新 MainForm)
pub type FormCell = Rc<RefCell<Option<MainFormState>>>;

/// 构建表单态 (对位原相 A 的 build_form_state: 与当前核共享 ConfigurationService,
/// Arc<ServiceInner> 克隆 = Java tc.configService 单对象语义)
pub fn build_form_state(shell: &AppShell) -> MainFormState {
    let config = shell
        .controller
        .as_ref()
        .map(|c| c.config.clone())
        .unwrap_or_else(|| ConfigurationService::new(Some(Arc::clone(&shell.ui_bus))));
    MainFormState::new(
        config,
        Arc::clone(&shell.ui_bus),
        Some(config_manager::get_user_config_path().to_string()),
    )
}

/// dispatcher 构造 (注入 ShellForm; 主线程调用, 无 Send 约束)
pub fn make_dispatcher(shell: &Arc<Mutex<AppShell>>, cell: FormCell) -> vm_webui::Dispatcher {
    let shell = Arc::clone(shell);
    Box::new(move |kind, rt| dispatch_form(kind, rt, &shell, &cell))
}

/// 请求执行体 (纯流程函数 — 可不开 webview 单测: shell/cell 以真对象驱动)
fn dispatch_form(
    kind: RequestKind,
    rt: &mut FormRuntime,
    shell: &Arc<Mutex<AppShell>>,
    cell: &FormCell,
) -> IpcReply {
    match kind {
        // 壳态请求走默认实现 (Ping/UiReady/WindowEcho)
        RequestKind::Ping { .. } | RequestKind::UiReady | RequestKind::WindowEcho => {
            ipc::dispatch(kind, rt)
        }
        RequestKind::GetLayoutTree => {
            let panels: Vec<PanelDto> = cell
                .borrow()
                .as_ref()
                .map(|f| f.groups().iter().map(Into::into).collect())
                .unwrap_or_default();
            serde_json::to_value(panels).map(IpcReply::Ok).unwrap_or_else(|e| IpcReply::Err(e.to_string()))
        }
        RequestKind::GetComboOptions { source, current } => {
            let borrowed = cell.borrow();
            match borrowed.as_ref() {
                Some(f) => serde_json::to_value(f.options_for(&source, &current))
                    .map(IpcReply::Ok)
                    .unwrap_or_else(|e| IpcReply::Err(e.to_string())),
                None => IpcReply::Err("表单态未初始化 (重建中)".to_string()),
            }
        }
        RequestKind::GetAssetRoot => std::env::current_dir()
            .map(|p| IpcReply::Ok(serde_json::json!(p.to_string_lossy())))
            .unwrap_or_else(|e| IpcReply::Err(e.to_string())),
        RequestKind::FormMessage(dto) => form_message(dto, shell, cell, rt),
        RequestKind::OpenComparisonWindow { fm0, fm1 } => {
            // FMLIST 行 对比按钮 (批3): Java FMListRowRenderer.java:124-144 View 键 —
            // 选中机型单机视图 (fm1 恒 null) 开对比窗; 参数由前端显式传 (对位 Java
            // 按钮体直取 combo 当前项), 不读 cfg。空 fm1 由 web_windows 归一为单机模式
            open_web_window(&WebWindowRequest::Comparison { fm0, fm1 }, rt)
        }
        RequestKind::GetVoicePacks => {
            // Java VoiceResourceManager.getInstance().get_available_packs():
            // "default" + voice/ 子目录。共享实例 = shell.voice (AppShell 字段,
            // Java 单例落位, winmm waveOut 播放器; 试听/告警装配复用同一实例)
            let mgr = Arc::clone(&shell.lock().expect("AppShell 锁中毒").voice);
            serde_json::to_value(mgr.get_available_packs())
                .map(IpcReply::Ok)
                .unwrap_or_else(|e| IpcReply::Err(e.to_string()))
        }
        RequestKind::PreviewVoice { key, pack } => {
            // Java VoiceRowRenderer.java:126-136 试听按钮 (按钮体提取为
            // preview_voice_clip 以注入 mock 播放器断言 load/play 与 pack 传递);
            // 忽略 enable 态 (preview 语义), 失败无声, 回执恒 Ok (Java 按钮无失败反馈面)
            let mgr = Arc::clone(&shell.lock().expect("AppShell 锁中毒").voice);
            let _ = preview_voice_clip(&mgr, &key, &pack); // 保活线程自持至播完
            IpcReply::Ok(serde_json::json!({ "ok": true }))
        }
        RequestKind::GetFmList => {
            // Java FMListRowRenderer:48-62 扫 flightmodels 根的中央文件名 (去扩展);
            // blkx→json 迁移: 只收 .json (data/ 双格式同名并存, 不过滤会重复)
            let dir = vm_core::fm::fm_data_paths::fm_dir();
            let mut names: Vec<String> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if !name.ends_with(".json") {
                        continue;
                    }
                    if let Some(stripped) =
                        vm_core::file_utils::get_file_name_no_ex(Some(&name))
                    {
                        names.push(stripped.to_string());
                    }
                }
            }
            names.sort();
            names.dedup();
            serde_json::to_value(names)
                .map(IpcReply::Ok)
                .unwrap_or_else(|e| IpcReply::Err(e.to_string()))
        }
        RequestKind::ImportConfig { path } => {
            // Java ConfigImportDialog → ConfigManager.importConfig (备份 + 模板哈希合并)。
            // 成功后由 controller 侧重载 + CONFIG_CHANGED 广播 (前端经 config-changed 重拉树)
            let ok = vm_core::config_manager::import_config(&path);
            if ok {
                // 重载服务树 + 快照 (对位 Java import 后 rebuild; 与核共享的 config 服务)
                let mut s = shell.lock().expect("AppShell 锁中毒");
                let user_cfg = vm_core::config_manager::get_user_config_path().to_string();
                if let Some(c) = s.controller.as_mut() {
                    c.config.load_layout(&user_cfg);
                }
                *cell.borrow_mut() = Some(build_form_state(&s));
                drop(s);
                // 广播整树变更 (前端重拉 + overlay 全量刷新, reset 链同款全局键)
                if let Ok(s) = shell.lock() {
                    s.ui_bus.publish(
                        vm_core::event::ui_state_events::CONFIG_CHANGED,
                        Some("ConfigImport"),
                        Some("ui_layout.cfg"),
                    );
                }
                IpcReply::Ok(serde_json::json!({ "ok": true }))
            } else {
                IpcReply::Err(format!("导入失败: {path} (备份已创建, 原配置未动)"))
            }
        }
    }
}

/// 批3 open* 按钮的开窗请求 (Java ButtonRowRenderer 直接 new 窗口的入参面)
#[derive(Debug, Clone, PartialEq)]
pub enum WebWindowRequest {
    /// CompactComparisonWindow(parent, ctr, fm0, fm1)
    Comparison { fm0: String, fm1: Option<String> },
    /// PowerCurveWindow(parent, fm0, fm1, speedKmh, wep)
    PowerCurve { fm0: String, fm1: Option<String>, speed_kmh: i32, wep: bool },
}

/// Java `Integer.parseInt` 语义 (ButtonRowRenderer.java:89-95): 可选 '-' + 纯
/// ASCII 数字 (无 '+', 无空白, 空串非法), 非法/溢出 → 调用方 catch 给 default
fn java_parse_int_or(s: &str, default: i32) -> i32 {
    let digits = s.strip_prefix('-').unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return default;
    }
    s.parse::<i32>().unwrap_or(default) // 溢出: Java 抛异常→catch→default, 同效
}

/// Java `Boolean.parseBoolean`: 仅忽略大小写的 "true" 为真
fn java_parse_boolean(s: &str) -> bool {
    s.eq_ignore_ascii_case("true")
}

/// open* 按钮分派 (Java ButtonRowRenderer.java:64-106 按钮体): 读 cfg 组装开窗
/// 入参; 非 open* 键返回 None (走原表单链)。纯流程函数 — cfg 读写可注入观测。
///
/// cfg 读取对位 Java RenderContext (DynamicDataPage.java:155-174):
/// getString(key, def) = getConfig 为 null/空 → def; getBool(key, false) 同。
fn route_open_action(action: &str, shell: &Arc<Mutex<AppShell>>) -> Option<WebWindowRequest> {
    use vm_core::config_api::ConfigProvider as _;

    let get_string = |key: &str, default: &str| -> String {
        let s = shell
            .lock()
            .expect("AppShell 锁中毒")
            .controller
            .as_ref()
            .and_then(|c| c.config.get_config(key))
            .unwrap_or_default();
        if s.is_empty() { default.to_string() } else { s }
    };

    match action {
        // Java :69-70: selectedFM0 缺省 "a_4h", selectedFM1 缺省 "a6m5_zero"
        "openComparison" => Some(WebWindowRequest::Comparison {
            fm0: get_string("selectedFM0", "a_4h"),
            fm1: Some(get_string("selectedFM1", "a6m5_zero")),
        }),
        // Java :85-98: fm0 缺省 "bf-109f-4", fm1 缺省 ""; speed parseInt 异常→0;
        // wep = Boolean.parseBoolean(powerCurveWep)
        "openPowerCurve" => Some(WebWindowRequest::PowerCurve {
            fm0: get_string("selectedFM0", "bf-109f-4"),
            fm1: Some(get_string("selectedFM1", "")),
            speed_kmh: java_parse_int_or(&get_string("powerCurveSpeed", "0"), 0),
            wep: java_parse_boolean(&get_string("powerCurveWep", "false")),
        }),
        _ => None,
    }
}

/// 开窗执行体: dispatcher 恰在主线程泵内 (ShellForm::pump_once), 满足 tao 建窗
/// 的主线程约束; 无 AppHandle (web 壳不可用/测试形态) 显式 Err 不静默
fn open_web_window(req: &WebWindowRequest, rt: &FormRuntime) -> IpcReply {
    let Some(handle) = rt.app_handle.as_ref() else {
        return IpcReply::Err("web 壳不可用, 无法打开辅助窗口".to_string());
    };
    let res = match req {
        WebWindowRequest::Comparison { fm0, fm1 } => {
            vm_webui::web_windows::open_comparison_window(handle, fm0, fm1.as_deref())
        }
        WebWindowRequest::PowerCurve { fm0, fm1, speed_kmh, wep } => {
            vm_webui::web_windows::open_power_curve_window(
                handle,
                fm0,
                fm1.as_deref(),
                *speed_kmh,
                *wep,
            )
        }
    };
    match res {
        Ok(()) => IpcReply::Ok(serde_json::json!({ "ok": true })),
        Err(e) => IpcReply::Err(e),
    }
}

/// 表单消息: 数据面全链 (WYSIWYG 写回在 update 内闭环);
/// StartGame/EndGame 附带 shell 命令 (对位原 iced 壳 hooks 的 tc 侧序列)。
fn form_message(
    dto: FormMessageDto,
    shell: &Arc<Mutex<AppShell>>,
    cell: &FormCell,
    rt: &FormRuntime,
) -> IpcReply {
    let msg = to_message(dto);
    // 批3: open* 两键在表单写链前拦截 — Java ButtonRowRenderer 直接开窗 (无确认
    // 模态/无表单副作用); vm-ui main_form 对 open* 只 warn+Ignore, 放行会丢动作
    if let Message::ButtonAction { action } = &msg {
        if let Some(req) = route_open_action(action, shell) {
            return open_web_window(&req, rt);
        }
    }
    match &msg {
        Message::StartGame | Message::EndGame => {
            // 保存链先行 (Java MainForm.confirm/mCancel 的 saveConfig), 再 tc 侧命令
            if let Some(f) = cell.borrow_mut().as_mut() {
                main_form::update(f, msg.clone());
            }
            let cmd = match &msg {
                Message::StartGame => UiCommand::StartGame,
                _ => UiCommand::EndGame,
            };
            if let Ok(mut s) = shell.lock() {
                s.dispatch(cmd);
            }
            IpcReply::Ok(serde_json::json!({ "ok": true }))
        }
        _ => {
            let mut borrowed = cell.borrow_mut();
            match borrowed.as_mut() {
                Some(f) => {
                    main_form::update(f, msg);
                    IpcReply::Ok(serde_json::json!({ "ok": true }))
                }
                None => IpcReply::Err("表单态未初始化 (重建中)".to_string()),
            }
        }
    }
}

/// Java VoiceRowRenderer.java:128-136 ▶ 按钮体: pKey = stripVoicePrefix(property),
/// clip = loadClip(pKey, 当前选中包), 非 null → setFramePosition(0) + start。
/// clip==null 静默返回 (Java 无声失败, 不弹错误); // ignoring enable state for preview
/// (试听无视 enable 开关)。提取为独立纯流程函数 — 可注入 mock SoundPlayer 断言
/// load/play 调用序列与 pack 传递 (AppShell 的共享实例持 winmm 播放器, 不可 mock)。
///
/// 返回保活线程 JoinHandle (审查 B-B1 修复): Java 局部 clip 引用出作用域后
/// 原生 line 靠 GC finalizer 非确定性延迟释放而自然播完; Rust 确定性 Drop
/// (RAII close → waveOutReset+Close) 会掐断刚提交的播放 — clip 交
/// [`vm_core::voice_warning::hold_clip_until_done`] 持至播完 (对位 GC 延迟
/// 语义)。生产调用点忽略返回值; 测试 join 后断言收尾。
fn preview_voice_clip(
    mgr: &vm_core::voice_resource_manager::VoiceResourceManager,
    key: &str,
    pack: &str,
) -> Option<std::thread::JoinHandle<()>> {
    let p_key =
        vm_core::audio::VoicePackConfig::strip_voice_prefix(Some(key)).unwrap_or_default();
    if let Some(clip) = mgr.load_clip(&p_key, Some(pack)) {
        clip.set_frame_position(0);
        clip.start();
        Some(vm_core::voice_warning::hold_clip_until_done(clip))
    } else {
        None
    }
}

/// dto → Message (一一对应; 组装层单点)
fn to_message(dto: FormMessageDto) -> Message {
    match dto {
        FormMessageDto::Toggle { panel, key, value } => Message::Toggle { panel, key, value },
        FormMessageDto::Slider { panel, key, value } => Message::Slider { panel, key, value },
        FormMessageDto::Combo { panel, key, value } => Message::Combo { panel, key, value },
        FormMessageDto::ColorPicked { panel, key, value } => Message::ColorPicked {
            panel,
            key,
            value,
        },
        FormMessageDto::Save => Message::Save,
        FormMessageDto::StartGame => Message::StartGame,
        FormMessageDto::EndGame => Message::EndGame,
        FormMessageDto::RefreshPreviews => Message::RefreshPreviews,
        FormMessageDto::ButtonAction { action } => Message::ButtonAction { action },
        FormMessageDto::ConfirmPending => Message::ConfirmPending,
        FormMessageDto::CancelPending => Message::CancelPending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// dto→Message 映射完整性: IPC 序列正确性的前提 (与 iced 基线 diff=0 验收配套)
    #[test]
    fn to_message_全变体逐字段映射() {
        assert!(matches!(
            to_message(FormMessageDto::Toggle { panel: "p".into(), key: "k".into(), value: true }),
            Message::Toggle { panel, key, value: true } if panel == "p" && key == "k"
        ));
        assert!(matches!(
            to_message(FormMessageDto::Slider { panel: "p".into(), key: "k".into(), value: 42 }),
            Message::Slider { value: 42, .. }
        ));
        assert!(matches!(
            to_message(FormMessageDto::Combo { panel: "p".into(), key: "k".into(), value: "v".into() }),
            Message::Combo { value, .. } if value == "v"
        ));
        assert!(matches!(
            to_message(FormMessageDto::ColorPicked { panel: "p".into(), key: "k".into(), value: [1, 2, 3, 4] }),
            Message::ColorPicked { value: [1, 2, 3, 4], .. }
        ));
        assert!(matches!(to_message(FormMessageDto::Save), Message::Save));
        assert!(matches!(to_message(FormMessageDto::StartGame), Message::StartGame));
        assert!(matches!(to_message(FormMessageDto::EndGame), Message::EndGame));
        assert!(matches!(to_message(FormMessageDto::RefreshPreviews), Message::RefreshPreviews));
        assert!(matches!(
            to_message(FormMessageDto::ButtonAction { action: "resetConfig".into() }),
            Message::ButtonAction { action } if action == "resetConfig"
        ));
        assert!(matches!(to_message(FormMessageDto::ConfirmPending), Message::ConfirmPending));
        assert!(matches!(to_message(FormMessageDto::CancelPending), Message::CancelPending));
    }

    /// 最小壳装配 (app_shell tests fixture 的 bin 侧本地版 — dispatcher 需真 shell)
    /// PORT(allow arc_with_non_send_sync): main.rs 同款 — Arc 复刻 Java this
    /// 引用共享 (dispatcher 注入面), 不为 lint 改 Rc
    #[allow(clippy::arc_with_non_send_sync)]
    fn min_shell() -> Arc<Mutex<AppShell>> {
        // 最小 cfg (原内联文本; tag 与 open* 测试的 tmp 文件互不覆盖)
        shell_with_cfg(
            "(panel \"T\" :visible true\n\
             \x20 (item \"auto\" :type switch :target \"autoStartGameMode\" :value false))\n\
            ",
            "min",
        )
    }

    /// 按给定 cfg 文本建壳 (min_shell 的可配置版; 测试并行各自独立 tmp 文件)
    #[allow(clippy::arc_with_non_send_sync)]
    fn shell_with_cfg(cfg_text: &str, tag: &str) -> Arc<Mutex<AppShell>> {
        use vm_app::ShellParts;
        let ui_bus = Arc::new(vm_core::ui_state_bus::UIStateBus::new());
        let config = ConfigurationService::new(Some(Arc::clone(&ui_bus)));
        let cfg = std::env::temp_dir().join(format!(
            "vm_app_formdisp_{tag}_{}.cfg",
            std::process::id()
        ));
        std::fs::write(&cfg, cfg_text).unwrap();
        config.load_layout(cfg.to_str().unwrap());
        let (hotkey, hotkey_rx) = vm_overlay::HotkeyManager::with_channel();
        let env = vm_app::Env::probe(&vm_core::lang::Lang::init_lang(), false);
        Arc::new(Mutex::new(AppShell::with_parts(ShellParts {
            env,
            config,
            ui_bus,
            flight_bus: Arc::new(vm_core::flight_data_bus::FlightDataBus::new()),
            fm: Arc::new(vm_core::fm::FMManager::new(Arc::new(vm_core::bus::EventBus::new()))),
            hotkey,
            hotkey_rx,
            debounce_delay: std::time::Duration::from_millis(30),
        })))
    }

    /// GetVoicePacks 走共享实例 shell.voice (Java getInstance() 单例; NoopPlayer
    /// 已退役, 播放器为 winmm waveOut 腿): 列表含 default + voice/ 子目录
    #[test]
    fn get_voice_packs_走共享实例含_default() {
        let shell = min_shell();
        let cell: FormCell = Rc::new(RefCell::new(None));
        let mut disp = make_dispatcher(&shell, Rc::clone(&cell));
        let mut rt = FormRuntime::default();
        let reply = disp(RequestKind::GetVoicePacks, &mut rt);
        let IpcReply::Ok(v) = reply else {
            panic!("期望 Ok: {reply:?}")
        };
        let arr = v.as_array().expect("语音包列表应为 JSON 数组");
        assert!(
            arr.iter().any(|p| p == "default"),
            "必须含 default (Java getAvailablePacks 恒含): {v}"
        );
    }

    /// 试听 (Java VoiceRowRenderer.java:126-136): voice_ 前缀 strip 后经共享
    /// 实例 load_clip; CWD 无 voice/<key>.wav → clip==null 静默 (Java 同款无声
    /// 失败), 回执恒 Ok — 不假成功也不报错 (Java 按钮无失败反馈面)
    #[test]
    fn preview_voice_缺失文件_静默ok() {
        let shell = min_shell();
        let cell: FormCell = Rc::new(RefCell::new(None));
        let mut disp = make_dispatcher(&shell, Rc::clone(&cell));
        let mut rt = FormRuntime::default();
        let reply = disp(
            RequestKind::PreviewVoice {
                key: "voice_aoaCrit".into(),
                pack: "default".into(),
            },
            &mut rt,
        );
        assert!(
            matches!(reply, IpcReply::Ok(_)),
            "缺失文件应静默 Ok (Java clip==null 无声): {reply:?}"
        );
    }

    // ------------------------------------------------------------------
    // mock SoundPlayer 面向: preview_voice_clip 的 strip/load/play/pack 断言
    // (共享实例持 winmm 播放器不可 mock — 按钮体已提取为独立纯流程函数)
    // ------------------------------------------------------------------

    /// mock SoundClip: 按序记录控制面调用 ("seek:N"/"start"/…);
    /// 音量控件不支持 (master_gain_range=None → applyVolume 空 catch 路径, §2.7)。
    /// 履行 trait 文档的 RAII 兜底契约 (Drop 等价 close, WaveOutClip 同款,
    /// 审查 B-B2); close 幂等 (closed 标志) — 保活线程显式 close 后 Drop 兜底
    /// 不再双记, "恰好一次 close" 断言语义保持
    struct MockClip {
        calls: Arc<Mutex<Vec<String>>>,
        closed: std::sync::atomic::AtomicBool,
    }
    impl vm_core::voice_resource_manager::SoundClip for MockClip {
        fn start(&self) {
            self.calls.lock().unwrap().push("start".into());
        }
        fn stop(&self) {
            self.calls.lock().unwrap().push("stop".into());
        }
        fn is_running(&self) -> bool {
            false
        }
        fn set_frame_position(&self, frame: i32) {
            self.calls.lock().unwrap().push(format!("seek:{frame}"));
        }
        fn close(&self) {
            if self
                .closed
                .swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                return; // 已 close 再 close 无副作用 (Java line close 状态机)
            }
            self.calls.lock().unwrap().push("close".into());
        }
        fn master_gain_range(&self) -> Option<(f32, f32)> {
            None // Control not supported
        }
        fn set_master_gain(&self, _value: f32) {
            self.calls.lock().unwrap().push("gain".into());
        }
    }
    impl Drop for MockClip {
        fn drop(&mut self) {
            // RAII 兜底契约 (trait 文档); trait 方法全限定调用 (模块未 use SoundClip)
            vm_core::voice_resource_manager::SoundClip::close(self);
        }
    }

    /// mock SoundPlayer: 记录 open_clip 收到的解析路径 (pack 传递的观测面)
    struct MockPlayer {
        opened: Arc<Mutex<Vec<std::path::PathBuf>>>,
        calls: Arc<Mutex<Vec<String>>>,
    }
    impl vm_core::voice_resource_manager::SoundPlayer for MockPlayer {
        fn open_clip(
            &self,
            path: &std::path::Path,
        ) -> Result<
            Box<dyn vm_core::voice_resource_manager::SoundClip>,
            vm_core::voice_resource_manager::SoundError,
        > {
            self.opened.lock().unwrap().push(path.to_path_buf());
            Ok(Box::new(MockClip {
                calls: Arc::clone(&self.calls),
                closed: std::sync::atomic::AtomicBool::new(false),
            }))
        }
    }

    /// mock_voice_mgr 的返回束: (管理器, open_clip 路径记录, 控制面调用记录, voice 根)
    /// — 四元组直书触发 clippy type_complexity, 别名收口
    type MockVoiceFixture = (
        vm_core::voice_resource_manager::VoiceResourceManager,
        Arc<Mutex<Vec<std::path::PathBuf>>>,
        Arc<Mutex<Vec<String>>>,
        std::path::PathBuf,
    );

    /// 临时 voice 根 + mock 管理器 (new_with_voice_dir 测试注入先例)
    fn mock_voice_mgr() -> MockVoiceFixture {
        let root = std::env::temp_dir().join(format!("vm_preview_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("jarvis")).unwrap();
        // default 根与 jarvis/ 各放一份 (pack 解析优先级观测)
        std::fs::write(root.join("aoaCrit.wav"), b"").unwrap();
        std::fs::write(root.join("jarvis/aoaCrit.wav"), b"").unwrap();
        let opened: Arc<Mutex<Vec<std::path::PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
        let calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let mgr = vm_core::voice_resource_manager::VoiceResourceManager::new_with_voice_dir(
            Box::new(MockPlayer {
                opened: Arc::clone(&opened),
                calls: Arc::clone(&calls),
            }),
            root.to_string_lossy().into_owned(),
        );
        (mgr, opened, calls, root)
    }

    /// 试听按钮体全链: voice_ 前缀 strip + pack 传达到文件解析 +
    /// setFramePosition(0) 先于 start (Java 按钮序, 逐句对齐)。
    /// B-B1 后按钮序多了保活收尾 close (播完后释放设备) — join 保活线程
    /// 消除与断言的竞态, 再核对完整调用序
    #[test]
    fn preview_voice_clip_strip前缀_load_play_与pack传递() {
        let (mgr, opened, calls, root) = mock_voice_mgr();
        let hold = preview_voice_clip(&mgr, "voice_aoaCrit", "jarvis");
        {
            let o = opened.lock().unwrap();
            assert_eq!(o.len(), 1, "恰好一次 load (Java loadClip 单调用)");
            assert!(
                o[0].ends_with(std::path::Path::new("jarvis").join("aoaCrit.wav")),
                "pack 必须传达到文件解析 (voice/jarvis/aoaCrit.wav): {}",
                o[0].display()
            );
        }
        hold.expect("clip 加载成功应有保活线程").join().unwrap();
        // 完整序 (join 后无竞态): seek 先于 start (Java 按钮序) + 保活收尾
        // close (mock is_running 恒 false → 保活线程立即收尾, B-B1 对位
        // GC finalizer 延迟释放)
        assert_eq!(
            *calls.lock().unwrap(),
            vec!["seek:0".to_string(), "start".to_string(), "close".to_string()],
            "按钮序: setFramePosition(0) → start → 播完 close"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// pack 无该文件时回退 default 根 (Java resolveAudioFile 第 2 步) — pack 仍传递
    #[test]
    fn preview_voice_clip_pack缺失回退default根() {
        let (mgr, opened, calls, root) = mock_voice_mgr();
        let hold = preview_voice_clip(&mgr, "voice_aoaCrit", "nosuch");
        {
            let o = opened.lock().unwrap();
            assert_eq!(o.len(), 1, "回退路径也只 load 一次");
            assert!(
                o[0].ends_with(std::path::Path::new("aoaCrit.wav"))
                    && !o[0].components().any(|c| c.as_os_str() == "nosuch"),
                "应回退 voice/aoaCrit.wav 而非 voice/nosuch/: {}",
                o[0].display()
            );
        }
        hold.expect("回退命中应有保活线程").join().unwrap();
        assert_eq!(
            *calls.lock().unwrap(),
            vec!["seek:0".to_string(), "start".to_string(), "close".to_string()],
            "回退路径照常播放 + 保活收尾同款"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 键与包全缺失 → 零调用零 panic (clip==null 静默, Java 无声失败)
    #[test]
    fn preview_voice_clip_全缺失_零调用静默() {
        let (mgr, opened, calls, root) = mock_voice_mgr();
        let hold = preview_voice_clip(&mgr, "voice_nosuch", "default");
        assert!(
            hold.is_none(),
            "clip==null 无保活线程 (Java 无声失败面)"
        );
        assert!(
            opened.lock().unwrap().is_empty(),
            "文件不存在不得触达播放器 (resolve 阶段即 None)"
        );
        assert!(
            calls.lock().unwrap().is_empty(),
            "无 clip 则无任何控制面调用"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// B-B1 回归锚 (fire-and-forget 存活性): 在播 clip 不得被立即 Drop 关闭 —
    /// 原 bug 形态 = preview_voice_clip 返回即 drop → close, 提交的播放被掐断
    /// (真实 winmm 播放器下试听无声; mock clip 的 drop 无副作用故旧测试探测
    /// 不到)。LatchClip 的 is_running 由测试侧控制: 起播后 150ms 内 close 不得
    /// 发生 (close 只会由保活线程在 is_running 翻 false / 60s 超时后调用,
    /// 正常实现下无竞态), 翻 false + join 后 close 恰好一次
    #[test]
    fn preview_voice_clip_在播期间不被提前close() {
        use std::sync::atomic::{AtomicBool, Ordering};

        struct LatchClip {
            running: Arc<AtomicBool>,
            closed: Arc<AtomicBool>,
            calls: Arc<Mutex<Vec<String>>>,
        }
        impl vm_core::voice_resource_manager::SoundClip for LatchClip {
            fn start(&self) {
                self.calls.lock().unwrap().push("start".into());
            }
            fn stop(&self) {}
            fn is_running(&self) -> bool {
                self.running.load(Ordering::SeqCst)
            }
            fn set_frame_position(&self, frame: i32) {
                self.calls
                    .lock()
                    .unwrap()
                    .push(format!("seek:{frame}"));
            }
            fn close(&self) {
                // swap 兼做幂等闸 (首调返回 false): 保活线程显式 close 后
                // Drop 兜底不再双记 "close" (MockClip 同款契约)
                if self.closed.swap(true, Ordering::SeqCst) {
                    return;
                }
                self.calls.lock().unwrap().push("close".into());
            }
            fn master_gain_range(&self) -> Option<(f32, f32)> {
                None
            }
            fn set_master_gain(&self, _value: f32) {}
        }
        impl Drop for LatchClip {
            fn drop(&mut self) {
                // RAII 兜底契约 (trait 文档); trait 方法全限定调用 (测试 fn 内无 use)
                vm_core::voice_resource_manager::SoundClip::close(self);
            }
        }
        struct LatchPlayer {
            running: Arc<AtomicBool>,
            closed: Arc<AtomicBool>,
            calls: Arc<Mutex<Vec<String>>>,
        }
        impl vm_core::voice_resource_manager::SoundPlayer for LatchPlayer {
            fn open_clip(
                &self,
                _path: &std::path::Path,
            ) -> Result<
                Box<dyn vm_core::voice_resource_manager::SoundClip>,
                vm_core::voice_resource_manager::SoundError,
            > {
                Ok(Box::new(LatchClip {
                    running: Arc::clone(&self.running),
                    closed: Arc::clone(&self.closed),
                    calls: Arc::clone(&self.calls),
                }))
            }
        }

        let root = std::env::temp_dir().join(format!("vm_b1latch_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("aoaCrit.wav"), b"").unwrap();
        let running = Arc::new(AtomicBool::new(true));
        let closed = Arc::new(AtomicBool::new(false));
        let calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let mgr = vm_core::voice_resource_manager::VoiceResourceManager::new_with_voice_dir(
            Box::new(LatchPlayer {
                running: Arc::clone(&running),
                closed: Arc::clone(&closed),
                calls: Arc::clone(&calls),
            }),
            root.to_string_lossy().into_owned(),
        );

        let hold = preview_voice_clip(&mgr, "voice_aoaCrit", "default")
            .expect("应加载 clip 并起保活线程");
        // 在播窗口内 (running=true): close 不得发生 — 若 clip 被函数尾 Drop
        // (原 bug), 150ms 时 close 必已置位
        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(
            !closed.load(Ordering::SeqCst),
            "在播 clip 不得被提前 close (B-B1: Drop 掐断 fire-and-forget 播放)"
        );
        assert_eq!(
            *calls.lock().unwrap(),
            vec!["seek:0".to_string(), "start".to_string()],
            "在播窗口内只有 seek+start"
        );
        // 播完 (is_running 翻 false) → 保活线程收尾 close
        running.store(false, Ordering::SeqCst);
        hold.join().unwrap();
        assert!(
            closed.load(Ordering::SeqCst),
            "播完后保活线程应 close 释放设备"
        );
        assert_eq!(
            *calls.lock().unwrap(),
            vec!["seek:0".to_string(), "start".to_string(), "close".to_string()],
            "完整序: seek → start → (播完) close"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // ------------------------------------------------------------------
    // 批3: open* 按钮分派 (route_open_action 读 cfg 组装开窗入参) + 表单链前拦截
    // ------------------------------------------------------------------

    /// 建核 (route_open_action 读 controller.config — with_parts 不建核, 须显式
    /// rebuild; 注入的 tmp cfg 被首核原样复用, 无写盘副作用)
    fn with_controller(shell: Arc<Mutex<AppShell>>) -> Arc<Mutex<AppShell>> {
        shell
            .lock()
            .expect("AppShell 锁中毒")
            .rebuild_controller(true);
        shell
    }

    /// OPEN_ROWS_CFG 壳 + 建核 (set_config 只改树中已有行 — vm-core
    /// ServiceInner.set_config 逐行匹配的 Java 保真语义, 故 set_cfg 透传
    /// 断言需行在位; 缺行→缺省分支用 with_controller(min_shell()) 单独断)
    fn min_shell_with_controller() -> Arc<Mutex<AppShell>> {
        with_controller(shell_with_cfg(OPEN_ROWS_CFG, "openrows"))
    }

    /// open* 相关键在位的 cfg (对位 ui_layout.cfg:272-278 的 fmlist/slider/switch
    /// 行 — set_config 只改树中已有行, 缺行的键走 Java 硬缺省分支)
    const OPEN_ROWS_CFG: &str = concat!(
        "(panel \"FM数据对比\" :visible true\n",
        "  (item \"FM 0\" :type fmlist :target \"selectedFM0\" :value \"spitfire_f24\")\n",
        "  (item \"FM 1\" :type fmlist :target \"selectedFM1\" :value \"p-51c-10-nt\")\n",
        "  (item \"选定速度\" :type slider :target \"powerCurveSpeed\" :min 0 :max 800 :value 350)\n",
        "  (item \"WEP模式\" :type switch :target \"powerCurveWep\" :value false))\n",
    );

    /// 写壳内核 cfg 键 (controller 与表单态共享同一 ConfigurationService —
    /// 对位 Java ButtonRowRenderer 经 RenderContext 读 configService)
    fn set_cfg(shell: &Arc<Mutex<AppShell>>, key: &str, value: &str) {
        use vm_core::config_api::ConfigProvider as _;
        let s = shell.lock().expect("AppShell 锁中毒");
        let c = s.controller.as_ref().expect("rebuild 后应有核");
        c.config.set_config(key, value);
    }

    /// openComparison 读 cfg: 缺键/空值 → Java ButtonRowRenderer.java:69-70 缺省对
    /// (a_4h / a6m5_zero); 行在位非空 → 透传
    #[test]
    fn route_open_action_对比窗口_cfg与缺省() {
        // 缺行壳 → 缺省对 (get_config 空串 → route_open_action 的缺省回退)
        let bare = with_controller(min_shell());
        assert_eq!(
            route_open_action("openComparison", &bare),
            Some(WebWindowRequest::Comparison {
                fm0: "a_4h".into(),
                fm1: Some("a6m5_zero".into())
            })
        );
        // 行在位壳 → 行值透传 (OPEN_ROWS_CFG 的 fmlist 行)
        let shell = min_shell_with_controller();
        assert_eq!(
            route_open_action("openComparison", &shell),
            Some(WebWindowRequest::Comparison {
                fm0: "spitfire_f24".into(),
                fm1: Some("p-51c-10-nt".into())
            })
        );
        // fm1 显式清空 → 仍回缺省 (Java getStringFromConfigService 空串→default
        // 语义, DynamicDataPage.java:169-174 / RenderContext 各实现一致 — 单机
        // 模式 Some("") 在 Java openComparison 路径不可达, 修正中断 agent 的假设)
        set_cfg(&shell, "selectedFM1", "");
        assert_eq!(
            route_open_action("openComparison", &shell),
            Some(WebWindowRequest::Comparison {
                fm0: "spitfire_f24".into(),
                fm1: Some("a6m5_zero".into())
            })
        );
        // 非 open* 键不分派 (resetConfig 走原确认模态链)
        assert_eq!(route_open_action("resetConfig", &shell), None);
        assert_eq!(route_open_action("factoryReset", &shell), None);
        assert_eq!(route_open_action("importConfig", &shell), None);
    }

    /// openPowerCurve 读 cfg (ButtonRowRenderer.java:85-98): 缺省 bf-109f-4 / "" /
    /// speed 0 / wep false; speed 非法串 parseInt 异常→0; wep 仅 "true" (忽略
    /// 大小写) 为真 — Boolean.parseBoolean 语义 ("1" 为 false)
    #[test]
    fn route_open_action_功率曲线_缺省与解析容错() {
        // PowerCurve 请求字段解构 (variant 无结构更新语法, 逐字段断言)
        let params = |req: Option<WebWindowRequest>| -> (String, Option<String>, i32, bool) {
            match req {
                Some(WebWindowRequest::PowerCurve { fm0, fm1, speed_kmh, wep }) => {
                    (fm0, fm1, speed_kmh, wep)
                }
                other => panic!("期望 PowerCurve 请求: {other:?}"),
            }
        };
        // 缺行壳 → Java 缺省 (bf-109f-4 / "" / 0 / false)
        let bare = with_controller(min_shell());
        assert_eq!(
            params(route_open_action("openPowerCurve", &bare)),
            ("bf-109f-4".into(), Some(String::new()), 0, false)
        );
        // 行在位壳 → fmlist 行值 + slider 350 透传 (wep 行 false)
        let shell = min_shell_with_controller();
        assert_eq!(
            params(route_open_action("openPowerCurve", &shell)),
            ("spitfire_f24".into(), Some("p-51c-10-nt".into()), 350, false)
        );
        set_cfg(&shell, "selectedFM0", "spitfire_f24");
        set_cfg(&shell, "selectedFM1", "p-51c-10-nt");
        set_cfg(&shell, "powerCurveSpeed", "350");
        set_cfg(&shell, "powerCurveWep", "true");
        assert_eq!(
            params(route_open_action("openPowerCurve", &shell)),
            ("spitfire_f24".into(), Some("p-51c-10-nt".into()), 350, true)
        );
        // parseInt 异常面: 非数字 / 空白 / '+' 前缀 → 0 (Java 均抛 NumberFormatException)
        for bad in ["abc", " 350", "+350"] {
            set_cfg(&shell, "powerCurveSpeed", bad);
            assert_eq!(
                params(route_open_action("openPowerCurve", &shell)).2,
                0,
                "非法速度串应回 0: {bad}"
            );
        }
        // Boolean.parseBoolean: "1" 非 true; "TRUE" 忽略大小写真
        set_cfg(&shell, "powerCurveSpeed", "350");
        set_cfg(&shell, "powerCurveWep", "1");
        assert!(!params(route_open_action("openPowerCurve", &shell)).3, "\"1\" 应为 false");
        set_cfg(&shell, "powerCurveWep", "TRUE");
        assert!(params(route_open_action("openPowerCurve", &shell)).3, "\"TRUE\" 应为 true");
    }

    /// 拦截面: open* ButtonAction 不再落 main_form::update — 无 webview 形态
    /// (rt 无 AppHandle) 显式 Err("web 壳不可用"); 对照组 resetConfig 走表单链
    /// (cell 空时报"表单态未初始化", 与开窗 Err 不同源 → 拦截路径可区分),
    /// Ping 证明 dispatcher 本身健在
    #[test]
    fn dispatch_open动作_表单链前拦截() {
        let shell = min_shell();
        let cell: FormCell = Rc::new(RefCell::new(None));
        let mut disp = make_dispatcher(&shell, Rc::clone(&cell));
        let mut rt = FormRuntime::default(); // 无 app_handle (不开 webview 的测试形态)
        for action in ["openComparison", "openPowerCurve"] {
            let reply = disp(
                RequestKind::FormMessage(FormMessageDto::ButtonAction { action: action.into() }),
                &mut rt,
            );
            match &reply {
                IpcReply::Err(e) => assert!(
                    e.contains("web 壳不可用"),
                    "拦截后应报 web 壳缺失而非静默 Ok: {e}"
                ),
                IpcReply::Ok(_) => panic!("{action} 放行表单链会丢开窗动作 (应拦截): {reply:?}"),
            }
        }
        // FMLIST 对比按钮链 (显式参数版) 同款
        let reply = disp(
            RequestKind::OpenComparisonWindow { fm0: "spitfire_f24".into(), fm1: None },
            &mut rt,
        );
        assert!(
            matches!(reply, IpcReply::Err(ref e) if e.contains("web 壳不可用")),
            "开窗请求无 webview 应显式 Err: {reply:?}"
        );
        // 对照组 1: 非开窗按钮动作走原表单链 (cell 空的 Err 与开窗 Err 不同文案)
        let reply = disp(
            RequestKind::FormMessage(FormMessageDto::ButtonAction { action: "resetConfig".into() }),
            &mut rt,
        );
        assert!(
            matches!(reply, IpcReply::Err(ref e) if e.contains("表单态未初始化")),
            "resetConfig 应走表单链: {reply:?}"
        );
        // 对照组 2: 壳态请求照常 Ok (dispatcher 健在)
        let reply = disp(RequestKind::Ping { nonce: 1 }, &mut rt);
        assert!(matches!(reply, IpcReply::Ok(_)), "Ping 不受拦截影响: {reply:?}");
    }

    /// java_parse_int_or 边界: 负数 / 溢出 / 空串 (Java parseInt 抛异常 → catch 0)
    #[test]
    fn java_parse_int_边界() {
        assert_eq!(java_parse_int_or("-25", 7), -25);
        assert_eq!(java_parse_int_or("", 7), 7);
        assert_eq!(java_parse_int_or("-", 7), 7);
        assert_eq!(java_parse_int_or("99999999999", 7), 7); // 溢出 → 异常 → 7
        assert_eq!(java_parse_int_or("0", 7), 0);
    }
}
