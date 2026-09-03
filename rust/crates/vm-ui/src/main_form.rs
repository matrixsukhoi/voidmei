//! MainForm 的表单数据层 (src/ui/MainForm.java + src/ui/layout/DynamicDataPage.java)。
//!
//! **D9 变更**: 设置窗换 Tauri 2 web 壳 (vm-webui) — 原 iced view 段已删,
//! 表单渲染归 web 壳 (tab 分页/卡片网格布局均在 web 壳, 数据驱动不写死面板);
//! 本模块仅存数据层 (Message/MainFormState/update/persist 写回链); --headless
//! 无窗口验收工具已提离至 [`headless`] 子模块 (E10, 不与状态机核心混排)。
//! - WYSIWYG 更新链 (对齐 Java MainForm→UIStateBus→Controller.refreshPreviews):
//!   值变更 → config.set_config (服务树更新 + 服务侧内联 publish CONFIG_CHANGED(key),
//!   对位 Java ConfigurationService.setConfig) → 保存链 persist_and_notify 落盘 +
//!   广播 CONFIG_CHANGED("ui_layout.cfg") (对位 DynamicDataPage.save)。
//!
//! PORT(clone-split 备案): Java 页面树与服务树是同一对象引用 (findGroupByTitle 返
//! 活引用); Rust 服务树锁内只能借出快照 → 本状态持 `groups` 快照与配置服务并存。
//! 行值键经 set_config 双写保持一致; 组字段 (fontSize/fontName 等 PropertyBinder
//! 写回目标, 快照独有) 与无 :target 行值经"挂起清单 + persist 以服务树为基重放"
//! 收敛 (Java 单对象语义的等价物 — 外部整树替换 (import/reset/watcher) 后服务树
//! 为最新, 不被陈旧快照覆盖, 对位 DynamicDataPage.rebuild 的 findGroupByTitle)。
//!
//! PORT(消息形状备案): 规格消息枚举为 Toggle(key,bool)/Slider(key,i32)/Combo(key,
//! String); 本实现各补 `panel` 字段 — Java 渲染器闭包捕获 (row, groupConfig),
//! PropertyBinder 字段写以 panel 级 GroupConfig 为目标,
//! 同名 key 可分布于多个 panel (现行 ui_layout.cfg: fontSize×7 / fontName×2),
//! 无 panel 无法保真定位写入目标。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;

// tests.rs 经 `use super::*` 消费的事件面符号 (cfg(test) 免非测试构建 unused 警告)
#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
use vm_core::base::bus::ui_state_bus::UiStateEvent;

use crate::render_context::RenderContext;
use vm_core::base::bus::ui_state_bus::UIStateBus;
use vm_core::base::event::ui_state_events;
use vm_core::base::logger;
use vm_core::config::config_api::ConfigProvider;
use vm_core::config::config_loader::{save_config as save_layout_file, GroupConfig, RowConfig};
use vm_core::config::configuration_service::ConfigurationService;

// F7: GroupConfig 字段表 (GroupField/PanelField 两域的单一真相)
use crate::renderer_config_helper::group_field_table;

use crate::renderers;

// --headless 无窗口验收工具 (E10 提离; bin 入口经 main_form::headless 引用)
pub mod headless;

// =====================================================================
// 消息
// =====================================================================

/// 交互消息 (panel = 行所属 panel 标题, 见模块文档形状备案)。
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
    /// 颜色选择 (主键十进制 + legacy 分键写回, 见 color::apply) — ColorRowRenderer
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
    /// 动作按钮按下 (Java ButtonRowRenderer 五键分派; 审查轮 2-D 接线):
    /// resetConfig/factoryReset → 挂确认模态; open* 三键由 vm-app dispatcher
    /// 在表单写链前拦截直接开窗 (form_dispatch.rs), 不达本层
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
    /// 配置服务句柄 (服务树 = 真相源, 对位 Java tc.configService)
    config: ConfigurationService,
    /// panel 树快照 (对位 Java tc.dynamicConfigs; 显示 + PropertyBinder 字段写回目标)
    groups: Vec<GroupConfig>,
    /// 与 ConfigurationService 共享的 UI 事件总线 (Java UIStateBus 单例的注入式替代):
    /// 服务侧 setConfig 发布 CONFIG_CHANGED(key), 本侧保存链发布
    /// CONFIG_CHANGED("ui_layout.cfg") (DynamicDataPage.save)
    ui_bus: Arc<UIStateBus>,
    /// 持久化目标路径 (生产 = config_manager 用户配置路径); None = 不落盘
    /// (--headless 状态机驱动 / 测试注入 tmp 路径用 Some)
    persist_path: Option<String>,
    /// 挂起确认动作 (确认模态态; Some = 模态显示中 — Java JOptionPane 模态期)
    pending_action: Option<String>,
    /// 下拉选项缓存 (D9: view 删除后缓存归数据层持有 — _CROSSHAIRS_ 磁盘源只解析
    /// 一次; _FONTS_ 依赖当前值不缓存。vm-webui GetComboOptions 经 dispatcher 调用)
    combo_cache: RefCell<HashMap<String, Vec<String>>>,
    /// 挂起编辑 (clone-split 待收敛): 渲染器写入快照但未进服务树的量 —
    /// (a) PropertyBinder 组字段写 (fontSize/fontName/…); (b) 无 :target 行的
    /// row.value (write_*(None) 不落服务)。persist 以服务树为基重放 (Java 单
    /// 对象语义的等价物; 外部整树替换后依然保真)。
    pending_panel_fields: Vec<(String, PanelField)>,
    /// 无 :target 行 (panel 标题, label) 的 row.value 挂起
    pending_row_values: Vec<(String, String)>,
}

impl MainFormState {
    pub fn new(
        config: ConfigurationService,
        ui_bus: Arc<UIStateBus>,
        persist_path: Option<String>,
    ) -> Self {
        let groups = config.get_layout_configs().unwrap_or_default();
        MainFormState {
            config,
            groups,
            ui_bus,
            persist_path,
            pending_action: None,
            combo_cache: RefCell::new(HashMap::new()),
            pending_panel_fields: Vec::new(),
            pending_row_values: Vec::new(),
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

    /// panel 树快照只读访问 (D9: vm-webui dto 序列化用)
    pub fn groups(&self) -> &[GroupConfig] {
        &self.groups
    }

    /// 下拉选项解析 (带缓存): _FONTS_ 依赖当前值不缓存, 其余按 source 缓存一次
    /// (D9: view 删除后由数据层承接, vm-webui GetComboOptions 经 dispatcher 调用)
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
}

// =====================================================================
// RenderContext 实现 (Java DynamicDataPage 匿名类)
// =====================================================================

/// 写侧上下文 (update 用): on_save 以标志位暂存, 由 [`with_panel`] 统一
/// flush (对位 Java 回调直调 save(); 写路径在 update, 无重入面)。
pub(crate) struct WriteContext<'a> {
    config: &'a ConfigurationService,
    ui_bus: &'a UIStateBus,
    save_requested: Cell<bool>,
}

impl<'a> WriteContext<'a> {
    pub(crate) fn new(config: &'a ConfigurationService, ui_bus: &'a UIStateBus) -> Self {
        WriteContext {
            config,
            ui_bus,
            save_requested: Cell::new(false),
        }
    }

    fn take_save(&self) -> bool {
        self.save_requested.replace(false)
    }
}

impl RenderContext for WriteContext<'_> {
    fn on_save(&self) {
        self.save_requested.set(true);
    }
    fn sync_to_config_service(&self, key: &str, value: bool) {
        // Java: setConfig(key, Boolean.toString(value)) + enableFMPrint 特例
        // (FM_PRINT_SWITCH_CHANGED 广播, 源串对位 Java "DynamicDataPage(RenderContext)")
        self.config.set_config(key, &value.to_string());
        if key == "enableFMPrint" {
            self.ui_bus.publish(
                ui_state_events::FM_PRINT_SWITCH_CHANGED,
                Some("DynamicDataPage(RenderContext)"),
                Some(&value.to_string()),
            );
        }
    }
    fn get_from_config_service(&self, key: &str, default_val: bool) -> bool {
        let val = self.config.get_config(key).unwrap_or_default();
        if val.is_empty() {
            default_val
        } else {
            val.eq_ignore_ascii_case("true")
        }
    }
    fn sync_string_to_config_service(&self, key: &str, value: &str) {
        self.config.set_config(key, value);
    }
    fn get_string_from_config_service(&self, key: &str, default_val: &str) -> String {
        let val = self.config.get_config(key).unwrap_or_default();
        if val.is_empty() {
            default_val.to_string()
        } else {
            val
        }
    }
}

// =====================================================================
// update (WYSIWYG 更新链)
// =====================================================================

/// 表单消息驱动的状态更新 (web 壳经 vm-app dispatcher 投递消息集, 链路不变)。
pub fn update(state: &mut MainFormState, message: Message) {
    match message {
        Message::Toggle { panel, key, value } => {
            with_panel(state, &panel, &key, |g, ctx| {
                renderers::switch::apply(g, &key, value, ctx)
            });
        }
        Message::Slider { panel, key, value } => {
            // (valueIsAdjusting 语义), 落盘由 on_release → Message::Save 承担
            with_panel(state, &panel, &key, |g, ctx| {
                renderers::slider::apply(g, &key, value, ctx)
            });
        }
        Message::Combo { panel, key, value } => {
            with_panel(state, &panel, &key, |g, ctx| {
                renderers::combo::apply(g, &key, &value, ctx)
            });
        }
        Message::Save => {
            // Java DynamicDataPage.save: saveDynamicConfig + 广播
            persist_and_notify(state);
        }
        Message::StartGame => {
            // Java MainForm.confirm: ACTION 日志 + endPreview + saveConfig +
            // loadFromConfig + tc.start() — 此处落保存链
            logger::info(
                "MainForm",
                "ACTION: User confirmed start. Initializing Game Mode...",
            );
            persist_and_notify(state);
        }
        Message::EndGame => {
            // 对位 Java 底部 mCancel 的保存语义; 进程退出/托盘回收归组装层 (vm-app)
            logger::info(
                "MainForm",
                "ACTION: User requested end. Saving configuration...",
            );
            persist_and_notify(state);
        }
        Message::RefreshPreviews => {
            // WYSIWYG 刷新触发: 广播 CONFIG_CHANGED("ui_layout.cfg") — 对位 Java
            // publish → Controller.refreshPreviews (订阅方批十三接线)
            publish_config_changed(&state.ui_bus, "ui_layout.cfg");
        }
        Message::ButtonAction { action } => {
            // Java ButtonRowRenderer: 确认对话框先行 (JOptionPane), OK 才执行。
            // Rust 以模态挂起等价 (web 壳画确认层, ConfirmPending 执行)。
            // open* 三键由 vm-app dispatcher 拦截开窗, 不达本臂
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
            // 确认框 OK 分支: 执行挂起动作 + 整树刷新 + 广播 (Java reset 链尾部
            // refreshAllPreviews 语义 — "ui_layout.cfg" 全局键触发全量 WYSIWYG)
            let action = state.pending_action.take().unwrap_or_default();
            let ok = match action.as_str() {
                // ButtonRowRenderer: resetToFactory (模板覆盖 + 备份)
                "factoryReset" => state.config.reset_to_factory(),
                // ButtonRowRenderer: publish(ACTION_RESET_REQUEST) → resetAllLayoutDefaults —
                // 总线订阅未接 (init_config 备案), 直调顶替 (configuration_service
                // reset_all_layout_defaults 注释指定的顶替路径)
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
            // 整树收敛: 服务重读自用户配置 (persist 优先, headless 回退全局路径)
            let path = state.persist_path.clone().unwrap_or_else(|| {
                vm_core::config::config_manager::get_user_config_path().to_string()
            });
            state.config.load_layout(&path);
            state.groups = state.config.get_layout_configs().unwrap_or_default();
            publish_config_changed(&state.ui_bus, "ui_layout.cfg");
        }
        Message::CancelPending => {
            state.pending_action = None; // 确认框 CANCEL_OPTION
        }
        Message::ColorPicked { panel, key, value } => {
            // Java ColorRowRenderer.applyColorChange: 主键十进制 + 分键
            // R/G/B/A 写服务 + row.value + onSave (即存, 每次 apply 落盘)
            with_panel(state, &panel, &key, |g, ctx| {
                renderers::color::apply(g, &key, value, ctx)
            });
        }
    }
}

/// 定位 panel 执行渲染器写链并 flush 回调标志。
/// 分域借用: ctx 持 &config/&ui_bus (不可变), groups 可变 (同结构体分字段)。
fn with_panel(
    state: &mut MainFormState,
    panel: &str,
    key: &str,
    f: impl FnOnce(&mut GroupConfig, &WriteContext<'_>),
) {
    let Some(pi) = panel_index_by_title(&state.groups, panel) else {
        // 域外消息 (面板已重建/标题漂移); Java 闭包捕获行对象无此失败面, 记日志保义
        logger::warn("MainForm", &format!("消息面板未命中: {panel}#{key}"));
        return;
    };
    let before = PanelFields::capture(&state.groups[pi]);
    let ctx = WriteContext::new(&state.config, &state.ui_bus);
    f(&mut state.groups[pi], &ctx);

    // 挂起编辑登记 (clone-split 待收敛, 见字段文档):
    // (a) 组字段差异 (PropertyBinder 写) → persist 重放进服务树
    for field in before.diff(&PanelFields::capture(&state.groups[pi])) {
        let e = (panel.to_string(), field);
        if !state.pending_panel_fields.contains(&e) {
            state.pending_panel_fields.push(e);
        }
    }
    // (b) 定位行的属性形态决定收敛方向: 有 :target → set_config 已全局更新服务树,
    // 从服务树回拷快照 (跨 panel 同 key); 无 :target (key=label) → 服务树未动,
    // row.value 仅快照持有 → 挂起 (回拷反而以旧值覆盖新值)
    let hit = renderers::find_row_path(&state.groups[pi].rows, key);
    let keyed_by_property = hit
        .as_ref()
        .and_then(|p| renderers::row_by_path(&state.groups[pi].rows, p))
        .is_some_and(|r| r.property.as_deref() == Some(key));
    if keyed_by_property {
        mirror_key_from_service(&state.config, &mut state.groups, key);
    } else if hit.is_some() {
        let e = (panel.to_string(), key.to_string());
        if !state.pending_row_values.contains(&e) {
            state.pending_row_values.push(e);
        }
    }

    // flush: on_save → 保存链
    // (Java rebuild 链已退役: D9 后视图刷新归 web 壳的 CONFIG_CHANGED 广播面,
    // "取最新配置树"目的由 persist 的服务树为基承担)
    // (先取标志再落保存链 — ctx 与 &mut state 的借用分界)
    let save = ctx.take_save();
    if save {
        persist_and_notify(state);
    }
}

/// 保存链 (Java DynamicDataPage.save):
/// saveDynamicConfig → configService.saveLayoutConfig (落盘) + publish(CONFIG_CHANGED,
/// "ui_layout.cfg")。
fn persist_and_notify(state: &mut MainFormState) {
    // 以**服务树**为基 + 重放挂起编辑: Java 落盘的是"共享活对象树" (= 服务树),
    // 快照仅持未收敛量 (组字段写/无 :target 行值)。外部整树替换 (import/reset/
    // watcher) 后服务树为最新 — 不被陈旧快照覆盖 (对位 rebuild 的 findGroupByTitle)
    let mut tree = state.config.get_layout_configs().unwrap_or_default();
    let pending_fields = std::mem::take(&mut state.pending_panel_fields);
    let pending_rows = std::mem::take(&mut state.pending_row_values);
    for (title, field) in &pending_fields {
        if let (Some(dst), Some(src)) = (
            group_by_title_mut(&mut tree, title),
            group_by_title(&state.groups, title),
        ) {
            field.copy(dst, src);
        }
    }
    for (title, label) in &pending_rows {
        let src_val = group_by_title(&state.groups, title)
            .and_then(|g| label_row(&g.rows, label))
            .and_then(|r| r.value.clone());
        let dst =
            group_by_title_mut(&mut tree, title).and_then(|g| label_row_mut(&mut g.rows, label));
        if let (Some(v), Some(dst)) = (src_val, dst) {
            dst.value = Some(v);
        }
    }
    match state.persist_path.clone() {
        Some(path) => {
            // Java saveLayoutConfig 的落盘日志 (ConfigurationService.java 同文案)
            logger::info(
                "ConfigurationService",
                &format!("ACTION: ConfigurationService: Saving to {path}"),
            );
            save_layout_file(&path, &tree);
            // clone-split 收敛: 服务树重读自盘 → == 落盘树
            state.config.load_layout(&path);
            // 快照重建 (对位 Java rebuild 取最新树; 亦吸收外部整树替换)
            state.groups = state.config.get_layout_configs().unwrap_or_default();
        }
        None => {
            logger::info("MainForm", "持久化停用 (--headless), 仅广播 CONFIG_CHANGED");
            // 无落盘收敛: 挂起编辑仍由快照持有 (tree 为局部变量), 不回刷
        }
    }
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

fn panel_index_by_title(groups: &[GroupConfig], title: &str) -> Option<usize> {
    // Java findGroupByTitle: 首个精确匹配
    groups.iter().position(|g| g.title == title)
}

/// 快照 ← 服务树的按键回拷 (clone-split 收敛的值面):
/// 服务树 set_config 后是该 key 的全局真相 (所有同 key 行 + switchKey 组可见性),
/// 快照只更新了交互命中的 panel。服务树整体克隆一次 (交互频率 ~Hz, 可接受)。
/// A4: 组配对按 title 查找 (对位 Java rebuild 的 findGroupByTitle)。原按位 zip
/// 依赖两树同构 — 外部整树替换 (import/reset/watcher) 后长度/顺序可漂移: 长度
/// 不等即静默截断, 顺序错位即把值镜像进不相干组。现按 title 首个精确匹配
/// (group_by_title 语义): 本地组在服务树无同名组时跳过 — 多出的组不再被错误
/// 配对 (与旧 zip 的截断同效), 服务树多出的组本就不在快照域, 天然不受影响。
fn mirror_key_from_service(config: &ConfigurationService, groups: &mut [GroupConfig], key: &str) {
    let svc = config.get_layout_configs().unwrap_or_default();
    for g in groups.iter_mut() {
        // 按 title 在服务树定位对应组; 找不到跳过 (不镜像进错位组)
        let Some(sg) = group_by_title(&svc, &g.title) else {
            continue;
        };
        if g.switch_key.as_deref() == Some(key) {
            g.visible = sg.visible;
        }
        mirror_rows(&mut g.rows, &sg.rows, key);
    }
}

/// A4 行级收尾: 本地行命中谓词即在服务行树按同谓词 DFS 取首个命中行 (服务侧
/// update_rows_recursive 写值的正是该行 — 真相行与树形对位无关)。服务树无命中
/// 行时跳过 (原按位 zip 依赖两树同构: 顺序漂移会把值镜像进不相干行, 长度不等
/// 静默截断; 现同构时语义等价 — 对位行即命中行, 漂移时不再错配)。
fn mirror_rows(rows: &mut [RowConfig], svc: &[RowConfig], key: &str) {
    let svc_row =
        renderers::find_row_path(svc, key).and_then(|path| renderers::row_by_path(svc, &path));
    for r in rows.iter_mut() {
        // 与服务侧 update_rows_recursive 同一命中谓词 (property 精确 / 无 property 时 label)
        let hit = r.property.as_deref() == Some(key) || (r.property.is_none() && key == r.label);
        if hit {
            if let Some(sr) = svc_row {
                r.value = sr.value.clone();
            }
        }
        if !r.children.is_empty() {
            mirror_rows(&mut r.children, svc, key);
        }
    }
}

// =====================================================================
// 挂起编辑重放的支撑类型 (persist 以服务树为基, 见 MainFormState 字段文档)
// =====================================================================

/// 表驱动的统一值拷贝 (F7): 克隆面抹平 String/Option 与 Copy 数值的差异 —
/// 生成处统一写 .dup() (直接 .clone() 会触发 Copy 类型的 clippy::clone_on_copy)
trait FieldDup {
    fn dup(&self) -> Self;
}

impl<T: Clone> FieldDup for T {
    fn dup(&self) -> Self {
        self.clone()
    }
}

// F7 表驱动: 与 renderer_config_helper.rs 共用字段表 (11 个值字段), 单次展开生成
// 全套 — 枚举 + copy + 捕捉结构体 + diff (变体序/copy 字段集/diff 序 = 表序,
// 与原手写一致)。rows 不在表内 → PanelField 域天然 = 11 个非 rows 字段
// (与 D7 注册表的非 rows 子集一致)。
// 注: 宏卫生限制 — 经两次宏展开的 ident 不与另一层展开的局部名统一, 故不做
// 逐条目递归咀嚼, 全套在单次展开内生成 (局部名 dst/src/g/self/other/v 同层)。
macro_rules! gen_panel_fields {
    ($( ($V:ident, $key:literal, $f:ident, $TY:ty) ),* $(,)?) => {
        /// PropertyBinder 可写的组字段 (键集合 = renderer_config_helper.rs 的 D7
        /// 注册表 11 个非 rows 字段); persist 时按字段从快照 panel 拷入服务树 panel。
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum PanelField {
            $($V,)*
        }

        impl PanelField {
            /// 把 src 面板的本字段值拷入 dst
            fn copy(self, dst: &mut GroupConfig, src: &GroupConfig) {
                match self {
                    $(PanelField::$V => dst.$f = src.$f.dup(),)*
                }
            }
        }

        /// panel 的组字段捕捉 (渲染器写链前后的差异检测)
        #[derive(Debug, Clone, PartialEq)]
        struct PanelFields {
            $($f: $TY,)*
        }

        impl PanelFields {
            fn capture(g: &GroupConfig) -> Self {
                PanelFields {
                    $($f: g.$f.dup(),)*
                }
            }

            /// 两次捕捉之间被渲染器改动的字段 (字段表序)
            fn diff(&self, other: &Self) -> Vec<PanelField> {
                let mut v = Vec::new();
                $(if self.$f != other.$f {
                    v.push(PanelField::$V);
                })*
                v
            }
        }
    };
}

group_field_table!(gen_panel_fields);

fn group_by_title<'a>(groups: &'a [GroupConfig], title: &str) -> Option<&'a GroupConfig> {
    groups.iter().find(|g| g.title == title)
}

fn group_by_title_mut<'a>(
    groups: &'a mut [GroupConfig],
    title: &str,
) -> Option<&'a mut GroupConfig> {
    groups.iter_mut().find(|g| g.title == title)
}

/// 无 :target 行定位 (与 update_rows_recursive 的 label 臂同一谓词; 含嵌套子行)
fn label_row<'a>(rows: &'a [RowConfig], label: &str) -> Option<&'a RowConfig> {
    for r in rows {
        if r.property.is_none() && r.label == label {
            return Some(r);
        }
        if !r.children.is_empty() {
            if let Some(found) = label_row(&r.children, label) {
                return Some(found);
            }
        }
    }
    None
}

fn label_row_mut<'a>(rows: &'a mut [RowConfig], label: &str) -> Option<&'a mut RowConfig> {
    for r in rows {
        if r.property.is_none() && r.label == label {
            return Some(r);
        }
        if !r.children.is_empty() {
            if let Some(found) = label_row_mut(&mut r.children, label) {
                return Some(found);
            }
        }
    }
    None
}

// =====================================================================
// Tests — 真实链路 (ConfigurationService + 总线 + 快照), 无 mock 造假
// =====================================================================
#[cfg(test)]
mod tests;
