//! 对应 Java: `src/prog/config/HUDSettings.java`

use crate::config_api::overlay_settings::OverlaySettings;

/// Interface for reading HUD-specific configurations.
/// Decouples the UI layer from underlying key names and parsing logic.
///
/// PORT: Java `extends OverlaySettings` → Rust supertrait 约束。
/// Java 源文件中以下三个方法以 @Override 显式重声明父接口方法, 对实现类是
/// 零语义增量 (接口继承已含契约, 实现类只提供一个方法):
/// ```text
/// String getNumFont(); 之外的重声明部分:
///     @Override int getWindowX(int canvasWidth);
///     @Override int getWindowY(int canvasHeight);
///     @Override void saveWindowPosition(double x, double y);
/// ```
/// PORT: Rust 子 trait 若按字面重声明同名同签名方法, 会造出两个独立分发槽
/// (调用点歧义 + 上转 dyn 后分发到父 trait 版本), 反而偏离 Java 单 vtable 槽
/// 语义 — 故不重声明, 契约由 supertrait `OverlaySettings` 唯一承载, 注释留痕。
pub trait HUDSettings: OverlaySettings {
    fn get_num_font(&self) -> String;

    fn get_crosshair_scale(&self) -> i32;

    fn get_crosshair_name(&self) -> String;

    fn is_display_crosshair(&self) -> bool;

    fn use_texture_crosshair(&self) -> bool;

    fn draw_hud_text(&self) -> bool;

    fn show_attitude_gauge(&self) -> bool;

    fn get_aoa_warning_ratio(&self) -> f64;

    fn get_aoa_bar_warning_ratio(&self) -> f64;

    fn enable_flap_angle_bar(&self) -> bool;

    fn show_speed_bar(&self) -> bool;

    fn draw_hud_mach(&self) -> bool;

    fn is_speed_label_disabled(&self) -> bool;

    fn is_altitude_label_disabled(&self) -> bool;

    fn is_sep_label_disabled(&self) -> bool;

    // 组件级独立显示开关 — 每个视觉元素可独立控制
    fn show_hud_speed(&self) -> bool;         // Row 0: 速度文字
    fn show_hud_aoa(&self) -> bool;           // Row 0: AoA bar + α文字
    fn show_hud_altitude(&self) -> bool;      // Row 1: 高度文字
    fn show_hud_energy(&self) -> bool;        // Row 1: 能量读数
    fn show_hud_mechanization(&self) -> bool; // Row 2: 襟翼/起落架文字 (旧，保留向后兼容)
    fn show_hud_flaps(&self) -> bool;         // Row 2: 襟翼/可变翼
    fn show_hud_airbrake(&self) -> bool;      // Row 2: 减速板 BRK
    fn show_hud_gear(&self) -> bool;          // Row 2: 起落架 GEA
    fn show_hud_sep(&self) -> bool;           // Row 3: 爬升率文字
    fn show_hud_g_load(&self) -> bool;        // Row 4: G-force 文字
    fn show_hud_maneuver_bar(&self) -> bool;  // Row 4: 机动条

    fn is_attitude_indicator_inertial_mode(&self) -> bool;

    /// PORT: 底层 GPUCompatibilityHelper 经 CLASSIFY 裁决"不迁移"(JVM 专属
    /// sun.java2d 属性, Rust 版无 JVM, 功能天然消亡); 接口方法按源文件原样保留,
    /// 未来 ConfigurationService 实现时可恒返回配置存储值/false。
    fn is_gpu_compatibility_mode(&self) -> bool;

    fn always_show_radar_altitude(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct GroupStub;

    // 最小 mock: 每个方法返回固定边界值, 全方法逐一验证契约
    struct FakeHud {
        saved: RefCell<(f64, f64)>,
    }

    impl FakeHud {
        fn new() -> Self {
            FakeHud { saved: RefCell::new((0.0, 0.0)) }
        }
    }

    // 父 trait 方法在 HUD 对象上唯一实现 (Java @Override 重声明对应的单 vtable 槽)
    impl OverlaySettings for FakeHud {
        type GroupConfig = GroupStub;

        fn get_window_x(&self, _width: i32) -> i32 {
            640
        }

        fn get_window_y(&self, _height: i32) -> i32 {
            360
        }

        fn save_window_position(&self, x: f64, y: f64) {
            *self.saved.borrow_mut() = (x, y);
        }

        fn get_font_name(&self) -> String {
            "text".to_string()
        }

        fn get_num_font_name(&self) -> String {
            "num".to_string()
        }

        fn get_font_size_add(&self) -> i32 {
            2
        }

        fn get_bool(&self, _key: &str, def: bool) -> bool {
            def
        }

        fn get_int(&self, _key: &str, def: i32) -> i32 {
            def
        }

        fn get_string(&self, _key: &str, def: &str) -> String {
            def.to_string()
        }

        fn get_group_config(&self) -> Option<&Self::GroupConfig> {
            None // HUD 主分组常在, 此处取 None 形态覆盖可空契约
        }

        fn auto_hide_on_focus_loss(&self) -> bool {
            false
        }
    }

    impl HUDSettings for FakeHud {
        fn get_num_font(&self) -> String {
            "DIN Pro 400".to_string()
        }

        fn get_crosshair_scale(&self) -> i32 {
            i32::MIN
        }

        fn get_crosshair_name(&self) -> String {
            "crosshair_01".to_string()
        }

        fn is_display_crosshair(&self) -> bool {
            true
        }

        fn use_texture_crosshair(&self) -> bool {
            false
        }

        fn draw_hud_text(&self) -> bool {
            true
        }

        fn show_attitude_gauge(&self) -> bool {
            true
        }

        fn get_aoa_warning_ratio(&self) -> f64 {
            0.85
        }

        fn get_aoa_bar_warning_ratio(&self) -> f64 {
            1.0
        }

        fn enable_flap_angle_bar(&self) -> bool {
            true
        }

        fn show_speed_bar(&self) -> bool {
            false
        }

        fn draw_hud_mach(&self) -> bool {
            true
        }

        fn is_speed_label_disabled(&self) -> bool {
            false
        }

        fn is_altitude_label_disabled(&self) -> bool {
            false
        }

        fn is_sep_label_disabled(&self) -> bool {
            true
        }

        fn show_hud_speed(&self) -> bool {
            true
        }

        fn show_hud_aoa(&self) -> bool {
            true
        }

        fn show_hud_altitude(&self) -> bool {
            true
        }

        fn show_hud_energy(&self) -> bool {
            true
        }

        fn show_hud_mechanization(&self) -> bool {
            false
        }

        fn show_hud_flaps(&self) -> bool {
            true
        }

        fn show_hud_airbrake(&self) -> bool {
            true
        }

        fn show_hud_gear(&self) -> bool {
            true
        }

        fn show_hud_sep(&self) -> bool {
            true
        }

        fn show_hud_g_load(&self) -> bool {
            true
        }

        fn show_hud_maneuver_bar(&self) -> bool {
            true
        }

        fn is_attitude_indicator_inertial_mode(&self) -> bool {
            true
        }

        fn is_gpu_compatibility_mode(&self) -> bool {
            false
        }

        fn always_show_radar_altitude(&self) -> bool {
            true
        }
    }

    // HUD 专属 getter 全量过一遍 (含 i32::MIN、f64 比率边界)
    #[test]
    fn test_hud_specific_getters() {
        let h = FakeHud::new();
        assert_eq!(h.get_num_font(), "DIN Pro 400");
        assert_eq!(h.get_crosshair_scale(), i32::MIN);
        assert_eq!(h.get_crosshair_name(), "crosshair_01");
        assert!(h.is_display_crosshair());
        assert!(!h.use_texture_crosshair());
        assert!(h.draw_hud_text());
        assert!(h.show_attitude_gauge());
        assert_eq!(h.get_aoa_warning_ratio(), 0.85);
        assert_eq!(h.get_aoa_bar_warning_ratio(), 1.0);
        assert!(h.enable_flap_angle_bar());
        assert!(!h.show_speed_bar());
        assert!(h.draw_hud_mach());
        assert!(!h.is_speed_label_disabled());
        assert!(!h.is_altitude_label_disabled());
        assert!(h.is_sep_label_disabled());
    }

    // 组件级独立显示开关 (Row 0~4) 逐一验证
    #[test]
    fn test_show_hud_component_switches() {
        let h = FakeHud::new();
        assert!(h.show_hud_speed());
        assert!(h.show_hud_aoa());
        assert!(h.show_hud_altitude());
        assert!(h.show_hud_energy());
        assert!(!h.show_hud_mechanization()); // 旧开关默认关, 新三开关开
        assert!(h.show_hud_flaps());
        assert!(h.show_hud_airbrake());
        assert!(h.show_hud_gear());
        assert!(h.show_hud_sep());
        assert!(h.show_hud_g_load());
        assert!(h.show_hud_maneuver_bar());
    }

    // 姿态/杂项开关
    #[test]
    fn test_misc_switches() {
        let h = FakeHud::new();
        assert!(h.is_attitude_indicator_inertial_mode());
        assert!(!h.is_gpu_compatibility_mode());
        assert!(h.always_show_radar_altitude());
    }

    // Java @Override 重声明对应的三方法: 单一实现, 具体类型与 dyn 分发同值
    #[test]
    fn test_inherited_window_methods_single_slot() {
        let h = FakeHud::new();
        assert_eq!(h.get_window_x(1920), 640);
        assert_eq!(h.get_window_y(900), 360);
        h.save_window_position(1.25, -0.5);
        assert_eq!(*h.saved.borrow(), (1.25, -0.5));
    }

    // 子 trait 到父 trait 的多态兼容: 泛型 T: OverlaySettings 接受 HUD 实现
    // (对应 Java "HUDSettings is-an OverlaySettings")
    #[test]
    fn test_hud_usable_as_overlay_settings_generic() {
        fn via_generic<T: OverlaySettings>(s: &T) -> String {
            s.get_font_name()
        }
        fn via_dyn(s: &dyn OverlaySettings<GroupConfig = GroupStub>) -> i32 {
            s.get_window_x(100)
        }
        let h = FakeHud::new();
        assert_eq!(via_generic(&h), "text");
        assert_eq!(via_dyn(&h), 640);
    }

    // trait upcasting: Box<dyn HUDSettings> 可上转 &dyn OverlaySettings (Rust 1.86+),
    // 上转前后分发到同一实现 (Java 单 vtable 槽语义)
    #[test]
    fn test_trait_upcast_to_overlay_settings() {
        let h: Box<dyn HUDSettings<GroupConfig = GroupStub>> = Box::new(FakeHud::new());
        assert_eq!(h.get_num_font(), "DIN Pro 400");
        let base: &dyn OverlaySettings<GroupConfig = GroupStub> = &*h;
        assert_eq!(base.get_window_x(1920), 640);
        assert_eq!(base.get_font_size_add(), 2);
        assert!(!base.auto_hide_on_focus_loss());
    }

    // 父 trait 泛型 getter 与 getGroupConfig 可空契约在 HUD 对象上同样可用
    #[test]
    fn test_inherited_generic_getters_on_hud() {
        let h = FakeHud::new();
        assert!(h.get_bool("any", true));
        assert!(!h.get_bool("any", false));
        assert_eq!(h.get_int("any", -7), -7);
        assert_eq!(h.get_string("any", "def"), "def");
        assert!(h.get_group_config().is_none());
    }
}
