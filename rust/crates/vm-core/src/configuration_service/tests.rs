use super::*;
use crate::config_loader::ConfigValue;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

static CFG_N: AtomicUsize = AtomicUsize::new(0);

fn tmp_cfg(content: &str) -> String {
    let n = CFG_N.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir()
        .join(format!("vm_core_cfgsvc_{}_{n}.cfg", std::process::id()))
        .to_str()
        .unwrap()
        .to_string();
    fs::write(&p, content).unwrap();
    p
}

/// 无总线服务 + 从临时 cfg 装载
fn svc(content: &str) -> ConfigurationService {
    let s = ConfigurationService::new(None);
    s.load_layout(&tmp_cfg(content));
    s
}

/// 带总线服务 + 事件记录器 (返回订阅句柄保活 — EventBus RAII, Drop 即退订)
fn svc_bus(
    content: &str,
) -> (
    ConfigurationService,
    Arc<Mutex<Vec<UiStateEvent>>>,
    crate::bus::Subscription<UiStateEvent>,
) {
    let bus = Arc::new(EventBus::new());
    let log = Arc::new(Mutex::new(Vec::new()));
    let l2 = Arc::clone(&log);
    let sub = bus.subscribe(move |ev: &UiStateEvent| l2.lock().unwrap().push(ev.clone()));
    let s = ConfigurationService::new(Some(bus));
    s.load_layout(&tmp_cfg(content));
    (s, log, sub)
}

/// 仓库根真实 ui_layout.cfg 装载 (config_loader 同款路径)
fn repo_cfg_path() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../ui_layout.cfg")
        .to_str()
        .unwrap()
        .to_string()
}

/// reset/save 类用例会向 CWD 写 ./ui_layout.user.cfg (ConfigManager 路径常量
/// 相对固定) — Drop 守卫清理。注: 不 chdir 沙箱, 因跨模块 CWD 测试锁尚无
/// 共享基建 (config_manager.rs 测试注释 / 审查 B4), CWD 变更会与既有用例互扰。
struct UserCfgGuard;
impl Drop for UserCfgGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file("./ui_layout.user.cfg");
        let _ = fs::remove_file("./ui_layout.user.cfg.bak");
    }
}

// ---- getConfig 优先级 / findRowRecursive ----

#[test]
fn get_config_priority_row_groupkey_missing() {
    let s = svc(
        "(panel \"G1\" :switch-key \"g1Switch\" :visible true\n\
             \x20 (item \"s\" :type switch :target \"k1\" :value true)\n\
             \x20 (item \"d\" :type data :target \"getIAS\" :unit \"Km/h\")\n\
             \x20 (group \"Sub\" (item \"n\" :type switch :target \"k2\" :value false))\n\
             \x20 (item \"onlyLabel\" :type info :value 7)\n)\
            ",
    );
    // 命中行 target
    assert_eq!(s.get_config("k1"), Some("true".to_string()));
    // 嵌套 group 子行递归
    assert_eq!(s.get_config("k2"), Some("false".to_string()));
    // 分组 switchKey → visible
    assert_eq!(s.get_config("g1Switch"), Some("true".to_string()));
    // property 缺失时按 label 命中
    assert_eq!(s.get_config("onlyLabel"), Some("7".to_string()));
    // 未找到: Java 实现恒返回 "" (非 null)
    assert_eq!(s.get_config("missing"), Some(String::new()));
}

// ---- SWITCH_INV 反转 ----

#[test]
fn switch_inv_inversion_cycle() {
    let s = svc("(panel \"P\" (item \"i\" :type switch-inv :target \"disableX\" :value true))");
    // getConfig = String.valueOf(!getBool()): value true → "false"
    assert_eq!(s.get_config("disableX"), Some("false".to_string()));
    // isFieldDisabled = !getBool(): value true → 未禁用
    assert!(!s.is_field_disabled("disableX"));
    // 视图 getBool 同反转
    let v = s.get_overlay_settings("P");
    assert!(!v.get_bool("disableX", false));

    // setConfig("true") → 存储 !parseBoolean("true") = false
    s.set_config("disableX", "true");
    assert_eq!(s.get_config("disableX"), Some("true".to_string()));
    assert!(s.is_field_disabled("disableX"));
    assert!(v.get_bool("disableX", false));
}

// ---- setConfig 全实例更新 + 类型一致性 ----

#[test]
fn set_config_updates_all_instances_and_types() {
    let s = svc(
        "(panel \"A\" (item \"i\" :type switch :target \"k1\" :value false))\n\
             (panel \"B\" (item \"j\" :type switch :target \"k1\" :value false)\n\
             \x20 (item \"s\" :type slider :target \"k2\" :value 1))\
            ",
    );
    // 递归更新 ALL 匹配实例 (两个分组同名 key)
    s.set_config("k1", "true");
    let cfgs = s.get_layout_configs().unwrap();
    assert_eq!(cfgs[0].rows[0].value, Some(ConfigValue::Bool(true)));
    assert_eq!(cfgs[1].rows[0].value, Some(ConfigValue::Bool(true)));

    // Integer 行: 可解析 → Int; 不可解析 → 存 String (Java catch 分支)
    s.set_config("k2", "42");
    assert_eq!(s.get_config("k2"), Some("42".to_string()));
    s.set_config("k2", "abc");
    assert_eq!(s.get_config("k2"), Some("abc".to_string()));
}

#[test]
fn set_config_group_switchkey_visible_and_event() {
    let (s, log, _sub) = svc_bus(
        "(panel \"P\" :switch-key \"pSwitch\" :visible false\n\
             \x20 (item \"x\" :type switch :target \"k\" :value true))\
            ",
    );
    s.set_config("pSwitch", "true");
    assert_eq!(s.get_config("pSwitch"), Some("true".to_string()));
    assert!(s.get_layout_configs().unwrap()[0].visible);
    // parseBoolean 非 "true" 一律 false
    s.set_config("pSwitch", "whatever");
    assert_eq!(s.get_config("pSwitch"), Some("false".to_string()));
    assert_eq!(log.lock().unwrap().len(), 2);
}

#[test]
fn set_config_event_payloads_via_bus() {
    let (s, log, _sub) = svc_bus(
        "(panel \"P\" :switch-key \"pSwitch\"\n\
             \x20 (item \"x\" :type switch :target \"k\" :value true))\
            ",
    );
    s.set_config("k", "false");
    s.set_config("pSwitch", "true");
    let events = log.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        UiStateEvent {
            event_type: "configChanged".to_string(),
            source: "ConfigurationService".to_string(),
            data: "k".to_string(),
        }
    );
    assert_eq!(events[1].data, "pSwitch");
    assert_eq!(events[1].event_type, "configChanged");
}

/// initConfig/loadLayout 之前 (layoutConfigs == null): 全部读面为空、写面 no-op
#[test]
fn set_config_without_layout_noop() {
    // 单总线 + 记录器订阅保活 + 未装载布局的服务
    let bus = Arc::new(EventBus::new());
    let log = Arc::new(Mutex::new(Vec::new()));
    let l2 = Arc::clone(&log);
    let _sub = bus.subscribe(move |ev: &UiStateEvent| l2.lock().unwrap().push(ev.clone()));
    let s = ConfigurationService::new(Some(bus));
    assert_eq!(s.get_layout_configs(), None);
    assert_eq!(s.get_config("any"), Some(String::new()));
    s.set_config("any", "v"); // 无事件、无崩溃
    assert!(!s.is_field_disabled("any"));
    assert!(!s.reset_all_layout_defaults());
    assert!(log.lock().unwrap().is_empty());
}

// ---- isFieldDisabled ----

#[test]
fn is_field_disabled_matrix() {
    let s = svc(
        "(panel \"P\"\n\
             \x20 (item \"inv1\" :type switch-inv :target \"disableX\" :value true)\n\
             \x20 (item \"inv2\" :type switch-inv :target \"disableY\" :value false)\n\
             \x20 (item \"sw\" :type switch :target \"boolFalse\" :value false)\n\
             \x20 (item \"sw2\" :type switch :target \"boolTrue\" :value true)\n\
             \x20 (item \"sl\" :type slider :target \"intRow\" :value 5)\n)\
            ",
    );
    assert!(!s.is_field_disabled(""));
    // SWITCH_INV: !getBool
    assert!(!s.is_field_disabled("disableX")); // value true → !true
    assert!(s.is_field_disabled("disableY")); // value false → !false
    // Boolean 值: false → 禁用
    assert!(s.is_field_disabled("boolFalse"));
    assert!(!s.is_field_disabled("boolTrue"));
    // 非 Boolean 值: 不返回, 继续扫描 → false
    assert!(!s.is_field_disabled("intRow"));
    assert!(!s.is_field_disabled("never-set"));
}

// ---- resetAllLayoutDefaults 三阶段 ----

#[test]
fn reset_all_layout_defaults_phases_and_notify() {
    let _guard = UserCfgGuard;
    let (s, log, _sub) = svc_bus(
        "(panel \"P\" :switch-key \"pSwitch\" :visible false\n\
             \x20 (group \"G\"\n\
             \x20   (item \"a\" :type switch :target \"k1\" :value true :default false)\n\
             \x20   (item \"b\" :type slider :target \"k2\" :value 9 :default 7)\n\
             \x20   (item \"c\" :type switch :target \"k3\" :value true :default true)\n\
             \x20   (item \"d\" :type combo :target \"kc\" :default \"A\"))\n)\
            ",
    );
    // value null + default "A": Java defaultValue.equals(null)=false → 仍收集
    assert_eq!(s.get_config("kc"), Some("null".to_string())); // getStr 对 null → "null"

    s.set_config("k1", "true"); // 先产生一条普通变更事件
    assert!(s.reset_all_layout_defaults());

    // Phase 2: 偏离 default 的行回写 default; 相同的 k3 不动
    assert_eq!(s.get_config("k1"), Some("false".to_string()));
    assert_eq!(s.get_config("k2"), Some("7".to_string()));
    assert_eq!(s.get_config("k3"), Some("true".to_string()));
    assert_eq!(s.get_config("kc"), Some("A".to_string()));
    // 分组 visible 非 RowConfig, 不在重置范围
    assert_eq!(s.get_config("pSwitch"), Some("false".to_string()));
    // Phase 3: 恰一条 RESET_COMPLETED, 顺序在 set 事件之后
    let events = log.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].data, "k1");
    assert_eq!(events[1].data, "RESET_COMPLETED");
    assert_eq!(events[1].event_type, "configChanged");
}

#[test]
fn reset_all_layout_defaults_no_change() {
    let (s, log, _sub) = svc_bus(
        "(panel \"P\" (item \"a\" :type switch :target \"k1\" :value true :default true))",
    );
    assert!(!s.reset_all_layout_defaults());
    assert!(log.lock().unwrap().is_empty());
}

/// 失败路径: 源缺失 / 模板缺失 (crate CWD 无 ./ui_layout.cfg) → false 且不发事件。
/// CWD 锁 (config_manager::CWD_LOCK): reset_to_factory 的模板存在性检查走全局
/// ./ui_layout.cfg — 与 config_manager 的 chdir 型沙箱测试并行时, 进程 CWD 可能
/// 已被切进沙箱 (沙箱内有模板) → reset 误判 true 打挂断言 (既有竞态, 本批新增
/// 沙箱用例加大暴露窗口后实测复现; group_position_read_write_roundtrip 同款锁)
#[test]
fn import_reset_failure_paths() {
    let _cwd = crate::config_manager::CWD_LOCK.lock().expect("cwd 测试锁中毒");
    let _guard = UserCfgGuard;
    let (s, log, _sub) = svc_bus("(panel \"P\")");
    assert!(!s.import_config("definitely_missing_zzz.cfg"));
    assert!(!s.reset_to_factory());
    assert!(log.lock().unwrap().is_empty());
    assert_eq!(s.get_config("k"), Some(String::new()));
}

// ---- 组装层位置桥 (group_position / save_group_position) ----

#[test]
fn group_position_read_write_roundtrip() {
    // 跨模块 CWD 锁 (config_manager::CWD_LOCK, 审查 B4): 落盘走全局路径
    // ./ui_layout.user.cfg, 须与 config_manager 的 chdir 型沙箱测试互斥 —
    // 并行时本测试的落盘会写进他人沙箱 (实测曾打挂 reset_to_factory 断言)
    let _cwd = crate::config_manager::CWD_LOCK.lock().expect("cwd 测试锁中毒");
    let _guard = UserCfgGuard; // save_group_position 落盘 → Drop 清理 ./ui_layout.user.cfg
    let s = svc("(panel \"飞行信息\" :x 0.0602 :y 0.1188)");
    // 读: 归一化原值 (忽略大小写, 对齐视图 getGroupConfig 语义)
    assert_eq!(s.group_position("飞行信息"), Some((0.0602, 0.1188)));
    assert_eq!(s.group_position("FLIGHT 信息"), None); // 未命中 → None (host 居中兜底)
    // 写: 归一化直写 + 回读一致 (落盘副作用同 save_window_position 测试先例)
    assert!(s.save_group_position("飞行信息", 0.25, 0.75));
    assert_eq!(s.group_position("飞行信息"), Some((0.25, 0.75)));
    // 未命中写: false 不 panic (Java warn 分支)
    assert!(!s.save_group_position("不存在", 0.1, 0.1));
    // 与视图像素面一致性: 写归一化后 get_window_x 跟随 (同源字段)
    s.set_screen_size(1920, 1080);
    assert_eq!(s.get_overlay_settings("飞行信息").get_window_x(0), 480); // round(0.25*1920)
}

// ---- findGroupByTitle (大小写敏感) vs 视图 getGroupConfig (忽略大小写) ----

#[test]
fn find_group_by_title_case_sensitive() {
    let s = svc("(panel \"Alpha\" :x 0.5 :y 0.25)\n(panel \"beta\")");
    assert!(s.find_group_by_title("Alpha").is_some());
    assert!(s.find_group_by_title("alpha").is_none()); // equals 大小写敏感
    assert!(s.find_group_by_title("").is_none());
    assert!((s.find_group_by_title("Alpha").unwrap().x - 0.5).abs() < 1e-12);

    // 视图查找忽略大小写
    s.set_screen_size(100, 100);
    let v = s.get_overlay_settings("ALPHA");
    assert_eq!(v.get_window_x(0), 50); // round(0.5*100)
    assert_eq!(v.get_window_y(0), 25);
}

// ---- 真实 ui_layout.cfg: 飞行信息视图 (任务必测项) ----

#[test]
fn overlay_settings_flight_info_repo() {
    let s = ConfigurationService::new(None);
    s.load_layout(&repo_cfg_path());
    s.set_screen_size(1920, 1080);
    let v = s.get_overlay_settings("飞行信息");

    // gc.x=0.0602 * 1920 = 115.584 → Math.round → 116
    assert_eq!(v.get_window_x(300), 116);
    // gc.y=0.1188 * 1080 = 128.304 → 128
    assert_eq!(v.get_window_y(300), 128);

    // :font "Sarasa Mono SC" (面板级字体)
    assert_eq!(v.get_font_name(), "Sarasa Mono SC");
    // 无 :font-size 面板属性 → 0 ("大小"滑条是行级 fontSize, 不影响 getFontSizeAdd)
    assert_eq!(v.get_font_size_add(), 0);

    // 组内行读取
    assert!(v.get_bool("flightInfoSwitch", false));
    assert!(!v.get_bool("flightInfoEdge", true));
    assert_eq!(v.get_int("flightInfoColumn", 0), 1);
    assert_eq!(v.get_int("fontSize", -99), 0);

    // trait 借出面
    assert_eq!(v.get_group_config().map(|g| g.title.as_str()), Some("飞行信息"));
}

/// 全局五色读链: cfg 五键 (ui_layout.cfg:379-383) 经 load_app_check 覆盖
/// Application 静态初值 — global_colors() 应返回模板色而非 Java 默认。
/// (人工验收: 组件曾用编译期 Java 初始值, 用户 cfg 改色后 Rust 不跟随)
#[test]
fn global_colors_read_repo_template_cfg() {
    let s = ConfigurationService::new(None);
    s.load_layout(&repo_cfg_path());
    let mut c = ControllerIntervals::default();
    s.load_app_check(&mut c);
    let g = s.global_colors();
    // 模板色 (hex #RRGGBBAA 直读)
    assert_eq!(g.num, [255, 255, 255, 255], "fontNum #FFFFFFFF");
    assert_eq!(g.label, [255, 255, 255, 255], "fontLabel #FFFFFFFF");
    assert_eq!(g.unit, [232, 147, 50, 255], "fontUnit #E89332FF");
    assert_eq!(g.warning, [255, 36, 0, 255], "fontWarn #FF2400FF");
    assert_eq!(g.shade_shape, [0, 0, 0, 255], "fontShade #000000FF");
    assert_ne!(g, GlobalColors::JAVA_DEFAULT, "运行时值已覆盖静态初始值");
}

// ---- 视图回退分支 ----

#[test]
fn overlay_center_fallback_and_guard_branches() {
    let s = svc("(panel \"A\" :x 0.9 :y 0.9)");
    s.set_screen_size(1920, 1080);
    // 分组不存在 → 居中回退 (int 除法)
    let v = s.get_overlay_settings("Zzz");
    assert_eq!(v.get_window_x(400), (1920 - 400) / 2);
    assert_eq!(v.get_window_y(300), (1080 - 300) / 2);
    // gc=null 分支 saveWindowPosition → warn (无保存、无崩溃)
    v.save_window_position(10.0, 10.0);

    // 分组存在但 screen<=0 → Java gc!=null && sw>0 && sh>0 为假 → warn
    let s2 = svc("(panel \"A\" :x 0.9 :y 0.9)");
    s2.set_screen_size(0, 0);
    s2.get_overlay_settings("A").save_window_position(10.0, 10.0);

    // screen=0 的读面: round(0.9*0)=0 (Java 同)
    assert_eq!(s2.get_overlay_settings("A").get_window_x(50), 0);
}

// ---- 真实 ui_layout.cfg: MiniHUD HUDSettings ----

#[test]
fn hud_settings_repo_minihud() {
    let s = ConfigurationService::new(None);
    s.load_layout(&repo_cfg_path());
    s.set_screen_size(1920, 1080);
    let h = s.get_hud_settings();

    // gc.x=0.3891*1920=747.072 → 747; gc.y=0.7042*1080=760.536 → 761
    assert_eq!(h.get_window_x(400), 747);
    assert_eq!(h.get_window_y(400), 761);

    assert_eq!(h.get_crosshair_scale(), 113); // :value 113
    assert_eq!(h.get_crosshair_name(), "软件渲染准星");
    assert!(h.is_display_crosshair()); // displayCrosshair=true
    assert!(!h.use_texture_crosshair()); // "软件渲染准星" → false
    assert!(h.draw_hud_text()); // drawHUDtext=true
    assert!(h.show_attitude_gauge()); // 默认 true (cfg 无该键)

    // AoA 比率: 布局优先 (Int 20 → 20>1 → 0.2; Int 25 → 0.25)
    assert!((h.get_aoa_warning_ratio() - 0.2).abs() < 1e-12);
    assert!((h.get_aoa_bar_warning_ratio() - 0.25).abs() < 1e-12);

    // 字体链: MonoNumFont = "Sarasa Mono SC"
    assert_eq!(h.get_num_font(), "Sarasa Mono SC");
    assert_eq!(h.get_num_font_name(), "Sarasa Mono SC");
    // getFontSizeAdd 走父类 (MiniHUD 无 :font-size → 0)
    assert_eq!(h.get_font_size_add(), 0);
}

// ---- HUDSettings 准星回退分支 ----

#[test]
fn hud_crosshair_fallback_writeback() {
    let s = svc(
        "(panel \"Other\"\n\
             \x20 (item \"cx\" :type slider :target \"crosshairX\" :value 500)\n\
             \x20 (item \"cy\" :type slider :target \"crosshairY\" :value 300))\
            ",
    );
    s.set_screen_size(1920, 1080);
    let h = s.get_hud_settings();
    // 无 "MiniHUD" 分组 → crosshairX/Y 行优先, 缺省才用居中默认
    assert_eq!(h.get_window_x(400), 500);
    assert_eq!(h.get_window_y(300), 300);

    // 位置写回走 setConfig((int)x)
    h.save_window_position(123.7, -45.9);
    assert_eq!(s.get_config("crosshairX"), Some("123".to_string()));
    assert_eq!(s.get_config("crosshairY"), Some("-45".to_string()));

    // 行也不存在 → 居中默认 (int 除法)
    let s2 = svc("(panel \"Other\")");
    s2.set_screen_size(1920, 1080);
    let h2 = s2.get_hud_settings();
    assert_eq!(h2.get_window_x(400), (1920 - 400) / 2);
    assert_eq!(h2.get_window_y(300), (1080 - 300) / 2);
}

// ---- AoA 比率归一 + getDoubleFromLayoutFirst ----

#[test]
fn hud_aoa_ratio_and_layout_first() {
    // 字符串值: parse 成功; Int 值: Number.doubleValue; >1 归一 /100
    let s = svc(
        "(panel \"MiniHUD\"\n\
             \x20 (group \"G\"\n\
             \x20   (item \"a\" :type info :target \"miniHUDaoaWarningRatio\" :value \"0.5\")\n\
             \x20   (item \"b\" :type info :target \"miniHUDaoaBarWarningRatio\" :value 100)\n\
             \x20   (item \"c\" :type info :target \"extraKey\" :value 2.5)\n\
             \x20   (item \"d\" :type info :target \"edgeKey\" :value 1)\n)\
            ",
    );
    let h = s.get_hud_settings();
    assert!((h.get_aoa_warning_ratio() - 0.5).abs() < 1e-12); // 0.5 ≤ 1 → 原值
    assert!((h.get_aoa_bar_warning_ratio() - 1.0).abs() < 1e-12); // 100 → 1.0
    assert!((h.get_double_from_layout_first("MiniHUD", "extraKey", 9.0) - 2.5).abs() < 1e-12); // Double 分支
    assert!((h.get_double_from_layout_first("MiniHUD", "edgeKey", 9.0) - 1.0).abs() < 1e-12); // Int 分支

    // 不可解析字符串 → catch ignore → Priority 2 全局 getDouble → 默认 25 → 0.25
    let s2 = svc(
        "(panel \"MiniHUD\" (group \"G\"\n\
             \x20 (item \"a\" :type info :target \"miniHUDaoaWarningRatio\" :value \"abc\")))\
            ",
    );
    assert!((s2.get_hud_settings().get_aoa_warning_ratio() - 0.25).abs() < 1e-12);

    // 段名不匹配 → Priority 1 落空; Priority 2 的 getDouble→getConfig 是
    // 全局行查找 (不限段), 命中 Other 面板的同 target 行 → 90 → 0.9 (Java 同流)
    let s3 = svc("(panel \"Other\" (item \"a\" :type info :target \"miniHUDaoaWarningRatio\" :value 90))");
    assert!((s3.get_hud_settings().get_aoa_warning_ratio() - 0.9).abs() < 1e-12);
}

// ---- loadAppCheck: 真实 ui_layout.cfg 全同步链 (任务必测项) ----

#[test]
fn load_app_check_repo_full_sync() {
    let s = ConfigurationService::new(None);
    s.load_layout(&repo_cfg_path());
    let mut c = ControllerIntervals::default();
    s.load_app_check(&mut c);
    let app = s.application_state();

    // dataPollIntervalMs=80: 2f/1.5f/1f 系数与 /3 整除
    assert_eq!(c.service_loop_interval_ms, 80);
    assert_eq!(c.engine_info_interval_ms, 160);
    assert_eq!(c.flight_info_interval_ms, 120);
    assert_eq!(c.altitude_interval_ms, 120);
    assert_eq!(c.gear_flaps_interval_ms, 160);
    assert_eq!(c.control_input_interval_ms, 80);
    assert_eq!(app.thread_sleep_time, 26); // 80/3 整除

    // 颜色 (#RRGGBBAA 语义: fontUnit #E89332FF → R=E8 G=93 B=32 A=FF)
    assert_eq!(app.color_num, [255, 255, 255, 255]);
    assert_eq!(app.color_label, [255, 255, 255, 255]);
    assert_eq!(app.color_unit, [232, 147, 50, 255]);
    assert_eq!(app.color_warning, [255, 36, 0, 255]);
    assert_eq!(app.color_shade_shape, [0, 0, 0, 255]);

    // voiceVolume=100 / AAEnable=true → On/On (默认链路: 值在 → parseBoolean)
    assert_eq!(app.voice_volumn, 100);
    assert!(app.aa_enable);
    assert_eq!(app.text_aa_setting, TextAaSetting::On);
    assert_eq!(app.graph_aa_setting, GraphAaSetting::On);

    // displayFmKey=25 (VC_P)
    assert_eq!(app.display_fm_key, 25);

    // HTTP 端口: 8111 / +1111; Lang.httpIp 默认 127.0.0.1
    assert_eq!(app.app_port, 8111);
    assert_eq!(app.app_port_bkp, 9222);
    assert_eq!(
        app.request_dest,
        Some(InetSocketAddress::new("127.0.0.1", 8111))
    );
    assert_eq!(
        app.request_dest_bkp,
        Some(InetSocketAddress::new("127.0.0.1", 9222))
    );

    // 全局字体同步
    assert_eq!(app.default_numfont_name, "Sarasa Mono SC");
    assert_eq!(app.default_font_name, "Sarasa Mono SC");
}

// ---- loadAppCheck: 键缺失时的默认链 ----

#[test]
fn load_app_check_missing_keys_defaults() {
    let s = svc("(panel \"P\" (item \"s\" :type switch :target \"someKey\" :value true))");
    let mut c = ControllerIntervals::default();
    s.load_app_check(&mut c);
    let app = s.application_state();

    // 间隔默认 50
    assert_eq!(c.service_loop_interval_ms, 50);
    assert_eq!(c.engine_info_interval_ms, 100);
    assert_eq!(c.flight_info_interval_ms, 75);
    assert_eq!(c.altitude_interval_ms, 75);
    assert_eq!(c.gear_flaps_interval_ms, 100);
    assert_eq!(c.control_input_interval_ms, 50);
    assert_eq!(app.thread_sleep_time, 16); // 50/3

    // 颜色缺失 → ColorHelper 默认 Color.WHITE
    assert_eq!(app.color_num, [255, 255, 255, 255]);
    assert_eq!(app.color_warning, [255, 255, 255, 255]);

    // voiceVolume 缺失 → 不触碰声明默认 100
    assert_eq!(app.voice_volumn, 100);

    // AAEnable 缺失 → false (else 分支) → Off
    assert!(!app.aa_enable);
    assert_eq!(app.text_aa_setting, TextAaSetting::Off);
    assert_eq!(app.graph_aa_setting, GraphAaSetting::Off);

    // 端口缺失 → 不同步
    assert_eq!(app.app_port, 0);
    assert_eq!(app.request_dest, None);

    // 字体缺失 → 声明默认
    assert_eq!(app.default_numfont_name, "Roboto");
    assert_eq!(app.default_font_name, "Microsoft YaHei UI");
}

// ---- loadAppCheck: 非法值恢复 ----

#[test]
fn load_app_check_bad_values_recovery() {
    let s = svc(
        "(panel \"P\"\n\
             \x20 (item \"i\" :type slider :target \"Interval\" :value 100)\n\
             \x20 (item \"v\" :type slider :target \"voiceVolume\" :value 0)\n\
             \x20 (item \"p\" :type data :target \"httpPort\" :value \"abc\")\n\
             \x20 (item \"a\" :type switch :target \"AAEnable\" :value false))\
            ",
    );
    let mut c = ControllerIntervals::default();
    s.load_app_check(&mut c);
    let app = s.application_state();

    // dataPollIntervalMs 缺失 → legacy "Interval"=100
    assert_eq!(c.service_loop_interval_ms, 100);
    assert_eq!(c.engine_info_interval_ms, 200);
    assert_eq!(c.flight_info_interval_ms, 150);
    assert_eq!(app.thread_sleep_time, 33);

    // voiceVolume="0" → 0 (合法解析)
    assert_eq!(app.voice_volumn, 0);
    // httpPort="abc" → NumberFormatException → Ignore (端口面不动)
    assert_eq!(app.app_port, 0);
    assert_eq!(app.request_dest, None);
    // AAEnable=false → Off
    assert!(!app.aa_enable);
    assert_eq!(app.text_aa_setting, TextAaSetting::Off);

    // 新键非数值 → NumberFormatException → 50 (legacy 键不再回看)
    let s2 = svc(
        "(panel \"P\"\n\
             \x20 (item \"n\" :type data :target \"dataPollIntervalMs\" :value \"xyz\")\n\
             \x20 (item \"i\" :type slider :target \"Interval\" :value 300))\
            ",
    );
    let mut c2 = ControllerIntervals::default();
    s2.load_app_check(&mut c2);
    assert_eq!(c2.service_loop_interval_ms, 50);
}

// ---- ColorHelper 消费面 ----

#[test]
fn color_parse_matrix() {
    let w = [255, 255, 255, 255];
    // hex
    assert_eq!(parse_color("#FF5500", w), [255, 85, 0, 255]); // RGB → alpha 255
    assert_eq!(parse_color("#FF5500AA", w), [255, 85, 0, 170]);
    assert_eq!(parse_color("#ff5500aa", w), [255, 85, 0, 170]); // 小写 hex
    // decimal
    assert_eq!(parse_color("255, 85, 0, 170", w), [255, 85, 0, 170]);
    assert_eq!(parse_color("255, 85, 0", w), [255, 85, 0, 255]);
    // clamp
    assert_eq!(parse_color(" 300 , -5 , 0 , 999 ", w), [255, 0, 0, 255]);
    // Java split 尾部空串剔除: "1,2,3," → 3 段
    assert_eq!(parse_color("1,2,3,", w), [1, 2, 3, 255]);
    // 失败 → 默认
    assert_eq!(parse_color("1,2", w), w);
    assert_eq!(parse_color("#GGGGGG", w), w);
    assert_eq!(parse_color("#12345", w), w); // 长度非 6/8
    assert_eq!(parse_color("1,2,c,4", w), w);
    assert_eq!(parse_color("", w), w);
    assert_eq!(parse_color("   ", w), w);
}

#[test]
fn set_color_config_roundtrip() {
    let s = svc("(panel \"P\" (item \"c\" :type color :target \"fontNum\" :value \"#FFFFFFFF\"))");
    // 原值 hex 可读
    assert_eq!(s.get_color_config("fontNum"), [255, 255, 255, 255]);
    // 写回十进制 "R, G, B, A" 格式 (Java: R + ", " + G + ...)
    s.set_color_config("fontNum", [1, 2, 3, 4]);
    assert_eq!(s.get_config("fontNum"), Some("1, 2, 3, 4".to_string()));
    assert_eq!(s.get_color_config("fontNum"), [1, 2, 3, 4]);
}

// ---- trait 对象分发 ----

#[test]
fn dyn_trait_dispatch() {
    let s = svc(
        "(panel \"MiniHUD\" :x 0.5 :y 0.5\n\
             \x20 (item \"sw\" :type switch :target \"showSpeedBar\" :value false))\
            ",
    );
    s.set_screen_size(100, 100);

    // Box<dyn ConfigProvider> (面向接口编程)
    let p: Box<dyn ConfigProvider> = Box::new(s.clone());
    p.set_config("showSpeedBar", "true");
    assert_eq!(p.get_config("showSpeedBar"), Some("true".to_string()));
    assert!(!p.is_field_disabled("showSpeedBar"));

    // Box<dyn HUDSettings> + 上转 &dyn OverlaySettings (单 vtable 槽)
    let h: Box<dyn HUDSettings<GroupConfig = GroupConfig>> = Box::new(s.get_hud_settings());
    assert!(h.show_speed_bar(), "setConfig 后视图重查应见新值");
    assert_eq!(h.get_window_x(0), 50);
    let base: &dyn OverlaySettings<GroupConfig = GroupConfig> = &*h;
    assert_eq!(base.get_window_x(0), 50);
    assert_eq!(base.get_font_size_add(), 0);
}

// ---- saveConfig 空方法 / saveLayoutConfig 空配置守卫 ----

#[test]
fn save_config_noop_and_empty_layout_guard() {
    let s = ConfigurationService::new(None);
    s.save_config();
    s.save_layout_config(); // layoutConfigs == null → 无落盘/无日志副作用, 不崩溃
}

// ---- 修复波次: InetSocketAddress 桩抛出面 / Double.equals 位级 / toString ----

#[test]
fn inet_socket_address_port_bounds() {
    assert_eq!(InetSocketAddress::new("host", 0).port, 0);
    let b = InetSocketAddress::new("host", 65535);
    assert_eq!((b.host.as_str(), b.port), ("host", 65535));
}

/// JDK: port 越界 → IllegalArgumentException (非 NumberFormatException,
/// 不被 loadAppCheck 的 catch 捕获) — Rust panic! 复刻
#[test]
#[should_panic(expected = "port out of range")]
fn inet_socket_address_negative_port_panics() {
    let _ = InetSocketAddress::new("host", -1);
}

#[test]
#[should_panic(expected = "port out of range")]
fn inet_socket_address_overflow_port_panics() {
    let _ = InetSocketAddress::new("host", 65536);
}

/// Double.toString 本地副本 — 期望值来自 Java 8 oracle 逐字面量对拍
/// (config_loader 同款 battery; 科学计数域 0.0001/1e7 为修复覆盖点)
#[test]
fn java_double_to_string_matches_java8_oracle() {
    let cases = [
        (0.03125, "0.03125"),
        (1.5, "1.5"),
        (1.0e7, "1.0E7"),
        (9999999.0, "9999999.0"),
        (0.001, "0.001"),
        (0.0001, "1.0E-4"),
        (1.0e-5, "1.0E-5"),
        (123456789012345.6, "1.234567890123456E14"),
        (0.0, "0.0"),
        (1.0000000000000002, "1.0000000000000002"),
        (0.002, "0.002"),
    ];
    for (d, want) in cases {
        assert_eq!(java_double_to_string(d), want, "{d} → 期望 {want}");
    }
    assert_eq!(java_double_to_string(-0.0), "-0.0");
    assert_eq!(java_double_to_string(f64::NAN), "NaN");
    assert_eq!(java_double_to_string(f64::INFINITY), "Infinity");
    assert_eq!(java_double_to_string(f64::NEG_INFINITY), "-Infinity");
    // ConfigValue 面 (reset 日志的 default 回显文本)
    assert_eq!(config_value_to_java_string(&ConfigValue::Double(1.0e7)), "1.0E7");
    assert_eq!(config_value_to_java_string(&ConfigValue::Double(20.0)), "20.0");
}

/// Double.equals = doubleToLongBits 位级: NaN==NaN true、+0.0!=-0.0;
/// 异型 instanceof 恒 false — 均与派生 PartialEq 相反 (Java 语义钉子)
#[test]
fn config_value_java_equals_double_bits() {
    assert!(config_value_java_equals(
        &ConfigValue::Double(f64::NAN),
        &ConfigValue::Double(f64::NAN)
    ));
    assert!(!config_value_java_equals(&ConfigValue::Double(0.0), &ConfigValue::Double(-0.0)));
    assert!(!config_value_java_equals(&ConfigValue::Double(-0.0), &ConfigValue::Double(0.0)));
    assert!(config_value_java_equals(&ConfigValue::Double(2.5), &ConfigValue::Double(2.5)));
    assert!(!config_value_java_equals(&ConfigValue::Double(1.0), &ConfigValue::Int(1)));
    assert!(config_value_java_equals(&ConfigValue::Int(1), &ConfigValue::Int(1)));
    assert!(config_value_java_equals(&ConfigValue::Bool(true), &ConfigValue::Bool(true)));
    assert!(!config_value_java_equals(
        &ConfigValue::Str("a".to_string()),
        &ConfigValue::Str("b".to_string())
    ));
}

/// 收集判定端到端: value=-0.0 / default=+0.0 → Java equals 不等 → 收集
/// (派生 PartialEq 会判等漏收)。cfg 文本无法到达该位形 (解析器 -0.0 折叠
/// Int(0)), 故直接构造 RowConfig。
#[test]
fn reset_collect_negative_zero_double() {
    let mut r = RowConfig::new("z".to_string(), None, String::new());
    r.value = Some(ConfigValue::Double(-0.0));
    r.default_value = Some(ConfigValue::Double(0.0));
    let mut pending = Vec::new();
    let mut path = Vec::new();
    collect_reset_candidates_recursive(&[r], 0, &mut path, &mut pending);
    assert_eq!(pending.len(), 1);

    // NaN 值 == NaN 默认 → Java equals 相等 → 不收集
    let mut r2 = RowConfig::new("n".to_string(), None, String::new());
    r2.value = Some(ConfigValue::Double(f64::NAN));
    r2.default_value = Some(ConfigValue::Double(f64::NAN));
    let mut pending2 = Vec::new();
    let mut path2 = Vec::new();
    collect_reset_candidates_recursive(&[r2], 0, &mut path2, &mut pending2);
    assert!(pending2.is_empty());
}
