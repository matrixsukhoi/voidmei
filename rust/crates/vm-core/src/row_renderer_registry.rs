//! 对应 Java: `src/ui/layout/renderer/RowRendererRegistry.java`
//!
//! Registry for row renderers. Maps row type strings to renderer instances.
//!
//! C 类边界 (D7): 各 XxxRowRenderer 实现与 WebPanel 返回类型均为 Swing UI
//! 类, 本层只翻注册/查找逻辑 —— `RowRenderer` 策略接口 (含其嵌套
//! `RenderContext`) 在此定义为泛型占位 trait, 返回类型以泛型参数 `R` 顶位
//! (Java `WebPanel`), 实现留后续 C 类批次。

use std::collections::HashMap;
use std::sync::Arc;

use crate::config_loader::{GroupConfig, RowConfig};

// =====================================================================
// RowRenderer.java 的占位翻译 (完整翻译归后续 C 类渲染器批次)
//
// TODO(port): RowRenderer.java 独立归 C 类 (CLASSIFY.md:199), 按规则 6 应
// 各占一个模块文件 —— C 类渲染器批次落地时必须显式裁决本归置: 确认挂靠
// 则删本标记, 否则拆出 row_renderer.rs 并把 trait 挪走。
// =====================================================================

/// Strategy interface for rendering different types of config rows.
/// Each row type (SLIDER, COMBO, SWITCH, DATA, HEADER) has its own renderer.
///
/// PORT: 返回类型 `WebPanel` (C 类 Swing 组件) → 泛型占位 `R`; `null` 返回
/// ("this row should not produce a component") → `Option<R>`。
///
/// PORT(C 类翻译者必读): 本 trait 的实现实例由注册表永久持有且 `get` 反复
/// 克隆共享 —— render() 内创建的订阅/音频资源**严禁存入渲染器实例字段**,
/// 否则即复刻 VoiceWarning 式泄漏 (LIFETIMES §2.1/§6.3)。Java 原形是把
/// CONFIG_CHANGED/VOICE_PACKS_REFRESH 订阅挂到返回的 panel 上、经
/// HierarchyListener 注销 (VoiceRowRenderer.java:264-335); Rust 正确形态
/// 是订阅 guard (`crate::ui_state_bus` 的 `Subscription`, Drop 即注销,
/// ui_state_bus.rs:115) 由返回的组件 `R` 持有, 以 Drop 取代 HierarchyListener。
pub trait RowRenderer<R> {
    /// Renders a config row into a UI component.
    ///
    /// @param row         The row configuration
    /// @param groupConfig The parent group configuration (for property binding)
    /// @param context     Rendering context with callbacks
    /// @return The rendered panel, or null if this row should not produce a
    ///         component
    fn render(
        &self,
        row: &RowConfig,
        group_config: &GroupConfig,
        context: &dyn RenderContext,
    ) -> Option<R>;
}

/// Context object providing callbacks and state for rendering.
///
/// PORT: Java `RowRenderer.RenderContext` 嵌套接口。方法取 `&self` (Java
/// 实例方法语义; DynamicDataPage 的匿名实现经共享引用改外部状态, Rust 实现
/// 侧以内部可变性对位)。默认方法 `resetToDefaults()` 的空实现体逐字保留。
pub trait RenderContext {
    /// Called when user changes a value and config should be saved
    fn on_save(&self);

    /// Called when layout needs to be rebuilt (e.g., panelColumns changed)
    fn on_rebuild(&self);

    /// Returns true if we're in the middle of programmatic updates
    fn is_updating(&self) -> bool;

    /// Syncs a boolean value to ConfigurationService (for overlay control)
    fn sync_to_config_service(&self, key: &str, value: bool);

    /// Gets a boolean value from ConfigurationService (for initial state)
    fn get_from_config_service(&self, key: &str, default_val: bool) -> bool;

    /// Syncs a string value to ConfigurationService
    fn sync_string_to_config_service(&self, key: &str, value: &str);

    /// Gets a string value from ConfigurationService (for initial state)
    fn get_string_from_config_service(&self, key: &str, default_val: &str) -> String;

    /// Resets all configuration items to their default values
    fn reset_to_defaults(&self) {}
}

// =====================================================================
// RowRendererRegistry.java 本体
// =====================================================================

/// Java `<clinit>` 静态块 15 条 `renderers.put(...)` 的数据化对位
/// (键 → 渲染器 Java 类简单名, 按 put 顺序原样)。
///
/// TODO(port): 各渲染器实现是 C 类 (Swing/WebPanel, CLASSIFY.md §13 豁免),
/// Java 类加载即完成的 15 条内建注册在 Rust 侧尚未接线 —— 由后续 C 类批次
/// 在 App 层构造点按本表逐条 `register` 补齐 (P5 MainForm 移植
/// DynamicDataPage 前必须完成, 否则 `get` 对一切类型回退默认)。
/// 本表锁定键集合/归属/顺序防漂移。
/// Java 侧每个 `new XxxRenderer()` 均为独立实例: INPUT 与 TEXT 各持一个
/// TextRowRenderer; "DATA" 条目与 defaultRenderer 也是两个不同实例。
pub const BUILTIN_ROW_TYPES: &[(&str, &str)] = &[
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
    // Note: HEADER is handled specially in the layout loop, not as a renderer
];

/// Registry for row renderers. Maps row type strings to renderer instances.
///
/// PORT(§2.9): Java `private static` 可变字段 (全库共享, 无锁跨线程可见)
/// 解散为实例字段, 实例由 UI/App 层拥有 (ui_state_bus.rs 单例解散先例;
/// Java 侧 PowerInfoOverlay 的 `new RowRendererRegistry()` 死实例化不迁移)。
/// PORT(§2.13): Java 裸 HashMap 无同步 —— 全库 register() 无调用方, 类
/// 初始化 (JLS 安全发布) 后实为只读; Rust 以 `&self` 读 / `&mut self` 写
/// 在编译期收紧为单写者, 可观察行为 (查表 → 命中或默认) 不变。
/// PORT(§2.5): map 只做 get/insert 从不迭代, 迭代序问题无涉。
/// PORT(线程边界): `dyn RowRenderer<R>` 未加 `Send + Sync` 界 → 本注册表
/// 及 `get` 发出的每个 Arc 均 !Send/!Sync, 属编译期收紧 (Java static 注册表
/// 理论上任意线程可达, 但唯一调用方 DynamicDataPage 在 EDT; 对齐 LIFETIMES
/// §5.3 UI 单写者纪律)。显式决策: 不补界。若 C 类/P5 MainForm(iced) 需在
/// UI 线程之外构造或跨线程持有注册表, 届时再加 bound 并记 DECISIONS.md。
pub struct RowRendererRegistry<R> {
    /// Java: `private static final Map<String, RowRenderer> renderers
    /// = new HashMap<>();` — 值为共享实例 (get 反复返回同一引用) → Arc。
    renderers: HashMap<String, Arc<dyn RowRenderer<R>>>,
    /// Java: `private static final RowRenderer defaultRenderer
    /// = new DataRowRenderer();` — DataRowRenderer 为 C 类, 本层不可构造,
    /// 构造时注入; 注意与 map 中 "DATA" 条目是不同实例 (见 BUILTIN_ROW_TYPES)。
    default_renderer: Arc<dyn RowRenderer<R>>,
}

impl<R> RowRendererRegistry<R> {
    /// 对应 Java 类加载期的 `<clinit>` (默认渲染器定死 + 15 条内建注册)。
    ///
    /// PORT: 内建渲染器实例是 C 类, 本构造器只承接默认渲染器注入;
    /// 15 条内建注册由 App 层构造点按 [`BUILTIN_ROW_TYPES`] 逐条 `register`
    /// (C 类批次接线)。未注入内建前 `get` 对一切类型回退默认 —— 与 Java
    /// 行为的差异仅限 C 类渲染器落地前的过渡期。
    pub fn new(default_renderer: Arc<dyn RowRenderer<R>>) -> Self {
        RowRendererRegistry {
            renderers: HashMap::new(),
            default_renderer,
        }
    }

    /// Gets the renderer for a given row type.
    ///
    /// @param rowType The row type string (SLIDER, COMBO, SWITCH, DATA)
    /// @return The appropriate renderer, or default DataRowRenderer if not found
    ///
    /// PORT: Java 静态方法 → 实例方法 (§2.9 单例解散)。键为精确 String
    /// 匹配 (大小写敏感, 对齐 HashMap/equals 语义) —— 大小写归一在上游
    /// ConfigLoader (`row.type = rawType.toUpperCase().replace("-", "_")`,
    /// ConfigLoader.java:295 / config_loader.rs:827), 注册表不做归一 (保真)。
    pub fn get(&self, row_type: &str) -> Arc<dyn RowRenderer<R>> {
        // Java: return renderers.getOrDefault(rowType, defaultRenderer);
        self.renderers
            .get(row_type)
            .cloned()
            .unwrap_or_else(|| Arc::clone(&self.default_renderer))
    }

    /// Registers a custom renderer for a row type.
    ///
    /// PORT: Java `Map.put` 覆盖旧值、返回值被丢弃 → `HashMap::insert`
    /// 同语义。Java 可 put null 值 (此后 get 命中 null 而非默认, 调用方
    /// NPE), Rust 的 Arc 非 null —— 该缺陷形态不可表示 (良性收紧)。
    pub fn register(&mut self, row_type: &str, renderer: Arc<dyn RowRenderer<R>>) {
        // Java: renderers.put(rowType, renderer);
        self.renderers.insert(row_type.to_string(), renderer);
    }
}

// =====================================================================
// Tests — 注册/查找逻辑边界测试 (C 类占位 trait 以测试替身驱动)
// =====================================================================
#[cfg(test)]
mod tests {
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
}
