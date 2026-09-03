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
            group: Some(GroupStub {
                title: "MiniHUD".to_string(),
            }),
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
    assert_eq!(
        s.get_group_config().map(|g| g.title.as_str()),
        Some("MiniHUD")
    );

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
