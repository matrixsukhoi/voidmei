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

// ---- 弹窗 sink (ConfigDialog: web 壳形态的 showMergeReport 转发面) ----

/// sink 是进程级静态 — 触碰 sink 的测试用此锁串行; Drop 摘除恢复日志兜底路径
static SINK_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct SinkGuard;
impl Drop for SinkGuard {
    fn drop(&mut self) {
        remove_config_dialog_sink_for_test();
    }
}

/// 模板升级合并 → MergeReport 经 sink 转发一次 (标题/正文 Rust 侧 Lang 就绪);
/// 首跑 (无合并) 不弹。sink 缺席路径 = 其余 initialize 用例 (日志兜底, 不弹不 panic)
#[test]
fn initialize_merge_report_reaches_dialog_sink() {
    let _sink_guard = SinkGuard;
    let _g = SINK_TEST_LOCK.lock().expect("sink 测试锁中毒");
    let (_g, _sg) = sandbox("init_sink");
    fs::write(TEMPLATE_PATH, TPL_V1).unwrap();
    initialize(); // 首跑: 无合并 → sink 不触发
    fs::write(TEMPLATE_PATH, TPL_V2).unwrap(); // 模板升级

    let seen: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = std::sync::Arc::clone(&seen);
    set_config_dialog_sink(std::sync::Arc::new(move |d: &ConfigDialog| {
        if let ConfigDialog::MergeReport(message) = d {
            recorder.lock().unwrap().push(message.clone());
        }
        // ParseError 分支: 调用点为 Java 侧死路径 (见 show_parse_error_dialog 注),
        // 唯一活路径 showMergeReport 不产生 — 记录面只挂 MergeReport
    }));

    let cfgs = initialize();
    assert_eq!(cfgs.len(), 2, "合并路径本身不受 sink 影响");
    let dialogs = seen.lock().unwrap();
    assert_eq!(dialogs.len(), 1, "合并报告应经 sink 转发恰一次: {dialogs:?}");
    assert!(dialogs[0].contains("新增面板:"), "Lang 标题就绪: {dialogs:?}");
    assert!(dialogs[0].contains("面板B"));
    assert!(dialogs[0].contains("新增配置项:"), "新增项 k3 在列: {dialogs:?}");
}

/// 无 sink 期 (启动早期) 的合并报告: 日志兜底 + 缓存最后一条; web 就绪后
/// replay 经已装 sink 补发恰一次, 取后清空 (审查 W2 — 首启模板升级场景)
#[test]
fn merge_report_without_sink_cached_then_replayed() {
    let _sink_guard = SinkGuard;
    let _g = SINK_TEST_LOCK.lock().expect("sink 测试锁中毒");
    clear_pending_config_dialog_for_test(); // 并行用例隔离: 不捡他人残留
    let (_g, _sg) = sandbox("init_replay");
    fs::write(TEMPLATE_PATH, TPL_V1).unwrap();
    initialize(); // 首跑 (无 sink): 无合并 → 无缓存
    fs::write(TEMPLATE_PATH, TPL_V2).unwrap(); // 模板升级
    initialize(); // 合并 → 报告无 sink: 日志兜底 + 缓存

    // 组装层等价面: web 就绪后装 sink → 回放恰一次, 再回放空
    let seen: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = std::sync::Arc::clone(&seen);
    set_config_dialog_sink(std::sync::Arc::new(move |d: &ConfigDialog| {
        if let ConfigDialog::MergeReport(message) = d {
            recorder.lock().unwrap().push(message.clone());
        }
    }));
    assert!(replay_pending_config_dialog(), "缓存中有待补发弹窗");
    assert!(!replay_pending_config_dialog(), "回放取后清空");
    let dialogs = seen.lock().unwrap();
    assert_eq!(dialogs.len(), 1, "补发恰一次: {dialogs:?}");
    assert!(dialogs[0].contains("面板B"), "合并报告内容在列: {dialogs:?}");
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
