//! HUDSettingsImpl — Java `private class HUDSettingsImpl extends
//! GenericOverlaySettingsImpl implements HUDSettings` 的组合+委托形态
//! (波11 自 configuration_service.rs 三分拆出)。

use super::*;

// =====================================================================
// HUDSettingsImpl — Java `private class HUDSettingsImpl extends
// GenericOverlaySettingsImpl implements HUDSettings` → 组合 + 委托
// =====================================================================

pub struct HUDSettingsImpl {
    /// PORT: extends → 组合 (§1); 父类方法经 base 委托 (Java 单实现继承)
    base: GenericOverlaySettingsImpl,
}

impl HUDSettingsImpl {
    /// Java: `public HUDSettingsImpl() { super("MiniHUD"); }`
    /// (私有构造: 视图仅经 getHUDSettings 工厂产出, 同模块内调用)
    pub(super) fn new(service: Arc<ServiceInner>) -> Self {
        HUDSettingsImpl {
            base: GenericOverlaySettingsImpl::new(service, "MiniHUD"),
        }
    }

    /// Java: `private double getDouble(String key, double def)`
    fn get_double(&self, key: &str, def: f64) -> f64 {
        let val = self.base.service.get_config_j(key);
        if val.is_empty() {
            def
        } else {
            parse_double(&val).unwrap_or(def)
        }
    }

    /// Java: `public String getNumFont()`
    fn get_num_font(&self) -> String {
        let mut font = self.base.service.get_config_j("MonoNumFont");
        if font.is_empty() {
            font = self.base.service.get_config_j("GlobalNumFont");
        }
        if font.is_empty() {
            self.base.service.app_default_numfont_name()
        } else {
            font
        }
    }

    /// Java: `private double getDoubleFromLayoutFirst(String section, String property, double defaultVal)`
    pub(super) fn get_double_from_layout_first(
        &self,
        section: &str,
        property: &str,
        default_val: f64,
    ) -> f64 {
        // Priority 1: Check in-memory LayoutConfigs
        // (锁作用域内完成查找, 放锁后再走 Priority 2 — 不嵌套)
        let found = {
            let configs = self.base.service.layout_configs.read().expect(LC_LOCK_MSG);
            configs
                .as_ref()
                .and_then(|list| layout_first_double(list, section, property))
        };
        // Priority 2: Check global config.properties
        found.unwrap_or_else(|| self.get_double(property, default_val))
    }
}

/// getDoubleFromLayoutFirst 的 Priority 1 查找体 (锁内执行)
fn layout_first_double(list: &[GroupConfig], section: &str, property: &str) -> Option<f64> {
    for gc in list {
        if section.eq_ignore_ascii_case(&gc.title) {
            if let Some(row) = find_row_recursive(&gc.rows, property) {
                if let Some(v) = &row.value {
                    match v {
                        ConfigValue::Int(i) => return Some(f64::from(*i)),
                        ConfigValue::Double(d) => return Some(*d),
                        other => {
                            //      catch (NumberFormatException e) { // ignore } → 继续循环
                            if let Ok(d) = parse_double(&config_value_to_string(other)) {
                                return Some(d);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

impl OverlaySettings for HUDSettingsImpl {
    type GroupConfig = GroupConfig;

    /// Java: `@Override public String getNumFontName() { return getNumFont(); }`
    fn get_num_font_name(&self) -> String {
        self.get_num_font()
    }

    /// Java: `@Override public int getWindowX(int canvasWidth)`
    fn get_window_x(&self, canvas_width: i32) -> i32 {
        let gc = self.base.get_group_config_snapshot();
        let (screen_w, _) = self.base.service.screen_size();
        if let Some(gc) = gc {
            // (int) Math.round(gc.x * Application.screenWidth) — 双转窄化复刻
            return ((gc.x * f64::from(screen_w) + 0.5).floor() as i64) as u32 as i32;
        }
        self.base
            .get_int("crosshairX", (screen_w - canvas_width) / 2)
    }

    /// Java: `@Override public int getWindowY(int canvasHeight)`
    fn get_window_y(&self, canvas_height: i32) -> i32 {
        let gc = self.base.get_group_config_snapshot();
        let (_, screen_h) = self.base.service.screen_size();
        if let Some(gc) = gc {
            return ((gc.y * f64::from(screen_h) + 0.5).floor() as i64) as u32 as i32;
        }
        self.base
            .get_int("crosshairY", (screen_h - canvas_height) / 2)
    }

    /// Java: `@Override public void saveWindowPosition(double x, double y)`
    fn save_window_position(&self, x: f64, y: f64) {
        let gc = self.base.get_group_config_snapshot();
        let (screen_w, screen_h) = self.base.service.screen_size();
        if gc.is_some() {
            // Rust f64 除零同义 — 与父类实现不同, 子类无 screen>0 守卫, 保真)
            self.base.service.set_group_position_ignore_case(
                &self.base.section_name,
                x / f64::from(screen_w),
                y / f64::from(screen_h),
            );
            self.base.service.save_layout_config();
        } else {
            // (int) double: JLS 5.1.3 (NaN→0/饱和/向零) ↔ Rust as i32 同义
            self.base
                .service
                .set_config("crosshairX", &(x as i32).to_string());
            self.base
                .service
                .set_config("crosshairY", &(y as i32).to_string());
        }
    }

    // ---- 以下为 GenericOverlaySettingsImpl 继承成员的委托 ----

    fn get_group_config(&self) -> Option<&GroupConfig> {
        self.base.get_group_config()
    }

    fn get_font_name(&self) -> String {
        self.base.get_font_name()
    }

    fn get_font_size_add(&self) -> i32 {
        self.base.get_font_size_add()
    }

    fn get_bool(&self, key: &str, def: bool) -> bool {
        self.base.get_bool(key, def)
    }

    fn get_int(&self, key: &str, def: i32) -> i32 {
        self.base.get_int(key, def)
    }

    fn get_string(&self, key: &str, def: &str) -> String {
        self.base.get_string(key, def)
    }

    fn auto_hide_on_focus_loss(&self) -> bool {
        self.base.auto_hide_on_focus_loss()
    }
}

impl HUDSettings for HUDSettingsImpl {
    /// Java: `@Override public String getNumFont()`
    fn get_num_font(&self) -> String {
        HUDSettingsImpl::get_num_font(self)
    }

    /// Java: `@Override public int getCrosshairScale()`
    fn get_crosshair_scale(&self) -> i32 {
        let scale = self.base.get_int("crosshairScale", 70);
        if scale == 0 {
            1
        } else {
            scale
        }
    }

    /// Java: `@Override public String getCrosshairName()`
    fn get_crosshair_name(&self) -> String {
        java_trim(&self.base.service.get_config_j("crosshairName")).to_string()
    }

    /// Java: `@Override public boolean isDisplayCrosshair()`
    fn is_display_crosshair(&self) -> bool {
        self.base.get_bool("displayCrosshair", false)
    }

    /// Java: `@Override public boolean useTextureCrosshair()`
    fn use_texture_crosshair(&self) -> bool {
        let name = self.get_crosshair_name();
        // (getCrosshairName 恒非 null; CJK 无大小写折叠, equalsIgnoreCase ≡ equals)
        !name.is_empty() && name != "软件渲染准星"
    }

    /// Java: `@Override public boolean drawHUDText()`
    fn draw_hud_text(&self) -> bool {
        self.base.get_bool("drawHUDtext", true)
    }

    /// Java: `@Override public boolean showAttitudeGauge()`
    fn show_attitude_gauge(&self) -> bool {
        self.base.get_bool("showAttitudeGauge", true)
    }

    /// Java: `@Override public double getAoAWarningRatio()`
    fn get_aoa_warning_ratio(&self) -> f64 {
        let val = self.get_double_from_layout_first(
            &self.base.section_name.clone(),
            "miniHUDaoaWarningRatio",
            25.0,
        );
        if val > 1.0 {
            val / 100.0
        } else {
            val
        }
    }

    /// Java: `@Override public double getAoABarWarningRatio()`
    fn get_aoa_bar_warning_ratio(&self) -> f64 {
        let val = self.get_double_from_layout_first(
            &self.base.section_name.clone(),
            "miniHUDaoaBarWarningRatio",
            0.0,
        );
        if val > 1.0 {
            val / 100.0
        } else {
            val
        }
    }

    /// Java: `@Override public boolean enableFlapAngleBar()`
    fn enable_flap_angle_bar(&self) -> bool {
        self.base.get_bool("enableFlapAngleBar", true)
    }

    /// Java: `@Override public boolean showSpeedBar()`
    fn show_speed_bar(&self) -> bool {
        self.base.get_bool("showSpeedBar", true)
    }

    /// Java: `@Override public boolean drawHudMach()`
    fn draw_hud_mach(&self) -> bool {
        self.base.get_bool("hudMach", false)
    }

    /// Java: `@Override public boolean isSpeedLabelDisabled()`
    fn is_speed_label_disabled(&self) -> bool {
        self.base.get_bool("disableHUDSpeedLabel", false)
    }

    /// Java: `@Override public boolean isAltitudeLabelDisabled()`
    fn is_altitude_label_disabled(&self) -> bool {
        self.base.get_bool("disableHUDHeightLabel", false)
    }

    /// Java: `@Override public boolean isSEPLabelDisabled()`
    fn is_sep_label_disabled(&self) -> bool {
        self.base.get_bool("disableHUDSEPLabel", false)
    }

    /// Java: `@Override public boolean showHUDSpeed()`
    fn show_hud_speed(&self) -> bool {
        self.base.get_bool("showHUDSpeed", true)
    }

    /// Java: `@Override public boolean showHUDAoA()`
    fn show_hud_aoa(&self) -> bool {
        self.base.get_bool("showHUDAoA", true)
    }

    /// Java: `@Override public boolean showHUDAltitude()`
    fn show_hud_altitude(&self) -> bool {
        self.base.get_bool("showHUDAltitude", true)
    }

    /// Java: `@Override public boolean showHUDEnergy()`
    fn show_hud_energy(&self) -> bool {
        self.base.get_bool("showHUDEnergy", true)
    }

    /// Java: `@Override public boolean showHUDMechanization()`
    fn show_hud_mechanization(&self) -> bool {
        self.base.get_bool("showHUDMechanization", true)
    }

    /// Java: `@Override public boolean showHUDFlaps()`
    fn show_hud_flaps(&self) -> bool {
        self.base.get_bool("showHUDFlaps", true)
    }

    /// Java: `@Override public boolean showHUDAirbrake()`
    fn show_hud_airbrake(&self) -> bool {
        self.base.get_bool("showHUDAirbrake", true)
    }

    /// Java: `@Override public boolean showHUDGear()`
    fn show_hud_gear(&self) -> bool {
        self.base.get_bool("showHUDGear", true)
    }

    /// Java: `@Override public boolean showHUDSep()`
    fn show_hud_sep(&self) -> bool {
        self.base.get_bool("showHUDSep", true)
    }

    /// Java: `@Override public boolean showHUDGLoad()`
    fn show_hud_g_load(&self) -> bool {
        self.base.get_bool("showHUDGLoad", true)
    }

    /// Java: `@Override public boolean showHUDManeuverBar()`
    fn show_hud_maneuver_bar(&self) -> bool {
        self.base.get_bool("showHUDManeuverBar", true)
    }

    /// Java: `@Override public boolean isAttitudeIndicatorInertialMode()`
    fn is_attitude_indicator_inertial_mode(&self) -> bool {
        self.base.get_bool("attitudeIndicatorInertialMode", false)
    }

    /// Java: `@Override public boolean isGPUCompatibilityMode()`
    /// (接口注释: 底层 GPUCompatibilityHelper 经 CLASSIFY 裁决不迁移; 本实现
    /// 按源文件原样读配置存储值)
    fn is_gpu_compatibility_mode(&self) -> bool {
        self.base.get_bool("gpuCompatibilityMode", false)
    }

    /// Java: `@Override public boolean alwaysShowRadarAltitude()`
    fn always_show_radar_altitude(&self) -> bool {
        self.base.get_bool("alwaysShowRadarAltitude", false)
    }
}
