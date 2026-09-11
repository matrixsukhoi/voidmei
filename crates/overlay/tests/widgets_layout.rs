//! widgets 域注册表黑盒场景: WidgetMeta 注册名清单 / 查表一致性 /
//! fm_sidecar 动作面字段可达性。零字体零窗口 — 注册表是编译期 const 表,
//! expect 快照防条目漂移 (palette 展示序即拼接序)。
#![allow(non_snake_case)] // 中文场景命名是项目惯例


use expect_test::expect;

use overlay::widgets::registry::{lookup_widget, widget_registry, WidgetCategory};
use overlay::widgets::SidecarAction;

/// 分类名速记 (expect 表内聚合)
fn cat_name(c: WidgetCategory) -> &'static str {
    match c {
        WidgetCategory::Text => "Text",
        WidgetCategory::Gauge => "Gauge",
        WidgetCategory::Chart => "Chart",
        WidgetCategory::List => "List",
        WidgetCategory::Composite => "Composite",
        WidgetCategory::Decor => "Decor",
    }
}

/// 注册表全量清单: type_name / 显示名 / 分类 / 黑盒复合位 (palette 展示序)。
/// 新增组件 → 本表同步 (expect 快照即守卫)。
#[test]
fn registry_注册名清单() {
    let actual = widget_registry()
        .iter()
        .map(|m| {
            format!(
                "{} | {} | {} | composite={}",
                m.type_name,
                m.display_zh,
                cat_name(m.category),
                m.composite
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        core.minihud.speed | 速度读数 | Text | composite=false
        core.minihud.aoa | AoA 指示 | Text | composite=false
        core.minihud.altitude | 高度读数 | Text | composite=false
        core.minihud.energy | 能量读数 | Text | composite=false
        core.minihud.flaps | 襟翼/可变翼 | Text | composite=false
        core.minihud.airbrake | 减速板 | Text | composite=false
        core.minihud.gear | 起落架 | Text | composite=false
        core.minihud.sep | SEP 读数 | Text | composite=false
        core.minihud.gload | G 读数 | Text | composite=false
        core.minihud.maneuverbar | 机动刻度条 | Text | composite=false
        core.minihud.flapBar | 智能襟翼条 | Gauge | composite=false
        core.minihud.speedBar | 速度条 | Gauge | composite=false
        core.minihud.throttleBar | 油门条 | Gauge | composite=false
        core.gauge.attitude | 姿态指示器 | Gauge | composite=false
        core.gauge.compass | 罗盘 | Gauge | composite=false
        core.decor.crosshair | 准星 | Decor | composite=false
        core.data.field | 数据字段 | Text | composite=false
        core.engine.gauge | 引擎仪表 | Gauge | composite=false
        core.axes.crosshair | 操纵面十字 | Gauge | composite=true
        core.attitude.window | 地平仪窗 | Gauge | composite=true
        core.gearflaps.flapbar | 襟翼竖条 | Gauge | composite=false
        core.gearflaps.warn | 起落架告警 | Text | composite=false
        core.axes.rudderbar | 方向舵横条 | Gauge | composite=false
        core.fm.field | FM字段 | Text | composite=false
        core.fm.meta | FM文本行 | Text | composite=false
        core.fm.list | FM数据列表 | List | composite=true
        core.fm.thrust_chart | 推力-真空速曲线 | Chart | composite=true"#]]
    .assert_eq(&actual);
}

/// 查表: 命中返回元数据, 未注册类型 None; default_props 恒为合法 JSON
/// (空值工厂 Err 的组件靠它兜底, 非法 JSON 会让编辑器静默不建)
#[test]
fn lookup_查表与默认props() {
    assert!(lookup_widget("core.minihud.speed").is_some());
    assert!(lookup_widget("core.fm.thrust_chart").is_some());
    assert!(lookup_widget("core.nope.ghost").is_none());
    assert!(lookup_widget("").is_none());

    // 全表 default_props 必须可解析为 JSON 对象
    for m in widget_registry() {
        let v: serde_json::Value = serde_json::from_str(m.default_props)
            .unwrap_or_else(|e| panic!("{} default_props 非法 JSON: {e}", m.type_name));
        assert!(v.is_object(), "{} default_props 应为对象", m.type_name);
    }

    // 抽查两个非空默认值 (编辑器新建组件的合法初值)
    expect!["core.fm.field -> {\"key\":\"weight.empty\"}"].assert_eq(&format!(
        "core.fm.field -> {}",
        lookup_widget("core.fm.field").unwrap().default_props
    ));
    expect!["core.engine.gauge -> {\"kind\":\"throttle\"}"].assert_eq(&format!(
        "core.engine.gauge -> {}",
        lookup_widget("core.engine.gauge").unwrap().default_props
    ));
}

/// 注册表元数据可达性: config_keys/data_shorts 可迭代; 键名不重复
#[test]
fn registry_键集无重复() {
    for m in widget_registry() {
        let mut keys = m.config_keys.to_vec();
        keys.sort_unstable();
        let n = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), n, "{} config_keys 有重复键", m.type_name);
    }
    // minihud.speed 的数据短名 (palette 提示/校验的抽样式断言)
    let speed = lookup_widget("core.minihud.speed").unwrap();
    expect![[r#"config ["drawHUDtext", "showHUDSpeed"] shorts ["ias"]"#]].assert_eq(&format!(
        "config {:?} shorts {:?}",
        speed.config_keys, speed.data_shorts
    ));
}

/// fm_sidecar 动作面: SidecarAction 是渲染线程/host 交互的返回值契约 —
/// 五变体 Copy+PartialEq 可比较 (守卫消费侧判等的面), 字段形态 Debug 钉死
#[test]
fn sidecar_动作面形态() {
    let cases = [
        SidecarAction::None,
        SidecarAction::Resize(320, 240),
        SidecarAction::SetVisible(false),
        SidecarAction::SetVisibleResize(true, 300, 500),
        SidecarAction::Close,
    ];
    let actual = cases
        .iter()
        .map(|a| format!("{a:?}"))
        .collect::<Vec<_>>()
        .join("\n");
    expect![[r#"
        None
        Resize(320, 240)
        SetVisible(false)
        SetVisibleResize(true, 300, 500)
        Close"#]]
    .assert_eq(&actual);

    // 相等性面: 同参等, 异参不等
    assert_eq!(SidecarAction::Resize(320, 240), SidecarAction::Resize(320, 240));
    assert_ne!(SidecarAction::None, SidecarAction::Close);
    // Copy 面 (值语义传递不克隆)
    let a = SidecarAction::SetVisibleResize(true, 1, 2);
    let b = a;
    assert_eq!(a, b);
}
