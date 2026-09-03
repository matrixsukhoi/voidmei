//! 对应 Java: `src/prog/config/HUDSettings.java`

use crate::config::config_api::overlay_settings::OverlaySettings;

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
    fn show_hud_speed(&self) -> bool; // Row 0: 速度文字
    fn show_hud_aoa(&self) -> bool; // Row 0: AoA bar + α文字
    fn show_hud_altitude(&self) -> bool; // Row 1: 高度文字
    fn show_hud_energy(&self) -> bool; // Row 1: 能量读数
    fn show_hud_mechanization(&self) -> bool; // Row 2: 襟翼/起落架文字 (旧，保留向后兼容)
    fn show_hud_flaps(&self) -> bool; // Row 2: 襟翼/可变翼
    fn show_hud_airbrake(&self) -> bool; // Row 2: 减速板 BRK
    fn show_hud_gear(&self) -> bool; // Row 2: 起落架 GEA
    fn show_hud_sep(&self) -> bool; // Row 3: 爬升率文字
    fn show_hud_g_load(&self) -> bool; // Row 4: G-force 文字
    fn show_hud_maneuver_bar(&self) -> bool; // Row 4: 机动条

    fn is_attitude_indicator_inertial_mode(&self) -> bool;

    /// PORT: 底层 GPUCompatibilityHelper 经 CLASSIFY 裁决"不迁移"(JVM 专属
    /// sun.java2d 属性, Rust 版无 JVM, 功能天然消亡); 接口方法按源文件原样保留,
    /// 未来 ConfigurationService 实现时可恒返回配置存储值/false。
    fn is_gpu_compatibility_mode(&self) -> bool;

    fn always_show_radar_altitude(&self) -> bool;
}

#[cfg(test)]
mod tests;
