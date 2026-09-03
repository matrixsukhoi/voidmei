//! HudSettingsSnapshot — HUDSettings 的 Send 快照 (渲染线程注册面输入)。
//! PORT(移仓备案): 原居 vm-app app_shell.rs; WYSIWYG reinit 链接通后
//! vm-overlay 的 spec 工厂 reinit 闭包需按快照重建 MiniHUD (reinit_config 泛型
//! S: HUDSettings 的实参), 快照随 trait 同居本仓 (vm-app 经 vm_core:: 引用)。

use std::collections::HashMap;

use super::{HUDSettings, OverlaySettings};

/// MiniHUD 注册所需的 HUDSettings 全量值快照。
/// ConfigurationService (!Send, Rc<SExp> 配置树) 不能进渲染线程,
/// 主线程 (AppShell) 构建本纯值快照随 [`Win32ThreadConfig`] 送入。
/// `get_window_x/y`: 窗口定位归 OverlayHost 位置存档 (host.materialize),
/// ctx.window_x/y 在 Rust 端无消费点 — 返回 0 (保位)。
/// PORT(派生面): Default/PartialEq 供 ReinitParams 缺省构造与 UiCommand 断言。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HudSettingsSnapshot {
    pub num_font: String,
    pub crosshair_scale: i32,
    pub crosshair_name: String,
    pub display_crosshair: bool,
    pub use_texture_crosshair: bool,
    pub draw_hud_text: bool,
    pub show_attitude_gauge: bool,
    pub aoa_warning_ratio: f64,
    pub aoa_bar_warning_ratio: f64,
    pub enable_flap_angle_bar: bool,
    pub show_speed_bar: bool,
    pub draw_hud_mach: bool,
    pub speed_label_disabled: bool,
    pub altitude_label_disabled: bool,
    pub sep_label_disabled: bool,
    pub show_hud_speed: bool,
    pub show_hud_aoa: bool,
    pub show_hud_altitude: bool,
    pub show_hud_energy: bool,
    pub show_hud_mechanization: bool,
    pub show_hud_flaps: bool,
    pub show_hud_airbrake: bool,
    pub show_hud_gear: bool,
    pub show_hud_sep: bool,
    pub show_hud_g_load: bool,
    pub show_hud_maneuver_bar: bool,
    pub attitude_indicator_inertial_mode: bool,
    pub gpu_compatibility_mode: bool,
    pub always_show_radar_altitude: bool,
    pub font_name: String,
    pub num_font_name: String,
    pub font_size_add: i32,
    pub auto_hide_on_focus_loss: bool,
    /// 通用 bool getter 快照 (minihud initModernLayout 读 "enableLayoutDebug")
    pub bools: HashMap<String, bool>,
}

impl HudSettingsSnapshot {
    /// 主线程从真实设置视图提取 (调用点持 ConfigurationService)
    pub fn build<S: HUDSettings>(s: &S) -> Self {
        HudSettingsSnapshot {
            num_font: s.get_num_font(),
            crosshair_scale: s.get_crosshair_scale(),
            crosshair_name: s.get_crosshair_name(),
            display_crosshair: s.is_display_crosshair(),
            use_texture_crosshair: s.use_texture_crosshair(),
            draw_hud_text: s.draw_hud_text(),
            show_attitude_gauge: s.show_attitude_gauge(),
            aoa_warning_ratio: s.get_aoa_warning_ratio(),
            aoa_bar_warning_ratio: s.get_aoa_bar_warning_ratio(),
            enable_flap_angle_bar: s.enable_flap_angle_bar(),
            show_speed_bar: s.show_speed_bar(),
            draw_hud_mach: s.draw_hud_mach(),
            speed_label_disabled: s.is_speed_label_disabled(),
            altitude_label_disabled: s.is_altitude_label_disabled(),
            sep_label_disabled: s.is_sep_label_disabled(),
            show_hud_speed: s.show_hud_speed(),
            show_hud_aoa: s.show_hud_aoa(),
            show_hud_altitude: s.show_hud_altitude(),
            show_hud_energy: s.show_hud_energy(),
            show_hud_mechanization: s.show_hud_mechanization(),
            show_hud_flaps: s.show_hud_flaps(),
            show_hud_airbrake: s.show_hud_airbrake(),
            show_hud_gear: s.show_hud_gear(),
            show_hud_sep: s.show_hud_sep(),
            show_hud_g_load: s.show_hud_g_load(),
            show_hud_maneuver_bar: s.show_hud_maneuver_bar(),
            attitude_indicator_inertial_mode: s.is_attitude_indicator_inertial_mode(),
            gpu_compatibility_mode: s.is_gpu_compatibility_mode(),
            always_show_radar_altitude: s.always_show_radar_altitude(),
            // 不取 get_font_name — 其 defaultFont 回退分支是 vm-core 保真
            // NPE (Application.defaultFont null, init_font 接线前不可达);
            // MiniHUD ctx 只消费 num 字体 (get_num_font), text 字体空串顶位
            font_name: String::new(),
            num_font_name: s.get_num_font_name(),
            font_size_add: s.get_font_size_add(),
            auto_hide_on_focus_loss: s.auto_hide_on_focus_loss(),
            // minihud 走通用 getter 的键集 (initModernLayout; 新键随接线补)
            bools: HashMap::from([(
                "enableLayoutDebug".to_string(),
                s.get_bool("enableLayoutDebug", false),
            )]),
        }
    }
}

impl OverlaySettings for HudSettingsSnapshot {
    type GroupConfig = ();
    fn get_window_x(&self, _width: i32) -> i32 {
        0 // 定位归 host 位置存档 (见类型注)
    }
    fn get_window_y(&self, _height: i32) -> i32 {
        0
    }
    fn save_window_position(&self, _x: f64, _y: f64) {
        // host.saved_positions 接管 (host.rs close 链), 无回写面
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
        self.bools.get(key).copied().unwrap_or(def)
    }
    fn get_int(&self, _key: &str, def: i32) -> i32 {
        def
    }
    fn get_string(&self, _key: &str, def: &str) -> String {
        def.to_string()
    }
    fn get_group_config(&self) -> Option<&Self::GroupConfig> {
        None
    }
    fn auto_hide_on_focus_loss(&self) -> bool {
        self.auto_hide_on_focus_loss
    }
}

impl HUDSettings for HudSettingsSnapshot {
    fn get_num_font(&self) -> String {
        self.num_font.clone()
    }
    fn get_crosshair_scale(&self) -> i32 {
        self.crosshair_scale
    }
    fn get_crosshair_name(&self) -> String {
        self.crosshair_name.clone()
    }
    fn is_display_crosshair(&self) -> bool {
        self.display_crosshair
    }
    fn use_texture_crosshair(&self) -> bool {
        self.use_texture_crosshair
    }
    fn draw_hud_text(&self) -> bool {
        self.draw_hud_text
    }
    fn show_attitude_gauge(&self) -> bool {
        self.show_attitude_gauge
    }
    fn get_aoa_warning_ratio(&self) -> f64 {
        self.aoa_warning_ratio
    }
    fn get_aoa_bar_warning_ratio(&self) -> f64 {
        self.aoa_bar_warning_ratio
    }
    fn enable_flap_angle_bar(&self) -> bool {
        self.enable_flap_angle_bar
    }
    fn show_speed_bar(&self) -> bool {
        self.show_speed_bar
    }
    fn draw_hud_mach(&self) -> bool {
        self.draw_hud_mach
    }
    fn is_speed_label_disabled(&self) -> bool {
        self.speed_label_disabled
    }
    fn is_altitude_label_disabled(&self) -> bool {
        self.altitude_label_disabled
    }
    fn is_sep_label_disabled(&self) -> bool {
        self.sep_label_disabled
    }
    fn show_hud_speed(&self) -> bool {
        self.show_hud_speed
    }
    fn show_hud_aoa(&self) -> bool {
        self.show_hud_aoa
    }
    fn show_hud_altitude(&self) -> bool {
        self.show_hud_altitude
    }
    fn show_hud_energy(&self) -> bool {
        self.show_hud_energy
    }
    fn show_hud_mechanization(&self) -> bool {
        self.show_hud_mechanization
    }
    fn show_hud_flaps(&self) -> bool {
        self.show_hud_flaps
    }
    fn show_hud_airbrake(&self) -> bool {
        self.show_hud_airbrake
    }
    fn show_hud_gear(&self) -> bool {
        self.show_hud_gear
    }
    fn show_hud_sep(&self) -> bool {
        self.show_hud_sep
    }
    fn show_hud_g_load(&self) -> bool {
        self.show_hud_g_load
    }
    fn show_hud_maneuver_bar(&self) -> bool {
        self.show_hud_maneuver_bar
    }
    fn is_attitude_indicator_inertial_mode(&self) -> bool {
        self.attitude_indicator_inertial_mode
    }
    fn is_gpu_compatibility_mode(&self) -> bool {
        self.gpu_compatibility_mode
    }
    fn always_show_radar_altitude(&self) -> bool {
        self.always_show_radar_altitude
    }
}
