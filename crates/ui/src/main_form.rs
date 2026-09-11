//! MainForm 的表单数据层 (src/ui/MainForm.java + src/ui/layout/DynamicDataPage.java)。
//!
//! **D9 变更**: 设置窗换 Tauri 2 web 壳 (webui) — 表单渲染归 web 壳,
//! 本模块仅存数据层 (Message/MainFormState/update/保存广播链)。
//!
//! **JSON 配置变更 (Phase 1)**: 旧 clone-split 三件套 (快照挂起编辑 +
//! persist 以服务树为基重放 + 镜像回拷) 整体退役 — 写链直调 ConfigurationService
//! (树更新 + delta 登记原子完成, 见 json_store), 本状态只剩显示快照与消息路由:
//! - 值变更 → config.set_config / set_group_field (树+delta 同步 + 内联 publish
//!   CONFIG_CHANGED(key));
//! - 保存链 → config.save_layout_config() (delta 落盘) + 广播
//!   CONFIG_CHANGED("ui_layout.cfg") (对位 DynamicDataPage.save)。

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use kernel::base::bus::ui_state_bus::UIStateBus;
use kernel::base::event::ui_state_events;
use kernel::base::logger;
use kernel::config::config_api::ConfigProvider;
use kernel::config::json_model::{ConfigValue, GroupConfig, RowConfig};
use kernel::config::json_store;
use kernel::config::configuration_service::ConfigurationService;

use crate::renderers;

// =====================================================================
// 消息
// =====================================================================

/// 交互消息 (panel = 行所属 panel 标题 — 同名 key 可分布于多个 panel
/// (fontSize×7 / fontName×2), 无 panel 无法定位组字段写入目标)。
#[derive(Debug, Clone)]
pub enum Message {
    /// 开关翻转 (value 为**显示值**, SWITCH_INV 落库取反) — SwitchRowRenderer 闭包
    Toggle {
        panel: String,
        key: String,
        value: bool,
    },
    /// 滑条值变更 (拖拽期实时, 不落盘) — SliderRowRenderer.persistValue 的内存链
    Slider {
        panel: String,
        key: String,
        value: i32,
    },
    /// 下拉选中 — ComboRowRenderer.addActionListener
    Combo {
        panel: String,
        key: String,
        value: String,
    },
    /// 颜色选择 — ColorRowRenderer (主键十进制落库)
    ColorPicked {
        panel: String,
        key: String,
        value: [u8; 4],
    },
    /// 保存 (按钮/滑条拖拽释放) — DynamicDataPage.save / saveDynamicConfig
    Save,
    /// 开始游戏 — MainForm.confirm
    StartGame,
    /// 结束游戏 — 底部按钮组 mCancel 的保存语义 (MainForm)
    EndGame,
    /// 刷新预览 — 主动广播 CONFIG_CHANGED, 对位 Controller.refreshPreviews 触发面
    RefreshPreviews,
    /// 动作按钮按下 (resetConfig/factoryReset → 挂确认模态; open* 三键由
    /// voidmei dispatcher 在表单写链前拦截直接开窗, 不达本层)
    ButtonAction { action: String },
    /// 确认模态「确定」(Java JOptionPane OK_OPTION 分支执行)
    ConfirmPending,
    /// 确认模态「取消」
    CancelPending,
}

// =====================================================================
// 状态
// =====================================================================

pub struct MainFormState {
    /// 配置服务句柄 (树 + delta 的同步写面, 持久化路径由服务自管)
    config: ConfigurationService,
    /// panel 树显示快照 (GetLayoutTree DTO 序列化源; 写后从服务回拉)
    groups: Vec<GroupConfig>,
    /// 与 ConfigurationService 共享的 UI 事件总线
    ui_bus: Arc<UIStateBus>,
    /// 挂起确认动作 (确认模态态; Some = 模态显示中)
    pending_action: Option<String>,
    /// 下拉选项缓存 (_CROSSHAIRS_ 磁盘源只解析一次; _FONTS_ 依赖当前值不缓存)
    combo_cache: RefCell<HashMap<String, Vec<String>>>,
}

impl MainFormState {
    pub fn new(config: ConfigurationService, ui_bus: Arc<UIStateBus>) -> Self {
        let groups = config.get_layout_configs().unwrap_or_default();
        MainFormState {
            config,
            groups,
            ui_bus,
            pending_action: None,
            combo_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn panel_count(&self) -> usize {
        self.groups.len()
    }

    /// 行总数 (含 HEADER 与嵌套子行)
    pub fn row_count(&self) -> usize {
        fn count(rows: &[RowConfig]) -> usize {
            rows.iter().map(|r| 1 + count(&r.children)).sum()
        }
        self.groups.iter().map(|g| count(&g.rows)).sum()
    }

    /// 首个指定类型且带 :target 的行 (panel 标题, key) — headless 驱动/测试辅助
    pub fn first_row_of_type(&self, want: &str) -> Option<(String, String)> {
        fn walk(rows: &[RowConfig], want: &str) -> Option<String> {
            for r in rows {
                if r.r#type == want {
                    if let Some(p) = &r.property {
                        return Some(p.clone());
                    }
                }
                if !r.children.is_empty() {
                    if let Some(p) = walk(&r.children, want) {
                        return Some(p);
                    }
                }
            }
            None
        }
        for g in &self.groups {
            if let Some(p) = walk(&g.rows, want) {
                return Some((g.title.clone(), p));
            }
        }
        None
    }

    /// 快照行取值 (headless/测试观测)
    pub fn snapshot_row(&self, panel: &str, key: &str) -> Option<RowConfig> {
        let g = self.groups.iter().find(|g| g.title == panel)?;
        renderers::find_row_path(&g.rows, key)
            .and_then(|p| renderers::row_by_path(&g.rows, &p))
            .cloned()
    }

    /// 服务侧配置串 (headless/测试观测; Java getConfig)
    pub fn service_string(&self, key: &str) -> String {
        self.config.get_config(key).unwrap_or_default()
    }

    /// panel 树快照只读访问 (webui dto 序列化用)
    pub fn groups(&self) -> &[GroupConfig] {
        &self.groups
    }

    /// 配置服务直访 (voidmei dispatcher 的 import/reset 链用)
    pub fn config(&self) -> &ConfigurationService {
        &self.config
    }

    /// 下拉选项解析 (带缓存): _FONTS_ 依赖当前值不缓存, 其余按 source 缓存一次
    pub fn options_for(&self, source: &str, current: &str) -> Vec<String> {
        if source == "_FONTS_" {
            return renderers::combo::resolve_options(source, current);
        }
        self.combo_cache
            .borrow_mut()
            .entry(source.to_string())
            .or_insert_with(|| renderers::combo::resolve_options(source, current))
            .clone()
    }

    /// 写后快照回拉 (树 + delta 已在服务侧同步落定)
    fn refresh_snapshot(&mut self) {
        self.groups = self.config.get_layout_configs().unwrap_or_default();
    }
}

// =====================================================================
// update (WYSIWYG 更新链)
// =====================================================================

/// 表单消息驱动的状态更新 (web 壳经 voidmei dispatcher 投递消息集, 链路不变)。
pub fn update(state: &mut MainFormState, message: Message) {
    match message {
        Message::Toggle { panel, key, value } => {
            let stored = match state.config.row_type(&panel, &key).as_deref() {
                // SWITCH_INV: 显示 ON → 存 false (取反转储)
                Some("SWITCH_INV") => (!value).to_string(),
                _ => value.to_string(),
            };
            write_control(state, &panel, &key, ConfigValue::Bool(value), &stored);
            state.config.save_layout_config(); // Java SwitchRowRenderer 即时 onSave
        }
        Message::Slider { panel, key, value } => {
            // 拖拽期实时链, 不落盘 (释放 → Message::Save 承担)
            write_control(
                state,
                &panel,
                &key,
                ConfigValue::Int(value),
                &value.to_string(),
            );
        }
        Message::Combo { panel, key, value } => {
            write_control(
                state,
                &panel,
                &key,
                ConfigValue::Str(value.clone()),
                &value,
            );
            state.config.save_layout_config(); // Java ComboRowRenderer 即时 onSave
        }
        Message::ColorPicked { panel, key, value } => {
            // 主键十进制存储 (向后兼容旧 cfg 双格式互通)
            let unified = renderers::combo::format_rgba_decimal(&value);
            write_control(
                state,
                &panel,
                &key,
                ConfigValue::Str(unified.clone()),
                &unified,
            );
            state.config.save_layout_config(); // Java ColorRowRenderer 即时 onSave
        }
        Message::Save => {
            // Java DynamicDataPage.save: saveDynamicConfig + 广播
            persist_and_notify(state);
        }
        Message::StartGame => {
            // Java MainForm.confirm: ACTION 日志 + endPreview + saveConfig + tc.start()
            logger::info(
                "MainForm",
                "ACTION: User confirmed start. Initializing Game Mode...",
            );
            persist_and_notify(state);
        }
        Message::EndGame => {
            // 对位 Java 底部 mCancel 的保存语义
            logger::info("MainForm", "ACTION: User requested end. Saving configuration...");
            persist_and_notify(state);
        }
        Message::RefreshPreviews => {
            // WYSIWYG 刷新触发: 广播全局键 (订阅方触发全量 overlay 刷新)
            publish_config_changed(&state.ui_bus, "ui_layout.cfg");
        }
        Message::ButtonAction { action } => {
            // open* 三键由 voidmei dispatcher 拦截开窗, 不达本臂
            match action.as_str() {
                "resetConfig" | "factoryReset" => {
                    logger::info(
                        "MainForm",
                        &format!("ACTION: 按钮按下 ({action}), 挂确认模态"),
                    );
                    state.pending_action = Some(action);
                }
                other => logger::warn("MainForm", &format!("未迁移动作键: {other}")),
            }
        }
        Message::ConfirmPending => {
            // 确认框 OK 分支: 执行挂起动作 (service 内完成重合成+落盘+RESET 广播)
            let action = state.pending_action.take().unwrap_or_default();
            let ok = match action.as_str() {
                // ButtonRowRenderer: resetToFactory (delta 清空 + 出厂重合成)
                "factoryReset" => state.config.reset_to_factory(),
                // ButtonRowRenderer: resetAllLayoutDefaults (行值 delta 清空)
                "resetConfig" => state.config.reset_all_layout_defaults(),
                _ => false,
            };
            logger::info(
                "MainForm",
                &format!(
                    "ACTION: 确认执行 ({action}) → {}",
                    if ok { "成功" } else { "失败" }
                ),
            );
            state.refresh_snapshot();
            publish_config_changed(&state.ui_bus, "ui_layout.cfg");
        }
        Message::CancelPending => {
            state.pending_action = None; // 确认框 CANCEL_OPTION
        }
    }
}

/// 控件值写链 (旧 RendererConfigHelper.write_* 语义的显式接替):
/// 组字段名 (fontSize/fontName/panelColumns…) 先写 panel 字段 (panel 作用域),
/// 行值**总是**经 set_config 全局更新 (SWITCH_INV 已在调用方反转);
/// panel 域外消息 (行不存在) 忽略 — 对位旧渲染器闭包捕获行对象的失败面。
/// enableFMPrint 特例额外广播 FM_PRINT_SWITCH_CHANGED (旧 WriteContext 语义)。
fn write_control(state: &mut MainFormState, panel: &str, key: &str, v: ConfigValue, raw: &str) {
    if state.config.row_type(panel, key).is_none() {
        logger::warn("MainForm", &format!("消息面板未命中: {panel}#{key}"));
        return;
    }
    if json_store::is_panel_field(key) {
        state.config.set_group_field(panel, key, v);
    }
    state.config.set_config(key, raw);
    if key == "enableFMPrint" {
        state.ui_bus.publish(
            ui_state_events::FM_PRINT_SWITCH_CHANGED,
            Some("DynamicDataPage(RenderContext)"),
            Some(raw),
        );
    }
    state.refresh_snapshot();
}

/// 保存链 (Java DynamicDataPage.save): delta 落盘 + publish(CONFIG_CHANGED, "ui_layout.cfg")
fn persist_and_notify(state: &mut MainFormState) {
    state.config.save_layout_config();
    publish_config_changed(&state.ui_bus, "ui_layout.cfg");
}

/// Java DynamicDataPage.save: publish(CONFIG_CHANGED, 类简单名, "ui_layout.cfg")
fn publish_config_changed(bus: &UIStateBus, data: &str) {
    bus.publish(
        ui_state_events::CONFIG_CHANGED,
        Some("DynamicDataPage"),
        Some(data),
    );
}

