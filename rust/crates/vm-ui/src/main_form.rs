//! MainForm 的表单数据层 (src/ui/MainForm.java + src/ui/layout/DynamicDataPage.java)。
//!
//! **D9 变更**: 设置窗换 Tauri 2 web 壳 (vm-webui) — 原 iced view 段 (view/
//! panel_section/build_rows/grid_section + ReadContext) 已删, 表单渲染归 web 壳;
//! 本模块仅存数据层 (Message/MainFormState/update/persist 写回链/run_headless)。
//! 下述 Elm/view 布局描述为 D1 历史备案。
//!
//! C 类语义复刻 (非机械翻译): Swing/WebLaF 无 Rust 对应物, 以 Elm 架构对位 —
//! - Java MainForm 的 WebTabbedPane 每 panel 一页 → 本 view 顺序平铺各 panel 区块
//!   (滚动列); tab 切换/窗口尺寸自适应属窗口管理层, 后续批次。
//! - Java DynamicDataPage.buildContainer 的 (group ...) 卡片/网格 → 原 build_rows
//!   的组标题 + 列数分块 (数据驱动, 不写死任何面板; view 已删, 布局归 web 壳)。
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

use vm_core::base::bus::Subscription;
use vm_core::config::config_api::ConfigProvider;
use vm_core::config::config_loader::{save_config as save_layout_file, GroupConfig, RowConfig};
use vm_core::config::configuration_service::ConfigurationService;
use vm_core::base::bus::ui_state_bus::{UIStateBus, UiStateEvent};
use vm_core::base::event::ui_state_events;
use vm_core::base::logger;
use crate::row_renderer_registry::RenderContext;

use crate::renderers;

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
// RenderContext 实现 (Java DynamicDataPage.java:126-175 匿名类)
// =====================================================================

/// 写侧上下文 (update 用): on_save/on_rebuild 以标志位暂存, 由 [`with_panel`] 统一
/// flush (对位 Java 回调直调 save()/rebuild(); Elm 下写路径在 update, 无重入面)。
pub(crate) struct WriteContext<'a> {
    config: &'a ConfigurationService,
    ui_bus: &'a UIStateBus,
    save_requested: Cell<bool>,
    rebuild_requested: Cell<bool>,
}

impl<'a> WriteContext<'a> {
    pub(crate) fn new(config: &'a ConfigurationService, ui_bus: &'a UIStateBus) -> Self {
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
        self.save_requested.set(true);
    }
    fn on_rebuild(&self) {
        self.rebuild_requested.set(true);
    }
    fn is_updating(&self) -> bool {
        // Java isUpdatingControls 仅 rebuild 期置位抑制 Swing 监听反馈环;
        // 本数据层无视图反馈环 (D9 后渲染归 web 壳), 恒 false
        false
    }
    fn sync_to_config_service(&self, key: &str, value: bool) {
        // Java L143-152: setConfig(key, Boolean.toString(value)) + enableFMPrint 特例
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

/// 表单消息驱动的状态更新 (D1 期为 iced update; D9 后由 web 壳经 vm-app
/// dispatcher 投递同一消息集, 链路不变)。
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
                .unwrap_or_else(|| vm_core::config::config_manager::get_user_config_path().to_string());
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

    // flush: on_save → 保存链; on_rebuild → D1 期为 iced 声明式视图每帧自重建,
    // D9 后视图刷新归 web 壳 (CONFIG_CHANGED 广播面), Java rebuild 的"取最新配置
    // 树"目的由 persist 的服务树为基承担
    // (先取标志再落保存链 — ctx 与 &mut state 的借用分界)
    let save = ctx.take_save();
    let rebuild = ctx.take_rebuild();
    if save {
        persist_and_notify(state);
    }
    if rebuild {
        logger::info("ComboDebug", "rebuild() called — 视图刷新归 web 壳 (D9)");
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

/// 无窗口状态机测试: 构建真实表单 → 驱动固定 Message 序列 → 断言 WYSIWYG 链路。
/// 返回进程退出码 (0 = 全部通过)。
///
/// `persist_path` (CLI `--persist <path>`): 固定序列落盘到指定路径 — D9 换框架
/// 验收工具 (相同序列在新旧 UI 层各跑一次 → ui_layout.user.cfg 逐字节 diff=0)。
/// None = 不落盘 (原纯链路断言形态)。
pub fn run_headless(persist_path: Option<String>) -> i32 {
    let Some(cfg_path) = locate_template_cfg() else {
        eprintln!("vm-ui: --headless 未找到 ui_layout.cfg (候选: ./ ../ ../../ ../../../)");
        return 2;
    };
    let bus = Arc::new(UIStateBus::new());
    let seen: Arc<Mutex<Vec<UiStateEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let s2 = Arc::clone(&seen);
    let _sub: Subscription<UiStateEvent> = bus.subscribe(
        ui_state_events::CONFIG_CHANGED,
        move |m: &UiStateEvent| {
            s2.lock().unwrap().push(m.clone());
        },
    );

    let config = ConfigurationService::new(Some(Arc::clone(&bus)));
    config.load_layout(cfg_path);
    if persist_path.is_some() {
        println!(
            "vm-ui: --persist 基线模式, 固定序列将落盘至 {}",
            persist_path.as_deref().unwrap_or_default()
        );
    }
    let mut state = MainFormState::new(config, Arc::clone(&bus), persist_path);
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
            .any(|e| e.event_type == ui_state_events::CONFIG_CHANGED && e.data.as_deref() == Some("ui_layout.cfg"));
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
mod tests;
