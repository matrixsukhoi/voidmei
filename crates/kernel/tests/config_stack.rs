//! 配置栈黑盒场景 (kernel::config): ConfigurationService (出厂 ⊕ 用户 delta
//! 合成 / 行值读写 / CONFIG_CHANGED 广播 / 恢复出厂 / 颜色通道) +
//! json_store (升级跟随 / 落盘往返 / 损坏隔离)。
//! 树注入走 pub 的 install_for_test; 临时文件 = temp_dir + pid + 计数。
#![allow(non_snake_case)] // 测试名 = 模块前缀英文 + 中文场景 (风格约定, 豁免蛇形)

use std::sync::{Arc, Mutex};

use expect_test::expect;
use kernel::base::bus::ui_state_bus::UIStateBus;
use kernel::base::event::ui_state_events;
use kernel::config::config_api::ConfigProvider;
use kernel::config::configuration_service::ConfigurationService;
use kernel::config::json_model::{ConfigValue, GroupConfig, RowConfig};
use kernel::config::json_store::{self, PanelDelta, UserDelta};

// ---- 夹具 ----

static CFG_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// 每测试独立临时 delta 文件 (temp_dir + pid + 计数, 并行测试互不串扰)
fn tmp_cfg(tag: &str) -> String {
    let n = CFG_N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir()
        .join(format!("vm_kcfg_{tag}_{}_{n}.json", std::process::id()))
        .to_str()
        .unwrap()
        .to_string()
}

fn trow(label: &str, ty: &str, value: Option<ConfigValue>) -> RowConfig {
    RowConfig {
        label: label.to_string(),
        r#type: ty.to_string(),
        property: Some(label.to_string()),
        value: value.clone(),
        default_value: value,
        ..RowConfig::default()
    }
}

fn tpanel(title: &str, rows: Vec<RowConfig>) -> GroupConfig {
    GroupConfig {
        title: title.to_string(),
        visible: true,
        rows,
        ..GroupConfig::default()
    }
}

/// 合成树: 开关 / 反转开关 / 整数滑杆 / 颜色键 / 嵌套子行
fn test_panels() -> Vec<GroupConfig> {
    let nested = RowConfig {
        label: "nestedData".into(),
        r#type: "DATA".into(),
        property: Some("nestedData".into()),
        value: Some(ConfigValue::Str("inner".into())),
        ..RowConfig::default()
    };
    let mut parent = trow("parentHeader", "HEADER", None);
    parent.children = vec![nested];
    vec![tpanel(
        "T",
        vec![
            trow("crosshairSwitch", "SWITCH", Some(ConfigValue::Bool(true))),
            trow("disableSimple", "SWITCH_INV", Some(ConfigValue::Bool(false))),
            trow("dataPollIntervalMs", "SLIDER", Some(ConfigValue::Int(50))),
            trow("fontNum", "COLOR", Some(ConfigValue::Str("#FF0000".into()))),
            parent,
        ],
    )]
}

/// 运行树摘要: panel 标题 + 行 (label: value)
fn digest(configs: &[GroupConfig]) -> String {
    fn rows(rs: &[RowConfig], depth: usize) -> String {
        rs.iter()
            .map(|r| {
                let v = r.value.as_ref().map(|v| v.as_config_string()).unwrap_or_default();
                let mut s = format!("{}{}: {}", "  ".repeat(depth + 1), r.label, v);
                if !r.children.is_empty() {
                    s.push('\n');
                    s.push_str(&rows(&r.children, depth + 1));
                }
                s
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    configs
        .iter()
        .map(|g| format!("{}:\n{}", g.title, rows(&g.rows, 0)))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---- 场景 ----

/// 出厂默认面板装载: install_for_test 注入后运行树/读值/未装载形态 (expect 表)
#[test]
fn 配置栈_出厂面板装载与读值() {
    let svc = ConfigurationService::new(None);
    assert_eq!(svc.get_layout_configs(), None, "未装载 (install 前) = Java null 对位");

    svc.install_for_test(test_panels(), &tmp_cfg("base"));
    let cfgs = svc.get_layout_configs().expect("装载后 Some");
    expect![[r#"
        T:
          crosshairSwitch: true
          disableSimple: false
          dataPollIntervalMs: 50
          fontNum: #FF0000
          parentHeader: 
            nestedData: inner"#]]
    .assert_eq(&digest(&cfgs));

    // 读值: ConfigProvider::get_config (行值通道) + SWITCH_INV 反转语义
    let actual = [
        svc.get_config("crosshairSwitch").unwrap(),
        // SWITCH_INV: 存 false, 读时反转 → true (disableXXX 键族)
        svc.get_config("disableSimple").unwrap(),
        svc.get_config("dataPollIntervalMs").unwrap(),
        svc.get_config("fontNum").unwrap(),
        // 嵌套子行也按 property 递归命中
        svc.get_config("nestedData").unwrap(),
        svc.get_config("unknownKey").unwrap(),
    ]
    .join("|");
    expect!["true|true|50|#FF0000|inner|"].assert_eq(&actual);

    // is_field_disabled: SWITCH false / SWITCH_INV 语义 (读 true 即禁用)
    assert!(!svc.is_field_disabled("crosshairSwitch"));
    assert!(svc.is_field_disabled("disableSimple"), "SWITCH_INV 读 true → 禁用");
}

/// set_config 写树 + 类型保型 (Int 行解析回 Int, 非法串落 Str) + SWITCH_INV 反写
#[test]
fn 配置栈_set往返与类型保型() {
    let svc = ConfigurationService::new(None);
    svc.install_for_test(test_panels(), &tmp_cfg("set"));

    svc.set_config("dataPollIntervalMs", "200");
    assert_eq!(svc.get_config("dataPollIntervalMs").as_deref(), Some("200"));
    // 类型保型: SLIDER 原值 Int → parse 成功保持 Int
    let row = svc
        .get_layout_configs()
        .unwrap()
        .iter()
        .flat_map(|g| g.rows.iter())
        .find(|r| r.label == "dataPollIntervalMs")
        .unwrap()
        .clone();
    assert_eq!(row.value, Some(ConfigValue::Int(200)));

    // 非法数值串: Int 行 parse 失败 → 降为 Str (Java else 分支)
    svc.set_config("dataPollIntervalMs", "fast");
    assert_eq!(svc.get_config("dataPollIntervalMs").as_deref(), Some("fast"));

    // SWITCH_INV 反写: set true 存 !true = false, 读时再反转 → true
    svc.set_config("disableSimple", "true");
    assert_eq!(svc.get_config("disableSimple").as_deref(), Some("true"));

    // 未知键: 无命中, 静默无效 (调用方回落行值通道的边界)
    svc.set_config("ghostKey", "1");
    assert_eq!(svc.get_config("ghostKey").as_deref(), Some(""));
}

/// set 后 CONFIG_CHANGED 广播: ui_bus 订阅计数 + payload = 改动的键
#[test]
fn 配置栈_set后CONFIG_CHANGED广播() {
    let bus = Arc::new(UIStateBus::new());
    let svc = ConfigurationService::new(Some(Arc::clone(&bus)));
    svc.install_for_test(test_panels(), &tmp_cfg("bus"));

    let hits = Arc::new(Mutex::new(Vec::<String>::new()));
    let h = Arc::clone(&hits);
    let _sub = bus.subscribe(ui_state_events::CONFIG_CHANGED, move |msg| {
        h.lock().unwrap().push(msg.data.clone().unwrap_or_default());
    });

    svc.set_config("crosshairSwitch", "false");
    svc.set_config("dataPollIntervalMs", "100");
    svc.set_config("ghostKey", "x"); // 无命中 → 不广播

    assert_eq!(
        *hits.lock().unwrap(),
        vec!["crosshairSwitch".to_string(), "dataPollIntervalMs".to_string()],
        "每个命中键一条 CONFIG_CHANGED, 未命中键静默"
    );
    assert_eq!(bus.subscriber_count(ui_state_events::CONFIG_CHANGED), 1);
}

/// 升级跟随 (json_store::synthesize): 旧版 delta 合成新基树 — 用户改过的键保持,
/// 新版新增键补出厂默认, delta 引用已删面板被丢弃
#[test]
fn 配置栈_升级跟随合成() {
    let delta = UserDelta {
        panels: [(
            "T".to_string(),
            PanelDelta {
                rows: [("rowA".to_string(), ConfigValue::Int(99))].into_iter().collect(),
                ..PanelDelta::default()
            },
        )]
        .into_iter()
        .collect(),
        ..UserDelta::default()
    };

    // v1 基树: rowA=1, rowB=2
    let v1 = vec![tpanel(
        "T",
        vec![
            trow("rowA", "SLIDER", Some(ConfigValue::Int(1))),
            trow("rowB", "SLIDER", Some(ConfigValue::Int(2))),
        ],
    )];
    let merged_v1 = json_store::synthesize(&v1, &delta);
    assert_eq!(merged_v1[0].rows[0].value, Some(ConfigValue::Int(99)), "delta 覆盖用户改过的键");
    assert_eq!(merged_v1[0].rows[1].value, Some(ConfigValue::Int(2)), "未提及键跟基树");

    // v2 基树 (升级): 新增 rowC=3 → delta 未提及 → 自动补新默认 (升级跟随语义)
    let v2 = vec![tpanel(
        "T",
        vec![
            trow("rowA", "SLIDER", Some(ConfigValue::Int(1))),
            trow("rowB", "SLIDER", Some(ConfigValue::Int(2))),
            trow("rowC", "SLIDER", Some(ConfigValue::Int(3))),
        ],
    )];
    let merged_v2 = json_store::synthesize(&v2, &delta);
    let summary: Vec<String> = merged_v2[0]
        .rows
        .iter()
        .map(|r| format!("{}={}", r.label, r.value.as_ref().unwrap().as_config_string()))
        .collect();
    expect!["rowA=99 rowB=2 rowC=3"].assert_eq(&summary.join(" "));

    // delta 引用基树不存在的 panel → 丢弃 (模板演化残留), 不 panic
    let orphan = UserDelta {
        panels: [("removed".to_string(), PanelDelta::default())].into_iter().collect(),
        ..UserDelta::default()
    };
    let out = json_store::synthesize(&v1, &orphan);
    assert_eq!(out.len(), 1, "孤儿 panel delta 丢弃, 基树原样");
}

/// 落盘往返 + 损坏隔离 (json_store): save → load 等值; .bak 滚动;
/// 损坏文件改名 .corrupt 隔离 + 空 delta 回退出厂
#[test]
fn 配置栈_落盘往返与损坏隔离() {
    let path = tmp_cfg("io");
    let delta = UserDelta {
        version: 7,
        panels: [(
            "T".to_string(),
            PanelDelta {
                visible: Some(false),
                rows: [("rowA".to_string(), ConfigValue::Int(99))].into_iter().collect(),
                ..PanelDelta::default()
            }),
        ]
        .into_iter()
        .collect(),
        ..UserDelta::default()
    };
    json_store::save_delta(&path, &delta).expect("落盘");
    let back = json_store::load_delta(&path);
    assert_eq!(back, delta, "save → load 往返等值");

    // 二次落盘: 旧文件滚动为 .bak
    json_store::save_delta(&path, &UserDelta::default()).expect("二次落盘");
    assert!(std::path::Path::new(&format!("{path}.bak")).is_file(), "滚动保留一份 .bak");
    assert_eq!(json_store::load_delta(&path), UserDelta::default());

    // 损坏文件: 改名 .corrupt 隔离 + 空 delta (回退出厂)
    std::fs::write(&path, "{ not json !!").unwrap();
    assert_eq!(json_store::load_delta(&path), UserDelta::default(), "损坏 → 空 delta");
    assert!(
        std::path::Path::new(&format!("{path}.corrupt")).is_file(),
        "损坏文件隔离到 .corrupt"
    );

    // 不存在的文件 → 空 delta (首次运行)
    assert_eq!(json_store::load_delta(&tmp_cfg("never")), UserDelta::default());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.bak"));
    let _ = std::fs::remove_file(format!("{path}.corrupt"));
}

/// 深度优先找第一个带 property 的行 (出厂树顶层多为 HEADER, 数据行在 children)
fn first_keyed_row(rows: &[RowConfig]) -> Option<(String, Option<String>)> {
    for r in rows {
        if let Some(p) = &r.property {
            return Some((p.clone(), r.value.as_ref().map(|v| v.as_config_string())));
        }
        if let Some(hit) = first_keyed_row(&r.children) {
            return Some(hit);
        }
    }
    None
}

/// 服务层 save → 新实例 load_layout 重读等价 (出厂基 ⊕ 落盘 delta;
/// 注入树取 factory_default 同源, 保证 load_layout 的基一致)
#[test]
fn 配置栈_save后新实例重读等价() {
    let factory = json_store::factory_default();
    // 取出厂树中一个真实 (panel, 行键) 对, 避免猜键名 (递归, 含 HEADER 子行)
    let (panel_title, key, old_val) = factory
        .panels
        .iter()
        .find_map(|g| first_keyed_row(&g.rows).map(|(k, v)| (g.title.clone(), k, v)))
        .expect("出厂树应含至少一行带 property 的行");
    assert!(!panel_title.is_empty() && !key.is_empty(), "出厂键位非空");

    let path = tmp_cfg("reload");
    let svc1 = ConfigurationService::new(None);
    svc1.install_for_test(factory.panels.clone(), &path);
    let new_val = match old_val.as_deref() {
        Some("true") => "false".to_string(),  // 布尔行翻转
        Some("false") => "true".to_string(),
        _ => "424242".to_string(), // 其余类型统一写可读串
    };
    svc1.set_config(&key, &new_val);
    svc1.save_layout_config();
    assert!(std::path::Path::new(&path).is_file(), "save 后 delta 落盘");

    // 新实例: load_layout = factory ⊕ delta (与 svc1 装载面同基)
    let svc2 = ConfigurationService::new(None);
    svc2.load_layout(&path);
    assert_eq!(svc2.get_config(&key).as_deref(), Some(new_val.as_str()), "改过的键持久生效");
    assert_eq!(svc1.get_config(&key).as_deref(), Some(new_val.as_str()));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.bak"));
}

/// 恢复出厂: reset_to_factory 清空全部 delta (行值 + 组字段);
/// reset_all_layout_defaults 只清行值 delta (组字段保留, Java 原语义)
#[test]
fn 配置栈_恢复出厂() {
    let bus = Arc::new(UIStateBus::new());
    let svc = ConfigurationService::new(Some(Arc::clone(&bus)));
    svc.install_for_test(test_panels(), &tmp_cfg("reset"));

    svc.set_config("crosshairSwitch", "false");
    svc.set_config("dataPollIntervalMs", "500");
    assert!(svc.reset_to_factory(), "reset_to_factory 恒返回 true");
    assert_eq!(svc.get_config("crosshairSwitch").as_deref(), Some("true"), "行值回出厂");
    assert_eq!(svc.get_config("dataPollIntervalMs").as_deref(), Some("50"));

    // 行值重置 (上层顶替 RESET_REQUEST 事件链的入口): 改行值后全清
    svc.set_config("crosshairSwitch", "false");
    svc.set_config("dataPollIntervalMs", "500");
    assert!(svc.reset_all_layout_defaults(), "有行值 delta 时返回 true");
    assert_eq!(svc.get_config("crosshairSwitch").as_deref(), Some("true"));
    assert_eq!(svc.get_config("dataPollIntervalMs").as_deref(), Some("50"));
    // 再调 (已无行值 delta) → false 且不动树
    assert!(!svc.reset_all_layout_defaults());
}

/// 颜色通道: set_color_config 写 "r, g, b, a" 十进制串 → get_color_config 双格式
/// 解析回 RGBA; 缺键回落白色 (loadAppCheck 的默认色)
#[test]
fn 配置栈_颜色读写() {
    let svc = ConfigurationService::new(None);
    svc.install_for_test(test_panels(), &tmp_cfg("color"));

    svc.set_color_config("fontNum", [10, 20, 30, 40]);
    assert_eq!(svc.get_color_config("fontNum"), [10, 20, 30, 40], "十进制往返");

    // hex 行值 (出厂形态) → RGBA 直读
    assert_eq!(svc.get_color_config("fontWarn"), [255, 255, 255, 255], "缺键 → COLOR_WHITE");

    // 原生 hex 行: fontNum 初始 #FF0000 (本测试树) — 已被上写覆盖, 换树验证
    let svc2 = ConfigurationService::new(None);
    svc2.install_for_test(test_panels(), &tmp_cfg("color2"));
    assert_eq!(svc2.get_color_config("fontNum"), [255, 0, 0, 255], "hex #RRGGBB → RGBA");
}

/// 组查询与行类型: find_group_by_title 精确等值 (大小写敏感), row_type 递归定位
#[test]
fn 配置栈_组查询与行类型() {
    let svc = ConfigurationService::new(None);
    svc.install_for_test(test_panels(), &tmp_cfg("find"));

    let g = svc.find_group_by_title("T").expect("精确命中");
    assert_eq!(g.rows.len(), 5, "顶层行数 (含 HEADER)");
    assert!(svc.find_group_by_title("t").is_none(), "Java equals 大小写敏感原语义");

    assert_eq!(svc.row_type("T", "crosshairSwitch").as_deref(), Some("SWITCH"));
    // 嵌套子行递归命中
    assert_eq!(svc.row_type("T", "nestedData").as_deref(), Some("DATA"));
    assert_eq!(svc.row_type("T", "ghost"), None);
    assert_eq!(svc.row_type("noPanel", "crosshairSwitch"), None);
}
