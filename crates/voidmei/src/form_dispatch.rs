//! D9 表单 IPC dispatcher (主线程执行体): tauri command (async 线程) → mpsc →
//! [`ShellForm::pump_once`] 内 drain → 本模块 → MainFormState 写链 / AppShell 命令。
//! 组装层单点粘合 `FormMessageDto ↔ Message` (webui 不依赖 ui)。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::{AppShell, UiCommand};
use kernel::base::java_compat::java_parse_boolean;
use kernel::config::configuration_service::ConfigurationService;
use ui::main_form::{self, MainFormState, Message};
use webui::dto::FormMessageDto;
use webui::ipc::{self, FormRuntime, IpcReply, RequestKind};

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
    MainFormState::new(config, Arc::clone(&shell.ui_bus))
}

/// dispatcher 构造 (注入 ShellForm; 主线程调用, 无 Send 约束)
pub fn make_dispatcher(shell: &Rc<RefCell<AppShell>>, cell: FormCell) -> webui::Dispatcher {
    let shell = Rc::clone(shell);
    Box::new(move |kind, rt| dispatch_form(kind, rt, &shell, &cell))
}

/// 请求执行体 (纯流程函数 — 可不开 webview 单测: shell/cell 以真对象驱动)
fn dispatch_form(
    kind: RequestKind,
    rt: &mut FormRuntime,
    shell: &Rc<RefCell<AppShell>>,
    cell: &FormCell,
) -> IpcReply {
    match kind {
        // 壳态请求走默认实现 (UiReady/WindowEcho)
        RequestKind::UiReady | RequestKind::WindowEcho => ipc::dispatch(kind, rt),
        RequestKind::GetLayoutTree => {
            // serde 模型直出 (Phase 1: dto 映射层退役)
            let panels = cell
                .borrow()
                .as_ref()
                .map(|f| f.groups().to_vec())
                .unwrap_or_default();
            serde_json::to_value(panels)
                .map(IpcReply::Ok)
                .unwrap_or_else(|e| IpcReply::Err(e.to_string()))
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
        // ---- W4 HUD 布局编辑器 ----
        RequestKind::GetComponentCatalog => {
            use overlay::widgets::registry::PropKind;
            let catalog: Vec<_> = overlay::widgets::widget_registry()
                .iter()
                .map(|m| {
                    let schema: Vec<_> = m
                        .props_schema
                        .iter()
                        .map(|p| {
                            let mut o = serde_json::json!({
                                "key": p.key,
                                "displayZh": p.display_zh,
                                "kind": match p.kind {
                                    PropKind::Bool => "Bool",
                                    PropKind::Int => "Int",
                                    PropKind::Str => "Str",
                                    PropKind::Color => "Color",
                                    PropKind::Target => "Target",
                                    PropKind::Enum(_) => "Enum",
                                },
                            });
                            if let PropKind::Enum(values) = p.kind {
                                o["values"] = serde_json::json!(values);
                            }
                            o
                        })
                        .collect();
                    serde_json::json!({
                        "typeName": m.type_name,
                        "displayZh": m.display_zh,
                        "category": format!("{:?}", m.category),
                        "composite": m.composite,
                        "configKeys": m.config_keys,
                        "dataShorts": m.data_shorts,
                        "propsSchema": schema,
                        // palette 新建初值 (const JSON 文本 → 对象直传; 空值工厂
                        // Err → 组件静默不建是 P0 静默失败问题的根源)
                        "defaultProps": serde_json::from_str::<serde_json::Value>(m.default_props)
                            .unwrap_or_else(|_| serde_json::json!({})),
                    })
                })
                .collect();
            // 常用字段预设 (出厂页 data.field/engine.gauge 组件原样导出 — palette
            // 直接列「表速」「节流阀」…, 点击即完整配置组件; 单一数据源)
            let lang = kernel::lang::Lang::init_lang();
            let engine_names: std::collections::HashMap<&str, String> =
                overlay::overlays::engine_control::ENGINE_GAUGE_DEFS
                    .iter()
                    .map(|def| (def.key, (def.label)(&lang).to_string()))
                    .collect();
            let field_presets: Vec<_> = kernel::config::json_store::factory()
                .pages
                .iter()
                .filter(|p| {
                    p.id == "flight-info-default" || p.id == "power-info-default"
                })
                .flat_map(|p| p.components.iter())
                .filter(|c| c.r#type == "core.data.field")
                .map(|c| {
                    serde_json::json!({
                        "label": c.props.get("label").and_then(|v| v.as_str()).unwrap_or(&c.id),
                        "props": c.props,
                    })
                })
                .chain(
                    // 引擎仪表预设 (7 仪表中文名; 出厂引擎页同款 props)
                    overlay::overlays::engine_control::ENGINE_GAUGE_DEFS
                        .iter()
                        .map(|def| {
                            serde_json::json!({
                                "label": engine_names.get(def.key).cloned().unwrap_or_else(|| def.key.to_string()),
                                "props": serde_json::json!({ "kind": def.key }),
                            })
                        }),
                )
                .collect();
            serde_json::to_value(serde_json::json!({
                "components": catalog,
                "fieldPresets": field_presets,
            }))
            .map(IpcReply::Ok)
            .unwrap_or_else(|e| IpcReply::Err(e.to_string()))
        }
        RequestKind::GetPages => {
            let s = shell.borrow();
            let config = s
                .controller
                .as_ref()
                .map(|c| c.config.clone())
                .unwrap_or_else(|| ConfigurationService::new(Some(Arc::clone(&s.ui_bus))));
            let pages = config.pages();
            let factory_ids: Vec<String> = kernel::config::json_store::factory()
                .pages
                .iter()
                .map(|p| p.id.clone())
                .collect();
            let hints = config.page_upgrade_hints();
            let list: Vec<_> = pages
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "id": p.id,
                        "name": p.name,
                        "activation": p.activation,
                        "isFactory": factory_ids.contains(&p.id),
                        "componentCount": p.components.len(),
                        "upgradeAvailable": hints.iter().any(|(id, _, _)| *id == p.id),
                    })
                })
                .collect();
            serde_json::to_value(serde_json::json!({
                "pages": list,
                // 文档全量 (编辑器前端全量编辑面)
                "docs": pages.iter().map(|p| serde_json::to_value(p).unwrap_or_default()).collect::<Vec<_>>(),
                // 出厂文档全量 (R7 编辑控制台「恢复出厂页」的源; 会话内 UpdatePage
                // 回出厂内容 = 恢复出厂, 退出提交时统一落盘)
                "factoryDocs": kernel::config::json_store::factory().pages
                    .iter()
                    .map(|p| serde_json::to_value(p).unwrap_or_default())
                    .collect::<Vec<_>>(),
                "upgradeHints": hints.iter().map(|(id, base, cur)| serde_json::json!({
                    "id": id, "userVersion": base, "factoryVersion": cur,
                })).collect::<Vec<_>>(),
            }))
            .map(IpcReply::Ok)
            .unwrap_or_else(|e| IpcReply::Err(e.to_string()))
        }
        // ---- R6/R7 真窗编辑会话 (主线程中转 → UiCommand 送渲染线程) ----
        RequestKind::BeginEditSession => {
            shell.borrow().send_ui(UiCommand::BeginEditSession);
            IpcReply::Ok(serde_json::json!({ "ok": true }))
        }
        RequestKind::EndEditSession { commit } => {
            shell.borrow().send_ui(UiCommand::EndEditSession { commit });
            IpcReply::Ok(serde_json::json!({ "ok": true }))
        }
        RequestKind::EditCommand { payload } => {
            // 载荷 = EditCommand serde Value (前端 camelCase; 反序列化在主线程,
            // 装箱送渲染线程)
            match serde_json::from_value::<crate::edit_session::EditCommand>(payload) {
                Ok(cmd) => {
                    shell.borrow().send_ui(UiCommand::Edit(Box::new(cmd)));
                    IpcReply::Ok(serde_json::json!({ "ok": true }))
                }
                Err(e) => IpcReply::Err(format!("编辑命令解析失败: {e}")),
            }
        }
        RequestKind::OpenComparisonWindow { fm0, fm1 } => {
            // FMLIST 行 对比按钮 (批3): Java FMListRowRenderer 的 View 键 —
            // 选中机型单机视图 (fm1 恒 null) 开对比窗; 参数由前端显式传 (对位 Java
            // 按钮体直取 combo 当前项), 不读 cfg。空 fm1 由 web_windows 归一为单机模式
            open_web_window(&WebWindowRequest::Comparison { fm0, fm1 }, rt)
        }
        RequestKind::GetVoicePacks => {
            // Java VoiceResourceManager.getInstance().get_available_packs():
            // "default" + voice/ 子目录。共享实例 = shell.voice (AppShell 字段,
            // Java 单例落位, winmm waveOut 播放器; 试听/告警装配复用同一实例)
            let mgr = Arc::clone(&shell.borrow().voice);
            serde_json::to_value(mgr.get_available_packs())
                .map(IpcReply::Ok)
                .unwrap_or_else(|e| IpcReply::Err(e.to_string()))
        }
        RequestKind::PreviewVoice { key, pack } => {
            // Java VoiceRowRenderer 试听按钮 (按钮体提取为
            // preview_voice_clip 以注入 mock 播放器断言 load/play 与 pack 传递);
            // 忽略 enable 态 (preview 语义), 失败无声, 回执恒 Ok (Java 按钮无失败反馈面)
            let mgr = Arc::clone(&shell.borrow().voice);
            let _ = preview_voice_clip(&mgr, &key, &pack); // 保活线程自持至播完
            IpcReply::Ok(serde_json::json!({ "ok": true }))
        }
        RequestKind::GetFmList => {
            // Java FMListRowRenderer 扫 flightmodels 根的中央文件名 (去扩展)。
            // 收敛点 list_fm_names: 只收 .json (blkx→json 迁移, data/ 双格式同名
            // 并存不过滤会重复), 排序去重, 目录不存在 → 空 vec
            let names = kernel::fm::data_paths::list_fm_names("");
            serde_json::to_value(names)
                .map(IpcReply::Ok)
                .unwrap_or_else(|e| IpcReply::Err(e.to_string()))
        }
        RequestKind::ImportConfig { path } => {
            // 导入 = 外部 delta 文件覆盖当前 delta + 重合成 + 落盘 (json_store 链)
            let ok = {
                let s = shell.borrow();
                s.controller
                    .as_ref()
                    .is_some_and(|c| c.config.import_config(&path))
            };
            if ok {
                // 重建表单快照 (对位 Java import 后 rebuild; 与核共享的 config 服务)
                let s = shell.borrow_mut();
                *cell.borrow_mut() = Some(build_form_state(&s));
                drop(s);
                // 广播整树变更 (前端重拉 + overlay 全量刷新, reset 链同款全局键)
                let s = shell.borrow();
                s.ui_bus.publish(
                    kernel::base::event::ui_state_events::CONFIG_CHANGED,
                    Some("ConfigImport"),
                    Some("ui_layout.cfg"),
                );
                IpcReply::Ok(serde_json::json!({ "ok": true }))
            } else {
                IpcReply::Err(format!("导入失败: {path} (解析错误, 原配置未动)"))
            }
        }
    }
}

// (R8: solve_page_ipc 快照链退役 — 真窗即画布, 编辑在渲染线程侧)

/// 批3 open* 按钮的开窗请求 (Java ButtonRowRenderer 直接 new 窗口的入参面)
#[derive(Debug, Clone, PartialEq)]
pub enum WebWindowRequest {
    /// CompactComparisonWindow(parent, ctr, fm0, fm1)
    Comparison { fm0: String, fm1: Option<String> },
    /// PowerCurveWindow(parent, fm0, fm1, speedKmh, wep)
    PowerCurve {
        fm0: String,
        fm1: Option<String>,
        speed_kmh: i32,
        wep: bool,
    },
}

// Java 标准库语义助手 (java_parse_int_or / java_parse_boolean) 已收敛
// kernel::base::java_compat, 本模块不再持本地副本。

/// open* 按钮分派 (Java ButtonRowRenderer 按钮体): 读 cfg 组装开窗
/// 入参; 非 open* 键返回 None (走原表单链)。纯流程函数 — cfg 读写可注入观测。
///
/// cfg 读取对位 Java RenderContext:
/// getString(key, def) = getConfig 为 null/空 → def; getBool(key, false) 同。
fn route_open_action(action: &str, shell: &Rc<RefCell<AppShell>>) -> Option<WebWindowRequest> {
    use kernel::config::config_api::ConfigProvider as _;

    let get_string = |key: &str, default: &str| -> String {
        let s = shell
            .borrow()
            .controller
            .as_ref()
            .and_then(|c| c.config.get_config(key))
            .unwrap_or_default();
        if s.is_empty() {
            default.to_string()
        } else {
            s
        }
    };

    match action {
        // Java 缺省: selectedFM0 → "a_4h", selectedFM1 → "a6m5_zero"
        "openComparison" => Some(WebWindowRequest::Comparison {
            fm0: get_string("selectedFM0", "a_4h"),
            fm1: Some(get_string("selectedFM1", "a6m5_zero")),
        }),
        // Java 缺省: fm0 "bf-109f-4", fm1 ""; speed parseInt 异常→0;
        // wep = Boolean.parseBoolean(powerCurveWep)
        "openPowerCurve" => Some(WebWindowRequest::PowerCurve {
            fm0: get_string("selectedFM0", "bf-109f-4"),
            fm1: Some(get_string("selectedFM1", "")),
            speed_kmh: get_string("powerCurveSpeed", "0").parse().unwrap_or(0),
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
            webui::web_windows::open_comparison_window(handle, fm0, fm1.as_deref())
        }
        WebWindowRequest::PowerCurve {
            fm0,
            fm1,
            speed_kmh,
            wep,
        } => webui::web_windows::open_power_curve_window(
            handle,
            fm0,
            fm1.as_deref(),
            *speed_kmh,
            *wep,
        ),
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
    shell: &Rc<RefCell<AppShell>>,
    cell: &FormCell,
    rt: &FormRuntime,
) -> IpcReply {
    let msg = to_message(dto);
    // 批3: open* 两键在表单写链前拦截 — Java ButtonRowRenderer 直接开窗 (无确认
    // 模态/无表单副作用); ui main_form 对 open* 只 warn+Ignore, 放行会丢动作
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
            shell.borrow_mut().dispatch(cmd);
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

/// Java VoiceRowRenderer ▶ 按钮体: pKey = stripVoicePrefix(property),
/// clip = loadClip(pKey, 当前选中包), 非 null → setFramePosition(0) + start。
/// clip==null 静默返回 (Java 无声失败, 不弹错误); // ignoring enable state for preview
/// (试听无视 enable 开关)。提取为独立纯流程函数 — 可注入 mock SoundPlayer 断言
/// load/play 调用序列与 pack 传递 (AppShell 的共享实例持 winmm 播放器, 不可 mock)。
///
/// 返回保活线程 JoinHandle (审查 B-B1 修复): Java 局部 clip 引用出作用域后
/// 原生 line 靠 GC finalizer 非确定性延迟释放而自然播完; Rust 确定性 Drop
/// (RAII close → waveOutReset+Close) 会掐断刚提交的播放 — clip 交
/// [`kernel::audio::voice_warning::hold_clip_until_done`] 持至播完 (对位 GC 延迟
/// 语义)。生产调用点忽略返回值; 测试 join 后断言收尾。
fn preview_voice_clip(
    mgr: &kernel::audio::voice_resource_manager::VoiceResourceManager,
    key: &str,
    pack: &str,
) -> Option<std::thread::JoinHandle<()>> {
    let p_key = kernel::audio::VoicePackConfig::strip_voice_prefix(Some(key)).unwrap_or_default();
    if let Some(clip) = mgr.load_clip(&p_key, Some(pack)) {
        clip.set_frame_position(0);
        clip.start();
        Some(kernel::audio::voice_warning::hold_clip_until_done(clip))
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
        FormMessageDto::ColorPicked { panel, key, value } => {
            Message::ColorPicked { panel, key, value }
        }
        FormMessageDto::Save => Message::Save,
        FormMessageDto::StartGame => Message::StartGame,
        FormMessageDto::EndGame => Message::EndGame,
        FormMessageDto::RefreshPreviews => Message::RefreshPreviews,
        FormMessageDto::ButtonAction { action } => Message::ButtonAction { action },
        FormMessageDto::ConfirmPending => Message::ConfirmPending,
        FormMessageDto::CancelPending => Message::CancelPending,
    }
}
