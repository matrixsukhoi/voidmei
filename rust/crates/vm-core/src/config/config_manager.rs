//! ConfigManager 的 Rust 移植 (src/prog/config/ConfigManager.java) — 一比一翻译。
//!
//! Manages dual-file configuration strategy with template hash detection and automatic merging.
//!
//! File responsibilities:
//! - ui_layout.cfg: Read-only template distributed with the program
//! - ui_layout.user.cfg: User configuration, actual read/write target
//! - ui_layout.user.cfg.bak: Automatic backup
//!
//! PORT: Java 类仅含 static 成员 → Rust 模块级自由函数 + 常量 (logger.rs 先例)。
//! PORT: Java 方法重载 (mergeConfigs 两参/三参) → Rust 无重载, 主名留给三参版
//! (信息完整形态), 两参版 `merge_configs_no_report` (logger.rs `_default` 后缀先例)。
//! PORT (§2.7 异常控制流): initialize/importConfig 的 `catch (Exception e)` 是
//! **Java 侧即不可达的死分支** — ConfigLoader.loadConfig/saveConfig 内部自吞一切
//! 异常返回部分结果 (config_loader.rs PORT 注), createBackup 自吞 IOException,
//! mergeConfigs 纯内存无抛出面。Rust 对应函数无异常面, 分支折叠并就地标注。
//! PORT: MessageDigest MD5 → 本文件内 RFC 1321 手写实现 (§3 禁重依赖;
//! Java MessageDigest.getInstance("MD5") 为必支持算法, 语义逐字节一致)。
//! PORT: prog.util.UIStateStorage 未译 (B 类后续波次) — 依赖桩 (非翻译) 已
//! 落地于 ui_state_storage.rs (波16 E5 抽出, md5.rs 同款先例), 顶住
//! loadTemplateHash/saveTemplateHash 消费面, 见该文件头注。
//! PORT: DialogService.showMessageDialog (C 类 Swing) → [`ConfigDialog`] sink
//! 转发 (vm-webui 波次装配): 组装层注入 tauri emit → 前端 Modal; 未装 sink
//! (启动早期配置装载先于 web 壳构造 / 无窗形态) 记日志兜底, 语义不丢。

// MD5 (RFC 1321) 手写实现 (§3 禁重依赖; 波11 抽出至 md5.rs)
use super::md5::md5_digest;

// UIStateStorage 依赖桩 (波16 E5 抽出至 ui_state_storage.rs)
use super::ui_state_storage::{ui_state_load_template_hash, ui_state_save_template_hash};

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};

use crate::config::config_loader::{load_config, save_config, GroupConfig, RowConfig};
use crate::lang::Lang;
use crate::base::logger;
use crate::base::java_compat::java_trim;

const TEMPLATE_PATH: &str = "./ui_layout.cfg";
const USER_PATH: &str = "./ui_layout.user.cfg";
const BACKUP_PATH: &str = "./ui_layout.user.cfg.bak";

/// Records details about what was merged during a config merge operation.
/// (Java: `public static class MergeReport`)
/// PORT: Java 字段初始化 `= new ArrayList<>()` ↔ derive(Default)。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MergeReport {
    pub added_panels: Vec<String>,
    pub added_items: Vec<String>,
    pub updated_items: Vec<String>,
}

impl MergeReport {
    /// Java: `public boolean hasChanges()`
    pub fn has_changes(&self) -> bool {
        !self.added_panels.is_empty() || !self.added_items.is_empty() || !self.updated_items.is_empty()
    }
}

/// Initializes configuration by handling first-run, template change detection, or parse errors.
///
/// @return List of GroupConfig to use for the application
pub fn initialize() -> Vec<GroupConfig> {
    let template_file = Path::new(TEMPLATE_PATH);
    let user_file = Path::new(USER_PATH);

    // Scenario: Template doesn't exist (shouldn't happen in normal distribution)
    if !template_file.exists() {
        logger::warn("ConfigManager", &format!("Template file not found: {TEMPLATE_PATH}"));
        return Vec::new();
    }

    // Calculate template file hash
    let current_template_hash = calculate_file_hash(TEMPLATE_PATH);

    // Scenario: First run - user config doesn't exist
    if !user_file.exists() {
        logger::info("ConfigManager", "First run detected, copying template to user config");
        let configs = load_config(TEMPLATE_PATH);
        save_config(USER_PATH, &configs);
        ui_state_save_template_hash(current_template_hash.as_deref());
        return configs;
    }

    // Try to load user config
    // PORT (§2.7): Java `try { loadConfig } catch (Exception e) { 解析错误弹窗+模板回退 }`
    // 为死分支 — loadConfig 内部 catch(Exception) 自吞 (部分组返回), 本方法层面无异常面。
    let user_configs = load_config(USER_PATH);

    // Read stored hash from UIStateStorage
    let stored_hash = ui_state_load_template_hash();

    // Hash matches - no merge needed
    // (equals(null) 恒 false ↔ Option 双 Some 匹配)
    if let (Some(cur), Some(stored)) = (&current_template_hash, &stored_hash) {
        if cur == stored {
            logger::info(
                "ConfigManager",
                &format!("Template unchanged, skipping merge, {cur} == {stored}"),
            );
            return user_configs;
        }
    }

    // Hash differs or missing - perform merge
    logger::info("ConfigManager", "Template changed, merging configs");
    create_backup();

    let template_configs = load_config(TEMPLATE_PATH);

    let mut report = MergeReport::default();
    let merged = merge_configs(&template_configs, &user_configs, Some(&mut report));

    save_config(USER_PATH, &merged);
    ui_state_save_template_hash(current_template_hash.as_deref());

    if report.has_changes() {
        show_merge_report(&report);
    }

    merged
}

/// Calculates the MD5 hash of a file.
///
/// @param filePath Path to the file
/// @return Hex string of the MD5 hash, or null on error
fn calculate_file_hash(file_path: &str) -> Option<String> {
    // (getInstance 对 MD5 不抛 NoSuchAlgorithmException — 必支持算法; 读文件 IO 是唯一异常面)
    match fs::read(file_path) {
        Ok(content) => {
            let hash = md5_digest(&content);
            let mut sb = String::new();
            for b in hash {
                sb.push_str(&format!("{b:02x}"));
            }
            Some(sb)
        }
        Err(e) => {
            logger::error("ConfigManager", &format!("Failed to calculate file hash: {e}"));
            None
        }
    }
}

/// Merges template and user configs, recording changes in the report.
///
/// Merge rules by field type:
/// - x, y, alpha, visible, value, hotkey: Preserve user settings
/// - columns, desc, format: Use template (latest definition)
/// - New config items: Insert with default values from template
///
/// @param template The template configuration (source of truth for structure)
/// @param user The user configuration (source of truth for user values)
/// @param report Records what was merged (can be null for silent merge)
/// @return Merged configuration
pub fn merge_configs(
    template: &[GroupConfig],
    user: &[GroupConfig],
    mut report: Option<&mut MergeReport>,
) -> Vec<GroupConfig> {
    // Build map of user panels by title
    // (HashMap 仅做点查, 输出顺序由 template Vec 驱动 — §2.5 无迭代序依赖)
    let mut user_panel_map: HashMap<&str, &GroupConfig> = HashMap::new();
    for gc in user {
        user_panel_map.insert(gc.title.as_str(), gc);
    }

    let mut merged: Vec<GroupConfig> = Vec::new();

    for template_panel in template {
        let user_panel = user_panel_map.get(template_panel.title.as_str());

        if user_panel.is_none() {
            // New panel in template - use template as-is
            merged.push(template_panel.clone());
            logger::info("ConfigManager", &format!("Added new panel: {}", template_panel.title));
            if let Some(r) = report.as_deref_mut() {
                r.added_panels.push(template_panel.title.clone());
            }
        } else {
            // Merge existing panel
            // PORT: Java 保真 — `userPanel == null` 判空 + else 分支直接用引用的直译
            #[allow(clippy::unnecessary_unwrap)]
            let merged_panel = merge_panel(template_panel, user_panel.unwrap(), report.as_deref_mut());
            merged.push(merged_panel);
        }
    }

    merged
}

/// Overload for backward compatibility (import, etc.) where no report is needed.
/// (Java: 两参 mergeConfigs 重载)
pub fn merge_configs_no_report(template: &[GroupConfig], user: &[GroupConfig]) -> Vec<GroupConfig> {
    merge_configs(template, user, None)
}

/// Merges a single panel from template and user.
fn merge_panel(template: &GroupConfig, user: &GroupConfig, report: Option<&mut MergeReport>) -> GroupConfig {
    let mut merged = GroupConfig::new(template.title.clone());

    // User-preserved fields
    merged.x = user.x;
    merged.y = user.y;
    merged.alpha = user.alpha;
    merged.visible = user.visible;

    // Template fields (structure)
    merged.columns = template.columns;
    merged.panel_columns = template.panel_columns;
    merged.font_name = template.font_name.clone();
    merged.font_size = template.font_size;
    merged.hotkey = user.hotkey; // Preserve user hotkey
    merged.switch_key = template.switch_key.clone();

    // Merge rows (pass panel title for report context)
    // PORT: Java 传 `template.title` 两次 (panelTitle 与 groupPath 同值, 后者未被读)
    merged.rows = merge_rows(
        &template.rows,
        &user.rows,
        &template.title,
        &template.title,
        report,
    );

    merged
}

/// Merges row configurations recursively.
///
/// @param panelTitle The panel title (for report display context)
/// @param groupPath Current path in hierarchy (e.g. "MiniHUD" or "MiniHUD/hud面板设置")
/// PORT: Java 的 groupPath 形参在方法体内从未被读取 (死参数, 保签名) — `_` 前缀消未用告警。
fn merge_rows(
    template_rows: &[RowConfig],
    user_rows: &[RowConfig],
    panel_title: &str,
    _group_path: &str,
    mut report: Option<&mut MergeReport>,
) -> Vec<RowConfig> {
    // Build map of user rows by property/target
    let mut user_row_map: HashMap<String, &RowConfig> = HashMap::new();
    build_row_map(user_rows, &mut user_row_map);

    let mut merged: Vec<RowConfig> = Vec::new();

    for template_row in template_rows {
        let key = get_row_key(template_row);

        // Skip merge tracking for items without valid keys (e.g., info-only items with empty labels)
        // These items have no user-modifiable values and don't need precise matching
        // PORT (§2.10): Java `key == null` 不可达 — getRowKey 的 property 分支之外恒返回
        // 非空容器 label (Rust String 无 null), 折叠为 isEmpty 判定
        if key.is_empty() {
            merged.push(template_row.clone());
            continue;
        }

        let user_row = user_row_map.get(&key);

        if user_row.is_none() {
            // New row in template - use template as-is
            merged.push(template_row.clone());
            logger::info("ConfigManager", &format!("Added new config item: {key}"));
            if let Some(r) = report.as_deref_mut() {
                let display_name = &template_row.label;
                r.added_items.push(format!("{panel_title}: {display_name}"));
            }
        } else {
            // Merge existing row
            // PORT: Java 保真 — `userRow == null` 判空 + else 分支直接用引用的直译
            #[allow(clippy::unnecessary_unwrap)]
            let merged_row = merge_row(template_row, user_row.unwrap(), panel_title, report.as_deref_mut());
            merged.push(merged_row);
        }
    }

    merged
}

/// Builds a map of rows by their key (property or label).
fn build_row_map<'a>(rows: &'a [RowConfig], map: &mut HashMap<String, &'a RowConfig>) {
    for row in rows {
        let key = get_row_key(row);
        if !key.is_empty() {
            map.insert(key, row);
        }
        // Recurse for HEADER/group children
        // (Java children 声明默认非 null `new ArrayList<>()`, 仅判 isEmpty)
        if !row.children.is_empty() {
            build_row_map(&row.children, map);
        }
    }
}

/// Gets the unique key for a row (property if available, otherwise label).
fn get_row_key(row: &RowConfig) -> String {
    if let Some(property) = &row.property {
        if !property.is_empty() {
            return property.clone();
        }
    }
    row.label.clone()
}

/// Merges a single row from template and user.
/// 检查点 1 (src/prog/config/CLAUDE.md): 新增 RowConfig 字段必须在此登记 —
/// 结构/定义字段取 template, 用户字段取 user, 遗漏即静默丢值 (2e53f94 教训)。
fn merge_row(
    template: &RowConfig,
    user: &RowConfig,
    panel_title: &str,
    report: Option<&mut MergeReport>,
) -> RowConfig {
    let mut merged = RowConfig::new(
        template.label.clone(),
        template.formula.clone(),
        template.format.clone(),
    );

    // Template fields (structure/definition)
    merged.r#type = template.r#type.clone();
    merged.property = template.property.clone();
    merged.unit = template.unit.clone();
    merged.format = template.format.clone();
    merged.desc = template.desc.clone();
    merged.desc_img = template.desc_img.clone();
    merged.preview_value = template.preview_value.clone();
    merged.hide_when_zero = template.hide_when_zero;
    merged.precision = template.precision;
    merged.target_name = template.target_name.clone();
    merged.min_val = template.min_val;
    merged.max_val = template.max_val;
    merged.group_columns = template.group_columns;
    merged.fg_color = template.fg_color.clone();
    merged.default_value = template.default_value.clone();
    // 动态单位/精度源（修复进气压单位显示问题）
    merged.unit_source = template.unit_source.clone();
    merged.precision_source = template.precision_source.clone();
    // 可见性和NA条件表达式（始终使用模板值，确保配置合并时保留显示控制逻辑）
    merged.visible_when = template.visible_when.clone();
    merged.na_when = template.na_when.clone();

    // User-preserved fields
    merged.value = user.value.clone();

    // Merge children recursively (pass report through for nested items)
    // PORT (§2.10): Java `user.children != null ? user.children : new ArrayList<>()`
    // — RowConfig.children 声明默认非 null (loadConfig 路径恒有实例), Rust Vec 恒为
    // 空容器, null 回退分支折叠
    if !template.children.is_empty() {
        merged.children = merge_rows(
            &template.children,
            &user.children,
            panel_title,
            &format!("{panel_title}/{}", template.label),
            report,
        );
    }

    merged
}

/// Creates a backup of the current user config.
pub fn create_backup() {
    let user_file = Path::new(USER_PATH);
    let backup_file = Path::new(BACKUP_PATH);

    if user_file.exists() {
        match fs::copy(user_file, backup_file) {
            Ok(_) => {
                logger::info("ConfigManager", &format!("Created backup: {BACKUP_PATH}"));
            }
            Err(e) => {
                logger::error("ConfigManager", &format!("Failed to create backup: {e}"));
            }
        }
    }
}

/// Imports configuration from an external file.
///
/// @param sourcePath Path to the config file to import
/// @return true if import was successful
pub fn import_config(source_path: &str) -> bool {
    if !Path::new(source_path).exists() {
        logger::error("ConfigManager", &format!("Import source not found: {source_path}"));
        return false;
    }

    // PORT (§2.7): Java 外层 `try { ... } catch (Exception e) { return false; }` 为死分支
    // — 体内 createBackup/loadConfig/mergeConfigs/saveConfig 均自吞异常 (见模块头注)。

    // Create backup before import
    create_backup();

    // Load and validate source config
    let imported_configs = load_config(source_path);
    if imported_configs.is_empty() {
        logger::error("ConfigManager", "Import file is empty or invalid");
        return false;
    }

    // Load template for merging
    let template_configs = load_config(TEMPLATE_PATH);

    // Merge imported config with template (to ensure structure compatibility)
    let merged = merge_configs_no_report(&template_configs, &imported_configs);

    // Save merged config
    save_config(USER_PATH, &merged);
    logger::info("ConfigManager", &format!("Config imported successfully from: {source_path}"));
    true
}

/// Resets configuration to factory defaults by copying template to user config.
///
/// @return true if reset was successful
pub fn reset_to_factory() -> bool {
    // Create backup before reset
    create_backup();

    // Copy template to user config
    let template_file = Path::new(TEMPLATE_PATH);
    let user_file = Path::new(USER_PATH);

    if !template_file.exists() {
        logger::error("ConfigManager", "Template file not found for factory reset");
        return false;
    }

    match fs::copy(template_file, user_file) {
        Ok(_) => {
            logger::info("ConfigManager", "Config reset to factory defaults");
            true
        }
        Err(e) => {
            logger::error("ConfigManager", &format!("Failed to reset config: {e}"));
            false
        }
    }
}

/// Gets the path to the user config file.
pub fn get_user_config_path() -> &'static str {
    USER_PATH
}

/// 跨模块共享的 CWD 测试锁 (审查 B4 落地): 进程级 CWD 对全部并行测试线程可见 —
/// 本模块沙箱测试 (chdir 型) 与任何触发全局路径 `./ui_layout.user.cfg` 落盘的
/// 用例 (configuration_service 的 save_group_position/save_window_position 族)
/// 必须经此锁互斥, 否则并行落盘会写进他人沙箱。锁内不 chdir 的调用方也须持锁
/// (其相对路径落盘与沙箱 chdir 互斥)。
#[cfg(test)]
pub(crate) static CWD_LOCK: Mutex<()> = Mutex::new(());

/// Gets the path to the template config file.
pub fn get_template_config_path() -> &'static str {
    TEMPLATE_PATH
}

/// 配置弹窗请求 (Java showParseErrorDialog/showMergeReport 的弹窗参数面):
/// web 壳形态经 sink 转发前端 Modal (组装层注入 tauri emit); 标题/正文在
/// Rust 侧以 Lang 就绪 (单一来源), 前端只渲染。
pub enum ConfigDialog {
    /// 解析失败弹窗 (Lang.mConfigErrorTitle + mConfigErrorContent,
    /// WebOptionPane.ERROR_MESSAGE 形态)
    ParseError,
    /// 合并报告弹窗 (Lang.mConfigMergedTitle + 逐条报告正文,
    /// WebOptionPane.INFORMATION_MESSAGE 形态)
    MergeReport(String),
}

/// 弹窗 sink 签名 (clippy type_complexity 折叠)
type ConfigDialogSink = Arc<dyn Fn(&ConfigDialog) + Send + Sync>;

/// 弹窗转发 sink (Java: SwingUtilities.invokeLater + DialogService → EDT 弹窗;
/// Rust: 任意线程可调, sink 自行派发)。Arc 承载: dispatch 锁内克隆即放锁再调用
/// (回调不得持锁执行 — 回调面若重入本域不会死锁)。组装层构造 web 壳后注入,
/// 启动早期 (AppShell 配置装载先于 web 壳) 未装时走日志兜底。
static CONFIG_DIALOG_SINK: LazyLock<Mutex<Option<ConfigDialogSink>>> =
    LazyLock::new(|| Mutex::new(None));

/// 启动期未达弹窗缓存 (审查 W2): 无 sink 时 (AppShell 配置装载先于 web 壳装配)
/// 的 ParseError/MergeReport 日志兜底之外再缓存最后一条; 组装层在 **web 就绪后**
/// 经 [`replay_pending_config_dialog`] 补发 — 不在 sink 安装时点立即回放, 因彼时
/// 前端 config-dialog 监听尚未注册 (App.tsx 就绪序: 监听注册 → ui_ready), 立即
/// 回放会再丢一次。Java 升级首跑的用户可见合并报告 (ConfigManager)
/// 由此在 web 形态达用户。
static PENDING_CONFIG_DIALOG: Mutex<Option<ConfigDialog>> = Mutex::new(None);

/// 安装弹窗 sink (vm-app 组装层: web 壳构造后调用; 覆盖式, 生产单装点)
pub fn set_config_dialog_sink(sink: ConfigDialogSink) {
    *CONFIG_DIALOG_SINK.lock().expect("config 弹窗 sink 锁中毒") = Some(sink);
}

/// 测试专用: 摘除 sink (恢复日志兜底路径; Drop 守卫配套); 同清待发缓存,
/// 不把残留弹窗漏给后续用例 (并行测试隔离)
#[cfg(test)]
pub(crate) fn remove_config_dialog_sink_for_test() {
    *CONFIG_DIALOG_SINK.lock().expect("config 弹窗 sink 锁中毒") = None;
    *PENDING_CONFIG_DIALOG.lock().expect("config 待发弹窗锁中毒") = None;
}

/// 回放缓存的未达弹窗 (组装层 web 就绪后调用一次): 经已装 sink 转发前端,
/// 取后清空。返回是否回放了弹窗 (日志/测试面)。
pub fn replay_pending_config_dialog() -> bool {
    let pending = PENDING_CONFIG_DIALOG
        .lock()
        .expect("config 待发弹窗锁中毒")
        .take();
    match pending {
        Some(dialog) => {
            dispatch_config_dialog(dialog);
            true
        }
        None => false,
    }
}

/// 测试专用: 清空待发弹窗缓存 (并行用例隔离 — 不捡他人 initialize 的残留)
#[cfg(test)]
pub(crate) fn clear_pending_config_dialog_for_test() {
    *PENDING_CONFIG_DIALOG.lock().expect("config 待发弹窗锁中毒") = None;
}

/// 弹窗分发: sink 在场转发; 未装 (启动早期/无窗形态) 记日志兜底 + 缓存待补发
fn dispatch_config_dialog(dialog: ConfigDialog) {
    let sink = CONFIG_DIALOG_SINK
        .lock()
        .expect("config 弹窗 sink 锁中毒")
        .clone();
    match sink {
        Some(sink) => sink(&dialog),
        None => {
            // Java 此处必弹窗; 无 sink 时日志承载 (对位 Java 弹窗的可见性)
            let lang = Lang::init_lang();
            match &dialog {
                ConfigDialog::ParseError => logger::warn(
                    "ConfigManager",
                    &format!("[弹窗兜底] {}: {}", lang.m_config_error_title, lang.m_config_error_content),
                ),
                ConfigDialog::MergeReport(message) => logger::info(
                    "ConfigManager",
                    &format!("[弹窗兜底] {}: {}", lang.m_config_merged_title, message),
                ),
            }
            // 缓存待 web 就绪后补发 (审查 W2; 覆盖式留最后一条 — Java 首启
            // 模板升级的合并报告单发, 无连弹队列面)
            *PENDING_CONFIG_DIALOG
                .lock()
                .expect("config 待发弹窗锁中毒") = Some(dialog);
        }
    }
}

/// Shows a detailed merge report dialog listing what was added/updated.
fn show_merge_report(report: &MergeReport) {
    let lang = Lang::init_lang();
    let mut sb = String::new();

    if !report.added_panels.is_empty() {
        sb.push_str(lang.m_merge_added_panels);
        sb.push('\n');
        for panel in &report.added_panels {
            sb.push_str(&format!("  \u{2022} {panel}\n"));
        }
        sb.push('\n');
    }

    if !report.added_items.is_empty() {
        sb.push_str(lang.m_merge_added_items);
        sb.push('\n');
        for item in &report.added_items {
            sb.push_str(&format!("  \u{2022} {item}\n"));
        }
        sb.push('\n');
    }

    if !report.updated_items.is_empty() {
        sb.push_str(lang.m_merge_updated_items);
        sb.push('\n');
        for item in &report.updated_items {
            sb.push_str(&format!("  \u{2022} {item}\n"));
        }
    }

    let message = java_trim(&sb).to_string();
    logger::info("ConfigManager", &format!("Merge report:\n{message}"));

    // Using DialogService to avoid overlay blocking
    dispatch_config_dialog(ConfigDialog::MergeReport(message));
}

// =====================================================================
// Tests — 公共项边界测试 (merge 双检查点语义 + 文件流)。
// Java 侧无对应单测 (ConfigManager 为手动验证), 本组为移植边界钉子。
// =====================================================================
#[cfg(test)]
mod tests;
