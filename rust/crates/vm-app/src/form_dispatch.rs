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
        RequestKind::FormMessage(dto) => form_message(dto, shell, cell),
        RequestKind::GetVoicePacks => {
            // Java VoiceResourceManager.get_available_packs: "default" + voice/ 子目录。
            // 仅列表用途 — NoopPlayer 占位 (播放装配属语音子系统, 与 UI 解耦)
            use vm_core::voice_resource_manager::{SoundClip, SoundError, SoundPlayer, VoiceResourceManager};
            struct NoopPlayer;
            impl SoundPlayer for NoopPlayer {
                fn open_clip(&self, _path: &std::path::Path) -> Result<Box<dyn SoundClip>, SoundError> {
                    Err("NoopPlayer (仅列表用途, 无播放装配)".into())
                }
            }
            let mgr =
                VoiceResourceManager::new_with_voice_dir(Box::new(NoopPlayer), "voice".to_string());
            serde_json::to_value(mgr.get_available_packs())
                .map(IpcReply::Ok)
                .unwrap_or_else(|e| IpcReply::Err(e.to_string()))
        }
        RequestKind::GetFmList => {
            // Java FMListRowRenderer:48-62 扫 flightmodels/fm 的 .blkx 文件名 (去扩展)
            let dir = vm_core::fm::fm_data_paths::fm_dir();
            let mut names: Vec<String> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if let Some(stripped) =
                        vm_core::file_utils::get_file_name_no_ex(Some(&name))
                    {
                        names.push(stripped.to_string());
                    }
                }
            }
            names.sort();
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
                    s.ui_bus.publish(&vm_core::configuration_service::UiStateEvent {
                        event_type: vm_core::event::ui_state_events::CONFIG_CHANGED.to_string(),
                        source: "ConfigImport".to_string(),
                        data: "ui_layout.cfg".to_string(),
                    });
                }
                IpcReply::Ok(serde_json::json!({ "ok": true }))
            } else {
                IpcReply::Err(format!("导入失败: {path} (备份已创建, 原配置未动)"))
            }
        }
    }
}

/// 表单消息: 数据面全链 (WYSIWYG 写回在 update 内闭环);
/// StartGame/EndGame 附带 shell 命令 (对位原 iced 壳 hooks 的 tc 侧序列)。
fn form_message(dto: FormMessageDto, shell: &Arc<Mutex<AppShell>>, cell: &FormCell) -> IpcReply {
    let msg = to_message(dto);
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
}
