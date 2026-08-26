//! 对应 Java: `src/prog/config/OverlaySettings.java`

/// Generalized interface for overlay settings.
/// Provides relative-to-absolute coordinate mapping and supports write-back for
/// dragging.
///
/// PORT: Java interface → Rust trait (§1 多实现接口, 实现 ConfigurationService.GenericOverlaySettingsImpl)。
pub trait OverlaySettings {

    /// PORT: Java `getGroupConfig()` 返回 `ConfigLoader.GroupConfig` — ConfigLoader
    /// 属 B 类尚未翻译, 以关联类型占位不引入前向依赖; ConfigurationService 实现
    /// 时指定 `type GroupConfig = crate::config_loader::GroupConfig`。
    type GroupConfig;

    /// Get absolute X coordinate in pixels.
    ///
    /// @param width Window width for centering fallback (if applicable)
    fn get_window_x(&self, width: i32) -> i32;

    /// Get absolute Y coordinate in pixels.
    ///
    /// @param height Window height for centering fallback (if applicable)
    fn get_window_y(&self, height: i32) -> i32;

    /// Save absolute pixel coordinates back to the relative coordinate system.
    ///
    /// PORT: Java 写方法 → &self (非 &mut): Java 实现是 ConfigurationService 的内部类
    /// 视图, 写回目标 gc.x/gc.y 位于共享的 layoutConfigs (ConfigurationService.java:460-472),
    /// 视图自身无独占状态; Rust 侧视图持共享句柄, 与 ConfigProvider::set_config 同方向
    /// (LIFETIMES §7 Arc<ConfigStore>), 写回由实现侧内部可变性完成。
    fn save_window_position(&self, x: f64, y: f64);

    /// Get the font name for this overlay.
    fn get_font_name(&self) -> String;

    /// Get the numeric font name for this overlay.
    fn get_num_font_name(&self) -> String;

    /// Get the font size adjustment for this overlay.
    fn get_font_size_add(&self) -> i32;

    /// Generic property getters
    ///
    /// PORT: Java 可空入参契约未声明, def 按非空 `&str` 处理; 返回 String (Java 返回引用, 值语义等价)。
    fn get_bool(&self, key: &str, def: bool) -> bool;

    fn get_int(&self, key: &str, def: i32) -> i32;

    fn get_string(&self, key: &str, def: &str) -> String;

    /// Get the underlying GroupConfig for advanced configuration access.
    ///
    /// PORT: Java 实现可返回 null (GenericOverlaySettingsImpl 找不到分组时) → Option;
    /// 借用仅当次有效 — Java 返回的是活对象引用 (EngineInfoConfig 保留该引用为字段,
    /// setConfig/import 后重读可见新值), Rust 借用无法跨 &self 长期持有, 有持有需求
    /// 的调用方须每次重取或存快照; 写回: 位置 x/y 走 save_window_position,
    /// 其余字段走 ConfigProvider::set_config。
    fn get_group_config(&self) -> Option<&Self::GroupConfig>;

    /// 获取是否启用游戏失焦时自动隐藏overlay功能。
    ///
    /// @return true如果启用自动隐藏
    fn auto_hide_on_focus_loss(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    // GroupConfig 关联类型的替身 (真实类型随 ConfigLoader 翻译落地)
    struct GroupStub {
        title: String,
    }

    // 最小 mock: 全字段直存直取, 验证 trait 契约的每个方法
    struct FakeOverlay {
        x: i32,
        y: i32,
        saved: RefCell<(f64, f64)>,
        font_name: String,
        num_font_name: String,
        font_size_add: i32,
        bools: Vec<(String, bool)>,
        ints: Vec<(String, i32)>,
        strings: Vec<(String, String)>,
        group: Option<GroupStub>,
        auto_hide: bool,
    }

    impl FakeOverlay {
        fn new() -> Self {
            FakeOverlay {
                x: 100,
                y: -50,
                saved: RefCell::new((0.0, 0.0)),
                font_name: "微软雅黑".to_string(),
                num_font_name: "DIN Pro 400".to_string(),
                font_size_add: -6,
                bools: vec![("flag_on".to_string(), true)],
                ints: vec![
                    ("int_min".to_string(), i32::MIN),
                    ("int_max".to_string(), i32::MAX),
                ],
                strings: vec![("name".to_string(), "MiniHUD".to_string())],
                group: Some(GroupStub { title: "MiniHUD".to_string() }),
                auto_hide: true,
            }
        }
    }

    impl OverlaySettings for FakeOverlay {
        type GroupConfig = GroupStub;

        fn get_window_x(&self, _width: i32) -> i32 {
            self.x
        }

        fn get_window_y(&self, _height: i32) -> i32 {
            self.y
        }

        fn save_window_position(&self, x: f64, y: f64) {
            *self.saved.borrow_mut() = (x, y);
        }

        fn get_font_name(&self) -> String {
            self.font_name.clone()
        }

        fn get_num_font_name(&self) -> String {
            self.num_font_name.clone()
        }

        fn get_font_size_add(&self) -> i32 {
            self.font_size_add
        }

        fn get_bool(&self, key: &str, def: bool) -> bool {
            self.bools
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| *v)
                .unwrap_or(def)
        }

        fn get_int(&self, key: &str, def: i32) -> i32 {
            self.ints
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| *v)
                .unwrap_or(def)
        }

        fn get_string(&self, key: &str, def: &str) -> String {
            self.strings
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| def.to_string())
        }

        fn get_group_config(&self) -> Option<&Self::GroupConfig> {
            self.group.as_ref()
        }

        fn auto_hide_on_focus_loss(&self) -> bool {
            self.auto_hide
        }
    }

    // 坐标 getter 含负值边界; 参数按契约传入 (宽度仅作回退用, mock 直存)
    #[test]
    fn test_window_coordinates() {
        let s = FakeOverlay::new();
        assert_eq!(s.get_window_x(1920), 100);
        assert_eq!(s.get_window_y(1080), -50);
    }

    // saveWindowPosition 记录绝对像素坐标 (f64 精度原样保持, 含负半屏)
    #[test]
    fn test_save_window_position_records_f64() {
        let s = FakeOverlay::new();
        s.save_window_position(-12.75, 4080.5);
        assert_eq!(*s.saved.borrow(), (-12.75, 4080.5));
    }

    // 字体名 getter (CJK 字体名 §2.1 边界) 与字号偏移负值
    #[test]
    fn test_font_getters() {
        let s = FakeOverlay::new();
        assert_eq!(s.get_font_name(), "微软雅黑");
        assert_eq!(s.get_num_font_name(), "DIN Pro 400");
        assert_eq!(s.get_font_size_add(), -6);
    }

    // 泛型 getter: 命中/缺省回退, int 取 i32 上下界
    #[test]
    fn test_generic_property_getters() {
        let s = FakeOverlay::new();
        assert!(s.get_bool("flag_on", false));
        assert!(!s.get_bool("missing", false));
        assert!(s.get_bool("missing", true)); // def 透传
        assert_eq!(s.get_int("int_min", 0), i32::MIN);
        assert_eq!(s.get_int("int_max", 0), i32::MAX);
        assert_eq!(s.get_int("missing", 42), 42);
        assert_eq!(s.get_string("name", "x"), "MiniHUD");
        assert_eq!(s.get_string("missing", "fallback"), "fallback");
    }

    // getGroupConfig: Some 借用可读内部字段; None 形态对应 Java 实现的 null 返回
    #[test]
    fn test_group_config_some_and_none() {
        let s = FakeOverlay::new();
        assert_eq!(s.get_group_config().map(|g| g.title.as_str()), Some("MiniHUD"));

        let mut empty = FakeOverlay::new();
        empty.group = None;
        assert!(empty.get_group_config().is_none());
    }

    // autoHideOnFocusLoss 直读
    #[test]
    fn test_auto_hide_on_focus_loss() {
        let s = FakeOverlay::new();
        assert!(s.auto_hide_on_focus_loss());
    }

    // trait 须可作 dyn 对象 (关联类型在 dyn 处显式指定)
    #[test]
    fn test_dyn_dispatch_with_associated_type() {
        let s: Box<dyn OverlaySettings<GroupConfig = GroupStub>> = Box::new(FakeOverlay::new());
        assert_eq!(s.get_window_x(800), 100);
        s.save_window_position(1.5, 2.5);
        assert_eq!(s.get_font_name(), "微软雅黑");
        assert!(s.get_group_config().is_some());
    }
}
