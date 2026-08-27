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
//! PORT: prog.util.UIStateStorage 未译 (B 类后续波次) — 本文件尾部的依赖桩
//! (非翻译) 顶住 loadTemplateHash/saveTemplateHash 消费面, 见桩头注。
//! PORT: DialogService/SwingUtilities.invokeLater 属 C 类 Swing 接线 (vm-ui 波次),
//! showParseErrorDialog/showMergeReport 的弹窗调用以 TODO(port) 挂起。

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config_loader::{load_config, save_config, GroupConfig, RowConfig};
use crate::lang::Lang;
use crate::logger;
#[cfg(test)]
use std::sync::Mutex;

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
    // Java: currentTemplateHash != null && currentTemplateHash.equals(storedHash)
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
    // Java: try { MessageDigest.getInstance("MD5") ... } catch (Exception e) { log; return null; }
    // (getInstance 对 MD5 不抛 NoSuchAlgorithmException — 必支持算法; 读文件 IO 是唯一异常面)
    match fs::read(file_path) {
        Ok(content) => {
            let hash = md5_digest(&content);
            let mut sb = String::new();
            for b in hash {
                // Java: String.format("%02x", b) — byte 按无符号两位小写 hex
                sb.push_str(&format!("{b:02x}"));
            }
            Some(sb)
        }
        Err(e) => {
            // Java: e.getMessage() ↔ io::Error Display (可观测意图等价, 框架先例 print_java_exception)
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
    // Java: new RowConfig(template.label, template.formula, template.format)
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
        // Java: Files.copy(..., REPLACE_EXISTING) ↔ fs::copy (存在即覆盖)
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

    // Java: Files.copy(..., REPLACE_EXISTING) 抛 IOException → catch → log + false
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

/// Java `String.trim()`: 剥首尾所有 `<= U+0020` 的字符 — 与 Rust `str::trim`
/// (Unicode White_Space, 会剥 U+3000 等) 不同; config_loader.rs 同款复刻。
fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| (c as u32) <= 0x20)
}

/// Shows a dialog when config parsing fails.
/// PORT: 调用点 (initialize 的 parse-error 分支) 为 Java 侧即不可达的死路径
/// (§2.7, 见 initialize 内标注), Java 同为永不走到的代码 — allow(dead_code) 保形。
#[allow(dead_code)]
fn show_parse_error_dialog() {
    // Run on EDT for thread safety (using DialogService to avoid overlay blocking)
    // TODO(port): javax.swing.SwingUtilities.invokeLater + DialogService.showMessageDialog(
    //     null, Lang.mConfigErrorContent, Lang.mConfigErrorTitle,
    //     WebOptionPane.ERROR_MESSAGE) — C 类 Swing 接线 (vm-ui 波次);
    // 文本届时经 crate::lang::Lang 取 m_config_error_content / m_config_error_title。
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
    // TODO(port): SwingUtilities.invokeLater + DialogService.showMessageDialog(
    //     null, message, Lang.mConfigMergedTitle, WebOptionPane.INFORMATION_MESSAGE)
    //     — C 类 Swing 接线 (vm-ui 波次); message 构造与 Logger 输出已保真落地。
}

// =====================================================================
// MD5 (RFC 1321) — java.security.MessageDigest.getInstance("MD5") 的
// 无依赖等价实现。Java 原生库逐字节一致 (RFC 向量测试钉住)。
// =====================================================================

/// MD5 摘要。PORT (§2.2): Java MessageDigest 内部 32 位运算静默回绕 ↔
/// 全程 wrapping_add/rotate_left 复刻。
fn md5_digest(input: &[u8]) -> [u8; 16] {
    // 每轮循环左移表: [7,12,17,22]*4, [5,9,14,20]*4, [4,11,16,23]*4, [6,10,15,21]*4
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
        14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
        21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    // K[i] = floor(abs(sin(i+1)) * 2^32) 的整数部分 (RFC 1321)
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];

    let mut a0: u32 = 0x6745_2301;
    let mut b0: u32 = 0xefcd_ab89;
    let mut c0: u32 = 0x98ba_dcfe;
    let mut d0: u32 = 0x1032_5476;

    // 填充: 0x80 + 0x00* + 64 位小端比特长度 (至 len ≡ 56 mod 64)
    let mut msg = input.to_vec();
    let bit_len = (input.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut m = [0u32; 16];
        for (i, w) in m.iter_mut().enumerate() {
            *w = u32::from_le_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }

        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let sum = f.wrapping_add(a).wrapping_add(K[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(sum.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..16].copy_from_slice(&d0.to_le_bytes());
    out
}

// =====================================================================
// prog.util.UIStateStorage 依赖桩 —— **不是** UIStateStorage 的翻译。
// (ui_model/config_stub.rs 桩先例; 仅覆盖 ConfigManager 消费面
// loadTemplateHash/saveTemplateHash 两个方法)
//
// TODO(port): ui_state_storage 波次落地后删除本节, initialize() 的两处调用
// 切换到 `crate::ui_state_storage::{load_template_hash, save_template_hash}`。
// =====================================================================

/// Java UIStateStorage.APP_NAME / STATE_FILE / KEY_TEMPLATE_HASH 原值
const UI_STATE_APP_NAME: &str = "voidmei";
const UI_STATE_FILE: &str = "ui_state.properties";
const UI_STATE_KEY_TEMPLATE_HASH: &str = "templateConfigHash";

/// 测试注入点: ui_state 目录覆盖 (Java 无此面 — 否则单测会写穿开发者真实
/// %APPDATA%/voidmei)。对齐 config_loader::set_legacy_screen_size 注入先例。
#[cfg(test)]
static UI_STATE_DIR_OVERRIDE: std::sync::RwLock<Option<PathBuf>> = std::sync::RwLock::new(None);

#[cfg(test)]
fn set_ui_state_dir_override(dir: Option<PathBuf>) {
    *UI_STATE_DIR_OVERRIDE.write().unwrap() = dir;
}

/// Java UIStateStorage.getConfigDir() 的路径规则:
/// Windows: %APPDATA%\\voidmei (无 APPDATA 则 ~\\.voidmei);
/// Linux: $XDG_CONFIG_HOME/voidmei (空/缺省则 ~/.config/voidmei);
/// 其余 (macOS): ~/.voidmei。
/// PORT: Java `System.getProperty("os.name")` 运行期判定 ↔ Rust cfg! 编译期目标
/// 三平台二进制等价; `user.home` ↔ USERPROFILE/HOME 环境变量。
/// PORT: Java getenv 判 null (空串可用) ↔ env::var().ok() 只滤未设置; XDG 的
/// `!= null && !isEmpty()` ↔ ok().filter非空 — 两处判定严格对齐 Java。
fn ui_state_config_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(d) = UI_STATE_DIR_OVERRIDE.read().unwrap().as_ref() {
        return d.clone();
    }

    // Java: System.getProperty("user.home") — 桩以 USERPROFILE/HOME 近似, 缺省 "."
    let user_home = || -> String {
        let v = if cfg!(windows) { env::var("USERPROFILE") } else { env::var("HOME") };
        v.unwrap_or_else(|_| ".".to_string())
    };
    // PORT: Java 是字符串拼接 `base + File.separator + tail` — 基座为空串时
    // (如 APPDATA="") 得 "\voidmei" (当前盘根的绝对路径); PathBuf::from("").join(tail)
    // 会折叠成相对路径 voidmei, 故同样以拼接复刻
    let join = |base: String, tail: String| -> String {
        format!("{base}{}{tail}", std::path::MAIN_SEPARATOR)
    };

    if cfg!(windows) {
        if let Ok(app_data) = env::var("APPDATA") {
            return PathBuf::from(join(app_data, UI_STATE_APP_NAME.to_string()));
        }
        PathBuf::from(join(user_home(), format!(".{UI_STATE_APP_NAME}")))
    } else if cfg!(target_os = "linux") {
        if let Some(xdg) = env::var("XDG_CONFIG_HOME").ok().filter(|s| !s.is_empty()) {
            return PathBuf::from(join(xdg, UI_STATE_APP_NAME.to_string()));
        }
        PathBuf::from(join(join(user_home(), ".config".to_string()), UI_STATE_APP_NAME.to_string()))
    } else {
        // macOS or others
        PathBuf::from(join(user_home(), format!(".{UI_STATE_APP_NAME}")))
    }
}

/// Java UIStateStorage.getConfigFile(): 目录不存在则 mkdirs (读路径同样建目录,
/// 原行为如此), 返回 <dir>/ui_state.properties。
fn ui_state_config_file() -> PathBuf {
    let dir = ui_state_config_dir();
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir); // Java dir.mkdirs() 返回值被忽略
    }
    dir.join(UI_STATE_FILE)
}

/// java.util.Properties.load 的兼容解析 (桩自用):
/// '#'/'!' 注释行、空行跳过; key 以未转义空白(' '/'\t'/'\f')/':'/'=' 终止; 值前
/// 分隔符与空白剥离; 行尾奇数 '\\' 续行 (续行首白空间丢弃, Properties.load 规范)。
/// \\n\\t\\r\\f 与 \\uXXXX 反转义 (JDK 常规转义面)。按字节索引切分 (键 ASCII 域安全)。
fn ui_state_parse_properties(text: &str) -> Vec<(String, String)> {
    let physical: Vec<&str> = text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();

    let count_trailing_backslashes = |s: &str| -> usize {
        s.bytes().rev().take_while(|&b| b == b'\\').count()
    };

    let unescape = |s: &str| -> String {
        let mut out = String::new();
        let mut it = s.chars();
        while let Some(c) = it.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('f') => out.push('\u{c}'),
                Some('u') => {
                    // \uXXXX; Java 对非 4 位 hex 抛 IllegalArgumentException → 外层
                    // catch 返回 null, 桩从简取字面 'u' (域内键值均 ASCII, 不可达)
                    let hex: String = it.clone().take(4).collect();
                    if hex.len() == 4 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                        for _ in 0..4 {
                            it.next();
                        }
                        let cp = u32::from_str_radix(&hex, 16).unwrap();
                        // Java String 可存孤立代理项, Rust char 不能 — 以 U+FFFD 顶替
                        out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                    } else {
                        out.push('u');
                    }
                }
                Some(other) => out.push(other), // \\ → \, \: → : (Java: 保留字符原样)
                None => out.push('\\'),         // 行尾悬挂反斜杠 (域内不可达)
            }
        }
        out
    };

    let mut entries = Vec::new();
    let mut i = 0;
    while i < physical.len() {
        let mut logical = physical[i].to_string();
        i += 1;
        // 续行: 行尾奇数个 '\', 下一自然行首白空间 (' ','\t','\f') 被丢弃不拼接
        while count_trailing_backslashes(&logical) % 2 == 1 {
            logical.pop();
            if i < physical.len() {
                logical.push_str(physical[i].trim_start_matches([' ', '\t', '\u{c}']));
                i += 1;
            } else {
                break;
            }
        }

        let line = logical.trim_start_matches(|c: char| (c as u32) <= 0x20);
        if line.is_empty() {
            continue;
        }
        let first = line.as_bytes()[0];
        if first == b'#' || first == b'!' {
            continue;
        }

        let b = line.as_bytes();
        let mut j = 0;
        let mut key_end = b.len();
        while j < b.len() {
            let c = b[j];
            if c == b'\\' {
                j += 2; // 转义对不参与分隔判定
                continue;
            }
            if c == b' ' || c == b'\t' || c == 0x0c || c == b':' || c == b'=' {
                key_end = j;
                break;
            }
            j += 1;
        }
        let raw_key = &line[..key_end];

        let mut v = key_end;
        while v < b.len() && (b[v] == b' ' || b[v] == b'\t' || b[v] == 0x0c) {
            v += 1;
        }
        if v < b.len() && (b[v] == b':' || b[v] == b'=') {
            v += 1;
            while v < b.len() && (b[v] == b' ' || b[v] == b'\t' || b[v] == 0x0c) {
                v += 1;
            }
        }
        let raw_val = &line[v..];

        entries.push((unescape(raw_key), unescape(raw_val)));
    }
    entries
}

/// Java Properties.load 按 ISO-8859-1 逐字节读 (任何字节序均合法); fs::read_to_string
/// 的严格 UTF-8 校验会把含原始高位字节的文件打成 Err → 误触合并/重写丢他键。
/// 这里以 Latin-1 解码 (b → char, 无损), 仅 IO 失败走 Err — 对齐 Java 读面。
fn ui_state_read_properties(path: &Path) -> std::io::Result<Vec<(String, String)>> {
    let bytes = fs::read(path)?;
    let text: String = bytes.iter().map(|&b| b as char).collect();
    Ok(ui_state_parse_properties(&text))
}

/// java.util.Properties.store 的 saveConvert 对齐: '\\' 与 \t\n\r\f 助记符,
/// <0x20 / >0x7E 转 \\uXXXX (JDK toHex 大写十六进制), "=:# 与空格加反斜杠前缀。
/// 保证 Latin-1 域内字节经写-读往返无损, Java 端 load 语义等价。
fn ui_state_escape_store(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{c}' => out.push_str("\\f"),
            _ if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            '=' | ':' | '#' | ' ' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Java UIStateStorage.loadTemplateHash(): 文件存在则读 properties 返回
/// templateConfigHash (键缺失 → None); 文件不存在 → None。
fn ui_state_load_template_hash() -> Option<String> {
    let file = ui_state_config_file();
    if file.exists() {
        // Java: FileInputStream/Properties.load 均在 try 内 — 读失败走
        // catch(Exception) → Logger.info("UIStateStorage", ...) 后返回 null
        match ui_state_read_properties(&file) {
            Ok(entries) => {
                for (k, v) in entries {
                    if k == UI_STATE_KEY_TEMPLATE_HASH {
                        return Some(v);
                    }
                }
            }
            Err(e) => {
                logger::info("UIStateStorage", &format!("Failed to load template hash: {e}"));
            }
        }
    }
    None
}

/// Java UIStateStorage.saveTemplateHash(hash): 载入既有键保留他键 → set → store。
/// PORT: Java Properties.setProperty(key, null) 抛 NullPointerException 且
/// ConfigManager 无 catch → None 入参以 panic 复刻 (与 Java 同为调用方崩溃面)。
/// 写出格式 `key=value` 行, 非 ASCII 以 \\uXXXX 转义; Java store 额外写 #日期 行且
/// 行分隔符随平台 — 桩不写日期行/固定 \n (双向 load 语义等价)。
fn ui_state_save_template_hash(hash: Option<&str>) {
    let hash = match hash {
        Some(h) => h,
        None => panic!("java.lang.NullPointerException: Properties.setProperty null value"),
    };

    let file = ui_state_config_file();

    // Load existing to preserve other keys
    // (Java: 载入失败 catch(IOException) 静默忽略 ↔ unwrap_or_default)
    let mut entries = ui_state_read_properties(&file).unwrap_or_default();

    if let Some(e) = entries.iter_mut().find(|(k, _)| k == UI_STATE_KEY_TEMPLATE_HASH) {
        e.1 = hash.to_string();
    } else {
        entries.push((UI_STATE_KEY_TEMPLATE_HASH.to_string(), hash.to_string()));
    }

    let mut out = String::from("#UI State for VoidMei\n");
    for (k, v) in &entries {
        out.push_str(&format!(
            "{}={}\n",
            ui_state_escape_store(k),
            ui_state_escape_store(v)
        ));
    }
    // Java: store 失败 catch(IOException) → Logger.info 后吞
    if let Err(e) = fs::write(&file, out) {
        logger::info("UIStateStorage", &format!("Failed to save template hash: {e}"));
    }
}

// =====================================================================
// Tests — 公共项边界测试 (merge 双检查点语义 + 文件流)。
// Java 侧无对应单测 (ConfigManager 为手动验证), 本组为移植边界钉子。
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_loader::ConfigValue;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::MutexGuard;

    // ---- MD5 (RFC 1321 附录 A.5 七向量, 与 java.security.MessageDigest 逐字节一致) ----

    #[test]
    fn md5_rfc1321_test_vectors() {
        let cases = [
            ("", "d41d8cd98f00b204e9800998ecf8427e"),
            ("a", "0cc175b9c0f1b6a831c399e269772661"),
            ("abc", "900150983cd24fb0d6963f7d28e17f72"),
            ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
            ("abcdefghijklmnopqrstuvwxyz", "c3fcd3d76192e4007dfb496cca67e13b"),
            (
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
                "d174ab98d277d9f5a5611c2c9f419d9f",
            ),
            (
                "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
                "57edf4a22be3c955ac49da2e2107b67a",
            ),
        ];
        for (input, want) in cases {
            let got = calculate_file_hash_on_bytes(input.as_bytes());
            assert_eq!(got, want, "MD5({input:?}) 失配");
        }
    }

    /// calculate_file_hash 的字节级直通 (hex 化与文件版同一逻辑)
    fn calculate_file_hash_on_bytes(b: &[u8]) -> String {
        md5_digest(b).iter().map(|x| format!("{x:02x}")).collect()
    }

    fn tmp(name: &str) -> String {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir()
            .join(format!("vm_core_config_manager_{name}_{}_{n}", std::process::id()))
            .to_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn calculate_file_hash_reads_file_and_missing_returns_none() {
        let p = tmp("hash_ok.cfg");
        fs::write(&p, "abc").unwrap();
        assert_eq!(calculate_file_hash(&p).as_deref(), Some("900150983cd24fb0d6963f7d28e17f72"));
        // Java: Files.readAllBytes IOException → catch → null
        let missing = tmp("hash_missing.cfg");
        let _ = fs::remove_file(&missing);
        assert_eq!(calculate_file_hash(&missing), None);
    }

    // ---- mergeRow 双检查点 (CLAUDE.md 检查点 1): 结构字段←template / 用户字段←user ----

    /// 全属性对撞样本: 模板与用户各字段全异, 断言逐字段来源
    const TPL_RICH: &str = "(panel \"P\"\n  (item \"进气压\" :type data :target \"getM\"\n      :unit \"Ata\" :precision 2 :format \"%.2f\" :desc \"帮助\" :desc-img \"img.png\"\n      :preview-value \"500\" :fgcolor \"1,2,3,4\" :target-name \"表压\"\n      :unit-source \"getU\" :precision-source \"getP\"\n      :visible-when (> value 0) :na-when (> value 9999)\n      :min 1 :max 9 :hide-when-zero true\n      :value true :default true)\n)\n";
    const USR_RICH: &str = "(panel \"P\"\n  (item \"进气压\" :type data :target \"getM\"\n      :unit \"OLD\" :precision 9 :format \"%.9f\" :desc \"旧\" :desc-img \"old.png\"\n      :preview-value \"999\" :fgcolor \"9,9,9,9\" :target-name \"旧名\"\n      :min 77 :max 88 :hide-when-zero false\n      :value false :default false)\n)\n";

    fn load_str_cfg(name: &str, content: &str) -> Vec<GroupConfig> {
        let p = tmp(name);
        fs::write(&p, content).unwrap();
        load_config(&p)
    }

    #[test]
    fn merge_row_field_sources_double_checkpoint() {
        let template = load_str_cfg("rich_t.cfg", TPL_RICH);
        let user = load_str_cfg("rich_u.cfg", USR_RICH);
        let mut report = MergeReport::default();
        let merged = merge_configs(&template, &user, Some(&mut report));

        assert_eq!(merged.len(), 1);
        assert!(!report.has_changes(), "同键同面板应无新增");

        let row = &merged[0].rows[0];
        // --- 模板来源 (结构/定义) ---
        assert_eq!(row.label, "进气压"); // 构造器: label←template
        assert_eq!(row.formula.as_deref(), Some("getM")); // 构造器: formula←template
        assert_eq!(row.r#type, "DATA");
        assert_eq!(row.property.as_deref(), Some("getM"));
        assert_eq!(row.unit, "Ata");
        assert_eq!(row.format, "%.2f");
        assert_eq!(row.desc.as_deref(), Some("帮助"));
        assert_eq!(row.desc_img.as_deref(), Some("img.png"));
        assert_eq!(row.preview_value.as_deref(), Some("500"));
        assert!(row.hide_when_zero, "对撞: 模板 true vs 用户 false → 取模板");
        assert_eq!(row.precision, 2);
        assert_eq!(row.target_name.as_deref(), Some("表压"));
        assert_eq!(row.min_val, 1, "对撞: 模板 1 vs 用户 77 → 取模板 (滑块范围模板定义)");
        assert_eq!(row.max_val, 9, "对撞: 模板 9 vs 用户 88 → 取模板");
        assert_eq!(row.group_columns, 0, "item 无 :column 解析面 → 声明默认 (group 级对撞另测)");
        assert_eq!(row.fg_color.as_deref(), Some("1,2,3,4"));
        assert_eq!(row.default_value, Some(ConfigValue::Bool(true)));
        assert_eq!(row.unit_source.as_deref(), Some("getU"));
        assert_eq!(row.precision_source.as_deref(), Some("getP"));
        // SExp Display 形态 (config_loader 写回同款)
        assert_eq!(row.visible_when.as_ref().map(|e| e.to_string()), Some("(> value 0)".to_string()));
        assert_eq!(row.na_when.as_ref().map(|e| e.to_string()), Some("(> value 9999)".to_string()));
        // --- 用户来源 ---
        assert_eq!(row.value, Some(ConfigValue::Bool(false)));
    }

    /// 回归 2e53f94: 老版本用户配置无 :unit-source → 合并后必须拿到模板的
    /// 动态单位/精度/可见性逻辑 (曾因此美系机显示 "Ata" 而非 "P/inHg")
    #[test]
    fn merge_row_unit_source_regression_2e53f94() {
        let template = load_str_cfg("reg_t.cfg", TPL_RICH);
        let user = load_str_cfg("reg_u.cfg", USR_RICH); // 无 unit-source/precision-source/visible-when/na-when
        let merged = merge_configs_no_report(&template, &user);
        let row = &merged[0].rows[0];
        assert_eq!(row.unit_source.as_deref(), Some("getU"), "unitSource 必须取模板");
        assert_eq!(row.precision_source.as_deref(), Some("getP"));
        assert!(row.visible_when.is_some(), "visibleWhen 必须取模板");
        assert!(row.na_when.is_some(), "naWhen 必须取模板");
        assert_eq!(row.precision, 2, "precision 必须取模板");
        assert_eq!(row.unit, "Ata", "unit 必须取模板");
    }

    #[test]
    fn merge_row_children_recursive_with_report() {
        let tpl = "(panel \"P\"\n  (group \"G\" :column 2\n    (item \"内\" :type switch :target \"k1\" :value true)\n    (item \"新子\" :type switch :target \"k2\" :value true :default true))\n  (item \"顶\" :type switch :target \"k3\" :value true))\n";
        let usr = "(panel \"P\"\n  (group \"G\" :column 5\n    (item \"内\" :type switch :target \"k1\" :value false))\n  (item \"顶\" :type switch :target \"k3\" :value true))\n";
        let template = load_str_cfg("nest_t.cfg", tpl);
        let user = load_str_cfg("nest_u.cfg", usr);
        let mut report = MergeReport::default();
        let merged = merge_configs(&template, &user, Some(&mut report));

        assert_eq!(merged[0].rows.len(), 2);
        let group = &merged[0].rows[0];
        assert_eq!(group.r#type, "HEADER");
        assert_eq!(group.group_columns, 2, "对撞: 模板 :column 2 vs 用户 5 → 取模板");
        assert_eq!(group.children.len(), 2);
        // 子行: 用户值保留 / 模板新增项
        assert_eq!(group.children[0].value, Some(ConfigValue::Bool(false)));
        assert_eq!(group.children[1].label, "新子");
        assert_eq!(group.children[1].value, Some(ConfigValue::Bool(true)));
        // 顶行匹配
        assert_eq!(merged[0].rows[1].value, Some(ConfigValue::Bool(true)));
        // 新增子项记入报告, 显示名 = 面板标题: 标签
        assert_eq!(report.added_items, vec!["P: 新子".to_string()]);
        assert!(report.has_changes());
    }

    /// buildRowMap 扁平化: 用户嵌套子行可被模板顶层行按键命中 (全局键空间)
    #[test]
    fn merge_configs_row_map_flattens_nested_user_children() {
        let tpl = "(panel \"P\" (item \"顶\" :type switch :target \"k1\" :value true))\n";
        let usr = "(panel \"P\" (group \"G\" (item \"藏\" :type switch :target \"k1\" :value false)))\n";
        let template = load_str_cfg("flat_t.cfg", tpl);
        let user = load_str_cfg("flat_u.cfg", usr);
        let merged = merge_configs_no_report(&template, &user);
        assert_eq!(merged[0].rows.len(), 1, "用户独有 group 行应被丢弃 (模板驱动结构)");
        assert_eq!(merged[0].rows[0].label, "顶");
        assert_eq!(merged[0].rows[0].value, Some(ConfigValue::Bool(false)), "值应取自嵌套用户子行");
    }

    /// 重复键后写覆盖 (Java HashMap.put 后者胜)
    #[test]
    fn merge_configs_duplicate_key_last_wins() {
        let tpl = "(panel \"P\" (item \"甲\" :type switch :target \"k1\" :value true))\n";
        let usr = "(panel \"P\"\n  (item \"一\" :type switch :target \"k1\" :value true)\n  (item \"二\" :type switch :target \"k1\" :value false))\n";
        let template = load_str_cfg("dup_t.cfg", tpl);
        let user = load_str_cfg("dup_u.cfg", usr);
        let merged = merge_configs_no_report(&template, &user);
        assert_eq!(merged[0].rows[0].value, Some(ConfigValue::Bool(false)));
    }

    /// 无键行 (空 label 且无 target): 原样透传, 不进合并追踪
    #[test]
    fn merge_configs_keyless_rows_skip_tracking() {
        let tpl = "(panel \"P\" (item \"\" :type info :value 7))\n";
        let usr = "(panel \"P\" (item \"\" :type info :value 9))\n";
        let template = load_str_cfg("nokey_t.cfg", tpl);
        let user = load_str_cfg("nokey_u.cfg", usr);
        let mut report = MergeReport::default();
        let merged = merge_configs(&template, &user, Some(&mut report));
        assert_eq!(merged[0].rows[0].value, Some(ConfigValue::Int(7)), "无键行应取模板原值");
        assert!(!report.has_changes());
    }

    /// mergePanel 字段来源: x/y/alpha/visible/hotkey←user; 结构列←template
    #[test]
    fn merge_panel_field_sources() {
        let tpl = "(panel \"面板A\" :x 0.1 :y 0.2 :alpha 100 :visible true :switch-key \"swA\"\n      :font \"F\" :font-size 3 :columns 1 :panel-columns 4 :hotkey \"P\"\n  (item \"甲\" :type switch :target \"k1\" :value true))\n";
        let usr = "(panel \"面板A\" :x 0.7 :y 0.8 :alpha 222 :visible false :hotkey \"F5\"\n  (item \"甲\" :type switch :target \"k1\" :value false))\n";
        let template = load_str_cfg("panel_t.cfg", tpl);
        let user = load_str_cfg("panel_u.cfg", usr);
        let merged = merge_configs_no_report(&template, &user);
        let m = &merged[0];
        // --- 用户来源 ---
        assert!((m.x - 0.7).abs() < 1e-12);
        assert!((m.y - 0.8).abs() < 1e-12);
        assert_eq!(m.alpha, 222);
        assert!(!m.visible);
        assert_eq!(m.hotkey, 63, "hotkey 应取用户 (F5=63), 模板 P=25");
        // --- 模板来源 (结构) ---
        assert_eq!(m.title, "面板A");
        assert_eq!(m.columns, 1);
        assert_eq!(m.panel_columns, 4);
        assert_eq!(m.font_name.as_deref(), Some("F"));
        assert_eq!(m.font_size, 3);
        assert_eq!(m.switch_key.as_deref(), Some("swA"));
        // 行值仍走用户
        assert_eq!(m.rows[0].value, Some(ConfigValue::Bool(false)));
    }

    /// 模板新增面板: 原样并入 + 报告记录; 用户独有面板被丢弃
    #[test]
    fn merge_configs_new_panel_reported_user_panel_dropped() {
        let tpl = "(panel \"A\" (item \"甲\" :type switch :target \"k1\" :value true))\n(panel \"B\" (item \"乙\" :type switch :target \"k2\" :value false))\n";
        let usr = "(panel \"A\" (item \"甲\" :type switch :target \"k1\" :value false))\n(panel \"C\" (item \"丙\" :type switch :target \"k9\" :value true))\n";
        let template = load_str_cfg("newp_t.cfg", tpl);
        let user = load_str_cfg("newp_u.cfg", usr);
        let mut report = MergeReport::default();
        let merged = merge_configs(&template, &user, Some(&mut report));

        assert_eq!(merged.len(), 2, "输出面板集合 = 模板面板集合");
        assert_eq!(merged[0].title, "A");
        assert_eq!(merged[1].title, "B");
        assert!(merged.iter().all(|g| g.title != "C"), "用户独有面板 C 应被丢弃");
        assert_eq!(report.added_panels, vec!["B".to_string()]);
        // Java: 全新面板走 `merged.add(templatePanel)` 整体透传, **不调 mergeRows**
        // — 面板内新项不进 addedItems (报告只记面板级); 新项报告仅发生在
        // 既有面板内 (见 merge_row_children_recursive_with_report)
        assert!(report.added_items.is_empty(), "全新面板的内部条目不应逐项报告");
    }

    #[test]
    fn merge_configs_no_report_equals_silent_merge() {
        let template = load_str_cfg("silent_t.cfg", TPL_RICH);
        let user = load_str_cfg("silent_u.cfg", USR_RICH);
        assert_eq!(
            merge_configs_no_report(&template, &user),
            merge_configs(&template, &user, None)
        );
    }

    /// 钉住 Java 原状: updated_items 声明了收集与展示, 但全类无任何写入点 — 恒空
    #[test]
    fn merge_report_updated_items_never_written_has_changes_matrix() {
        let mut r = MergeReport::default();
        assert!(!r.has_changes());
        r.added_panels.push("p".into());
        assert!(r.has_changes());
        let mut r = MergeReport::default();
        r.added_items.push("i".into());
        assert!(r.has_changes());
        let mut r = MergeReport::default();
        r.updated_items.push("u".into());
        assert!(r.has_changes());

        // 全量合并后 updated_items 仍恒空 (新增面板/新增项都不碰它)
        let tpl = "(panel \"A\" (item \"甲\" :type switch :target \"k1\" :value true))\n(panel \"B\" (item \"乙\" :type switch :target \"k2\" :value false))\n";
        let usr = "(panel \"A\" (item \"甲\" :type switch :target \"k1\" :value false))\n";
        let template = load_str_cfg("upd_t.cfg", tpl);
        let user = load_str_cfg("upd_u.cfg", usr);
        let mut r2 = MergeReport::default();
        let _ = merge_configs(&template, &user, Some(&mut r2));
        assert!(r2.has_changes());
        assert!(r2.updated_items.is_empty(), "Java 原实现无 updatedItems 写入点");
    }

    #[test]
    fn config_paths_constants_match_java() {
        assert_eq!(get_user_config_path(), "./ui_layout.user.cfg");
        assert_eq!(get_template_config_path(), "./ui_layout.cfg");
        assert_eq!(BACKUP_PATH, "./ui_layout.user.cfg.bak");
    }

    // ---- 文件流 (initialize/import/reset/backup): CWD 沙箱 + ui_state 目录注入 ----
    // 相对路径常量照 Java (工作区根运行), 测试以串行锁 + 沙箱目录隔离。
    // CWD_LOCK 已上移模块级 pub(crate) (跨模块共享 — 见其声明处注, 审查 B4)。

    // 注意: lang/blkx 等模块读 "./lang/cur.properties"、"./data/..." 的相对路径
    // 用例不在锁保护内 — 现状两类 CWD 下同 miss/降级一致故全绿。

    /// CWD 沙箱守卫: Drop 恢复原工作目录并清除 ui_state 注入 (panic 安全)
    struct SandboxGuard {
        orig: PathBuf,
    }
    impl Drop for SandboxGuard {
        fn drop(&mut self) {
            set_ui_state_dir_override(None);
            let _ = env::set_current_dir(&self.orig);
        }
    }

    fn sandbox(name: &str) -> (MutexGuard<'static, ()>, SandboxGuard) {
        let guard = CWD_LOCK.lock().expect("cwd 测试锁中毒");
        let dir = tmp_dir(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        set_ui_state_dir_override(Some(dir.join("ui_state")));
        let orig = env::current_dir().unwrap();
        env::set_current_dir(&dir).unwrap();
        (guard, SandboxGuard { orig })
    }

    fn tmp_dir(name: &str) -> PathBuf {
        static DIRS: AtomicUsize = AtomicUsize::new(0);
        let n = DIRS.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("vm_core_cm_dir_{name}_{}_{n}", std::process::id()))
    }

    fn md5_hex(b: &[u8]) -> String {
        md5_digest(b).iter().map(|x| format!("{x:02x}")).collect()
    }

    const TPL_V1: &str = "(panel \"面板A\"\n  :x 0.1 :y 0.2 :alpha 100 :visible true :switch-key \"swA\"\n      :font \"F\" :font-size 3 :columns 1 :panel-columns 4 :hotkey \"P\"\n  (item \"甲\" :type switch :target \"k1\" :value true)\n  (item \"乙\" :type slider :target \"k2\" :min 1 :max 9 :value 5 :unit \"px\")\n)\n";
    const TPL_V2: &str = "(panel \"面板A\"\n  :x 0.1 :y 0.2 :alpha 100 :visible true :switch-key \"swA\"\n      :font \"F\" :font-size 3 :columns 1 :panel-columns 4 :hotkey \"P\"\n  (item \"甲\" :type switch :target \"k1\" :value true)\n  (item \"乙\" :type slider :target \"k2\" :min 1 :max 9 :value 5 :unit \"px\")\n  (item \"丙\" :type switch :target \"k3\" :value false :default false)\n)\n(panel \"面板B\"\n  :x 0.4 :y 0.4\n  (item \"丁\" :type combo :target \"k4\" :value \"X\")\n)\n";

    #[test]
    fn initialize_missing_template_returns_empty() {
        let (_g, _sg) = sandbox("init_no_tpl");
        let cfgs = initialize();
        assert!(cfgs.is_empty());
        assert!(!Path::new(USER_PATH).exists(), "首跑分支未走, 不应生成用户配置");
    }

    #[test]
    fn initialize_first_run_copies_template_and_saves_hash() {
        let (_g, _sg) = sandbox("init_first");
        fs::write(TEMPLATE_PATH, TPL_V1).unwrap();
        let cfgs = initialize();

        assert_eq!(cfgs.len(), 1);
        assert_eq!(cfgs[0].title, "面板A");
        // 用户配置已落盘且可重读
        assert!(Path::new(USER_PATH).exists());
        assert_eq!(load_config(USER_PATH)[0].title, "面板A");
        // 模板哈希已存 (与 Java 共存 ui_state.properties 的 templateConfigHash)
        assert_eq!(ui_state_load_template_hash().as_deref(), Some(md5_hex(TPL_V1.as_bytes()).as_str()));
        // 首跑不生成备份
        assert!(!Path::new(BACKUP_PATH).exists());
    }

    #[test]
    fn initialize_unchanged_template_skips_merge() {
        let (_g, _sg) = sandbox("init_same");
        fs::write(TEMPLATE_PATH, TPL_V1).unwrap();
        initialize();

        // 用户改值: k1 → false
        let mut user = load_config(USER_PATH);
        user[0].rows[0].value = Some(ConfigValue::Bool(false));
        save_config(USER_PATH, &user);

        let cfgs = initialize();
        assert_eq!(cfgs[0].rows[0].value, Some(ConfigValue::Bool(false)), "哈希一致应原样返回用户配置");
        assert!(!Path::new(BACKUP_PATH).exists(), "跳过合并不生成备份");
        assert_eq!(ui_state_load_template_hash().as_deref(), Some(md5_hex(TPL_V1.as_bytes()).as_str()));
    }

    #[test]
    fn initialize_template_change_merges_backs_up_updates_hash() {
        let (_g, _sg) = sandbox("init_merge");
        fs::write(TEMPLATE_PATH, TPL_V1).unwrap();
        initialize();

        // 用户改值: k1 → false, x → 0.9
        let mut user = load_config(USER_PATH);
        user[0].rows[0].value = Some(ConfigValue::Bool(false));
        user[0].x = 0.9;
        save_config(USER_PATH, &user);
        let pre_merge_user = fs::read(USER_PATH).unwrap();

        // 模板升级 (V1 → V2: 新增面板 B + 新增项 k3)
        fs::write(TEMPLATE_PATH, TPL_V2).unwrap();

        let cfgs = initialize();
        assert_eq!(cfgs.len(), 2, "新面板 B 应并入");
        assert_eq!(cfgs[0].rows[0].value, Some(ConfigValue::Bool(false)), "用户开关值保留");
        assert!((cfgs[0].x - 0.9).abs() < 1e-12, "用户面板位置保留");
        assert_eq!(cfgs[0].rows[2].property.as_deref(), Some("k3"), "模板新项并入");
        assert_eq!(cfgs[0].rows[2].value, Some(ConfigValue::Bool(false)), "模板新项取模板默认值");
        assert_eq!(cfgs[1].title, "面板B");

        // 备份 = 合并前用户文件; 用户文件已重写为合并结果; 哈希更新为新模板
        assert_eq!(fs::read(BACKUP_PATH).unwrap(), pre_merge_user, "备份应是合并前的用户配置");
        let reread = load_config(USER_PATH);
        assert_eq!(reread.len(), 2);
        assert_eq!(reread[0].rows[0].value, Some(ConfigValue::Bool(false)));
        assert_eq!(ui_state_load_template_hash().as_deref(), Some(md5_hex(TPL_V2.as_bytes()).as_str()));
    }

    /// 存储哈希缺失 (老版本程序升级, 无 ui_state 记录) → 触发合并
    #[test]
    fn initialize_missing_stored_hash_triggers_merge() {
        let (_g, _sg) = sandbox("init_nohash");
        fs::write(TEMPLATE_PATH, TPL_V2).unwrap();
        // 手工放置用户配置 (不走首跑, 不落哈希)
        fs::write(USER_PATH, TPL_V1).unwrap();
        assert_eq!(ui_state_load_template_hash(), None);

        let cfgs = initialize();
        assert_eq!(cfgs.len(), 2, "应走合并路径并入新面板");
        assert!(Path::new(BACKUP_PATH).exists());
        assert_eq!(ui_state_load_template_hash().as_deref(), Some(md5_hex(TPL_V2.as_bytes()).as_str()));
    }

    #[test]
    fn create_backup_only_when_user_exists() {
        let (_g, _sg) = sandbox("bak_none");
        create_backup();
        assert!(!Path::new(BACKUP_PATH).exists(), "无用户配置不生成备份");

        fs::write(USER_PATH, "old-bytes").unwrap();
        create_backup();
        assert_eq!(fs::read(BACKUP_PATH).unwrap(), b"old-bytes", "备份覆盖已有 .bak 并复制用户文件");
    }

    #[test]
    fn import_config_success_merges_with_template() {
        let (_g, _sg) = sandbox("imp_ok");
        fs::write(TEMPLATE_PATH, TPL_V2).unwrap();
        // 外部配置: 基于 V1 且 k1 关闭 (TPL_V1 中仅 "甲" 带 :value true, 乙是 :value 5)
        let external = TPL_V1.replacen(":value true", ":value false", 1);
        fs::write("external.cfg", &external).unwrap();

        assert!(import_config("external.cfg"));
        assert!(Path::new(USER_PATH).exists());
        // 导入前无既有用户配置 → createBackup 无源可拷, .bak 不落盘
        assert!(!Path::new(BACKUP_PATH).exists());

        let user = load_config(USER_PATH);
        assert_eq!(user.len(), 2, "外部配置(V1 结构)与模板(V2)合并");
        assert_eq!(user[0].rows[0].value, Some(ConfigValue::Bool(false)), "外部值保留");
        assert_eq!(user[1].title, "面板B", "模板新面板并入");
    }

    #[test]
    fn import_config_missing_source_false() {
        let (_g, _sg) = sandbox("imp_missing");
        assert!(!import_config("nope.cfg"));
        assert!(!Path::new(USER_PATH).exists(), "源不存在时不应触碰用户配置");
    }

    #[test]
    fn import_config_empty_source_false() {
        let (_g, _sg) = sandbox("imp_empty");
        fs::write(TEMPLATE_PATH, TPL_V2).unwrap();
        fs::write("external.cfg", "not a config file (((").unwrap();
        assert!(!import_config("external.cfg"));
        assert!(!Path::new(USER_PATH).exists());
    }

    /// 钉住 Java 行为: 模板缺失时 importConfig 仍返回 true — 空模板合并结果为空,
    /// 用户配置被写成空文件 (Java 无模板存在性校验)
    #[test]
    fn import_config_missing_template_writes_empty_user() {
        let (_g, _sg) = sandbox("imp_notpl");
        fs::write("external.cfg", TPL_V1).unwrap();
        assert!(import_config("external.cfg"), "Java: 模板缺失不构成导入失败");
        assert!(Path::new(USER_PATH).exists());
        assert!(load_config(USER_PATH).is_empty(), "空模板 → 合并结果为空");
    }

    #[test]
    fn reset_to_factory_overwrites_user_with_template() {
        let (_g, _sg) = sandbox("reset_ok");
        fs::write(TEMPLATE_PATH, TPL_V2).unwrap();
        fs::write(USER_PATH, "user custom junk").unwrap();

        assert!(reset_to_factory());
        assert_eq!(fs::read(USER_PATH).unwrap(), TPL_V2.as_bytes(), "用户配置应被模板字节覆盖");
        assert_eq!(fs::read(BACKUP_PATH).unwrap(), b"user custom junk", "重置前备份");
    }

    #[test]
    fn reset_to_factory_missing_template_false() {
        let (_g, _sg) = sandbox("reset_notpl");
        fs::write(USER_PATH, "keep me").unwrap();
        assert!(!reset_to_factory());
        assert_eq!(fs::read(USER_PATH).unwrap(), b"keep me", "失败路径不动用户配置");
    }

    /// env 恢复守卫 (panic 安全): Drop 时还原被测试改写的环境变量
    struct EnvRestore(&'static str, Option<String>);
    impl Drop for EnvRestore {
        fn drop(&mut self) {
            match &self.1 {
                Some(v) => env::set_var(self.0, v),
                None => env::remove_var(self.0),
            }
        }
    }

    /// Latin-1 读面 + 他键保留 (审查 B2): Java Properties.load 按 ISO-8859-1 读
    /// (任何字节合法), 严格 UTF-8 会把高位字节打成读失败 → 误触合并/重写丢他键。
    #[test]
    fn ui_state_properties_latin1_bytes_preserve_other_keys() {
        let (_g, _sg) = sandbox("latin1");
        let file = ui_state_config_file();
        let mut bytes = b"#comment\nlastActiveMainFormTab=3\nmainFormX=100\nnote=caf".to_vec();
        bytes.push(0xE9); // Latin-1 é, 非 UTF-8 合法序列
        fs::write(&file, bytes).unwrap();

        // 读面不因编码失败 (对齐 Java), 哈希键缺失 → None
        assert_eq!(ui_state_load_template_hash(), None);

        ui_state_save_template_hash(Some("ab01"));
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.contains("templateConfigHash=ab01"), "模板哈希已写入");
        for key in ["lastActiveMainFormTab=3", "mainFormX=100"] {
            assert!(text.contains(key), "重写不得丢他键: {text}");
        }
        // 非 ASCII 值以 \uXXXX (大写, JDK toHex) 转义 — ASCII 安全, Java load 可还原
        assert!(text.contains("note=caf\\u00E9"), "Latin-1 值应转义: {text}");
        // 转义往返: 重新读回后值还原
        let reloaded = ui_state_read_properties(&file).unwrap();
        assert!(reloaded.iter().any(|(k, v)| k == "note" && v == "caf\u{e9}"));
    }

    /// 对撞 (审查 A1): Windows APPDATA 为空串时, Java 字符串拼接得 "\voidmei"
    /// (当前盘根); PathBuf::from("").join() 会折叠成相对路径 — 钉住拼接语义。
    #[test]
    #[cfg(windows)]
    fn ui_state_config_dir_empty_appdata_is_drive_rooted() {
        let _g = CWD_LOCK.lock().expect("cwd 测试锁中毒");
        set_ui_state_dir_override(None);
        let orig = env::var("APPDATA").ok();
        env::set_var("APPDATA", "");
        let _restore = EnvRestore("APPDATA", orig);

        let dir = ui_state_config_dir();
        let expected = PathBuf::from(format!("{}{UI_STATE_APP_NAME}", std::path::MAIN_SEPARATOR));
        assert_eq!(dir, expected, "空 APPDATA + File.separator + APP_NAME = 盘根路径");
        assert!(dir.has_root(), "Java new File(\"\\\\voidmei\") 为绝对路径语义");
    }

    /// 同上, Linux 侧 HOME 为空串的等价对撞 (user.home="" → "/.config/voidmei")
    #[test]
    #[cfg(target_os = "linux")]
    fn ui_state_config_dir_empty_home_is_rooted() {
        let _g = CWD_LOCK.lock().expect("cwd 测试锁中毒");
        set_ui_state_dir_override(None);
        let orig = env::var("HOME").ok();
        env::set_var("HOME", "");
        let _restore = EnvRestore("HOME", orig);

        let dir = ui_state_config_dir();
        assert_eq!(dir, PathBuf::from(format!("/.config/{UI_STATE_APP_NAME}")));
        assert!(dir.has_root());
    }
}
