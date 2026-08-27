//! MainForm 的 iced 语义复刻 (src/ui/MainForm.java + src/ui/layout/DynamicDataPage.java)。
//!
//! C 类语义复刻 (非机械翻译): Swing/WebLaF 无 Rust 对应物, 以 Elm 架构对位 —
//! - Java MainForm 的 WebTabbedPane 每 panel 一页 → 本 view 顺序平铺各 panel 区块
//!   (滚动列); tab 切换/窗口尺寸自适应属窗口管理层, 后续批次。
//! - Java DynamicDataPage.buildContainer 的 (group ...) 卡片/网格 → [`build_rows`]
//!   的组标题 + 列数分块 (数据驱动, 不写死任何面板)。
//! - WYSIWYG 更新链 (对齐 Java MainForm→UIStateBus→Controller.refreshPreviews):
//!   值变更 → config.set_config (服务树更新 + 服务侧内联 publish CONFIG_CHANGED(key),
//!   对位 Java ConfigurationService.setConfig) → 保存链 persist_and_notify 落盘 +
//!   广播 CONFIG_CHANGED("ui_layout.cfg") (对位 DynamicDataPage.save L252)。
//!   overlay 侧订阅接线属批十三。
//!
//! PORT(clone-split 备案): Java 页面树与服务树是同一对象引用 (findGroupByTitle 返
//! 活引用); Rust 服务树锁内只能借出快照 → 本状态持 `groups` 快照与配置服务并存。
//! 行值键经 set_config 双写保持一致; 组字段 (fontSize/fontName 等 PropertyBinder
//! 写回目标, 快照独有) 与无 :target 行值经"挂起清单 + persist 以服务树为基重放"
//! 收敛 (Java 单对象语义的等价物 — 外部整树替换 (import/reset/watcher) 后服务树
//! 为最新, 不被陈旧快照覆盖, 对位 DynamicDataPage.rebuild 的 findGroupByTitle)。
//!
//! PORT(消息形状备案): 规格消息枚举为 Toggle(key,bool)/Slider(key,i32)/Combo(key,
//! String); 本实现各补 `panel` 字段 — Java 渲染器闭包捕获 (row, groupConfig)
//! (SwitchRowRenderer.java:40), PropertyBinder 字段写以 panel 级 GroupConfig 为目标,
//! 同名 key 可分布于多个 panel (现行 ui_layout.cfg: fontSize×7 / fontName×2),
//! 无 panel 无法保真定位写入目标。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use iced::widget::{button, column, container, scrollable, text, Column, Row};
use iced::{Element, Length};
use vm_core::bus::EventBus;
use vm_core::config_api::ConfigProvider;
use vm_core::config_loader::{save_config as save_layout_file, GroupConfig, RowConfig};
use vm_core::configuration_service::{ConfigurationService, UiStateEvent};
use vm_core::event::ui_state_events;
use vm_core::lang::Lang;
use vm_core::logger;
use vm_core::row_renderer_registry::RenderContext;

use crate::renderers::{self, combo};

// =====================================================================
// 消息
// =====================================================================

/// 交互消息 (Elm 架构; panel = 行所属 panel 标题, 见模块文档形状备案)。
#[derive(Debug, Clone)]
pub enum Message {
    /// 开关翻转 (value 为**显示值**, SWITCH_INV 落库取反) — SwitchRowRenderer 闭包
    Toggle { panel: String, key: String, value: bool },
    /// 滑条值变更 (拖拽期实时, 不落盘) — SliderRowRenderer.persistValue 的内存链
    Slider { panel: String, key: String, value: i32 },
    /// 下拉选中 — ComboRowRenderer.addActionListener
    Combo { panel: String, key: String, value: String },
    /// 颜色选择 (主键十进制 + legacy 分键写回, 见 color::apply) — ColorRowRenderer
    ColorPicked { panel: String, key: String, value: [u8; 4] },
    /// 保存 (按钮/滑条拖拽释放) — DynamicDataPage.save / saveDynamicConfig
    Save,
    /// 开始游戏 — MainForm.confirm (L265-278)
    StartGame,
    /// 结束游戏 — 底部按钮组 mCancel 的保存语义 (MainForm.java:92-98)
    EndGame,
    /// 刷新预览 — 主动广播 CONFIG_CHANGED, 对位 Controller.refreshPreviews 触发面
    RefreshPreviews,
    /// 动作按钮按下 (Java ButtonRowRenderer 五键分派; 审查轮 2-D 接线):
    /// resetConfig/factoryReset → 挂确认模态; open* 三键未迁移仍走 Ignore
    ButtonAction { action: String },
    /// 确认模态「确定」(Java JOptionPane OK_OPTION 分支执行)
    ConfirmPending,
    /// 确认模态「取消」
    CancelPending,
    /// 无操作 (property/label 双空残端控件的交互回包, 对位 Java isUpdating 抑制;
    /// 正常无 :target 控件以 label 为键走键控消息)
    Ignore,
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
    /// CONFIG_CHANGED("ui_layout.cfg") (DynamicDataPage.save, Java L252)
    ui_bus: Arc<EventBus<UiStateEvent>>,
    /// 持久化目标路径 (生产 = config_manager 用户配置路径); None = 不落盘
    /// (--headless 状态机驱动 / 测试注入 tmp 路径用 Some)
    persist_path: Option<String>,
    /// 挂起确认动作 (确认模态态; Some = 模态显示中 — Java JOptionPane 模态期)
    pending_action: Option<String>,
    /// 下拉选项缓存 (view 每帧调用, _CROSSHAIRS_ 磁盘源只解析一次; 单 UI 线程)
    combo_cache: RefCell<HashMap<String, Vec<String>>>,
    /// 挂起编辑 (clone-split 待收敛): 渲染器写入快照但未进服务树的量 —
    /// (a) PropertyBinder 组字段写 (fontSize/fontName/…); (b) 无 :target 行的
    /// row.value (write_*(None) 不落服务)。persist 以服务树为基重放 (Java 单
    /// 对象语义的等价物; 外部整树替换后依然保真)。
    pending_panel_fields: Vec<(String, PanelField)>,
    /// 无 :target 行 (panel 标题, label) 的 row.value 挂起
    pending_row_values: Vec<(String, String)>,
    /// i18n 快照 (Java Lang 静态字段; init_lang 读盘, 构造期一次)
    lang: Lang,
}

impl MainFormState {
    pub fn new(
        config: ConfigurationService,
        ui_bus: Arc<EventBus<UiStateEvent>>,
        persist_path: Option<String>,
    ) -> Self {
        // Java: tc.dynamicConfigs = configService.layoutConfigs (构造期快照)
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
            lang: Lang::init_lang(),
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

    /// 下拉选项解析 (带缓存): _FONTS_ 依赖当前值不缓存, 其余按 source 缓存一次
    fn options_of(&self, source: &str, current: &str) -> Vec<String> {
        if source == "_FONTS_" {
            return combo::resolve_options(source, current);
        }
        self.combo_cache
            .borrow_mut()
            .entry(source.to_string())
            .or_insert_with(|| combo::resolve_options(source, current))
            .clone()
    }
}

// =====================================================================
// RenderContext 实现 (Java DynamicDataPage.java:126-175 匿名类)
// =====================================================================

/// 读侧上下文 (view 用, 纯函数只读)。
/// PORT: sync_*/on_* 为写路径契约方法, view 侧不可达 — 空实现保留 trait 完整性
/// (写路径走 [`WriteContext`])。
pub(crate) struct ReadContext<'a> {
    config: &'a ConfigurationService,
}

impl<'a> ReadContext<'a> {
    pub(crate) fn new(config: &'a ConfigurationService) -> Self {
        ReadContext { config }
    }
}

impl RenderContext for ReadContext<'_> {
    fn on_save(&self) {}
    fn on_rebuild(&self) {}
    fn is_updating(&self) -> bool {
        // Java isUpdatingControls 仅 rebuild 期置位抑制 Swing 监听反馈环;
        // Elm 视图纯函数无此环
        false
    }
    fn sync_to_config_service(&self, _key: &str, _value: bool) {}
    fn get_from_config_service(&self, key: &str, default_val: bool) -> bool {
        // Java L155-161: getConfig; 空 → 默认; Boolean.parseBoolean (= equalsIgnoreCase("true"))
        let val = self.config.get_config(key).unwrap_or_default();
        if val.is_empty() {
            default_val
        } else {
            val.eq_ignore_ascii_case("true")
        }
    }
    fn sync_string_to_config_service(&self, _key: &str, _value: &str) {}
    fn get_string_from_config_service(&self, key: &str, default_val: &str) -> String {
        // Java L169-174: getConfig; 空 → 默认
        let val = self.config.get_config(key).unwrap_or_default();
        if val.is_empty() {
            default_val.to_string()
        } else {
            val
        }
    }
}

/// 写侧上下文 (update 用): on_save/on_rebuild 以标志位暂存, 由 [`with_panel`] 统一
/// flush (对位 Java 回调直调 save()/rebuild(); Elm 下写路径在 update, 无重入面)。
pub(crate) struct WriteContext<'a> {
    config: &'a ConfigurationService,
    ui_bus: &'a EventBus<UiStateEvent>,
    save_requested: Cell<bool>,
    rebuild_requested: Cell<bool>,
}

impl<'a> WriteContext<'a> {
    pub(crate) fn new(config: &'a ConfigurationService, ui_bus: &'a EventBus<UiStateEvent>) -> Self {
        WriteContext {
            config,
            ui_bus,
            save_requested: Cell::new(false),
            rebuild_requested: Cell::new(false),
        }
    }

    fn take_save(&self) -> bool {
        self.save_requested.replace(false)
    }

    fn take_rebuild(&self) -> bool {
        self.rebuild_requested.replace(false)
    }
}

impl RenderContext for WriteContext<'_> {
    fn on_save(&self) {
        self.save_requested.set(true); // Java: save() (L128-131)
    }
    fn on_rebuild(&self) {
        self.rebuild_requested.set(true); // Java: rebuild() (L133-137)
    }
    fn is_updating(&self) -> bool {
        false // 同 ReadContext 注
    }
    fn sync_to_config_service(&self, key: &str, value: bool) {
        // Java L143-152: setConfig(key, Boolean.toString(value)) + enableFMPrint 特例
        // (FM_PRINT_SWITCH_CHANGED 广播, 源串对位 Java "DynamicDataPage(RenderContext)")
        self.config.set_config(key, &value.to_string());
        if key == "enableFMPrint" {
            self.ui_bus.publish(&UiStateEvent {
                event_type: ui_state_events::FM_PRINT_SWITCH_CHANGED.to_string(),
                source: "DynamicDataPage(RenderContext)".to_string(),
                data: value.to_string(),
            });
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
        // Java L164-166
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

/// iced update 函数 (D1: 具名函数, 闭包触发高阶生命周期推断失败)。
pub fn update(state: &mut MainFormState, message: Message) {
    match message {
        Message::Toggle { panel, key, value } => {
            // Java: SwitchRowRenderer.java:41-68 / SwitchInvRowRenderer.java:30-39 闭包体
            with_panel(state, &panel, &key, |g, ctx| {
                renderers::switch::apply(g, &key, value, ctx)
            });
        }
        Message::Slider { panel, key, value } => {
            // Java: SliderRowRenderer.persistValue (L53-66); 拖拽期不落盘
            // (valueIsAdjusting 语义), 落盘由 on_release → Message::Save 承担
            with_panel(state, &panel, &key, |g, ctx| {
                renderers::slider::apply(g, &key, value, ctx)
            });
        }
        Message::Combo { panel, key, value } => {
            // Java: ComboRowRenderer.java:52-62
            with_panel(state, &panel, &key, |g, ctx| {
                renderers::combo::apply(g, &key, &value, ctx)
            });
        }
        Message::Save => {
            // Java DynamicDataPage.save (L245-255): saveDynamicConfig + 广播
            persist_and_notify(state);
        }
        Message::StartGame => {
            // Java MainForm.confirm (L265-278): ACTION 日志 + endPreview + saveConfig +
            // loadFromConfig + tc.start() — Controller 未翻译 (批十三), 此处落保存链
            logger::info("MainForm", "ACTION: User confirmed start. Initializing Game Mode...");
            persist_and_notify(state);
        }
        Message::EndGame => {
            // 对位 Java 底部 mCancel (MainForm.java:92-98) 的保存语义; 进程退出/托盘
            // 回收归组装层 (批十三)
            logger::info("MainForm", "ACTION: User requested end. Saving configuration...");
            persist_and_notify(state);
        }
        Message::RefreshPreviews => {
            // WYSIWYG 刷新触发: 广播 CONFIG_CHANGED("ui_layout.cfg") — 对位 Java
            // publish → Controller.refreshPreviews (订阅方批十三接线)
            publish_config_changed(&state.ui_bus, "ui_layout.cfg");
        }
        Message::ButtonAction { action } => {
            // Java ButtonRowRenderer: 确认对话框先行 (JOptionPane), OK 才执行。
            // Rust 以模态挂起等价 (view 画确认层, ConfirmPending 执行)。
            // open* 三键未走本消息 (button.rs 仍 Ignore — 窗口/文件对话框未迁移)
            match action.as_str() {
                "resetConfig" | "factoryReset" => {
                    logger::info("MainForm", &format!("ACTION: 按钮按下 ({action}), 挂确认模态"));
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
                // ButtonRowRenderer L121-147: resetToFactory (模板覆盖 + 备份)
                "factoryReset" => state.config.reset_to_factory(),
                // L38-62: publish(ACTION_RESET_REQUEST) → resetAllLayoutDefaults —
                // 总线订阅未接 (init_config 备案), 直调顶替 (configuration_service
                // reset_all_layout_defaults 注释指定的顶替路径)
                "resetConfig" => state.config.reset_all_layout_defaults(),
                _ => false,
            };
            logger::info(
                "MainForm",
                &format!("ACTION: 确认执行 ({action}) → {}", if ok { "成功" } else { "失败" }),
            );
            // 整树收敛: 服务重读自用户配置 (persist 优先, headless 回退全局路径)
            let path = state
                .persist_path
                .clone()
                .unwrap_or_else(|| vm_core::config_manager::get_user_config_path().to_string());
            state.config.load_layout(&path);
            state.groups = state.config.get_layout_configs().unwrap_or_default();
            publish_config_changed(&state.ui_bus, "ui_layout.cfg");
        }
        Message::CancelPending => {
            state.pending_action = None; // 确认框 CANCEL_OPTION
        }
        Message::Ignore => {} // 残端控件回包, 对位 Java isUpdating 抑制期
        Message::ColorPicked { panel, key, value } => {
            // Java ColorRowRenderer.applyColorChange (L110-136): 主键十进制 + 分键
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

    // flush: on_save → 保存链; on_rebuild → iced 声明式视图每帧自重建 (组列数变更
    // 即时生效), Java rebuild 的"取最新配置树"目的由 persist 的服务树为基承担
    // (先取标志再落保存链 — ctx 与 &mut state 的借用分界)
    let save = ctx.take_save();
    let rebuild = ctx.take_rebuild();
    if save {
        persist_and_notify(state);
    }
    if rebuild {
        logger::info("ComboDebug", "rebuild() called — iced 声明式视图即时生效");
    }
}

/// 保存链 (Java DynamicDataPage.save L245-255):
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
        if let (Some(dst), Some(src)) =
            (group_by_title_mut(&mut tree, title), group_by_title(&state.groups, title))
        {
            field.copy(dst, src);
        }
    }
    for (title, label) in &pending_rows {
        let src_val = group_by_title(&state.groups, title)
            .and_then(|g| label_row(&g.rows, label))
            .and_then(|r| r.value.clone());
        let dst = group_by_title_mut(&mut tree, title).and_then(|g| label_row_mut(&mut g.rows, label));
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

/// Java DynamicDataPage.save (L252): publish(CONFIG_CHANGED, 类简单名, "ui_layout.cfg")
fn publish_config_changed(bus: &EventBus<UiStateEvent>, data: &str) {
    bus.publish(&UiStateEvent {
        event_type: ui_state_events::CONFIG_CHANGED.to_string(),
        source: "DynamicDataPage".to_string(),
        data: data.to_string(),
    });
}

fn panel_index_by_title(groups: &[GroupConfig], title: &str) -> Option<usize> {
    // Java findGroupByTitle: 首个精确匹配
    groups.iter().position(|g| g.title == title)
}

/// 快照 ← 服务树的按键回拷 (clone-split 收敛的值面):
/// 服务树 set_config 后是该 key 的全局真相 (所有同 key 行 + switchKey 组可见性),
/// 快照只更新了交互命中的 panel。PORT: 两树同源同构 (new() 克隆而来, 运行期无
/// 结构变更), 按位 zip 安全; 服务树整体克隆一次 (交互频率 ~Hz, 可接受)。
fn mirror_key_from_service(config: &ConfigurationService, groups: &mut [GroupConfig], key: &str) {
    let svc = config.get_layout_configs().unwrap_or_default();
    for (g, sg) in groups.iter_mut().zip(svc.iter()) {
        if g.switch_key.as_deref() == Some(key) {
            g.visible = sg.visible;
        }
        mirror_rows(&mut g.rows, &sg.rows, key);
    }
}

fn mirror_rows(rows: &mut [RowConfig], svc: &[RowConfig], key: &str) {
    for (r, sr) in rows.iter_mut().zip(svc.iter()) {
        // 与服务侧 update_rows_recursive 同一命中谓词 (property 精确 / 无 property 时 label)
        if r.property.as_deref() == Some(key) || (r.property.is_none() && key == r.label) {
            r.value = sr.value.clone();
        }
        if !r.children.is_empty() {
            mirror_rows(&mut r.children, &sr.children, key);
        }
    }
}

// =====================================================================
// 挂起编辑重放的支撑类型 (persist 以服务树为基, 见 MainFormState 字段文档)
// =====================================================================

/// PropertyBinder 可写的组字段 (键集合 = renderer_config_helper.rs 的 D7 注册表
/// 11 个非 rows 字段); persist 时按字段从快照 panel 拷入服务树 panel。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelField {
    Title,
    X,
    Y,
    Alpha,
    Hotkey,
    Visible,
    FontName,
    FontSize,
    Columns,
    PanelColumns,
    SwitchKey,
}

impl PanelField {
    /// 把 src 面板的本字段值拷入 dst
    fn copy(self, dst: &mut GroupConfig, src: &GroupConfig) {
        match self {
            PanelField::Title => dst.title = src.title.clone(),
            PanelField::X => dst.x = src.x,
            PanelField::Y => dst.y = src.y,
            PanelField::Alpha => dst.alpha = src.alpha,
            PanelField::Hotkey => dst.hotkey = src.hotkey,
            PanelField::Visible => dst.visible = src.visible,
            PanelField::FontName => dst.font_name = src.font_name.clone(),
            PanelField::FontSize => dst.font_size = src.font_size,
            PanelField::Columns => dst.columns = src.columns,
            PanelField::PanelColumns => dst.panel_columns = src.panel_columns,
            PanelField::SwitchKey => dst.switch_key = src.switch_key.clone(),
        }
    }
}

/// panel 的组字段捕捉 (渲染器写链前后的差异检测)
#[derive(Debug, Clone, PartialEq)]
struct PanelFields {
    title: String,
    x: f64,
    y: f64,
    alpha: i32,
    hotkey: i32,
    visible: bool,
    font_name: Option<String>,
    font_size: i32,
    columns: i32,
    panel_columns: i32,
    switch_key: Option<String>,
}

impl PanelFields {
    fn capture(g: &GroupConfig) -> Self {
        PanelFields {
            title: g.title.clone(),
            x: g.x,
            y: g.y,
            alpha: g.alpha,
            hotkey: g.hotkey,
            visible: g.visible,
            font_name: g.font_name.clone(),
            font_size: g.font_size,
            columns: g.columns,
            panel_columns: g.panel_columns,
            switch_key: g.switch_key.clone(),
        }
    }

    /// 两次捕捉之间被渲染器改动的字段
    fn diff(&self, other: &Self) -> Vec<PanelField> {
        let mut v = Vec::new();
        if self.title != other.title {
            v.push(PanelField::Title);
        }
        if self.x != other.x {
            v.push(PanelField::X);
        }
        if self.y != other.y {
            v.push(PanelField::Y);
        }
        if self.alpha != other.alpha {
            v.push(PanelField::Alpha);
        }
        if self.hotkey != other.hotkey {
            v.push(PanelField::Hotkey);
        }
        if self.visible != other.visible {
            v.push(PanelField::Visible);
        }
        if self.font_name != other.font_name {
            v.push(PanelField::FontName);
        }
        if self.font_size != other.font_size {
            v.push(PanelField::FontSize);
        }
        if self.columns != other.columns {
            v.push(PanelField::Columns);
        }
        if self.panel_columns != other.panel_columns {
            v.push(PanelField::PanelColumns);
        }
        if self.switch_key != other.switch_key {
            v.push(PanelField::SwitchKey);
        }
        v
    }
}

fn group_by_title<'a>(groups: &'a [GroupConfig], title: &str) -> Option<&'a GroupConfig> {
    groups.iter().find(|g| g.title == title)
}

fn group_by_title_mut<'a>(groups: &'a mut [GroupConfig], title: &str) -> Option<&'a mut GroupConfig> {
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
// view (数据驱动, 不写死面板)
// =====================================================================

/// iced view 函数 (D1: 具名函数)。
pub fn view(state: &MainFormState) -> Element<'_, Message> {
    let options_of = |src: &str, cur: &str| state.options_of(src, cur);
    let ctx = ReadContext::new(&state.config);

    let mut panels = Column::new().spacing(12);
    if state.groups.is_empty() {
        // Java MainForm.java:155-157: dynamicConfigs 空 → "Data (Empty)" 占位页
        panels = panels.push(text("Data (Empty)"));
    }
    for g in &state.groups {
        panels = panels.push(panel_section(g, &ctx, &options_of));
    }

    // Java MainForm 底部按钮组 (createbuttonGroup L80-105 / createLBGroup L109-135);
    // 预览开关 (startPreview/stopPreview) 合并为"刷新预览"广播触发
    let actions = Row::with_children(vec![
        button("保存").on_press(Message::Save).into(),
        button(state.lang.m_start).on_press(Message::StartGame).into(),
        button(state.lang.m_cancel).on_press(Message::EndGame).into(),
        button("刷新预览").on_press(Message::RefreshPreviews).into(),
    ])
    .spacing(8);

    // Java 标题: appName + " v" + version (版本号注入属组装层, 批十三)
    let title = format!("{} 设置", state.lang.app_name);

    // 确认模态 (pending_action 挂起期): Java JOptionPane 模态对话框等价 —
    // 模态期主面板不可交互 (替换式呈现, 语义同阻塞)
    if let Some(action) = state.pending_action.as_deref() {
        let desc = match action {
            "factoryReset" => "将把全部配置恢复为出厂默认, 当前配置会先备份。",
            "resetConfig" => "将把全部配置项重置为默认值。",
            _ => "确认执行?",
        };
        let buttons = Row::new()
            .spacing(8)
            .push(button("确定").on_press(Message::ConfirmPending))
            .push(button("取消").on_press(Message::CancelPending));
        return Column::with_children(vec![
            text(title).size(16).into(),
            text(format!("确认{desc}")).size(14).into(),
            buttons.into(),
        ])
        .spacing(12)
        .padding(20)
        .into();
    }

    Column::with_children(vec![
        text(title).size(16).into(),
        actions.into(),
        scrollable(panels).height(Length::Fill).into(),
    ])
    .spacing(8)
    .padding(10)
    .into()
}

/// 一个 panel 区块 (Java: WebTabbedPane 一页 = DynamicDataPage)。
fn panel_section<'a>(
    group: &'a GroupConfig,
    ctx: &dyn RenderContext,
    options_of: &dyn Fn(&str, &str) -> Vec<String>,
) -> Element<'a, Message> {
    // Java rebuildSimple L123: pCols = panelColumns > 0 ? panelColumns : 2
    let p_cols = if group.panel_columns > 0 { group.panel_columns } else { 2 };
    let body = build_rows(&group.rows, group, p_cols, ctx, &group.title, options_of);
    container(
        Column::with_children(vec![
            text(group.title.clone()).size(18).into(), // Java tab 标题
            body.into(),
        ])
        .spacing(8),
    )
    .padding(10)
    .into()
}

/// 行树 → 列 (Java DynamicDataPage.buildContainer L184-243):
/// HEADER 行 = 组标题 + 子项递归 (列数 = :column > 0 ? :column : 父默认列数, L207);
/// 松散项按默认列数分块入网格。
fn build_rows<'a>(
    rows: &'a [RowConfig],
    panel: &'a GroupConfig,
    default_cols: i32,
    ctx: &dyn RenderContext,
    panel_title: &'a str,
    options_of: &dyn Fn(&str, &str) -> Vec<String>,
) -> Column<'a, Message> {
    let mut col = Column::new().spacing(6);
    let mut items: Vec<Element<'a, Message>> = Vec::new();
    for r in rows {
        if r.r#type == "HEADER" {
            if !items.is_empty() {
                let grid = grid_section(std::mem::take(&mut items), default_cols);
                col = col.push(grid);
            }
            // Java createContainer(label) 卡片标题
            col = col.push(text(r.label.clone()).size(15));
            let cols = if r.group_columns > 0 { r.group_columns } else { default_cols };
            col = col.push(build_rows(&r.children, panel, cols, ctx, panel_title, options_of));
        } else {
            // Java: RowRendererRegistry.get(rowType).render(row, groupConfig, ctx)
            items.push(renderers::view_row(r, panel, ctx, panel_title, options_of));
        }
    }
    if !items.is_empty() {
        let grid = grid_section(items, default_cols);
        col = col.push(grid);
    }
    col
}

/// 网格分块 (Java ResponsiveGridLayout(cols, hgap=10, vgap=5) 的 iced 近位:
/// 行内横向间距 = hgap 10, 行间纵向间距 = vgap 5 — 构造参数序 (cols, hgap, vgap))。
fn grid_section(items: Vec<Element<'_, Message>>, cols: i32) -> Element<'_, Message> {
    let cols = cols.max(1) as usize;
    let mut lines: Vec<Element<'_, Message>> = Vec::new();
    let mut cur: Vec<Element<'_, Message>> = Vec::new();
    for el in items {
        if cur.len() == cols {
            let line = Row::with_children(
                std::mem::take(&mut cur)
                    .into_iter()
                    .map(|e| container(e).width(Length::Fill).into())
                    .collect::<Vec<_>>(),
            )
            .spacing(10);
            lines.push(line.into());
        }
        cur.push(el);
    }
    if !cur.is_empty() {
        let line = Row::with_children(
            cur.into_iter()
                .map(|e| container(e).width(Length::Fill).into())
                .collect::<Vec<_>>(),
        )
        .spacing(10);
        lines.push(line.into());
    }
    column(lines).spacing(5).into()
}

// =====================================================================
// --headless 状态机驱动 (无窗口, 读真实 ui_layout.cfg)
// =====================================================================

/// 仓库模板 ui_layout.cfg 的 CWD 相对候选 (cargo run 自 rust/ 或手工自仓库根)。
pub fn locate_template_cfg() -> Option<&'static str> {
    const CANDIDATES: &[&str] = &[
        "./ui_layout.cfg",
        "../ui_layout.cfg",
        "../../ui_layout.cfg",
        "../../../ui_layout.cfg",
    ];
    CANDIDATES
        .iter()
        .copied()
        .find(|p| std::path::Path::new(p).exists())
}

/// 无窗口状态机测试: 构建真实表单 → 驱动四类渲染器写回消息 → 断言 WYSIWYG 链路。
/// 返回进程退出码 (0 = 全部通过)。
pub fn run_headless() -> i32 {
    let Some(cfg_path) = locate_template_cfg() else {
        eprintln!("vm-ui: --headless 未找到 ui_layout.cfg (候选: ./ ../ ../../ ../../../)");
        return 2;
    };
    let bus = Arc::new(EventBus::new());
    let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let s2 = Arc::clone(&seen);
    let _sub = bus.subscribe(move |m: &UiStateEvent| {
        s2.lock().unwrap().push(m.clone());
    });

    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.load_layout(cfg_path);
    let mut state = MainFormState::new(config, Arc::clone(&bus), None);
    println!(
        "vm-ui: --headless 表单构建成功: {} panels / {} rows (源: {cfg_path})",
        state.panel_count(),
        state.row_count()
    );
    let mut failures = 0u32;
    let verdict = |ok: bool| if ok { "PASS" } else { "FAIL" };

    // 1) 开关链路: 翻转 → 服务/快照/总线事件
    if let Some((panel, key)) = state.first_row_of_type("SWITCH") {
        let before = state.service_string(&key);
        let new_val = !before.eq_ignore_ascii_case("true");
        update(&mut state, Message::Toggle { panel, key: key.clone(), value: new_val });
        let after = state.service_string(&key);
        let ok = after == new_val.to_string();
        println!("vm-ui: [{}] Toggle {key}: 服务 {before:?} → {after:?} (期望 {new_val})", verdict(ok));
        failures += u32::from(!ok);
    } else {
        println!("vm-ui: [SKIP] cfg 无 SWITCH 行");
    }

    // 2) 反相开关链路: 显示 true → 落库 false
    if let Some((panel, key)) = state.first_row_of_type("SWITCH_INV") {
        update(&mut state, Message::Toggle { panel, key: key.clone(), value: true });
        let after = state.service_string(&key);
        let ok = after == "false";
        println!("vm-ui: [{}] SwitchInv {key}: 显示 true → 服务 {after:?} (期望 false)", verdict(ok));
        failures += u32::from(!ok);
    } else {
        println!("vm-ui: [SKIP] cfg 无 SWITCH_INV 行");
    }

    // 3) 滑条链路: 区间中点 → 快照行值 + 服务值 (不落盘)
    if let Some((panel, key)) = state.first_row_of_type("SLIDER") {
        let row = state.snapshot_row(&panel, &key);
        let (min, max) = row.as_ref().map_or((0, 100), |r| (r.min_val, r.max_val));
        let (min, max) = renderers::slider::effective_range(min, max);
        let v = min + (max - min) / 2;
        update(&mut state, Message::Slider { panel: panel.clone(), key: key.clone(), value: v });
        let row_ok = state.snapshot_row(&panel, &key).is_some_and(|r| r.get_int() == v);
        let svc = state.service_string(&key);
        let ok = row_ok && svc == v.to_string();
        println!("vm-ui: [{}] Slider {key}={v}: 快照行值+服务 {svc:?} (期望 {})", verdict(ok), v);
        failures += u32::from(!ok);
    } else {
        println!("vm-ui: [SKIP] cfg 无 SLIDER 行");
    }

    // 4) 下拉链路: 重选当前值 → 服务保持 + 快照 Str
    if let Some((panel, key)) = state.first_row_of_type("COMBO") {
        let current = state.service_string(&key);
        update(&mut state, Message::Combo { panel: panel.clone(), key: key.clone(), value: current.clone() });
        let svc = state.service_string(&key);
        let row_ok = state
            .snapshot_row(&panel, &key)
            .is_some_and(|r| r.get_str() == current);
        let ok = svc == current && row_ok;
        println!("vm-ui: [{}] Combo {key}={current:?}: 服务 {svc:?} + 快照行值", verdict(ok));
        failures += u32::from(!ok);
    } else {
        println!("vm-ui: [SKIP] cfg 无 COMBO 行");
    }

    // 5) 颜色链路: 选色 → 主键十进制写服务 + 快照行值 (分键 keyR/G/B/A 为忠实
    //    no-op 写, cfg 无对应行, 语义由 color.rs MapCtx 单测断言)
    if let Some((panel, key)) = state.first_row_of_type("COLOR") {
        let rgba = [232u8, 147, 50, 200];
        let decimal = "232, 147, 50, 200";
        update(&mut state, Message::ColorPicked { panel: panel.clone(), key: key.clone(), value: rgba });
        let svc = state.service_string(&key);
        let row_ok = state
            .snapshot_row(&panel, &key)
            .is_some_and(|r| r.get_str() == decimal);
        let ok = svc == decimal && row_ok;
        println!("vm-ui: [{}] ColorPicked {key}: 服务 {svc:?} + 快照行值 (期望 {decimal:?})", verdict(ok));
        failures += u32::from(!ok);
    } else {
        println!("vm-ui: [SKIP] cfg 无 COLOR 行");
    }

    // 6) 保存链路: 广播 CONFIG_CHANGED("ui_layout.cfg")
    {
        seen.lock().unwrap().clear();
        update(&mut state, Message::Save);
        let published = seen
            .lock()
            .unwrap()
            .iter()
            .any(|e| e.event_type == ui_state_events::CONFIG_CHANGED && e.data == "ui_layout.cfg");
        println!("vm-ui: [{}] Save 广播 CONFIG_CHANGED(\"ui_layout.cfg\")", verdict(published));
        failures += u32::from(!published);
    }

    if failures == 0 {
        println!("vm-ui: --headless 状态机全部通过");
        0
    } else {
        eprintln!("vm-ui: --headless 失败 {failures} 项");
        1
    }
}

// =====================================================================
// Tests — 真实链路 (ConfigurationService + 总线 + 快照), 无 mock 造假
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use vm_core::config_loader::ConfigValue;

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
        std::env::temp_dir()
            .join(format!("vm_ui_main_form_{name}.cfg"))
            .to_str()
            .unwrap()
            .to_string()
    }

    /// 真实链路环境: 模板落 tmp → ConfigurationService 装载 + 总线录制订阅。
    /// 返回订阅句柄 — 调用方须绑定保活 (`_sub`), RAII Drop 即注销。
    fn mk_state(
        name: &str,
        persist: Option<String>,
    ) -> (MainFormState, Arc<Mutex<Vec<UiStateEvent>>>, vm_core::bus::Subscription<UiStateEvent>) {
        let p = tmp_path(name);
        std::fs::write(&p, TEST_CFG).unwrap();
        let bus = Arc::new(EventBus::new());
        let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let s2 = Arc::clone(&seen);
        let sub = bus.subscribe(move |m: &UiStateEvent| {
            s2.lock().unwrap().push(m.clone());
        });
        let config = ConfigurationService::new(Some(Arc::clone(&bus)));
        config.load_layout(&p);
        (MainFormState::new(config, bus, persist), seen, sub)
    }

    fn events_of(seen: &Arc<Mutex<Vec<UiStateEvent>>>) -> Vec<(String, String)> {
        seen.lock()
            .unwrap()
            .iter()
            .map(|e| (e.event_type.clone(), e.data.clone()))
            .collect()
    }

    // Toggle 全链: 服务树 + 快照 (含跨 panel 同 key 全局更新, 对位 setConfig 的
    // update_rows_recursive 全实例语义) + CONFIG_CHANGED(key) + 保存链广播
    #[test]
    fn toggle_updates_service_snapshot_and_bus() {
        let (mut state, seen, _sub) = mk_state("toggle", None);
        update(&mut state, Message::Toggle { panel: "面板A".into(), key: "k1".into(), value: false });

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
        assert!(evs.contains(&(ui_state_events::CONFIG_CHANGED.into(), "ui_layout.cfg".into())));
    }

    // SwitchInv 反相链: 显示 true → 服务存 false + row.value 存显示值
    #[test]
    fn toggle_switch_inv_inverts_on_write() {
        let (mut state, _seen, _sub) = mk_state("inv", None);
        update(&mut state, Message::Toggle { panel: "面板A".into(), key: "k2".into(), value: true });
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

        update(&mut state, Message::Slider { panel: "面板A".into(), key: "fontSize".into(), value: 7 });
        // 实时链: 快照行值 + 组字段 + 服务值
        assert_eq!(state.snapshot_row("面板A", "fontSize").unwrap().get_int(), 7);
        assert_eq!(state.service_string("fontSize"), "7");
        // 拖拽语义: 不落盘 (on_release 前文件不存在)
        assert!(!std::path::Path::new(&persist).exists(), "Slider 拖拽期不得落盘");
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
        assert_eq!(group_a.font_size, 7, "组字段经落盘→load_layout 收敛回服务树");
        let _ = std::fs::remove_file(&persist);
    }

    // Combo 选中链: row.value Str + 服务 + on_save 即落盘 (Java 每次交互即存)
    #[test]
    fn combo_pick_persists_immediately() {
        let persist = tmp_path("combo_user");
        let _ = std::fs::remove_file(&persist);
        let (mut state, _seen, _sub) = mk_state("combo", Some(persist.clone()));

        update(&mut state, Message::Combo { panel: "面板A".into(), key: "style".into(), value: "B".into() });
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
        let bus = Arc::new(EventBus::new());
        let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let s2 = Arc::clone(&seen);
        let _sub = bus.subscribe(move |m: &UiStateEvent| {
            s2.lock().unwrap().push(m.clone());
        });
        let config = ConfigurationService::new(Some(Arc::clone(&bus)));
        config.load_layout(&p);
        let mut state = MainFormState::new(config, bus, Some(persist.clone()));

        update(
            &mut state,
            Message::ColorPicked { panel: "P".into(), key: "fontWarn".into(), value: [255, 36, 0, 128] },
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
        assert!(evs.contains(&(ui_state_events::CONFIG_CHANGED.into(), "ui_layout.cfg".into())));
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
        let bus = Arc::new(EventBus::new());
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

        update(&mut state, Message::Toggle { panel: "P".into(), key: "裸开关".into(), value: false });
        assert_eq!(state.snapshot_row("P", "裸开关").unwrap().value, Some(ConfigValue::Bool(false)));
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

        update(&mut state, Message::Slider { panel: "P".into(), key: "裸滑条".into(), value: 7 });
        assert!(!std::path::Path::new(&persist).exists(), "拖拽期不落盘");
        assert_eq!(state.snapshot_row("P", "裸滑条").unwrap().get_int(), 7);

        update(&mut state, Message::Save);
        assert!(std::path::Path::new(&persist).exists());
        assert_eq!(state.config.get_layout_configs().unwrap()[0].rows[0].get_int(), 7);
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

        update(&mut state, Message::Combo { panel: "P".into(), key: "裸下拉".into(), value: "Y".into() });
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
        update(&mut state, Message::Toggle { panel: "P".into(), key: "e1".into(), value: false });
        assert!(std::path::Path::new(&persist).exists());

        // 外部替换: 持久化路径被外部重写 (e1=true) 且服务树重载 — 快照变陈旧
        std::fs::write(&persist, cfg).unwrap();
        state.config.load_layout(&persist);

        // 交互 2: e2=false → 落盘必须保留外部 e1=true (旧实现写陈旧快照会回滚 e1)
        update(&mut state, Message::Toggle { panel: "P".into(), key: "e2".into(), value: false });
        let reread = vm_core::config_loader::load_config(&persist);
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
            vec![(ui_state_events::CONFIG_CHANGED.into(), "ui_layout.cfg".into())]
        );
    }

    // 域外面板消息: 无副作用无 panic
    #[test]
    fn unknown_panel_message_is_ignored() {
        let (mut state, seen, _sub) = mk_state("unknown_panel", None);
        update(&mut state, Message::Toggle { panel: "不存在".into(), key: "k1".into(), value: false });
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
        assert_eq!(state.first_row_of_type("COMBO"), Some(("面板A".to_string(), "style".to_string())));
        assert_eq!(state.first_row_of_type("COLOR"), None);
    }

    // enableFMPrint 特例: sync_to_config_service 额外广播 FM_PRINT_SWITCH_CHANGED
    // (Java DynamicDataPage.java:148-151)
    #[test]
    fn write_context_fmprint_special_publishes() {
        let p = tmp_path("fmp");
        std::fs::write(&p, r#"(panel "p" (item "fm" :type switch :target "enableFMPrint" :value true))"#).unwrap();
        let bus = Arc::new(EventBus::new());
        let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let s2 = Arc::clone(&seen);
        let _sub = bus.subscribe(move |m: &UiStateEvent| {
            s2.lock().unwrap().push(m.clone());
        });
        let config = ConfigurationService::new(Some(Arc::clone(&bus)));
        config.load_layout(&p);

        let ctx = WriteContext::new(&config, &bus);
        ctx.sync_to_config_service("enableFMPrint", false);
        let evs = events_of(&seen);
        assert_eq!(
            evs,
            vec![
                (ui_state_events::CONFIG_CHANGED.into(), "enableFMPrint".into()),
                (ui_state_events::FM_PRINT_SWITCH_CHANGED.into(), "false".into()),
            ]
        );
    }

    // ReadContext: 空配置串回落默认值 (Java L169-174 守卫)
    #[test]
    fn read_context_empty_falls_back_to_default() {
        let p = tmp_path("readctx");
        std::fs::write(&p, r#"(panel "p" (item "s" :type switch :target "absent" :value true))"#).unwrap();
        let bus = Arc::new(EventBus::new());
        let config = ConfigurationService::new(Some(bus));
        config.load_layout(&p);

        let ctx = ReadContext::new(&config);
        // "missing_key" 不属任何行 :target (注意 "absent" 恰是上方 cfg 的键)
        assert_eq!(ctx.get_string_from_config_service("missing_key", "默认"), "默认");
        assert!(ctx.get_from_config_service("missing_key", true));
        assert!(!ctx.get_from_config_service("missing_key", false));
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
        let _sandbox = Sandbox { orig, dir: dir.clone() };

        let bus = Arc::new(EventBus::new());
        let config = ConfigurationService::new(Some(Arc::clone(&bus)));
        config.load_layout("ui_layout.cfg");
        let mut state = MainFormState::new(
            config,
            Arc::clone(&bus),
            Some(dir.join("ui_layout.user.cfg").to_string_lossy().into_owned()),
        );
        let n_before = state.groups.len();

        // ① 按下 factoryReset → 挂起确认模态 (不执行)
        update(&mut state, Message::ButtonAction { action: "factoryReset".into() });
        assert!(state.pending_action.is_some(), "确认模态应挂起");
        assert_eq!(state.groups.len(), n_before, "未确认前不得重置");

        // ② 取消 → 无副作用
        update(&mut state, Message::CancelPending);
        assert!(state.pending_action.is_none());

        // ③ 再按 + 确认 → reset 执行 + 整树收敛 (模板组数回归)
        update(&mut state, Message::ButtonAction { action: "factoryReset".into() });
        update(&mut state, Message::ConfirmPending);
        assert!(state.pending_action.is_none(), "执行后模态关闭");
        assert!(
            state.groups.len() >= 10,
            "整树应从模板收敛 (实得 {} 组)",
            state.groups.len()
        );
    }
}
