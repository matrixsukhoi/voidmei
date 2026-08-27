use super::*;
use std::cell::RefCell;

/// 测试替身渲染器: render 返回固定占位 "面板" (R = &'static str),
/// 返回 None 复刻 Java "render 返回 null" 路径。
struct FakeRenderer(Option<&'static str>);

impl RowRenderer<&'static str> for FakeRenderer {
    fn render(
        &self,
        _row: &RowConfig,
        _group_config: &GroupConfig,
        _context: &dyn RenderContext,
    ) -> Option<&'static str> {
        self.0
    }
}

/// 探针渲染器: 验证 `&dyn RenderContext` 动态分发管道 —— is_updating
/// 为真时返回 None (对位 Java 渲染器按上下文状态返回 null 的形态)。
struct ProbingRenderer;

impl RowRenderer<&'static str> for ProbingRenderer {
    fn render(
        &self,
        _row: &RowConfig,
        _group_config: &GroupConfig,
        context: &dyn RenderContext,
    ) -> Option<&'static str> {
        if context.is_updating() {
            None
        } else {
            Some("rendered")
        }
    }
}

/// 测试替身上下文: 记录全部回调; **不覆写** reset_to_defaults
/// (走 trait 默认空实现, 对位 Java default 方法)。
struct RecordingContext {
    updating: bool,
    calls: RefCell<Vec<String>>,
}

impl RecordingContext {
    fn new(updating: bool) -> Self {
        RecordingContext {
            updating,
            calls: RefCell::new(Vec::new()),
        }
    }
}

impl RenderContext for RecordingContext {
    fn on_save(&self) {
        self.calls.borrow_mut().push("on_save".to_string());
    }

    fn on_rebuild(&self) {
        self.calls.borrow_mut().push("on_rebuild".to_string());
    }

    fn is_updating(&self) -> bool {
        self.updating
    }

    fn sync_to_config_service(&self, key: &str, value: bool) {
        self.calls
            .borrow_mut()
            .push(format!("sync:{key}={value}"));
    }

    fn get_from_config_service(&self, key: &str, default_val: bool) -> bool {
        self.calls.borrow_mut().push(format!("getBool:{key}"));
        default_val
    }

    fn sync_string_to_config_service(&self, key: &str, value: &str) {
        self.calls
            .borrow_mut()
            .push(format!("syncStr:{key}={value}"));
    }

    fn get_string_from_config_service(&self, key: &str, default_val: &str) -> String {
        self.calls.borrow_mut().push(format!("getStr:{key}"));
        default_val.to_string()
    }
}

fn row_and_group() -> (RowConfig, GroupConfig) {
    (
        RowConfig::new("label".to_string(), None, "%.0f".to_string()),
        GroupConfig::new("group".to_string()),
    )
}

// Java: getOrDefault 未知键 → defaultRenderer (本层为注入的默认)
#[test]
fn get_unknown_type_falls_back_to_default() {
    let default: Arc<dyn RowRenderer<&'static str>> = Arc::new(FakeRenderer(Some("default")));
    let mut reg = RowRendererRegistry::new(Arc::clone(&default));
    reg.register("SLIDER", Arc::new(FakeRenderer(Some("slider"))));

    let (row, group) = row_and_group();
    let ctx = RecordingContext::new(false);
    let got = reg.get("NO_SUCH_TYPE");
    assert!(Arc::ptr_eq(&got, &default)); // Java 返回同一 defaultRenderer 引用
    assert_eq!(got.render(&row, &group, &ctx), Some("default"));
}

// Java: 命中键返回 map 内共享实例, 反复 get 同一引用
#[test]
fn get_returns_registered_shared_instance() {
    let default: Arc<dyn RowRenderer<&'static str>> = Arc::new(FakeRenderer(Some("default")));
    let mut reg = RowRendererRegistry::new(default);
    let shared: Arc<dyn RowRenderer<&'static str>> = Arc::new(FakeRenderer(Some("combo-impl")));
    reg.register("COMBO", Arc::clone(&shared));

    let a = reg.get("COMBO");
    let b = reg.get("COMBO");
    assert!(Arc::ptr_eq(&a, &b)); // Java: map 值即同一实例
    assert!(Arc::ptr_eq(&a, &shared));

    let (row, group) = row_and_group();
    let ctx = RecordingContext::new(false);
    assert_eq!(a.render(&row, &group, &ctx), Some("combo-impl"));
}

// Java: Map.put 覆盖旧映射 (register 二次调用后新渲染器生效)
#[test]
fn register_overwrites_existing_mapping() {
    let default: Arc<dyn RowRenderer<&'static str>> = Arc::new(FakeRenderer(Some("default")));
    let mut reg = RowRendererRegistry::new(default);
    reg.register("SWITCH", Arc::new(FakeRenderer(Some("old"))));
    reg.register("SWITCH", Arc::new(FakeRenderer(Some("new"))));

    let (row, group) = row_and_group();
    let ctx = RecordingContext::new(false);
    assert_eq!(reg.get("SWITCH").render(&row, &group, &ctx), Some("new"));
}

// Java: render 可返回 null ("should not produce a component") → Option::None;
// 同时验证 &dyn RenderContext 动态分发到实现 (is_updating 路由)
#[test]
fn render_none_path_and_context_dispatch() {
    let default: Arc<dyn RowRenderer<&'static str>> = Arc::new(ProbingRenderer);
    let reg = RowRendererRegistry::new(default);
    let (row, group) = row_and_group();

    let idle = RecordingContext::new(false);
    assert_eq!(reg.get("ANY").render(&row, &group, &idle), Some("rendered"));

    let updating = RecordingContext::new(true);
    assert_eq!(reg.get("ANY").render(&row, &group, &updating), None);
}

// Java 静态块表: 完整 15 条序列逐位锁定 —— 顺序/键/归属全部对拍
// (注释声称"按 put 顺序原样", 测试即锁全序列而非首尾抽样);
// 键唯一、TextRowRenderer×2、HEADER 缺席均被序列蕴含;
// HEADER 特判不进注册表 (静态块尾注)
#[test]
fn builtin_table_matches_java_static_block() {
    // Java <clinit> 静态块 15 条 renderers.put(...) 逐行原样
    let expected: &[(&str, &str)] = &[
        ("SLIDER", "SliderRowRenderer"),
        ("COMBO", "ComboRowRenderer"),
        ("SWITCH", "SwitchRowRenderer"),
        ("SWITCH_INV", "SwitchInvRowRenderer"),
        ("FILELIST", "FileListRowRenderer"),
        ("FMLIST", "FMListRowRenderer"),
        ("HOTKEY", "HotkeyRowRenderer"),
        ("COLOR", "ColorRowRenderer"),
        ("BUTTON", "ButtonRowRenderer"),
        ("DATA", "DataRowRenderer"),
        ("INPUT", "TextRowRenderer"),
        ("TEXT", "TextRowRenderer"),
        ("VOICE", "VoiceRowRenderer"),
        ("VOICE_GLOBAL", "VoiceGlobalRenderer"),
        ("INFO", "InfoRowRenderer"),
    ];
    assert_eq!(BUILTIN_ROW_TYPES, expected);
}

// Java: HashMap 精确 String 匹配 —— 大小写不同即未命中走默认;
// 归一化责任在上游 ConfigLoader (ConfigLoader.java:295 / config_loader.rs:827)
#[test]
fn lookup_is_exact_case_sensitive_match() {
    let default: Arc<dyn RowRenderer<&'static str>> = Arc::new(FakeRenderer(Some("default")));
    let mut reg = RowRendererRegistry::new(default);
    reg.register("SLIDER", Arc::new(FakeRenderer(Some("slider"))));

    let (row, group) = row_and_group();
    let ctx = RecordingContext::new(false);
    assert_eq!(
        reg.get("slider").render(&row, &group, &ctx),
        Some("default"),
        "小写未命中 (Java equals 精确匹配) → defaultRenderer"
    );
}

// Java: interface 默认方法 resetToDefaults() 空实现 — 无副作用可调用
#[test]
fn render_context_default_reset_to_defaults_is_noop() {
    let ctx = RecordingContext::new(false);
    ctx.on_save(); // 对照组: 具体方法有记录
    ctx.reset_to_defaults(); // 默认实现 (Java default 空方法体)
    let calls = ctx.calls.borrow();
    assert_eq!(*calls, vec!["on_save".to_string()]);
}
